//! Tests of the fallback chain (a `pt-BR` pack over a `pt` one, over the
//! source), of a source the game declares, and of the folders' tags.

use super::*;
use crate::dialogue::{DialogueTable, NoDirective, Scene, parse_dlg};

/// No data text rules of a game's, no tokens, no variants.
const RULES: PackRules<'static> = PackRules {
    data_problem: |_, _| None,
    tokens: &crate::dialogue::NoTokens,
    limits: crate::dialogue::tests::LIMITS,
    widths: &CellWidths::NARROW,
    variants: &[],
};

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

/// Dialogue entries by key, each `(key, source, text)`.
fn lines(entries: &[(&str, &str, &str)]) -> BTreeMap<String, LineEntry> {
    entries
        .iter()
        .map(|&(key, source, text)| {
            let entry = LineEntry {
                key: key.to_owned(),
                source: source.to_owned(),
                text: LineText::One(text.to_owned()),
            };
            (key.to_owned(), entry)
        })
        .collect()
}

/// A pack named `name` with screen text, data text and lines, each
/// `(key, source, text)`.
fn pack(
    name: &str,
    ui: &[(&str, &str, &str)],
    data: &[(&str, &str, &str)],
    dialogue: &[(&str, &str, &str)],
) -> LangPack {
    LangPack {
        info: LangInfo {
            name: name.to_owned(),
            made_by: MadeBy::Machine,
        },
        ui: entries(ui),
        data: entries(data),
        lines: lines(dialogue),
    }
}

const SCENE: &str = "@scene gate\n> Halt.\n> Who goes?\n> Pass.\n@end\n";

/// The scene `gate`.
fn scene() -> Scene {
    let (parsed, errors) = parse_dlg::<NoDirective>("story/gate.dlg", SCENE);
    assert_eq!(errors, []);
    parsed[0].scene.clone()
}

/// The id of the line of `gate` written `text`.
fn id(text: &str) -> String {
    let scene = scene();
    let line = scene.lines().into_iter().find(|l| l.text == text).unwrap();
    line.id.to_string()
}

/// English with packs `pt` and `pt-BR` (with `pt-BR` too if `brazil`).
/// Keys `a` and the line `Halt.`: both packs fresh. Keys `b` and `Who
/// goes?`: only `pt`. Keys `c` and `Pass.`: `pt-BR` stale, `pt` fresh.
/// Key `d`: neither.
fn portuguese(brazil: bool) -> Lang {
    let ui = owned(&[("a", "A"), ("b", "B"), ("c", "C"), ("d", "D")]);
    let mut source = SourceText::new(ui.clone(), crate::names::Names::default());
    let mut table = DialogueTable::<NoDirective>::default();
    table.scenes.insert("gate".to_owned(), scene());
    source.add_dialogue(&table);
    let (halt, who, pass) = (id("Halt."), id("Who goes?"), id("Pass."));
    let pt = pack(
        "Português",
        &[("a", "A", "pt a"), ("b", "B", "pt b"), ("c", "C", "pt c")],
        &[("a", "A", "pt a"), ("b", "B", "pt b"), ("c", "C", "pt c")],
        &[
            (&halt, "Halt.", "Alto."),
            (&who, "Who goes?", "Quem vem?"),
            (&pass, "Pass.", "Passe."),
        ],
    );
    let br = pack(
        "Português do Brasil",
        &[("a", "A", "br a"), ("c", "old C", "br c")],
        &[("a", "A", "br a"), ("c", "old C", "br c")],
        &[(&halt, "Halt.", "Pare."), (&pass, "Pass!", "Passa!")],
    );
    let mut packs = BTreeMap::from([(code("pt"), pt)]);
    if brazil {
        packs.insert(code("pt-BR"), br);
    }
    Lang::with_source(ui, source, packs)
}

#[test]
fn the_chain_is_the_tag_its_parents_with_packs_and_the_source() {
    let lang = portuguese(true);
    let chain = |tag: &str| -> Vec<String> {
        let chain = lang.chain(&code(tag));
        chain.iter().map(ToString::to_string).collect()
    };
    assert_eq!(chain("pt-BR"), ["pt-BR", "pt", "en"]);
    assert_eq!(chain("pt-BR-x-test"), ["pt-BR", "pt", "en"]);
    assert_eq!(chain("pt"), ["pt", "en"]);
    assert_eq!(chain("pt-PT"), ["pt", "en"]);
    assert_eq!(chain("en"), ["en"]);
    assert_eq!(chain("en-GB"), ["en"]);
    assert_eq!(chain("fr"), ["en"]);
    // Without the middle pack.
    let mut packs = BTreeMap::new();
    packs.insert(code("pt-BR"), pack("Br", &[], &[], &[]));
    let lang = Lang::new(BTreeMap::new(), packs);
    assert_eq!(lang.chain(&code("pt-BR")), [code("pt-BR"), code("en")]);
    // A pack under the source's own tag is never in it.
    let packs = BTreeMap::from([(code("en"), pack("En", &[("a", "A", "x")], &[], &[]))]);
    let lang = Lang::new(owned(&[("a", "A")]), packs);
    assert_eq!(lang.chain(&code("en")), [code("en")]);
    assert_eq!(lang.text(&code("en"), "a"), "A");
}

