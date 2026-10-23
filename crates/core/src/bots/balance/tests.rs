//! Tests of the autobalance search, on a coin race whose right setting is
//! worked out by hand.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::ToString;
use alloc::vec;
use core::cell::RefCell;

use proptest::prelude::*;

use super::*;
use crate::bots::playtest::{
    Band, Condition, Judged, Measure, Measures, MedianVerdict, RateVerdict, Tried, UnknownEntrant,
    Want, judge,
};
use crate::bots::{Bot, Report};
use crate::turn::TurnRules;

/// A one-side coin race. The stage is a target; the try of seed `s` has
/// `s % 10` coins, and its one command counts them: won when they reach
/// the target. So over seeds in tens, target `t` (0 to 10) wins
/// `(10 - t) * 10`% of tries.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CoinRace {
    target: i32,
    coins: i32,
    counted: bool,
}

impl TurnRules for CoinRace {
    type Command = ();
    type Event = ();
    type Error = ();
    /// Whether the coins reached the target.
    type Outcome = bool;

    fn check(&self, (): &()) -> Result<(), ()> {
        if self.counted { Err(()) } else { Ok(()) }
    }

    fn apply(&mut self, command: &()) -> Result<Vec<()>, ()> {
        self.check(command)?;
        self.counted = true;
        Ok(Vec::new())
    }

    fn legal_commands(&self) -> Vec<()> {
        if self.counted { Vec::new() } else { vec![()] }
    }

    fn outcome(&self) -> Option<bool> {
        self.counted.then_some(self.coins >= self.target)
    }
}

impl Sides for CoinRace {
    type Side = ();

    fn side_to_move(&self) {}

    fn turn(&self) -> u32 {
        1 + u32::from(self.counted)
    }
}

/// Measures `won`.
struct Won;

impl Measure<CoinRace> for Won {
    fn observe(&mut self, _: &CoinRace, (): &(), _: &[()]) {}

    fn finish(self: Box<Self>, report: &Report<CoinRace>, _: &CoinRace) -> Measures {
        [("won".to_string(), i64::from(report.outcome == Some(true)))].into()
    }
}

/// The coin race's one entrant, `counter`.
struct Counter;

impl Entrant<i32, CoinRace> for Counter {
    fn name(&self) -> &'static str {
        "counter"
    }

    fn start(&self, target: &i32, seed: u64) -> CoinRace {
        CoinRace {
            target: *target,
            coins: i32::try_from(seed % 10).unwrap_or(0),
            counted: false,
        }
    }

    fn bots(&self, _: u64) -> Vec<((), Box<dyn Bot<CoinRace>>)> {
        vec![((), Box::new(|_: &CoinRace| ()))]
    }

    fn measure(&self) -> Box<dyn Measure<CoinRace>> {
        Box::new(Won)
    }
}

/// A knob of the target: a step adds to it. It writes down every step it
/// is applied with, and can't apply at or under `none_from`.
struct Target {
    range: RangeInclusive<i32>,
    stride: u32,
    none_from: Option<i32>,
    applied: RefCell<Vec<i32>>,
}

impl Target {
    fn new(range: RangeInclusive<i32>, stride: u32) -> Self {
        Self {
            range,
            stride,
            none_from: None,
            applied: RefCell::new(Vec::new()),
        }
    }

    fn applied(&self) -> Vec<i32> {
        self.applied.borrow().clone()
    }
}

impl Knob<i32> for Target {
    fn name(&self) -> &'static str {
        "target"
    }

    fn range(&self) -> RangeInclusive<i32> {
        self.range.clone()
    }

    fn stride(&self) -> u32 {
        self.stride
    }

    fn apply(&self, target: &i32, step: i32) -> Option<i32> {
        self.applied.borrow_mut().push(step);
        if self.none_from.is_some_and(|from| step <= from) {
            return None;
        }
        Some(target + step)
    }

    fn describe(&self, step: i32) -> String {
        let way = if step < 0 { "lower" } else { "higher" };
        format!("the target is {} {way}", step.unsigned_abs())
    }
}

