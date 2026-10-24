//! The concerns file, module metadata, a game's `Cargo.toml` and a
//! module's package folder, on small folders made here and on this
//! repository.

use super::*;

/// The repository's root.
fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// An empty folder of its own for test `name`.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-catalog-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `files` written under `dir`.
fn write(dir: &Path, files: &[(&str, &str)]) {
    for (path, text) in files {
        let to = dir.join(path);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(to, text).unwrap();
    }
}

const ALPHA: &str = r#"[package]
name = "shuyi-alpha"
version = "0.1.0"

[package.metadata.shuyi]
answers = [{ concern = "bodies", answer = "rigid-body" }]
serves = ["side-view"]
alternatives = ["shuyi-beta"]
"#;

#[test]
fn the_engines_concerns_read_and_name_modules_or_the_default() {
    let catalog = Catalog::read(&repo()).unwrap_or_else(|e| panic!("{e}"));
    let ids: Vec<&str> = catalog
        .concerns
        .concerns
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(
        ids,
        [
            "time",
            "perspective",
            "bodies",
            "online",
            "world-drawing",
            "kind"
        ]
    );
    for concern in &catalog.concerns.concerns {
        for answer in &concern.answers {
            assert!(
                answer.by == "default" || answer.by.starts_with(PREFIX),
                "{}: {}",
                concern.id,
                answer.by
            );
        }
    }
    let kind = &catalog.concerns.concerns[5];
    assert!(!kind.alternatives);
    assert_eq!(
        catalog.concerns.choice("kind", "tactics").map(|c| c.status),
        Some(Status::Ready)
    );
    assert_eq!(catalog.concerns.choice("kind", "chess"), None);
    let by_3d: Vec<&str> = catalog
        .concerns
        .answered_by("shuyi-3d")
        .map(|(_, a)| a.id.as_str())
        .collect();
    assert_eq!(
        by_3d,
        ["first-person", "third-person", "top-down-3d", "full-3d"]
    );
    // Every module crate of the repository reads (`check-modules` checks
    // the rest); none is built yet.
    for module in &catalog.modules {
        assert!(!module.info.answers.is_empty(), "{}", module.name);
    }
}

