//! The bots' settings, built from outside the crate as a game builds them:
//! through their constructors, since they are `#[non_exhaustive]`
//! (ADR-0039, ticket 2085).

use shuyi_core::{Horizon, PlanSettings, RolloutSettings};

/// A persona's settings, in a `const` item as a game writes them.
const PLAN: PlanSettings = PlanSettings::new(4, 3, 2, 5, 10);

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
    let rollout = RolloutSettings::new(3, 8, Horizon::Turns(2), 40, 0);
    assert_eq!(
        (rollout.candidates, rollout.samples, rollout.commands),
        (3, 8, 40)
    );
    assert_eq!(rollout.horizon, Horizon::Turns(2));
}