/// A knob that changes nothing, writing down its steps.
struct Idle {
    applied: RefCell<Vec<i32>>,
}

impl Knob<i32> for Idle {
    fn name(&self) -> &'static str {
        "idle"
    }

    fn range(&self) -> RangeInclusive<i32> {
        -3..=3
    }

    fn apply(&self, target: &i32, step: i32) -> Option<i32> {
        self.applied.borrow_mut().push(step);
        Some(*target)
    }

    fn describe(&self, _: i32) -> String {
        "nothing".to_string()
    }
}

fn won() -> Vec<Condition> {
    vec![Condition {
        measure: "won".to_string(),
        min: Some(1),
        max: None,
    }]
}

fn band(min: u32, max: u32) -> Vec<Check> {
    vec![Check::Rate {
        entrant: "counter".to_string(),
        success: won(),
        band: Band { min, max },
        floor: None,
    }]
}

/// 100 tries, quick runs of 10, room for 100 evaluations.
const SETTINGS: BalanceSettings = BalanceSettings {
    tries: Tries {
        count: 100,
        seed: 0,
        turns: 5,
    },
    quick: 10,
    evaluations: 100,
};

fn search(
    target: i32,
    knobs: &[&dyn Knob<i32>],
    checks: &[Check],
    settings: &BalanceSettings,
) -> Balanced<i32> {
    autobalance(&target, knobs, &[&Counter], checks, settings).unwrap()
}

/// The rate check's successes of `report`.
fn successes(report: &PlaytestReport) -> u32 {
    match report.checks[0].found {
        Found::Rate { successes, .. } => successes,
        Found::Median { .. } => panic!("{report:?}"),
    }
}

#[test]
fn a_target_set_too_high_comes_back_to_the_setting_worked_out_by_hand() {
    // Target 9 wins 10%; the band 60–70% wants target 3 or 4. From 0 at
    // stride 2 the quick runs step -2 (30%), -4 (50%) and -6 (target 3,
    // 70%), in the band; the full run confirms it, and the prune finds
    // -5 (target 4, 60%) still in it and -4 not.
    let knob = Target::new(-9..=0, 2);
    let found = search(9, &[&knob], &band(60, 70), &SETTINGS);
    assert_eq!(found.stop, Stop::InBands);
    assert_eq!(found.stage, 4);
    assert_eq!(
        found.changes,
        [Change {
            knob: "target".to_string(),
            step: -5,
            what: "the target is 5 lower".to_string(),
        }]
    );
    assert_eq!(
        (successes(&found.before), successes(&found.after)),
        (10, 60)
    );
    // Before, quick runs of 0, -2, -4 and -6, -6 in full, then -5 and -4
    // in full: 0 and -6 were not played twice.
    assert_eq!(found.evaluations, 8);
    let table = found.to_string();
    assert!(
        table.starts_with(
            "in every band · 8 evaluations\ntarget -5: the target is 5 lower\n\nbefore: 100 tries"
        ),
        "{table}"
    );
    assert!(table.contains("after: 100 tries"), "{table}");
    assert!(table.contains("counter · 60/100 succeed"), "{table}");
}

#[test]
fn a_stage_already_in_its_bands_stops_after_one_evaluation() {
    let knob = Target::new(-9..=0, 2);
    let found = search(4, &[&knob], &band(60, 70), &SETTINGS);
    assert_eq!(found.stop, Stop::AlreadyInBands);
    assert_eq!(found.evaluations, 1);
    assert_eq!((found.stage, found.changes.len()), (4, 0));
    assert_eq!(found.after, found.before);
    assert!(knob.applied().is_empty());
    assert!(
        found
            .to_string()
            .starts_with("already in every band: nothing to change · 1 evaluations\nno change\n")
    );
}

#[test]
fn the_same_inputs_give_the_same_result_twice() {
    let first = search(9, &[&Target::new(-9..=0, 2)], &band(60, 70), &SETTINGS);
    let second = search(9, &[&Target::new(-9..=0, 2)], &band(60, 70), &SETTINGS);
    assert_eq!(first, second);
}

