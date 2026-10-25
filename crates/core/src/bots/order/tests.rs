//! Tests of the ranked order, alone and as the counters' order for the
//! turn planner.

use alloc::vec;
use alloc::vec::Vec;

use proptest::prelude::*;

use super::super::fixtures::{Counter, Counters, points_ahead};
use super::super::{Bot, PlanSettings, TurnPlan};
use super::*;
use crate::turn::TurnRules;

/// `command` in group 0, worth 0, one of `key`.
fn keyed(command: &'static str, key: char) -> Ranked<&'static str, char> {
    Ranked::new(command, 0, (0, 0), Some(key))
}

#[test]
fn one_per_key_comes_first() {
    let items = vec![keyed("a1", 'a'), keyed("a2", 'a'), keyed("b1", 'b')];
    assert_eq!(ranked_order(items.clone(), None, 10), ["a1", "b1", "a2"]);
    assert_eq!(ranked_order(items, Some("end"), 2), ["a1", "end"]);
}

#[test]
fn the_rounds_go_all_firsts_then_all_seconds() {
    let items = vec![
        keyed("a1", 'a'),
        keyed("a2", 'a'),
        keyed("a3", 'a'),
        keyed("b1", 'b'),
        keyed("b2", 'b'),
        keyed("c1", 'c'),
    ];
    assert_eq!(
        ranked_order(items, None, 10),
        ["a1", "b1", "c1", "a2", "b2", "a3"]
    );
}

#[test]
fn groups_ascend_and_the_best_value_comes_first() {
    let items: Vec<Ranked<&str, char>> = vec![
        Ranked::new("late", 2, (9, 9), None),
        Ranked::new("weak", 0, (1, 0), None),
        Ranked::new("strong", 0, (2, -5), None),
        Ranked::new("middle", 1, (0, 0), None),
        Ranked::new("strong, safe", 0, (2, 0), None),
    ];
    assert_eq!(
        ranked_order(items, None, 10),
        ["strong, safe", "strong", "weak", "middle", "late"]
    );
}

#[test]
fn ties_keep_the_games_order() {
    let items: Vec<Ranked<u8, char>> = (0..6).map(|n| Ranked::new(n, 0, (0, 0), None)).collect();
    assert_eq!(ranked_order(items, None, 10), [0, 1, 2, 3, 4, 5]);
}

#[test]
fn a_command_with_no_key_is_never_held_back() {
    let items = vec![
        keyed("a1", 'a'),
        keyed("a2", 'a'),
        Ranked::new("free", 0, (0, 0), None),
        Ranked::new("free too", 0, (0, 0), None),
    ];
    assert_eq!(
        ranked_order(items, None, 10),
        ["a1", "free", "free too", "a2"]
    );
}

#[test]
fn a_key_is_one_per_group() {
    // The same key in another group is that group's first.
    let items = vec![
        keyed("a1", 'a'),
        keyed("a2", 'a'),
        Ranked::new("a in group 1", 1, (0, 0), Some('a')),
        Ranked::new("b in group 1", 1, (0, 0), Some('b')),
    ];
    assert_eq!(
        ranked_order(items, None, 10),
        ["a1", "a2", "a in group 1", "b in group 1"]
    );
}

#[test]
fn the_last_goes_in_place_of_the_capth_or_at_the_end() {
    let items = || -> Vec<Ranked<&str, char>> {
        vec![
            Ranked::new("1", 0, (3, 0), None),
            Ranked::new("2", 0, (2, 0), None),
            Ranked::new("3", 0, (1, 0), None),
        ]
    };
    assert_eq!(ranked_order(items(), Some("end"), 3), ["1", "2", "end"]);
    assert_eq!(
        ranked_order(items(), Some("end"), 4),
        ["1", "2", "3", "end"]
    );
    assert_eq!(
        ranked_order(items(), Some("end"), 99),
        ["1", "2", "3", "end"]
    );
    assert_eq!(ranked_order(items(), Some("end"), 1), ["end"]);
    assert_eq!(
        ranked_order(Vec::<Ranked<&str, char>>::new(), Some("end"), 3),
        ["end"]
    );
}

#[test]
fn a_cap_of_0_counts_as_1() {
    let items = || vec![keyed("a1", 'a'), keyed("b1", 'b')];
    assert_eq!(ranked_order(items(), None, 0), ["a1"]);
    assert_eq!(ranked_order(items(), Some("end"), 0), ["end"]);
    assert_eq!(ranked_order(items(), None, 2), ["a1", "b1"]);
}

/// The counters' order: the raises, counter 1's first, then ending the
/// turn kept within `cap`.
fn counters_order(cap: usize) -> impl FnMut(&Counters, Vec<Counter>) -> Vec<Counter> {
    move |_, commands| {
        let mut end = None;
        let mut items = Vec::new();
        for command in commands {
            match command {
                Counter::Raise(counter) => {
                    let value = i64::try_from(counter).unwrap_or(0);
                    items.push(Ranked::new(command, 0, (value, 0), None::<()>));
                }
                Counter::End => end = Some(command),
            }
        }
        ranked_order(items, end, cap)
    }
}

#[test]
fn on_the_counters_the_end_of_the_turn_is_never_cut() {
    let state = Counters::at([0, 0], false, 1);
    let legal = state.legal_commands();
    assert_eq!(
        counters_order(9)(&state, legal.clone()),
        [Counter::Raise(1), Counter::Raise(0), Counter::End]
    );
    assert_eq!(
        counters_order(2)(&state, legal.clone()),
        [Counter::Raise(1), Counter::End]
    );
    assert_eq!(counters_order(1)(&state, legal), [Counter::End]);
}

#[test]
fn the_planner_finds_the_combo_through_a_capped_ranked_order() {
    // Two candidates a partial turn: a raise and the end. The raise pays
    // only with the other raise, which the order lists next.
    let settings = PlanSettings {
        width: 2,
        candidates: 2,
        depth: 2,
        samples: 1,
        temperature: 0,
    };
    let mut planner = TurnPlan::new(counters_order(2), points_ahead, settings, 3);
    let state = Counters::at([0, 0], false, 1);
    assert_eq!(planner.choose(&state), Counter::Raise(1));
}

/// Items numbered in order, from `(group, value, key)`.
fn numbered(specs: &[(u8, i64, Option<u8>)]) -> Vec<Ranked<usize, u8>> {
    specs
        .iter()
        .enumerate()
        .map(|(n, &(group, value, key))| Ranked::new(n, group, (value, 0), key))
        .collect()
}

proptest! {
    #[test]
    fn the_order_is_a_capped_ranking_of_what_was_given(
        specs in proptest::collection::vec((0u8..3, 0i64..3, proptest::option::of(0u8..3)), 0..12),
        end in any::<bool>(),
        cap in 0usize..16,
    ) {
        let items = numbered(&specs);
        let last = end.then_some(usize::MAX);
        let full = ranked_order(items.clone(), None, usize::MAX);
        let order = ranked_order(items, last, cap);

        // The full order is every item once, groups ascending, and in a
        // group's round the best value first.
        let mut sorted = full.clone();
        sorted.sort_unstable();
        prop_assert_eq!(sorted, (0..specs.len()).collect::<Vec<_>>());
        let ranks: Vec<(u8, usize, core::cmp::Reverse<i64>)> = full
            .iter()
            .enumerate()
            .map(|(at, &n)| {
                let (group, value, key) = specs[n];
                let earlier = full[..at]
                    .iter()
                    .filter(|&&m| key.is_some() && specs[m].0 == group && specs[m].2 == key)
                    .count();
                (group, earlier, core::cmp::Reverse(value))
            })
            .collect();
        prop_assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]));

        // The capped order is the full one's start, with the last kept.
        let cap = cap.max(1);
        let kept = if end { cap - 1 } else { cap };
        let kept = kept.min(full.len());
        prop_assert_eq!(&order[..kept], &full[..kept]);
        prop_assert_eq!(order.len(), kept + usize::from(end));
        if end {
            prop_assert_eq!(order.last(), Some(&usize::MAX));
        }
    }
}
