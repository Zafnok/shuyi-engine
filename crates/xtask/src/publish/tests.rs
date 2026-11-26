use super::*;

const ROOT: &str = r#"
[workspace.package]
version = "0.1.0"
publish = false

[workspace.dependencies]
shuyi-core = { path = "crates/core", version = "=0.1.0", default-features = false }
shuyi-ui = { path = "crates/ui", version = "=0.1.0", default-features = false }
serde = { version = "1" }
"#;

const CORE: &str = r#"
[package]
name = "shuyi-core"
version.workspace = true
publish.workspace = true

[dependencies]
serde.workspace = true

[dev-dependencies]
shuyi-core = { path = ".", features = ["toy"] }
shuyi-ui = { path = "../ui" }
"#;

const UI: &str = r#"
[package]
name = "shuyi-ui"
version.workspace = true
publish.workspace = true

[dependencies]
shuyi-core = { workspace = true, features = ["default"] }

[target.'cfg(windows)'.build-dependencies]
shuyi-core = { workspace = true, optional = true }
"#;

const EXAMPLE: &str = r#"
[package]
name = "hello"
version.workspace = true
publish = false

[dependencies]
shuyi-ui = { path = "../../crates/ui" }
"#;

fn toml(text: &str) -> toml::Table {
    text.parse().unwrap()
}

/// A core, a ui over it (its tests close a cycle with the core's) and an
/// example: as the rules say.
fn clean() -> Workspace {
    let mut workspace = parse_root(&toml(ROOT));
    for (name, text) in [("shuyi-core", CORE), ("shuyi-ui", UI), ("hello", EXAMPLE)] {
        workspace
            .members
            .insert(name.into(), parse_member(&toml(text)));
    }
    workspace
}

/// `clean()` with one member's manifest replaced.
fn with(name: &str, text: &str) -> Workspace {
    let mut workspace = clean();
    workspace
        .members
        .insert(name.into(), parse_member(&toml(text)));
    workspace
}

#[test]
fn a_workspace_as_the_rules_say_passes() {
    assert_eq!(check(&clean()), Vec::<String>::new());
}

#[test]
fn manifests_are_read_with_their_kinds_targets_and_renames() {
    let ui = parse_member(&toml(UI));
    assert_eq!(
        (&ui.version, &ui.publish),
        (&Field::Inherited, &Field::Inherited)
    );
    let deps: Vec<(&str, Kind, bool)> = ui
        .deps
        .iter()
        .map(|d| (d.name.as_str(), d.kind, d.workspace))
        .collect();
    assert_eq!(
        deps,
        [
            ("shuyi-core", Kind::Normal, true),
            ("shuyi-core", Kind::Build, true)
        ]
    );
    let renamed = parse_member(&toml(
        "[package]\nversion = \"0.2.0\"\npublish = false\n[dependencies]\ncore = { package = \
         \"shuyi-core\", version = \"1\" }\nlibm = \"0.2\"\n",
    ));
    assert_eq!(renamed.version, Field::Own("0.2.0".into()));
    assert_eq!(renamed.publish, Field::Own("false".into()));
    assert_eq!(renamed.deps[0].name, "shuyi-core");
    assert_eq!(renamed.deps[0].version.as_deref(), Some("1"));
    assert_eq!(renamed.deps[1].version.as_deref(), Some("0.2"));
    let root = parse_root(&toml(ROOT));
    assert_eq!(root.version.as_deref(), Some("0.1.0"));
    assert_eq!(root.entries["shuyi-core"].as_deref(), Some("=0.1.0"));
}

#[test]
fn an_engine_crate_with_its_own_version_fails() {
    let core = CORE.replace("version.workspace = true", "version = \"0.1.0\"");
    assert_eq!(
        check(&with("shuyi-core", &core)),
        [
            "shuyi-core's `version` is not the workspace's: write `version.workspace = true` \
             (one version for every engine crate, ADR-0152)"
        ]
    );
}

#[test]
fn an_engine_crate_with_its_own_publish_fails() {
    let core = CORE.replace("publish.workspace = true", "publish = false");
    assert_eq!(
        check(&with("shuyi-core", &core)),
        [
            "shuyi-core sets its own `publish`: an engine crate is published with the rest, \
             so write `publish.workspace = true` (ADR-0152)"
        ]
    );
}

#[test]
fn an_example_without_publish_false_fails() {
    for publish in ["publish.workspace = true", "publish = true", ""] {
        let example = EXAMPLE.replace("publish = false", publish);
        assert_eq!(
            check(&with("hello", &example)),
            [
                "hello is no engine crate and has no `publish = false` of its own: add it to \
                 its [package], so the workspace's `publish` never makes it publishable \
                 (ADR-0152)"
            ],
            "{publish:?}"
        );
    }
}