#[test]
fn a_step_a_knob_cant_apply_is_skipped_and_the_strides_halve() {
    // Target 3 (-6) can't be had: at stride 2 the search stops at -4
    // (50%), halves the stride, and finds -5 (target 4, 60%).
    let knob = Target {
        none_from: Some(-6),
        ..Target::new(-9..=0, 2)
    };
    let found = search(9, &[&knob], &band(60, 70), &SETTINGS);
    assert_eq!((found.stop, found.stage), (Stop::InBands, 4));
    assert_eq!(found.changes[0].step, -5);
    assert_eq!(knob.applied()[..5], [-2, -4, -6, -3, -5]);
}

#[test]
fn a_search_that_cant_reach_the_band_is_stuck_nearest_it() {
    // The knob goes down to target 4 (60%) at most; the band wants 95%.
    let knob = Target::new(-5..=0, 1);
    let found = search(9, &[&knob], &band(95, 100), &SETTINGS);
    assert_eq!(found.stop, Stop::Stuck);
    assert_eq!((found.stage, successes(&found.after)), (4, 60));
    assert!(distance(&found.after) < distance(&found.before));
}

#[test]
fn a_budget_of_three_stops_with_the_best_setting_seen() {
    // Quick runs of all 100 tries are full runs: before (0), -2 and -4,
    // and no evaluation left for -6.
    let settings = BalanceSettings {
        quick: 100,
        evaluations: 3,
        ..SETTINGS
    };
    let found = search(9, &[&Target::new(-9..=0, 2)], &band(60, 70), &settings);
    assert_eq!(found.stop, Stop::Budget);
    assert_eq!(found.evaluations, 3);
    assert_eq!((found.stage, successes(&found.after)), (5, 50));
}

#[test]
fn a_budget_keeps_back_one_evaluation_for_the_full_run() {
    // Before, quick runs of 0 and -2, and -2 in full.
    let settings = BalanceSettings {
        evaluations: 4,
        ..SETTINGS
    };
    let found = search(9, &[&Target::new(-9..=0, 2)], &band(60, 70), &settings);
    assert_eq!((found.stop, found.evaluations), (Stop::Budget, 4));
    assert_eq!((found.stage, successes(&found.after)), (7, 30));
}

#[test]
fn a_quick_run_in_the_band_that_the_full_run_isnt_carries_on_in_full() {
    // Quick runs of 15 tries: coins 0 to 9, then 0 to 4. Target 2 wins
    // 11 of 15 there (73%, in 65–75%) but 80 of 100 in full; target 3
    // wins 70 of 100.
    let settings = BalanceSettings {
        quick: 15,
        ..SETTINGS
    };
    let found = search(9, &[&Target::new(-9..=0, 2)], &band(65, 75), &settings);
    assert_eq!((found.stop, found.stage), (Stop::InBands, 3));
    assert_eq!(successes(&found.after), 70);
}

#[test]
fn a_setting_no_nearer_its_bands_in_full_than_the_stage_as_authored_is_dropped() {
    // In full, target 9 wins 10% and target 1 wins 90%: both 3 000 from
    // 40–60%. The quick runs of 5 tries (coins 0 to 4) find target 1
    // nearer (4 of 5 against none), and the budget runs out there.
    let settings = BalanceSettings {
        quick: 5,
        evaluations: 4,
        ..SETTINGS
    };
    let found = search(9, &[&Target::new(-8..=0, 8)], &band(40, 60), &settings);
    assert_eq!((found.stop, found.evaluations), (Stop::Budget, 4));
    assert_eq!((found.stage, found.changes.len()), (9, 0));
    assert_eq!(found.after, found.before);
}

