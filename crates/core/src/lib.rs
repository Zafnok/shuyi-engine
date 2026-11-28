//! Shuyi Engine's core: the seeded RNG, timers in steps or turns
//! ([`timer`]), ease curves and tweens drawn from the time alone, in
//! integers ([`ease`]: [`Ease`], [`Tween`], [`Lerp`]), the simulation traits (turns, [`TurnRules`], and fixed
//! steps in time, [`tick`]: [`TickSim`], its input, its records and their
//! [`replay`](tick::replay)), a toy one for the engine's tests (`toy`, with
//! the `toy` feature) and the tactics kit (a square grid, paths over it, the tiles each piece can
//! strike and the danger at a tile ([`threat`]), turns and their history,
//! and bots that play a game to its end), a hash of a state that replays
//! compare, the stamp of what made a record or a save ([`made`]), the
//! version every [`save`] names, and what text and stories need below the
//! assets: character ids and the `{token}` splitter.
//!
//! The bots ([`bots`]: [`Sides`], [`Bot`], [`RandomBot`], [`play`] and
//! [`play_observed`] at the crate's root) and their modules:
//!
//! - [`bots::determinize`]: the copy a bot that plans sees and how it
//!   scores one, [`Determinize`] and [`Evaluate`], at the root;
//! - [`bots::rollout`]: a rollout bot, [`Rollout`], [`RolloutSettings`],
//!   [`RolloutSearch`], [`Horizon`], [`Order`], [`Unordered`] and
//!   [`Playout`], at the root;
//! - [`bots::plan`]: a turn planner that plans its side's whole turn,
//!   [`TurnPlan`], [`PlanSettings`] and [`PlanSearch`], at the root;
//! - [`bots::order`]: the few candidates such a bot tries first, in
//!   ranked groups, [`ranked_order`] and [`Ranked`], at the root (for grid
//!   games, [`TacticsOrder`] and [`OrderHooks`] in [`threat`]);
//! - [`bots::weigh`]: weighted features and personas as data,
//!   [`Weights`], [`Weighted`], [`Persona`] and [`UnknownFeature`], at the
//!   root;
//! - [`bots::playtest`]: a playtest runner that checks how often each of
//!   several bots succeeds; its items stay in [`bots::playtest`]
//!   (`run` and `judge` are too plain a name for the root);
//! - [`bots::balance`]: an autobalance search whose [`Knob`]s change a
//!   stage until each bot lands in its band, [`autobalance`],
//!   [`autobalance_by`], [`BalanceSettings`], [`Balanced`] and [`Stop`],
//!   at the root.
//!
//! The level solver ([`search`]: [`solve`], a [`Solver`] stepped in
//! slices, its [`Limits`], [`Solve`] and [`SolveError`], at the root): a
//! breadth-first search over any [`TurnRules`] game's states, which proves
//! a level can be won and gives its shortest line.
//!
//! Pure and deterministic: no I/O, no clock, no platform. A game's rules
//! crate sits on it and keeps its own rules, turn loop and data.
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
// No floats in a simulation (ADR-0002 §2, ADR-0032); tests may use them.
#![cfg_attr(not(test), deny(clippy::float_arithmetic))]

extern crate alloc;

pub mod bots;
pub mod character;
pub mod ease;
pub mod grid;
pub mod hash;
pub mod history;
pub mod made;
pub mod path;
pub mod rng;
pub mod save;
pub mod search;
pub mod text;
pub mod threat;
pub mod tick;
pub mod timer;
#[cfg(any(test, feature = "toy"))]
pub mod toy;
pub mod turn;

pub use bots::{
    BalanceSettings, Balanced, Bot, Determinize, Evaluate, Horizon, Knob, Order, Persona,
    PlanSearch, PlanSettings, PlayError, Playout, RandomBot, Ranked, Report, Rollout,
    RolloutSearch, RolloutSettings, Sides, Stop, TurnPlan, UnknownFeature, Unordered, Weighted,
    Weights, autobalance, autobalance_by, play, play_observed, ranked_order,
};
pub use character::CharacterId;
pub use ease::{Ease, Easing, Family, Lerp, Tween};
pub use grid::{Dir, Grid, GridSizeError, Pos, TerrainId, TileSet};
pub use hash::{HashError, StateHash, state_hash};
pub use history::{History, NoSuchPoint, Replayed};
pub use made::{ContentHash, Made};
pub use path::{Reach, flow_field, reachable};
pub use rng::{Bag, RandomSource, ScriptedRng, SimRng};
pub use save::{SaveFormat, SaveHeader};
pub use search::{Limits, Solve, SolveError, Solver, solve};
pub use text::{Part, split_tokens};
pub use threat::{
    Danger, GoalDistance, OrderHooks, Ranges, Strike, TacticsOrder, Threat, Threats, strike_area,
    strike_area_from,
};
pub use tick::{ActionId, SeatInput, TickInput, TickRecord, TickSim};
pub use timer::{Countdown, Every, STEPS_PER_SECOND, ms_from_steps, steps_from_ms};
pub use turn::TurnRules;

/// What a game's rules crate names (ADR-0161), with `use
/// shuyi_core::prelude::*;` (a game's own crate, which depends on `shuyi`,
/// writes `shuyi::core::prelude`): the RNG, the turn and step traits, the
/// step's input, record and [`tick`] module (for [`tick::replay`]), the
/// timers, the grid's positions and paths, the ease and tween types, a
/// state's hash and a save's header, the bots that play a turn game and
/// the level solver that proves a level winnable (the playtest runner and the autobalance search are in
/// `shuyi::prelude::test`). A game's code names nothing deeper than a
/// prelude but the paths `cargo xtask check-imports` allows
/// (`tools/imports.ron`).
pub mod prelude {
    pub use crate::tick;
    pub use crate::{
        ActionId, Bag, Bot, ContentHash, Countdown, Determinize, Dir, Ease, Every, Family, Grid,
        History, Horizon, Limits, Made, PlayError, Pos, RandomBot, RandomSource, Reach, Report,
        Rollout, RolloutSettings, SaveFormat, SaveHeader, ScriptedRng, SeatInput, Sides, SimRng,
        Solve, SolveError, Solver, StateHash, TerrainId, TickInput, TickRecord, TickSim, TurnRules,
        Tween, Unordered, flow_field, play, reachable, solve, state_hash,
    };
}
