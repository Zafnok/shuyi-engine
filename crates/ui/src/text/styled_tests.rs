//! [`StyledText`] and [`fill_styled`].

use proptest::prelude::*;
use shuyi_assets::font::CellWidths;

use super::*;
use crate::frame::draw::cut;

const KEY: Option<TextRole> = Some(TextRole::Key);
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
    match n % 4 {
        0 => None,
        1 => KEY,
        2 => WARN,
        _ => Some(TextRole::Palette("gold")),
    }
}

/// Texts made of up to eight pushes of short text (wide glyphs among them)
/// in random roles.
fn styled() -> impl Strategy<Value = StyledText> {
    proptest::collection::vec(("[a あ.]{0,4}", 0..4_u8), 0..8).prop_map(|pushes| {
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
}