#[test]
fn the_prune_removes_a_change_that_isnt_needed() {
    let (target, idle) = (
        Target::new(-9..=0, 2),
        Idle {
            applied: RefCell::new(Vec::new()),
        },
    );
    let knobs: [&dyn Knob<i32>; 2] = [&idle, &target];
    let entrants: [&dyn Entrant<i32, CoinRace>; 1] = [&Counter];
    let checks = band(60, 70);
    let mut search = Search::new(
        &9,
        &knobs,
        &SETTINGS,
        |stage: &i32, tries: &Tries| playtest::run(stage, &entrants, tries, &checks),
        || false,
    );
    // The idle knob at +2 with target 3 (70%): the idle knob goes, and the
    // target goes to 4 (60%).
    assert_eq!(search.prune(vec![2, -6]).unwrap(), [0, -5]);
    // The whole search never moves the idle knob, whose steps lower
    // nothing: each round tries it at +1 and -1, then the target. Before,
    // 0, three rounds of three quick runs, -6 in full, then -5 and -4.
    let found = autobalance(&9, &knobs, &entrants, &checks, &SETTINGS).unwrap();
    assert_eq!(found.changes.len(), 1);
    assert_eq!((found.changes[0].knob.as_str(), found.stage), ("target", 4));
    assert_eq!(found.evaluations, 14);
    // A knob's stride is 1 unless it says.
    assert_eq!(idle.stride(), 1);
    assert!(idle.applied.borrow().iter().all(|s| s.abs() == 1));
}

#[test]
fn a_check_naming_no_entrant_fails_before_any_try() {
    let checks = vec![Check::Rate {
        entrant: "nobody".to_string(),
        success: won(),
        band: Band { min: 0, max: 100 },
        floor: None,
    }];
    let knob = Target::new(-9..=0, 2);
    let error = autobalance(&9, &[&knob], &[&Counter], &checks, &SETTINGS).unwrap_err();
    assert!(matches!(
        error,
        PlaytestError::UnknownEntrant(UnknownEntrant { check: 0, .. })
    ));
}

/// The coin race's search by [`autobalance_by`], each setting played by
/// the runner and never told to stop: what it found, and the try counts
/// of the runs it asked for, in turn.
fn search_by(target: i32, knob: &Target, settings: &BalanceSettings) -> (Balanced<i32>, Vec<u32>) {
    let checks = band(60, 70);
    let asked = RefCell::new(Vec::new());
    let found = autobalance_by(
        &target,
        &[knob],
        settings,
        |stage: &i32, tries: &Tries| {
            asked.borrow_mut().push(tries.count);
            playtest::run(stage, &[&Counter], tries, &checks)
        },
        || false,
    )
    .unwrap();
    (found, asked.into_inner())
}

#[test]
fn by_the_runner_it_finds_what_autobalance_finds() {
    for settings in [
        SETTINGS,
        BalanceSettings {
            evaluations: 4,
            ..SETTINGS
        },
    ] {
        let (found, asked) = search_by(9, &Target::new(-9..=0, 2), &settings);
        let want = search(9, &[&Target::new(-9..=0, 2)], &band(60, 70), &settings);
        assert_eq!(found, want);
        // One run a call, the stage as authored first, in full.
        assert_eq!(asked.len(), usize::try_from(found.evaluations).unwrap());
        assert_eq!(asked[0], 100);
    }
    // Before, the quick runs of 0, -2, -4 and -6, then -6, -5 and -4 in
    // full.
    let (_, asked) = search_by(9, &Target::new(-9..=0, 2), &SETTINGS);
    assert_eq!(asked, [100, 10, 10, 10, 10, 100, 100, 100]);
}

/// The coin race's report of `target` at `tries`, made up without a game:
/// the try of seed `s` is won when `s % 10` reaches the target.
fn made_up(target: i32, tries: &Tries, checks: &[Check]) -> Result<PlaytestReport, UnknownEntrant> {
    let played = tries
        .seeds()
        .map(|seed| Tried {
            seed,
            outcome: None,
            turns: 1,
            measures: [(
                "won".to_string(),
                i64::from(i32::try_from(seed % 10).unwrap() >= target),
            )]
            .into(),
        })
        .collect();
    judge(vec![("counter".to_string(), played)], checks)
}

