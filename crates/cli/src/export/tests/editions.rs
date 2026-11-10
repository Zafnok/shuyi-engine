//! Editions (ADR-0100): `shuyi.ron`'s `editions` parsed and chosen, their
//! builds' flags and refusals, and two editions of a stand-in game built by
//! the real Cargo and packaged by the real exporters, whose final packages
//! are read back for what only the full game has.
//!
//! The stand-in has no dependency: its build script embeds the files of its
//! content root (`SHUYI_CONTENT`, else `files`) as the template's
//! `include_dir!` does, and its `full` feature, on by default, adds a line of
//! code only the full game has.

use std::collections::BTreeMap;

use shuyi_assets::pack::{self as assets_pack, Content, PackInfo};

use super::super::live_link::{Node, refuse, unwanted};
use super::super::{CONTENT, EditionBuild, ExportCx, desktop, find, playdate, web};
use super::*;
use crate::contract::export::{ExportInput, export, parse_args};
use crate::project::{Config, Project};

/// The texts only the full game holds: a future map, a line of dialogue, a
/// line of code, and a file of a pack.
const MAP: &str = "EDITIONS-FUTURE-MAP-CANARY";
const DIALOGUE: &str = "EDITIONS-DIALOGUE-CANARY";
const CODE: &str = "EDITIONS-FULL-CODE-CANARY";
const PACKED: &str = "EDITIONS-PACKED-CANARY";
const CANARIES: [&str; 4] = [MAP, DIALOGUE, CODE, PACKED];

/// The project file: a public demo held to the canaries, and the full game.
const PROJECT: &str = r#"(
    targets: {
        "desktop": (frame: (384, 384), scale: integer, text_px: 16),
        "web": (frame: (384, 384), scale: integer, text_px: 16),
    },
    export: (package: "game", features: ["packaged"]),
    web: (package: "game", bin: "game", title: "Game", dist: "dist/web"),
    editions: {
        "public-demo": (
            default_features: false,
            features: ["demo"],
            never_features: ["full"],
            content: "game/files-demo",
            beside: [(path: "README-demo.txt", name: "README.txt")],
            disclosure: (
                allowed_files: ["README.txt"],
                allowed_pack_entries: ["first.ron"],
                forbidden_paths: ["maps/future.ron"],
                canaries: [
                    "EDITIONS-FUTURE-MAP-CANARY",
                    "EDITIONS-DIALOGUE-CANARY",
                    "EDITIONS-FULL-CODE-CANARY",
                    "EDITIONS-PACKED-CANARY",
                ],
            ),
        ),
        "full": (default_features: true, content: "game/files"),
        "writer-preview": (default_features: false, bin: "preview", content: "game/files-demo"),
    },
)"#;

/// The game's manifest below `[package]`.
const MANIFEST: &str = "[features]\ndefault = [\"full\"]\nfull = []\ndemo = []\npackaged = []\n";

