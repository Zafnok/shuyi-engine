//! `autobalance`'s arguments, its checks of them, the search on threads,
//! the budget, the time limit, `apply` and its answer, on the fake game.

use shuyi_core::bots::playtest::{Check, Found};

use super::*;
use crate::contract::fake::{Fake, FakeBalance, FakePlaytest, fake_checks};

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).into()).collect()
}

/// An input of ten tries, every run of the search in full, and the rest
/// as `more` sets it.
fn input(more: impl FnOnce(&mut AutobalanceInput)) -> AutobalanceInput {
    let mut input = AutobalanceInput {
        tries: Some(10),
        quick: Some(10),
        ..AutobalanceInput::default()
    };
    more(&mut input);
    input
}

/// The fake with `balance`.
fn fake(balance: FakeBalance) -> Fake {
    Fake {
        balance: Ok(balance),
        ..Fake::default()
    }
}

/// What `autobalance` found on `game` with `input`, which ran.
fn found(game: &Fake, input: &AutobalanceInput) -> AutobalanceOutput {
    let answer = autobalance(game, input);
    assert!(answer.error.is_none(), "{:?}", answer.error);
    answer.result.unwrap()
}

/// The successes of `report`'s first rate, `walk`'s.
fn walk_won(report: &PlaytestReport) -> u32 {
    match report.checks[0].found {
        Found::Rate { successes, .. } => successes,
        Found::Median { .. } => panic!("{report:?}"),
    }
}

#[test]
fn the_arguments_name_the_stage_the_entrants_the_search_and_its_limits() {
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
            "50",
            "--quick",
            "10",
            "--budget",
            "40",
            "--time",
            "600",
            "--seed",
            "9",
            "--turns",
            "30",
            "--threads",
            "3",
            "--apply",
            "--hard",
        ]),
        &options,
    )
    .unwrap();
    let want = AutobalanceInput {
        stage: Some("cave".to_owned()),
        entrants: args(&["walk", "all"]),
        tries: Some(50),
        quick: Some(10),
        budget: Some(40),
        time: Some(600),
        seed: Some(9),
        turns: Some(30),
        threads: Some(3),
        apply: true,
        start: vec![GameArg {
            flag: "--hard".to_owned(),
            value: None,
        }],
    };
    assert_eq!(parsed, want);
    // Nothing given: every number is the default.
    assert_eq!(parse_args(&[], &[]).unwrap(), AutobalanceInput::default());
    assert_eq!(
        parse_args(&args(&["3"]), &options).unwrap_err(),
        "no <games> here, only flags: 3"
    );
    assert_eq!(
        parse_args(&args(&["--time", "soon"]), &[]).unwrap_err(),
        "--time: not a whole number: soon"
    );
    for bad in [
        &["--tries"][..],
        &["--quick", "-1"],
        &["--budget", "9999999999"],
        &["--time"],
        &["--stage"],
        &["--entrant"],
        &["--threads", "x"],
        &["--hard"],
    ] {
        assert!(parse_args(&args(bad), &[]).is_err(), "{bad:?}");
    }
}

#[test]
fn a_number_not_given_is_the_default_and_the_plan_is_asked_with_them() {
    let defaults = Tries {
        count: 100,
        seed: 3,
        turns: 50,
    };
    let given = AutobalanceInput::default();
    assert_eq!(
        given.settings(&defaults),
        BalanceSettings {
            tries: defaults,
            quick: 20,
            evaluations: DEFAULT_BUDGET,
        }
    );
    // A fifth of the tries, at least one.
    let few = AutobalanceInput {
        tries: Some(4),
        ..AutobalanceInput::default()
    };
    assert_eq!(few.settings(&defaults).quick, 1);
    let game = Fake {
        playtest: Ok(FakePlaytest {
            defaults,
            ..FakePlaytest::default()
        }),
        ..Fake::default()
    };
    let output = found(&game, &AutobalanceInput::default());
    assert_eq!(output.settings, given.settings(&defaults));
    let planned = game.balanced.lock().unwrap().clone();
    let want = AutobalanceInput {
        tries: Some(100),
        quick: Some(20),
        budget: Some(100),
        seed: Some(3),
        turns: Some(50),
        ..AutobalanceInput::default()
    };
    assert_eq!(planned, [want]);
}

