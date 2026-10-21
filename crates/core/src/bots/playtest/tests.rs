//! Tests of the playtest runner: the checks on hand-made tries, and runs
//! of a dice race.

use alloc::string::ToString;
use alloc::vec;

use proptest::prelude::*;

use super::*;
use crate::bots::fixtures::{DiceRace, Nim, Race};
use crate::turn::TurnRules;

/// A try of `seed` with `measures`.
fn tried(seed: u64, measures: &[(&str, i64)]) -> Tried {
    Tried {
        seed,
        outcome: Some("0".to_string()),
        turns: 1,
        measures: measures.iter().map(|&(n, v)| (n.to_string(), v)).collect(),
    }
}

/// `count` tries of which the first `successes` have `won` 1.
fn tries(successes: u32, count: u32) -> Vec<Tried> {
    (0..count)
        .map(|i| tried(u64::from(i), &[("won", i64::from(i < successes))]))
        .collect()
}

fn won() -> Vec<Condition> {
    vec![Condition {
        measure: "won".to_string(),
        min: Some(1),
        max: None,
    }]
}

fn rate(entrant: &str, min: u32, max: u32, floor: Option<u32>) -> Check {
    Check::Rate {
        entrant: entrant.to_string(),
        success: won(),
        band: Band { min, max },
        floor,
    }
}

fn median_check(entrant: &str, want: Want, than: &str) -> Check {
    Check::Median {
        entrant: entrant.to_string(),
        measure: "turns".to_string(),
        among: won(),
        want,
        than: than.to_string(),
    }
}

fn found(report: &PlaytestReport, i: usize) -> &Found {
    &report.checks[i].found
}

#[test]
fn a_band_holds_both_its_ends() {
    let verdicts: Vec<RateVerdict> = [59, 60, 85, 86]
        .into_iter()
        .map(|s| {
            let report = judge(
                vec![("a".into(), tries(s, 100))],
                &[rate("a", 60, 85, None)],
            );
            match found(&report.unwrap(), 0) {
                Found::Rate { verdict, .. } => *verdict,
                other @ Found::Median { .. } => panic!("{other:?}"),
            }
        })
        .collect();
    assert_eq!(
        verdicts,
        [
            RateVerdict::Below,
            RateVerdict::Within,
            RateVerdict::Within,
            RateVerdict::Above
        ]
    );
}

#[test]
fn a_floor_blocks_a_run_under_it() {
    let blocked = |s| {
        judge(
            vec![("a".into(), tries(s, 100))],
            &[rate("a", 0, 100, Some(20))],
        )
        .unwrap()
        .blocked
    };
    assert!(blocked(19));
    assert!(!blocked(20));
    let none = judge(
        vec![("a".into(), tries(0, 100))],
        &[rate("a", 50, 60, None)],
    );
    assert!(!none.unwrap().blocked);
}

#[test]
fn the_median_of_an_even_count_is_the_lower_middle() {
    assert_eq!(median(vec![3, 1, 4, 2]), Some(2));
    assert_eq!(median(vec![3, 1, 2]), Some(2));
    assert_eq!(median(Vec::new()), None);
}

#[test]
fn a_median_check_compares_two_entrants() {
    let with_turns = |turns: &[i64]| -> Vec<Tried> {
        turns
            .iter()
            .zip(0..)
            .map(|(&t, seed)| tried(seed, &[("won", 1), ("turns", t)]))
            .collect()
    };
    let tries = vec![
        ("fast".to_string(), with_turns(&[3, 1, 4, 2])),
        ("slow".to_string(), with_turns(&[9, 7])),
        ("never".to_string(), tries(0, 5)),
    ];
    let checks = [
        median_check("fast", Want::Lower, "slow"),
        median_check("fast", Want::Higher, "slow"),
        median_check("slow", Want::Higher, "fast"),
        median_check("fast", Want::Lower, "never"),
        median_check("fast", Want::Lower, "fast"),
        median_check("fast", Want::Higher, "fast"),
    ];
    let report = judge(tries, &checks).unwrap();
    let verdicts: Vec<(Option<i64>, Option<i64>, MedianVerdict)> = report
        .checks
        .iter()
        .map(|j| match j.found {
            Found::Median {
                median,
                than,
                verdict,
            } => (median, than, verdict),
            Found::Rate { .. } => panic!("{j:?}"),
        })
        .collect();
    assert_eq!(
        verdicts,
        [
            (Some(2), Some(7), MedianVerdict::Pass),
            (Some(2), Some(7), MedianVerdict::Fail),
            (Some(7), Some(2), MedianVerdict::Pass),
            (Some(2), None, MedianVerdict::Unknown),
            (Some(2), Some(2), MedianVerdict::Fail),
            (Some(2), Some(2), MedianVerdict::Fail),
        ]
    );
}