#[test]
fn screen_and_data_text_fall_back_along_the_chain() {
    let lang = portuguese(true);
    let br = code("pt-BR");
    let text = |key| lang.text(&br, key);
    assert_eq!(
        [text("a"), text("b"), text("c"), text("d")],
        ["br a", "pt b", "pt c", "D"]
    );
    let data = |key: &str, english| lang.data_text(&br, || key.to_owned(), english);
    assert_eq!(
        [
            data("a", "A"),
            data("b", "B"),
            data("c", "C"),
            data("d", "D")
        ],
        ["br a", "pt b", "pt c", "D"]
    );
    assert_eq!(lang.name(&br, "nobody", "Nobody"), "Nobody");
    // `found` and `names` walk it too.
    assert_eq!(lang.found(&br, "", "B"), "pt b");
    assert_eq!(lang.found(&br, "", "C"), "pt c");
    assert_eq!(lang.found(&code("fr"), "", "B"), "B");
    // `pt` itself never reads `pt-BR`'s.
    assert_eq!(lang.text(&code("pt"), "a"), "pt a");
    // Without `pt-BR`'s pack, `pt-BR` is `pt`.
    let lang = portuguese(false);
    assert_eq!(lang.text(&br, "a"), "pt a");
    assert_eq!(lang.data_text(&br, || "a".to_owned(), "A"), "pt a");
}

#[test]
fn the_key_of_data_text_is_built_only_with_a_pack_along_the_chain() {
    let lang = portuguese(true);
    let unbuilt = || -> String { panic!("the key was built") };
    assert_eq!(lang.data_text(&code("fr"), unbuilt, "A"), "A");
    let names = crate::names::Names {
        names: owned(&[("king", "Osgar")]),
    };
    assert_eq!(lang.names(&code("fr"), &names), names);
    assert_eq!(lang.names(&code("pt-BR"), &names), names);
}

#[test]
fn lines_and_scenes_fall_back_along_the_chain() {
    let lang = portuguese(true);
    let br = code("pt-BR");
    let line = |text| lang.line(&br, &id(text), text, "");
    assert_eq!(
        [line("Halt."), line("Who goes?"), line("Pass.")],
        ["Pare.", "Quem vem?", "Passe."]
    );
    let shown = lang.scene(&br, &scene(), "");
    let texts: Vec<&str> = shown.lines().iter().map(|l| l.text).collect();
    assert_eq!(texts, ["Pare.", "Quem vem?", "Passe."]);
    let shown = lang.scene(&code("fr"), &scene(), "");
    assert_eq!(shown, scene());
}

#[test]
fn status_is_what_the_one_pack_lacks() {
    let lang = portuguese(true);
    let status = lang.status(&code("pt-BR")).unwrap();
    assert_eq!(status.missing[..2], ["b".to_owned(), "d".to_owned()]);
    assert_eq!(status.stale[..1], ["c".to_owned()]);
    assert_eq!(lang.status(&code("en")), Some(LangStatus::default()));
}

const INFO: &str = r#"(name: "English", made_by: Human("A. Translator"))"#;

/// The languages of a game written in Japanese made of `files`.
fn japanese(files: &[(&str, &str)]) -> Result<Lang, Vec<String>> {
    let files: Vec<LangFile<'_>> = files
        .iter()
        .map(|&(path, source)| LangFile {
            path,
            source: Some(source),
        })
        .collect();
    let source = SourceLang::new(code("ja"), "日本語");
    from_files_in(&source, &files, None, |p| format!("lang/{p}"), &RULES)
        .map_err(|errors| errors.iter().map(ToString::to_string).collect())
}

const JA_UI: &str = r#"{"title.start": "はじめる", "title.quit": "やめる"}"#;

