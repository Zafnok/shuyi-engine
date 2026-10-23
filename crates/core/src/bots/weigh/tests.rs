//! Tests of weighted features and personas, on Nim.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};

use proptest::prelude::*;
use serde::Deserialize;

use super::super::fixtures::Nim;
use super::super::{Bot, TurnPlan, Unordered};
use super::*;
use crate::turn::TurnRules;

/// Nim's features.
const NAMES: [&str; 3] = ["won", "lost", "taken"];

/// Nim's counts of [`NAMES`] for `side`, from `start` to `end`.
fn nim_counts(start: &Nim, end: &Nim, side: usize) -> [i64; 3] {
    let winner = end.outcome();
    [
        i64::from(winner == Some(side)),
        i64::from(winner.is_some_and(|winner| winner != side)),
        i64::from(start.pile - end.pile),
    ]
}

/// A map of weights by name.
fn named(weights: &[(&str, i32)]) -> BTreeMap<String, i32> {
    weights
        .iter()
        .map(|&(name, weight)| (name.to_string(), weight))
        .collect()
}

#[test]
fn an_undeclared_feature_is_an_error_listing_the_known_ones() {
    let error = Weights::from_named(&NAMES, &named(&[("won", 1), ("hp", 2)])).unwrap_err();
    assert_eq!(
        error,
        UnknownFeature {
            name: "hp".into(),
            known: "won, lost, taken".into(),
        }
    );
    assert_eq!(
        error.to_string(),
        "no feature \"hp\" (there: won, lost, taken)"
    );
}

#[test]
fn a_missing_weight_is_0() {
    let weights = Weights::from_named(&NAMES, &named(&[("taken", -3)])).unwrap();
    assert_eq!(weights, Weights([0, 0, -3]));
    assert_eq!(weights.get(&NAMES, "won"), Some(0));
    assert_eq!(weights.get(&NAMES, "taken"), Some(-3));
    assert_eq!(weights.get(&NAMES, "hp"), None);
    assert_eq!(weights.score(&[5, 5, 2]), -6);
}

#[test]
fn score_saturates_rather_than_wrapping() {
    assert_eq!(Weights([2]).score(&[i64::MAX]), i32::MAX);
    assert_eq!(Weights([2, 2]).score(&[i64::MAX, i64::MAX]), i32::MAX);
    assert_eq!(Weights([-2]).score(&[i64::MAX]), i32::MIN);
    assert_eq!(Weights([2]).score(&[i64::MIN]), i32::MIN);
    assert_eq!(Weights::<0>([]).score(&[]), 0);
}

#[test]
fn weighted_is_an_evaluate_of_the_counts() {
    let eval = Weighted {
        weights: Weights([1000, -1000, 5]),
        counts: nim_counts,
    };
    let start = Nim::new(3);
    let mut end = start.clone();
    end.apply(&3).unwrap();
    // Side 0 took the last 3 and won.
    assert_eq!(eval.score(&start, &end, 0), 1015);
    assert_eq!(eval.score(&start, &end, 1), -985);
}

/// The game's own extras of a test persona.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Table {
    pile: u32,
}

/// Two personas differing only in what they think of taking.
const PERSONAS: &str = r#"{
    "greedy": (
        settings: (width: 2, candidates: 3, depth: 1, samples: 1, temperature: 0),
        weights: {"won": 1000, "taken": 1},
        game: (pile: 10),
    ),
    "sparing": (
        settings: (width: 2, candidates: 3, depth: 1, samples: 1, temperature: 0),
        weights: {"won": 1000, "taken": -1},
        game: (pile: 10),
    ),
}"#;

/// The first command `persona` picks, on the pile its extras name.
fn first_command(persona: &Persona<Table>) -> u32 {
    let eval = Weighted {
        weights: persona.weights(&NAMES).unwrap(),
        counts: nim_counts,
    };
    let mut bot = TurnPlan::new(Unordered, eval, persona.settings, 7);
    bot.choose(&Nim::new(persona.game.pile))
}

#[test]
fn personas_from_one_file_with_opposite_weights_pick_different_commands() {
    let personas: BTreeMap<String, Persona<Table>> = ron::from_str(PERSONAS).unwrap();
    assert_eq!(first_command(&personas["greedy"]), 3);
    assert_eq!(first_command(&personas["sparing"]), 1);
}

#[test]
fn a_persona_may_leave_out_its_weights_and_extras() {
    let persona: Persona<()> = ron::from_str(
        "(settings: (width: 1, candidates: 1, depth: 1, samples: 1, temperature: 0))",
    )
    .unwrap();
    assert!(persona.weights.is_empty(), "{:?}", persona.weights);
    assert_eq!(persona.weights(&NAMES).unwrap(), Weights([0; 3]));
}

#[test]
fn a_persona_refuses_unknown_fields_and_features() {
    let settings = "settings: (width: 1, candidates: 1, depth: 1, samples: 1, temperature: 0)";
    assert!(ron::from_str::<Persona<()>>(&alloc::format!("({settings}, mood: 1)")).is_err());
    let persona: Persona<()> =
        ron::from_str(&alloc::format!("({settings}, weights: {{\"hp\": 1}})")).unwrap();
    assert_eq!(persona.weights(&NAMES).unwrap_err().name, "hp");
}

proptest! {
    #[test]
    fn score_is_the_wide_sum_clamped(
        counts in prop::array::uniform4(any::<i64>()),
        weights in prop::array::uniform4(any::<i32>()),
    ) {
        let wide: i128 = counts
            .iter()
            .zip(weights)
            .map(|(&count, weight)| i128::from(count) * i128::from(weight))
            .sum();
        let clamped = i32::try_from(wide.clamp(i128::from(i32::MIN), i128::from(i32::MAX))).unwrap();
        prop_assert_eq!(Weights(weights).score(&counts), clamped);
    }
}
