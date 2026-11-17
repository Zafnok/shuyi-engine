//! [`StyledText`] and [`fill_styled`].

use proptest::prelude::*;
use shuyi_assets::font::CellWidths;

use super::*;
use crate::frame::draw::cut;

const KEY: Option<TextRole> = Some(TextRole::Key);
const ARROWS: Option<TextRole> = Some(TextRole::Arrows);
const WARN: Option<TextRole> = Some(TextRole::Engine(EngineColor::Warning));

/// `text`'s spans as `(text, role)` pairs.
fn spans(text: &StyledText) -> Vec<(&str, Option<TextRole>)> {
    text.spans()
        .iter()
        .map(|s| (s.text.as_str(), s.role))
        .collect()
}

#[test]
fn a_push_joins_a_span_of_its_role_and_an_empty_one_adds_nothing() {
    let mut s = StyledText::new();
    assert!(s.is_empty());
    s.push_plain("").push_key("");
    assert!(s.is_empty() && s.spans().is_empty());
    s.push_key("Left").push_key(" Shift").push_plain(" run");
    s.push("", WARN).push(" now", None);
    assert_eq!(spans(&s), [("Left Shift", KEY), (" run now", None)]);
    assert!(!s.is_empty());
    // The same words in the same roles are equal, however pushed.
    let mut t = StyledText::new();
    t.push("Left Shift", KEY).push_plain(" run now");
    assert_eq!(s, t);
    // Another role doesn't join.
    t.push("!", WARN);
    assert_eq!(spans(&t).len(), 3);
}

#[test]
fn append_joins_at_the_seam_and_plain_and_display_drop_the_roles() {
    let mut s = StyledText::from("hold ");
    let mut key = StyledText::new();
    key.push_plain("the ").push_key("f").push_plain(" key");
    s.append(&key).append(&StyledText::new());
    assert_eq!(spans(&s), [("hold the ", None), ("f", KEY), (" key", None)]);
    assert_eq!(s.plain(), "hold the f key");
    assert_eq!(s.to_string(), "hold the f key");
    assert_eq!(StyledText::from(String::new()), StyledText::new());
    assert_eq!(StyledText::from(String::from("a")), StyledText::from("a"));
}

#[test]
fn width_counts_wide_glyphs_as_two_cells() {
    let widths = CellWidths::wide(['あ']);
    let mut s = StyledText::new();
    s.push_key("あ").push_plain(" ok");
    assert_eq!(s.width(&widths), 5);
    assert_eq!(s.width(&CellWidths::NARROW), 4);
}

#[test]
fn a_cut_keeps_each_glyphs_role_and_its_ellipsis_takes_the_next_ones() {
    let n = &CellWidths::NARROW;
    let mut s = StyledText::new();
    s.push_key("Enter").push_plain(" done");
    // It fits: unchanged.
    assert_eq!(s.cut(10, n), s);
    // Cut inside the key: the `…` is the key's.
    assert_eq!(spans(&s.cut(4, n)), [("Ent…", KEY)]);
    // Cut where the key ends: the `…` stands for the plain text.
    assert_eq!(spans(&s.cut(6, n)), [("Enter", KEY), ("…", None)]);
    assert_eq!(spans(&s.cut(1, n)), [("…", KEY)]);
    assert!(s.cut(0, n).is_empty());
    // A wide glyph that doesn't fit before the `…` is dropped whole.
    let widths = CellWidths::wide(['あ']);
    let mut wide = StyledText::new();
    wide.push_plain("aあ").push("b", WARN);
    assert_eq!(spans(&wide.cut(3, &widths)), [("a…", None)]);
}

#[test]
fn a_span_is_built_by_its_constructor() {
    let span = TextSpan::new("f", KEY);
    assert_eq!((span.text.as_str(), span.role), ("f", KEY));
}

fn lookup(name: &str) -> Option<String> {
    match name {
        "Confirm" => Some("f".to_owned()),
        "Run" => Some("Left Shift".to_owned()),
        _ => None,
    }
}

