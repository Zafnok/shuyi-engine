//! What building a language costs (ticket 2266): a language of plain texts
//! allocates nothing to learn their forms, and a load the size of a
//! game's menus (833 screen texts, 33 of them plurals or selects, about
//! 350 with a placeholder, and two packs of them) parses each text at most
//! twice. One test: the heap counter is the whole process's, so nothing
//! runs beside it.
//!
//! `cargo test -p shuyi-assets --test lang_load -- --nocapture` prints the
//! load's heap peak and allocations.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use shuyi_alloc_count::{global, tags};
use shuyi_assets::dialogue::{DialogueLimits, NoTokens};
use shuyi_assets::font::CellWidths;
use shuyi_assets::lang::{
    Lang, LangCode, LangFile, LangPack, PackRules, SourceLang, english_from_source, from_files_in,
    pack_from_sources,
};
use shuyi_assets::message::{Form, has_message_syntax, parse_count};

/// How many screen texts the synthetic language has, and of what kind.
const KEYS: usize = 833;
const MESSAGES: usize = 33;
const PLACEHOLDERS: usize = 350;

/// The source text of key number `n`, and a pack's translation of it in
/// a language with only the plural category `other` (Japanese, Chinese).
fn text(n: usize) -> (String, String) {
    if n < MESSAGES / 2 {
        let source = format!("{{n, plural, one {{# item {n}}} other {{# items {n}}}}}");
        (source, format!("{{n, plural, other {{# 個 {n}}}}}"))
    } else if n < MESSAGES {
        let source =
            format!("{{device, select, playdate {{Crank {n}}} other {{Turn {n}}}}} {{Confirm}}");
        let text =
            format!("{{device, select, playdate {{回す {n}}} other {{回る {n}}}}} {{Confirm}}");
        (source, text)
    } else if n < MESSAGES + PLACEHOLDERS {
        let source = format!("{{Cursor}} move · {{Confirm}} pick {n} · {{count}} left");
        (
            source,
            format!("{{Cursor}} 移動 · {{Confirm}} 選ぶ {n} · {{count}}"),
        )
    } else {
        (format!("Menu entry number {n}"), format!("メニュー {n}"))
    }
}

fn key(n: usize) -> String {
    format!("menu.entry_{n:03}")
}

/// RON's string literal of `text`.
fn quoted(text: &str) -> String {
    format!("{text:?}")
}

/// The synthetic languages' files, of the keys `keys`: `en/ui.ron`, and
/// `ja` and `zh-Hans` packs of every key.
fn files(keys: core::ops::Range<usize>) -> Vec<(String, String)> {
    let mut english = String::from("{\n");
    let mut pack = String::from("[\n");
    for n in keys {
        let (source, text) = text(n);
        let _ = writeln!(english, "    {}: {},", quoted(&key(n)), quoted(&source));
        let _ = writeln!(
            pack,
            "    (key: {}, source: {}, text: {}),",
            quoted(&key(n)),
            quoted(&source),
            quoted(&text)
        );
    }
    english.push('}');
    pack.push(']');
    let info = |name: &str| format!("(name: {}, made_by: Machine)", quoted(name));
    vec![
        ("en/ui.ron".to_owned(), english),
        ("ja/lang.ron".to_owned(), info("日本語")),
        ("ja/ui.ron".to_owned(), pack.clone()),
        ("zh-Hans/lang.ron".to_owned(), info("简体中文")),
        ("zh-Hans/ui.ron".to_owned(), pack),
    ]
}

const RULES: PackRules<'static> = PackRules::new(
    |_, _| None,
    &NoTokens,
    DialogueLimits::new(40, 3).with_max_text(200),
    &CellWidths::NARROW,
);

/// The languages `files` make: a function of its own, which Callgrind
/// counts the instructions of (`--toggle-collect='lang_load::load'`).
#[inline(never)]
fn load(files: &[(String, String)]) -> Lang {
    let files: Vec<LangFile<'_>> = files
        .iter()
        .map(|(path, source)| LangFile {
            path,
            source: Some(source),
        })
        .collect();
    let source = SourceLang::default();
    from_files_in(&source, &files, None, ToOwned::to_owned, &RULES)
        .unwrap_or_else(|e| panic!("{e:?}"))
}

/// The heap's counts: only a build that counts runs this test.
fn allocs() -> u32 {
    global::reading()
        .unwrap_or_else(|| panic!("this build doesn't count"))
        .allocs
}

/// The screen texts and packs of the plain keys only, built before the
/// count.
fn plain_texts() -> (BTreeMap<String, String>, BTreeMap<LangCode, LangPack>) {
    let files: BTreeMap<String, String> = files(MESSAGES..KEYS).into_iter().collect();
    let english =
        english_from_source("en/ui.ron", &files["en/ui.ron"]).unwrap_or_else(|e| panic!("{e:?}"));
    let packs = ["ja", "zh-Hans"].map(|tag| {
        let (info, ui) = (format!("{tag}/lang.ron"), format!("{tag}/ui.ron"));
        let pack = pack_from_sources((&info, &files[&info]), (&ui, &files[&ui]), &english, &RULES);
        let code = LangCode::new(tag).unwrap_or_else(|| panic!("{tag}"));
        (code, pack.unwrap_or_else(|e| panic!("{e:?}")))
    });
    (english, packs.into())
}

#[cfg_attr(not(debug_assertions), ignore = "only a debug build counts the heap")]
#[test]
fn plain_texts_cost_nothing_to_learn_and_a_load_parses_each_text_at_most_twice() {
    // Plain texts: building the language from them allocates nothing,
    // as before their forms were learned.
    let (ui, packs) = plain_texts();
    // The source's language, made by every `Lang`: two strings.
    let english = SourceLang::default();
    allocs();
    let lang = Lang::new(ui, packs);
    assert_eq!(allocs(), 2, "a language of plain texts allocated");
    assert_eq!(lang.source_lang(), &english);
    drop(lang);

    // The synthetic load, from its files.
    let files = files(0..KEYS);
    let tag = tags::tag(2, "lang-load");
    parse_count::take();
    allocs();
    let outside = tags::enter(tag);
    let lang = load(&files);
    tags::leave(outside);
    let (load_allocs, parses) = (allocs(), parse_count::take());
    let peak = global::rows()
        .and_then(|rows| rows.into_iter().find(|row| row.tag == tag))
        .map_or(0, |row| row.peak_bytes);
    eprintln!(
        "the synthetic load: {load_allocs} allocations, a heap peak of {peak} bytes, {parses} parses"
    );

    // The check parses each source text and each entry's text and source;
    // the record each text with message syntax, once more.
    let syntax = (0..KEYS)
        .map(text)
        .filter(|(source, _)| has_message_syntax(source))
        .count();
    let packs = 2;
    let checked = KEYS + packs * 2 * KEYS;
    assert_eq!(parses, checked + syntax * (1 + packs));

    // Each message's form is kept, and none is parsed again.
    for tag in ["en", "ja", "zh-Hans"] {
        let code = LangCode::new(tag).unwrap();
        let kept = (0..KEYS)
            .filter(|&n| matches!(lang.form_in(&code, &key(n)).0, Form::Message(_)))
            .count();
        assert_eq!(kept, MESSAGES, "{tag}");
    }
    assert_eq!(parse_count::take(), 0);
}
