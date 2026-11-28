//~ `Stage` is not a game's rules
//~ write `impl TurnRules for Stage`
//~ `check`, `apply`, `legal_commands` and `outcome`
//~ https://zafnok.github.io/shuyi-engine/learn/book/rules-as-commands/
//! A game's code with its rules, their `impl TurnRules` missing (ticket 2510).

use shuyi::core::Sides;

/// The game's state.
struct Stage {
    turn: u32,
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

fn main() {
    let _ = Stage { turn: 0 }.turn();
}