#[test]
fn fill_styled_puts_looked_up_keys_and_key_args_in_key_spans() {
    let filled = fill_styled(
        "{Confirm} pick · {Run} run · {done} done · {count} left",
        &[("count", &3)],
        &[("done", &"Num Enter")],
        lookup,
    );
    assert_eq!(
        spans(&filled),
        [
            ("f", KEY),
            (" pick · ", None),
            ("Left\u{a0}Shift", KEY),
            (" run · ", None),
            ("Num\u{a0}Enter", KEY),
            (" done · 3 left", None),
        ]
    );
}

#[test]
fn fill_styled_takes_args_first_and_leaves_the_rest_as_fill_does() {
    // A value in `args` wins over a key of the same name, and is plain.
    let filled = fill_styled(
        "{Confirm} {x}",
        &[("Confirm", &"y")],
        &[("x", &"z")],
        lookup,
    );
    assert_eq!(spans(&filled), [("y ", None), ("z", KEY)]);
    // Unknown placeholders, empty ones and an unclosed `{` stay as written.
    for text in ["{nope} {} {", "a {Confirm", "} {", ""] {
        let filled = fill_styled(text, &[], &[], lookup);
        assert_eq!(filled.plain(), fill(text, &[], lookup), "{text}");
        assert!(filled.spans().iter().all(|s| s.role.is_none()), "{text}");
    }
    assert_eq!(
        fill_styled("{nope} {", &[], &[], lookup).plain(),
        "{nope} {"
    );
}

/// A role, by a number.
fn role(n: u8) -> Option<TextRole> {
    match n % 5 {
        0 => None,
        1 => KEY,
        2 => WARN,
        3 => ARROWS,
        _ => Some(TextRole::Palette("gold")),
    }
}

/// Texts made of up to eight pushes of short text (wide glyphs among them)
/// in random roles.
fn styled() -> impl Strategy<Value = StyledText> {
    proptest::collection::vec(("[a あ.]{0,4}", 0..5_u8), 0..8).prop_map(|pushes| {
        let mut s = StyledText::new();
        for (text, n) in &pushes {
            s.push(text, role(*n));
        }
        s
    })
}

proptest! {
    #[test]
    fn spans_are_joined_and_read_as_the_plain_text(s in styled()) {
        let joined: String = s.spans().iter().map(|span| span.text.as_str()).collect();
        prop_assert_eq!(joined, s.plain());
        prop_assert!(s.spans().iter().all(|span| !span.text.is_empty()));
        prop_assert!(s.spans().windows(2).all(|w| w[0].role != w[1].role));
    }

    #[test]
    fn width_and_cut_are_the_plain_texts(s in styled(), w in 0..20_usize) {
        let widths = CellWidths::wide(['あ']);
        prop_assert_eq!(s.width(&widths), text_width(&s.plain(), &widths));
        prop_assert_eq!(s.cut(w, &widths).plain(), cut(&s.plain(), w, &widths));
    }

    /// With caps on, the text is the same, and the width is the plain
    /// text's and a cap's room a key; a cut fits.
    #[test]
    fn with_caps_the_width_is_the_plain_texts_and_the_keys_room(
        s in styled(),
        w in 0..20_usize,
    ) {
        // Without a picture, the arrow keys are a cap of their word.
        let keys = s.spans().iter().filter(|span| span.role == KEY || span.role == ARROWS);
        let plain = text_width(&s.plain(), &capped());
        prop_assert_eq!(s.width(&capped()), plain + keys.count());
        prop_assert!(s.cut(w, &capped()).width(&capped()) <= w);
    }

    /// With the arrow cluster's picture, an arrows span takes the
    /// picture's cells, whatever its word; a cut fits and keeps it whole.
    #[test]
    fn with_a_picture_the_arrows_take_its_cells(s in styled(), w in 0..20_usize) {
        let widths = pictured();
        let mut expected = 0;
        for span in s.spans() {
            expected += match span.role {
                ARROWS => 3,
                KEY => text_width(&span.text, &widths) + 1,
                _ => text_width(&span.text, &widths),
            };
        }
        prop_assert_eq!(s.width(&widths), expected);
        let cut = s.cut(w, &widths);
        prop_assert!(cut.width(&widths) <= w);
        let whole = |span: &TextSpan| s.spans().iter().any(|t| t.text == span.text);
        prop_assert!(cut.spans().iter().filter(|t| t.role == ARROWS).all(whole));
    }
}