#[test]
fn autobalance_checks_its_numbers_and_the_stage_then_asks_the_game() {
    let game = Fake::default();
    let wrong = |input: &AutobalanceInput| {
        let answer = autobalance(&game, input);
        assert_eq!(answer.exit_code(), 2, "{input:?}");
        answer.error.unwrap()
    };
    assert_eq!(
        wrong(&input(|i| i.tries = Some(0))),
        "tries: 1 to 10000, not 0"
    );
    assert_eq!(
        wrong(&input(|i| i.quick = Some(11))),
        "quick: 1 to the tries (10), not 11"
    );
    assert_eq!(
        wrong(&input(|i| i.quick = Some(0))),
        "quick: 1 to the tries (10), not 0"
    );
    assert_eq!(
        wrong(&input(|i| i.budget = Some(0))),
        "budget: 1 to 10000, not 0"
    );
    assert_eq!(
        wrong(&input(|i| i.budget = Some(MAX_BUDGET + 1))),
        "budget: 1 to 10000, not 10001"
    );
    assert_eq!(
        wrong(&input(|i| i.threads = Some(0))),
        "threads: at least 1"
    );
    assert_eq!(
        wrong(&input(|i| i.seed = Some(u64::MAX))),
        "seed: too big for that many tries"
    );
    assert_eq!(
        wrong(&input(|i| i.stage = Some("cave".into()))),
        "stage: the game has no stages, not cave"
    );
    assert_eq!(
        wrong(&input(|i| i.entrants = args(&["nobody"]))),
        "entrant: no nobody; the entrants: walk, run; the groups: all"
    );
    // The game's plan doesn't load, or the game doesn't offer one.
    let broken = Fake {
        balance: Err("no stage".into()),
        ..Fake::default()
    };
    let answer = autobalance(&broken, &input(|_| {}));
    assert_eq!(answer.error.as_deref(), Some("no stage"));
    assert_eq!(answer.code.as_deref(), Some("SH3002"));
    let left_out = Fake {
        balance: Err(crate::game::unsupported("autobalance", "autobalance_plan")),
        ..Fake::default()
    };
    let answer = autobalance(&left_out, &input(|_| {}));
    assert_eq!(answer.code.as_deref(), Some("SH3001"));
    assert_eq!(answer.op, Op::Autobalance);
}

#[test]
fn a_game_of_several_stages_searches_the_one_named() {
    let staged = |stages: &[&str]| Fake {
        playtest: Ok(FakePlaytest {
            stages: stages.iter().map(|s| (*s).to_owned()).collect(),
            ..FakePlaytest::default()
        }),
        ..Fake::default()
    };
    let game = staged(&["cave", "lake"]);
    let answer = autobalance(&game, &input(|_| {}));
    assert_eq!(answer.exit_code(), 2);
    assert_eq!(
        answer.error.as_deref(),
        Some("stage: the game has 2 stages, name one: cave, lake")
    );
    let lake = found(&game, &input(|i| i.stage = Some("lake".into())));
    assert_eq!(lake.stage.as_deref(), Some("lake"));
    assert_eq!(lake.title, "the fake on lake");
    let moon = autobalance(&game, &input(|i| i.stage = Some("moon".into())));
    assert_eq!(
        moon.error.as_deref(),
        Some("stage: no moon; the stages: cave, lake")
    );
    // One stage is searched without naming it.
    let one = found(&staged(&["cave"]), &input(|_| {}));
    assert_eq!(one.stage.as_deref(), Some("cave"));
}

