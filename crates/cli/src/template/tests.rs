//! Templates: each check of the loader, `When`, the answers, each error
//! of the composer, the merged files, the placeholders, `apply`, and the
//! engine's own template's plans, golden.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use proptest::prelude::*;

use super::*;
use crate::catalog::{Catalog, Concerns, ModuleCrate, ModuleInfo};

/// A fixture template's manifest: three questions, a short name, and
/// parts that need, exclude and write over each other.
const MANIFEST_OK: &str = r#"Template(
    id: "fixture",
    name: "A fixture",
    version: "1.0.0",
    engine: ">=0.1, <0.2",
    license: "MIT OR Apache-2.0",
    format_version: 1,
    questions: [
        Question(
            id: "look",
            text: "Which looks?",
            pick: Many,
            answers: [Answer(id: "a", text: "A"), Answer(id: "b", text: "B")],
            default: ["a"],
        ),
        Question(
            id: "one",
            text: "Which one?",
            pick: One,
            answers: [Answer(id: "x", text: "X"), Answer(id: "y", text: "Y")],
        ),
        Question(
            id: "spare",
            text: "Anything else?",
            pick: Many,
            answers: [Answer(id: "unserved", text: "nothing serves it")],
        ),
    ],
    short_names: { "plain": {}, "both": { "look": ["a", "b"] } },
    parts: [
        Part(id: "base", when: Always, files: "base", targets: ["desktop"]),
        Part(id: "pa", when: Picked("look", "a"), files: "pa"),
        Part(id: "pb", when: Picked("look", "b"), files: "pb/", conflicts: ["pa"], targets: ["web", "desktop"]),
        Part(id: "px", when: All([Picked("one", "x"), Any([Picked("look", "a"), Picked("look", "b")])]), files: "px", requires: ["py"]),
        Part(id: "py", when: Picked("one", "y"), files: "py"),
    ],
)"#;

/// `files` as a template's files.
fn files(files: &[(&str, &str)]) -> Files {
    files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), text.as_bytes().to_vec()))
        .collect()
}

/// The fixture's files, with `manifest` as its `template.ron`.
fn fixture_files(manifest: &str) -> Files {
    files(&[
        (MANIFEST, manifest),
        (
            "base/Cargo.toml",
            "[workspace.dependencies]\nshuyi = { path = \"x\" }\n",
        ),
        ("base/shuyi.ron", "(\n    command: \"x\",\n)\n"),
        ("base/README.md", "# {{name}}\n"),
        ("pa/a.txt", "a of {{crate_name}}\n"),
        ("pb/b.txt", "b\n"),
        ("px/x.txt", "x\n"),
        ("py/y.txt", "y\n"),
    ])
}

/// The engine version the fixture is for.
fn engine() -> Version {
    Version::new(0, 1, 5)
}

/// The fixture template, its manifest `manifest`.
fn fixture(manifest: &str) -> Result<Template, LoadError> {
    Template::read("fixtures/t", fixture_files(manifest), &engine())
}

/// The fixture's manifest with `from` replaced by `to`, read.
fn broken(from: &str, to: &str) -> LoadError {
    assert!(MANIFEST_OK.contains(from), "{from}");
    fixture(&MANIFEST_OK.replacen(from, to, 1)).unwrap_err()
}

/// What fills the fixture's placeholders.
fn filled() -> Fill {
    Fill {
        name: "Moon Garden".to_owned(),
        crate_name: "moon-garden".to_owned(),
        engine: BTreeMap::from([("shuyi".to_owned(), "{ path = 'e' }".to_owned())]),
    }
}

/// A catalog with one module, `shuyi-alpha`, and one planned,
/// `shuyi-gamma`.
fn catalog() -> Catalog {
    let concerns = Concerns::parse(
        r#"(concerns: [(id: "bodies", name: "Bodies", alternatives: true, answers: [
            (id: "soft", name: "soft bodies", by: "shuyi-gamma", status: planned, tickets: ["9"]),
        ])])"#,
    )
    .unwrap();
    Catalog {
        root: PathBuf::from("engine"),
        concerns,
        modules: vec![ModuleCrate {
            name: "shuyi-alpha".to_owned(),
            dir: PathBuf::from("engine/crates/alpha"),
            info: ModuleInfo::default(),
        }],
    }
}

