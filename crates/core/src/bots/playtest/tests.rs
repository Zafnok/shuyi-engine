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
                other => panic!("{other:?}"),
            }
        })
        .collect();
    // Just out, the interval still reaches the band.
    assert_eq!(
        verdicts,
        [
            RateVerdict::MaybeBelow,
            RateVerdict::Within,
            RateVerdict::Within,
            RateVerdict::MaybeAbove
        ]
    );
}

#[test]
fn the_interval_of_44_in_100_is_35_to_53() {
    assert_eq!(Band::interval(44, 100), Some((35, 53)));
    // Wilson's interval is never empty, and stays inside 0–100 at the ends.
    assert_eq!(Band::interval(0, 100), Some((0, 3)));
    assert_eq!(Band::interval(100, 100), Some((97, 100)));
    assert_eq!(Band::interval(1, 1), Some((21, 100)));
    assert_eq!(Band::interval(0, 0), None);
    assert_eq!(Band::interval(5, 0), None);
    // More successes than tries count as all of them.
    assert_eq!(Band::interval(7, 4), Band::interval(4, 4));
}

#[test]
fn an_interval_narrower_than_a_percent_is_the_shares_own() {
    // 33.33 % of a million: 33.24–33.43, no whole percent inside.
    assert_eq!(Band::interval(333_333, 1_000_000), Some((33, 33)));
    // The most tries there can be, all of them: no overflow.
    assert_eq!(Band::interval(u32::MAX, u32::MAX), Some((100, 100)));
    assert_eq!(Band::interval(0, u32::MAX), Some((0, 0)));
}

#[test]
fn a_rate_is_sure_out_of_its_band_only_when_its_whole_interval_is() {
    let band = Band { min: 30, max: 55 };
    let at = |successes| band.verdict(successes, 100);
    // 20: 14–28. 29: 22–38. 40: in. 60: 51–69. 70: 61–78.
    assert_eq!(at(20), RateVerdict::Below);
    assert_eq!(at(29), RateVerdict::MaybeBelow);
    assert_eq!(at(40), RateVerdict::Within);
    assert_eq!(at(60), RateVerdict::MaybeAbove);
    assert_eq!(at(70), RateVerdict::Above);
    // The interval's own ends: 38 reaches a band from 38, not one from 39.
    assert_eq!(
        Band { min: 38, max: 55 }.verdict(29, 100),
        RateVerdict::MaybeBelow
    );
    assert_eq!(
        Band { min: 39, max: 55 }.verdict(29, 100),
        RateVerdict::Below
    );
    assert_eq!(
        Band { min: 0, max: 51 }.verdict(60, 100),
        RateVerdict::MaybeAbove
    );
    assert_eq!(
        Band { min: 0, max: 50 }.verdict(60, 100),
        RateVerdict::Above
    );
    // No try is in every band.
    assert_eq!(band.verdict(0, 0), RateVerdict::Within);
}

#[test]
fn the_floor_is_still_judged_on_the_share_itself() {
    // 29 of 100 is maybe too hard, and under a 30 % floor all the same.
    let report = judge(
        vec![("a".into(), tries(29, 100))],
        &[rate("a", 30, 55, Some(30))],
    )
    .unwrap();
    assert_eq!(
        found(&report, 0),
        &Found::Rate {
            successes: 29,
            count: 100,
            verdict: RateVerdict::MaybeBelow,
            blocked: true,
            interval: Some((22, 38)),
        }
    );
    assert!(report.blocked);
    // 20 is surely too hard and over a 20 % floor.
    let report = judge(
        vec![("a".into(), tries(20, 100))],
        &[rate("a", 30, 55, Some(20))],
    )
    .unwrap();
    assert!(matches!(
        found(&report, 0),
        Found::Rate {
            verdict: RateVerdict::Below,
            blocked: false,
            ..
        }
    ));
    assert!(!report.blocked);
}

#[test]
fn a_rate_kept_before_the_interval_still_reads() {
    let old = "Rate(successes: 1, count: 2, verdict: Within, blocked: false)";
    let found: Found = ron::from_str(old).unwrap();
    assert!(
        matches!(found, Found::Rate { interval: None, .. }),
        "{found:?}"
    );
    let judged = r#"(check: Rate(entrant: "a", success: [], band: (min: 0, max: 100)),
                     found: Rate(successes: 1, count: 2, verdict: Within, blocked: false))"#;
    assert!(ron::from_str::<Judged>(judged).is_ok());
}

