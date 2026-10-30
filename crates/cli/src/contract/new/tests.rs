//! What `new` makes of the engine's template, and its arguments.

use super::*;

/// A fresh empty folder under the system temp folder.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-new-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The engine's template.
fn built_in() -> Template {
    template::built_in().unwrap_or_else(|e| panic!("{e}"))
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
fn every_short_name_fills_every_placeholder_on_every_engine() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for short in ["grid", "blank"] {
        for engine in [
            Engine::Main,
            Engine::Rev {
                rev: "abc1234".to_owned(),
            },
            Engine::Path {
                path: ".".to_owned(),
            },
        ] {
            let mut input = NewInput::new("g").template(short);
            input.engine = engine.clone();
            let (plan, _) = plan(&cwd, &template, &input, "Moon Garden", "moon-garden")
                .unwrap_or_else(|(_, e)| panic!("{short}: {e}"));
            let paths = plan.paths();
            for file in [
                "Cargo.toml",
                "AGENTS.md",
                "shuyi.ron",
                ".github/workflows/ci.yml",
            ] {
                assert!(paths.iter().any(|p| p == file), "{short}: {file}");
            }
            let manifest = String::from_utf8_lossy(&plan.files[Path::new("Cargo.toml")]);
            let expected = match &engine {
                Engine::Main => format!("{{ git = \"{ENGINE_GIT}\", branch = \"main\" }}"),
                Engine::Rev { rev } => format!("{{ git = \"{ENGINE_GIT}\", rev = \"{rev}\" }}"),
                _ => "/crates/shuyi' }".to_owned(),
            };
            assert!(manifest.contains(&expected), "{manifest}");
        }
    }
}

#[test]
fn the_grid_game_calls_the_engines_playtest_workflow_at_its_engine_commit() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let full = "0123456789abcdef0123456789abcdef01234567";
    let call = "uses: Zafnok/shuyi-engine/.github/workflows/playtest.yml@";
    let pinned = |rev: &str| Engine::Rev {
        rev: rev.to_owned(),
    };
    for (engine, at, limited) in [
        (pinned(full), full, false),
        (pinned("abc1234"), "main", true),
        (Engine::Main, "main", false),
    ] {
        let mut input = NewInput::new("g").template("grid");
        input.engine = engine;
        let (plan, limits) = plan(cwd, &template, &input, "Moon", "moon").unwrap();
        let workflow = String::from_utf8_lossy(&plan.files[Path::new(PLAYTEST_WORKFLOW)]);
        assert!(workflow.contains(&format!("{call}{at} ")), "{workflow}");
        assert!(workflow.contains("cancel-in-progress: ${{ github.event_name != 'push' }}"));
        let says = limits.iter().any(|l| l.contains("40 digits"));
        assert_eq!(says, limited, "{limits:?}");
    }
    // The blank game has no stage to play.
    let (plan, _) = plan(
        cwd,
        &template,
        &NewInput::new("g").template("blank"),
        "Moon",
        "moon",
    )
    .unwrap();
    assert!(!plan.files.contains_key(Path::new(PLAYTEST_WORKFLOW)));
}

#[test]
fn every_short_names_game_ships_on_the_desktop_and_the_web_and_fits_them() {
    let template = built_in();
    for short in ["grid", "blank"] {
        for (name, krate) in [("Moon", "moon"), ("Lune Café", "lune-caf")] {
            let input = NewInput::new("g").template(short);
            let (plan, _) = plan(Path::new("."), &template, &input, name, krate).unwrap();
            let dir = scratch(&format!("fits-{short}-{krate}"));
            template::apply(&plan, &dir, false).unwrap();
            let project = crate::project::Project::load(&dir).unwrap();
            let config = &project.config;
            let ids: Vec<&str> = config.targets.keys().map(String::as_str).collect();
            assert_eq!(ids, ["desktop", "web"], "{short}");
            // The game's `app_info.rs` is the one `app-info` writes.
            let app = config.app.as_ref().unwrap();
            assert_eq!(app.name, name);
            let command = config.command();
            assert!(
                crate::app_info::stale(app, &dir, command).is_empty(),
                "{short} {name}"
            );
            let input = crate::contract::check::CheckInput {
                target: Some("all".to_owned()),
                ..crate::contract::check::CheckInput::default()
            };
            let answer = crate::contract::check::targets(config, &dir, &input, None);
            assert!(answer.ok, "{short}: {answer:?}");
            // The one problem: the placeholder id, on each target.
            for report in &answer.result.as_ref().unwrap().targets {
                let found: Vec<(&str, &str)> = report
                    .problems
                    .iter()
                    .map(|p| {
                        (
                            p.setting.as_str(),
                            &p.message[..p.message.find(' ').unwrap()],
                        )
                    })
                    .collect();
                let id = format!("\"com.example.{krate}\"");
                assert_eq!(found, [("app.id", id.as_str())], "{short} {name}");
            }
            fs::remove_dir_all(&dir).unwrap();
        }
    }
}