#[test]
fn a_missing_measure_fails_its_condition() {
    let all = Condition {
        measure: "won".to_string(),
        min: None,
        max: None,
    };
    assert!(all.met_by(&tried(0, &[("won", -5)]).measures));
    assert!(!all.met_by(&tried(0, &[("lost", 1)]).measures));
    let tries = vec![tried(0, &[("lost", 1)]), tried(1, &[("won", 1)])];
    let report = judge(vec![("a".into(), tries)], &[rate("a", 50, 50, None)]).unwrap();
    assert!(matches!(
        found(&report, 0),
        Found::Rate {
            successes: 1,
            count: 2,
            verdict: RateVerdict::Within,
            ..
        }
    ));
    let bounded = Condition {
        measure: "won".to_string(),
        min: Some(1),
        max: Some(2),
    };
    let met: Vec<bool> = [0, 1, 2, 3]
        .into_iter()
        .map(|v| bounded.met_by(&tried(0, &[("won", v)]).measures))
        .collect();
    assert_eq!(met, [false, true, true, false]);
}

/// A dice race played by side 0 always stepping or always rolling, side 1
/// always stepping; its measures `won` (side 0 won) and `turns`.
struct Racer {
    name: &'static str,
    command: Race,
}

struct Watch {
    moves: i64,
}

impl Measure<DiceRace> for Watch {
    fn observe(&mut self, _: &DiceRace, _: &Race, _: &[u32]) {
        self.moves += 1;
    }

    fn finish(self: Box<Self>, report: &Report<DiceRace>, end: &DiceRace) -> Measures {
        assert_eq!(report.outcome, end.outcome());
        [
            ("won".to_string(), i64::from(report.outcome == Some(0))),
            ("turns".to_string(), i64::from(report.turns)),
            ("moves".to_string(), self.moves),
        ]
        .into()
    }
}

impl Entrant<(), DiceRace> for Racer {
    fn name(&self) -> &str {
        self.name
    }

    fn start(&self, (): &(), seed: u64) -> DiceRace {
        DiceRace::new(seed)
    }

    fn bots(&self, _: u64) -> Vec<(usize, Box<dyn Bot<DiceRace>>)> {
        let command = self.command;
        vec![
            (0, Box::new(move |_: &DiceRace| command)),
            (1, Box::new(|_: &DiceRace| Race::Step)),
        ]
    }

    fn measure(&self) -> Box<dyn Measure<DiceRace>> {
        Box::new(Watch { moves: 0 })
    }
}

const STEPPER: Racer = Racer {
    name: "stepper",
    command: Race::Step,
};
const ROLLER: Racer = Racer {
    name: "roller",
    command: Race::Roll,
};

fn race_checks() -> Vec<Check> {
    vec![
        rate("stepper", 100, 100, Some(100)),
        rate("roller", 50, 100, Some(1)),
        median_check("roller", Want::Lower, "stepper"),
    ]
}

fn race(tries: &Tries) -> PlaytestReport {
    let entrants: [&dyn Entrant<(), DiceRace>; 2] = [&STEPPER, &ROLLER];
    run(&(), &entrants, tries, &race_checks()).unwrap()
}

const FIFTY: Tries = Tries {
    count: 50,
    seed: 0,
    turns: 100,
};