fn gap(entrant: &str, than: &str, most: u32) -> Check {
    Check::gap(entrant, than, won(), most)
}

/// The verdict of a gap of `most` points between `best` and `starter`,
/// each `(successes, count)`.
fn gap_of(best: (u32, u32), starter: (u32, u32), most: u32) -> GapVerdict {
    let tries = vec![
        ("best".to_string(), tries(best.0, best.1)),
        ("starter".to_string(), tries(starter.0, starter.1)),
    ];
    let report = judge(tries, &[gap("best", "starter", most)]).unwrap();
    assert!(!report.blocked, "a gap never blocks");
    match found(&report, 0) {
        Found::Gap {
            successes,
            count,
            than_successes,
            than_count,
            verdict,
        } => {
            assert_eq!((*successes, *count), best);
            assert_eq!((*than_successes, *than_count), starter);
            *verdict
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_gap_of_10_points_holds_at_60_against_50_and_not_at_61() {
    assert_eq!(gap_of((60, 100), (50, 100), 10), GapVerdict::Within);
    assert_eq!(gap_of((61, 100), (50, 100), 10), GapVerdict::Over);
    // Under the other entrant's share is always within.
    assert_eq!(gap_of((10, 100), (90, 100), 0), GapVerdict::Within);
    assert_eq!(gap_of((50, 100), (50, 100), 0), GapVerdict::Within);
    assert_eq!(gap_of((51, 100), (50, 100), 0), GapVerdict::Over);
}

#[test]
fn a_gap_compares_in_integers_across_different_try_counts() {
    // 6 of 10 is 60 %, 121 of 200 is 60.5 %: against 50 of 100.
    assert_eq!(gap_of((6, 10), (50, 100), 10), GapVerdict::Within);
    assert_eq!(gap_of((120, 200), (5, 10), 10), GapVerdict::Within);
    assert_eq!(gap_of((121, 200), (5, 10), 10), GapVerdict::Over);
    // 1 of 3 is 33.3 %, over 1 of 4 by 8.3 points.
    assert_eq!(gap_of((1, 3), (1, 4), 9), GapVerdict::Within);
    assert_eq!(gap_of((1, 3), (1, 4), 8), GapVerdict::Over);
    // With no try on a side there is nothing to be over.
    assert_eq!(gap_of((0, 0), (0, 100), 0), GapVerdict::Within);
    assert_eq!(gap_of((5, 5), (0, 0), 0), GapVerdict::Within);
    // The most tries there can be: no overflow.
    assert_eq!(
        gap_verdict((u32::MAX, u32::MAX), (0, u32::MAX), 100),
        GapVerdict::Within
    );
    assert_eq!(
        gap_verdict((u32::MAX, u32::MAX), (0, u32::MAX), 99),
        GapVerdict::Over
    );
}

#[test]
fn a_gap_names_both_its_entrants() {
    let check = gap("best", "starter", 15);
    assert_eq!(check.entrants(), ["best", "starter"]);
    let judged = judge(vec![("best".into(), tries(1, 2))], &[check]);
    assert_eq!(judged, Err(UnknownEntrant::new(0, "starter")));
}

#[test]
fn a_bands_distance_is_0_inside_and_grows_past_each_edge() {
    let band = Band { min: 60, max: 85 };
    let at = |successes| band.distance(successes, 100);
    assert_eq!([at(58), at(59), at(60), at(61)], [200, 100, 0, 0]);
    assert_eq!([at(84), at(85), at(86), at(87)], [0, 0, 100, 200]);
    // Out of 3 tries, 2 is 66 %: 200 against 180 and 255.
    assert_eq!(band.distance(2, 3), 0);
    assert_eq!(band.distance(1, 3), 80);
    assert_eq!(band.distance(3, 3), 45);
    assert_eq!(band.distance(0, 0), 0);
}

#[test]
fn an_unknown_entrant_is_an_error() {
    let error: &dyn core::error::Error = &UnknownEntrant::new(0, "b");
    assert_eq!(error.to_string(), "check 1 names `b`, who isn't in the run");
}

#[test]
fn a_bots_seed_differs_from_the_try_seed_and_is_the_same_for_the_same_seed() {
    for seed in [0, 1, 42, u64::MAX] {
        assert_ne!(bot_seed(seed), seed);
        assert_eq!(bot_seed(seed), bot_seed(seed));
    }
    assert_ne!(bot_seed(0), bot_seed(1));
}

#[test]
fn a_bots_seed_is_two_draws_of_the_try_seeds_rng_the_high_half_first() {
    for seed in [0, 7, u64::MAX] {
        let mut rng = crate::rng::SimRng::new(seed);
        let (high, low) = (rng.next_u32(), rng.next_u32());
        assert_eq!(bot_seed(seed) >> 32, u64::from(high), "seed {seed}");
        assert_eq!(
            bot_seed(seed) & u64::from(u32::MAX),
            u64::from(low),
            "seed {seed}"
        );
    }
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
            _ => panic!("{j:?}"),
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

/// [`STEPPER`], keeping a record of each try: its seed and its commands.
struct Recorder;

impl Entrant<(), DiceRace> for Recorder {
    fn name(&self) -> &str {
        STEPPER.name()
    }

    fn start(&self, stage: &(), seed: u64) -> DiceRace {
        STEPPER.start(stage, seed)
    }

    fn bots(&self, seed: u64) -> Vec<(usize, Box<dyn Bot<DiceRace>>)> {
        STEPPER.bots(seed)
    }

    fn measure(&self) -> Box<dyn Measure<DiceRace>> {
        STEPPER.measure()
    }

    fn record(&self, seed: u64, commands: &[Race]) -> Option<String> {
        Some(format!("{seed}: {}", commands.len()))
    }
}

#[test]
fn a_try_keeps_its_commands_and_the_entrants_record() {
    let played = play_try(&(), &Recorder, 7, 100).unwrap();
    let tries = play_tries(&(), &STEPPER, 7..8, 100).unwrap();
    assert_eq!(played.tried, tries[0]);
    // Both sides step until the first is home on turn 10: 19 moves.
    assert_eq!(played.commands, 19);
    assert_eq!(played.record.as_deref(), Some("7: 19"));
    // An entrant that keeps no record.
    let plain = play_try(&(), &STEPPER, 7, 100).unwrap();
    assert_eq!(plain, PlayedTry::new(tries[0].clone(), 19));
    let stopped = play_try(&(), &STEPPER, 7, 3).unwrap();
    assert_eq!((stopped.tried.outcome, stopped.commands), (None, 6));
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
            gap("casual", "hardcore", 20),
        ],
    )
    .unwrap();
    // 3 of 4 is 31–95 %, 1 of 4 is 5–69 %, so only maybe too hard; and
    // casual succeeds 50 points more often, over the 20 the gap allows.
    assert_eq!(
        report.to_string(),
        "4 tries (seeds 0–3)\n\
         \n\
         casual   · 3/4 succeed                   band 60–85% · 95% 31–95%   ✓\n\
         hardcore · 1/4 succeed                   band 50–75% · 95% 5–69%   ~ maybe too hard · blocked: under the 30% floor\n\
         hardcore · turns median 4 (casual 6)     lower ✓\n\
         casual   · 3/4 succeed (hardcore 1/4)    at most 20 points over ✗\n\
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
    // Brief, the table stops before the tries.
    let table = report.to_string();
    let brief = report.brief();
    assert!(brief.ends_with("over ✗\n"), "{brief}");
    assert_eq!(table[..brief.len()], brief);
    assert!(!brief.contains("try "));
    let unknown = judge(
        vec![("a".to_string(), Vec::new())],
        &[
            rate("a", 10, 20, None),
            median_check("a", Want::Higher, "a"),
            gap("a", "a", 0),
        ],
    )
    .unwrap();
    // No try: no interval to show.
    assert_eq!(
        unknown.to_string(),
        "0 tries\n\
         \n\
         a · 0/0 succeed             band 10–20%   ✓\n\
         a · turns median – (a –)    higher –\n\
         a · 0/0 succeed (a 0/0)     at most 0 points over ✓\n"
    );
}

/// A game that refuses every command, saying so in words.
struct Locked;

/// Why [`Locked`] refuses.
#[derive(Debug)]
struct Shut;

impl fmt::Display for Shut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the door is shut")
    }
}

impl TurnRules for Locked {
    type Command = u8;
    type Event = ();
    type Error = Shut;
    type Outcome = ();

    fn check(&self, _: &u8) -> Result<(), Shut> {
        Err(Shut)
    }

    fn apply(&mut self, _: &u8) -> Result<Vec<()>, Shut> {
        Err(Shut)
    }

    fn legal_commands(&self) -> Vec<u8> {
        Vec::new()
    }

    fn outcome(&self) -> Option<()> {
        None
    }
}

impl Sides for Locked {
    type Side = u8;

    fn side_to_move(&self) -> u8 {
        0
    }

    fn turn(&self) -> u32 {
        1
    }
}

#[test]
fn a_playtest_error_says_what_went_wrong() {
    let unknown: PlaytestError<Locked> = UnknownEntrant {
        check: 2,
        entrant: "nobody".to_string(),
    }
    .into();
    assert_eq!(
        unknown.to_string(),
        "check 3 names `nobody`, who isn't in the run"
    );
    let refused: PlaytestError<Locked> = PlaytestError::Play {
        entrant: "knocker".to_string(),
        error: PlayError {
            command: 7,
            side: 0,
            turn: 1,
            error: Shut,
        },
    };
    assert_eq!(
        refused.to_string(),
        "knocker: turn 1, 0 to move: 7 refused: the door is shut"
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
        let (low, high) = Band::interval(successes, count).unwrap();
        prop_assert_eq!(verdict == RateVerdict::Below, below && high < min);
        prop_assert_eq!(verdict == RateVerdict::MaybeBelow, below && high >= min);
        let above = !inside && !below;
        prop_assert_eq!(verdict == RateVerdict::Above, above && low > max);
        prop_assert_eq!(verdict == RateVerdict::MaybeAbove, above && low <= max);
    }

    #[test]
    fn the_interval_holds_the_shares_own_percent(
        count in prop_oneof![1..1_000_u32, 1..=u32::MAX],
        successes in any::<u32>(),
    ) {
        // Any share, from none to every try.
        let successes = u32::try_from(u64::from(successes) % (u64::from(count) + 1)).unwrap();
        let (low, high) = Band::interval(successes, count).unwrap();
        prop_assert!(low <= high && high <= 100, "{low}–{high}");
        // The share, rounded down and up, against the interval's ends.
        let share = u64::from(successes) * 100;
        let count = u64::from(count);
        let down = share / count;
        let up = share.div_ceil(count);
        prop_assert!(u64::from(low) <= up && down <= u64::from(high),
            "{successes} of {count}: {low}–{high}");
        // A whole percent is inside it.
        if share % count == 0 {
            prop_assert!((u64::from(low)..=u64::from(high)).contains(&down));
        }
    }

    #[test]
    fn a_gap_holds_exactly_when_the_shares_differ_by_at_most_its_most(
        count in 1..300_u32,
        than_count in 1..300_u32,
        successes in 0..300_u32,
        than_successes in 0..300_u32,
        most in 0..=100_u32,
    ) {
        let (s, ts) = (successes.min(count), than_successes.min(than_count));
        let verdict = gap_verdict((s, count), (ts, than_count), most);
        // The two shares over a common count, in hundredths of a percent.
        let mine = i64::from(s) * 100 * i64::from(than_count);
        let theirs = i64::from(ts) * 100 * i64::from(count);
        let allowed = i64::from(most) * i64::from(count) * i64::from(than_count);
        prop_assert_eq!(verdict == GapVerdict::Within, mine - theirs <= allowed);
        // A larger most never turns a gap over.
        if verdict == GapVerdict::Within {
            prop_assert_eq!(gap_verdict((s, count), (ts, than_count), most + 1), GapVerdict::Within);
        }
    }
}

/// A tier of the targets' fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
enum Tier {
    Easy,
    Hard,
}

/// Targets for three entrants, `base` a baseline with none of its own.
const TARGETS: &str = r#"(
    success: {
        "a": [(measure: "won", min: Some(1))],
        "b": [(measure: "won", min: Some(1)), (measure: "deaths", max: Some(0))],
    },
    bands: {
        Easy: {"a": (min: 70, max: 90), "b": (min: 50, max: 80)},
    },
    floors: {"a": 20},
    medians: [(entrant: "b", measure: "turns", among: [(measure: "won", min: Some(1))], want: Lower, than: "base")],
)"#;

