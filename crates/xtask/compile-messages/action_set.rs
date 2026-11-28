//~ `Action` is not a set of actions
//~ write `impl ActionSet for Action`, or use the engine's `UiAction`
//~ deriving `Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug`
//~ https://zafnok.github.io/shuyi-engine/learn/book/actions/
//! A game's code with its actions, their `impl ActionSet` missing (ticket 2510).

use shuyi::prelude::*;

/// What the player asks for.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
enum Action {
    Jump,
}

fn start(_data: &BasicData<Action>) {}

fn main() {
    let _ = (start, Action::Jump);
}
