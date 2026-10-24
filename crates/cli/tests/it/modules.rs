//! `modules` and `add` on a game made by `new`: on an engine checkout
//! with two modules that are alternatives (`shuyi-alpha` and
//! `shuyi-beta`, made here), and on this repository, which builds none
//! yet.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use shuyi_cli::contract::add::{AddInput, AddOutput, BESIDE, add};
use shuyi_cli::contract::modules::{ModulesInput, ModulesOutput, modules};
use shuyi_cli::contract::new::{Engine, NewInput, new};
use shuyi_cli::contract::{Answer, Op, op_schemas};
use shuyi_cli::project::Project;

use crate::scratch;

/// `files` written under `dir`.
fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let to = dir.join(path);
        let folder = to.parent().unwrap_or(dir);
        fs::create_dir_all(folder).unwrap_or_else(|e| panic!("{}: {e}", folder.display()));
        fs::write(&to, text).unwrap_or_else(|e| panic!("{}: {e}", to.display()));
    }
}

const CONCERNS: &str = r#"(concerns: [
    (id: "bodies", name: "Bodies and motion", alternatives: true, answers: [
        (id: "movers", name: "movers", by: "default", status: ready, tickets: []),
        (id: "rigid-body", name: "rigid-body physics", by: "shuyi-alpha", status: ready, tickets: ["1"]),
        (id: "procedural", name: "procedural animation", by: "shuyi-beta", status: ready, tickets: ["2"]),
        (id: "soft-body", name: "soft bodies", by: "shuyi-gamma", status: parked, tickets: ["3", "4"]),
    ]),
])"#;

/// An engine checkout at `dir`: its concerns, a default crate, and two
/// modules that are alternatives, `shuyi-alpha` with a skill and
/// `shuyi-beta` with formats.
fn engine(dir: &Path) {
    let module = |name: &str, answer: &str, other: &str, more: &str| {
        format!(
            "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[package.metadata.shuyi]\n\
             answers = [{{ concern = \"bodies\", answer = \"{answer}\" }}]\n\
             serves = [\"side-view\"]\nalternatives = [\"{other}\"]\n{more}"
        )
    };
    write(
        dir,
        &[
            ("docs/modules.ron", CONCERNS),
            (
                "crates/core/Cargo.toml",
                "[package]\nname = \"shuyi-core\"\n",
            ),
            (
                "crates/alpha/Cargo.toml",
                &module("shuyi-alpha", "rigid-body", "shuyi-beta", ""),
            ),
            (
                "crates/alpha/README.md",
                "# shuyi-alpha\n\nRigid bodies that push and stack,\nfor side-view games.\n\n\
                 ## Beside its alternatives\n\nA crate pushed by a creature.\n",
            ),
            (
                "crates/alpha/skill/SKILL.md",
                "---\nname: shuyi-alpha\n---\n",
            ),
            (
                "crates/beta/Cargo.toml",
                &module(
                    "shuyi-beta",
                    "procedural",
                    "shuyi-alpha",
                    "formats = \"shuyi_beta::formats\"\n",
                ),
            ),
            (
                "crates/beta/README.md",
                "# shuyi-beta\n\nBodies of points and rules.\n",
            ),
        ],
    );
}

/// A game made by `new` from `template` in `root/game`, on the engine at
/// `engine`.
fn game(root: &Path, template: &str, engine: &Path) -> Project {
    let mut input = NewInput::new("game").template(template);
    input.engine = Engine::Path {
        path: engine.to_string_lossy().into_owned(),
    };
    let made = new(root, &input);
    assert!(made.ok, "{made:?}");
    as_its_schema_says(Op::New, "NewOutput", &made);
    Project::load(&root.join("game")).unwrap_or_else(|e| panic!("{e:?}"))
}

/// The names of `value`'s fields.
fn keys(value: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = value
        .as_object()
        .map(|o| o.keys().map(String::as_str).collect())
        .unwrap_or_default();
    keys.sort_unstable();
    keys
}

/// `answer` as JSON, with the fields of `op`'s output schema, `output`
/// its result's type; and read back as the answer it is.
fn as_its_schema_says<T>(op: Op, output: &str, answer: &Answer<T>)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let [_, schema] = op_schemas(op, false);
    let json = |text: &str| -> serde_json::Value {
        serde_json::from_str(text).unwrap_or_else(|e| panic!("{e}: {text}"))
    };
    let schema = json(&schema.text);
    let answered = json(&answer.to_json());
    assert_eq!(keys(&answered), keys(&schema["properties"]));
    assert_eq!(
        keys(&answered["result"]),
        keys(&schema["$defs"][output]["properties"])
    );
    let read: Answer<T> = serde_json::from_value(answered).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(&read, answer);
}

