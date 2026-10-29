//! Chapter 8, testing: a bot that plays the garden to its end, a property
//! that holds for any seed, and a record replayed step by step with the
//! hash of each step.

use shuyi::core::bots::{Bot, Report, play};
use shuyi::core::{RandomBot, TurnRules};

use crate::assets::Plan;
use crate::rules::{Command, Garden};

// ANCHOR: bot
/// A bot that plants wherever nothing grows yet, and walks right to the
/// next bare plot otherwise.
#[derive(Debug, Clone, Copy, Default)]
pub struct Gardener;

impl Bot<Garden> for Gardener {
    fn choose(&mut self, garden: &Garden) -> Command {
        if garden.check(&Command::Plant).is_ok() {
            Command::Plant
        } else if garden.check(&Command::Right).is_ok() {
            Command::Right
        } else {
            Command::Left
        }
    }
}

/// How one play of `bot` from `seed` ended, within 200 turns.
pub fn play_once(bot: &mut dyn Bot<Garden>, seed: u64) -> Report<Garden> {
    let mut garden = Garden::new(&Plan::new(6, 9, 75), seed);
    let report = play(&mut garden, &mut [((), bot)], 200);
    report.unwrap_or_else(|e| panic!("{e}"))
}
// ANCHOR_END: bot

/// A random bot's play, for a test to compare with.
pub fn play_randomly(seed: u64) -> Report<Garden> {
    play_once(&mut RandomBot::new(seed), seed)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use shuyi::core::state_hash;
    use shuyi_cli::contract::replay::turns;

    use super::*;
    use crate::rules::Outcome;

    // ANCHOR: test_bot
    #[test]
    fn the_gardener_ends_every_game_and_wins_some() {
        let wins = (0..50)
            .map(|seed| play_once(&mut Gardener, seed))
            .filter(|report| report.outcome == Some(Outcome::Won))
            .count();
        assert!((10..50).contains(&wins), "{wins} wins in 50");
        assert!(play_randomly(1).commands.len() <= 200);
    }
    // ANCHOR_END: test_bot

    // ANCHOR: property
    proptest! {
        #[test]
        fn any_seed_and_commands_play_the_same_twice(
            seed in any::<u64>(),
            commands in prop::collection::vec(0..3_u8, 0..40),
        ) {
            let run = || {
                let mut garden = Garden::new(&Plan::new(6, 9, 75), seed);
                for &n in &commands {
                    let command = [Command::Left, Command::Right, Command::Plant][usize::from(n)];
                    let _ = garden.apply(&command);
                }
                state_hash(&garden).unwrap()
            };
            prop_assert_eq!(run(), run());
        }
    }
    // ANCHOR_END: property

    // ANCHOR: replay
    #[test]
    fn a_record_replays_with_a_hash_at_every_step() {
        let commands = [Command::Plant, Command::Left, Command::Right];
        let garden = Garden::new(&Plan::new(6, 9, 75), 7);
        let run = turns("records/one.ron", garden, &commands).unwrap();
        // Step 0 is the state before the first command.
        assert_eq!(run.steps.len(), 4);
        // At the left edge, Left is refused, and changes nothing.
        assert!(run.steps[2].refused.is_some());
        assert_eq!(run.steps[2].hash, run.steps[1].hash);
    }
    // ANCHOR_END: replay
}
