//! A playtest of the stage (`shuyi_core::bots::playtest`): the random
//! hero and the walker each play seeds 0 to 49, judged by
//! `playtest.ron`.

use hello_grid::Hello;
use hello_grid::rules::Stage;
use hello_grid::tools::{ENTRANTS, HelloTools, playtest_checks, playtest_run};
use shuyi_cli::GameArg;
use shuyi_cli::contract::playtest::{PlaytestInput, playtest};
use shuyi_core::bots::playtest::{
    Found, MedianVerdict, PlaytestReport, RateVerdict, Tries, judge, play_tries,
};

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