#[test]
fn a_run_plays_every_entrant_from_the_same_seeds() {
    let report = race(&FIFTY);
    assert_eq!(report.entrants.len(), 2);
    for e in &report.entrants {
        let seeds: Vec<u64> = e.tries.iter().map(|t| t.seed).collect();
        assert_eq!(seeds, (0..50).collect::<Vec<_>>(), "{}", e.name);
    }
    // Stepping first always wins in 10 turns; a roller wins most races
    // (it moves 3.5 a move), and faster.
    let stepper = &report.entrants[0].tries[0];
    assert_eq!(stepper.outcome.as_deref(), Some("0"));
    assert_eq!(stepper.measures["turns"], 10);
    assert_eq!(stepper.measures["moves"], 19);
    assert!(!report.blocked);
    for j in &report.checks {
        assert!(
            matches!(
                j.found,
                Found::Rate {
                    verdict: RateVerdict::Within,
                    ..
                } | Found::Median {
                    verdict: MedianVerdict::Pass,
                    ..
                }
            ),
            "{j:?}"
        );
    }
    assert_eq!(race(&FIFTY), report);
}

#[test]
fn a_try_stopped_by_the_limit_has_no_outcome() {
    let report = race(&Tries { turns: 3, ..FIFTY });
    let t = &report.entrants[0].tries[0];
    assert_eq!((t.outcome.as_deref(), t.turns), (None, 4));
    assert!(report.blocked);
}

#[test]
fn judging_seed_ranges_in_any_order_gives_the_report_of_a_run() {
    let ranges = [34..50, 17..34, 0..17];
    let mut parts = Vec::new();
    for range in ranges {
        for racer in [&ROLLER, &STEPPER] {
            let tries = play_tries(&(), racer, range.clone(), FIFTY.turns).unwrap();
            parts.push((racer.name.to_string(), tries));
        }
    }
    // The roller's parts came first, so its tries come first too.
    let mut report = judge(parts, &race_checks()).unwrap();
    report.entrants.reverse();
    assert_eq!(report, race(&FIFTY));
    let whole = vec![
        (
            "stepper".to_string(),
            play_tries(&(), &STEPPER, 0..50, 100).unwrap(),
        ),
        (
            "roller".to_string(),
            play_tries(&(), &ROLLER, 0..50, 100).unwrap(),
        ),
    ];
    assert_eq!(judge(whole, &race_checks()).unwrap(), race(&FIFTY));
}

/// An entrant that is never asked to play.
struct Unplayable;

impl Entrant<(), DiceRace> for Unplayable {
    fn name(&self) -> &'static str {
        "stepper"
    }

    fn start(&self, (): &(), _: u64) -> DiceRace {
        panic!("a try was played")
    }

    fn bots(&self, _: u64) -> Vec<(usize, Box<dyn Bot<DiceRace>>)> {
        panic!("a try was played")
    }

    fn measure(&self) -> Box<dyn Measure<DiceRace>> {
        panic!("a try was played")
    }
}

#[test]
fn a_check_naming_a_missing_entrant_fails_before_any_try() {
    let entrants: [&dyn Entrant<(), DiceRace>; 1] = [&Unplayable];
    let checks = [
        rate("stepper", 0, 100, None),
        median_check("stepper", Want::Lower, "nobody"),
    ];
    let Err(PlaytestError::UnknownEntrant(error)) = run(&(), &entrants, &FIFTY, &checks) else {
        panic!("it ran");
    };
    assert_eq!(
        error.to_string(),
        "check 2 names `nobody`, who isn't in the run"
    );
    assert_eq!(error.check, 1);
    let judged = judge(vec![("a".into(), tries(1, 1))], &[rate("b", 0, 1, None)]);
    assert_eq!(
        judged,
        Err(UnknownEntrant {
            check: 0,
            entrant: "b".to_string()
        })
    );
}

/// Nim, with a taker that takes too many.
struct Greedy;

struct Nothing;

impl Measure<Nim> for Nothing {
    fn observe(&mut self, _: &Nim, _: &u32, _: &[u32]) {}

    fn finish(self: Box<Self>, _: &Report<Nim>, _: &Nim) -> Measures {
        Measures::new()
    }
}

