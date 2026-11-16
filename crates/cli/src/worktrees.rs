//! A repository's git worktrees, each with a build output of its own:
//! [`trim`] frees the disk the idle ones' `target/` takes, never removing
//! a worktree (ticket 2076).

pub mod trim;