/// Its build script: the files of its content root, embedded, as the
/// template's `include_dir!` embeds them.
const BUILD: &str = r#"use std::path::{Path, PathBuf};
fn walk(dir: &Path, root: &Path, out: &mut Vec<(String, PathBuf)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            walk(&path, root, out);
        } else {
            let name = path.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.push((name, path));
        }
    }
}
fn main() {
    println!("cargo:rerun-if-env-changed=SHUYI_CONTENT");
    let root = std::env::var("SHUYI_CONTENT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| Path::new(&std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("files"));
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    walk(&root, &root, &mut files);
    let mut code = String::from("pub static FILES: &[(&str, &[u8])] = &[\n");
    for (name, path) in files {
        code.push_str(&format!("    ({name:?}, include_bytes!({:?})),\n", path.display().to_string()));
    }
    code.push_str("];\n");
    let out = Path::new(&std::env::var("OUT_DIR").unwrap()).join("content.rs");
    std::fs::write(out, code).unwrap();
}
"#;

/// Its program: it lists its files and their bytes, and the full game says
/// its line of code too.
const MAIN: &str = r#"include!(concat!(env!("OUT_DIR"), "/content.rs"));

#[cfg(feature = "full")]
static NEXT_CHAPTER: &str = "EDITIONS-FULL-CODE-CANARY";

fn main() {
    let mut bytes = 0;
    for (name, data) in std::hint::black_box(FILES) {
        println!("{name}");
        bytes += data.len();
    }
    #[cfg(feature = "full")]
    println!("{}", std::hint::black_box(NEXT_CHAPTER));
    println!("{bytes} bytes");
}
"#;

fn write(path: &Path, bytes: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

/// A pack of `files`.
fn pack_of(files: &[(&str, &str)]) -> Vec<u8> {
    let info = PackInfo {
        stamp: "the game 1".to_owned(),
        ..PackInfo::default()
    };
    let files: BTreeMap<String, Content> = files
        .iter()
        .map(|(path, text)| ((*path).to_owned(), Content::Raw(text.as_bytes().to_vec())))
        .collect();
    assets_pack::write(&info, &files)
}

/// The stand-in game's workspace under the temporary folder, for test
/// `name`, fresh: its full files in `game/files` and the demo's in
/// `game/files-demo`.
fn game(name: &str) -> Project {
    let root = std::env::temp_dir().join(format!("shuyi-editions-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(
        &root.join("Cargo.toml"),
        b"[workspace]\nresolver = \"2\"\nmembers = [\"game\"]\n",
    );
    let manifest = format!(
        "[package]\nname = \"game\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n\
         {MANIFEST}"
    );
    write(&root.join("game/Cargo.toml"), manifest.as_bytes());
    write(&root.join("game/build.rs"), BUILD.as_bytes());
    write(&root.join("game/src/main.rs"), MAIN.as_bytes());
    write(&root.join("game/src/bin/preview.rs"), MAIN.as_bytes());
    let full = root.join("game/files");
    write(&full.join("maps/first.ron"), b"(name: \"the first map\")\n");
    write(
        &full.join("maps/future.ron"),
        format!("(name: \"{MAP}\")\n").as_bytes(),
    );
    write(
        &full.join("dialogue/act2.dlg"),
        format!("= act two\n{DIALOGUE}\n").as_bytes(),
    );
    let pack = pack_of(&[
        ("first.ron", "the first map"),
        ("future/secret.ron", PACKED),
    ]);
    write(&full.join("game.shuyipak"), &pack);
    let demo = root.join("game/files-demo");
    write(&demo.join("maps/first.ron"), b"(name: \"the first map\")\n");
    write(
        &demo.join("game.shuyipak"),
        &pack_of(&[("first.ron", "the first map")]),
    );
    write(&root.join("README-demo.txt"), b"A demo of the game.\n");
    write(&root.join("shuyi.ron"), PROJECT.as_bytes());
    Project::load(&root).unwrap()
}

fn input(target: &str, edition: Option<&str>) -> ExportInput {
    ExportInput {
        target: target.to_owned(),
        edition: edition.map(str::to_owned),
        ..ExportInput::default()
    }
}

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|&item| item.to_owned()).collect()
}

#[test]
fn an_edition_is_parsed_and_always_says_its_default_features() {
    let config = Config::parse(PROJECT).unwrap();
    let demo = &config.editions["public-demo"];
    assert!(!demo.default_features);
    assert_eq!(demo.features, ["demo"]);
    assert_eq!(demo.never_features, ["full"]);
    assert_eq!(demo.content.as_deref(), Some("game/files-demo"));
    assert_eq!(demo.beside.as_ref().map(Vec::len), Some(1));
    let part = demo.disclosure.as_ref().unwrap();
    assert_eq!(part.canaries.len(), 4);
    assert!(config.editions["full"].default_features);
    assert_eq!(config.editions["full"].disclosure, None);
    let unsaid = Config::parse(r#"(editions: {"demo": (features: ["demo"])})"#).unwrap_err();
    assert!(unsaid.contains("default_features"), "{unsaid}");
    let unknown =
        Config::parse(r#"(editions: {"demo": (default_features: true, files: "x")})"#).unwrap_err();
    assert!(unknown.contains("files"), "{unknown}");
    assert_eq!(
        parse_args(&args(&["web", "--edition", "public-demo"]))
            .unwrap()
            .edition
            .as_deref(),
        Some("public-demo")
    );
    assert!(parse_args(&args(&["web", "--edition"])).is_err());
    assert!(parse_args(&args(&["web", "--edition", "a", "--edition", "b"])).is_err());
}

#[test]
fn an_edition_chooses_the_package_program_features_files_and_folder() {
    let project = game("cx");
    let demo = ExportCx::of(
        &project,
        &desktop::WINDOWS,
        &input("windows", Some("public-demo")),
    )
    .unwrap();
    assert_eq!(demo.turned_on, ["demo"]);
    assert!(!demo.default_features());
    assert_eq!(demo.content(), Some(project.root.join("game/files-demo")));
    assert_eq!(demo.beside[0].path, "README-demo.txt");
    assert_eq!(demo.base(), "game-0.1.0-public-demo-windows");
    assert_eq!(demo.shown, "dist/export/public-demo/windows");
    let build = demo.cargo_args("x86_64-pc-windows-gnu");
    assert!(
        build.contains(&"--no-default-features".to_owned()),
        "{build:?}"
    );
    assert!(
        build.windows(2).any(|w| w == ["--features", "demo"]),
        "{build:?}"
    );
    let web_build = web::WEB.cargo_args(
        &ExportCx::of(&project, &web::WEB, &input("web", Some("public-demo"))).unwrap(),
    );
    assert!(
        web_build[0].contains(&"--no-default-features".to_owned()),
        "{web_build:?}"
    );
    // The full game keeps its defaults, and turns on no feature of the
    // `export` part's, which its edition's list replaces.
    let full = ExportCx::of(&project, &desktop::WINDOWS, &input("windows", Some("full"))).unwrap();
    assert!(full.default_features() && full.turned_on.is_empty());
    assert!(
        !full
            .cargo_args("x86_64-pc-windows-gnu")
            .contains(&"--no-default-features".to_owned())
    );
    // With no edition, the game as `export` builds it.
    let plain = ExportCx::of(&project, &desktop::WINDOWS, &input("windows", None)).unwrap();
    assert_eq!(plain.base(), "game-0.1.0-windows");
    assert_eq!(
        (plain.edition, plain.turned_on),
        (None, args(&["packaged"]))
    );
    // `--without` leaves out what the edition lists, not the part's.
    let mut without = input("windows", Some("public-demo"));
    without.without = args(&["packaged"]);
    let refused = ExportCx::of(&project, &desktop::WINDOWS, &without).unwrap_err();
    assert!(
        refused
            .message()
            .contains("`editions.public-demo.features`"),
        "{refused:?}"
    );
    fs::remove_dir_all(&project.root).unwrap();
}

#[test]
fn an_edition_that_cant_be_built_as_it_says_is_refused_before_anything_is_built() {
    let mut project = game("refused");
    let unknown =
        ExportCx::of(&project, &desktop::WINDOWS, &input("windows", Some("demo"))).unwrap_err();
    assert_eq!(unknown.code(), codes::PROJECT_FILE);
    assert!(
        unknown
            .message()
            .contains("it has full, public-demo, writer-preview"),
        "{unknown:?}"
    );
    for (content, why) in [
        ("game/no-such-folder", "isn't a folder"),
        ("../outside", "from the project's root"),
        ("game\\files-demo", "from the project's root"),
    ] {
        let demo = project.config.editions.get_mut("public-demo").unwrap();
        demo.content = Some(content.to_owned());
        let refused = ExportCx::of(
            &project,
            &desktop::WINDOWS,
            &input("windows", Some("public-demo")),
        )
        .unwrap_err();
        assert!(refused.message().contains(why), "{content}: {refused:?}");
    }
    let demo = project.config.editions.get_mut("public-demo").unwrap();
    demo.content = None;
    demo.bin = Some("no-such-program".to_owned());
    let refused = ExportCx::of(
        &project,
        &desktop::WINDOWS,
        &input("windows", Some("public-demo")),
    )
    .unwrap_err();
    assert!(
        refused.message().contains("`no-such-program`"),
        "{refused:?}"
    );
    let demo = project.config.editions.get_mut("public-demo").unwrap();
    demo.bin = None;
    demo.features.push("full".to_owned());
    let refused = ExportCx::of(
        &project,
        &desktop::WINDOWS,
        &input("windows", Some("public-demo")),
    )
    .unwrap_err();
    assert!(
        refused.message().contains("`never_features`"),
        "{refused:?}"
    );
    // The Playdate builds no default features: an edition keeping them is
    // refused as it prepares. One with a content root goes on to its bake
    // (ADR-0101), which needs the game's tools.
    let playdate = find("playdate").unwrap();
    let mut cx = ExportCx::of(&project, &desktop::WINDOWS, &input("windows", None)).unwrap();
    for (edition, why) in [
        (EditionBuild::new("full", true), "default features"),
        (
            EditionBuild {
                content: Some("game/files".to_owned()),
                ..EditionBuild::new("demo", false)
            },
            "no `game` part",
        ),
    ] {
        cx.edition = Some(edition);
        cx.features = args(&["playdate"]);
        let refused = playdate.prepare(&cx, &project, None).unwrap_err();
        assert!(refused.message().contains(why), "{refused:?}");
    }
    fs::remove_dir_all(&project.root).unwrap();
}

fn node(depth: usize, name: &str, features: &[&str]) -> Node {
    Node {
        depth,
        name: name.to_owned(),
        features: args(features),
    }
}

#[test]
fn never_features_count_in_the_package_built_and_a_named_dependency() {
    let nodes = [
        node(0, "game", &["default", "full"]),
        node(1, "story", &["act-two"]),
        node(1, "other", &["full"]),
    ];
    let found = unwanted(&nodes, &args(&["full", "story/act-two", "story/act-three"]));
    let named: Vec<(&str, &str, usize)> = found
        .iter()
        .map(|u| (u.package.as_str(), u.feature.as_str(), u.path.len()))
        .collect();
    assert_eq!(named, [("game", "full", 1), ("story", "act-two", 2)]);
    assert_eq!(unwanted(&nodes, &args(&["demo"])), []);
}

#[test]
fn a_default_feature_never_turns_full_only_code_on_in_the_public_demo() {
    let mut project = game("defaults");
    let at = |project: &Project| {
        ExportCx::of(
            project,
            &desktop::WINDOWS,
            &input("windows", Some("public-demo")),
        )
        .unwrap()
    };
    // As written: the defaults off, so `full` is off as Cargo resolves it.
    assert_eq!(refuse(&at(&project), &desktop::WINDOWS), Ok(()));
    // Kept, they turn `full` on, which is refused, naming it.
    let demo = project.config.editions.get_mut("public-demo").unwrap();
    demo.default_features = true;
    let refused = refuse(&at(&project), &desktop::WINDOWS).unwrap_err();
    assert!(
        refused
            .message()
            .starts_with("the edition `public-demo` never turns on")
            && refused.message().contains("`game/full`"),
        "{refused:?}"
    );
    // So is a feature the edition turns on that turns `full` on.
    let manifest = project.root.join("game/Cargo.toml");
    let text = fs::read_to_string(&manifest).unwrap();
    fs::write(&manifest, text.replace("demo = []", "demo = [\"full\"]")).unwrap();
    let demo = project.config.editions.get_mut("public-demo").unwrap();
    demo.default_features = false;
    let refused = refuse(&at(&project), &desktop::WINDOWS).unwrap_err();
    assert!(refused.message().contains("`game/full`"), "{refused:?}");
    fs::remove_dir_all(&project.root).unwrap();
}

/// The desktop exporter of this system, if its Rust target is installed.
fn desktop_here() -> Option<&'static str> {
    let id = if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        return None;
    };
    let triple = find(id)?.triples()[0];
    let installed = std::process::Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .ok()?;
    String::from_utf8_lossy(&installed.stdout)
        .lines()
        .any(|line| line.trim() == triple)
        .then_some(id)
}

/// Every file of the package `answer` names, unzipped, with its bytes.
fn package_files(project: &Project, out: &str, package: &str) -> Vec<(String, Vec<u8>)> {
    let file = project.root.join(out).join(package);
    archive::read(&file, 1 << 30).unwrap()
}

/// Whether `bytes` hold `text`.
fn holds(bytes: &[u8], text: &str) -> bool {
    bytes
        .windows(text.len())
        .any(|window| window == text.as_bytes())
}

#[test]
fn two_editions_are_built_and_only_the_full_one_holds_what_isnt_out() {
    let project = game("built");
    let mut targets = vec!["web"];
    targets.extend(desktop_here());
    for target in targets {
        let mut held = BTreeMap::new();
        for edition in ["public-demo", "full"] {
            let answer = export(&project, &input(target, Some(edition)), None);
            assert!(answer.ok, "{target} {edition}: {:?}", answer.error);
            let output = answer.result.unwrap();
            assert_eq!(output.edition.as_deref(), Some(edition));
            let files = package_files(&project, &output.out, &output.package);
            let canaries: Vec<&str> = CANARIES
                .into_iter()
                .filter(|canary| files.iter().any(|(_, bytes)| holds(bytes, canary)))
                .collect();
            held.insert(edition, canaries);
            let exposure: disclosure::ExposureReport = serde_json::from_slice(
                &fs::read(project.root.join(&output.out).join(disclosure::REPORT)).unwrap(),
            )
            .unwrap();
            let inputs = &exposure.inputs;
            assert_eq!(inputs.edition.as_deref(), Some(edition), "{target}");
            assert_eq!(inputs.package, "game");
            let entries: Vec<&str> = exposure
                .pack_entries
                .iter()
                .map(|e| e.path.as_str())
                .collect();
            if edition == "full" {
                assert_eq!(inputs.content_root.as_deref(), Some("game/files"));
                assert!(inputs.default_features && inputs.features.contains(&"full".to_owned()));
                assert!(!exposure.declared);
                assert!(
                    entries.contains(&"future/secret.ron"),
                    "{target}: {entries:?}"
                );
            } else {
                assert_eq!(inputs.content_root.as_deref(), Some("game/files-demo"));
                assert!(!inputs.default_features, "{target}");
                assert_eq!(inputs.features, ["demo"], "{target}");
                assert!(exposure.declared && exposure.ok, "{target}");
                assert_eq!(entries, ["first.ron"], "{target}");
                assert!(files.iter().any(|(name, _)| name.ends_with("README.txt")));
            }
            if target != "web" {
                // It plays: the program runs, and lists the files it has.
                let program = project.root.join(&output.out).join(&output.sizes[1].file);
                let ran = std::process::Command::new(program).output().unwrap();
                assert!(ran.status.success(), "{target} {edition}");
                let said = String::from_utf8_lossy(&ran.stdout);
                assert!(said.contains("maps/first.ron"), "{said}");
                assert_eq!(
                    said.contains("maps/future.ron"),
                    edition == "full",
                    "{said}"
                );
                assert_eq!(said.contains(CODE), edition == "full", "{said}");
            }
        }
        assert_eq!(held["public-demo"], Vec::<&str>::new(), "{target}");
        assert_eq!(held["full"], CANARIES, "{target}");
    }
    fs::remove_dir_all(&project.root).unwrap();
}

#[test]
fn a_demo_built_from_the_full_games_files_is_refused_and_leaves_no_package() {
    let mut project = game("leaked");
    let demo = project.config.editions.get_mut("public-demo").unwrap();
    demo.content = Some("game/files".to_owned());
    demo.disclosure.as_mut().unwrap().allowed_pack_entries =
        args(&["first.ron", "future/secret.ron"]);
    let answer = export(&project, &input("web", Some("public-demo")), None);
    assert!(!answer.ok);
    assert_eq!(answer.code.as_deref(), Some("SH3012"));
    let error = answer.error.unwrap();
    for canary in CANARIES {
        assert!(
            !error.contains(canary),
            "a canary is never printed: {error}"
        );
    }
    let out = project.root.join("dist/export/public-demo/web");
    let left: Vec<String> = fs::read_dir(&out)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(left, [disclosure::REPORT]);
    let report = fs::read_to_string(out.join(disclosure::REPORT)).unwrap();
    for canary in CANARIES {
        assert!(report.contains("\"canary\": ") && !report.contains(canary));
    }
    fs::remove_dir_all(&project.root).unwrap();
}

#[test]
fn a_build_without_a_content_root_takes_the_variable_away() {
    let project = game("unset");
    let cx = ExportCx::of(&project, &desktop::WINDOWS, &input("windows", None)).unwrap();
    let mut cargo = std::process::Command::new("cargo");
    cargo.env(CONTENT, "somewhere");
    cx.content_env(&mut cargo);
    let set: Vec<_> = cargo
        .get_envs()
        .filter(|(name, _)| *name == CONTENT)
        .collect();
    assert_eq!(set, [(std::ffi::OsStr::new(CONTENT), None)]);
    let demo = ExportCx::of(
        &project,
        &desktop::WINDOWS,
        &input("windows", Some("public-demo")),
    )
    .unwrap();
    demo.content_env(&mut cargo);
    let set: Vec<_> = cargo
        .get_envs()
        .filter(|(name, _)| *name == CONTENT)
        .collect();
    let path = project.root.join("game/files-demo");
    assert_eq!(
        set,
        [(std::ffi::OsStr::new(CONTENT), Some(path.as_os_str()))]
    );
    fs::remove_dir_all(&project.root).unwrap();
}

#[test]
fn an_edition_ships_another_program_of_the_package_with_its_own_files() {
    // A writer's preview: the package's second program, built from a folder
    // without the game's scenes.
    let project = game("preview");
    let answer = export(&project, &input("web", Some("writer-preview")), None);
    assert!(answer.ok, "{:?}", answer.error);
    let output = answer.result.unwrap();
    assert_eq!(output.package, "preview-0.1.0-writer-preview-web.zip");
    let files = package_files(&project, &output.out, &output.package);
    let program = files.iter().find(|(name, _)| name == "preview.wasm");
    let (_, bytes) = program.expect("the preview's program");
    for canary in CANARIES {
        assert!(!holds(bytes, canary), "{canary}");
    }
    assert!(holds(bytes, "maps/first.ron"));
    fs::remove_dir_all(&project.root).unwrap();
}

/// The project file of the stand-in game with a tool program: a public demo
/// held to the canaries of the full game's files, and the full game.
const TOOL_PROJECT: &str = r#"(
    game: (package: "game", bin: "tools"),
    editions: {
        "public-demo": (
            default_features: false,
            content: "game/files-demo",
            disclosure: (
                allowed_pack_entries: ["maps/first.ron"],
                forbidden_paths: ["maps/future.ron"],
                canaries: ["EDITIONS-FUTURE-MAP-CANARY", "EDITIONS-DIALOGUE-CANARY"],
            ),
        ),
        "full": (default_features: false, content: "game/files"),
    },
)"#;

