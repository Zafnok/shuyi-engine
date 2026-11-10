//! The debug features' check: its arguments, its reading of `cargo tree`,
//! its refusals, its table of the engine's debug features, and small
//! workspaces resolved by Cargo itself, with stand-ins for the engine's
//! three packages that have them (no network, no SDK).

use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::contract::codes;
use crate::export::{desktop, playdate, web};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// The tail every `cargo tree` call ends with.
const TAIL: [&str; 6] = ["-e", "normal,build", "--prefix", "depth", "-f", FORMAT];

fn tree(items: &[&str]) -> Vec<String> {
    let mut all = args(&["tree"]);
    all.extend(args(items));
    all.extend(args(&TAIL));
    all
}

#[test]
fn the_tree_resolves_each_exporters_builds_with_their_own_flags() {
    let mut cx = crate::export::tests::cx("live-link-args", "windows");
    cx.release = true;
    cx.turned_on = args(&["packaged"]);
    let windows = desktop::WINDOWS.cargo_args(&cx);
    assert_eq!(
        tree_args(&windows[0]).unwrap(),
        tree(&[
            "-p",
            "hello-grid",
            "--target",
            "x86_64-pc-windows-gnu",
            "--features",
            "packaged"
        ])
    );
    let builds = web::WEB.cargo_args(&cx);
    assert_eq!(
        tree_args(&builds[0]).unwrap(),
        tree(&[
            "-p",
            "hello-grid",
            "--target",
            "wasm32-unknown-unknown",
            "--features",
            "packaged"
        ])
    );
    // The Playdate's library builds take no default feature, and the tree
    // doesn't either.
    let builds = playdate::PLAYDATE.cargo_args(&cx);
    assert_eq!(builds.len(), 2);
    for (build, triple) in builds.iter().zip(playdate::PLAYDATE.triples()) {
        assert_eq!(
            tree_args(build).unwrap(),
            tree(&[
                "-p",
                "hello-grid",
                "--no-default-features",
                "--features",
                "playdate,packaged",
                "--target",
                triple
            ])
        );
    }
    fs::remove_dir_all(&cx.out).unwrap();
}

#[test]
fn the_tree_keeps_every_feature_flag_and_refuses_a_flag_it_doesnt_know() {
    assert_eq!(
        tree_args(&args(&[
            "build",
            "--package",
            "g",
            "-F",
            "a",
            "--features=b",
            "--all-features",
            "--lib"
        ]))
        .unwrap(),
        tree(&[
            "--package",
            "g",
            "-F",
            "a",
            "--features=b",
            "--all-features"
        ])
    );
    for wrong in [
        &["test", "-p", "g"][..],
        &[],
        &["build", "--manifest-path", "x/Cargo.toml"],
        &["build", "-p"],
        &["build", "--profile", "dist"],
    ] {
        let refused = tree_args(&args(wrong)).unwrap_err();
        assert_eq!(refused.code(), codes::PROJECT_FILE, "{wrong:?}");
    }
}

#[test]
fn cargo_trees_answer_is_read_package_by_package() {
    let text = "0|default,link|game v0.1.0 (C:\\a b|c\\game)\n\
                1||helper v0.1.0 (/x/helper)\n\
                2|link|shuyi-app v0.1.0 (https://github.com/Zafnok/shuyi-engine#1234)\n\
                \n\
                1|link|shuyi-app v0.1.0 (/x) (*)\n";
    let nodes = parse(text).unwrap();
    assert_eq!(nodes.len(), 4);
    assert_eq!(
        nodes[0],
        Node {
            depth: 0,
            name: "game".to_owned(),
            features: args(&["default", "link"]),
        }
    );
    assert_eq!(nodes[1].features, Vec::<String>::new());
    assert_eq!((nodes[2].depth, nodes[2].name.as_str()), (2, "shuyi-app"));
    for wrong in [
        "",
        "game v0.1.0",
        "x|link|game",
        "1|link|game v0.1.0",
        "0|link|",
    ] {
        assert!(parse(wrong).is_err(), "{wrong:?}");
    }
}

fn node(depth: usize, name: &str, features: &[&str]) -> Node {
    Node {
        depth,
        name: name.to_owned(),
        features: args(features),
    }
}

