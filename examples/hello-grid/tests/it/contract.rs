//! The tool contract (ticket 1129) on Hello, Grid: each operation through
//! the game's tools, as `cargo shuyi <op>` runs it in the game's tool
//! program. `new` and `doctor` need no game; they run on a scratch folder
//! and this machine.

use std::fs;
use std::path::{Path, PathBuf};

use hello_grid::tools::{HelloTools, StageRecord};
use shuyi_cli::GameTools;
use shuyi_cli::contract::bots::{BotsInput, bots};
use shuyi_cli::contract::check::{CheckInput, check};
use shuyi_cli::contract::doctor::{DoctorInput, ThisMachine, doctor};
use shuyi_cli::contract::import::{ImportInput, import};
use shuyi_cli::contract::new::{Engine, NewInput, Template, new};
use shuyi_cli::contract::replay::{ReplayInput, replay};
use shuyi_cli::contract::run::{RunInput, keys, run, scene};
use shuyi_cli::contract::session::Event;
use shuyi_cli::{GameArg, contract};

/// The game's folder, its project's root.
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// A fresh empty folder under the system temp folder.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hello-grid-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    dir
}

/// The start option `--stage`.
fn on_the_stage() -> Vec<GameArg> {
    vec![GameArg {
        flag: "--stage".to_owned(),
        value: None,
    }]
}