#[test]
fn an_answers_file_makes_the_same_bytes_as_the_set_flags_and_the_short_name() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut from_file = NewInput::new("g");
    from_file.answers = Some("tests/fixtures/answers.ron".to_owned());
    let from_set = parse_args(&[
        "g".into(),
        "--set".into(),
        "perspective=top-down-grid".into(),
    ])
    .unwrap();
    let from_name = NewInput::new("g").template("grid");
    let made = |input: &NewInput| plan(cwd, &template, input, "Moon", "moon").unwrap().0;
    let file_plan = made(&from_file);
    assert_eq!(file_plan, made(&from_set));
    assert_eq!(file_plan, made(&from_name));
    assert_eq!(file_plan.stages, ["top-down-grid"]);
    // A later source answers in place of an earlier one.
    let mut blank_again = from_file.clone();
    blank_again.set = Answers::parse("set", "{ \"perspective\": [] }").unwrap();
    let stages = made(&blank_again).stages;
    assert!(stages.is_empty(), "{stages:?}");
}

#[test]
fn answers_the_template_doesnt_take_are_refused() {
    let template = built_in();
    let cwd = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut wrong = NewInput::new("g");
    wrong.set = Answers::parse("set", "{ \"perspective\": [\"side-view\"] }").unwrap();
    let (code, error) = plan(cwd, &template, &wrong, "Moon", "moon").unwrap_err();
    assert_eq!(code, codes::WRONG_ARGUMENTS);
    assert!(
        error.contains("`side-view` is no answer of `perspective`"),
        "{error}"
    );
    let (code, error) = plan(
        cwd,
        &template,
        &NewInput::new("g").template("platformer"),
        "Moon",
        "moon",
    )
    .unwrap_err();
    assert_eq!(code, codes::WRONG_ARGUMENTS);
    assert_eq!(
        error,
        "no template platformer: the templates are blank, grid"
    );
    let mut missing = NewInput::new("g");
    missing.answers = Some("tests/fixtures/nowhere.ron".to_owned());
    assert!(plan(cwd, &template, &missing, "Moon", "moon").is_err());
}