#[test]
fn a_path_only_normal_or_build_dependency_on_an_engine_crate_fails() {
    for section in ["dependencies", "build-dependencies"] {
        let ui = format!(
            "[package]\nversion.workspace = true\npublish.workspace = true\n[{section}]\n\
             shuyi-core = {{ path = \"../core\" }}\n"
        );
        let kind = section.strip_suffix("-dependencies").unwrap_or("normal");
        assert_eq!(
            check(&with("shuyi-ui", &ui)),
            [format!(
                "shuyi-ui's {kind} dependency on shuyi-core doesn't come from \
                 [workspace.dependencies]: write `shuyi-core = {{ workspace = true, ... }}` \
                 with its own `features` and `optional` (its exact pin, ADR-0152)"
            )],
            "{section}"
        );
    }
}

#[test]
fn a_workspace_entry_pinned_without_equals_fails() {
    let mut workspace = clean();
    workspace
        .entries
        .insert("shuyi-core".into(), Some("0.1.0".into()));
    assert_eq!(
        check(&workspace),
        [
            "[workspace.dependencies] pins shuyi-core to \"0.1.0\", which lets cargo pick a \
             later version: write \"=0.1.0\", exactly the workspace's version (ADR-0152)"
        ]
    );
}

#[test]
fn a_pin_that_differs_from_the_workspaces_version_fails() {
    let mut workspace = clean();
    workspace
        .entries
        .insert("shuyi-ui".into(), Some("=0.0.9".into()));
    assert_eq!(
        check(&workspace),
        [
            "[workspace.dependencies] pins shuyi-ui to \"=0.0.9\", and the workspace's \
             version is 0.1.0: write \"=0.1.0\" (ADR-0152)"
        ]
    );
}

#[test]
fn a_missing_or_unversioned_entry_fails() {
    let mut workspace = clean();
    workspace.entries.remove("shuyi-core");
    assert_eq!(
        check(&workspace),
        [
            "an engine crate depends on shuyi-core, and [workspace.dependencies] has no entry \
             for it: add `shuyi-core = { path = ..., version = \"=0.1.0\", default-features = \
             false }` (ADR-0152)"
        ]
    );
    workspace.entries.insert("shuyi-core".into(), None);
    assert_eq!(
        check(&workspace),
        [
            "[workspace.dependencies]'s entry for shuyi-core has no `version`, and cargo \
             refuses to package a crate that depends on it: write `version = \"=0.1.0\"` \
             (ADR-0152)"
        ]
    );
    workspace.version = None;
    assert_eq!(check(&workspace).len(), 1);
}

#[test]
fn a_dev_dependency_with_a_version_fails() {
    for dep in [
        "shuyi-ui = { path = \"../ui\", version = \"=0.1.0\" }",
        "shuyi-ui = { workspace = true }",
    ] {
        let core = CORE.replace("shuyi-ui = { path = \"../ui\" }", dep);
        assert_eq!(
            check(&with("shuyi-core", &core)),
            [
                "shuyi-core's dev-dependency on shuyi-ui carries a version: keep it path-only \
                 (`path = ...` alone), so cargo leaves it out of the package (ADR-0152)"
            ],
            "{dep}"
        );
    }
}

#[test]
fn an_engine_crate_depending_on_an_unpublished_member_fails() {
    let ui = UI.replace(
        "shuyi-core = { workspace = true, features = [\"default\"] }",
        "shuyi-core = { workspace = true, features = [\"default\"] }\n\
         hello = { path = \"../../examples/hello\" }",
    );
    assert_eq!(
        check(&with("shuyi-ui", &ui)),
        [
            "shuyi-ui depends on hello, a member that isn't published, so shuyi-ui's package \
             can't resolve it: make it a dev-dependency, or move what it needs into an engine \
             crate (ADR-0152)"
        ]
    );
}

#[test]
fn a_cycle_of_two_engine_crates_fails() {
    let core = CORE.replace(
        "[dependencies]\nserde.workspace = true",
        "[dependencies]\nserde.workspace = true\nshuyi-ui.workspace = true",
    );
    let workspace = with("shuyi-core", &core);
    assert_eq!(
        order(&workspace),
        Err(vec![
            "shuyi-core".into(),
            "shuyi-ui".into(),
            "shuyi-core".into()
        ])
    );
    assert_eq!(
        check(&workspace),
        [
            "the engine crates' normal and build dependencies make a cycle (shuyi-core → \
             shuyi-ui → shuyi-core): no order publishes them, so break it (a dev-dependency \
             may close a cycle, ADR-0152)"
        ]
    );
}