#[test]
fn the_link_counts_in_the_package_built_and_the_engine_only() {
    let nodes = [
        node(0, "game", &["default", "linked"]),
        node(1, "helper", &["link"]),
        node(2, "shuyi-app", &["link"]),
        node(3, "shuyi-platform", &["default", "link", "std"]),
        node(1, "engine-tools", &["link"]),
        node(1, "shuyi-app", &["link"]),
    ];
    assert_eq!(
        violations(&nodes),
        [
            Violation {
                package: "shuyi-app".to_owned(),
                feature: "link",
                path: args(&["game", "helper", "shuyi-app"]),
            },
            Violation {
                package: "shuyi-platform".to_owned(),
                feature: "link",
                path: args(&["game", "helper", "shuyi-app", "shuyi-platform"]),
            },
        ]
    );
    assert_eq!(
        violations(&[node(0, "game", &["default", "link"])]),
        [Violation {
            package: "game".to_owned(),
            feature: "link",
            path: args(&["game"]),
        }]
    );
    assert_eq!(
        violations(&[node(0, "game", &["linked"]), node(1, "shuyi-app", &[])]),
        []
    );
}

#[test]
fn a_refusal_names_the_package_the_feature_and_the_way_there() {
    let nodes = [
        node(0, "game", &["default", "devtools", "link"]),
        node(1, "helper", &[]),
        node(2, "shuyi-app", &["link"]),
    ];
    let build = args(&["build", "-p", "game", "--target", "wasm32-unknown-unknown"]);
    let refused = refusal(&build, &nodes, &violations(&nodes));
    assert_eq!(refused.code(), codes::PROJECT_FILE);
    let message = refused.message();
    assert!(
        message.starts_with(
            "an export never turns on a debug feature, and Cargo's build of `game` for \
             wasm32-unknown-unknown turns on:\n  `game/link`, the debug build's live link, \
             among the package's features as built: default, devtools, link\n  \
             `shuyi-app/link`, the debug build's live link, reached through game -> helper -> \
             shuyi-app\n"
        ),
        "{message}"
    );
    assert!(message.contains("`--without` leaves out only"), "{message}");
}

/// The packages of `cargo tree`'s answer `lines`, one package a line.
fn tree_of(lines: &[String]) -> Vec<Node> {
    parse(&lines.join("\n")).unwrap()
}

#[test]
fn each_debug_feature_counts_by_a_default_an_alias_and_a_dependency() {
    for (feature, engine) in crate::export::ENGINE_DEBUG_FEATURES {
        let what = what(feature);
        // The package's own, by its `default` list and by an alias.
        for features in [format!("default,{feature}"), format!("devtools,{feature}")] {
            let nodes = tree_of(&[format!("0|{features}|game v0.1.0 (/g)")]);
            assert_eq!(
                violations(&nodes),
                [Violation {
                    package: "game".to_owned(),
                    feature,
                    path: args(&["game"]),
                }],
                "{features}"
            );
            let message = refusal(&args(&["build"]), &nodes, &violations(&nodes)).message();
            assert!(
                message.contains(&format!(
                    "`game/{feature}`, {what}, among the package's features as built: {}",
                    features.replace(',', ", ")
                )),
                "{message}"
            );
        }
        for package in engine {
            // An alias of the game's forwarding to the engine's, and a
            // dependency's defaults asking for it, under the game's alias.
            let alias = tree_of(&[
                "0|devtools|game v0.1.0 (/g)".to_owned(),
                format!("1|{feature}|{package} v0.1.0 (/e)"),
            ]);
            let dependency = tree_of(&[
                "0||game v0.1.0 (/g)".to_owned(),
                "1|default,live|helper v0.1.0 (/h)".to_owned(),
                format!("2|default,{feature}|{package} v0.1.0 (/e)"),
            ]);
            for (nodes, path) in [
                (alias, vec!["game", package]),
                (dependency, vec!["game", "helper", package]),
            ] {
                let found = violations(&nodes);
                assert_eq!(
                    found,
                    [Violation {
                        package: (*package).to_owned(),
                        feature,
                        path: args(&path),
                    }],
                    "{package}/{feature}"
                );
                let message = refusal(&args(&["build"]), &nodes, &found).message();
                assert!(
                    message.contains(&format!(
                        "`{package}/{feature}`, {what}, reached through {}",
                        path.join(" -> ")
                    )),
                    "{message}"
                );
            }
        }
        // A dependency that isn't the engine's, or an engine package that
        // hasn't the feature, isn't watched for it.
        let other = ["shuyi-app", "shuyi-platform", "shuyi-ui"]
            .into_iter()
            .find(|package| !engine.contains(package))
            .unwrap();
        let nodes = tree_of(&[
            "0||game v0.1.0 (/g)".to_owned(),
            format!("1|{feature}|helper v0.1.0 (/h)"),
            format!("1|{feature}|{other} v0.1.0 (/e)"),
        ]);
        assert_eq!(violations(&nodes), [], "{feature} in {other}");
    }
}

