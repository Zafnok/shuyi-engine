//! The plain path of a fill (ticket 3219): a text with no message syntax
//! is filled by its `{name}`s alone, never parsed, and fills to what the
//! message path fills it to.

use core::cell::Cell;

use proptest::prelude::*;

use super::*;

/// The values a text is filled with: numbers for plurals, words, and a
/// value with braces.
fn values(n: &i64) -> [(&str, &dyn ToArg); 6] {
    [
        ("n", n),
        ("count", n),
        ("name", &"Mei"),
        ("done", &"{Confirm}"),
        ("device", &"playdate"),
        ("target", &"desktop"),
    ]
}

/// What a capitalized name looks up: a key of two words, so its space
/// joins.
fn keys(name: &str) -> Option<StyledText> {
    let first = name.chars().next()?;
    first.is_uppercase().then(|| {
        let mut key = StyledText::new();
        key.push_key(&format!("Left {name}"));
        key
    })
}

/// `text` filled as a fill does, and as the message path alone does (the
/// parse tried on every text, as before 3219), with `args` as values or as
/// key values.
fn both(text: &str, lang: &str, args: &Args<'_>, as_keys: bool) -> (StyledText, StyledText) {
    let (values, key_args) = if as_keys {
        (&[][..], args)
    } else {
        (args, &[][..])
    };
    let fast = fill_styled_with(text, lang, values, key_args, keys);
    let by = |name: &str| value_of(values, name);
    let parsed = fill_parsed(text, Message::parse(text).ok(), lang, by, key_args, keys);
    (fast, parsed)
}

/// Checks the two paths agree on `text`, in two languages, its values
/// plain and as keys, for numbers that pick different plural forms.
fn agree(text: &str) {
    for n in [0, 1, 2, 5, 21, -3] {
        for lang in ["en", "ru", "fr"] {
            for as_keys in [false, true] {
                let (fast, parsed) = both(text, lang, &values(&n), as_keys);
                assert_eq!(fast, parsed, "{text:?} in {lang}, n = {n}, keys {as_keys}");
            }
        }
    }
}

/// Every string literal of Rust source `source`, its escapes roughly read:
/// the texts the text tests fill.
fn literals(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = source.chars();
    while let Some(c) = chars.next() {
        if c != '"' {
            continue;
        }
        let mut text = String::new();
        while let Some(c) = chars.next() {
            match c {
                '"' => break,
                '\\' => match chars.next() {
                    Some('n') => text.push('\n'),
                    Some(other) => text.push(other),
                    None => {}
                },
                _ => text.push(c),
            }
        }
        out.push(text);
    }
    out
}

/// The text of a language file, `en.ron` written `source`.
fn lang_file(source: &str) -> Vec<String> {
    let text: shuyi_assets::lang::English = ron::from_str(source).unwrap();
    text.0.into_values().collect()
}

#[test]
fn every_text_of_the_toy_hello_grid_and_the_text_tests_fills_the_same_either_way() {
    let mut texts = lang_file(include_str!("../toy/en.ron"));
    texts.extend(lang_file(include_str!(
        "../../../../examples/hello-grid/files/en.ron"
    )));
    for source in [
        include_str!("../text.rs"),
        include_str!("styled_tests.rs"),
        include_str!("hint_tests.rs"),
        include_str!("../../../assets/src/message/tests.rs"),
    ] {
        texts.extend(literals(source));
    }
    assert!(texts.len() > 200, "{}", texts.len());
    let plain = texts.iter().filter(|t| !has_message_syntax(t)).count();
    // Most texts take the plain path, and some the message path.
    assert!(plain > texts.len() / 2 && plain < texts.len(), "{plain}");
    for text in &texts {
        agree(text);
    }
}

#[test]
fn a_text_with_no_message_syntax_is_never_parsed() {
    let parses = Cell::new(0);
    let counted = |text| {
        parses.set(parses.get() + 1);
        Message::parse(text)
    };
    for text in [
        "",
        "no braces",
        "{Confirm} pick · {done} done",
        "l'épée, don't",
        "} {key} {",
        "{} {a}} #",
        "{n} keys, # left",
    ] {
        assert!(message_by(text, counted).is_none(), "{text:?}");
    }
    assert_eq!(parses.get(), 0);
    for text in [
        "{n, plural, one {# key} other {# keys}}",
        "{device, select, playdate {Crank} other {Turn}}",
        "{ n }",
        "'{'Confirm'}'",
        "it''s",
    ] {
        assert!(message_by(text, counted).is_some(), "{text:?}");
    }
    assert_eq!(parses.get(), 5);
}

#[test]
fn what_a_message_reads_differently_takes_the_message_path() {
    let n = 2;
    let args = values(&n);
    // A quote, a plural and a placeholder with spaces each fill as a
    // message, not by their `{name}`s.
    assert_eq!(fill_values("'{'n'}'", "en", &args), "{n}");
    assert_eq!(fill_values("it''s", "en", &args), "it's");
    assert_eq!(fill_values("{ name }", "en", &args), "Mei");
    assert_eq!(
        fill_values("{n, plural, one {# key} other {# keys}}", "en", &args),
        "2 keys"
    );
}

proptest! {
    #[test]
    fn the_plain_path_fills_as_the_message_path_does(
        text in "([a-z]{1,3}|[{}',# \t\u{a0}\u{3000}é]|\\{[a-zA-Z]{1,4}\\}|\\{n, plural, other \\{#\\}\\}){0,12}",
        n in -5i64..30,
        as_keys in proptest::bool::ANY,
    ) {
        let (fast, parsed) = both(&text, "en", &values(&n), as_keys);
        prop_assert_eq!(fast, parsed);
    }
}
