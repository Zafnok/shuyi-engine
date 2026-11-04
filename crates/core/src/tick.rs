//! Rules that step in time (ADR-0003 §2): a game whose rules run 60 fixed
//! steps a second implements [`TickSim`]. Each step takes a [`TickInput`],
//! what each seat's player pressed, held and released in it, and a run is
//! its seed and its inputs, a [`TickRecord`], which [`replay`] plays again,
//! giving the state's hash after each step.
//!
//! The rules time things in steps with [`timer`](crate::timer), never with
//! a frame's seconds, so a run steps the same at any frame rate.
//! `shuyi_ui::tick::TickDriver` steps a simulation as many times as a frame
//! has steps due, and gives each step what the player did since the last
//! one, so a record of the steps replays the same whatever frame rate made
//! it.
//!
//! ```
//! use shuyi_core::tick::{ActionId, SeatInput, TickInput, TickSim};
//!
//! /// A counter of the steps its one action was held.
//! #[derive(Default)]
//! struct Held(u32);
//!
//! impl TickSim for Held {
//!     type Event = ();
//!
//!     fn step(&mut self, input: &TickInput) -> Vec<()> {
//!         self.0 += u32::from(input.is_held(0, ActionId(0)));
//!         Vec::new()
//!     }
//!
//!     fn hash(&self) -> u64 {
//!         u64::from(self.0)
//!     }
//! }
//!
//! let mut sim = Held::default();
//! let held = TickInput::one(SeatInput::new(vec![], vec![ActionId(0)], vec![]));
//! sim.step(&held);
//! sim.step(&TickInput::default());
//! assert_eq!(sim.hash(), 1);
//! ```

use alloc::vec::Vec;

use serde::{Deserialize, Serialize};

use crate::made::{Made, Mismatch};

/// The most seats a [`TickInput`] holds: four players at one machine.
pub const MAX_SEATS: usize = 4;

/// An action, by its place in the game's list of actions (`ActionSet::ALL`
/// in `shuyi-assets`): 0 is the first. The core names no game's actions,
/// so a step's input holds these; `shuyi_ui::tick::ActionIds` turns a
/// game's action into its id and back.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct ActionId(pub u16);

/// What one seat's player did in one step: the actions pressed, held and
/// released, each by its [`ActionId`] and at most once in each list.
///
/// - `pressed`: the actions that went down since the last step, in the
///   order they went down.
/// - `held`: the actions down at the end of the frame the step ran in, in
///   the game's order. A tap (down and up again between two steps) is
///   pressed without being held.
/// - `released`: the actions that came up, in the order they came up. A
///   release is always in a step after its press's, so a step that has an
///   action both released and pressed let it go and pressed it again.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
// What a step's input holds may grow (axes and the pointer, ticket 1121;
// ADR-0039).
#[non_exhaustive]
pub struct SeatInput {
    /// The actions that went down since the last step, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pressed: Vec<ActionId>,
    /// The actions down at the end of the frame the step ran in.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub held: Vec<ActionId>,
    /// The actions that came up since the last step, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub released: Vec<ActionId>,
}

impl SeatInput {
    /// A seat's input with these `pressed`, `held` and `released` actions.
    pub fn new(pressed: Vec<ActionId>, held: Vec<ActionId>, released: Vec<ActionId>) -> Self {
        Self {
            pressed,
            held,
            released,
        }
    }

    /// Whether `action` went down since the last step.
    pub fn is_pressed(&self, action: ActionId) -> bool {
        self.pressed.contains(&action)
    }

    /// Whether `action` is down.
    pub fn is_held(&self, action: ActionId) -> bool {
        self.held.contains(&action)
    }

    /// Whether `action` came up since the last step.
    pub fn is_released(&self, action: ActionId) -> bool {
        self.released.contains(&action)
    }

    /// Whether nothing was pressed, held or released.
    pub fn is_empty(&self) -> bool {
        self.pressed.is_empty() && self.held.is_empty() && self.released.is_empty()
    }
}

/// What one step of a [`TickSim`] is given: one [`SeatInput`] per seat,
/// indexed by seat number from 0, at most [`MAX_SEATS`]. A game with one
/// player has seat 0 alone.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
// It may grow (ADR-0039): built with `new` or `one`.
#[non_exhaustive]
pub struct TickInput {
    /// Each seat's input, by seat number.
    pub seats: Vec<SeatInput>,
}

impl TickInput {
    /// The input of `seats`, by seat number from 0: the first
    /// [`MAX_SEATS`] of them.
    pub fn new(mut seats: Vec<SeatInput>) -> Self {
        seats.truncate(MAX_SEATS);
        Self { seats }
    }

    /// The input of seat 0 alone, a game with one player's.
    pub fn one(seat: SeatInput) -> Self {
        Self {
            seats: [seat].into(),
        }
    }

    /// Seat `seat`'s input, if it has a seat that far.
    pub fn seat(&self, seat: usize) -> Option<&SeatInput> {
        self.seats.get(seat)
    }

    /// Whether seat `seat` pressed `action` since the last step.
    pub fn is_pressed(&self, seat: usize, action: ActionId) -> bool {
        self.seat(seat).is_some_and(|s| s.is_pressed(action))
    }

    /// Whether seat `seat` holds `action`.
    pub fn is_held(&self, seat: usize, action: ActionId) -> bool {
        self.seat(seat).is_some_and(|s| s.is_held(action))
    }