#[test]
fn check_finds_no_error_in_the_games_files() {
    let answer = check(&HelloTools, &CheckInput::default());
    assert!(answer.ok, "{answer:?}");
    assert_eq!(answer.op, contract::Op::Check);
    let got = answer.result.unwrap().errors;
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn run_plays_from_the_title_into_the_stages_scene() {
    let input = RunInput {
        script: keys("Enter"),
        frames: 2,
        ..RunInput::default()
    };
    let answer = run(&HelloTools, &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap();
    assert_eq!(out.frames, 4);
    assert_eq!(out.screens, ["title", "stage", "talk"]);
    assert_eq!(out.top_screen, "talk");
    assert_eq!(
        out.events[..2],
        [
            Event::Opened {
                frame: 1,
                screen: "stage".into()
            },
            Event::Opened {
                frame: 2,
                screen: "talk".into()
            },
        ]
    );
    assert_eq!(out.sounds[0].cue, "ui_select");
    // The stage is under the scene: its map is the scene.
    let scene = out.scene.unwrap();
    assert_eq!(
        (scene.screen.as_str(), scene.kind.as_str()),
        ("stage", "grid map")
    );
    assert_eq!(scene.data["turn"], 1);
    assert!(out.frame.contains("Turn 1"), "{}", out.frame);
    assert!(answer.limits.is_empty(), "{:?}", answer.limits);
}

#[test]
fn run_says_what_it_could_not_see() {
    // Quit from the title: Down three times, then Enter.
    let input = RunInput {
        script: keys("Down Down Down Enter"),
        frames: 5,
        ..RunInput::default()
    };
    let answer = run(&HelloTools, &input);
    let out = answer.result.as_ref().unwrap();
    assert_eq!(out.quit, Some(7));
    assert!(out.events.contains(&Event::Quit { frame: 7 }));
    assert!(out.scene.is_none());
    assert_eq!(
        answer.limits,
        [
            "the game asked to quit on frame 7: the 6 frame(s) after it changed nothing",
            "no scene: no stage is open (the top screen is `title`)",
        ]
    );
    // A script the Harness can't play is refused before it starts.
    let bad = RunInput {
        script: keys("NoSuchKey"),
        ..RunInput::default()
    };
    let refused = run(&HelloTools, &bad);
    assert!(!refused.ok);
    assert!(refused.error.unwrap().starts_with("keys: "));
    let unknown = RunInput {
        start: vec![GameArg {
            flag: "--hard".into(),
            value: None,
        }],
        ..RunInput::default()
    };
    assert_eq!(
        run(&HelloTools, &unknown).error.as_deref(),
        Some("unknown option --hard")
    );
}

#[test]
fn scene_dumps_the_stage_and_its_state() {
    let input = RunInput {
        start: on_the_stage(),
        script: keys("z Right"),
        ..RunInput::default()
    };
    let answer = scene(&HelloTools, &input);
    assert!(answer.ok, "{answer:?}");
    let scene = answer.result.unwrap().scene.unwrap();
    assert!(scene.text.contains("@ (1,8)"), "{}", scene.text);
    assert!(scene.text.contains("path (1,8) (2,8)"), "{}", scene.text);
    assert_eq!(scene.data["hero"]["x"], 1);
    assert_eq!(scene.data["to_move"], "Hero");
    // At the title there is none, and it fails.
    let title = scene_at_title();
    assert!(!title.ok);
    assert!(title.limits[0].starts_with("no scene: no stage is open"));
}

fn scene_at_title() -> contract::Answer<contract::run::SceneOutput> {
    scene(&HelloTools, &RunInput::default())
}

#[test]
fn bots_play_the_stage_and_measure_it() {
    let input = BotsInput {
        games: 30,
        seed: 0,
        turns: 200,
        start: Vec::new(),
    };
    let answer = bots(&HelloTools, &input);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap();
    assert_eq!(out.games, 30);
    assert_eq!(out.outcomes.values().sum::<u32>(), 30);
    assert!(out.outcomes.contains_key("won") && out.outcomes.contains_key("caught"));
    let trips = &out.measures["foe trips"];
    assert!(trips.total > 0 && trips.min >= 0);
    let turns = out.turns.as_ref().unwrap();
    assert!(turns.min >= 1 && turns.max <= 200);
    assert_eq!(out.plays[3].seed, 3);
    // The same seeds play the same games.
    assert_eq!(bots(&HelloTools, &input).result.unwrap(), out);
    let none = BotsInput { games: 0, ..input };
    assert!(
        bots(&HelloTools, &none)
            .error
            .unwrap()
            .starts_with("games: ")
    );
}

#[test]
fn replay_hashes_every_step_and_finds_where_two_records_part() {
    let dir = scratch("replay");
    let plays = bots(
        &HelloTools,
        &BotsInput {
            games: 2,
            ..BotsInput::default()
        },
    )
    .result
    .unwrap()
    .plays;
    let (a, b) = (
        plays[0].record.clone().unwrap(),
        plays[1].record.clone().unwrap(),
    );
    fs::write(dir.join("a.ron"), &a).unwrap();
    fs::write(dir.join("b.ron"), &b).unwrap();
    let once = ReplayInput {
        record: "a.ron".to_owned(),
        other: None,
    };
    let answer = replay(&HelloTools, &dir, &once);
    assert!(answer.ok, "{answer:?}");
    let out = answer.result.unwrap();
    let record: StageRecord = ron::from_str(&a).unwrap();
    assert_eq!(out.runs[0].steps.len(), record.commands.len() + 1);
    assert_eq!(out.runs[0], out.runs[1]);
    // The bots' play ended, and its replay ends the same way.
    let outcome = out.runs[0].outcome.as_deref().unwrap();
    assert!(
        outcome.starts_with("Won") || outcome.starts_with("Caught"),
        "{outcome}"
    );
    assert!(out.first_difference.is_none());
    // Two records of different seeds part at the start.
    let two = ReplayInput {
        record: "a.ron".to_owned(),
        other: Some("b.ron".to_owned()),
    };
    let answer = replay(&HelloTools, &dir, &two);
    assert!(!answer.ok);
    let difference = answer.result.unwrap().first_difference.unwrap();
    assert_eq!(difference.step, 0);
    // A command the rules refuse is said, and changes nothing.
    let mut bad = record;
    bad.commands
        .insert(0, hello_grid::rules::Command::Step(None));
    fs::write(dir.join("bad.ron"), ron::to_string(&bad).unwrap()).unwrap();
    let answer = replay(
        &HelloTools,
        &dir,
        &ReplayInput {
            record: "bad.ron".to_owned(),
            other: None,
        },
    );
    let run = &answer.result.as_ref().unwrap().runs[0];
    assert_eq!(run.steps[1].refused.as_deref(), Some("it is Hero's move"));
    assert_eq!(run.steps[1].hash, run.steps[0].hash);
    assert!(
        answer
            .limits
            .iter()
            .any(|l| l.contains("refused 1 command"))
    );
    let missing = replay(
        &HelloTools,
        &dir,
        &ReplayInput {
            record: "nope.ron".to_owned(),
            other: None,
        },
    );
    assert!(missing.error.unwrap().starts_with("nope.ron: "));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn import_reads_the_games_files_as_their_kinds() {
    let importers = HelloTools.importers();
    let read = |file: &str, print: bool| {
        import(
            &importers,
            root(),
            &ImportInput {
                file: file.to_owned(),
                kind: None,
                print,
            },
        )
    };
    let map = read("files/stage.map", false);
    assert!(map.ok, "{map:?}");
    let out = map.result.unwrap();
    assert_eq!(out.read_as.len(), 1);
    assert_eq!(out.read_as[0].summary, "map: 12 tiles by 10, 2 piece(s)");
    assert_eq!(out.read_as[0].text, None);
    let tiles = read("files/tiles.png", false).result.unwrap();
    assert_eq!(tiles.read_as[0].summary, "image: 192×24 pixels");
    // A font atlas reads its picture beside it.
    let font = read("files/font/atlas.ron", false).result.unwrap();
    assert_eq!(font.read_as[0].kind, "font atlas");
    let palette = read("files/palette.ron", true);
    let printed = palette.result.as_ref().unwrap().read_as[0]
        .text
        .clone()
        .unwrap();
    assert!(printed.contains("\"floor_fg\""), "{printed}");
    assert!(palette.limits[0].starts_with("no writer of our text format for the palette"));
}

#[test]
fn import_reports_every_error_with_its_line() {
    let dir = scratch("import");
    fs::write(dir.join("broken.map"), "(\n    name: 3,\n)\n---\n").unwrap();
    fs::write(dir.join("notes.txt"), "hello").unwrap();
    let importers = HelloTools.importers();
    let read = |file: &str| {
        import(
            &importers,
            &dir,
            &ImportInput {
                file: file.to_owned(),
                kind: None,
                print: false,
            },
        )
    };
    let broken = read("broken.map");
    assert!(!broken.ok);
    let errors = broken.result.unwrap().errors;
    assert!(!errors.is_empty(), "{errors:?}");
    assert_eq!(errors[0].file, "broken.map");
    assert!(errors[0].line.is_some(), "{errors:?}");
    let unknown = read("notes.txt");
    assert!(
        unknown.result.unwrap().errors[0]
            .message
            .starts_with("no format of these kinds")
    );
    assert!(unknown.limits[0].contains("Tiled, LDtk and Aseprite"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn doctor_finds_the_rust_this_test_was_built_with() {
    let answer = doctor(&ThisMachine, &DoctorInput::default());
    let out = answer.result.unwrap();
    assert!(out.host.is_some());
    let rust = &out.checks[0];
    assert_eq!(rust.name, "rust");
    assert!(rust.ok, "{rust:?}");
    assert!(rust.found.as_deref().unwrap().starts_with("rustc "));
}

#[test]
fn new_makes_a_game_from_each_template() {
    let dir = scratch("new");
    for template in Template::ALL {
        let input = NewInput {
            template,
            dir: template.name().to_owned(),
            name: Some(format!("Demo {}", template.name())),
            engine: Engine::Path {
                path: root().join("../..").to_string_lossy().into_owned(),
            },
        };
        let answer = new(&dir, &input);
        assert!(answer.ok, "{answer:?}");
        let made = dir.join(template.name());
        let manifest = fs::read_to_string(made.join("crates/game/Cargo.toml")).unwrap();
        assert!(
            manifest.contains(&format!("name = \"demo-{}\"", template.name())),
            "{manifest}"
        );
        assert!(made.join("AGENTS.md").is_file());
        assert!(made.join("shuyi.ron").is_file());
        assert!(made.join(".github/workflows/ci.yml").is_file());
    }
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn run_lists_the_rules_events_after_the_engines() {
    // Pick your piece up, steer it a tile right, send it; then the foe.
    let input = RunInput {
        start: on_the_stage(),
        script: keys("z Right z"),
        frames: 60,
    };
    let out = run(&HelloTools, &input).result.unwrap();
    let game: Vec<&str> = out
        .events
        .iter()
        .filter_map(|e| match e {
            Event::Game { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(game[0].starts_with("Moved"), "{game:?}");
    assert!(
        game[1].starts_with("Stepped") || game[1] == "Tripped",
        "{game:?}"
    );
    assert!(matches!(out.events.last(), Some(Event::Game { .. })));
}