fn targets() -> Targets<Tier> {
    ron::from_str(TARGETS).unwrap()
}

#[test]
fn targets_give_a_rate_per_entrant_with_a_band_then_the_medians() {
    let checks = targets().checks(&Tier::Easy, &["a", "b", "base"]);
    let b_success = vec![
        won()[0].clone(),
        Condition {
            measure: "deaths".to_string(),
            min: None,
            max: Some(0),
        },
    ];
    assert_eq!(
        checks,
        vec![
            rate("a", 70, 90, Some(20)),
            Check::Rate {
                entrant: "b".to_string(),
                success: b_success,
                band: Band { min: 50, max: 80 },
                floor: None,
            },
            median_check("b", Want::Lower, "base"),
        ]
    );
}

#[test]
fn a_median_whose_other_entrant_doesnt_play_is_left_out() {
    let checks = targets().checks(&Tier::Easy, &["a", "b"]);
    assert_eq!(checks.len(), 2);
    assert!(checks.iter().all(|c| matches!(c, Check::Rate { .. })));
}

#[test]
fn a_tier_without_bands_gives_only_the_medians() {
    let checks = targets().checks(&Tier::Hard, &["a", "b", "base"]);
    assert_eq!(checks, vec![median_check("b", Want::Lower, "base")]);
}

