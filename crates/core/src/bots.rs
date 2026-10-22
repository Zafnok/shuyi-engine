//! Playtest bots over any [`TurnRules`] game: players made of code, the
//! loop that lets them play a game to its end, a seeded random bot, two
//! bots that plan on copies of the game (over [`determinize`]): one that
//! weighs its next command alone ([`rollout`]) and one that plans its
//! side's whole turn, several commands deep ([`plan`]), and a playtest
//! runner that lets several bots play a stage many times and checks how
//! often each succeeds ([`playtest`]).
//!
//! A game's own bots (a baseline, personas) implement [`Bot`] for its
//! state, and its measures watch the loop through [`play_observed`]. Bots
//! never rewind: the loop keeps no history. Like the rest of the core, a
//! bot is pure: the same seed and the same states give the same commands.

use alloc::vec::Vec;
use core::fmt;

use crate::rng::SimRng;
use crate::turn::TurnRules;

pub mod determinize;
pub mod plan;
pub mod playtest;
pub mod rollout;

pub use determinize::{Determinize, Evaluate};
pub use plan::{PlanSearch, PlanSettings, TurnPlan};
pub use rollout::{Horizon, Order, Playout, Rollout, RolloutSearch, RolloutSettings, Unordered};

/// A game whose state says whose move it is and which turn it is, so a
/// loop can ask the right bot for each command.
pub trait Sides: TurnRules {
    /// Who moves: a player, a faction, a phase.
    type Side: Copy + Eq + fmt::Debug;

    /// Who issues the next command.
    fn side_to_move(&self) -> Self::Side;

    /// The turn being played, counting from the game's first.
    fn turn(&self) -> u32;
}

/// A player made of code. [`play`] asks it for every command of the sides
/// it plays.
///
/// A bot must never learn what the game hides from its players (the luck
/// to come, above all): any randomness of its own comes from a seed it
/// was built with. The same seed and the same states must give the same
/// commands.
pub trait Bot<R: TurnRules> {
    /// The next command in `rules`, which is not over and has this bot's
    /// side to move. It should be one [`TurnRules::check`] accepts; a
    /// refused command ends the play with a [`PlayError`].
    fn choose(&mut self, rules: &R) -> R::Command;
}

/// A function of the state is a bot (one with no seed, or a seed of its
/// own captured).
impl<R: TurnRules, F: FnMut(&R) -> R::Command> Bot<R> for F {
    fn choose(&mut self, rules: &R) -> R::Command {
        self(rules)
    }
}

/// How a play went.
pub struct Report<R: TurnRules> {
    /// How the game ended; `None` if the turn limit stopped it first.
    pub outcome: Option<R::Outcome>,
    /// The turn the game was on when the play stopped: past the limit if
    /// the limit stopped it.
    pub turns: u32,
    /// Every command applied, in order.
    pub commands: Vec<R::Command>,
}

/// A command the game refused during a play: a bug in a bot, never
/// skipped.
pub struct PlayError<R: Sides> {
    /// The refused command.
    pub command: R::Command,
    /// The side that issued it.
    pub side: R::Side,
    /// The turn it was issued on.
    pub turn: u32,
    /// Why it was refused.
    pub error: R::Error,
}

// By hand rather than derived: a derive would ask the game's state itself
// to be `Clone`, `Debug` and so on, where only its commands and outcomes
// need to be.

impl<R: TurnRules> fmt::Debug for Report<R>
where
    R::Outcome: fmt::Debug,
    R::Command: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Report")
            .field("outcome", &self.outcome)
            .field("turns", &self.turns)
            .field("commands", &self.commands)
            .finish()
    }
}

impl<R: TurnRules> Clone for Report<R>
where
    R::Outcome: Clone,
    R::Command: Clone,
{
    fn clone(&self) -> Self {
        Self {
            outcome: self.outcome.clone(),
            turns: self.turns,
            commands: self.commands.clone(),
        }
    }
}

impl<R: TurnRules> PartialEq for Report<R>
where
    R::Outcome: PartialEq,
    R::Command: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.outcome == other.outcome
            && self.turns == other.turns
            && self.commands == other.commands
    }
}

impl<R: Sides> fmt::Debug for PlayError<R>
where
    R::Command: fmt::Debug,
    R::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlayError")
            .field("command", &self.command)
            .field("side", &self.side)
            .field("turn", &self.turn)
            .field("error", &self.error)
            .finish()
    }
}

impl<R: Sides> fmt::Display for PlayError<R>
where
    R::Command: fmt::Debug,
    R::Error: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "turn {}, {:?} to move: {:?} refused: {}",
            self.turn, self.side, self.command, self.error
        )
    }
}

/// Plays `rules` until the game ends or turn `limit` has passed, asking
/// the bot of the side to move for each command. `bots_by_side` gives one
/// bot per side; a side with no bot there is a mistake in the caller and
/// panics.
///
/// A play stopped by the limit has no outcome, and its
/// [`turns`](Report::turns) is the turn that was starting.
pub fn play<R: Sides>(
    rules: &mut R,
    bots_by_side: &mut [(R::Side, &mut dyn Bot<R>)],
    limit: u32,
) -> Result<Report<R>, PlayError<R>>
where
    R::Command: Clone,
{
    play_observed(rules, bots_by_side, limit, |_, _, _| {})
}

/// [`play`], telling `observe` about every applied command: the game
/// after it, the command and the events it gave. A game's measures are
/// built this way.
pub fn play_observed<R: Sides>(
    rules: &mut R,
    bots_by_side: &mut [(R::Side, &mut dyn Bot<R>)],
    limit: u32,
    mut observe: impl FnMut(&R, &R::Command, &[R::Event]),
) -> Result<Report<R>, PlayError<R>>
where
    R::Command: Clone,
{
    let mut commands = Vec::new();
    while rules.outcome().is_none() && rules.turn() <= limit {
        let side = rules.side_to_move();
        let Some((_, bot)) = bots_by_side.iter_mut().find(|(s, _)| *s == side) else {
            panic!("no bot plays side {side:?}");
        };
        let command = bot.choose(rules);
        match rules.apply(&command) {
            Ok(events) => {
                observe(rules, &command, &events);
                commands.push(command);
            }
            Err(error) => {
                return Err(PlayError {
                    command,
                    side,
                    turn: rules.turn(),
                    error,
                });
            }
        }
    }
    Ok(Report {
        outcome: rules.outcome(),
        turns: rules.turn(),
        commands,
    })
}

/// A bot that picks one of [`TurnRules::legal_commands`] at random, from
/// its own seeded RNG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RandomBot {
    rng: SimRng,
}

impl RandomBot {
    /// A random bot whose choices follow from `seed`.
    pub fn new(seed: u64) -> Self {
        Self {
            rng: SimRng::new(seed),
        }
    }
}

impl<R: TurnRules> Bot<R> for RandomBot {
    /// One of the legal commands, each as likely as the others.
    ///
    /// # Panics
    ///
    /// If the game lists no legal command though it is not over.
    fn choose(&mut self, rules: &R) -> R::Command {
        let mut legal = rules.legal_commands();
        assert!(!legal.is_empty(), "no legal command to choose from");
        let count = u32::try_from(legal.len()).unwrap_or(u32::MAX);
        let pick = usize::try_from(self.rng.roll_below(count)).unwrap_or(0);
        legal.swap_remove(pick)
    }
}

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;
