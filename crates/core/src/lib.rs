//! Shuyi Engine's core: the seeded RNG, the simulation traits and the
//! tactics kit (a square grid, paths over it, turns and their history,
//! and bots that play a game to its end, among them a rollout bot that
//! plans on copies of the game: [`Determinize`], [`Evaluate`], [`Order`],
//! [`Unordered`], [`Playout`], [`Rollout`], [`RolloutSettings`],
//! [`RolloutSearch`] and [`Horizon`], a turn planner that plans its
//! side's whole turn on them: [`TurnPlan`], [`PlanSettings`] and
//! [`PlanSearch`], a playtest runner that checks how often each of
//! several bots succeeds, [`bots::playtest`], and an autobalance search
//! whose [`Knob`]s change a stage until each lands in its band:
//! [`autobalance`], [`BalanceSettings`], [`Balanced`] and [`Stop`]), a
//! hash of a state that replays compare, the version every [`save`]
//! names, and what text and stories need below the assets: character ids
//! and the `{token}` splitter.
//!
//! Pure and deterministic: no I/O, no clock, no platform. A game's rules
//! crate sits on it and keeps its own rules, turn loop and data (game ADR-0063
//! §2, §6).
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
// No floats in a simulation (ADR-0002 §2, ADR-0032); tests may use them.
#![cfg_attr(not(test), deny(clippy::float_arithmetic))]

extern crate alloc;

pub mod bots;
pub mod character;
pub mod grid;
pub mod hash;
pub mod history;
pub mod path;
pub mod rng;
pub mod save;
pub mod text;
pub mod turn;

pub use bots::{
    BalanceSettings, Balanced, Bot, Determinize, Evaluate, Horizon, Knob, Order, PlanSearch,
    PlanSettings, PlayError, Playout, RandomBot, Report, Rollout, RolloutSearch, RolloutSettings,
    Sides, Stop, TurnPlan, Unordered, autobalance, play, play_observed,
};
pub use character::CharacterId;
pub use grid::{Dir, Grid, GridSizeError, Pos, TerrainId, TileSet};
pub use hash::{HashError, StateHash, state_hash};
pub use history::{History, NoSuchPoint, Replayed};
pub use path::{Reach, flow_field, reachable};
pub use rng::{RandomSource, ScriptedRng, SimRng};
pub use save::{SaveFormat, SaveHeader};
pub use text::{Part, split_tokens};
pub use turn::TurnRules;