#[test]
fn unknown_entrants_are_the_names_the_game_has_none_for() {
    let text = r#"(
        success: {"a": []},
        bands: {Easy: {"a": (min: 0, max: 100), "c": (min: 0, max: 100)}},
        floors: {"b": 10},
        medians: [(entrant: "d", measure: "turns", want: Higher, than: "a"),
                  (entrant: "c", measure: "turns", want: Lower, than: "d")],
    )"#;
    let targets: Targets<Tier> = ron::from_str(text).unwrap();
    assert_eq!(targets.unknown_entrants(&["a", "b"]), vec!["c", "d"]);
    let got = targets.unknown_entrants(&["a", "b", "c", "d"]);
    assert!(got.is_empty(), "{got:?}");
}

#[test]
fn an_unknown_field_in_targets_is_an_error() {
    let text = "(success: {}, faster: ())";
    assert!(ron::from_str::<Targets<Tier>>(text).is_err());
    let text =
        r#"(success: {}, medians: [(entrant: "a", measure: "t", want: Lower, than: "b", by: 1)])"#;
    assert!(ron::from_str::<Targets<Tier>>(text).is_err());
}

#[test]
fn targets_with_only_success_rules_give_no_checks() {
    let targets: Targets<Tier> = ron::from_str(r#"(success: {"a": []})"#).unwrap();
    let got = targets.checks(&Tier::Easy, &["a"]);
    assert!(got.is_empty(), "{got:?}");
}

/// [`TARGETS`] with a hold that leaves the medians out and a trial.
fn kind_targets() -> Targets<Tier> {
    let mut with_kinds = targets();
    let text = r#"{
        "hold": (bands: {Easy: {"a": (min: 40, max: 60)}, Hard: {"b": (min: 5, max: 15)}},
                 medians: false),
        "trial": (bands: {Easy: {"a": (min: 10, max: 20), "b": (min: 30, max: 40)}}),
    }"#;
    with_kinds.kinds = ron::from_str(text).unwrap();
    with_kinds
}