#[test]
fn new_makes_the_game_in_an_empty_folder_only_unless_forced() {
    let dir = scratch("make");
    let input = NewInput::new("Moon Garden");
    let answer = new(&dir, &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.as_ref().unwrap();
    assert_eq!(
        (out.name.as_str(), out.crate_name.as_str()),
        ("Moon Garden", "moon-garden")
    );
    assert_eq!(
        (out.template.as_str(), out.parts.as_slice()),
        ("shuyi-default", &["base".to_owned()][..])
    );
    assert!(out.files.contains(&"Cargo.toml".to_owned()));
    assert!(out.overwritten.is_empty() && out.modules.is_empty() && out.stages.is_empty());
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
    let error = again.error.as_deref().unwrap();
    assert!(
        error.starts_with("Moon Garden is not empty (it would write over .cargo/config.toml, "),
        "{error}"
    );
    assert!(
        error.ends_with(": pass --force to write into it"),
        "{error}"
    );
    assert_eq!(again.code.as_deref(), Some("SH0001"));
    assert_eq!(again.exit_code(), 2);
    // With --force, over it, saying so.
    let mut forced = input.clone();
    forced.force = true;
    let answer = new(&dir, &forced);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap();
    assert_eq!(out.overwritten, out.files);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_engine_is_a_commit_or_a_checkout() {
    let dir = scratch("engine");
    let input = |engine| {
        let mut input = NewInput::new("g");
        input.name = Some("Game".to_owned());
        input.engine = engine;
        input
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
    assert!(manifest.contains("/crates/shuyi' }"), "{manifest}");
    assert!(!manifest.contains('\\'), "{manifest}");
    assert!(answer.limits[0].contains("engine checkout"));
    let mut pinned = input(Engine::Rev {
        rev: "0123abcd".to_owned(),
    });
    pinned.dir = "h".to_owned();
    let pinned = new(&dir, &pinned);
    assert!(pinned.ok, "{pinned:?}");
    assert!(!pinned.limits[0].contains("main branch"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_arguments_name_a_folder_and_the_answers() {
    let args = |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).into()).collect() };
    let input = parse_args(&args(&[
        "g",
        "--template",
        "grid",
        "--name",
        "G",
        "--engine-rev",
        "abc1234",
        "--answers",
        "a.ron",
        "--set",
        "perspective=top-down-grid",
        "--set",
        "music=none,recorded",
        "--force",
    ]))
    .unwrap();
    assert_eq!(input.dir, "g");
    assert_eq!(input.template.as_deref(), Some("grid"));
    assert_eq!(input.answers.as_deref(), Some("a.ron"));
    assert_eq!(input.name.as_deref(), Some("G"));
    assert!(input.force);
    assert_eq!(input.set.0["music"], ["none", "recorded"]);
    assert_eq!(input.set.0["perspective"], ["top-down-grid"]);
    assert_eq!(
        input.engine,
        Engine::Rev {
            rev: "abc1234".to_owned()
        }
    );
    // The older form: a template, then the folder.
    let older = parse_args(&args(&["blank", "g", "--engine-path", ".."])).unwrap();
    assert_eq!(
        (older.template.as_deref(), older.dir.as_str()),
        (Some("blank"), "g")
    );
    assert_eq!(
        older.engine,
        Engine::Path {
            path: "..".to_owned()
        }
    );
    assert!(parse_args(&args(&[])).is_err());
    assert!(parse_args(&args(&["a", "b", "c"])).is_err());
    assert!(parse_args(&args(&["a", "b", "--template", "grid"])).is_err());
    assert!(parse_args(&args(&["a", "--name"])).is_err());
    assert!(parse_args(&args(&["a", "--set", "nothing"])).is_err());
    assert_eq!(
        parse_args(&args(&["a", "--bogus"])).unwrap_err(),
        "unknown option: --bogus"
    );
    let usage = usage("shuyi");
    assert!(
        usage.starts_with("usage: shuyi new <dir> [--template NAME]"),
        "{usage}"
    );
    assert!(usage.contains("(blank, grid)"), "{usage}");
}

#[test]
fn a_commit_is_seven_to_forty_hex_digits() {
    let dir = scratch("revs");
    let make = |dir_name: &str, rev: &str| {
        let mut input = NewInput::new(dir_name);
        input.name = Some("Game".to_owned());
        input.engine = Engine::Rev {
            rev: rev.to_owned(),
        };
        new(&dir, &input)
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
    let answer = new(&dir, &NewInput::new("123"));
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
        template: "shuyi-default".into(),
        answers: Answers::default(),
        parts: vec!["base".into(), "stage".into()],
        files: vec!["Cargo.toml".into(), "README.md".into()],
        overwritten: vec!["README.md".into()],
        modules: vec!["shuyi-physics".into()],
        targets: vec!["desktop".into()],
        stages: Vec::new(),
        skills: vec!["shuyi-gates".into(), "shuyi-test".into()],
    };
    assert_eq!(
        text(&out),
        "new: Game (game) in g, 2 file(s), from shuyi-default: base, stage\n\
         modules on: shuyi-physics\nskills: shuyi-gates, shuyi-test\nwritten over: README.md\n  \
         Cargo.toml\n  README.md\n"
    );
}

#[test]
fn a_new_input_names_the_game_for_its_folder_on_mains_engine() {
    let input = NewInput::new("games/moon");
    assert_eq!(input.dir, "games/moon");
    assert!(input.template.is_none() && input.answers.is_none() && input.set.0.is_empty());
    assert_eq!(
        (input.name, input.engine, input.force),
        (None, Engine::Main, false)
    );
    assert_eq!(
        NewInput::new("g").template("grid").template.as_deref(),
        Some("grid")
    );
}

/// The engine's checkout, this crate's grandparent.
fn engine_checkout() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The game each of the engine's template's short names makes, by its
/// short name.
fn every_short_names_game() -> Vec<(String, Plan)> {
    let template = built_in();
    template
        .short_names
        .keys()
        .map(|short| {
            let input = NewInput::new("g").template(short);
            let (plan, _) = plan(&engine_checkout(), &template, &input, "Moon", "moon")
                .unwrap_or_else(|(_, e)| panic!("{short}: {e}"));
            (short.clone(), plan)
        })
        .collect()
}

/// The words of the script's `NAME="…"` line.
fn script_list<'a>(script: &'a str, name: &str) -> Vec<&'a str> {
    let line = script
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{name}=\"")))
        .unwrap_or_else(|| panic!("no {name}= line"));
    line.strip_suffix('"').unwrap().split_whitespace().collect()
}