impl Entrant<u32, Nim> for Greedy {
    fn name(&self) -> &'static str {
        "greedy"
    }

    fn start(&self, pile: &u32, _: u64) -> Nim {
        Nim::new(*pile)
    }

    fn bots(&self, _: u64) -> Vec<(usize, Box<dyn Bot<Nim>>)> {
        vec![(0, Box::new(|_: &Nim| 5)), (1, Box::new(|_: &Nim| 1))]
    }

    fn measure(&self) -> Box<dyn Measure<Nim>> {
        Box::new(Nothing)
    }
}

#[test]
fn a_refused_command_stops_the_run() {
    let entrants: [&dyn Entrant<u32, Nim>; 1] = [&Greedy];
    let Err(error) = run(&9, &entrants, &FIFTY, &[]) else {
        panic!("it ran");
    };
    assert!(matches!(
        error,
        PlaytestError::Play { ref entrant, ref error } if entrant == "greedy" && error.command == 5
    ));
    assert!(alloc::format!("{error:?}").starts_with("Play { entrant: \"greedy\""));
}

#[test]
fn the_report_reads_as_a_table() {
    let mut casual = tries(3, 4);
    for (t, turns) in casual.iter_mut().zip([5, 6, 9, 9]) {
        t.measures.insert("turns".to_string(), turns);
    }
    casual[3].outcome = None;
    let mut hardcore = tries(1, 4);
    hardcore[0].measures.insert("turns".to_string(), 4);
    let report = judge(
        vec![
            ("casual".to_string(), casual),
            ("hardcore".to_string(), hardcore),
        ],
        &[
            rate("casual", 60, 85, None),
            rate("hardcore", 50, 75, Some(30)),
            median_check("hardcore", Want::Lower, "casual"),
        ],
    )
    .unwrap();
    assert_eq!(
        report.to_string(),
        "4 tries (seeds 0–3)\n\
         \n\
         casual   · 3/4 succeed                  band 60–85%   ✓\n\
         hardcore · 1/4 succeed                  band 50–75%   ✗ too hard · blocked: under the 30% floor\n\
         hardcore · turns median 4 (casual 6)    lower ✓\n\
         \n\
         try 0 · casual · 0 · turns 1 · turns 5 · won 1\n\
         try 1 · casual · 0 · turns 1 · turns 6 · won 1\n\
         try 2 · casual · 0 · turns 1 · turns 9 · won 1\n\
         try 3 · casual · unfinished · turns 1 · turns 9 · won 0\n\
         try 0 · hardcore · 0 · turns 1 · turns 4 · won 1\n\
         try 1 · hardcore · 0 · turns 1 · won 0\n\
         try 2 · hardcore · 0 · turns 1 · won 0\n\
         try 3 · hardcore · 0 · turns 1 · won 0\n"
    );
    let unknown = judge(
        vec![("a".to_string(), Vec::new())],
        &[
            rate("a", 10, 20, None),
            median_check("a", Want::Higher, "a"),
        ],
    )
    .unwrap();
    assert_eq!(
        unknown.to_string(),
        "0 tries\n\
         \n\
         a · 0/0 succeed             band 10–20%   ✓\n\
         a · turns median – (a –)    higher –\n"
    );
}

proptest! {
    #[test]
    fn a_band_gives_one_verdict_and_within_iff_inside(
        min in 0..=100_u32,
        max in 0..=100_u32,
        count in 1..500_u32,
        successes in 0..500_u32,
    ) {
        let successes = successes.min(count);
        let verdict = Band { min, max }.verdict(successes, count);
        // The share in hundredths of a percent, exactly.
        let share = u64::from(successes) * 100;
        let inside = share >= u64::from(min) * u64::from(count)
            && share <= u64::from(max) * u64::from(count);
        prop_assert_eq!(verdict == RateVerdict::Within, inside);
        let below = share < u64::from(min) * u64::from(count);
        prop_assert_eq!(verdict == RateVerdict::Below, below);
        prop_assert_eq!(verdict == RateVerdict::Above, !inside && !below);
    }
}
