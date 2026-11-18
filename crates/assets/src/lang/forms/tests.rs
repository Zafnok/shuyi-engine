//! Each text's form, learned as the languages are built: plain, a message
//! or not a message, along the fallback chain, for screen and data text.

use super::*;
use crate::lang::{LangInfo, MadeBy, SourceText};
use crate::message::{Arg, parse_count};

fn code(code: &str) -> LangCode {
    LangCode::new(code).unwrap()
}

fn owned(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|&(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

/// Entries by key, each `(key, source, text)`.
fn entries(entries: &[(&str, &str, &str)]) -> BTreeMap<String, Entry> {
    entries
        .iter()
        .map(|&(key, source, text)| {
            let entry = Entry {
                key: key.to_owned(),
                source: source.to_owned(),
                text: text.to_owned(),
            };
            (key.to_owned(), entry)
        })
        .collect()
}

fn pack(ui: &[(&str, &str, &str)], data: &[(&str, &str, &str)]) -> LangPack {
    LangPack {
        info: LangInfo::new("Test", MadeBy::Machine),
        ui: entries(ui),
        data: entries(data),
        lines: BTreeMap::new(),
    }
}

const COUNT: &str = "{n, plural, one {# key} other {# keys}}";
const TURN: &str = "{device, select, playdate {Crank} other {Turn}}";
const BROKEN: &str = "{n, plural, one {# key}";

/// English with packs `pt` and `pt-BR`: `plain` and `count` in both (a
/// stale `pt-BR` `count`), `turn` only in `pt`, `broken` in neither.
fn lang() -> Lang {
    let texts = [
        ("plain", "Press {Confirm}"),
        ("count", COUNT),
        ("turn", TURN),
        ("broken", BROKEN),
    ];
    let ui = owned(&texts);
    let source = SourceText::new(owned(&texts), crate::names::Names::default());
    let pt_count = "{n, plural, one {# chave} other {# chaves}}";
    let pt_turn = "{device, select, playdate {Gire} other {Vire}}";
    let pt_entries = [
        ("plain", "Press {Confirm}", "Aperte {Confirm}"),
        ("count", COUNT, pt_count),
        ("turn", TURN, pt_turn),
    ];
    let br_entries = [
        ("plain", "Press {Confirm}", "Pressione {Confirm}"),
        (
            "count",
            "{n} keys",
            "{n, plural, one {# tecla} other {# teclas}}",
        ),
    ];
    let packs = BTreeMap::from([
        (code("pt"), pack(&pt_entries, &pt_entries)),
        (code("pt-BR"), pack(&br_entries, &br_entries)),
    ]);
    Lang::with_source(ui, source, packs)
}

/// What `form` says with `n` and `device`, and which kind it is.
fn said(form: &Form<'_>, lang: &str) -> (&'static str, String) {
    let value = |name: &str| match name {
        "n" => Some(Arg::Int(2)),
        "device" => Some(Arg::Text("playdate".to_owned())),
        _ => None,
    };
    match form {
        Form::Plain(text) => ("plain", (*text).to_owned()),
        Form::Message(message) => ("message", message.format(lang, value)),
        Form::NotAMessage(text) => ("not a message", (*text).to_owned()),
    }
}

#[test]
fn a_screen_text_has_its_form_along_the_chain() {
    let lang = lang();
    parse_count::take();
    let check = |tag: &str, key: &str, (kind, says, from): (&str, &str, &str)| {
        let (form, code) = lang.form_in(&self::code(tag), key);
        let text = lang.text_in(&self::code(tag), key);
        assert_eq!((form.text(), code), text, "{tag} {key}");
        assert_eq!(said(&form, code), (kind, says.to_owned()), "{tag} {key}");
        assert_eq!(code, from, "{tag} {key}");
    };
    check("pt-BR", "plain", ("plain", "Pressione {Confirm}", "pt-BR"));
    // `pt-BR`'s `count` is stale: `pt`'s.
    check("pt-BR", "count", ("message", "2 chaves", "pt"));
    // `pt-BR` lacks `turn`: `pt`'s.
    check("pt-BR", "turn", ("message", "Gire", "pt"));
    check("pt", "turn", ("message", "Gire", "pt"));
    // Neither has `broken`: the source's, which isn't a message.
    check("pt-BR", "broken", ("not a message", BROKEN, "en"));
    check("en", "count", ("message", "2 keys", "en"));
    check("fr", "turn", ("message", "Crank", "en"));
    // A key the source lacks is itself.
    check("pt", "no.such.key", ("plain", "no.such.key", "en"));
    assert_eq!(
        parse_count::take(),
        0,
        "a form is learned once, at the load"
    );
}

#[test]
fn a_data_text_has_its_form_along_the_chain() {
    let lang = lang();
    parse_count::take();
    let data = |tag: &str, key: &str, english: &str| {
        let owned_key = key.to_owned();
        let (form, code) = lang.data_form_in(&code(tag), || owned_key, english);
        let text = lang.data_text_in(&self::code(tag), || key.to_owned(), english);
        assert_eq!((form.text(), code), text, "{tag} {key}");
        let (kind, said) = said(&form, code);
        (kind, said, code.to_owned())
    };
    assert_eq!(
        data("pt-BR", "count", COUNT),
        ("message", "2 chaves".to_owned(), "pt".to_owned())
    );
    assert_eq!(
        data("fr", "turn", TURN),
        ("message", "Crank".to_owned(), "en".to_owned())
    );
    assert_eq!(
        data("fr", "plain", "Press {Confirm}"),
        ("plain", "Press {Confirm}".to_owned(), "en".to_owned())
    );
    assert_eq!(
        parse_count::take(),
        0,
        "a form is learned once, at the load"
    );
    // Text the source's data doesn't have is read as the caller hands it.
    let held = "{n, plural, one {# tip} other {# tips}}";
    assert_eq!(
        data("pt", "tips.other", held),
        ("message", "2 tips".to_owned(), "en".to_owned())
    );
    assert_eq!(parse_count::take(), 1);
}

#[test]
fn a_plain_text_is_never_parsed_nor_kept() {
    parse_count::take();
    let plain = owned(&[("a", "Press {Confirm}"), ("b", "l'épée, {n} keys}")]);
    let lang = Lang::new(plain, BTreeMap::new());
    assert_eq!(parse_count::take(), 0);
    assert_eq!(lang.forms, Forms::default());
    let messages = owned(&[("a", COUNT), ("b", "{ n }")]);
    let lang = Lang::new(messages, BTreeMap::new());
    assert_eq!(parse_count::take(), 2);
    assert_eq!(lang.forms.ui.len(), 2);
}

#[test]
fn two_langs_of_the_same_texts_are_equal_and_a_clone_fills_the_same() {
    let (one, two) = (lang(), lang());
    assert_eq!(one, two);
    let clone = one.clone();
    for tag in ["pt-BR", "pt", "en"] {
        for key in ["plain", "count", "turn", "broken"] {
            let (a, b) = (one.form_in(&code(tag), key), clone.form_in(&code(tag), key));
            assert_eq!(said(&a.0, a.1), said(&b.0, b.1), "{tag} {key}");
        }
    }
}
