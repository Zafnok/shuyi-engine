//! A playtest of the stage (`shuyi::core::bots::playtest`): the random
//! hero and the walker each play seeds 0 to 49, judged by
//! `playtest.ron`, by the core's runner and by the tool contract's
//! `playtest` on threads.

use hello_grid::Hello;
use hello_grid::rules::Stage;
use hello_grid::rules::{Command, Error, Side};
use hello_grid::tools::{ENTRANTS, HelloTools, playtest_checks, playtest_error, playtest_run};
use shuyi::core::bots::playtest::{
    Check, Found, MedianVerdict, PlaytestError, PlaytestReport, RateVerdict, Tries, UnknownEntrant,
    judge, play_tries,
};
use shuyi::core::{PlayError, Pos};
use shuyi_cli::GameArg;
use shuyi_cli::contract::Answer;
use shuyi_cli::contract::bots::{BotsInput, bots};
use shuyi_cli::contract::playtest::{self, PlaytestInput, PlaytestOutput, playtest};
use shuyi_cli::serde_json;

const FIFTY: Tries = Tries::new(50, 0, 200);

fn fifty() -> PlaytestReport {
    playtest_run(&FIFTY).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn every_check_holds_on_seeds_0_to_49() {
    let report = fifty();
    assert!(!report.blocked);
    let found: Vec<&Found> = report.checks.iter().map(|j| &j.found).collect();
    // The numbers ticket 2126's completion notes give.
    assert_eq!(
        found,
        [
            &Found::Rate {
                successes: 5,
                count: 50,
                verdict: RateVerdict::Within,
                blocked: false
            },
            &Found::Rate {
                successes: 50,
                count: 50,
                verdict: RateVerdict::Within,
                blocked: false
            },
            &Found::Median {
                median: Some(4),
                than: Some(28),
                verdict: MedianVerdict::Pass
            },
        ]
    );
    for entrant in &report.entrants {
        assert_eq!(entrant.tries.len(), 50, "{}", entrant.name);
        for t in &entrant.tries {
            assert!(t.outcome.is_some(), "{} seed {}", entrant.name, t.seed);
            assert_eq!(t.measures["turns"], i64::from(t.turns));
            assert!(t.measures.contains_key("foe trips"));
        }
    }
    let table = report.to_string();
    assert!(table.starts_with("50 tries (seeds 0–49)\n"), "{table}");
    assert!(table.contains("walk   · 50/50 succeed"), "{table}");
}

#[test]
fn the_same_tries_give_the_same_report() {
    assert_eq!(fifty(), fifty());
}

#[test]
fn seed_ranges_judged_in_any_order_give_the_same_report() {
    let game = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    let stage = Stage::new(&game.stage_map, 0).unwrap_or_else(|e| panic!("{e}"));
    let mut parts = Vec::new();
    for range in [34..50, 17..34, 0..17] {
        for hero in &ENTRANTS {
            let tries = play_tries(&stage, hero, range.clone(), FIFTY.turns)
                .unwrap_or_else(|e| panic!("{e}"));
            parts.push((hero.name.to_owned(), tries));
        }
    }
    let checks = playtest_checks().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(judge(parts, &checks).unwrap(), fifty());
}

/// `playtest`'s input of fifty tries, with `more`.
fn input(more: impl FnOnce(&mut PlaytestInput)) -> PlaytestInput {
    let mut input = PlaytestInput::default();
    input.tries = Some(50);
    more(&mut input);
    input
}

/// `playtest`'s one report of `input`.
fn answered(input: &PlaytestInput) -> PlaytestOutput {
    let answer = playtest(&HelloTools, input);
    assert!(answer.ok, "{:?}", answer.error);
    let mut outputs = answer.result.unwrap_or_else(|| panic!("no result"));
    assert_eq!(outputs.len(), 1);
    outputs.remove(0)
}

#[test]
fn the_tool_contract_passes_and_takes_no_start_option() {
    let output = answered(&input(|_| {}));
    assert_eq!(output.report, fifty());
    assert_eq!(output.stage, None);
    assert_eq!(output.title, "the stage (files/stage.map)");
    let names: Vec<&str> = output.entrants.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["random", "walk"]);
    let with_option = input(|i| {
        i.start.push(GameArg {
            flag: "--stage".to_owned(),
            value: None,
        });
    });
    let error = playtest(&HelloTools, &with_option).error.unwrap();
    assert_eq!(error, "playtest takes no start option, not --stage");
}

#[test]
fn the_report_is_the_same_on_one_thread_or_four() {
    let one = answered(&input(|i| i.threads = Some(1)));
    let four = answered(&input(|i| i.threads = Some(4)));
    assert_eq!(one.report, fifty());
    assert_eq!(four.report, one.report);
}

#[test]
fn an_entrant_asked_for_plays_alone_with_its_own_checks() {
    let walk = answered(&input(|i| i.entrants = vec!["walk".to_owned()]));
    let names: Vec<&str> = walk
        .report
        .entrants
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(names, ["walk"]);
    // Of playtest.ron's checks, walk's rate alone: the median compares it
    // with the random hero, who doesn't play.
    let checks = playtest_checks().unwrap_or_else(|e| panic!("{e}"));
    let judged: Vec<&Check> = walk.report.checks.iter().map(|j| &j.check).collect();
    assert_eq!(judged, [&checks[1]]);
    assert_eq!(walk.report.entrants[0], fifty().entrants[1]);
    let nobody = playtest(
        &HelloTools,
        &input(|i| i.entrants = vec!["nobody".to_owned()]),
    );
    assert_eq!(
        nobody.error.as_deref(),
        Some("entrant: no nobody; the entrants: random, walk")
    );
}