#[test]
fn every_setup_script_is_the_engines_and_installs_doctors_table() {
    use crate::contract::doctor::linux::ROWS;
    let engine = fs::read(engine_checkout().join("scripts/setup-linux.sh")).unwrap();
    assert!(!engine.contains(&b'\r'));
    let script = String::from_utf8(engine.clone()).unwrap();
    let column = |pick: fn(&crate::contract::doctor::linux::Row) -> &'static str| {
        ROWS.iter().map(pick).collect::<Vec<_>>()
    };
    assert_eq!(script_list(&script, "APT"), column(|r| r.apt));
    assert_eq!(script_list(&script, "DNF"), column(|r| r.dnf));
    assert_eq!(script_list(&script, "PACMAN"), column(|r| r.pacman));
    for (short, plan) in every_short_names_game() {
        let copy = &plan.files[Path::new("scripts/setup-linux.sh")];
        assert!(
            copy == &engine,
            "{short}: scripts/setup-linux.sh is not the engine's"
        );
    }
}

#[test]
fn every_dev_container_is_set_up_as_the_engines() {
    let read = |bytes: &[u8]| -> serde_json::Value {
        serde_json::from_slice(bytes).unwrap_or_else(|e| panic!("{e}"))
    };
    let engine =
        read(&fs::read(engine_checkout().join(".devcontainer/devcontainer.json")).unwrap());
    assert_eq!(engine["name"], "Shuyi Engine");
    let setup = "bash scripts/setup-linux.sh && . \"$HOME/.cargo/env\" && ";
    let engine_command = engine["postCreateCommand"].as_str().unwrap();
    assert_eq!(
        engine_command,
        format!(
            "{setup}cargo run -q -p shuyi-cli -- mcp install --yes --command \"cargo run -q -p shuyi-cli --\""
        )
    );
    for (short, plan) in every_short_names_game() {
        let game = read(&plan.files[Path::new(".devcontainer/devcontainer.json")]);
        assert_eq!(game["name"], "Moon", "{short}");
        for key in ["image", "remoteEnv", "customizations"] {
            assert_eq!(game[key], engine[key], "{short}: {key}");
        }
        assert_eq!(
            game["postCreateCommand"].as_str().unwrap(),
            format!("{setup}cargo shuyi mcp install --yes"),
            "{short}"
        );
    }
}

#[test]
fn no_workflow_installs_doctors_packages_by_hand() {
    use crate::contract::doctor::linux::ROWS;
    let mut workflows = Vec::new();
    for entry in fs::read_dir(engine_checkout().join(".github/workflows")).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        workflows.push((path.display().to_string(), text));
    }
    for (short, plan) in every_short_names_game() {
        for (path, bytes) in &plan.files {
            if path.starts_with(".github/workflows") {
                let text = String::from_utf8(bytes.clone()).unwrap();
                workflows.push((format!("{short}: {}", path.display()), text));
            }
        }
    }
    assert!(workflows.len() > 3, "{}", workflows.len());
    let packages: Vec<&str> = ROWS.iter().flat_map(|r| [r.apt, r.dnf, r.pacman]).collect();
    for (file, text) in &workflows {
        for line in text.lines().filter(|l| l.contains("apt-get install")) {
            let words = line.split(|c: char| c.is_whitespace() || c == '"' || c == '\'');
            for word in words {
                assert!(
                    !packages.contains(&word),
                    "{file} installs {word} by hand: call `bash scripts/setup-linux.sh`\n{line}"
                );
            }
        }
    }
    // The playtest workflow's default packages, for a game with no script
    // of its own, are the table's.
    let (_, playtest) = workflows
        .iter()
        .find(|(file, _)| file.ends_with("playtest.yml") && !file.contains(": "))
        .unwrap();
    let default = playtest
        .split("system-packages:")
        .nth(1)
        .and_then(|input| {
            input
                .lines()
                .find_map(|l| l.trim().strip_prefix("default: "))
        })
        .unwrap();
    let apt: Vec<&str> = ROWS.iter().map(|r| r.apt).collect();
    for package in default.split_whitespace() {
        assert!(apt.contains(&package), "playtest.yml's default: {package}");
    }
}
