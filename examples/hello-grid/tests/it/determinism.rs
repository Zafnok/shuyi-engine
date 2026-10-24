//! The same seed and the same commands give the same game: in the rules
//! (every event and the state after), and through the whole game played
//! by the same keys (the same commands, the same frame).

use hello_grid::Hello;
use hello_grid::rules::{Chase, Command, Event, Side, Stage};
use shuyi::core::{Bot, RandomBot, TurnRules, play};
use shuyi::platform::harness::Harness;

use crate::{play_out, stage, title, to_the_goal};

fn stage_of(seed: u64) -> Stage {
    let game = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    Stage::new(&game.stage_map, seed).unwrap_or_else(|e| panic!("{e}"))
}

/// The commands of a whole play of stage `seed`: a random bot against
/// the chaser.
fn commands(seed: u64) -> Vec<Command> {
    let mut stage = stage_of(seed);
    let mut hero = RandomBot::new(seed);
    let mut bots: [(Side, &mut dyn Bot<Stage>); 2] =
        [(Side::Hero, &mut hero), (Side::Foe, &mut Chase)];
    play(&mut stage, &mut bots, 200)
        .unwrap_or_else(|e| panic!("seed {seed}: {e}"))
        .commands
}

/// Stage `seed` with `commands` applied: every event, and the state.
fn replay(seed: u64, commands: &[Command]) -> (Vec<Vec<Event>>, Stage) {
    let mut stage = stage_of(seed);
    let events = commands
        .iter()
        .map(|c| stage.apply(c).unwrap_or_else(|e| panic!("{c:?}: {e}")))
        .collect();
    (events, stage)
}

#[test]
fn the_same_seed_and_commands_give_the_same_events_and_state() {
    for seed in [1, 2, 3] {
        let commands = commands(seed);
        let (events, end) = replay(seed, &commands);
        assert_eq!(replay(seed, &commands), (events.clone(), end.clone()));
        assert!(end.outcome().is_some(), "seed {seed}");
        assert_eq!(events.len(), commands.len());
    }
}

#[test]
fn the_seed_is_the_luck() {
    // Your piece waits; the foe steps toward it each turn. Some seeds
    // trip it on its first step and some don't.
    let first_step = |seed: u64| {
        let mut stage = stage_of(seed);
        stage.apply(&Command::Move(stage.hero())).unwrap();
        let step = Chase.choose(&stage);
        stage.apply(&step).unwrap()
    };
    let tripped = (0..50)
        .filter(|&s| first_step(s) == [Event::Tripped])
        .count();
    assert!(tripped > 0 && tripped < 50, "{tripped} of 50 tripped");
    for seed in 0..50 {
        assert_eq!(first_step(seed), first_step(seed));
    }
}

/// The game from its title to the end of a stage, by keys.
fn played(seed: u64) -> Harness<Hello> {
    let mut h = title(seed);
    h.keys("Enter Backspace");
    play_out(&mut h, to_the_goal);
    h
}

#[test]
fn the_same_seed_and_keys_play_the_same_game_frame_for_frame() {
    let mut h = title(11);
    h.keys("Enter Backspace");
    let mut again = title(11);
    again.keys("Enter Backspace");
    for _ in 0..3 {
        let to = to_the_goal(&stage(&h).stage);
        crate::walk_to(&mut h, to);
        crate::walk_to(&mut again, to);
        assert_eq!(stage(&h).commands, stage(&again).commands);
        assert_eq!(stage(&h).stage, stage(&again).stage);
        assert_eq!(h.snapshot(), again.snapshot());
    }
    let (a, b) = (played(11), played(11));
    assert_eq!(a.snapshot(), b.snapshot());
}
