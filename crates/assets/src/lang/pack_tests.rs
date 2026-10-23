//! Tests of a pack's data and dialogue text: the English it translates
//! ([`SourceText`]), what its files may hold, the lookups, and what a pack
//! lacks. The game here has a `{hero}` token and lines with variants `a`
//! and `b`.

use super::*;
use crate::dialogue::tests::{HeroTokens, LIMITS};
use crate::dialogue::{DialogueTable, NoDirective, Scene, Step, caption_key, parse_dlg};

/// Data text rules of the tests' game: a `tips.*` text is at most 5 cells.
fn data_problem(key: &str, text: &str) -> Option<String> {
    (key.starts_with("tips.") && text_width(text) > 5).then(|| "a tip is 5 cells".to_owned())
}

const RULES: PackRules<'static> = PackRules {
    data_problem,
    tokens: &HeroTokens,
    limits: LIMITS,
    variants: &["a", "b"],
};

/// A scene with a caption, narration, speech with a name token, replies
/// with reactions, an `@if` block and a hero token.
const DLG: &str = "\
@scene gate
@caption The gate, dusk
@left guide neutral
> The gate is shut.
guide: Open it, {n:king}.
@choice
* firm: I will wait here.
  > Nobody answers.
  @caption Later
* soft: Let us go.
@endchoice
@if guide
guide: Still here.
@else
> Gone.
@endif
guide: {hero} waits.
@end
";

fn dialogue() -> DialogueTable {
    let (parsed, errors) = parse_dlg::<NoDirective>("story/gate.dlg", DLG);
    assert_eq!(errors, []);
    DialogueTable {
        scenes: parsed
            .iter()
            .map(|p| (p.scene.id.clone(), p.scene.clone()))
            .collect(),
        files: [("gate".to_owned(), "story/gate.dlg".to_owned())].into(),
    }
}

fn names(entries: &[(&str, &str)]) -> Names {
    Names {
        names: entries
            .iter()
            .map(|&(id, name)| (id.to_owned(), name.to_owned()))
            .collect(),
    }
}

/// The English: an item's name, a tip, the names table and the scene.
fn source() -> SourceText {
    let data = [
        ("items.rope.name", "Rope"),
        ("tips.a.text", "Look"),
        ("terrain.bog.name", "Bog"),
        ("terrain.fen.name", "Fen"),
    ]
    .map(|(k, v)| (k.to_owned(), v.to_owned()));
    let mut source = SourceText::new(data.into(), names(&[("king", "Osgar")]));
    source.add_dialogue(&dialogue());
    source
}

/// The id of the line of the scene written `text`.
fn id(text: &str) -> String {
    let source = source();
    let found = source.lines.iter().find(|(_, l)| l.text == text);
    found.map(|(k, _)| k.clone()).unwrap_or_default()
}

const INFO: &str = r#"(name: "Xx", made_by: Machine)"#;

/// The pack `xx` with `data.ron` `data_ron` and the dialogue files
/// `dialogue`, or its errors.
fn pack(data_ron: &str, dialogue: &[(&str, &str)]) -> Result<LangPack, Vec<String>> {
    let sources = PackSources {
        info: ("xx/lang.ron", INFO),
        ui: ("xx/ui.ron", "[]"),
        data: Some(("xx/data.ron", data_ron)),
        dialogue: dialogue.to_vec(),
    };
    pack_from_files(&sources, &BTreeMap::new(), Some(&source()), &RULES)
        .map_err(|e| e.iter().map(ToString::to_string).collect())
}

/// The errors of a pack with dialogue file `gate.ron` holding `entries`.
fn line_problems(entries: &str) -> Vec<ContentError> {
    let sources = PackSources {
        info: ("xx/lang.ron", INFO),
        ui: ("xx/ui.ron", "[]"),
        data: Some(("xx/data.ron", "[]")),
        dialogue: vec![("xx/dialogue/gate.ron", entries)],
    };
    pack_from_files(&sources, &BTreeMap::new(), Some(&source()), &RULES)
        .err()
        .unwrap_or_default()
}

fn line_errors(entries: &str) -> Vec<String> {
    pack("[]", &[("xx/dialogue/gate.ron", entries)])
        .err()
        .unwrap_or_default()
}

fn code(code: &str) -> LangCode {
    LangCode::new(code).unwrap()
}

