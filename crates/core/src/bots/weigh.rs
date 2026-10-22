//! Weighted features: a bot's score as the game's named counts times
//! weights from a file, and personas (how a bot thinks and what it cares
//! about) as data.
//!
//! A game declares a fixed list of feature names (`["won", "lost", "hp
//! lost"]`) and a function that counts each of them between the state a
//! decision began in and where a copy got to. [`Weights`] holds what one
//! kind of player cares about each, read by name, and its
//! [`score`](Weights::score) is the sum of count × weight, in integers
//! that saturate rather than wrap. [`Weighted`] pairs the two into an
//! [`Evaluate`] for the bots that plan. What a feature means and how it is
//! counted stay the game's.
//!
//! A [`Persona`] is one kind of player: its [`PlanSettings`], its weights
//! by name, and whatever the game adds of its own (a game mode, a
//! difficulty). A map of them by name is a personas file:
//!
//! ```ron
//! {
//!     "careful": (
//!         settings: (width: 4, candidates: 10, depth: 6, samples: 2, temperature: 0),
//!         weights: {"won": 1000, "hp lost": -4},
//!         game: (mode: Classic),
//!     ),
//! }
//! ```
//!
//! A weight naming a feature the game didn't declare is an error
//! ([`UnknownFeature`]); a feature with no weight weighs 0.

use alloc::collections::BTreeMap;
use alloc::string::String;
use core::fmt;

use serde::{Deserialize, Serialize};

use super::Sides;
use super::determinize::Evaluate;
use super::plan::PlanSettings;

/// What a kind of player cares about each of a game's `N` features, in
/// the order of the game's names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weights<const N: usize>(pub [i32; N]);

impl<const N: usize> Weights<N> {
    /// The weights of `by_name`, put in the order of `names`. A feature
    /// `by_name` leaves out weighs 0.
    ///
    /// # Errors
    ///
    /// [`UnknownFeature`] for the first name of `by_name` (in its order)
    /// that isn't one of `names`.
    pub fn from_named(
        names: &[&str; N],
        by_name: &BTreeMap<String, i32>,
    ) -> Result<Self, UnknownFeature> {
        let mut weights = [0; N];
        for (name, &weight) in by_name {
            let Some(index) = names.iter().position(|known| known == name) else {
                return Err(UnknownFeature {
                    name: name.clone(),
                    known: names.join(", "),
                });
            };
            weights[index] = weight;
        }
        Ok(Self(weights))
    }

    /// The sum of each count times its weight, clamped into `i32`: never
    /// a wrap. It allocates nothing.
    pub fn score(&self, counts: &[i64; N]) -> i32 {
        // In i128 a product (under 2^94) never overflows and a sum of any
        // real number of them neither, so a large term that a later one
        // cancels still counts in full; the adds saturate all the same.
        let sum = counts
            .iter()
            .zip(self.0)
            .fold(0_i128, |sum, (&count, weight)| {
                sum.saturating_add(i128::from(count) * i128::from(weight))
            });
        i32::try_from(sum).unwrap_or(if sum < 0 { i32::MIN } else { i32::MAX })
    }

    /// The weight of the feature `name` among `names`, or `None` when it
    /// isn't one of them.
    pub fn get(&self, names: &[&str; N], name: &str) -> Option<i32> {
        names
            .iter()
            .position(|known| *known == name)
            .map(|index| self.0[index])
    }
}

/// A weight naming a feature the game didn't declare.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownFeature {
    /// The name that isn't a feature.
    pub name: String,
    /// The game's feature names, joined with ", ".
    pub known: String,
}

impl fmt::Display for UnknownFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no feature {:?} (there: {})", self.name, self.known)
    }
}

impl core::error::Error for UnknownFeature {}

/// An [`Evaluate`] made of the game's counts and a kind of player's
/// weights: it scores `weights.score(&counts(start, end, side))`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weighted<F, const N: usize> {
    /// What the player cares about each feature.
    pub weights: Weights<N>,
    /// The game's counts of its features, between the state a decision
    /// began in and where a copy got to, for a side.
    pub counts: F,
}

impl<R, F, const N: usize> Evaluate<R> for Weighted<F, N>
where
    R: Sides,
    F: Fn(&R, &R, R::Side) -> [i64; N],
{
    fn score(&self, start: &R, end: &R, side: R::Side) -> i32 {
        self.weights.score(&(self.counts)(start, end, side))
    }
}

/// One kind of player: how it thinks, what it cares about, and the
/// game's own extras `X` (a game mode, a difficulty; `()` for none).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    deny_unknown_fields,
    bound(
        serialize = "X: Serialize",
        deserialize = "X: Default + Deserialize<'de>"
    )
)]
pub struct Persona<X> {
    /// How its planner thinks.
    pub settings: PlanSettings,
    /// What it cares about each feature, by the game's names. A feature
    /// left out weighs 0.
    #[serde(default)]
    pub weights: BTreeMap<String, i32>,
    /// The game's own extras; their default when left out.
    #[serde(default)]
    pub game: X,
}

impl<X> Persona<X> {
    /// Its weights over the game's feature `names`.
    ///
    /// # Errors
    ///
    /// [`UnknownFeature`] when a weight names a feature not in `names`.
    pub fn weights<const N: usize>(&self, names: &[&str; N]) -> Result<Weights<N>, UnknownFeature> {
        Weights::from_named(names, &self.weights)
    }
}

#[cfg(test)]
mod tests;