#[test]
fn a_module_is_read_from_its_package_and_a_plain_crate_is_none() {
    let dir = folder("read");
    write(&dir, &[("alpha/Cargo.toml", ALPHA)]);
    let module = ModuleCrate::read(&dir.join("alpha")).unwrap().unwrap();
    assert_eq!(module.name, "shuyi-alpha");
    assert_eq!(module.feature(), "alpha");
    assert_eq!(
        module.info.answers,
        [Answered {
            concern: "bodies".into(),
            answer: "rigid-body".into()
        }]
    );
    assert_eq!(module.info.serves, ["side-view"]);
    assert_eq!(module.info.alternatives, ["shuyi-beta"]);
    assert!(module.info.works_with.is_empty() && module.info.formats.is_none());
    write(
        &dir,
        &[
            ("plain/Cargo.toml", "[package]\nname = \"plain\"\n"),
            (
                "other/Cargo.toml",
                "[package]\nname = \"other\"\n[package.metadata.docs]\nx = 1\n",
            ),
            ("virtual/Cargo.toml", "[workspace]\n"),
        ],
    );
    for plain in ["plain", "other", "virtual"] {
        assert_eq!(ModuleCrate::read(&dir.join(plain)), Ok(None), "{plain}");
    }
    write(
        &dir,
        &[(
            "typo/Cargo.toml",
            "[package]\nname = \"shuyi-typo\"\n[package.metadata.shuyi]\nanswers = []\nserves = []\nserve = []\n",
        )],
    );
    let error = ModuleCrate::read(&dir.join("typo")).unwrap_err();
    assert!(error.contains("serve"), "{error}");
    assert!(ModuleCrate::read(&dir.join("missing")).is_err());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_catalog_lists_the_modules_under_crates_by_name() {
    let dir = folder("catalog");
    let beta = ALPHA
        .replace("shuyi-alpha", "shuyi-beta")
        .replace("[\"shuyi-beta\"]", "[\"shuyi-alpha\"]");
    write(
        &dir,
        &[
            (CONCERNS, "(concerns: [])"),
            ("crates/beta/Cargo.toml", &beta),
            ("crates/alpha/Cargo.toml", ALPHA),
            (
                "crates/core/Cargo.toml",
                "[package]\nname = \"shuyi-core\"\n",
            ),
            ("crates/notes/README.md", "no crate"),
        ],
    );
    let catalog = Catalog::read(&dir).unwrap();
    let names: Vec<&str> = catalog.modules.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, ["shuyi-alpha", "shuyi-beta"]);
    assert_eq!(
        catalog.module("beta").map(|m| m.name.as_str()),
        Some("shuyi-beta")
    );
    assert_eq!(
        catalog.module("shuyi-alpha").map(|m| m.dir.clone()),
        Some(dir.join("crates/alpha"))
    );
    assert_eq!(catalog.module("gamma"), None);
    write(&dir, &[(CONCERNS, "(concerns: [], extra: 1)")]);
    assert!(
        Catalog::read(&dir)
            .unwrap_err()
            .starts_with("docs/modules.ron: ")
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// A game's `Cargo.toml` as `shuyi new` makes it, on a checkout.
const GAME: &str = "[workspace]\nmembers = [\"crates/*\"]\n\n[workspace.dependencies]\n\
                    # The engine.\nshuyi = { path = '/e/crates/shuyi' }\ninclude_dir = \"0.7\"\n";

#[test]
fn a_feature_is_turned_on_once_keeping_the_layout() {
    assert_eq!(features_on(GAME), Ok(vec![]));
    let on = turn_on(GAME, "alpha").unwrap().unwrap();
    assert_eq!(
        on,
        GAME.replace(
            "shuyi = { path = '/e/crates/shuyi' }",
            "shuyi = { path = '/e/crates/shuyi', features = [\"alpha\"] }"
        )
    );
    assert_eq!(features_on(&on), Ok(vec!["alpha".to_owned()]));
    assert_eq!(turn_on(&on, "alpha"), Ok(None));
    let both = turn_on(&on, "beta").unwrap().unwrap();
    assert_eq!(
        features_on(&both),
        Ok(vec!["alpha".to_owned(), "beta".to_owned()])
    );
}

#[test]
fn a_version_a_table_and_plain_dependencies_take_a_feature_too() {
    let version = "[dependencies]\nshuyi = \"0.1\"\n";
    let on = turn_on(version, "alpha").unwrap().unwrap();
    assert_eq!(
        on,
        "[dependencies]\nshuyi = { version = \"0.1\", features = [\"alpha\"] }\n"
    );
    let table = "[workspace.dependencies.shuyi]\ngit = \"https://x\"\nfeatures = [\"net\"]\n";
    assert_eq!(features_on(table), Ok(vec!["net".to_owned()]));
    let on = turn_on(table, "alpha").unwrap().unwrap();
    assert_eq!(
        features_on(&on),
        Ok(vec!["net".to_owned(), "alpha".to_owned()])
    );
    assert!(on.starts_with("[workspace.dependencies.shuyi]\ngit = \"https://x\"\n"));
    for none in ["[dependencies]\nserde = \"1\"\n", "", "[workspace]\n"] {
        assert!(
            features_on(none)
                .unwrap_err()
                .contains("no `shuyi` dependency"),
            "{none}"
        );
        assert!(turn_on(none, "alpha").is_err());
    }
    assert!(
        features_on("shuyi = ")
            .unwrap_err()
            .starts_with("Cargo.toml: ")
    );
    assert!(turn_on("[dependencies]\nshuyi = 3\n", "alpha").is_err());
}

#[test]
fn a_shuyi_crate_not_named_by_path_is_where_cargo_resolved_it() {
    // A game whose `shuyi` comes from the workspace's own member: cargo
    // resolves it with no network, as it would a git checkout.
    let dir = folder("resolved");
    write(
        &dir,
        &[
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"engine/crates/shuyi\", \"game\"]\nresolver = \"3\"\n",
            ),
            (
                "engine/crates/shuyi/Cargo.toml",
                "[package]\nname = \"shuyi\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
            ),
            ("engine/crates/shuyi/src/lib.rs", ""),
            (
                "game/Cargo.toml",
                "[package]\nname = \"game\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
                 [dependencies]\nshuyi = { version = \"0.1\", path = \"../engine/crates/shuyi\" }\n",
            ),
            ("game/src/lib.rs", ""),
        ],
    );
    let found = resolved_shuyi(&dir.join("game")).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        fs::canonicalize(found).unwrap(),
        fs::canonicalize(dir.join("engine/crates/shuyi")).unwrap()
    );
    // A folder cargo can't read, and a workspace with no `shuyi`.
    let empty = folder("unresolved");
    let error = resolved_shuyi(&empty).unwrap_err();
    assert!(error.starts_with("cargo metadata failed: "), "{error}");
    write(
        &empty,
        &[
            (
                "Cargo.toml",
                "[package]\nname = \"other\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
            ),
            ("src/lib.rs", ""),
        ],
    );
    assert_eq!(
        resolved_shuyi(&empty),
        Err("cargo metadata names no `shuyi` package for the game".to_owned())
    );
    fs::remove_dir_all(&dir).unwrap();
    fs::remove_dir_all(&empty).unwrap();
}

#[test]
fn the_engine_of_a_game_on_a_checkout_is_the_folder_above_the_shuyi_crate() {
    let dir = folder("engine");
    write(&dir, &[("Cargo.toml", GAME)]);
    // On Windows `/e` is on the game folder's drive.
    let from_the_root = Path::new("/e");
    assert_eq!(engine_root(&dir), Ok(dir.join(from_the_root)));
    write(
        &dir,
        &[(
            "Cargo.toml",
            "[dependencies]\nshuyi = { path = \"../e/crates/shuyi\" }\n",
        )],
    );
    assert_eq!(engine_root(&dir), Ok(dir.join("../e")));
    write(&dir, &[("Cargo.toml", "[dependencies]\n")]);
    assert!(
        engine_root(&dir)
            .unwrap_err()
            .contains("no `shuyi` dependency")
    );
    fs::remove_dir_all(&dir).unwrap();
    assert!(engine_root(&dir).is_err());
}

#[test]
fn a_readmes_first_paragraph_follows_its_title() {
    let dir = folder("readme");
    write(
        &dir,
        &[(
            "README.md",
            "# shuyi-alpha\n\nRigid bodies, for side view\nand top-down games.\n\n## Not for\n\nGrids.\n",
        )],
    );
    assert_eq!(
        readme_paragraph(&dir),
        Ok("Rigid bodies, for side view\nand top-down games.".to_owned())
    );
    write(&dir, &[("README.md", "# Title only\n\n")]);
    assert!(readme_paragraph(&dir).unwrap_err().contains("no paragraph"));
    fs::remove_dir_all(&dir).unwrap();
    assert!(readme_paragraph(&dir).is_err());
}

#[test]
fn a_skill_is_copied_once_into_a_game_that_keeps_skills() {
    let dir = folder("skill");
    write(
        &dir,
        &[
            ("engine/crates/alpha/skill/SKILL.md", "own"),
            ("engine/crates/alpha/skill/reference/more.md", "more"),
            ("engine/skills/shuyi-beta/SKILL.md", "pack"),
            ("engine/crates/beta/Cargo.toml", ""),
        ],
    );
    let alpha = dir.join("engine/crates/alpha");
    let beta = dir.join("engine/crates/beta");
    assert_eq!(skill_of(&alpha, "shuyi-alpha"), Some(alpha.join("skill")));
    let pack = skill_of(&beta, "shuyi-beta").unwrap();
    assert_eq!(fs::read_to_string(pack.join("SKILL.md")).unwrap(), "pack");
    assert_eq!(skill_of(&beta, "shuyi-gamma"), None);

    let game = dir.join("game");
    fs::create_dir_all(&game).unwrap();
    let skill = alpha.join("skill");
    assert_eq!(
        copy_skill(&skill, &game, "shuyi-alpha"),
        Ok(SkillCopy::NoSkills)
    );
    fs::create_dir_all(game.join(SKILLS)).unwrap();
    let to = game.join(SKILLS).join("shuyi-alpha");
    assert_eq!(
        copy_skill(&skill, &game, "shuyi-alpha"),
        Ok(SkillCopy::Copied(to.clone()))
    );
    assert_eq!(
        fs::read_to_string(to.join("reference/more.md")).unwrap(),
        "more"
    );
    fs::write(to.join("SKILL.md"), "the game's own").unwrap();
    assert_eq!(
        copy_skill(&skill, &game, "shuyi-alpha"),
        Ok(SkillCopy::AlreadyThere(to.clone()))
    );
    assert_eq!(
        fs::read_to_string(to.join("SKILL.md")).unwrap(),
        "the game's own"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_game_installs_modules_when_its_code_names_enabled_modules() {
    let dir = folder("installs");
    write(
        &dir,
        &[
            ("crates/game/src/lib.rs", "fn game() {}\n"),
            ("target/x.rs", "shuyi::enabled_modules"),
            (".hidden/x.rs", "shuyi::enabled_modules"),
            ("notes.txt", "shuyi::enabled_modules"),
        ],
    );
    assert!(!installs_modules(&dir));
    write(
        &dir,
        &[(
            "crates/game/src/lib.rs",
            "    .modules(shuyi::enabled_modules)\n",
        )],
    );
    assert!(installs_modules(&dir));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_feature_is_its_modules_name_without_the_prefix() {
    assert_eq!(feature_of("shuyi-physics"), "physics");
    assert_eq!(feature_of("physics"), "physics");
    assert_eq!(module_of("physics"), "shuyi-physics");
    assert_eq!(Status::Parked.name(), "parked");
}
