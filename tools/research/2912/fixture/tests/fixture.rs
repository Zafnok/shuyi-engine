//! Deterministic replay and state-transition checks for both public fixtures.

use cloning_fixture::{courier, workshop};
use shuyi_core::{TurnRules, state_hash};

fn courier_play(seed: u64) -> (courier::Courier, Vec<String>) {
    let mut game = courier::Courier::new(seed);
    let mut frames = vec![game.frame()];
    let first = if game.parcel_y == 1 {
        courier::Action::Up
    } else {
        courier::Action::Down
    };
    let second = if game.parcel_y == 1 {
        courier::Action::Down
    } else {
        courier::Action::Up
    };
    let actions = [
        first,
        courier::Action::Right,
        second,
        courier::Action::Right,
        courier::Action::Right,
        courier::Action::Right,
        courier::Action::Right,
        courier::Action::Right,
    ];
    for action in actions {
        assert!(game.apply(&action).is_ok(), "{action:?}");
        frames.push(game.frame());
    }
    (game, frames)
}

#[test]
fn courier_replays_every_frame_and_can_deliver_across_seeded_bridges() {
    for seed in [0, 1, 11, 55, u64::MAX] {
        let (first, frames) = courier_play(seed);
        let (second, again) = courier_play(seed);
        assert_eq!(first, second);
        assert_eq!(frames, again);
        assert_eq!(state_hash(&first).unwrap(), state_hash(&second).unwrap());
        assert_eq!(first.outcome(), Some(courier::Outcome::Won));
        assert_eq!(first.legal_commands(), Vec::<courier::Action>::new());
    }
}

#[test]
fn courier_refusal_is_atomic() {
    let mut game = courier::Courier::new(3);
    let before = game.clone();
    assert_eq!(
        game.check(&courier::Action::Left),
        Err(courier::Error::Blocked)
    );
    assert_eq!(
        game.apply(&courier::Action::Left),
        Err(courier::Error::Blocked)
    );
    assert_eq!(game, before);
}

fn workshop_play(seed: u64) -> (workshop::Workshop, Vec<String>) {
    let mut game = workshop::Workshop::new(seed);
    let mut frames = vec![game.frame()];
    let actions = [
        workshop::Action::Gather,
        workshop::Action::Gather,
        workshop::Action::Refine,
        workshop::Action::Craft,
        workshop::Action::Deliver,
        workshop::Action::Gather,
        workshop::Action::Rest,
        workshop::Action::Gather,
        workshop::Action::Refine,
        workshop::Action::Craft,
        workshop::Action::Deliver,
    ];
    for action in actions {
        assert!(game.apply(&action).is_ok(), "{action:?}");
        frames.push(game.frame());
    }
    (game, frames)
}

#[test]
fn workshop_replays_every_frame_and_can_fill_orders() {
    for seed in [0, 1, 11, 55, u64::MAX] {
        let (first, frames) = workshop_play(seed);
        let (second, again) = workshop_play(seed);
        assert_eq!(first, second);
        assert_eq!(frames, again);
        assert_eq!(state_hash(&first).unwrap(), state_hash(&second).unwrap());
        assert_eq!(first.outcome(), Some(workshop::Outcome::Won));
        assert_eq!(first.legal_commands(), Vec::<workshop::Action>::new());
    }
}

#[test]
fn workshop_refusal_is_atomic() {
    let mut game = workshop::Workshop::new(3);
    let before = game.clone();
    assert_eq!(
        game.check(&workshop::Action::Craft),
        Err(workshop::Error::NoIngot)
    );
    assert_eq!(
        game.apply(&workshop::Action::Craft),
        Err(workshop::Error::NoIngot)
    );
    assert_eq!(game, before);
}