#[test]
fn a_stage_too_hard_is_eased_into_its_band() {
    // Hard 8 wins 2 tries in 10, the band wants 5: ease 1 (3 of 10), 2
    // (4) and 3 (5, in it); the prune finds 2 not.
    let game = Fake::default();
    let answer = autobalance(&game, &input(|_| {}));
    assert!(answer.ok, "{:?}", answer.error);
    assert!(answer.limits.is_empty(), "{:?}", answer.limits);
    let output = answer.result.unwrap();
    assert!(output.in_bands);
    assert_eq!(output.stop, Stop::InBands);
    assert_eq!(
        output.changes,
        [Change {
            knob: "ease".to_owned(),
            step: -3,
            what: "3 easier".to_owned(),
        }]
    );
    assert_eq!((walk_won(&output.before), walk_won(&output.after)), (2, 5));
    assert_eq!(output.evaluations, 4);
    assert_eq!(output.written, None);
    assert!(game.wrote.lock().unwrap().is_empty());
    assert!(output.seconds >= 0.0);
    // At the default quick runs of 20 tries, the stage as authored plays
    // quick too, and -3 and -2 in full: three runs more.
    let output = found(&game, &AutobalanceInput::default());
    assert_eq!((output.changes[0].step, output.evaluations), (-3, 7));
}

#[test]
fn what_it_finds_is_the_same_on_any_number_of_threads() {
    let game = Fake::default();
    let on = |threads: u32| {
        let mut output = found(&game, &input(|i| i.threads = Some(threads)));
        output.seconds = 0.0;
        output
    };
    let one = on(1);
    for threads in [2, 4, 64] {
        assert_eq!(on(threads), one, "{threads} threads");
    }
}

#[test]
fn the_entrants_asked_for_play_with_only_their_checks() {
    let output = found(&Fake::default(), &input(|i| i.entrants = args(&["walk"])));
    let names: Vec<&str> = output
        .after
        .entrants
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert_eq!(names, ["walk"]);
    let checks: Vec<&Check> = output.after.checks.iter().map(|j| &j.check).collect();
    assert_eq!(checks, fake_checks(&["walk"]).iter().collect::<Vec<_>>());
    assert!(output.in_bands);
}

#[test]
fn a_budget_of_three_stops_with_the_best_setting_seen() {
    // Before, ease 1 and ease 2: no evaluation left for 3.
    let answer = autobalance(&Fake::default(), &input(|i| i.budget = Some(3)));
    assert_eq!(answer.exit_code(), 1);
    assert!(answer.error.is_none());
    let output = answer.result.unwrap();
    assert_eq!((output.stop, output.evaluations), (Stop::Budget, 3));
    assert!(!output.in_bands);
    assert_eq!(output.changes[0].step, -2);
    assert_eq!(walk_won(&output.after), 4);
}

#[test]
fn a_time_of_0_stops_before_the_second_evaluation() {
    let answer = autobalance(&Fake::default(), &input(|i| i.time = Some(0)));
    assert_eq!(answer.exit_code(), 1);
    let output = answer.result.unwrap();
    assert_eq!((output.stop, output.evaluations), (Stop::Stopped, 1));
    // The best setting seen: the stage as authored.
    assert!(output.changes.is_empty());
    assert_eq!(output.after, output.before);
    assert_eq!(
        answer.limits,
        [
            "the time limit of 0 s ran out after 1 evaluations: the best setting seen, not what \
          more time would find (it depends on the machine)"
        ]
    );
    // A time that doesn't run out changes nothing.
    let mut timed = found(&Fake::default(), &input(|i| i.time = Some(3600)));
    let mut untimed = found(&Fake::default(), &input(|_| {}));
    (timed.seconds, untimed.seconds) = (0.0, 0.0);
    assert_eq!(timed, untimed);
}

