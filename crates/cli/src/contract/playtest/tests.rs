//! `playtest`'s arguments, its choice of stages and entrants, the tries on
//! threads, and its answer, on the fake game.

use shuyi_core::bots::playtest::{Check, Found};

use super::*;
use crate::contract::fake::{Fake, FakePlaytest, fake_checks};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).into()).collect()
}

/// The fake with `playtest`.
fn fake(playtest: FakePlaytest) -> Fake {
    Fake {
        playtest: Ok(playtest),
        ..Fake::default()
    }
}

/// The fake with stages `stages`.
fn staged(stages: &[&str]) -> Fake {
    fake(FakePlaytest {
        stages: stages.iter().map(|s| (*s).to_owned()).collect(),
        ..FakePlaytest::default()
    })
}

/// An input of `tries` tries and the rest as `more` sets it.
fn input(tries: u32, more: impl FnOnce(&mut PlaytestInput)) -> PlaytestInput {
    let mut input = PlaytestInput {
        tries: Some(tries),
        ..PlaytestInput::default()
    };
    more(&mut input);
    input
}

#[test]
fn the_arguments_name_the_stage_the_entrants_the_tries_and_the_threads() {
    let options = [GameOption {
        flag: "--hard",
        value: None,
        help: "harder",
    }];
    let parsed = parse_args(
        &args(&[
            "--stage",
            "cave",
            "--entrant",
            "walk",
            "--entrant",
            "all",
            "--tries",
            "5",
            "--seed",
            "9",
            "--turns",
            "50",
            "--threads",
            "3",
            "--report",
            "out.json",
            "--hard",
        ]),
        &options,
    )
    .unwrap();
    let want = PlaytestInput {
        stage: Some("cave".to_owned()),
        every_stage: false,
        entrants: args(&["walk", "all"]),
        tries: Some(5),
        seed: Some(9),
        turns: Some(50),
        threads: Some(3),
        start: vec![GameArg {
            flag: "--hard".to_owned(),
            value: None,
        }],
    };
    assert_eq!(parsed.input, want);
    assert_eq!(parsed.report, Some(PathBuf::from("out.json")));
    assert!(
        parse_args(&args(&["--every-stage"]), &[])
            .unwrap()
            .input
            .every_stage
    );
    // Nothing given: every number is the game's.
    assert_eq!(parse_args(&[], &[]).unwrap(), PlaytestArgs::default());
    assert_eq!(
        parse_args(&args(&["3"]), &options).unwrap_err(),
        "no <games> here, only flags: 3"
    );
    assert_eq!(
        parse_args(&args(&["--threads", "x"]), &[]).unwrap_err(),
        "--threads: not a whole number: x"
    );
    for bad in [
        &["3"][..],
        &["--tries"],
        &["--stage"],
        &["--entrant"],
        &["--report"],
        &["--seed", "-1"],
        &["--tries", "9999999999"],
        &["--threads", "9999999999"],
        &["--hard"],
    ] {
        assert!(parse_args(&args(bad), &[]).is_err(), "{bad:?}");
    }
}

#[test]
fn playtest_checks_the_tries_the_seeds_and_the_threads_then_asks_the_game() {
    let game = Fake::default();
    let answer = playtest(&game, &input(1, |_| {}));
    assert!(answer.ok, "{:?}", answer.error);
    assert_eq!(answer.op, Op::Playtest);
    assert!(answer.limits.is_empty());
    assert!(playtest(&game, &input(MAX_GAMES, |_| {})).ok);
    for tries in [0, MAX_GAMES + 1] {
        let answer = playtest(&game, &input(tries, |_| {}));
        assert_eq!(answer.exit_code(), 2);
        let error = answer.error.unwrap();
        assert!(error.starts_with("tries: 1 to 10000"), "{error}");
    }
    assert_eq!(
        playtest(&game, &input(2, |i| i.seed = Some(u64::MAX - 1)))
            .error
            .as_deref(),
        Some("seed: too big for that many tries")
    );
    let none = playtest(&game, &input(2, |i| i.threads = Some(0)));
    assert_eq!(none.error.as_deref(), Some("threads: at least 1"));
    assert_eq!(none.exit_code(), 2);
    // The game's plan doesn't load, or the game doesn't offer one.
    let broken = Fake {
        playtest: Err("no stage".into()),
        ..Fake::default()
    };
    let answer = playtest(&broken, &input(1, |_| {}));
    assert_eq!(answer.error.as_deref(), Some("no stage"));
    assert_eq!(answer.code.as_deref(), Some("SH3002"));
    let left_out = Fake {
        playtest: Err(crate::game::unsupported("playtest", "playtest_plan")),
        ..Fake::default()
    };
    let answer = playtest(&left_out, &input(1, |_| {}));
    assert_eq!(answer.code.as_deref(), Some("SH3001"));
}

