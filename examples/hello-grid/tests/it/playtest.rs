//! A playtest of the stage (`shuyi_core::bots::playtest`): the random
//! hero and the walker each play seeds 0 to 49, judged by
//! `playtest.ron`.

use hello_grid::Hello;
use hello_grid::rules::Stage;
use hello_grid::rules::{Command, Error, Side};
use hello_grid::tools::{ENTRANTS, HelloTools, playtest_checks, playtest_run};
use shuyi_cli::GameArg;
use shuyi_cli::contract::bots::{BotsInput, bots};
use shuyi_cli::contract::playtest::{PlaytestInput, playtest};
use shuyi_core::bots::playtest::{
    Found, MedianVerdict, PlaytestError, PlaytestReport, RateVerdict, Tries, UnknownEntrant, judge,
    play_tries,
};
use shuyi_core::{PlayError, Pos};

const FIFTY: Tries = Tries {
    count: 50,
    seed: 0,
    turns: 200,
};

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

#[test]
fn the_tool_contract_passes_and_takes_no_start_option() {
    let input = PlaytestInput {
        tries: 50,
        ..PlaytestInput::default()
    };
    let answer = playtest(&HelloTools, &input);
    assert!(answer.ok, "{:?}", answer.error);
    assert_eq!(answer.result.unwrap(), fifty());
    let mut with_option = input;
    with_option.start.push(GameArg {
        flag: "--stage".to_owned(),
        value: None,
    });
    let error = playtest(&HelloTools, &with_option).error.unwrap();
    assert_eq!(error, "playtest takes no start option, not --stage");
}

#[test]
fn the_random_hero_trips_the_foe_as_often_as_in_bots() {
    // `bots` plays the same games: a random hero of the seed, the chaser.
    let plays = bots(
        &HelloTools,
        &BotsInput {
            games: 50,
            ..BotsInput::default()
        },
    )
    .result
    .unwrap()
    .plays;
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
    let unknown: PlaytestError<Stage> = UnknownEntrant {
        check: 0,
        entrant: "nobody".to_owned(),
    }
    .into();
    assert_eq!(
        unknown.to_string(),
        "check 1 names `nobody`, who isn't in the run"
    );
    let refused: PlaytestError<Stage> = PlaytestError::Play {
        entrant: "walk".to_owned(),
        error: PlayError {
            command: Command::Move(Pos::new(0, 0)),
            side: Side::Hero,
            turn: 3,
            error: Error::Over,
        },
    };
    assert_eq!(
        refused.to_string(),
        "walk: turn 3, Hero to move: Move(Pos { x: 0, y: 0 }) refused: the stage is over"
    );
}