#[test]
fn apply_has_the_game_write_the_changed_stage_and_only_then() {
    let game = Fake::default();
    let output = found(&game, &input(|i| i.apply = true));
    assert_eq!(output.written, Some(vec!["stage.ron".to_owned()]));
    // Hard 8 eased by 3.
    assert_eq!(*game.wrote.lock().unwrap(), [5]);
    found(&game, &input(|_| {}));
    assert_eq!(*game.wrote.lock().unwrap(), [5]);
    // A stage already in its band is not written.
    let easy = fake(FakeBalance {
        hard: 5,
        ..FakeBalance::default()
    });
    let output = found(&easy, &input(|i| i.apply = true));
    assert_eq!(output.stop, Stop::AlreadyInBands);
    assert_eq!(output.written, Some(Vec::new()));
    assert!(easy.wrote.lock().unwrap().is_empty());
    // A stage the game can't write fails the answer, which keeps what the
    // search found.
    let stuck = fake(FakeBalance {
        write_fails: Some("read-only"),
        ..FakeBalance::default()
    });
    let answer = autobalance(&stuck, &input(|i| i.apply = true));
    assert!(!answer.ok);
    assert_eq!(answer.code.as_deref(), Some("SH3002"));
    assert_eq!(
        answer.error.as_deref(),
        Some("apply: the game couldn't write the stage: read-only")
    );
    let output = answer.result.unwrap();
    assert!(output.in_bands);
    assert_eq!(output.written, None);
}

#[test]
fn unfinished_tries_are_a_limit() {
    let game = fake(FakeBalance {
        unfinished: true,
        ..FakeBalance::default()
    });
    let answer = autobalance(&game, &input(|i| i.turns = Some(30)));
    assert_eq!(
        answer.limits,
        ["after: 20 of the tries reached turn 30 without an end: their outcome is unknown"]
    );
}

#[test]
fn what_it_found_reads_for_people() {
    let mut output = found(&Fake::default(), &input(|i| i.apply = true));
    output.seconds = 1.5;
    let said = text(&output);
    assert!(
        said.starts_with(
            "the fake on its stage\n\nin every band · 4 evaluations\nease -3: 3 easier\n\n\
             before: 10 tries (seeds 0–9)\n"
        ),
        "{said}"
    );
    assert!(said.contains("\nafter: 10 tries (seeds 0–9)\n"), "{said}");
    assert!(
        said.ends_with("\napply: wrote stage.ron\n\ntook 1.50 s\n"),
        "{said}"
    );
    output.written = Some(Vec::new());
    assert!(text(&output).contains("\napply: nothing changed, nothing written\n"));
    output.written = None;
    output.title = String::new();
    let plain = text(&output);
    assert!(plain.starts_with("in every band"), "{plain}");
    assert!(!plain.contains("apply:"), "{plain}");
    let usage = usage(
        "tools",
        &[GameOption {
            flag: "--hard",
            value: None,
            help: "harder",
        }],
    );
    assert!(usage.starts_with("usage: tools autobalance [--stage ID]"));
    assert!(usage.contains("\n  --time SECONDS"), "{usage}");
    assert!(usage.contains("\n  --apply "), "{usage}");
    assert!(
        usage.ends_with("\n  --hard               harder\n"),
        "{usage}"
    );
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

#[test]
fn the_json_answer_has_the_fields_of_its_schema() {
    let [_, schema] = crate::contract::op_schemas(Op::Autobalance, false);
    let schema: serde_json::Value = serde_json::from_str(&schema.text).unwrap();
    let output = &schema["$defs"]["AutobalanceOutput"];
    for apply in [false, true] {
        let answer = autobalance(&Fake::default(), &input(|i| i.apply = apply));
        let json: serde_json::Value = serde_json::from_str(&answer.to_json()).unwrap();
        assert_eq!(keys(&json), keys(&schema["properties"]));
        assert_eq!(keys(&json["result"]), keys(&output["properties"]));
        for required in output["required"].as_array().unwrap() {
            let field = required.as_str().unwrap();
            assert!(!json["result"][field].is_null(), "{field}");
        }
        assert_eq!(json["result"]["stop"], "InBands");
        // It reads back as the answer it is.
        let read: Answer<AutobalanceOutput> = serde_json::from_value(json).unwrap();
        assert_eq!(read, answer);
    }
}
