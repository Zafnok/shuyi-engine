//! An autobalance search over the stage (`shuyi_core::autobalance`): the
//! knobs of `tools::knobs` turn a stage whose foe never trips until the
//! walker sees it stumble as often as the band below wants. The heroes of
//! `tools::ENTRANTS` and the checks of `playtest.ron` stay as they are.

use std::cell::RefCell;
use std::ops::RangeInclusive;

use hello_grid::Hello;
use hello_grid::rules::{Stage, Tuning};
use hello_grid::tools::{ENTRANTS, knobs, playtest_checks};
use shuyi_core::bots::balance::Change;
use shuyi_core::bots::playtest::{Band, Check, Condition, Entrant, Found, PlaytestReport, Tries};
use shuyi_core::{BalanceSettings, Balanced, Knob, Stop, autobalance};

/// Seeds 0 to 49 decide, quick runs of 10, at most 40 runs.
const SETTINGS: BalanceSettings = BalanceSettings {
    tries: Tries {
        count: 50,
        seed: 0,
        turns: 200,
    },
    quick: 10,
    evaluations: 40,
};

/// The foe should stumble at least once in 50–70% of the walker's tries.
/// With no trip chance it never does: 0 of 50 (measured once, with these
/// seeds); at 20% it does in 30 of 50, at 15% in 20.
fn stumbles() -> Check {
    Check::Rate {
        entrant: "walk".to_owned(),
        success: vec![Condition {
            measure: "foe trips".to_owned(),
            min: Some(1),
            max: None,
        }],
        band: Band { min: 50, max: 70 },
        floor: None,
    }
}

/// A knob that writes down every step it is applied with.
struct Watched<K> {
    knob: K,
    applied: RefCell<Vec<i32>>,
}

impl<K: Knob<Stage>> Watched<K> {
    fn new(knob: K) -> Self {
        Self {
            knob,
            applied: RefCell::new(Vec::new()),
        }
    }

    /// Whether every step it was applied with is in its range.
    fn stayed_in_range(&self) -> bool {
        let range = self.knob.range();
        self.applied.borrow().iter().all(|s| range.contains(s))
    }
}

impl<K: Knob<Stage>> Knob<Stage> for Watched<K> {
    fn name(&self) -> &str {
        self.knob.name()
    }

    fn range(&self) -> RangeInclusive<i32> {
        self.knob.range()
    }

    fn stride(&self) -> u32 {
        self.knob.stride()
    }

    fn apply(&self, stage: &Stage, step: i32) -> Option<Stage> {
        self.applied.borrow_mut().push(step);
        self.knob.apply(stage, step)
    }

    fn describe(&self, step: i32) -> String {
        self.knob.describe(step)
    }
}

/// The search from a stage whose foe never trips: whether each knob
/// stayed in its range, and what it found.
fn balance() -> (bool, Balanced<Stage>) {
    let game = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    let tuning = Tuning {
        trip_percent: 0,
        ..Tuning::default()
    };
    let stage = Stage::with_tuning(&game.stage_map, 0, tuning).unwrap_or_else(|e| panic!("{e}"));
    let (trip, head_start) = knobs(tuning);
    let (trip, head_start) = (Watched::new(trip), Watched::new(head_start));
    let entrants: Vec<&dyn Entrant<Stage, Stage>> = ENTRANTS
        .iter()
        .map(|e| e as &dyn Entrant<Stage, Stage>)
        .collect();
    let mut checks = playtest_checks().unwrap_or_else(|e| panic!("{e}"));
    checks.push(stumbles());
    let found = autobalance(&stage, &[&trip, &head_start], &entrants, &checks, &SETTINGS)
        .unwrap_or_else(|e| panic!("{e}"));
    (
        trip.stayed_in_range() && head_start.stayed_in_range(),
        found,
    )
}

/// The successes of `report`'s last check, the stumbles.
fn stumbled(report: &PlaytestReport) -> u32 {
    match report.checks.last().map(|j| &j.found) {
        Some(Found::Rate { successes, .. }) => *successes,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_foe_that_never_trips_is_tuned_until_every_rate_is_in_its_band() {
    let (in_range, found) = balance();
    assert!(in_range);
    assert_eq!(found.stop, Stop::InBands);
    assert_eq!(
        found.changes,
        [Change {
            knob: "trip".to_owned(),
            step: 4,
            what: "the foe trips 20% of the time, not 0%".to_owned(),
        }]
    );
    assert_eq!(
        found.stage.tuning(),
        Tuning {
            trip_percent: 20,
            foe_waits: 0
        }
    );
    assert_eq!((stumbled(&found.before), stumbled(&found.after)), (0, 30));
    assert!(!found.after.blocked);
    let table = found.to_string();
    assert!(
        table.starts_with(&format!(
            "in every band · {} evaluations\ntrip +4: the foe trips 20% of the time, not 0%\n",
            found.evaluations
        )),
        "{table}"
    );
    assert!(found.evaluations <= SETTINGS.evaluations);
}

#[test]
fn the_same_inputs_give_the_same_changes_twice() {
    assert_eq!(balance().1, balance().1);
}

#[test]
fn the_knobs_say_their_changes_in_words_and_keep_to_their_ranges() {
    let (trip, head_start) = knobs(Tuning {
        trip_percent: 25,
        foe_waits: 1,
    });
    assert_eq!((trip.range(), head_start.range()), (-5..=10, -1..=2));
    assert_eq!((trip.stride(), head_start.stride()), (2, 1));
    assert_eq!((trip.name(), head_start.name()), ("trip", "head start"));
    assert_eq!(trip.describe(-2), "the foe trips 15% of the time, not 25%");
    assert_eq!(
        head_start.describe(1),
        "the foe waits 2 turns before it moves, not 1 turn"
    );
    let game = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    let stage = Stage::new(&game.stage_map, 0).unwrap_or_else(|e| panic!("{e}"));
    assert!(trip.apply(&stage, 11).is_none());
    let most = trip.apply(&stage, 10).map(|s| s.tuning().trip_percent);
    assert_eq!(most, Some(75));
    assert!(head_start.apply(&stage, -2).is_none());
    let tuned = head_start
        .apply(&stage, 2)
        .and_then(|s| trip.apply(&s, -5))
        .map(|s| s.tuning());
    assert_eq!(
        tuned,
        Some(Tuning {
            trip_percent: 0,
            foe_waits: 3
        })
    );
}