#[test]
fn a_made_up_evaluator_steers_the_search_as_the_runner_does() {
    let checks = band(60, 70);
    let knob = Target::new(-9..=0, 2);
    let found = autobalance_by(
        &9,
        &[&knob],
        &SETTINGS,
        |&stage: &i32, tries: &Tries| made_up(stage, tries, &checks),
        || false,
    )
    .unwrap();
    let want = search(9, &[&Target::new(-9..=0, 2)], &checks, &SETTINGS);
    assert_eq!(
        (found.stage, &found.changes, found.stop, found.evaluations),
        (want.stage, &want.changes, want.stop, want.evaluations)
    );
    assert_eq!(found.after.checks, want.after.checks);
    assert_eq!(found.before.checks, want.before.checks);
}

#[test]
fn the_first_error_of_the_evaluator_stops_the_search() {
    let checks = band(60, 70);
    let mut calls = 0;
    let error = autobalance_by(
        &9,
        &[&Target::new(-9..=0, 2)],
        &SETTINGS,
        |&stage: &i32, tries: &Tries| {
            calls += 1;
            if calls == 3 {
                return Err(format!("run {calls}"));
            }
            made_up(stage, tries, &checks).map_err(|e| e.to_string())
        },
        || false,
    )
    .unwrap_err();
    assert_eq!((error.as_str(), calls), ("run 3", 3));
}

/// The coin race's search from target 9 by the runner, `should_stop`
/// saying to stop from its `from`th question on: what it found, and how
/// often it was asked.
fn stopped_from(from: u32) -> (Balanced<i32>, u32) {
    let checks = band(60, 70);
    let knob = Target::new(-9..=0, 2);
    let mut asked = 0;
    let found = autobalance_by(
        &9,
        &[&knob],
        &SETTINGS,
        |stage: &i32, tries: &Tries| playtest::run(stage, &[&Counter], tries, &checks),
        || {
            asked += 1;
            asked >= from
        },
    )
    .unwrap();
    (found, asked)
}

#[test]
fn told_to_stop_at_once_it_plays_the_stage_as_authored_alone() {
    let (found, asked) = stopped_from(1);
    assert_eq!((found.stop, found.evaluations), (Stop::Stopped, 1));
    assert_eq!((found.stage, found.changes.len()), (9, 0));
    assert_eq!(found.after, found.before);
    // Asked before the first run of the search, and never again.
    assert_eq!(asked, 1);
    assert!(
        found
            .to_string()
            .starts_with("stopped: the best setting seen · 1 evaluations\nno change\n")
    );
}

#[test]
fn told_to_stop_later_it_keeps_the_best_setting_seen_and_plays_it_in_full() {
    // Asked before the quick runs of 0 and -2, then told to stop before
    // -4: -2, the best seen, plays in full, as with a budget of 4.
    let (found, asked) = stopped_from(3);
    assert_eq!((found.stop, found.evaluations), (Stop::Stopped, 4));
    assert_eq!((found.stage, successes(&found.after)), (7, 30));
    assert_eq!(asked, 3);
    let budget = BalanceSettings {
        evaluations: 4,
        ..SETTINGS
    };
    let spent = search(9, &[&Target::new(-9..=0, 2)], &band(60, 70), &budget);
    assert_eq!(
        (found.stage, found.changes, found.after),
        (spent.stage, spent.changes, spent.after)
    );
}

#[test]
fn a_stop_while_pruning_keeps_a_setting_in_every_band() {
    // -6 is in the band in full (the fifth question); told to stop before
    // -5, the prune keeps -6.
    let (found, asked) = stopped_from(6);
    assert_eq!((found.stop, found.stage), (Stop::InBands, 3));
    assert_eq!((found.evaluations, asked), (6, 6));
    assert_eq!(distance(&found.after), 0);
    // Told to stop as it confirms -6 in full, it plays -6 in full all the
    // same: the run kept back.
    let (found, _) = stopped_from(5);
    assert_eq!(
        (found.stop, found.stage, found.evaluations),
        (Stop::Stopped, 3, 6)
    );
    assert_eq!(successes(&found.after), 70);
}

