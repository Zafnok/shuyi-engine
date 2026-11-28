//~ `Chase` is not a bot of `Stage`
//~ write `impl Bot<Stage> for Chase`
//~ one method, `choose`
//~ https://zafnok.github.io/shuyi-engine/learn/book/rules-as-commands/
//! A game's code with its bot, its `impl Bot` missing (ticket 2510).

use shuyi::core::{Bot, Sides, TurnRules};

/// The game's state.
#[derive(Debug, Clone)]
struct Stage {
    turn: u32,
}

impl TurnRules for Stage {
    type Command = u8;
    type Event = u8;
    type Error = String;
    type Outcome = ();

    fn check(&self, _command: &u8) -> Result<(), String> {
        Ok(())
    }

    fn apply(&mut self, command: &u8) -> Result<Vec<u8>, String> {
        self.turn += 1;
        Ok(vec![*command])
    }

    fn legal_commands(&self) -> Vec<u8> {
        vec![0]
    }

    fn outcome(&self) -> Option<()> {
        None
    }
}

impl Sides for Stage {
    type Side = u8;

    fn side_to_move(&self) -> u8 {
        0
    }

    fn turn(&self) -> u32 {
        self.turn
    }
}

/// A bot that chases the player.
struct Chase;

fn main() {
    let mut stage = Stage { turn: 0 };
    let mut chase = Chase;
    let bots: &mut [(u8, &mut dyn Bot<Stage>)] = &mut [(0, &mut chase)];
    let _ = shuyi::core::play(&mut stage, bots, 10);
}