/// The command `playtest` with `more` arguments, its history in `dir`'s
/// `history`: the answer it wrote to `dir`'s `out.json`.
fn command(dir: &std::path::Path, more: &[&str]) -> PlaytestOutput {
    let file = dir.join("out.json");
    let history = dir.join("history");
    let path = |p: &std::path::Path| p.to_string_lossy().into_owned();
    let mut args: Vec<String> = ["playtest", "--tries", "50", "--threads", "3", "--brief"]
        .iter()
        .chain(more)
        .map(|a| (*a).to_owned())
        .collect();
    args.extend([
        "--history".to_owned(),
        path(&history),
        "--report".to_owned(),
        path(&file),
    ]);
    let code = shuyi_cli::main_with(args.into_iter(), Some(&HelloTools));
    assert_eq!(code, 0);
    let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("{e}"));
    let answer: Answer<Vec<PlaytestOutput>> =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    assert!(answer.ok, "{:?}", answer.error);
    let mut outputs = answer.result.unwrap_or_else(|| panic!("no result"));
    outputs.remove(0)
}

#[test]
fn the_command_keeps_a_history_and_the_second_run_says_what_the_first_was() {
    let dir = std::env::temp_dir().join(format!("hello-grid-playtest-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let first = command(&dir, &[]);
    assert_eq!(first.report, fifty());
    // Each entrant's speed and summary are in the answer.
    assert_eq!(first.entrants.len(), 2);
    assert!(first.entrants.iter().all(|p| p.seconds_per_try > 0.0));
    assert_eq!(first.entrants[0].summary[0], "won 5/50");
    assert_eq!(first.entrants[1].summary[0], "won 50/50");
    assert!(first.entrants[1].summary[1].starts_with("turns median "));
    assert_eq!(first.previous, None);
    assert!(first.entrants.iter().all(|p| p.was.is_none()));

    let second = command(&dir, &[]);
    let previous = second
        .previous
        .clone()
        .unwrap_or_else(|| panic!("no previous"));
    assert!(previous.date.ends_with(" UTC"), "{}", previous.date);
    let was: Vec<Option<&str>> = second.entrants.iter().map(|p| p.was.as_deref()).collect();
    assert_eq!(was, [Some("was 5/50 succeed"), Some("was 50/50 succeed")]);
    let text = playtest::text(std::slice::from_ref(&second));
    assert!(text.contains("random · won 5/50 · turns median "), "{text}");
    assert!(text.contains(" s per try · was 5/50 succeed\n"), "{text}");
    assert!(
        text.ends_with(&format!(
            "previous {} · commit {}\n",
            previous.date, previous.commit
        )),
        "{text}"
    );
    let kept = dir.join("history").join("stage-random+walk.jsonl");
    assert_eq!(std::fs::read_to_string(&kept).unwrap().lines().count(), 2);

    // Other entrants keep another file, and have nothing to compare with.
    let walk = command(&dir, &["--entrant", "walk"]);
    assert_eq!(walk.previous, None);
    assert_eq!(walk.entrants[0].was, None);
    assert!(dir.join("history").join("stage-walk.jsonl").is_file());
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_random_hero_trips_the_foe_as_often_as_in_bots() {
    // `bots` plays the same games: a random hero of the seed, the chaser.
    let mut fifty_games = BotsInput::default();
    fifty_games.games = 50;
    let plays = bots(&HelloTools, &fifty_games).result.unwrap().plays;
    let report = fifty();
    let random = &report.entrants[0];
    assert_eq!(random.name, "random");
    let trips: Vec<i64> = random
        .tries
        .iter()
        .map(|t| t.measures["foe trips"])
        .collect();
    let counted: Vec<i64> = plays.iter().map(|p| p.measures["foe trips"]).collect();
    assert_eq!(trips, counted);
    assert!(trips.iter().any(|&n| n > 1), "{trips:?}");
}

#[test]
fn a_playtest_error_says_what_went_wrong() {
    let unknown: PlaytestError<Stage> = UnknownEntrant::new(0, "nobody").into();
    assert_eq!(
        unknown.to_string(),
        "check 1 names `nobody`, who isn't in the run"
    );
    let refused: PlaytestError<Stage> = PlaytestError::play(
        "walk",
        PlayError::new(Command::Move(Pos::new(0, 0)), Side::Hero, 3, Error::Over),
    );
    assert_eq!(
        refused.to_string(),
        "walk: turn 3, Hero to move: Move(Pos { x: 0, y: 0 }) refused: the stage is over"
    );
    // The tools blame `playtest.ron` for a check naming nobody who plays,
    // and say a refusal as it is.
    assert_eq!(
        playtest_error(&unknown),
        "playtest.ron: check 1 names `nobody`, who isn't in the run"
    );
    assert_eq!(playtest_error(&refused), refused.to_string());
}