fn lang(pack: LangPack) -> Lang {
    Lang::with_source(BTreeMap::new(), source(), [(code("xx"), pack)].into())
}

/// A dialogue entry for the line written `text`, translated `translation`.
fn entry(text: &str, translation: &str) -> String {
    format!(
        "(key: {:?}, source: {text:?}, text: {translation:?})",
        id(text)
    )
}

#[test]
fn english_lines_replies_and_captions_have_their_kind_and_scene() {
    let source = source();
    let kind = |text: &str| {
        let line = source.lines.values().find(|l| l.text == text).unwrap();
        assert_eq!((line.scene.as_str(), line.file.as_str()), ("gate", "gate"));
        line.kind
    };
    assert_eq!(kind("The gate is shut."), LineKind::Speech);
    assert_eq!(kind("Open it, {n:king}."), LineKind::Speech);
    assert_eq!(kind("I will wait here."), LineKind::Reply);
    assert_eq!(kind("Nobody answers."), LineKind::Speech);
    assert_eq!(kind("The gate, dusk"), LineKind::Caption);
    assert_eq!(kind("Later"), LineKind::Caption);
    assert_eq!(kind("Still here."), LineKind::Speech);
    assert_eq!(kind("Gone."), LineKind::Speech);
    assert_eq!(source.lines.len(), 10);
    assert!(source.lines.contains_key(&caption_key("gate", "Later")));
    // The names table's names are data text too.
    assert_eq!(source.data["names.king"], "Osgar");
    assert_eq!(names_key("king"), "names.king");
    // A table not read from files has no file.
    let mut table = dialogue();
    table.files.clear();
    let mut bare = SourceText::default();
    bare.add_dialogue(&table);
    assert!(bare.lines.values().all(|l| l.file.is_empty()));
    let mut other = dialogue();
    other.files.insert("gate".into(), "a\\b\\gate".into());
    let mut windows = SourceText::default();
    windows.add_dialogue(&other);
    assert!(windows.lines.values().all(|l| l.file == "gate"));
}

#[test]
fn a_data_entry_keeps_to_the_names_and_the_games_rules() {
    let errors = pack(
        r#"[
            (key: "names.king", source: "Osgar", text: " "),
            (key: "items.rope.name", source: "Rope", text: "{x}"),
            (key: "tips.a.text", source: "Look", text: "Regardez"),
            (key: "nope", source: "x", text: "y"),
            (key: "items.rope.name", source: "Rope", text: "Corde"),
        ]"#,
        &[],
    )
    .unwrap_err();
    assert_eq!(
        errors,
        [
            "xx/data.ron: \"names.king\": a name must not be empty",
            "xx/data.ron: \"items.rope.name\": the text and its source must have the same placeholders",
            "xx/data.ron: \"tips.a.text\": a tip is 5 cells",
            "xx/data.ron: \"nope\": no such key in the data",
            "xx/data.ron: \"items.rope.name\": the key is used twice",
        ]
    );
    let brace = pack(
        r#"[(key: "names.king", source: "Osgar", text: "Os}gar")]"#,
        &[],
    );
    assert_eq!(
        brace.unwrap_err(),
        ["xx/data.ron: \"names.king\": a name can't hold a brace"]
    );
    assert!(
        pack(
            r#"[(key: "names.king", source: "Osgar", text: "Oscar")]"#,
            &[]
        )
        .is_ok()
    );
}

#[test]
fn a_dialogue_entry_has_text_or_every_variant() {
    let key = id("Gone.");
    let entry = |fields: &str| format!("[(key: {key:?}, source: \"Gone.\", {fields})]");
    let got = line_errors(&entry("text: \"Parti.\""));
    assert!(got.is_empty(), "{got:?}");
    let got = line_errors(&entry("text_a: \"Parti.\", text_b: \"Partie.\""));
    assert!(got.is_empty(), "{got:?}");
    for bad in [
        "text_a: \"x\"",
        "text_b: \"x\"",
        "text: \"x\", text_a: \"x\"",
        "text: \"x\", text_a: \"x\", text_b: \"x\"",
    ] {
        assert_eq!(
            line_errors(&entry(bad)),
            [format!(
                "xx/dialogue/gate.ron: {key:?}: an entry has `text`, or both `text_a` and `text_b`"
            )],
            "{bad}"
        );
    }
    // A variant the game doesn't name, or a field twice, is a file error.
    let unknown = line_errors(&entry("text_c: \"x\""));
    assert_eq!(unknown.len(), 1);
    assert!(
        unknown[0].contains(
            "unknown field `text_c`, expected one of `key`, `source`, `text`, `text_a`, `text_b`"
        ),
        "{unknown:?}"
    );
    let twice = line_errors(&entry("text: \"x\", text: \"y\""));
    assert!(twice[0].contains("duplicate field `text`"), "{twice:?}");
    let missing = line_errors("[(source: \"x\", text: \"y\")]");
    assert!(missing[0].contains("key"), "{missing:?}");
    let no_source = line_errors(&format!("[(key: {key:?}, text: \"y\")]"));
    assert!(no_source[0].contains("source"), "{no_source:?}");
    let not_a_list = line_errors("(key: \"x\")");
    assert_eq!(not_a_list.len(), 1);
    let not_an_entry = line_errors("[1]");
    assert_eq!(not_an_entry.len(), 1);
}