/// The bands of `checks`' rates, by entrant.
fn bands_of(checks: &[Check]) -> Vec<(&str, u32, u32)> {
    checks
        .iter()
        .filter_map(|check| match check {
            Check::Rate { entrant, band, .. } => Some((entrant.as_str(), band.min, band.max)),
            _ => None,
        })
        .collect()
}

/// [`kind_targets`] with two gaps, one of them to an entrant who
/// doesn't always play.
fn gap_targets() -> Targets<Tier> {
    let mut with_gaps = kind_targets();
    let text = r#"[
        (entrant: "a", than: "base", success: [(measure: "won", min: Some(1))], most: 15),
        (entrant: "b", than: "a", success: [(measure: "won", min: Some(1))], most: 5),
    ]"#;
    with_gaps.gaps = ron::from_str(text).unwrap();
    with_gaps
}

#[test]
fn targets_give_the_gaps_after_the_medians_wherever_both_entrants_play() {
    let targets = gap_targets();
    let all = ["a", "b", "base"];
    let checks = targets.checks(&Tier::Easy, &all);
    assert_eq!(
        checks[2..],
        [
            median_check("b", Want::Lower, "base"),
            gap("a", "base", 15),
            gap("b", "a", 5),
        ]
    );
    // Without `base`, its gap is left out, and its median.
    let checks = targets.checks(&Tier::Easy, &["a", "b"]);
    assert_eq!(checks.last(), Some(&gap("b", "a", 5)));
    assert_eq!(checks.len(), 3);
    // A tier with no bands still checks the gaps.
    let hard = targets.checks(&Tier::Hard, &all);
    assert_eq!(hard[1..], [gap("a", "base", 15), gap("b", "a", 5)]);
}

