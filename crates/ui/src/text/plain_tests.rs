//! The plain path of a fill (ticket 3219): a text with no message syntax
//! is filled by its `{name}`s alone, never parsed, and fills to what the
//! message path fills it to.

use proptest::prelude::*;
use shuyi_assets::lang::Lang;
use shuyi_assets::message::{Message, has_message_syntax, parse_count};

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
    let form = Message::parse(text).map_or(Form::NotAMessage(text), Form::Message);
    let parsed = fill_form(&form, lang, by, key_args, keys);
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
    parse_count::take();
    for text in [
        "",
        "no braces",
        "{Confirm} pick · {done} done",
        "l'épée, don't",
        "} {key} {",
        "{} {a}} #",
        "{n} keys, # left",
    ] {
        assert!(matches!(Form::of(text), Form::Plain(_)), "{text:?}");
    }
    assert_eq!(parse_count::take(), 0);
    for text in [
        "{n, plural, one {# key} other {# keys}}",
        "{device, select, playdate {Crank} other {Turn}}",
        "{ n }",
        "'{'Confirm'}'",
        "it''s",
    ] {
        assert!(matches!(Form::of(text), Form::Message(_)), "{text:?}");
    }
    assert_eq!(parse_count::take(), 5);
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

/// `text` filled from the form `lang` learned of it when it was built, and
/// by [`fill_styled_with`] of its text, with `args` as values or as key
/// values.
fn from_form(key: &str, lang: &Lang, args: &Args<'_>, as_keys: bool) -> (StyledText, StyledText) {
    let (values, key_args) = if as_keys {
        (&[][..], args)
    } else {
        (args, &[][..])
    };
    let source = lang.source_lang().code.clone();
    let (form, code) = lang.form_in(&source, key);
    let by = |name: &str| value_of(values, name);
    let kept = fill_form(&form, code, by, key_args, keys);
    let text = fill_styled_with(lang.text(&source, key), code, values, key_args, keys);
    (kept, text)
}

#[test]
fn every_text_of_the_toy_and_hello_grid_fills_from_its_form_as_from_its_text() {
    let mut english = lang_map(include_str!("../toy/en.ron"));
    english.extend(lang_map(include_str!(
        "../../../../examples/hello-grid/files/en.ron"
    )));
    let lang = Lang::new(english.clone(), BTreeMap::new());
    parse_count::take();
    for n in [0, 1, 2, 5, 21, -3] {
        for as_keys in [false, true] {
            for key in english.keys() {
                let (kept, text) = from_form(key, &lang, &values(&n), as_keys);
                assert_eq!(kept, text, "{key}, n = {n}, keys {as_keys}");
                let (kept, text) = from_form(key, &lang, &[], as_keys);
                assert_eq!(kept, text, "{key} with no values");
            }
        }
    }
    // Only the fills of the texts, never the forms, parsed.
    let messages = english.values().filter(|t| has_message_syntax(t)).count();
    assert_eq!(parse_count::take(), messages * 6 * 2 * 2);
}

/// The texts of a language file written `source`, by key.
fn lang_map(source: &str) -> BTreeMap<String, String> {
    let text: shuyi_assets::lang::English = ron::from_str(source).unwrap();
    text.0
}

proptest! {
    #[test]
    fn a_text_fills_from_its_form_as_from_its_text(
        text in r"([a-z]{1,3}|[{}',# \t\u{a0}é]|\{[a-zA-Z]{1,4}\}|\{n, plural, one \{#\} other \{# x\}\}|\{device, select, playdate \{p\} other \{o\}\}){0,12}",
        n in -5i64..30,
        as_keys in proptest::bool::ANY,
    ) {
        let lang = Lang::new(BTreeMap::from([("k".to_owned(), text.clone())]), BTreeMap::new());
        let (kept, filled) = from_form("k", &lang, &values(&n), as_keys);
        prop_assert_eq!(kept, filled);
        let (form, _) = lang.form_in(&lang.source_lang().code, "k");
        // Plain is the check alone: the record parsed nothing it calls so.
        prop_assert_eq!(matches!(form, Form::Plain(_)), !has_message_syntax(&text));
    }
}