#[test]
fn the_engines_debug_features_are_its_crates_features() {
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut found: Vec<(&str, Vec<String>)> = DEBUG_FEATURES
        .iter()
        .map(|(name, _)| (*name, Vec::new()))
        .collect();
    for entry in fs::read_dir(&crates).unwrap() {
        let manifest = entry.unwrap().path().join("Cargo.toml");
        let Ok(text) = fs::read_to_string(&manifest) else {
            continue;
        };
        let doc: toml_edit::DocumentMut = text.parse().unwrap();
        let name = doc["package"]["name"].as_str().unwrap().to_owned();
        let Some(features) = doc.get("features").and_then(|f| f.as_table()) else {
            continue;
        };
        for (feature, packages) in &mut found {
            if features.contains_key(feature) {
                packages.push(name.clone());
            }
        }
    }
    for (feature, packages) in &mut found {
        packages.sort();
        let mut table: Vec<String> = crate::export::ENGINE_DEBUG_FEATURES
            .iter()
            .find(|(name, _)| name == feature)
            .map(|(_, engine)| engine.iter().map(|&p| p.to_owned()).collect())
            .unwrap_or_default();
        table.sort();
        assert_eq!(*packages, table, "the engine's crates with `{feature}`");
    }
}

/// An exporter whose builds are `builds`, and which fails if anything
/// past its plan is asked of it.
struct Plan {
    builds: Vec<Vec<String>>,
    rustflags: &'static [&'static str],
}

impl Exporter for Plan {
    fn id(&self) -> &'static str {
        "plan"
    }

    fn profile(&self) -> &'static str {
        "desktop"
    }

    fn triples(&self) -> &'static [&'static str] {
        &[]
    }

    fn cargo_args(&self, _cx: &ExportCx) -> Vec<Vec<String>> {
        self.builds.clone()
    }

    fn rustflags(&self, _triple: &str) -> &'static [&'static str] {
        self.rustflags
    }

    fn export(&self, _cx: &ExportCx) -> Result<super::super::ExportReport, ExportError> {
        unreachable!("the check never builds")
    }
}

/// The stand-ins' and the game's manifests, and the game's: `[package]` is
/// written for each.
const SHUYI_PLATFORM: &str = "[features]\ndefault = []\nlink = []\ncapture = []\n";
const SHUYI_UI: &str = "[features]\ndebug-tools = []\n";
const SHUYI_APP: &str = "[dependencies]\nshuyi-platform = { path = \"../shuyi-platform\" }\n\
    shuyi-ui = { path = \"../shuyi-ui\" }\n\n\
    [features]\nlink = [\"shuyi-platform/link\"]\ncapture = [\"shuyi-platform/capture\"]\n\n\
    # As the engine's own: its tests turn its link and its capture on.\n\
    [dev-dependencies]\nshuyi-app = { path = \".\", features = [\"link\", \"capture\"] }\n";
/// Another member, not the game's, that turns the link on.
const OTHER: &str =
    "[dependencies]\nshuyi-app = { path = \"../shuyi-app\", features = [\"link\"] }\n";