#[test]
fn a_number_not_given_is_the_games_default() {
    let game = fake(FakePlaytest {
        defaults: Tries {
            count: 3,
            seed: 1,
            turns: 60,
        },
        ..FakePlaytest::default()
    });
    let answer = playtest(&game, &PlaytestInput::default());
    let report = &answer.result.unwrap()[0].report;
    for entrant in &report.entrants {
        let seeds: Vec<u64> = entrant.tries.iter().map(|t| t.seed).collect();
        assert_eq!(seeds, [1, 2, 3], "{}", entrant.name);
    }
    // The fake's walker stops at the turn limit.
    assert_eq!(report.entrants[0].tries[0].turns, 60);
    // The game's plan is asked for with the numbers filled in, the rest as
    // given.
    let planned = game.planned.lock().unwrap().clone();
    let want = PlaytestInput {
        tries: Some(3),
        seed: Some(1),
        turns: Some(60),
        ..PlaytestInput::default()
    };
    assert_eq!(planned, [want]);
    // A number given wins over the game's.
    let answer = playtest(&game, &input(2, |i| i.turns = Some(9)));
    let report = &answer.result.unwrap()[0].report;
    assert_eq!(report.entrants[0].tries.len(), 2);
    assert_eq!(report.entrants[0].tries[1].seed, 2);
    assert_eq!(report.entrants[0].tries[0].turns, 9);
    // And the engine's, when the game has none of its own.
    assert_eq!(
        PlaytestInput::default().tries(&Fake::default().playtest_defaults()),
        DEFAULT_TRIES
    );
}

#[test]
fn the_entrants_asked_for_play_with_only_their_checks() {
    let game = Fake::default();
    let played = |entrants: &[&str]| {
        let answer = playtest(&game, &input(2, |i| i.entrants = args(entrants)));
        answer.result.unwrap().remove(0)
    };
    let names = |output: &PlaytestOutput| -> Vec<String> {
        output
            .report
            .entrants
            .iter()
            .map(|e| e.name.clone())
            .collect()
    };
    let walk = played(&["walk"]);
    assert_eq!(names(&walk), ["walk"]);
    let checks: Vec<&Check> = walk.report.checks.iter().map(|j| &j.check).collect();
    assert_eq!(checks, fake_checks(&["walk"]).iter().collect::<Vec<_>>());
    assert_eq!(walk.entrants.len(), 1);
    // A group, a name twice, and names out of order: the plan's order.
    for asked in [&["all"][..], &["run", "walk"], &["run", "all", "run"]] {
        let output = played(asked);
        assert_eq!(names(&output), ["walk", "run"], "{asked:?}");
        assert_eq!(output.report.checks.len(), 3, "{asked:?}");
    }
    assert_eq!(names(&played(&[])), ["walk", "run"]);
    // An entrant who isn't there.
    let answer = playtest(&game, &input(2, |i| i.entrants = args(&["nobody"])));
    assert_eq!(answer.exit_code(), 2);
    assert_eq!(
        answer.error.as_deref(),
        Some("entrant: no nobody; the entrants: walk, run; the groups: all")
    );
    // The game's default entrants, when none is asked for.
    let game = fake(FakePlaytest {
        default_entrants: args(&["run"]),
        ..FakePlaytest::default()
    });
    let answer = playtest(&game, &input(2, |_| {}));
    assert_eq!(names(&answer.result.unwrap()[0]), ["run"]);
    let wrong = fake(FakePlaytest {
        default_entrants: args(&["ghost"]),
        ..FakePlaytest::default()
    });
    let answer = playtest(&wrong, &input(2, |_| {}));
    assert_eq!(answer.code.as_deref(), Some("SH3002"));
    assert!(answer.error.unwrap().starts_with("entrant: no ghost;"));
}

#[test]
fn a_game_of_several_stages_plays_the_one_named_or_every_one() {
    let game = staged(&["cave", "hard hill"]);
    let answer = playtest(&game, &input(2, |_| {}));
    assert_eq!(answer.exit_code(), 2);
    assert_eq!(
        answer.error.as_deref(),
        Some("stage: the game has 2 stages, name one or play every stage: cave, hard hill")
    );
    let cave = playtest(&game, &input(2, |i| i.stage = Some("cave".into())));
    assert!(cave.ok);
    let outputs = cave.result.unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].stage.as_deref(), Some("cave"));
    assert_eq!(outputs[0].title, "the fake on cave");
    let hill = playtest(&game, &input(2, |i| i.stage = Some("hard hill".into())));
    assert!(!hill.ok);
    assert_eq!(hill.exit_code(), 1);
    assert!(hill.result.unwrap()[0].report.blocked);
    // Every stage: a report each, failing if either is blocked.
    let every = playtest(&game, &input(2, |i| i.every_stage = true));
    assert_eq!(every.exit_code(), 1);
    let outputs = every.result.unwrap();
    let stages: Vec<Option<&str>> = outputs.iter().map(|o| o.stage.as_deref()).collect();
    assert_eq!(stages, [Some("cave"), Some("hard hill")]);
    let blocked: Vec<bool> = outputs.iter().map(|o| o.report.blocked).collect();
    assert_eq!(blocked, [false, true]);
    assert!(
        playtest(
            &staged(&["cave", "lake"]),
            &input(2, |i| i.every_stage = true)
        )
        .ok
    );
    // Wrong stages.
    let error = |game: &Fake, more: fn(&mut PlaytestInput)| {
        let answer = playtest(game, &input(2, more));
        assert_eq!(answer.exit_code(), 2);
        answer.error.unwrap()
    };
    assert_eq!(
        error(&game, |i| i.stage = Some("moon".into())),
        "stage: no moon; the stages: cave, hard hill"
    );
    assert_eq!(
        error(&game, |i| {
            i.stage = Some("cave".into());
            i.every_stage = true;
        }),
        "stage or every stage, not both"
    );
    assert_eq!(
        error(&Fake::default(), |i| i.stage = Some("cave".into())),
        "stage: the game has no stages, not cave"
    );
    // One stage is played without naming it; no stages, the unnamed one.
    let one = playtest(&staged(&["cave"]), &input(2, |_| {}));
    assert_eq!(one.result.unwrap()[0].stage.as_deref(), Some("cave"));
    let unnamed = playtest(&Fake::default(), &input(2, |i| i.every_stage = true));
    let outputs = unnamed.result.unwrap();
    assert_eq!(outputs.len(), 1);
    assert_eq!(outputs[0].stage, None);
}

