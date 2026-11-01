//! Public synthetic games for the permissioned cloning pilot.
//!
//! Each game is a small, deterministic [`shuyi_core::TurnRules`] state.
//! The binary in this package exposes a line-oriented oracle and a batch
//! replay, while the study keeps its validation seeds and cases separate.

pub mod courier;
pub mod workshop;