/// A workspace under the temporary folder for test `name`, with the
/// stand-ins, `other`, the game (`game`, its manifest's body below
/// `[package]`) and each of `more` (a name and its body), fresh.
fn workspace(name: &str, game: &str, more: &[(&str, &str)]) -> PathBuf {
    let root = std::env::temp_dir().join(format!("shuyi-live-link-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let mut members = vec![
        ("shuyi-platform", SHUYI_PLATFORM),
        ("shuyi-ui", SHUYI_UI),
        ("shuyi-app", SHUYI_APP),
        ("other", OTHER),
        ("game", game),
    ];
    members.extend(more);
    let names: Vec<String> = members.iter().map(|(n, _)| format!("{n:?}")).collect();
    write(
        &root.join("Cargo.toml"),
        &format!(
            "[workspace]\nresolver = \"2\"\nmembers = [{}]\n",
            names.join(", ")
        ),
    );
    for (member, body) in members {
        let dir = root.join(member);
        write(
            &dir.join("Cargo.toml"),
            &format!(
                "[package]\nname = \"{member}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\
                 publish = false\n\n{body}"
            ),
        );
        write(&dir.join("src/lib.rs"), "");
    }
    write(&root.join("game/src/main.rs"), "fn main() {}\n");
    root
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// The game's dependency on the stand-in app, with `features`.
fn app(features: &str) -> String {
    format!("[dependencies]\nshuyi-app = {{ path = \"../shuyi-app\", features = [{features}] }}\n")
}

/// The check of `builds` (on `game`, `cargo build` arguments after `-p
/// game`) in `root`, the Rust flags `rustflags` added.
fn check(
    root: &Path,
    builds: &[&[&str]],
    rustflags: &'static [&'static str],
) -> Result<(), ExportError> {
    let name = root.file_name().unwrap().to_string_lossy();
    let mut cx = crate::export::tests::cx(&format!("check-{name}"), "windows");
    fs::remove_dir_all(&cx.out).unwrap();
    cx.root = root.to_path_buf();
    let plan = Plan {
        builds: builds
            .iter()
            .map(|rest| {
                let mut build = args(&["build", "-p", "game", "--bin", "game"]);
                build.extend(args(rest));
                build
            })
            .collect(),
        rustflags,
    };
    refuse(&cx, &plan)
}

const WINDOWS: [&str; 2] = ["--target", "x86_64-pc-windows-gnu"];

/// Whether `result` refuses the link of `package` (named as
/// `package/link`).
fn refuses(result: &Result<(), ExportError>, package: &str) -> bool {
    result.as_ref().is_err_and(|e| {
        e.code() == codes::PROJECT_FILE && e.message().contains(&format!("`{package}/link`"))
    })
}

#[test]
fn the_games_link_by_its_defaults_and_its_aliases_is_refused() {
    let features = "[features]\nlink = [\"shuyi-app/link\"]\ndevtools = [\"link\"]\n\
                    tools = [\"devtools\"]\nlinked = []\n";
    let root = workspace("aliases", &format!("{}\n{features}", app("")), &[]);
    // An alias one feature away, then two: each turns on the game's link and
    // the engine's.
    for alias in ["devtools", "tools"] {
        let refused = check(
            &root,
            &[&[WINDOWS[0], WINDOWS[1], "--features", alias]],
            &[],
        );
        assert!(refuses(&refused, "game"), "{refused:?}");
        assert!(refuses(&refused, "shuyi-app"), "{refused:?}");
        assert!(refuses(&refused, "shuyi-platform"), "{refused:?}");
    }
    // `link` itself, named in the arguments, is refused before Cargo runs.
    let refused = check(&root, &[&["--features", "link"]], &[]).unwrap_err();
    assert!(
        refused.message().contains("never turns on `link`"),
        "{refused:?}"
    );
    // A feature that merely starts like it, or no feature, passes.
    assert_eq!(
        check(
            &root,
            &[&WINDOWS, &[WINDOWS[0], WINDOWS[1], "--features", "linked"]],
            &[]
        ),
        Ok(())
    );
    fs::remove_dir_all(&root).unwrap();

    let root = workspace(
        "default",
        &format!(
            "{}\n[features]\ndefault = [\"link\"]\nlink = [\"shuyi-app/link\"]\n",
            app("")
        ),
        &[],
    );
    let refused = check(&root, &[&WINDOWS], &[]);
    assert!(refuses(&refused, "game"), "{refused:?}");
    let message = refused.unwrap_err().message();
    assert!(
        message.contains("features as built: default, link"),
        "{message}"
    );
    // Every build of the plan is checked: one without the defaults passes,
    // and the next is refused.
    let refused = check(
        &root,
        &[&[WINDOWS[0], WINDOWS[1], "--no-default-features"], &WINDOWS],
        &[],
    );
    assert!(refuses(&refused, "game"), "{refused:?}");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_dependency_turning_on_the_engines_link_is_refused_under_any_name() {
    let helper = format!(
        "{}\n[features]\ndefault = [\"live\"]\nlive = [\"shuyi-app/link\"]\n",
        app("")
    );
    let game = "[dependencies]\nengine = { package = \"shuyi-app\", path = \"../shuyi-app\" }\n\
                platform = { package = \"shuyi-platform\", path = \"../shuyi-platform\", \
                optional = true }\n\
                helper = { path = \"../helper\", optional = true }\n\n\
                [features]\ndev = [\"engine/link\"]\nwire = [\"platform/link\"]\n\
                viahelper = [\"dep:helper\"]\n";
    let root = workspace("dependencies", game, &[("helper", &helper)]);
    // A helper whose defaults ask for the engine's link.
    let refused = check(
        &root,
        &[&[WINDOWS[0], WINDOWS[1], "--features", "viahelper"]],
        &[],
    );
    assert!(refuses(&refused, "shuyi-app"), "{refused:?}");
    let message = refused.unwrap_err().message();
    assert!(
        message.contains("reached through game -> helper -> shuyi-app"),
        "{message}"
    );
    assert!(!message.contains("`game/link`"), "{message}");
    // The engine renamed, its feature forwarded by one of the game's.
    let refused = check(
        &root,
        &[&[WINDOWS[0], WINDOWS[1], "--features", "dev"]],
        &[],
    );
    assert!(refuses(&refused, "shuyi-app"), "{refused:?}");
    // The platform's link alone, renamed.
    let refused = check(
        &root,
        &[&[WINDOWS[0], WINDOWS[1], "--features", "wire"]],
        &[],
    );
    assert!(refuses(&refused, "shuyi-platform"), "{refused:?}");
    assert!(!refuses(&refused, "shuyi-app"), "{refused:?}");
    // None of them asked: the stand-in app's own dev-dependency on its
    // link, and `other`'s, don't count.
    assert_eq!(check(&root, &[&WINDOWS], &[]), Ok(()));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn the_debug_tools_and_capture_are_refused_by_a_default_or_a_dependency_not_a_dev_dependency() {
    let helper = format!(
        "{}shuyi-ui = {{ path = \"../shuyi-ui\" }}\n\n\
         [features]\ndefault = [\"tools\"]\ntools = [\"shuyi-ui/debug-tools\", \"shuyi-app/capture\"]\n",
        app("")
    );
    let game = format!(
        "{}ui = {{ package = \"shuyi-ui\", path = \"../shuyi-ui\" }}\n\
         helper = {{ path = \"../helper\", optional = true, default-features = false }}\n\n\
         [features]\ndefault = [\"debug-tools\"]\ndebug-tools = [\"ui/debug-tools\"]\n\
         viahelper = [\"helper/default\"]\n\n\
         [dev-dependencies]\nshuyi-app = {{ path = \"../shuyi-app\", features = [\"capture\"] }}\n",
        app("")
    );
    let root = workspace("every", &game, &[("helper", &helper)]);
    // The package's `default` list: its own `debug-tools` and the engine's.
    let refused = check(&root, &[&WINDOWS], &[]);
    let message = refused.unwrap_err().message();
    assert!(
        message.contains(
            "`game/debug-tools`, the debug tools: the debug menu, `SHUYI_PERF` traces and \
             session recording, among the package's features as built: debug-tools, default"
        ),
        "{message}"
    );
    // The engine's, the first way Cargo shows it.
    assert!(
        message.contains(
            "`shuyi-ui/debug-tools`, the debug tools: the debug menu, `SHUYI_PERF` traces and \
             session recording, reached through game -> shuyi-app -> shuyi-ui"
        ),
        "{message}"
    );
    // A dependency's defaults asking for the engine's debug tools and its
    // capture mode, each named with the way there.
    let no_default = [WINDOWS[0], WINDOWS[1], "--no-default-features"];
    let mut viahelper = no_default.to_vec();
    viahelper.extend(["--features", "viahelper"]);
    let message = check(&root, &[&viahelper], &[]).unwrap_err().message();
    assert!(!message.contains("`game/"), "{message}");
    assert!(
        message.contains("`shuyi-ui/debug-tools`")
            && message.contains("reached through game -> helper -> shuyi-app -> shuyi-ui"),
        "{message}"
    );
    assert!(
        message.contains("`shuyi-app/capture`, the capture mode, for CI's pictures, reached through game -> helper -> shuyi-app"),
        "{message}"
    );
    assert!(message.contains("`shuyi-platform/capture`"), "{message}");
    // None of them asked: the game's and the stand-in app's dev-dependencies
    // turning on the capture don't count.
    assert_eq!(check(&root, &[&no_default], &[]), Ok(()));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_dev_dependency_or_another_targets_link_passes_and_its_own_target_is_refused() {
    let game = format!(
        "{}\n[features]\nplaydate = []\n\n\
         [dev-dependencies]\nshuyi-app = {{ path = \"../shuyi-app\", features = [\"link\"] }}\n\n\
         [target.'cfg(target_arch = \"wasm32\")'.dependencies]\n\
         web-link = {{ package = \"shuyi-app\", path = \"../shuyi-app\", features = [\"link\"] }}\n\n\
         [target.'cfg(shuyi_playdate_sdk)'.dependencies]\n\
         sdk-link = {{ package = \"shuyi-platform\", path = \"../shuyi-platform\", features = [\"link\"] }}\n",
        app("")
    );
    let root = workspace("targets", &game, &[]);
    assert_eq!(check(&root, &[&WINDOWS], &[]), Ok(()));
    let refused = check(&root, &[&["--target", "wasm32-unknown-unknown"]], &[]);
    assert!(refuses(&refused, "shuyi-app"), "{refused:?}");
    // The Playdate's own `cfg` reaches the tree as it reaches the build.
    let device = [
        "--target",
        "thumbv7em-none-eabihf",
        "--no-default-features",
        "--features",
        "playdate",
    ];
    assert_eq!(check(&root, &[&device], &[]), Ok(()));
    let refused = check(&root, &[&device], &["--cfg=shuyi_playdate_sdk"]);
    assert!(refuses(&refused, "shuyi-platform"), "{refused:?}");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_build_cargo_cant_resolve_is_refused_with_its_cause() {
    let root = workspace("unresolved", &app(""), &[]);
    let refused = check(
        &root,
        &[&[WINDOWS[0], WINDOWS[1], "--features", "nosuch"]],
        &[],
    )
    .unwrap_err();
    assert_eq!(refused.code(), codes::PROJECT_FILE);
    let message = refused.message();
    assert!(
        message.contains(
            "failed, so the export can't tell whether its build turns on a debug feature"
        ),
        "{message}"
    );
    assert!(message.contains("nosuch"), "{message}");
    fs::remove_dir_all(&root).unwrap();
    // No workspace at all.
    let refused = check(&root, &[&WINDOWS], &[]).unwrap_err();
    assert!(refused.message().contains("can't tell"), "{refused:?}");
}

#[test]
fn hello_grids_exports_pass_though_the_engines_tests_turn_its_link_on() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // The engine's own tests do turn it on: a tree with dev-dependencies
    // shows it.
    let output = std::process::Command::new(crate::cargo())
        .args(["tree", "-p", "shuyi-app", "--prefix", "depth", "-f", FORMAT])
        .current_dir(&root)
        .output()
        .unwrap();
    let nodes = parse(&String::from_utf8_lossy(&output.stdout)).unwrap();
    assert!(
        nodes
            .iter()
            .any(|n| n.name == "shuyi-app" && n.features.iter().any(|f| f == "link"))
    );
    let mut cx = crate::export::tests::cx("live-link-hello-grid", "windows");
    cx.turned_on = args(&["packaged"]);
    for release in [false, true] {
        cx.release = release;
        for exporter in crate::export::exporters() {
            assert_eq!(refuse(&cx, *exporter), Ok(()), "{}", exporter.id());
        }
    }
    // Its debug build's `link`, asked for, is refused by Cargo's answer.
    cx.turned_on = args(&["window"]);
    let mut build = desktop::WINDOWS.cargo_args(&cx).remove(0);
    let at = build.iter().position(|a| a == "window").unwrap();
    build[at] = "link".to_owned();
    let nodes = resolve(&cx, &build, &[]).unwrap();
    let found = violations(&nodes);
    let packages: Vec<&str> = found.iter().map(|v| v.package.as_str()).collect();
    assert_eq!(packages, ["hello-grid", "shuyi-app", "shuyi-platform"]);
    fs::remove_dir_all(&cx.out).unwrap();
}

#[test]
fn an_export_is_refused_before_its_target_prepares_or_builds_anything() {
    use crate::contract::export::{ExportInput, export};
    let game = format!(
        "{}\n[features]\ndefault = [\"link\"]\nlink = [\"shuyi-app/link\"]\nplaydate = [\"link\"]\n",
        app("")
    );
    let root = workspace("contract", &game, &[]);
    let text = r#"(
        targets: {
            "desktop": (frame: (384, 384), scale: integer, text_px: 16),
            "web": (frame: (384, 384), scale: integer, text_px: 16),
            "playdate": (frame: (400, 240), scale: integer, text_px: 16),
        },
        export: (package: "game"),
        web: (package: "game", bin: "game", title: "Game", dist: "dist/web"),
    )"#;
    let project = crate::project::Project {
        root: root.clone(),
        config: crate::project::Config::parse(text).unwrap(),
    };
    // The Playdate's would fail to prepare (there is no `game` part to bake
    // its pack), the web's and Windows' to build: each is refused first.
    for target in ["windows", "web", "playdate"] {
        let input = ExportInput {
            target: target.to_owned(),
            run: true,
            ..ExportInput::default()
        };
        let answer = export(&project, &input, None);
        assert!(!answer.ok, "{target}");
        assert_eq!(answer.code.as_deref(), Some("SH1002"), "{target}");
        let error = answer.error.unwrap();
        assert!(
            error.starts_with(
                "an export never turns on a debug feature, and Cargo's build of `game`"
            ),
            "{target}: {error}"
        );
        assert!(error.contains("`game/link`"), "{target}: {error}");
        let out = root.join("dist/export").join(target);
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0, "{target}");
    }
    assert!(!root.join("dist/web").exists());
    assert!(!root.join("target").exists());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn without_passes_only_when_no_other_way_turns_the_link_on() {
    use crate::contract::export::ExportInput;
    let text = r#"(export: (package: "game", features: ["devtools"]))"#;
    let input = ExportInput {
        target: "windows".to_owned(),
        without: args(&["devtools"]),
        ..ExportInput::default()
    };
    for (name, default, ok) in [
        ("without", "", true),
        ("without-default", "\"link\"", false),
    ] {
        let game = format!(
            "{}\n[features]\ndefault = [{default}]\nlink = [\"shuyi-app/link\"]\n\
             devtools = [\"link\"]\n",
            app("")
        );
        let root = workspace(name, &game, &[]);
        let project = crate::project::Project {
            root: root.clone(),
            config: crate::project::Config::parse(text).unwrap(),
        };
        let mut cx = ExportCx::of(&project, &desktop::WINDOWS, &input).unwrap();
        assert_eq!(cx.turned_on, Vec::<String>::new());
        let checked = refuse(&cx, &desktop::WINDOWS);
        assert_eq!(checked.is_ok(), ok, "{name}: {checked:?}");
        if !ok {
            assert!(refuses(&checked, "game"), "{checked:?}");
        }
        // Without `--without`, the feature `export.features` lists is refused.
        cx.turned_on = args(&["devtools"]);
        assert!(refuses(&refuse(&cx, &desktop::WINDOWS), "game"), "{name}");
        fs::remove_dir_all(&root).unwrap();
    }
}
