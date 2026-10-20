//! The templates and what `new` makes of them.

use super::*;

/// A fresh empty folder under the system temp folder.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-new-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_crate_name_is_lowercase_words_joined_by_dashes() {
    assert_eq!(crate_name("Moon Garden").unwrap(), "moon-garden");
    assert_eq!(crate_name("  Moon -- Garden 2 ").unwrap(), "moon-garden-2");
    assert_eq!(crate_name("demo").unwrap(), "demo");
    assert!(crate_name("2048").is_err());
    assert!(crate_name("é").is_err());
    assert!(crate_name("").is_err());
}

#[test]
fn a_name_must_not_need_escaping() {
    assert!(check_name("Moon Garden").is_ok());
    for bad in ["", "  ", "a\"b", r"a\b", "a{b", "a}b", "a\nb"] {
        assert!(check_name(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn every_template_fills_every_placeholder() {
    for template in Template::ALL {
        assert_eq!(Template::named(template.name()), Some(template));
        for engine in [
            Engine::Main,
            Engine::Rev {
                rev: "abc1234".to_owned(),
            },
            Engine::Path {
                path: "x".to_owned(),
            },
        ] {
            let root = matches!(engine, Engine::Path { .. }).then_some("/e");
            let files = files(template, "Moon Garden", "moon-garden", &engine, root).unwrap();
            assert!(files.iter().any(|(p, _)| p == "Cargo.toml"), "{template:?}");
            assert!(files.iter().any(|(p, _)| p == "AGENTS.md"));
            assert!(files.iter().any(|(p, _)| p == "shuyi.ron"));
            assert!(files.iter().any(|(p, _)| p == ".github/workflows/ci.yml"));
            for (path, bytes) in &files {
                if let Ok(text) = std::str::from_utf8(bytes) {
                    assert!(!text.contains("{{"), "{path}");
                }
            }
            let manifest = &files.iter().find(|(p, _)| p == "Cargo.toml").unwrap().1;
            let manifest = String::from_utf8_lossy(manifest);
            let expected = match &engine {
                Engine::Main => format!("{{ git = \"{ENGINE_GIT}\", branch = \"main\" }}"),
                Engine::Rev { rev } => format!("{{ git = \"{ENGINE_GIT}\", rev = \"{rev}\" }}"),
                Engine::Path { .. } => "{ path = '/e/crates/ui' }".to_owned(),
            };
            assert!(
                manifest.contains(&format!("shuyi-ui = {expected}")),
                "{manifest}"
            );
        }
    }
    assert_eq!(Template::named("platformer"), None);
}

#[test]
fn every_templates_game_ships_on_the_desktop_and_the_web_and_fits_them() {
    for template in Template::ALL {
        let files = files(template, "Moon Garden", "moon-garden", &Engine::Main, None).unwrap();
        let text = &files.iter().find(|(p, _)| p == "shuyi.ron").unwrap().1;
        let config = crate::project::Config::parse(&String::from_utf8_lossy(text)).unwrap();
        let ids: Vec<&str> = config.targets.keys().map(String::as_str).collect();
        assert_eq!(ids, ["desktop", "web"], "{template:?}");
        let input = crate::contract::check::CheckInput {
            target: Some("all".to_owned()),
            ..crate::contract::check::CheckInput::default()
        };
        let answer = crate::contract::check::targets(&config, Path::new("."), &input);
        assert!(answer.ok, "{template:?}: {answer:?}");
    }
}

#[test]
fn an_unknown_placeholder_is_an_error_naming_its_file_and_line() {
    let error = fill("a.rs", "one\ntwo {{nope}} three", "N", "n", &[]).unwrap_err();
    assert_eq!(
        error,
        "the template's a.rs:2 has an unknown placeholder {{nope}}"
    );
    assert_eq!(
        fill("b", "{{name}} is {{crate}}", "Moon", "moon", &[]).unwrap(),
        "Moon is moon"
    );
    let unclosed = fill("c", "{{", "N", "n", &[]).unwrap_err();
    assert!(unclosed.ends_with("placeholder {{"), "{unclosed}");
}

#[test]
fn new_makes_the_game_in_an_empty_folder_only() {
    let dir = scratch("make");
    let input = NewInput {
        template: Template::Blank,
        dir: "Moon Garden".to_owned(),
        name: None,
        engine: Engine::Main,
    };
    let answer = new(&dir, &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.as_ref().unwrap();
    assert_eq!(
        (out.name.as_str(), out.crate_name.as_str()),
        ("Moon Garden", "moon-garden")
    );
    assert!(out.files.contains(&"Cargo.toml".to_owned()));
    let made = fs::read_to_string(dir.join("Moon Garden/README.md")).unwrap();
    assert!(made.starts_with("# Moon Garden\n"), "{made}");
    assert!(
        answer.limits[0].contains("main branch"),
        "{:?}",
        answer.limits
    );
    assert!(answer.limits.last().unwrap().contains("not built"));
    // Not again on top of it.
    let again = new(&dir, &input);
    assert_eq!(again.error.as_deref(), Some("Moon Garden is not empty"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_engine_is_a_commit_or_a_checkout() {
    let dir = scratch("engine");
    let input = |engine| NewInput {
        template: Template::Blank,
        dir: "g".to_owned(),
        name: Some("Game".to_owned()),
        engine,
    };
    let bad_rev = new(
        &dir,
        &input(Engine::Rev {
            rev: "main".to_owned(),
        }),
    );
    assert_eq!(
        bad_rev.error.as_deref(),
        Some("--engine-rev: not a commit: main")
    );
    let no_engine = new(
        &dir,
        &input(Engine::Path {
            path: "nowhere".to_owned(),
        }),
    );
    assert!(
        no_engine
            .error
            .unwrap()
            .contains("no checkout of the engine")
    );
    // This repository is one.
    let engine = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let answer = new(
        &dir,
        &input(Engine::Path {
            path: engine.to_string_lossy().into_owned(),
        }),
    );
    assert!(answer.ok, "{answer:?}");
    let manifest = fs::read_to_string(dir.join("g/Cargo.toml")).unwrap();
    assert!(manifest.contains("/crates/ui' }"), "{manifest}");
    assert!(!manifest.contains('\\'), "{manifest}");
    assert!(answer.limits[0].contains("engine checkout"));
    let rev = input(Engine::Rev {
        rev: "0123abcd".to_owned(),
    });
    let pinned = new(
        &dir,
        &NewInput {
            dir: "h".to_owned(),
            ..rev
        },
    );
    assert!(pinned.ok, "{pinned:?}");
    assert!(!pinned.limits[0].contains("main branch"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_arguments_name_a_template_and_a_folder() {
    let args = |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).into()).collect() };
    let input = parse_args(&args(&[
        "grid",
        "g",
        "--name",
        "G",
        "--engine-rev",
        "abc1234",
    ]))
    .unwrap();
    assert_eq!(input.template, Template::Grid);
    assert_eq!(input.name.as_deref(), Some("G"));
    assert_eq!(
        input.engine,
        Engine::Rev {
            rev: "abc1234".to_owned()
        }
    );
    let path = parse_args(&args(&["blank", "g", "--engine-path", ".."])).unwrap();
    assert_eq!(
        path.engine,
        Engine::Path {
            path: "..".to_owned()
        }
    );
    assert!(parse_args(&args(&["grid"])).is_err());
    assert!(parse_args(&args(&["grid", "a", "b"])).is_err());
    assert!(parse_args(&args(&["grid", "a", "--name"])).is_err());
    assert_eq!(
        parse_args(&args(&["grid", "a", "--bogus"])).unwrap_err(),
        "unknown option: --bogus"
    );
    assert_eq!(
        parse_args(&args(&["grid", "--bogus"])).unwrap_err(),
        "unknown option: --bogus"
    );
    let unknown = parse_args(&args(&["platformer", "a"])).unwrap_err();
    assert_eq!(
        unknown,
        "no template platformer: the templates are grid, blank"
    );
    assert!(usage("shuyi").starts_with("usage: shuyi new <template> <dir>"));
}

#[test]
fn a_commit_is_seven_to_forty_hex_digits() {
    let dir = scratch("revs");
    let make = |dir_name: &str, rev: &str| {
        new(
            &dir,
            &NewInput {
                template: Template::Blank,
                dir: dir_name.to_owned(),
                name: Some("Game".to_owned()),
                engine: Engine::Rev {
                    rev: rev.to_owned(),
                },
            },
        )
    };
    assert!(make("a", "abc1234").ok);
    assert!(make("b", &"f".repeat(40)).ok);
    for (i, bad) in ["abc123", &"f".repeat(41), "zzzzzzz", "abc123g"]
        .iter()
        .enumerate()
    {
        let answer = make(&format!("bad{i}"), bad);
        assert_eq!(
            answer.error.as_deref(),
            Some(format!("--engine-rev: not a commit: {bad}").as_str())
        );
    }
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_engine_path_with_a_quote_is_refused() {
    let dir = scratch("quote");
    let engine = dir.join("it's");
    fs::create_dir_all(engine.join("crates/core")).unwrap();
    fs::write(engine.join("crates/core/Cargo.toml"), "").unwrap();
    let error = engine_root(&dir, "it's").unwrap_err();
    assert!(
        error.starts_with("the engine's path may not hold a quote: "),
        "{error}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_name_from_the_folder_must_make_a_crate_name() {
    let dir = scratch("folder");
    let answer = new(
        &dir,
        &NewInput {
            template: Template::Blank,
            dir: "123".to_owned(),
            name: None,
            engine: Engine::Main,
        },
    );
    assert!(answer.error.unwrap().contains("makes no crate name"));
    assert!(!dir.join("123").exists());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn what_new_made_reads_for_people() {
    let out = NewOutput {
        dir: "g".into(),
        name: "Game".into(),
        crate_name: "game".into(),
        files: vec!["Cargo.toml".into(), "README.md".into()],
    };
    assert_eq!(
        text(&out),
        "new: Game (game) in g, 2 file(s)\n  Cargo.toml\n  README.md\n"
    );
}
