//! Tools over a repository's other worktrees' build output: [`seed`]
//! starts a new worktree's `target/` from a warm one (ticket 2078).
//!
//! They act on folders outside the project, so they are commands of the
//! repository's tools (`cargo xtask seed-target`, and a game's `cargo shuyi`
//! with ticket 3403), never operations of the tool contract or tools of the
//! MCP server.

pub mod seed;