#[test]
fn a_game_written_in_japanese_reads_its_folder_as_the_source() {
    let en_ui = r#"[(key: "title.start", source: "はじめる", text: "Start")]"#;
    let lang = japanese(&[
        ("ja/ui.ron", JA_UI),
        ("en/lang.ron", INFO),
        ("en/ui.ron", en_ui),
    ])
    .unwrap();
    assert_eq!(lang.source_lang().code, code("ja"));
    assert_eq!(lang.source_lang().name, "日本語");
    let en = code("en");
    assert_eq!(lang.text(&en, "title.start"), "Start");
    assert_eq!(lang.text(&en, "title.quit"), "やめる");
    assert_eq!(lang.chain(&en), [code("en"), code("ja")]);
    assert_eq!(lang.source_text("title.quit"), Some("やめる"));
    assert_eq!(lang.status(&code("ja")), Some(LangStatus::default()));
    assert_eq!(lang.status(&en).unwrap().missing, ["title.quit"]);
    // The source's folder holds its screen text only.
    let errors = japanese(&[("ja/ui.ron", JA_UI), ("ja/lang.ron", INFO)]).unwrap_err();
    assert_eq!(errors, ["lang/ja/lang.ron: not a file a language has"]);
    // Without its folder, the source is missing.
    let errors = japanese(&[("en/ui.ron", "[]"), ("en/lang.ron", INFO)]).unwrap_err();
    assert_eq!(errors, ["lang/ja/ui.ron: file not found in asset bundle"]);
    // A pack's unknown key names the source by its tag.
    let unknown = r#"[(key: "title.nope", source: "x", text: "y")]"#;
    let errors = japanese(&[
        ("ja/ui.ron", JA_UI),
        ("en/lang.ron", INFO),
        ("en/ui.ron", unknown),
    ])
    .unwrap_err();
    assert_eq!(
        errors,
        ["lang/en/ui.ron: \"title.nope\": no such key in the source language (ja)"]
    );
}

#[test]
fn a_folder_is_named_by_its_canonical_tag() {
    let files = |dir: &str| {
        let (info, ui) = (format!("{dir}/lang.ron"), format!("{dir}/ui.ron"));
        let files = [
            ("en/ui.ron", "{}".to_owned()),
            (info.as_str(), INFO.to_owned()),
            (ui.as_str(), "[]".to_owned()),
        ];
        let files: Vec<LangFile<'_>> = files
            .iter()
            .map(|(path, source)| LangFile {
                path,
                source: Some(source),
            })
            .collect();
        from_files(&files, None, |p| format!("lang/{p}"), &RULES)
            .map(|lang| lang.codes().map(ToString::to_string).collect::<Vec<_>>())
            .map_err(|errors| errors.iter().map(ToString::to_string).collect::<Vec<_>>())
    };
    assert_eq!(files("pt-BR"), Ok(vec!["pt-BR".to_owned()]));
    assert_eq!(files("zh-Hans"), Ok(vec!["zh-Hans".to_owned()]));
    assert_eq!(
        files("pt-br"),
        Err(vec![
            "lang/pt-br: name a language's folder by its tag as BCP 47 writes it: pt-BR".to_owned()
        ])
    );
    assert_eq!(
        files("x1"),
        Err(vec![
            "lang/x1: name a language's folder by a BCP 47 tag (en, pt-BR, zh-Hans): a tag \
             starts with its language, 2 to 8 letters, not \"x1\""
                .to_owned()
        ])
    );
}

/// The tags of the subset, from a few parts, so that chains share parents.
fn tag_text() -> impl proptest::strategy::Strategy<Value = String> {
    use proptest::prelude::*;
    (
        proptest::sample::select(vec!["en", "pt", "zh"]),
        proptest::option::of(proptest::sample::select(vec!["Hans", "Latn"])),
        proptest::option::of(proptest::sample::select(vec!["BR", "419"])),
        proptest::option::of(proptest::sample::select(vec!["1996", "x-test"])),
    )
        .prop_map(|(language, script, region, last)| {
            let parts = [Some(language), script, region, last];
            parts.into_iter().flatten().collect::<Vec<_>>().join("-")
        })
}

proptest::proptest! {
    /// Whatever the packs and the source, a chain is never empty, ends
    /// with the source and holds no tag twice.
    #[test]
    fn a_chain_ends_with_the_source_and_holds_no_tag_twice(
        tag in tag_text(),
        packs in proptest::collection::vec(tag_text(), 0..6),
        source in tag_text(),
    ) {
        let packs = packs.iter().map(|t| (code(t), pack(t, &[], &[], &[]))).collect();
        let source = SourceLang::new(code(&source), "Source");
        let lang = Lang::new(BTreeMap::new(), packs).with_source_lang(source.clone());
        let chain = lang.chain(&code(&tag));
        proptest::prop_assert_eq!(chain.last(), Some(&source.code));
        let unique: alloc::collections::BTreeSet<&LangCode> = chain.iter().collect();
        proptest::prop_assert_eq!(unique.len(), chain.len(), "{:?}", chain);
    }
}