#[test]
fn a_kind_that_leaves_the_medians_out_keeps_the_gaps() {
    let targets = gap_targets();
    let all = ["a", "b", "base"];
    let hold = targets.checks_of(&Tier::Easy, &["hold"], &all);
    assert!(!hold.iter().any(|c| matches!(c, Check::Median { .. })));
    assert_eq!(hold[2..], [gap("a", "base", 15), gap("b", "a", 5)]);
}

#[test]
fn a_gap_left_out_of_the_file_is_none_and_an_unknown_field_is_an_error() {
    assert_eq!(targets().gaps, Vec::<GapTarget>::new());
    let text = r#"(success: {}, gaps: [(entrant: "a", than: "b", success: [], most: 1, by: 2)])"#;
    assert!(ron::from_str::<Targets<Tier>>(text).is_err());
    // A gap without its most is an error, not a gap of 0.
    let text = r#"(success: {}, gaps: [(entrant: "a", than: "b", success: [])])"#;
    assert!(ron::from_str::<Targets<Tier>>(text).is_err());
}

#[test]
fn unknown_entrants_include_a_gaps_two() {
    let text = r#"(
        success: {"best": [], "starter": []},
        gaps: [(entrant: "bets", than: "starter", success: [], most: 15),
               (entrant: "best", than: "startr", success: [], most: 15)],
    )"#;
    let targets: Targets<Tier> = ron::from_str(text).unwrap();
    assert_eq!(
        targets.unknown_entrants(&["best", "starter"]),
        vec!["bets", "startr"]
    );
}

#[test]
fn a_gap_and_targets_built_by_their_constructors_are_the_literals() {
    assert_eq!(
        GapTarget::new("best", "starter", won(), 15),
        GapTarget {
            entrant: "best".into(),
            than: "starter".into(),
            success: won(),
            most: 15,
        }
    );
    let success = BTreeMap::from([("a".to_string(), won())]);
    assert_eq!(
        Targets::<Tier>::new(success.clone()),
        Targets {
            success,
            bands: BTreeMap::new(),
            floors: BTreeMap::new(),
            medians: Vec::new(),
            kinds: BTreeMap::new(),
            gaps: Vec::new(),
        }
    );
    let check = Check::gap("best", "starter", won(), 15);
    assert_eq!(
        check,
        Check::Gap {
            entrant: "best".into(),
            than: "starter".into(),
            success: won(),
            most: 15,
        }
    );
    assert_eq!(check.clone().with_floor(Some(20)), check);
    assert_eq!(check.clone().with_among(Vec::new()), check);
}