#[test]
fn the_distance_sums_how_far_each_rate_is_outside_and_skips_medians() {
    let rate = |successes, count, min, max| Judged {
        check: Check::Rate {
            entrant: "a".to_string(),
            success: won(),
            band: Band { min, max },
            floor: None,
        },
        found: Found::Rate {
            successes,
            count,
            verdict: Band { min, max }.verdict(successes, count),
            blocked: false,
        },
    };
    let median = Judged {
        check: Check::Median {
            entrant: "a".to_string(),
            measure: "turns".to_string(),
            among: Vec::new(),
            want: Want::Lower,
            than: "a".to_string(),
        },
        found: Found::Median {
            median: Some(1),
            than: Some(1),
            verdict: MedianVerdict::Fail,
        },
    };
    let report = |checks| PlaytestReport {
        entrants: Vec::new(),
        checks,
        blocked: false,
    };
    // 3 of 10 against 60–70%: 600 - 300. 9 of 10 against 60–70%: 900 -
    // 700. 5 of 10 inside 40–60%: 0.
    assert_eq!(distance(&report(vec![rate(3, 10, 60, 70)])), 300);
    assert_eq!(distance(&report(vec![rate(9, 10, 60, 70)])), 200);
    assert_eq!(
        distance(&report(vec![
            rate(3, 10, 60, 70),
            rate(9, 10, 60, 70),
            rate(5, 10, 40, 60),
            median,
        ])),
        500
    );
    assert_eq!(distance(&report(Vec::new())), 0);
}

proptest! {
    #[test]
    fn the_distance_is_0_iff_every_rate_is_within(
        rates in proptest::collection::vec(
            (0..200_u32, 0..200_u32, 0..=100_u32, 0..=100_u32),
            0..5,
        ),
    ) {
        let checks: Vec<Judged> = rates
            .iter()
            .map(|&(successes, count, min, max)| {
                let successes = successes.min(count);
                Judged {
                    check: Check::Rate {
                        entrant: "a".to_string(),
                        success: Vec::new(),
                        band: Band { min, max },
                        floor: None,
                    },
                    found: Found::Rate {
                        successes,
                        count,
                        verdict: Band { min, max }.verdict(successes, count),
                        blocked: false,
                    },
                }
            })
            .collect();
        let within = checks.iter().all(|j| {
            matches!(j.found, Found::Rate { verdict: RateVerdict::Within, .. })
        });
        let report = PlaytestReport { entrants: Vec::new(), checks, blocked: false };
        prop_assert_eq!(distance(&report) == 0, within);
    }

    #[test]
    fn every_setting_tried_stays_inside_every_knobs_range(
        authored in 0..=10_i32,
        low in -10..=0_i32,
        high in 0..=5_i32,
        stride in 0..=9_u32,
        min in 0..=100_u32,
        width in 0..=40_u32,
        quick in prop_oneof![Just(5_u32), Just(10), Just(15), Just(100)],
        evaluations in 1..=60_u32,
    ) {
        let target = Target::new(low..=high, stride);
        let idle = Idle { applied: RefCell::new(Vec::new()) };
        let settings = BalanceSettings { quick, evaluations, ..SETTINGS };
        let found = search(
            authored,
            &[&target, &idle],
            &band(min, (min + width).min(100)),
            &settings,
        );
        prop_assert!(target.applied().iter().all(|s| (low..=high).contains(s)));
        prop_assert!(idle.applied.borrow().iter().all(|s| (-3..=3).contains(s)));
        prop_assert!(found.evaluations <= evaluations.max(1));
        prop_assert!(distance(&found.after) <= distance(&found.before));
        if matches!(found.stop, Stop::InBands | Stop::AlreadyInBands) {
            prop_assert_eq!(distance(&found.after), 0);
        }
        prop_assert_eq!(found.changes.is_empty(), found.stage == authored);
    }
}