/// [`capped`], drawing the arrow keys as a picture three cells wide.
fn pictured() -> CellWidths {
    capped().with_arrows_cells(Some(3))
}

#[test]
fn the_arrows_take_the_pictures_cells_and_are_cut_whole() {
    let mut hint = StyledText::new();
    hint.push_arrows("arrows").push_plain(" move");
    assert_eq!(spans(&hint), [("arrows", ARROWS), (" move", None)]);
    assert_eq!(hint.width(&CellWidths::NARROW), 11);
    // Caps without a picture: a cap of the word, a cell wider.
    assert_eq!(hint.width(&capped()), 12);
    assert_eq!(hint.width(&pictured()), 8);
    assert_eq!(hint.cut(8, &pictured()), hint);
    let mut kept = StyledText::new();
    kept.push_arrows("arrows").push_plain(" m…");
    assert_eq!(hint.cut(6, &pictured()), kept);
    let mut dropped = StyledText::new();
    dropped.push_plain("…");
    for w in 1..=3 {
        assert_eq!(hint.cut(w, &pictured()), dropped, "{w}");
    }
}

#[test]
fn fill_styled_with_keeps_the_roles_of_what_it_fills_in() {
    let cursor = |name: &str| {
        let mut keys = StyledText::new();
        match name {
            "Cursor" => keys.push_key("w").push_plain(" ").push_key("a"),
            "Arrows" => keys.push_arrows("arrow keys"),
            _ => return None,
        };
        Some(keys)
    };
    let filled = fill_styled_with(
        "{Cursor} · {Arrows} · {x} {y}",
        &[],
        &[("x", &"Num Enter")],
        cursor,
    );
    assert_eq!(
        spans(&filled),
        [
            ("w", KEY),
            ("\u{a0}", None),
            ("a", KEY),
            (" · ", None),
            ("arrow\u{a0}keys", ARROWS),
            (" · ", None),
            ("Num\u{a0}Enter", KEY),
            (" {y}", None),
        ]
    );
}

/// Widths that draw a key as a cap a cell wider than its name.
fn capped() -> CellWidths {
    CellWidths::wide(['あ']).with_key_cap_cells(Some(1))
}

#[test]
fn with_caps_a_hints_width_counts_each_keys_room() {
    let mut hint = StyledText::new();
    hint.push_key("F")
        .push_plain(" select · ")
        .push_key("Space");
    hint.push_plain(" end turn");
    assert_eq!(hint.plain(), "F select · Space end turn");
    assert_eq!(hint.width(&CellWidths::NARROW), 25);
    assert_eq!(hint.width(&capped()), 27);
}

#[test]
fn with_caps_a_cut_keeps_a_key_whole_or_puts_a_plain_ellipsis_for_it() {
    let mut hint = StyledText::new();
    hint.push_plain("a ").push_key("Space").push_plain(" b");
    // "a " 2, the cap 6, " b" 2: 10.
    assert_eq!(hint.cut(10, &capped()), hint);
    let mut kept = StyledText::new();
    kept.push_plain("a ").push_key("Space").push_plain("…");
    assert_eq!(hint.cut(9, &capped()), kept);
    let mut dropped = StyledText::new();
    dropped.push_plain("a …");
    for w in 3..=8 {
        assert_eq!(hint.cut(w, &capped()), dropped, "{w}");
    }
    assert!(hint.cut(0, &capped()).is_empty());
}