/// `modules` on `project`, which passes as its schema says.
fn modules_of(project: &Project) -> ModulesOutput {
    let answer = modules(project, &ModulesInput::default());
    as_its_schema_says(Op::Modules, "ModulesOutput", &answer);
    answer
        .result
        .unwrap_or_else(|| panic!("modules failed: {:?}", answer.error))
}

/// `add` of `module` on `project`, which passes as its schema says.
fn added(project: &Project, module: &str) -> AddOutput {
    let answer = add(project, &AddInput::new(module));
    as_its_schema_says(Op::Add, "AddOutput", &answer);
    answer
        .result
        .unwrap_or_else(|| panic!("add failed: {:?}", answer.error))
}

#[test]
fn add_turns_a_module_on_once_and_modules_lists_it() {
    let root = scratch("modules-add");
    let checkout = root.join("engine");
    engine(&checkout);
    let project = game(&root, "blank", &checkout);
    let game = project.root.clone();
    fs::create_dir_all(game.join(".claude/skills")).unwrap();

    let before = modules_of(&project);
    assert_eq!(
        PathBuf::from(&before.engine),
        std::path::absolute(&checkout).unwrap()
    );
    let names: Vec<&str> = before.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["shuyi-alpha", "shuyi-beta"]);
    assert!(before.on.is_empty() && before.modules.iter().all(|m| !m.on));
    assert_eq!(before.concerns[0].answers.len(), 4);

    let first = added(&project, "alpha");
    assert!(first.changed);
    assert_eq!(first.module, "shuyi-alpha");
    assert_eq!(first.feature, "alpha");
    assert_eq!(
        first.about,
        "Rigid bodies that push and stack,\nfor side-view games."
    );
    assert_eq!(first.skill.as_deref(), Some(".claude/skills/shuyi-alpha"));
    assert!(game.join(".claude/skills/shuyi-alpha/SKILL.md").is_file());
    assert!(first.alternatives_on.is_empty() && first.beside.is_none());
    // The blank template installs `shuyi::enabled_modules` already.
    assert_eq!(first.install, None);
    assert_eq!(first.formats, None);
    let manifest = fs::read_to_string(game.join("Cargo.toml")).unwrap();
    assert_eq!(
        manifest.matches("features = [\"alpha\"]").count(),
        1,
        "{manifest}"
    );

    // Twice: nothing changes, and it says so.
    let second = added(&project, "shuyi-alpha");
    assert!(!second.changed);
    assert_eq!(second.skill, None);
    assert_eq!(
        fs::read_to_string(game.join("Cargo.toml")).unwrap(),
        manifest
    );
    let answer = add(&project, &AddInput::new("alpha"));
    assert!(
        answer
            .limits
            .iter()
            .any(|l| l.contains("already has a skill")),
        "{:?}",
        answer.limits
    );

    let after = modules_of(&project);
    assert_eq!(after.on, ["shuyi-alpha"]);
    assert!(after.modules[0].on && !after.modules[1].on);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn add_of_a_module_whose_alternative_is_on_warns_and_links_the_readme() {
    let root = scratch("modules-alternative");
    let checkout = root.join("engine");
    engine(&checkout);
    let project = game(&root, "grid", &checkout);
    assert!(added(&project, "alpha").changed);

    let answer = add(&project, &AddInput::new("beta"));
    let beta = answer.result.clone().unwrap();
    assert!(beta.changed);
    assert_eq!(beta.alternatives_on, ["shuyi-alpha"]);
    let beside = beta.beside.as_deref().unwrap();
    assert!(
        beside.ends_with(&format!("crates/beta/README.md#{BESIDE}")),
        "{beside}"
    );
    let text = answer.to_text(shuyi_cli::contract::add::text);
    assert!(
        text.contains("warning: the game already has shuyi-alpha on, an alternative"),
        "{text}"
    );
    assert!(text.contains(beside), "{text}");
    // Its formats are the game's call; it has no skill, and the game
    // keeps none.
    assert!(beta.formats.unwrap().contains("`shuyi_beta::formats`"));
    assert!(
        answer
            .limits
            .iter()
            .any(|l| l == "shuyi-beta has no skill to copy"),
        "{:?}",
        answer.limits
    );
    let manifest = fs::read_to_string(project.root.join("Cargo.toml")).unwrap();
    assert!(
        manifest.contains("features = [\"alpha\", \"beta\"]"),
        "{manifest}"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_game_that_never_installs_modules_is_given_the_line() {
    let root = scratch("modules-install");
    let checkout = root.join("engine");
    engine(&checkout);
    let project = game(&root, "blank", &checkout);
    let lib = project.root.join("crates/game/src/lib.rs");
    let text = fs::read_to_string(&lib).unwrap();
    assert!(text.contains(".modules(shuyi::enabled_modules)"), "{text}");
    fs::write(&lib, text.replace(".modules(shuyi::enabled_modules)", "")).unwrap();
    let alpha = added(&project, "alpha");
    let install = alpha.install.unwrap();
    assert!(
        install.contains("`.modules(shuyi::enabled_modules)`")
            && install.contains("`.with_modules(shuyi::enabled_modules())`"),
        "{install}"
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn an_unknown_name_lists_the_known_ones_and_a_planned_one_says_so() {
    let root = scratch("modules-unknown");
    let checkout = root.join("engine");
    engine(&checkout);
    let project = game(&root, "blank", &checkout);
    let before = fs::read_to_string(project.root.join("Cargo.toml")).unwrap();
    for (name, error) in [
        (
            "nope",
            "no module nope: the game's engine builds shuyi-alpha, shuyi-beta",
        ),
        (
            "gamma",
            "shuyi-gamma is in the catalog but not built yet: soft bodies is parked (3, 4)",
        ),
    ] {
        let answer = add(&project, &AddInput::new(name));
        assert!(!answer.ok);
        assert_eq!(answer.code.as_deref(), Some("SH0003"));
        assert_eq!(answer.error.as_deref(), Some(error));
    }
    assert_eq!(
        fs::read_to_string(project.root.join("Cargo.toml")).unwrap(),
        before
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn on_this_repository_the_catalog_is_its_own_and_no_module_is_built_yet() {
    let root = scratch("modules-repository");
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = game(&root, "grid", &repo);
    let catalog = modules_of(&project);
    let concerns: Vec<&str> = catalog.concerns.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        concerns,
        [
            "time",
            "perspective",
            "bodies",
            "online",
            "world-drawing",
            "kind"
        ]
    );
    assert!(catalog.modules.is_empty() && catalog.on.is_empty());
    let answer = add(&project, &AddInput::new("physics"));
    assert_eq!(answer.code.as_deref(), Some("SH0003"));
    assert_eq!(
        answer.error.as_deref(),
        Some(
            "shuyi-physics is in the catalog but not built yet: rigid-body physics is parked (1128)"
        )
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_binary_answers_modules_and_add_with_their_exit_codes() {
    let root = scratch("modules-binary");
    let checkout = root.join("engine");
    engine(&checkout);
    let project = game(&root, "blank", &checkout);
    let shuyi = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_shuyi"))
            .args(args)
            .current_dir(&project.root)
            .output()
            .unwrap()
    };
    let output = shuyi(&["modules", "--json"]);
    assert_eq!(output.status.code(), Some(0));
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["op"], "modules");
    assert_eq!(json["result"]["modules"][0]["name"], "shuyi-alpha");
    assert_eq!(shuyi(&["modules", "x"]).status.code(), Some(2));
    assert_eq!(shuyi(&["add"]).status.code(), Some(2));
    assert_eq!(shuyi(&["add", "nope"]).status.code(), Some(2));
    let output = shuyi(&["add", "alpha"]);
    assert_eq!(output.status.code(), Some(0));
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.starts_with("add: shuyi-alpha on"), "{text}");
    let output = shuyi(&["add", "alpha"]);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.starts_with("add: shuyi-alpha was already on: nothing changed"),
        "{text}"
    );
    let output = shuyi(&["modules"]);
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        text.contains("rigid-body physics: shuyi-alpha, ready (on)"),
        "{text}"
    );
    assert!(text.contains("shuyi-alpha (on): feature `alpha`"), "{text}");
    fs::remove_dir_all(&root).unwrap();
}