/// The stand-in's tool program: its `bake` writes a pack of the files it
/// embeds (its content root, [`BUILD`]) where `--out` says, and answers as
/// a game's tool program does.
const TOOLS: &str = r#"include!(concat!(env!("OUT_DIR"), "/content.rs"));

use std::collections::BTreeMap;

use shuyi_assets::pack::{self, Content, PackInfo};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    assert_eq!(args.first().map(String::as_str), Some("bake"), "{args:?}");
    let value = |flag: &str| {
        let at = args.iter().position(|arg| arg == flag).unwrap();
        args[at + 1].clone()
    };
    let info = PackInfo {
        stamp: "the stand-in 1".to_owned(),
        target: Some(value("--target")),
        ..PackInfo::default()
    };
    let files: BTreeMap<String, Content> = FILES
        .iter()
        .map(|(name, bytes)| ((*name).to_owned(), Content::Raw(bytes.to_vec())))
        .collect();
    let out = std::path::PathBuf::from(value("--out"));
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(&out, pack::write(&info, &files)).unwrap();
    println!(
        "{{\"contract\": CONTRACT, \"op\": \"bake\", \"ok\": true, \"result\": {{\"errors\": [], \
         \"pack\": null}}, \"error\": null, \"code\": null, \"help\": null, \"limits\": []}}"
    );
}
"#;

