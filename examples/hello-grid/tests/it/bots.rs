//! The engine's bots playing the stage's rules (`shuyi_core::bots`): a
//! random bot for your piece against the foe's own mind, and against
//! another random bot. Every command a bot chooses is one the rules
//! accept, every play ends, and the same seeds play the same game.

use hello_grid::Hello;
use hello_grid::rules::{Chase, Outcome, Side, Stage};
use shuyi_core::{Bot, RandomBot, Report, play};

/// The stage of the game's map, seeded with `seed`.
fn stage(seed: u64) -> Stage {
    let game = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    Stage::new(&game.stage_map, seed).unwrap_or_else(|e| panic!("{e}"))
}

/// A play of stage `seed` to its end (or turn 200), your piece moved by
/// a random bot of `seed`, the foe by `foe`.
fn play_with(seed: u64, foe: &mut dyn Bot<Stage>) -> Report<Stage> {
    let mut stage = stage(seed);
    let mut hero = RandomBot::new(seed);
    let mut bots: [(Side, &mut dyn Bot<Stage>); 2] = [(Side::Hero, &mut hero), (Side::Foe, foe)];
    play(&mut stage, &mut bots, 200).unwrap_or_else(|e| panic!("seed {seed}: {e}"))
}

#[test]
fn a_random_bot_against_the_chaser_always_plays_to_an_end() {
    let mut won = 0;
    let mut caught = 0;
    for seed in 0..60 {
        let report = play_with(seed, &mut Chase);
        match report.outcome {
            Some(Outcome::Won { turn }) => {
                won += 1;
                assert_eq!(turn, report.turns, "seed {seed}");
            }
            Some(Outcome::Caught { turn }) => {
                caught += 1;
                assert_eq!(turn, report.turns, "seed {seed}");
            }
            None => panic!("seed {seed}: no end in 200 turns"),
        }
        // A command for each side's move: yours, then the foe's.
        assert!(!report.commands.is_empty());
    }
    // A random walker sometimes finds the goal, and is sometimes caught.
    assert!(won > 0 && caught > 0, "won {won}, caught {caught}");
}

#[test]
fn two_random_bots_play_only_legal_commands() {
    for seed in 0..30 {
        let report = play_with(seed, &mut RandomBot::new(seed + 1000));
        assert!(report.turns >= 1);
    }
}

#[test]
fn the_same_seeds_play_the_same_game() {
    for seed in [0, 7, 42] {
        assert_eq!(play_with(seed, &mut Chase), play_with(seed, &mut Chase));
    }
}