/// `pairs` as answers.
fn answers(pairs: &[(&str, &[&str])]) -> Answers {
    Answers(
        pairs
            .iter()
            .map(|(q, a)| ((*q).to_owned(), a.iter().map(|s| (*s).to_owned()).collect()))
            .collect(),
    )
}

/// The fixture composed for `answers`.
fn composed(template: &Template, answers: &Answers) -> Result<Plan, ComposeError> {
    compose(template, answers, &catalog(), &filled())
}

/// The text of the file `path` of `plan`.
fn text_of(plan: &Plan, path: &str) -> String {
    let bytes = plan
        .files
        .get(Path::new(path))
        .unwrap_or_else(|| panic!("no {path} in {:?}", plan.paths()));
    String::from_utf8(bytes.clone()).unwrap()
}

#[test]
fn the_fixture_reads_with_its_files_and_origin() {
    let template = fixture(MANIFEST_OK).unwrap();
    assert_eq!(template.id, "fixture");
    assert_eq!(template.origin, "fixtures/t");
    assert!(!template.files.contains_key(MANIFEST));
    assert_eq!(template.files.len(), 7);
    assert_eq!(template.short_name("both").unwrap().0["look"], ["a", "b"]);
    assert!(template.short_name("nope").is_none());
}

#[test]
fn each_load_check_names_the_file_and_the_field() {
    let cases: [(LoadError, &str); 18] = [
        (
            fixture("(").unwrap_err(),
            "fixtures/t/template.ron: 1:1-1:2: Unexpected end of RON",
        ),
        (
            broken("license", "licence"),
            "fixtures/t/template.ron: 6:5-6:12: Unexpected field named `licence` in \
             `Template`, expected one of `id`, `name`, `version`, `engine`, `license`, \
             `format_version`, `questions`, `short_names`, or `parts` instead",
        ),
        (
            broken("format_version: 1", "format_version: 2"),
            "fixtures/t/template.ron: format_version: 2: this shuyi reads templates of format 1",
        ),
        (
            broken(">=0.1, <0.2", ">=0.2"),
            "fixtures/t/template.ron: engine: `>=0.2` doesn't match this engine, 0.1.5: the \
             template makes games for another engine",
        ),
        (
            broken("id: \"one\"", "id: \"look\""),
            "fixtures/t/template.ron: questions[look]: a second question with this id",
        ),
        (
            broken("Answer(id: \"b\"", "Answer(id: \"a\""),
            "fixtures/t/template.ron: questions[look].answers: `a` is in twice",
        ),
        (
            broken("default: [\"a\"]", "default: [\"c\"]"),
            "fixtures/t/template.ron: questions[look].default: `c` is no answer of `look`: its \
             answers are `a`, `b`",
        ),
        (
            broken(
                "Answer(id: \"y\", text: \"Y\")],",
                "Answer(id: \"y\", text: \"Y\")], default: [\"x\", \"y\"],",
            ),
            "fixtures/t/template.ron: questions[one].default: `one` takes one answer, not 2",
        ),
        (
            broken("\"plain\": {}", "\"plain\": { \"size\": [\"big\"] }"),
            "fixtures/t/template.ron: short_names[plain]: no question `size`: the template asks \
             `look`, `one`, `spare`",
        ),
        (
            broken("id: \"pa\"", "id: \"base\""),
            "fixtures/t/template.ron: parts[base]: a second part with this id",
        ),
        (
            broken("files: \"pa\"", "files: \"nowhere\""),
            "fixtures/t/template.ron: parts[pa].files: the folder `nowhere/` holds no file",
        ),
        (
            broken("files: \"pa\"", "files: \"../pa\""),
            "fixtures/t/template.ron: parts[pa].files: `../pa` is no folder inside the \
             template: give a path from its folder, with no `.` or `..`",
        ),
        (
            broken("requires: [\"py\"]", "requires: [\"pz\"]"),
            "fixtures/t/template.ron: parts[px].requires: no part `pz`",
        ),
        (
            broken("conflicts: [\"pa\"]", "conflicts: [\"pz\"]"),
            "fixtures/t/template.ron: parts[pb].conflicts: no part `pz`",
        ),
        (
            broken(
                "Picked(\"look\", \"a\"), files: \"pa\"",
                "Picked(\"size\", \"a\"), files: \"pa\"",
            ),
            "fixtures/t/template.ron: parts[pa].when: `Picked` names no question of the \
             template: `size`",
        ),
        (
            broken("Picked(\"one\", \"x\")", "Picked(\"one\", \"z\")"),
            "fixtures/t/template.ron: parts[px].when: `Picked` names no answer of `one`: `z`",
        ),
        (
            broken("files: \"py\"", "files: \"py\", stage: Some(\"Top Down\")"),
            "fixtures/t/template.ron: parts[py].stage: `Top Down`: a stage's id is lowercase \
             letters and digits in words joined by `-`, starting with a letter",
        ),
        (
            Template::read("fixtures/t", Files::new(), &engine()).unwrap_err(),
            "fixtures/t/template.ron: missing: every template has one",
        ),
    ];
    for (error, expected) in cases {
        assert_eq!(error.to_string(), expected);
    }
    let not_text = Template::read(
        "t",
        Files::from([(MANIFEST.to_owned(), vec![0xff])]),
        &engine(),
    )
    .unwrap_err();
    assert!(not_text.message.starts_with("not UTF-8 text"), "{not_text}");
}

