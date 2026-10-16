//! A game's history: the state it started in and every command applied
//! since. Its rules are deterministic ([`TurnRules`]), so any earlier point
//! is rebuilt by replaying, which is what replays, rewinds and saving a
//! game in progress need. What a rewind costs, if anything, is the game's.
//!
//! # Rules
//!
//! - **Points.** Point `n` is the game after its first `n` commands
//!   ([`History::state_at`]); point 0 is the state it started in.
//!   Rewinding to point `n` ([`History::rewind_to`]) forgets command `n` and
//!   every one after it.
//! - **A refused command replays as nothing**, as it changed nothing when
//!   it was sent; a history that recorded one by mistake replays as if it
//!   had not.
//! - **Saving.** A history is written as its first state and its
//!   commands: `(initial: …, commands: [ … ])`.

use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::turn::TurnRules;

/// A game's first state and the commands applied since, oldest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(bound(
    serialize = "R: Serialize, R::Command: Serialize",
    deserialize = "R: Deserialize<'de>, R::Command: Deserialize<'de>"
))]
pub struct History<R: TurnRules> {
    initial: R,
    commands: Vec<R::Command>,
}

/// One past command, replayed ([`History::replay`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replayed<R: TurnRules> {
    /// The game just before the command (the point a rewind to it goes
    /// back to).
    pub before: R,
    /// The command.
    pub command: R::Command,
    /// What it did (nothing, if the rules refused it).
    pub events: Vec<R::Event>,
}

/// Why a rewind was refused: there is no earlier point `n` (it must be
/// below the number of commands). Nothing changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoSuchPoint(pub usize);

impl fmt::Display for NoSuchPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no earlier point {} to rewind to", self.0)
    }
}

impl core::error::Error for NoSuchPoint {}

impl<R: TurnRules> History<R> {
    /// A history starting at `initial`, with no command yet.
    pub fn new(initial: R) -> Self {
        Self::from_parts(initial, Vec::new())
    }

    /// A history from its first state and the commands applied since,
    /// oldest first (as a save holds them).
    pub fn from_parts(initial: R, commands: Vec<R::Command>) -> Self {
        Self { initial, commands }
    }

    /// Its first state and its commands, oldest first.
    pub fn into_parts(self) -> (R, Vec<R::Command>) {
        (self.initial, self.commands)
    }

    /// The state the game started in (point 0).
    pub fn initial(&self) -> &R {
        &self.initial
    }

    /// The state the game started in, to change: for putting back what a
    /// save leaves out of it. A change to how its commands apply changes
    /// every later point.
    pub fn initial_mut(&mut self) -> &mut R {
        &mut self.initial
    }

    /// Records `command`, which was just applied to the game successfully.
    pub fn push(&mut self, command: R::Command) {
        self.commands.push(command);
    }

    /// The commands applied so far, oldest first.
    pub fn commands(&self) -> &[R::Command] {
        &self.commands
    }

    /// How many commands were applied (the latest point).
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether no command was applied yet.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl<R: TurnRules + Clone> History<R> {
    /// The game after its first `n` commands (all of them if `n` is past
    /// the end).
    pub fn state_at(&self, n: usize) -> R {
        let mut state = self.initial.clone();
        for command in self.commands.iter().take(n) {
            // A refused command changes nothing, as when it was first sent.
            let _ = state.apply(command);
        }
        state
    }

    /// Every command replayed in order, with the game just before it and
    /// its events: one replay for the whole list.
    pub fn replay(&self) -> Vec<Replayed<R>>
    where
        R::Command: Clone,
    {
        let mut state = self.initial.clone();
        self.commands
            .iter()
            .map(|command| {
                let before = state.clone();
                let events = state.apply(command).unwrap_or_default();
                Replayed {
                    before,
                    command: command.clone(),
                    events,
                }
            })
            .collect()
    }

    /// Goes back to point `n` (before command `n`), forgetting that command
    /// and every later one, and returns the game at that point.
    pub fn rewind_to(&mut self, n: usize) -> Result<R, NoSuchPoint> {
        if n >= self.commands.len() {
            return Err(NoSuchPoint(n));
        }
        self.commands.truncate(n);
        Ok(self.state_at(n))
    }
}

#[cfg(test)]
mod tests;