#[test]
fn a_kinds_bands_lie_over_the_tiers_entrant_by_entrant_and_a_later_kind_wins() {
    let targets = kind_targets();
    let all = ["a", "b", "base"];
    let hold = targets.checks_of(&Tier::Easy, &["hold"], &all);
    assert_eq!(bands_of(&hold), [("a", 40, 60), ("b", 50, 80)]);
    // The floor is the entrant's, whatever the band.
    assert!(matches!(
        &hold[0],
        Check::Rate {
            floor: Some(20),
            ..
        }
    ));
    let both = targets.checks_of(&Tier::Easy, &["hold", "trial"], &all);
    assert_eq!(bands_of(&both), [("a", 10, 20), ("b", 30, 40)]);
    let both = targets.checks_of(&Tier::Easy, &["trial", "hold"], &all);
    assert_eq!(bands_of(&both), [("a", 40, 60), ("b", 30, 40)]);
    // A tier with no bands of its own takes a kind's.
    let hard = targets.checks_of(&Tier::Hard, &["hold"], &all);
    assert_eq!(bands_of(&hard), [("b", 5, 15)]);
}

#[test]
fn a_kind_may_leave_the_medians_out_and_keeps_them_by_default() {
    let targets = kind_targets();
    let all = ["a", "b", "base"];
    let median = median_check("b", Want::Lower, "base");
    let trial = targets.checks_of(&Tier::Easy, &["trial"], &all);
    assert_eq!(trial.last(), Some(&median));
    for kinds in [&["hold"][..], &["trial", "hold"], &["hold", "trial"]] {
        let checks = targets.checks_of(&Tier::Easy, kinds, &all);
        assert!(!checks.contains(&median), "{kinds:?}");
    }
    assert!(targets.kinds["trial"].medians);
}

#[test]
fn a_kind_the_targets_dont_name_changes_nothing() {
    let targets = kind_targets();
    let all = ["a", "b", "base"];
    for tier in [Tier::Easy, Tier::Hard] {
        let plain = targets.checks(&tier, &all);
        assert_eq!(targets.checks_of(&tier, &["race"], &all), plain);
        assert_eq!(targets.checks_of(&tier, &[], &all), plain);
    }
    // And today's checks are what they were.
    assert_eq!(
        kind_targets().checks(&Tier::Easy, &all),
        self::targets().checks(&Tier::Easy, &all)
    );
}

#[test]
fn unknown_entrants_include_a_kinds_bands() {
    let text = r#"(
        success: {"a": []},
        kinds: {"hold": (bands: {Easy: {"a": (min: 0, max: 100), "aa": (min: 0, max: 100)}})},
    )"#;
    let targets: Targets<Tier> = ron::from_str(text).unwrap();
    assert_eq!(targets.unknown_entrants(&["a"]), vec!["aa"]);
}

#[test]
fn a_kind_built_by_its_constructor_is_the_literal() {
    let bands = BTreeMap::from([(Tier::Easy, BTreeMap::new())]);
    assert_eq!(
        KindTargets::new(bands.clone(), false),
        KindTargets {
            bands,
            medians: false,
        }
    );
    let text = "(bands: {Easy: {}}, medians: true, by: 1)";
    assert!(ron::from_str::<KindTargets<Tier>>(text).is_err());
}

#[cfg(feature = "schema")]
#[test]
fn targets_over_string_tiers_have_a_schema() {
    let schema = schemars::schema_for!(Targets<String>);
    let json = schema.as_value().to_string();
    assert!(json.contains("bands") && json.contains("MedianTarget"));
}

/// Any targets over tiers 0 to 2 and names from a small pool.
fn any_targets() -> impl Strategy<Value = Targets<u8>> {
    let name = prop::sample::select(vec!["a", "b", "c", "d", "e"]).prop_map(str::to_string);
    let band = (0..=100_u32, 0..=100_u32).prop_map(|(min, max)| Band { min, max });
    let median = (name.clone(), name.clone()).prop_map(|(entrant, than)| MedianTarget {
        entrant,
        measure: "turns".to_string(),
        among: Vec::new(),
        want: Want::Lower,
        than,
    });
    let bands = prop::collection::btree_map(
        0..3_u8,
        prop::collection::btree_map(name.clone(), band, 0..5),
        0..3,
    );
    let kind = (bands.clone(), any::<bool>())
        .prop_map(|(bands, medians)| KindTargets::new(bands, medians));
    let kind_name = prop::sample::select(vec!["hold", "trial"]).prop_map(str::to_string);
    let gap = (name.clone(), name.clone(), 0..=100_u32)
        .prop_map(|(entrant, than, most)| GapTarget::new(entrant, than, won(), most));
    (
        prop::collection::btree_map(name.clone(), Just(won()), 0..5),
        bands,
        prop::collection::btree_map(name.clone(), 0..=100_u32, 0..5),
        prop::collection::vec(median, 0..5),
        prop::collection::btree_map(kind_name, kind, 0..3),
        prop::collection::vec(gap, 0..5),
    )
        .prop_map(|(success, bands, floors, medians, kinds, gaps)| Targets {
            success,
            bands,
            floors,
            medians,
            kinds,
            gaps,
        })
}