#[test]
fn a_template_loads_from_a_folder() {
    let dir = std::env::temp_dir().join(format!("shuyi-template-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let manifest = MANIFEST_OK.replace(">=0.1, <0.2", &format!("={ENGINE_VERSION}"));
    for (path, bytes) in fixture_files(&manifest) {
        let to = dir.join(&path);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::write(to, bytes).unwrap();
    }
    let template = Template::load(&dir).unwrap();
    assert_eq!(template.files.len(), 7);
    assert!(template.files.contains_key("base/README.md"));
    let missing = Template::load(&dir.join("nowhere")).unwrap_err();
    assert!(missing.field.is_empty(), "{missing}");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_stage_id_is_lowercase_words_joined_by_dashes() {
    for good in ["top-down-grid", "side-view", "a", "grid2"] {
        assert!(is_stage_id(good), "{good}");
    }
    for bad in ["1abc", "Top", "top--down", "top-", "-top", "top_down", ""] {
        assert!(!is_stage_id(bad), "{bad}");
    }
}

#[test]
fn when_holds_and_says_why_in_words() {
    let picked = answers(&[("look", &["a"]), ("one", &["x"])]);
    let a = When::Picked("look".into(), "a".into());
    let b = When::Picked("look".into(), "b".into());
    let x = When::Picked("one".into(), "x".into());
    assert!(When::Always.holds(&picked));
    assert!(a.holds(&picked) && !b.holds(&picked));
    assert!(When::All(vec![a.clone(), x.clone()]).holds(&picked));
    assert!(!When::All(vec![a.clone(), b.clone()]).holds(&picked));
    assert!(When::Any(vec![b.clone(), x.clone()]).holds(&picked));
    assert!(When::All(vec![]).holds(&picked) && !When::Any(vec![]).holds(&picked));
    assert_eq!(When::Always.because(&picked), "always");
    assert_eq!(
        When::All(vec![a.clone(), When::Any(vec![b.clone(), x.clone()])]).because(&picked),
        "`look` is `a` and `one` is `x`"
    );
    assert_eq!(When::All(vec![]).because(&picked), "always");
    assert_eq!(
        When::Any(vec![a, b, x]).picks(),
        [("look", "a"), ("look", "b"), ("one", "x")]
    );
}

#[test]
fn answers_read_from_a_file_and_from_set_flags() {
    let file = Answers::parse("a.ron", "{ \"look\": [\"a\", \"b\"] }").unwrap();
    assert_eq!(file, answers(&[("look", &["a", "b"])]));
    assert_eq!(Answers::parse("a.ron", &file.to_ron()).unwrap(), file);
    assert!(
        Answers::parse("a.ron", "[1]")
            .unwrap_err()
            .starts_with("a.ron: ")
    );
    assert_eq!(
        Answers::parse_set("look = a, b,").unwrap(),
        ("look".to_owned(), vec!["a".to_owned(), "b".to_owned()])
    );
    assert_eq!(
        Answers::parse_set("look=").unwrap(),
        ("look".to_owned(), vec![])
    );
    assert!(Answers::parse_set("look").is_err());
    assert!(Answers::parse_set("=a").is_err());
    let set = answers(&[("one", &["y"]), ("look", &["b"])]);
    assert_eq!(
        file.and(&set),
        answers(&[("look", &["b"]), ("one", &["y"])])
    );
}

#[test]
fn the_parts_whose_when_holds_are_written_with_their_placeholders_filled() {
    let template = fixture(MANIFEST_OK).unwrap();
    let plan = composed(&template, &Answers::default()).unwrap();
    assert_eq!(plan.parts, ["base", "pa"]);
    assert_eq!(
        plan.paths(),
        ["Cargo.toml", "README.md", "a.txt", "shuyi.ron"]
    );
    assert_eq!(text_of(&plan, "README.md"), "# Moon Garden\n");
    assert_eq!(text_of(&plan, "a.txt"), "a of moon-garden\n");
    assert_eq!(plan.answers.0["look"], ["a"]);
    assert_eq!(plan.targets, ["desktop"]);
    assert!(plan.stages.is_empty() && plan.modules.is_empty());
    assert_eq!(
        plan.limits,
        Vec::<String>::new(),
        "the defaults are all served"
    );
    let unserved = composed(&template, &answers(&[("spare", &["unserved"])])).unwrap();
    assert_eq!(
        unserved.limits,
        ["no part of the template for `spare` = `unserved`: nothing added"]
    );
}

#[test]
fn an_answer_the_template_doesnt_take_is_refused() {
    let template = fixture(MANIFEST_OK).unwrap();
    let cases = [
        (
            answers(&[("size", &["big"])]),
            "no question `size`: the template asks `look`, `one`, `spare`",
        ),
        (
            answers(&[("look", &["c"])]),
            "`c` is no answer of `look`: its answers are `a`, `b`",
        ),
        (
            answers(&[("one", &["x", "y"])]),
            "`one` takes one answer, not 2",
        ),
    ];
    for (picked, expected) in cases {
        let error = composed(&template, &picked).unwrap_err();
        assert_eq!(error.to_string(), expected);
        assert!(matches!(error, ComposeError::Answer { .. }));
    }
}

#[test]
fn a_part_that_needs_another_names_the_answer_that_picked_it() {
    let template = fixture(MANIFEST_OK).unwrap();
    let error = composed(&template, &answers(&[("one", &["x"])])).unwrap_err();
    assert_eq!(
        error.to_string(),
        "part `px` (picked because `one` is `x` and `look` is `a`) needs part `py`, which no \
         answer picks"
    );
}

#[test]
fn two_parts_that_conflict_say_which_answers_led_to_each() {
    let template = fixture(MANIFEST_OK).unwrap();
    let both = template.short_name("both").unwrap().clone();
    let error = composed(&template, &both).unwrap_err();
    assert_eq!(
        error,
        ComposeError::Conflict {
            parts: ["pb".to_owned(), "pa".to_owned()],
            because: "`pb` is picked because `look` is `b`, and `pa` because `look` is `a`"
                .to_owned(),
        }
    );
    assert_eq!(
        error.to_string(),
        "parts `pb` and `pa` can't be in one game: `pb` is picked because `look` is `b`, and \
         `pa` because `look` is `a`"
    );
}

/// The fixture with `extra` files beside its own.
fn with_files(extra: &[(&str, &str)]) -> Template {
    let mut all = fixture_files(MANIFEST_OK);
    all.extend(files(extra));
    Template::read("fixtures/t", all, &engine()).unwrap()
}

#[test]
fn two_parts_writing_one_unmerged_file_fail_naming_both() {
    let template = with_files(&[("base/notes.md", "one\n"), ("pa/notes.md", "two\n")]);
    let error = composed(&template, &Answers::default()).unwrap_err();
    assert_eq!(
        error,
        ComposeError::SameFile {
            path: "notes.md".to_owned(),
            parts: vec!["base".to_owned(), "pa".to_owned()],
        }
    );
    assert_eq!(
        error.to_string(),
        "notes.md is written by the parts `base` and `pa`, and only a Cargo.toml or a .ron file \
         is merged"
    );
}

#[test]
fn cargo_tomls_merge_key_by_key_with_their_comments() {
    let template = with_files(&[(
        "pa/Cargo.toml",
        "[workspace.dependencies]\n# The rules' own.\nserde = \"1\"\nshuyi = { path = \"x\" }\n\n\
         [workspace]\nmembers = [\"crates/*\"]\n",
    )]);
    let plan = composed(&template, &Answers::default()).unwrap();
    assert_eq!(
        text_of(&plan, "Cargo.toml"),
        "[workspace]\nmembers = [\"crates/*\"]\n[workspace.dependencies]\nshuyi = { path = \"x\" \
         }\n# The rules' own.\nserde = \"1\"\n"
    );
    let clash = with_files(&[(
        "pa/Cargo.toml",
        "[workspace.dependencies]\nshuyi = { path = \"y\" }\n",
    )]);
    let error = composed(&clash, &Answers::default()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "Cargo.toml, written by the parts `base` and `pa`, doesn't merge at \
         `workspace.dependencies.shuyi.path`: `\"x\"` in one, `\"y\"` in another"
    );
    let lists =
        compose::merge_toml("a = [1, 2]\n[t]\nk = 1\n", "a = [2, 3]\n[t]\nj = 2\n").unwrap();
    assert_eq!(lists, "a = [1, 2, 3]\n[t]\nk = 1\nj = 2\n");
    assert!(compose::merge_toml("a = 1\n", "a = \"1\"\n").is_err());
    // Arrays of tables gain the tables they lack, once.
    let bins = compose::merge_toml(
        "[[bin]]\nname = \"a\"\n",
        "[[bin]]\nname = \"a\"\n\n[[bin]]\nname = \"b\"\n",
    )
    .unwrap();
    assert_eq!(bins, "[[bin]]\nname = \"a\"\n\n[[bin]]\nname = \"b\"\n");
    assert!(compose::merge_toml("a = [\n", "").is_err());
}

#[test]
fn ron_files_merge_and_a_clash_names_the_field() {
    let template = with_files(&[("pa/shuyi.ron", "(\n    rules: \"r\",\n)\n")]);
    let plan = composed(&template, &Answers::default()).unwrap();
    assert_eq!(
        text_of(&plan, "shuyi.ron"),
        "(\n    command: \"x\",\n    rules: \"r\",\n)\n"
    );
    let clash = with_files(&[("pa/shuyi.ron", "(command: \"y\")")]);
    let error = composed(&clash, &Answers::default()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "shuyi.ron, written by the parts `base` and `pa`, doesn't merge at `command`: `\"x\"` \
         in one, `\"y\"` in another"
    );
    let not_text = with_files(&[("pa/shuyi.ron", "(\n")]);
    assert!(matches!(
        composed(&not_text, &Answers::default()),
        Err(ComposeError::Merge { .. })
    ));
}

#[test]
fn placeholders_fill_in_text_and_names_and_actions_expressions_pass() {
    let picked = answers(&[("look", &["a", "b"]), ("one", &[])]);
    let with = filled();
    let text = "{{name}} / {{crate_name}} / shuyi = \"{{engine.shuyi}}\" / {{answer.look}} / \
                {{answer.one}} / ${{ matrix.target }} / ${{github.sha}}";
    assert_eq!(
        fill("f", text, &with, &picked).unwrap(),
        "Moon Garden / moon-garden / shuyi = { path = 'e' } / a,b /  / ${{ matrix.target }} / \
         ${{github.sha}}"
    );
    let unknown = fill("a.rs", "one\ntwo {{nope}} three", &with, &picked).unwrap_err();
    assert_eq!(
        unknown.to_string(),
        "the template's a.rs:2 has an unknown placeholder {{nope}}"
    );
    for bad in [
        "{{crate}}",
        "{{answer.size}}",
        "{{engine.shuyi}}",
        "\"{{engine.core}}\"",
        "{{",
    ] {
        let error = fill("f", bad, &with, &picked).unwrap_err();
        assert!(
            matches!(error, ComposeError::Placeholder { line: 1, .. }),
            "{bad}: {error}"
        );
    }
    let template = with_files(&[("pa/{{crate_name}}.txt", "")]);
    let plan = composed(&template, &Answers::default()).unwrap();
    assert!(plan.files.contains_key(Path::new("moon-garden.txt")));
    let template = with_files(&[("pa/{{what}}.txt", "")]);
    let error = composed(&template, &Answers::default()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the template's pa/{{what}}.txt:1 has an unknown placeholder {{what}}"
    );
}

#[test]
fn a_parts_modules_are_turned_on_through_adds_code() {
    let manifest = MANIFEST_OK.replace(
        "files: \"pa\"",
        "files: \"pa\", modules: [\"alpha\", \"shuyi-alpha\"]",
    );
    let template = fixture(&manifest).unwrap();
    let plan = composed(&template, &Answers::default()).unwrap();
    assert_eq!(plan.modules, ["shuyi-alpha"]);
    assert_eq!(
        text_of(&plan, "Cargo.toml"),
        "[workspace.dependencies]\nshuyi = { path = \"x\", features = [\"alpha\"] }\n"
    );
    let planned = fixture(&manifest.replace("\"shuyi-alpha\"", "\"gamma\"")).unwrap();
    let error = composed(&planned, &Answers::default()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "part `pa` turns on `gamma`: shuyi-gamma is in the catalog but not built yet: soft \
         bodies is planned (9)"
    );
    let mut no_manifest = fixture(&manifest).unwrap();
    no_manifest.files.remove("base/Cargo.toml");
    let error = composed(&no_manifest, &Answers::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .ends_with("the game has no Cargo.toml to turn it on in"),
        "{error}"
    );
}

/// The fixture with a stage on `pa` (and on `pb` if `two`), and `extra`
/// files.
fn staged(two: bool, extra: &[(&str, &str)]) -> Template {
    let mut manifest =
        MANIFEST_OK.replace("files: \"pa\"", "files: \"pa\", stage: Some(\"side-view\")");
    if two {
        manifest = manifest
            .replace(
                "files: \"pb/\"",
                "files: \"pb/\", stage: Some(\"top-down\")",
            )
            .replace("conflicts: [\"pa\"]", "conflicts: []");
    }
    let mut all = fixture_files(&manifest);
    all.extend(files(extra));
    Template::read("fixtures/t", all, &engine()).unwrap()
}

#[test]
fn a_stage_joins_the_generated_list_of_stages() {
    let module = ("pa/crates/game/src/stages/side_view.rs", "//! A stage.\n");
    let template = staged(false, &[module]);
    let plan = composed(&template, &Answers::default()).unwrap();
    assert_eq!(plan.stages, ["side-view"]);
    let list = text_of(&plan, GENERATED_STAGES);
    assert_eq!(list, stages_rs(&["side-view".to_owned()]));
    for line in [
        "pub mod side_view;\n",
        "&[\"side-view\"]",
        "pub type Own = side_view::Own;",
        "Some(side_view::first(ctx))",
        "pub fn tools() -> side_view::Tools {",
    ] {
        assert!(list.contains(line), "{line}: {list}");
    }
    // With the stage's part left out, the list is empty and still written.
    let none = composed(&template, &answers(&[("look", &[])])).unwrap();
    assert!(none.stages.is_empty(), "{:?}", none.stages);
    let list = text_of(&none, GENERATED_STAGES);
    assert_eq!(list, stages_rs(&[]));
    for line in [
        "pub type Own = ();",
        "    None\n",
        "shuyi_cli::BasicTools::new(crate::game())",
    ] {
        assert!(list.contains(line), "{line}: {list}");
    }
    // A template with no stage at all has no list.
    let stageless = fixture(MANIFEST_OK).unwrap();
    let plan = composed(&stageless, &Answers::default()).unwrap();
    assert!(!plan.files.contains_key(Path::new(GENERATED_STAGES)));
}

#[test]
fn a_stage_the_flow_cant_take_fails() {
    let error = composed(&staged(false, &[]), &Answers::default()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the stages of `pa`: the part adds the stage `side-view` but writes no \
         crates/game/src/stages/side_view.rs"
    );
    let both = answers(&[("look", &["a", "b"])]);
    let error = composed(&staged(true, &[]), &both).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the stages of `pa` and `pb`: each adds a stage, and a game has one stage for now: \
         pick one"
    );
    let writes_list = staged(false, &[("base/crates/game/src/stages.rs", "")]);
    let error = composed(&writes_list, &Answers::default()).unwrap_err();
    assert_eq!(
        error.to_string(),
        "the stages of `base`: crates/game/src/stages.rs is written by shuyi new from the \
         parts' stages, not by a part"
    );
}

#[test]
fn a_part_built_in_code_with_a_folder_outside_the_template_fails() {
    let mut template = fixture(MANIFEST_OK).unwrap();
    template.parts[1].files = PathBuf::from("../elsewhere");
    let error = composed(&template, &Answers::default()).unwrap_err();
    assert!(
        error
            .to_string()
            .starts_with("part `pa`: `../elsewhere` is no folder"),
        "{error}"
    );
}

#[test]
fn any_answers_over_the_fixture_give_a_plan_or_an_error_and_the_same_twice() {
    let template = fixture(MANIFEST_OK).unwrap();
    let question = prop::sample::select(vec!["look", "one", "spare", "size"]);
    let answer = prop::sample::select(vec!["a", "b", "x", "y", "unserved", "c"]);
    let sets = prop::collection::btree_map(
        question.prop_map(str::to_owned),
        prop::collection::vec(answer.prop_map(str::to_owned), 0..3),
        0..4,
    );
    proptest!(|(set in sets)| {
        let picked = Answers(set);
        let first = composed(&template, &picked);
        prop_assert_eq!(first, composed(&template, &picked));
    });
}

#[test]
fn any_bytes_as_a_manifest_never_panic() {
    proptest!(|(bytes in prop::collection::vec(any::<u8>(), 0..300))| {
        let all = Files::from([(MANIFEST.to_owned(), bytes)]);
        let _ = Template::read("t", all, &engine());
    });
    let pieces = prop::collection::vec(
        prop::sample::select(vec![
            "Template(",
            "Part(",
            "Question(",
            ")",
            "[",
            "]",
            "{",
            "}",
            ",",
            "id: \"a\"",
            "when: Always",
            "Picked(\"q\", \"a\")",
            "files: \"base\"",
            "format_version: 1",
            "engine: \">=0\"",
            "version: \"1.0.0\"",
            "parts: [",
            "questions: [",
        ]),
        0..30,
    );
    proptest!(|(manifest in pieces)| {
        let _ = fixture(&manifest.concat());
    });
}

#[test]
fn apply_writes_into_an_empty_folder_and_over_one_only_with_force() {
    let dir = std::env::temp_dir().join(format!("shuyi-apply-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let template = fixture(MANIFEST_OK).unwrap();
    let plan = composed(&template, &Answers::default()).unwrap();
    let applied = apply(&plan, &dir, false).unwrap();
    assert_eq!(applied.written, plan.paths());
    assert!(applied.overwritten.is_empty(), "{:?}", applied.overwritten);
    assert_eq!(
        std::fs::read_to_string(dir.join("a.txt")).unwrap(),
        "a of moon-garden\n"
    );
    let refused = apply(&plan, &dir, false).unwrap_err();
    assert_eq!(
        refused,
        "is not empty (it would write over Cargo.toml, README.md, a.txt, shuyi.ron): pass \
         --force to write into it"
    );
    std::fs::write(dir.join("a.txt"), "changed").unwrap();
    let forced = apply(&plan, &dir, true).unwrap();
    assert_eq!(forced.overwritten, plan.paths());
    assert_eq!(
        std::fs::read_to_string(dir.join("a.txt")).unwrap(),
        "a of moon-garden\n"
    );
    std::fs::remove_dir_all(&dir).unwrap();
}

/// FNV-1a, 64 bits: a hash of `bytes` that never changes.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &b| {
        (hash ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// The engine's own template's plan for its short name `name`, a game
/// called Moon Garden on the engine's `main`.
fn built_in_plan(name: &str) -> Plan {
    let template = built_in().unwrap_or_else(|e| panic!("{e}"));
    let answers = template.short_name(name).unwrap().clone();
    let fill = crate::contract::new::fill_for(
        "Moon Garden",
        "moon-garden",
        &crate::contract::new::Engine::Main,
        None,
    );
    compose(&template, &answers, &Catalog::embedded().unwrap(), &fill)
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The golden file of the engine's own template's plans.
const GOLDEN: &str = "src/template/plans.golden";

#[test]
fn the_engine_templates_plans_are_golden() {
    let mut text = String::from(
        "# The engine's own template's plans for Moon Garden on the engine's main: each\n\
         # file's FNV-1a hash and path. Written by `SHUYI_BLESS=1 cargo test -p shuyi-cli\n\
         # --lib template`, after a change to templates/shuyi-default.\n",
    );
    for name in ["blank", "grid"] {
        let plan = built_in_plan(name);
        assert_eq!(plan, built_in_plan(name), "{name} twice");
        // Writing to a `String` can't fail.
        let _ = write!(text, "\n[{name}] {}\n", plan.parts.join(", "));
        for (path, bytes) in &plan.files {
            let shown = path.to_string_lossy().replace('\\', "/");
            let _ = writeln!(text, "{:016x} {shown}", fnv(bytes));
        }
    }
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(GOLDEN);
    if std::env::var_os("SHUYI_BLESS").is_some() {
        std::fs::write(&file, &text).unwrap();
    }
    let golden = std::fs::read_to_string(&file)
        .unwrap_or_default()
        .replace("\r\n", "\n");
    assert!(
        golden == text,
        "{GOLDEN} is not the plans the template gives: if the change is meant, write it again \
         with `SHUYI_BLESS=1 cargo test -p shuyi-cli --lib template`. The plans:\n{text}"
    );
}

#[test]
fn the_engine_template_has_blank_and_grid_and_a_stage_for_grid() {
    let template = built_in().unwrap();
    assert_eq!(template.id, "shuyi-default");
    let names: Vec<&str> = template.short_names.keys().map(String::as_str).collect();
    assert_eq!(names, ["blank", "grid"]);
    let blank = built_in_plan("blank");
    assert_eq!(blank.parts, ["base"]);
    assert!(blank.stages.is_empty() && blank.limits.is_empty());
    let grid = built_in_plan("grid");
    assert_eq!(grid.parts, ["base", "stage-top-down-grid"]);
    assert_eq!(grid.stages, ["top-down-grid"]);
    assert_eq!(grid.targets, ["desktop", "web"]);
    for path in [
        "crates/rules/src/lib.rs",
        "crates/game/src/stages/top_down_grid.rs",
        "crates/game/files/stage.map",
    ] {
        assert!(grid.files.contains_key(Path::new(path)), "{path}");
        assert!(!blank.files.contains_key(Path::new(path)), "{path}");
    }
    let config = crate::project::Config::parse(&text_of(&grid, "shuyi.ron")).unwrap();
    assert_eq!(config.rules.as_deref(), Some("moon-garden-rules"));
    for plan in [&blank, &grid] {
        let config = crate::project::Config::parse(&text_of(plan, "shuyi.ron")).unwrap();
        let ids: Vec<&str> = config.targets.keys().map(String::as_str).collect();
        assert_eq!(ids, ["desktop", "web"]);
        for (path, bytes) in &plan.files {
            if let Ok(text) = std::str::from_utf8(bytes) {
                let left = text.replace("${{", "");
                assert!(!left.contains("{{"), "{}", path.display());
            }
        }
    }
}
