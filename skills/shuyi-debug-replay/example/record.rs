//! A record kept as a test: the seed and the commands of
//! `records/bump.ron`, played by the rules, and what must hold of them.

use rules::{Command, Error, Stage, parse_map};
use shuyi_core::{Dir, TurnRules, state_hash};

/// The stage's map, as the game reads it.
const MAP: &str = include_str!("../../game/files/stage.map");

#[test]
fn a_step_into_a_wall_is_refused_and_changes_nothing() {
    let map = parse_map("stage.map", MAP).unwrap();
    let mut stage = Stage::new(&map, 0).unwrap();
    stage.apply(&Command::Step(Dir::Right)).unwrap();
    let before = state_hash(&stage).unwrap();
    assert_eq!(
        stage.apply(&Command::Step(Dir::Up)),
        Err(Error::Blocked(Dir::Up))
    );
    assert_eq!(state_hash(&stage).unwrap(), before);
}

#[test]
fn the_same_record_plays_the_same_twice() {
    let map = parse_map("stage.map", MAP).unwrap();
    let play = || {
        let mut stage = Stage::new(&map, 0).unwrap();
        for command in [Command::Step(Dir::Right), Command::Step(Dir::Right)] {
            stage.apply(&command).unwrap();
        }
        state_hash(&stage).unwrap()
    };
    assert_eq!(play(), play());
}
