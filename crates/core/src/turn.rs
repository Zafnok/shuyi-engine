//! The rules of a turn-based game as the engine's generic code sees them:
//! a state that takes commands and answers with what happened. The game
//! keeps its own turn loop, phases and rules, and implements
//! [`TurnRules`] for its state; [`History`](crate::History) (replay,
//! rewind, saving a game in progress) is written against it.

use alloc::vec::Vec;

/// A turn-based game's state, and the commands that change it.
///
/// # Contract
///
/// - **Deterministic.** The same state and command always give the same
///   events and the same new state. Luck comes from a seeded RNG kept in
///   the state ([`SimRng`](crate::SimRng)), never from outside, so a game
///   can be rebuilt by replaying its commands.
/// - **All or nothing.** [`apply`](Self::apply) either changes the state
///   and returns what happened, or refuses the command with an error and
///   changes nothing.
/// - [`check`](Self::check) gives [`apply`](Self::apply)'s answer without
///   changing anything, luck included.
/// - [`legal_commands`](Self::legal_commands) lists commands
///   [`check`](Self::check) accepts, always in the same order. A game may
///   leave some out (when there are too many to list), and says which.
/// - Once [`outcome`](Self::outcome) is `Some`, the game is over and
///   [`legal_commands`](Self::legal_commands) is empty.
pub trait TurnRules {
    /// What a player (or an AI) asks for.
    type Command;
    /// What happened when a command was applied.
    type Event;
    /// Why a command was refused.
    type Error;
    /// How a game ended.
    type Outcome;

    /// Whether [`apply`](Self::apply) would accept `command`, and if not
    /// why, without changing anything.
    fn check(&self, command: &Self::Command) -> Result<(), Self::Error>;

    /// Applies `command` and returns what happened, or refuses it and
    /// changes nothing.
    fn apply(&mut self, command: &Self::Command) -> Result<Vec<Self::Event>, Self::Error>;

    /// The commands [`check`](Self::check) accepts now, in a fixed order.
    fn legal_commands(&self) -> Vec<Self::Command>;

    /// How the game ended, once it has.
    fn outcome(&self) -> Option<Self::Outcome>;
}