proptest! {
    #[test]
    fn targets_checks_name_only_the_entrants_that_play(
        targets in any_targets(),
        tier in 0..3_u8,
        entrants in prop::sample::subsequence(vec!["a", "b", "c", "d", "e"], 0..=5),
        kinds in prop::sample::subsequence(vec!["hold", "trial", "race"], 0..=3),
    ) {
        prop_assert_eq!(targets.checks_of(&tier, &[], &entrants), targets.checks(&tier, &entrants));
        let checks = targets.checks_of(&tier, &kinds, &entrants);
        for check in &checks {
            for name in check.entrants() {
                prop_assert!(entrants.contains(&name));
            }
        }
        // So judging them never meets an unknown entrant.
        let tries = entrants.iter().map(|n| ((*n).to_string(), tries(1, 2))).collect();
        prop_assert!(judge(tries, &checks).is_ok());
    }
}

/// Tries as games build theirs, in a `const` item.
const TEN: Tries = Tries::new(10).with_seed(4).with_turns(30);

#[test]
fn the_constructors_give_the_literals() {
    assert_eq!(
        TEN,
        Tries {
            count: 10,
            seed: 4,
            turns: 30,
        }
    );
    // Left alone, from seed 0, stopped at turn 200.
    let plain = Tries::new(10);
    assert_eq!((plain.seed, plain.turns), (0, 200));
    let won = Condition::new("won").with_min(Some(1)).with_max(Some(3));
    assert_eq!(
        won,
        Condition {
            measure: "won".into(),
            min: Some(1),
            max: Some(3),
        }
    );
    assert_eq!(
        Condition::new("won"),
        Condition {
            measure: "won".into(),
            min: None,
            max: None,
        }
    );
    let band = Band { min: 50, max: 70 };
    let rate = Check::rate("walk", vec![won.clone()], band);
    assert_eq!(
        rate,
        Check::Rate {
            entrant: "walk".into(),
            success: vec![won.clone()],
            band,
            floor: None,
        }
    );
    assert_eq!(
        rate.clone().with_floor(Some(20)),
        Check::Rate {
            entrant: "walk".into(),
            success: vec![won.clone()],
            band,
            floor: Some(20),
        }
    );
    assert_eq!(rate.clone().with_among(vec![won.clone()]), rate);
    let median = Check::median("run", "turns", Want::Lower, "walk");
    assert_eq!(
        median,
        Check::Median {
            entrant: "run".into(),
            measure: "turns".into(),
            among: Vec::new(),
            want: Want::Lower,
            than: "walk".into(),
        }
    );
    assert_eq!(
        median.clone().with_among(vec![won.clone()]),
        Check::Median {
            entrant: "run".into(),
            measure: "turns".into(),
            among: vec![won],
            want: Want::Lower,
            than: "walk".into(),
        }
    );
    assert_eq!(median.clone().with_floor(Some(20)), median);
    assert_eq!(
        UnknownEntrant::new(2, "nobody"),
        UnknownEntrant {
            check: 2,
            entrant: "nobody".into(),
        }
    );
}

#[test]
fn a_refusal_built_by_its_constructors_is_the_literal() {
    let error = PlayError::<Nim>::new(4, 1, 7, ());
    let PlayError {
        command,
        side,
        turn,
        error: (),
    } = error;
    assert_eq!((command, side, turn), (4, 1, 7));
    let play = PlaytestError::<Nim>::play("walk", PlayError::new(4, 1, 7, ()));
    assert!(matches!(
        play,
        PlaytestError::Play { ref entrant, error: PlayError { command: 4, side: 1, turn: 7, .. } }
            if entrant == "walk"
    ));
}