#[test]
fn a_dialogue_key_used_twice_fails() {
    let one = entry("Gone.", "Parti.");
    let errors = line_problems(&format!("[{one}, {one}]"));
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(
        errors[0].to_string(),
        format!(
            "xx/dialogue/gate.ron: {:?}: the key is used twice",
            id("Gone.")
        )
    );
    assert_eq!(errors[0].code, Some(codes::DUPLICATE));
}

#[test]
fn a_dialogue_entry_keeps_to_the_rules_of_english() {
    let shut = id("The gate is shut.");
    let reply = id("I will wait here.");
    let caption = caption_key("gate", "The gate, dusk");
    let long = |n: usize| "x".repeat(n);
    let errors = line_errors(&format!(
        "[
            (key: {shut:?}, source: \"x\", text: \" \"),
            (key: {reply:?}, source: \"x\", text: {:?}),
            (key: {caption:?}, source: \"x\", text: {:?}),
            (key: \"gate\", source: \"x\", text: \"y\"),
            (key: \"gone_0123abcd\", source: \"x\", text: {:?}),
            (key: {shut:?}, source: \"x\", text: \"{{x}} {{hero}} {{n:nobody}} {{\"),
        ]",
        long(LIMITS.max_option + 1),
        long(500),
        long(500),
    ));
    let file = "xx/dialogue/gate.ron";
    assert_eq!(
        errors,
        [
            format!("{file}: {shut:?}: the text must not be empty"),
            format!("{file}: {reply:?}: text is 31 cells; the limit is 30"),
            format!("{file}: \"gate\": not a line id or a caption key"),
            format!("{file}: {shut:?}: the key is used twice"),
            format!("{file}: {shut:?}: unknown token \"{{x}}\"; use a name, {{n:<id>}}"),
            format!("{file}: {shut:?}: unknown name id \"nobody\" in {{n:nobody}}"),
            format!("{file}: {shut:?}: \"{{\" has no closing \"}}\""),
        ]
    );
    // A line may be exactly as long as its limit.
    let gone = id("Gone.");
    let exact = long(LIMITS.max_text);
    assert_eq!(
        line_errors(&format!(
            "[(key: {gone:?}, source: \"Gone.\", text: {exact:?})]"
        )),
        [] as [&str; 0]
    );
    // A speech line keeps to the box's limit, every variant of it.
    let gone = id("Gone.");
    let over = long(LIMITS.max_text + 1);
    assert_eq!(
        line_errors(&format!(
            "[(key: {gone:?}, source: \"Gone.\", text_a: \"ok\", text_b: {over:?})]"
        )),
        [format!(
            "{file}: {gone:?}: text is 81 cells; the limit is 80"
        )]
    );
}

#[test]
fn a_name_token_counts_as_the_packs_longest_name() {
    let line = id("Open it, {n:king}.");
    let text = format!("{} {{n:king}}", "x".repeat(LIMITS.max_text - 11));
    let entries = format!("[(key: {line:?}, source: \"Open it, {{n:king}}.\", text: {text:?})]");
    // English's name (5 cells) fits; the pack's own longer one doesn't.
    assert_eq!(
        pack("[]", &[("xx/dialogue/gate.ron", &entries)]).err(),
        None
    );
    let longer = r#"[(key: "names.king", source: "Osgar", text: "Osgar the Old")]"#;
    let errors = pack(longer, &[("xx/dialogue/gate.ron", &entries)]).unwrap_err();
    assert_eq!(
        errors,
        [format!(
            "xx/dialogue/gate.ron: {line:?}: text is 83 cells; the limit is 80"
        )]
    );
    // A name made from older English isn't the pack's.
    let stale = r#"[(key: "names.king", source: "Old", text: "Osgar the Old")]"#;
    assert!(pack(stale, &[("xx/dialogue/gate.ron", &entries)]).is_ok());
}

#[test]
fn lines_and_scenes_in_a_language() {
    let later = format!(
        "(key: {:?}, source: \"Later\", text: \"Plus tard\")",
        caption_key("gate", "Later")
    );
    let entries = format!(
        "[{}, {}, {}, {}, {later}, (key: {:?}, source: \"{{hero}} waits.\", text_a: \"{{hero}} A.\", text_b: \"{{hero}} B.\")]",
        entry("The gate is shut.", "La porte."),
        entry("I will wait here.", "J'attends."),
        entry("Nobody answers.", "Personne."),
        entry("Gone.", "Parti."),
        id("{hero} waits."),
    );
    let lang = lang(pack("[]", &[("xx/dialogue/gate.ron", &entries)]).unwrap());
    let xx = code("xx");
    let shut = id("The gate is shut.");
    assert_eq!(lang.line(&xx, &shut, "The gate is shut.", "a"), "La porte.");
    // Made from other English: stale.
    assert_eq!(
        lang.line(&xx, &shut, "The gate is open.", "a"),
        "The gate is open."
    );
    // English, an unknown code, an unknown key.
    assert_eq!(
        lang.line(&LangCode::english(), &shut, "The gate is shut.", "a"),
        "The gate is shut."
    );
    assert_eq!(
        lang.line(&code("zz"), &shut, "The gate is shut.", "a"),
        "The gate is shut."
    );
    assert_eq!(lang.line(&xx, "nope", "x", "a"), "x");
    let waits = id("{hero} waits.");
    assert_eq!(lang.line(&xx, &waits, "{hero} waits.", "a"), "{hero} A.");
    assert_eq!(lang.line(&xx, &waits, "{hero} waits.", "b"), "{hero} B.");
    // A variant the line hasn't: English.
    assert_eq!(
        lang.line(&xx, &waits, "{hero} waits.", "c"),
        "{hero} waits."
    );
    let pack_line = lang.pack(&xx).unwrap().line(&waits).unwrap();
    assert_eq!(pack_line.source, "{hero} waits.");
    // The scene: every text step, in blocks too, keeping its ids.
    let table = dialogue();
    let scene: &Scene = table.get("gate").unwrap();
    let shown = lang.scene(&xx, scene, "b");
    let texts: Vec<String> = shown.lines().iter().map(|l| l.text.to_owned()).collect();
    assert_eq!(
        texts,
        [
            "La porte.",
            "Open it, {n:king}.",
            "J'attends.",
            "Personne.",
            "Let us go.",
            "Still here.",
            "Parti.",
            "{hero} B.",
        ]
    );
    let ids: Vec<String> = shown.lines().iter().map(|l| l.id.to_string()).collect();
    let before: Vec<String> = scene.lines().iter().map(|l| l.id.to_string()).collect();
    assert_eq!(ids, before);
    let Step::Choice { options } = &shown.steps[4] else {
        panic!("{:?}", shown.steps[4]);
    };
    assert_eq!(
        options[0].steps[1],
        Step::Caption {
            text: "Plus tard".into()
        }
    );
    assert_eq!(
        shown.steps[0],
        Step::Caption {
            text: "The gate, dusk".into()
        }
    );
    // A code with no pack, the scene as it is.
    assert_eq!(&lang.scene(&code("zz"), scene, "a"), scene);
}

#[test]
fn data_lookups_use_the_pack_while_its_source_is_todays_english() {
    let data = r#"[
        (key: "items.rope.name", source: "Rope", text: "Corde"),
        (key: "names.king", source: "Osgar", text: "Oscar"),
        (key: "terrain.bog.name", source: "Bog", text: "Marais"),
        (key: "terrain.fen.name", source: "Old Fen", text: "Vieux"),
    ]"#;
    let lang = lang(pack(data, &[]).unwrap());
    let xx = code("xx");
    let key = || "items.rope.name".to_owned();
    assert_eq!(lang.data_text(&xx, key, "Rope"), "Corde");
    assert_eq!(lang.data_text(&xx, key, "Rope!"), "Rope!");
    assert_eq!(lang.data_text(&xx, || "nope".to_owned(), "x"), "x");
    assert_eq!(lang.data_text(&code("zz"), key, "Rope"), "Rope");
    assert_eq!(lang.name(&xx, "king", "Osgar"), "Oscar");
    let table = names(&[("king", "Osgar"), ("queen", "Ila")]);
    assert_eq!(
        lang.names(&xx, &table),
        names(&[("king", "Oscar"), ("queen", "Ila")])
    );
    assert_eq!(lang.names(&code("zz"), &table), table);
    // Found by English among keys with a prefix.
    assert_eq!(lang.found(&xx, "terrain.", "Bog"), "Marais");
    assert_eq!(lang.found(&xx, "terrain.", "Fen"), "Fen");
    assert_eq!(lang.found(&xx, "terrain.", "Rope"), "Rope");
    assert_eq!(lang.found(&xx, "items.", "Bog"), "Bog");
    assert_eq!(lang.found(&code("zz"), "terrain.", "Bog"), "Bog");
    assert_eq!(
        lang.pack(&xx)
            .unwrap()
            .entry("items.rope.name")
            .unwrap()
            .text,
        "Corde"
    );
}

#[test]
fn status_lists_data_and_lines_and_pairs_orphans() {
    let gone = id("Gone.");
    let entries = format!(
        "[{}, (key: \"gate_0123abcd\", source: \"Go.\", text: \"Va.\"), (key: \"caption.gate_0123abcd\", source: \"Dusk\", text: \"Soir\"), (key: \"other_0123abcd\", source: \"x\", text: \"y\")]",
        entry("Gone.", "Parti.")
    );
    let data = r#"[(key: "items.rope.name", source: "Old rope", text: "Corde")]"#;
    let lang = lang(pack(data, &[("xx/dialogue/gate.ron", &entries)]).unwrap());
    let status = lang.status(&code("xx")).unwrap();
    assert_eq!(status.stale, ["items.rope.name"]);
    assert!(status.missing.contains(&"names.king".to_owned()));
    assert!(status.missing.contains(&id("Still here.")));
    assert!(!status.missing.contains(&gone));
    let orphans: Vec<(&str, Option<&str>)> = status
        .orphans
        .iter()
        .map(|o| (o.key.as_str(), o.nearest.as_deref()))
        .collect();
    let later = caption_key("gate", "Later");
    let go = id("Let us go.");
    assert_eq!(
        orphans,
        [
            ("caption.gate_0123abcd", Some(later.as_str())),
            ("gate_0123abcd", Some(go.as_str())),
            ("other_0123abcd", None),
        ]
    );
    assert_eq!(status.orphans[1].text, LineText::One("Va.".into()));
    assert_eq!(status.orphans[1].source, "Go.");
}

#[test]
fn from_files_reads_data_and_dialogue() {
    let files = [
        ("en/ui.ron", "{\"a.one\": \"One\"}"),
        ("xx/lang.ron", INFO),
        ("xx/ui.ron", "[]"),
        (
            "xx/data.ron",
            r#"[(key: "items.rope.name", source: "Rope", text: "Corde")]"#,
        ),
        ("xx/dialogue/gate.ron", "[]"),
    ];
    let files: Vec<LangFile<'_>> = files
        .iter()
        .map(|&(path, source)| LangFile {
            path,
            source: Some(source),
        })
        .collect();
    let lang = from_files(&files, Some(source()), str::to_owned, &RULES).unwrap();
    assert_eq!(
        lang.data_text(&code("xx"), || "items.rope.name".into(), "Rope"),
        "Corde"
    );
    assert_eq!(lang.source(), &source());
    assert_eq!(lang.english("items.rope.name"), Some("Rope"));
    assert_eq!(lang.english(&id("Gone.")), Some("Gone."));
    // Without the data, a pack's data and dialogue are left out.
    let lang = from_files(&files, None, str::to_owned, &RULES).unwrap();
    assert_eq!(lang.source(), &SourceText::default());
    assert_eq!(
        lang.data_text(&code("xx"), || "items.rope.name".into(), "Rope"),
        "Rope"
    );
    assert!(format!("{RULES:?}").contains("variants"));
}