#[test]
fn the_report_is_the_same_on_any_number_of_threads() {
    let game = Fake::default();
    let report = |threads: u32| {
        let answer = playtest(&game, &input(25, |i| i.threads = Some(threads)));
        answer.result.unwrap().remove(0)
    };
    let one = report(1);
    for threads in [2, 4, 7, 64] {
        assert_eq!(report(threads).report, one.report, "{threads} threads");
    }
    assert!(!one.report.blocked);
    assert!(matches!(
        one.report.checks[2].found,
        Found::Median {
            median: Some(199),
            than: Some(200),
            ..
        }
    ));
    // Every entrant is timed, with its label.
    let played: Vec<(&str, Option<&str>)> = one
        .entrants
        .iter()
        .map(|p| (p.name.as_str(), p.label.as_deref()))
        .collect();
    assert_eq!(played, [("walk", None), ("run", Some("fast"))]);
    assert!(one.entrants.iter().all(|p| p.seconds_per_try >= 0.0));
}

#[test]
fn an_entrants_time_is_the_mean_of_its_tries() {
    // Four tries of 10 ms each: 10 ms a try, give or take the sleep's
    // overshoot, not their sum.
    let game = fake(FakePlaytest {
        sleep: Duration::from_millis(10),
        ..FakePlaytest::default()
    });
    let answer = playtest(&game, &input(4, |i| i.entrants = args(&["walk"])));
    let seconds = answer.result.unwrap()[0].entrants[0].seconds_per_try;
    assert!((0.01..0.04).contains(&seconds), "{seconds}");
}

#[test]
fn a_try_that_fails_stops_the_playtest_naming_it() {
    let game = fake(FakePlaytest {
        fail: Some(3),
        ..FakePlaytest::default()
    });
    for threads in [1, 4] {
        let answer = playtest(&game, &input(10, |i| i.threads = Some(threads)));
        assert_eq!(answer.error.as_deref(), Some("walk: try 3: refused"));
        assert_eq!(answer.code.as_deref(), Some("SH3002"));
    }
}

#[test]
fn unfinished_tries_are_a_limit() {
    let game = fake(FakePlaytest {
        outcome: None,
        stages: vec!["cave".to_owned()],
        ..FakePlaytest::default()
    });
    let answer = playtest(&game, &input(1, |_| {}));
    assert_eq!(
        answer.limits,
        ["cave: 2 of the tries reached turn 200 without an end: their outcome is unknown"]
    );
    let game = fake(FakePlaytest {
        outcome: None,
        ..FakePlaytest::default()
    });
    let answer = playtest(&game, &input(1, |_| {}));
    assert_eq!(
        answer.limits,
        ["2 of the tries reached turn 200 without an end: their outcome is unknown"]
    );
}

#[test]
fn what_the_playtest_found_reads_for_people() {
    let answer = playtest(
        &staged(&["cave", "lake"]),
        &input(1, |i| i.every_stage = true),
    );
    let mut outputs = answer.result.unwrap();
    outputs[0].entrants[0].seconds_per_try = 2.5;
    outputs[0].entrants[1].seconds_per_try = 0.001;
    assert_eq!(seconds(0.01), "0.01");
    assert_eq!(seconds(0.009), "< 0.01");
    let text = text(&outputs);
    let cave = format!(
        "the fake on cave\n\n{}\nwalk · 2.50 s per try\nrun  · fast · < 0.01 s per try\n",
        outputs[0].report
    );
    assert!(text.starts_with(&cave), "{text}");
    assert!(
        text[cave.len()..].starts_with("\nthe fake on lake\n\n"),
        "{text}"
    );
    let usage = usage(
        "tools",
        &[GameOption {
            flag: "--hard",
            value: None,
            help: "harder\nstill",
        }],
    );
    assert!(usage.starts_with("usage: tools playtest [--stage ID | --every-stage]"));
    assert!(usage.contains("\n  --threads N          the threads to play on"));
    assert!(
        usage.ends_with("\n  --hard               harder\n"),
        "{usage}"
    );
}
