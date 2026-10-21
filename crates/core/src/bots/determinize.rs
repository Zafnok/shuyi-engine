//! What a bot that plans may look at: a copy of the game as its side
//! knows it ([`Determinize`]), and how good a state is for a side
//! ([`Evaluate`]).
//!
//! A bot that plans plays moves out on copies of the game. Cloning the
//! state itself would show it the luck to come (the rolls waiting in the
//! state's [`SimRng`](crate::SimRng)) and whatever the other sides hide.
//! The bots that plan on copies take only games that implement
//! [`Determinize`], so each game decides what its bots may see
//! (ADR-0027).

use super::Sides;

/// A game that can make a copy of itself as one side knows it.
pub trait Determinize: Sides + Clone {
    /// A copy of the state as `side` knows it, with everything `side`
    /// can't see redrawn from `seed`: the luck to come (a new
    /// [`SimRng::new(seed)`](crate::SimRng::new)), and the other sides'
    /// hidden things (a hand dealt again from what is left, a piece that
    /// hasn't shown up yet removed).
    ///
    /// The copy must list the same [`legal_commands`](crate::TurnRules::legal_commands)
    /// as `self`, in the same order, and accept each of them: the
    /// [`Rollout`](crate::Rollout) bot checks the list and panics when it
    /// differs.
    ///
    /// A game with no luck and nothing hidden returns `self.clone()` and
    /// says so in a comment. A game whose difficulty lets a bot see more
    /// says so here.
    #[must_use]
    fn determinize(&self, side: Self::Side, seed: u64) -> Self;
}

/// How good a state is for a side: higher is better, in whole points.
pub trait Evaluate<R: Sides> {
    /// How good `end` is for `side`, reached from `start` (the state the
    /// decision began in). A score that only needs the end ignores
    /// `start`; one that counts what was lost on the way compares the
    /// two. A finished game scores its outcome.
    fn score(&self, start: &R, end: &R, side: R::Side) -> i32;
}

impl<R: Sides, F: Fn(&R, &R, R::Side) -> i32> Evaluate<R> for F {
    fn score(&self, start: &R, end: &R, side: R::Side) -> i32 {
        self(start, end, side)
    }
}