/// A member of `deps`' engine crates, each depending normally on those
/// named.
fn graph(edges: &[(&str, &[&str])]) -> Workspace {
    let mut workspace = parse_root(&toml(ROOT));
    for (name, on) in edges {
        let deps = on
            .iter()
            .map(|d| Dep {
                name: (*d).into(),
                kind: Kind::Normal,
                workspace: true,
                version: None,
            })
            .collect();
        workspace.members.insert(
            (*name).into(),
            Manifest {
                deps,
                ..Manifest::default()
            },
        );
    }
    workspace
}

#[test]
fn the_order_puts_each_crate_after_what_it_depends_on() {
    let workspace = graph(&[
        ("shuyi", &["shuyi-app", "shuyi-core"]),
        ("shuyi-app", &["shuyi-ui", "shuyi-assets"]),
        (
            "shuyi-assets",
            &["shuyi-embed-macros", "shuyi-core", "serde"],
        ),
        ("shuyi-core", &[]),
        ("shuyi-embed", &[]),
        ("shuyi-embed-macros", &["shuyi-embed"]),
        ("shuyi-text", &[]),
        ("shuyi-ui", &["shuyi-text"]),
        ("xtask", &["shuyi"]),
    ]);
    let order = order(&workspace).unwrap();
    assert_eq!(
        order,
        [
            "shuyi-core",
            "shuyi-embed",
            "shuyi-embed-macros",
            "shuyi-assets",
            "shuyi-text",
            "shuyi-ui",
            "shuyi-app",
            "shuyi",
        ]
    );
}

#[test]
fn dev_edges_are_not_in_the_order_and_a_dev_cycle_passes() {
    // The core's tests use the ui, which depends on the core.
    let workspace = clean();
    assert_eq!(order(&workspace).unwrap(), ["shuyi-core", "shuyi-ui"]);
    assert_eq!(check(&workspace), Vec::<String>::new());
}

#[test]
fn the_index_paths_follow_crates_ios_layout() {
    assert_eq!(index_path("a"), "1/a");
    assert_eq!(index_path("ab"), "2/ab");
    assert_eq!(index_path("abc"), "3/a/abc");
    assert_eq!(index_path("shuyi"), "sh/uy/shuyi");
    assert_eq!(index_path("Shuyi-Core"), "sh/uy/shuyi-core");
}

#[test]
fn a_fake_index_fails_the_one_missing_name_and_passes_the_rest() {
    let names: Vec<String> = ["shuyi", "shuyi-core", "shuyi-net"]
        .map(String::from)
        .to_vec();
    let errors = missing_from_index(&names, |name| Ok(name != "shuyi-net"));
    assert_eq!(
        errors,
        [
            "shuyi-net is not on crates.io (https://index.crates.io/sh/uy/shuyi-net): its name \
             must be held before it can be published, which is Nick's to do (a 0.0.0 \
             placeholder, ask with the `ask-nick` skill)"
        ]
    );
    let errors = missing_from_index(&names[..1], |_| Err("offline".into()));
    assert_eq!(errors, ["shuyi: crates.io's index can't be read: offline"]);
}

#[test]
fn cargo_metadata_gives_the_members_alone() {
    let json = r#"{
        "packages": [
            {"id": "a", "name": "shuyi-core", "manifest_path": "/r/crates/core/Cargo.toml"},
            {"id": "b", "name": "serde", "manifest_path": "/s/Cargo.toml"}
        ],
        "workspace_members": ["a"]
    }"#;
    assert_eq!(
        parse_members(json).unwrap(),
        [(
            "shuyi-core".to_owned(),
            PathBuf::from("/r/crates/core/Cargo.toml")
        )]
    );
    assert!(
        parse_members("[]")
            .unwrap_err()
            .starts_with("cargo metadata's output: ")
    );
}

#[test]
fn the_repository_passes_and_publishes_the_api_crates() {
    let root = crate::repo_root();
    let workspace = read(&root).unwrap();
    assert_eq!(check(&workspace), Vec::<String>::new());
    let order = order(&workspace).unwrap();
    let published: BTreeSet<&str> = order.iter().map(String::as_str).collect();
    let api: BTreeSet<&str> = crate::api::API_CRATES.iter().map(|c| c.name).collect();
    assert_eq!(published, api);
    let at = |name: &str| order.iter().position(|n| n == name).unwrap();
    assert!(at("shuyi-embed") < at("shuyi-embed-macros"));
    assert!(at("shuyi-embed-macros") < at("shuyi-assets"));
    assert!(at("shuyi-sound") < at("shuyi-sound-out"));
    assert!(at("shuyi-sound-out") < at("shuyi-app"));
}

#[test]
fn a_folder_with_no_workspace_is_an_error() {
    let root = std::env::temp_dir().join(format!("xtask-publish-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let error = read(&root).unwrap_err();
    let _ = fs::remove_dir_all(&root);
    assert!(error.starts_with("cargo metadata failed: "), "{error}");
}
