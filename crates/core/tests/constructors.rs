//! The bots' settings, built from outside the crate as a game builds them:
//! through their constructors, since they are `#[non_exhaustive]`
//! (ADR-0039, ticket 2085).

use shuyi_core::threat::Strike;
use shuyi_core::{Horizon, PlanSettings, RolloutSettings};

/// A persona's settings, in a `const` item as a game writes them.
const PLAN: PlanSettings = PlanSettings::DEFAULT
    .with_width(4)
    .with_candidates(3)
    .with_depth(2)
    .with_samples(5)
    .with_temperature(10);

#[test]
fn the_planners_settings_are_built_by_their_constructor() {
    assert_eq!(
        (
            PLAN.width,
            PLAN.candidates,
            PLAN.depth,
            PLAN.samples,
            PLAN.temperature
        ),
        (4, 3, 2, 5, 10)
    );
    let mut colder = PLAN;
    colder.temperature = 0;
    assert_eq!(colder.temperature, 0);
}

#[test]
fn the_rollouts_settings_are_built_by_their_constructor() {
    let rollout = RolloutSettings::new(Horizon::Turns(2))
        .with_candidates(3)
        .with_samples(8)
        .with_commands(40);
    assert_eq!(
        (rollout.candidates, rollout.samples, rollout.commands),
        (3, 8, 40)
    );
    assert_eq!(rollout.horizon, Horizon::Turns(2));
}

#[test]
fn a_strike_is_built_from_its_default() {
    const STRIKE: Strike = Strike::DEFAULT
        .with_expected(450)
        .with_all_hit(6)
        .with_struck_back(120);
    assert_eq!(
        (STRIKE.expected, STRIKE.all_hit, STRIKE.struck_back),
        (450, 6, 120)
    );
    assert_eq!(Strike::DEFAULT, Strike::default());
}