/// The stand-in game with a tool program, for test `name`, fresh: its full
/// files in `game/files` and the demo's in `game/files-demo`. Its program
/// depends on the engine's `shuyi-assets` alone, resolved from the engine's
/// lock file with no network, and is built in a folder kept between runs.
fn tool_game(name: &str) -> Project {
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let engine = std::path::absolute(&engine).unwrap();
    let root = std::env::temp_dir().join(format!("shuyi-editions-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    write(
        &root.join("Cargo.toml"),
        b"[workspace]\nresolver = \"2\"\nmembers = [\"game\"]\n",
    );
    for file in ["Cargo.lock", "rust-toolchain.toml"] {
        write(&root.join(file), &fs::read(engine.join(file)).unwrap());
    }
    let target = std::env::temp_dir().join("shuyi-editions-tool-game-target");
    let config = format!(
        "[build]\ntarget-dir = '{}'\n\n[net]\noffline = true\n",
        target.display()
    );
    write(&root.join(".cargo/config.toml"), config.as_bytes());
    let manifest = format!(
        "[package]\nname = \"game\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n\
         [dependencies]\nshuyi-assets = {{ path = '{}', default-features = false }}\n",
        engine.join("crates/assets").display()
    );
    write(&root.join("game/Cargo.toml"), manifest.as_bytes());
    write(&root.join("game/build.rs"), BUILD.as_bytes());
    write(&root.join("game/src/main.rs"), b"fn main() {}\n");
    let tools = TOOLS.replace("CONTRACT", &crate::contract::VERSION.to_string());
    write(&root.join("game/src/bin/tools.rs"), tools.as_bytes());
    let full = root.join("game/files");
    write(&full.join("maps/first.ron"), b"(name: \"the first map\")\n");
    write(
        &full.join("maps/future.ron"),
        format!("(name: \"{MAP}\")\n").as_bytes(),
    );
    write(
        &full.join("dialogue/act2.dlg"),
        format!("= act two\n{DIALOGUE}\n").as_bytes(),
    );
    let demo = root.join("game/files-demo");
    write(&demo.join("maps/first.ron"), b"(name: \"the first map\")\n");
    write(&root.join("shuyi.ron"), TOOL_PROJECT.as_bytes());
    Project::load(&root).unwrap()
}

/// The Playdate export of `edition` of `project`, as far as its `prepare`.
fn playdate_cx(project: &Project, edition: &str) -> ExportCx {
    let content = project.config.editions[edition].content.clone();
    let shown = format!("dist/export/{edition}/playdate");
    ExportCx {
        target: "playdate".to_owned(),
        profile: "playdate".to_owned(),
        workspace: project.root.clone(),
        target_dir: project.root.join("target"),
        out: project.root.join(&shown),
        shown,
        root: project.root.clone(),
        release: false,
        package: "game".to_owned(),
        bin: "game".to_owned(),
        version: "0.1.0".to_owned(),
        title: "Game".to_owned(),
        game: Vec::new(),
        web: None,
        app: None,
        features: args(&[playdate::FEATURE]),
        turned_on: Vec::new(),
        beside: Vec::new(),
        wasm_opt: crate::project::WasmOptRule::Required,
        edition: Some(EditionBuild {
            content,
            ..EditionBuild::new(edition, false)
        }),
    }
}

#[test]
fn a_playdate_editions_pack_is_baked_from_its_own_folder_and_holds_nothing_else() {
    let project = tool_game("playdate");
    let demo_part = project.config.editions["public-demo"]
        .disclosure
        .clone()
        .unwrap();
    // The game's tools running now embed neither folder: the bake is the
    // tool program's, built from the edition's.
    let running = crate::contract::fake::Fake::default();
    for (edition, entries) in [
        ("public-demo", vec!["maps/first.ron"]),
        (
            "full",
            vec!["dialogue/act2.dlg", "maps/first.ron", "maps/future.ron"],
        ),
    ] {
        let cx = playdate_cx(&project, edition);
        let input = playdate::bake_input(&cx);
        assert_eq!(
            input.content, project.config.editions[edition].content,
            "{edition}"
        );
        playdate::PLAYDATE
            .prepare(&cx, &project, Some(&running))
            .unwrap_or_else(|e| panic!("{edition}: {e:?}"));
        // The pack, as the .pdx would hold it, read back by the gate.
        let gate = project.root.join("gate").join(edition);
        let tree = gate.join("game.pdx");
        write(
            &tree.join("game.pack"),
            &fs::read(cx.out.join("game.pack")).unwrap(),
        );
        let mut shipped = disclosure::Shipped::new(tree, None, "");
        shipped.source(
            "game.pack",
            disclosure::FileKind::Pack,
            "the game's bundle, baked",
        );
        let report = disclosure::inspect(
            &gate,
            Some(&demo_part),
            &shipped,
            1 << 30,
            disclosure::BuildInputs::default(),
        )
        .unwrap();
        let held: Vec<&str> = report
            .pack_entries
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        assert_eq!(held, entries, "{edition}");
        // The demo passes its canaries; the full game's pack, held to the
        // demo's part, would not.
        assert_eq!(report.ok, edition == "public-demo", "{edition}: {report:?}");
    }
    fs::remove_dir_all(&project.root).unwrap();
}