    /// Whether seat `seat` let go of `action` since the last step.
    pub fn is_released(&self, seat: usize, action: ActionId) -> bool {
        self.seat(seat).is_some_and(|s| s.is_released(action))
    }
}

/// A game's rules that step in time: 60 fixed steps a second
/// ([`STEPS_PER_SECOND`](crate::STEPS_PER_SECOND)), each given the players'
/// input for it.
///
/// # Contract
///
/// - **Deterministic.** The same state and input always give the same
///   events and the same new state, on every platform. Luck comes from a
///   seeded RNG kept in the state ([`SimRng`](crate::SimRng)); there is no
///   clock, no I/O and no float in the state or the step. So a run is
///   rebuilt from its seed and its inputs ([`TickRecord`], [`replay`]).
/// - **Time is steps.** One call of [`step`](Self::step) is one step.
///   Timers advance by 1 in it ([`timer`](crate::timer)).
/// - **Events are the only way out.** What a screen shows or plays (a
///   sound, say) comes from the events [`step`](Self::step) returns, so a
///   step run again after a rewind plays nothing twice
///   (`shuyi_ui::tick::TickDriver` hands each step's events on once).
/// - [`hash`](Self::hash) changes when the state does and is the same on
///   every platform: most simply `state_hash(self)`'s
///   ([`state_hash`](crate::state_hash)).
pub trait TickSim {
    /// What happened in a step.
    type Event;

    /// Runs one step with `input`, and says what happened.
    fn step(&mut self, input: &TickInput) -> Vec<Self::Event>;

    /// The hash of the state, which a replay compares step by step.
    fn hash(&self) -> u64;
}

/// One run of a [`TickRecord`]: an input and the steps in a row that saw
/// it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    /// How many steps in a row saw it: at least 1.
    pub steps: u32,
    /// What they saw.
    pub input: TickInput,
}

/// A run of a [`TickSim`]: the stamp of the build that made it
/// ([`Made`]), the seed its state was built from, and the input of each
/// step, oldest first. Steps in a row with the same input are written once
/// with their count, so a minute of a dot held right is one line.
///
/// It holds what each step saw, after the driver gave each press and
/// release to its step, so it replays the same steps whatever frame rate
/// made it. Written as `(made: …, seed: …, runs: [(steps: 30, input:
/// (seats: [(held: [3])])), …])`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
// Its runs stay merged, and it may grow (ADR-0039): built with `new` and
// `push`.
#[non_exhaustive]
pub struct TickRecord {
    /// The build that made it.
    made: Made,
    /// The seed the run's first state was built from.
    seed: u64,
    /// The steps' inputs, run-length encoded, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    runs: Vec<Run>,
}

impl TickRecord {
    /// A record of no step yet, of a run made by `made` from `seed`.
    pub fn new(made: Made, seed: u64) -> Self {
        Self {
            made,
            seed,
            runs: Vec::new(),
        }
    }

    /// The build that made it.
    pub fn made(&self) -> &Made {
        &self.made
    }

    /// The seed the run's first state was built from.
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// The steps' inputs as written: each with how many steps in a row saw
    /// it.
    pub fn runs(&self) -> &[Run] {
        &self.runs
    }

    /// Records the next step's input.
    pub fn push(&mut self, input: TickInput) {
        match self.runs.last_mut() {
            Some(last) if last.input == input && last.steps < u32::MAX => last.steps += 1,
            _ => self.runs.push(Run { steps: 1, input }),
        }
    }

    /// How many steps it holds.
    pub fn len(&self) -> u64 {
        self.runs.iter().map(|run| u64::from(run.steps)).sum()
    }

    /// Whether it holds no step.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Each step's input, oldest first.
    pub fn inputs(&self) -> impl Iterator<Item = &TickInput> {
        self.runs.iter().flat_map(|run| {
            let steps = usize::try_from(run.steps).unwrap_or(usize::MAX);
            core::iter::repeat_n(&run.input, steps)
        })
    }

    /// Keeps its first `steps` steps and forgets the rest: after a rewind
    /// to step `steps`, the steps after it are recorded again as they run.
    pub fn truncate(&mut self, steps: u64) {
        let mut left = steps;
        let mut keep = 0;
        for run in &mut self.runs {
            if left == 0 {
                break;
            }
            let kept = u32::try_from(left).unwrap_or(u32::MAX).min(run.steps);
            run.steps = kept;
            left -= u64::from(kept);
            keep += 1;
        }
        self.runs.truncate(keep);
    }
}

/// Plays `record` on `sim`, the state the caller built from the record's
/// [`seed`](TickRecord::seed), and gives the state's hash after each step,
/// one per step of the record, oldest first. `sim` is left at the record's
/// end. A record whose stamp isn't `made`, this build's, is refused before
/// anything steps, naming each part that differs ([`Made::check`]): another
/// build may step the same inputs differently without a bug in either.
pub fn replay<S: TickSim>(
    sim: &mut S,
    record: &TickRecord,
    made: &Made,
) -> Result<Vec<u64>, Mismatch> {
    record.made.check(made)?;
    Ok(record
        .inputs()
        .map(|input| {
            sim.step(input);
            sim.hash()
        })
        .collect())
}

#[cfg(test)]
mod tests;
