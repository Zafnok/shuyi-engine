//! Shuyi Engine's core: the seeded RNG, the simulation traits and the
//! tactics kit (a square grid, paths over it, turns and their history).
//!
//! Pure and deterministic: no I/O, no clock, no platform. A game's rules
//! crate sits on it and keeps its own rules, turn loop and data (ADR-0063
//! §2, §6).
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

extern crate alloc;

pub mod grid;
pub mod history;
pub mod path;
pub mod rng;
pub mod save;
pub mod turn;

pub use grid::{Dir, Grid, GridSizeError, Pos, TileSet};
pub use history::{History, NoSuchPoint, Replayed};
pub use path::{Reach, flow_field, reachable};
pub use rng::{RandomSource, ScriptedRng, SimRng};
pub use save::{SaveFormat, SaveHeader};
pub use turn::TurnRules;
