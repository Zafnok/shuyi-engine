//! Tools over a repository's other worktrees' build output: [`seed`]
//! starts a new worktree's `target/` from a warm one (ticket 2078), and
//! [`trim`] frees the disk the idle ones' `target/` takes, never removing
//! a worktree (ticket 2076).
//!
//! They act on folders outside the project, so they are commands of the
//! repository's tools (`cargo xtask seed-target` and `trim-targets`, and a
//! game's `cargo shuyi`, the private module `command`: ticket 3403), never
//! operations of the tool contract or tools of the MCP server.

pub(crate) mod command;
pub mod seed;
pub mod trim;
