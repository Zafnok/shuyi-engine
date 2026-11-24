//! The table `build.rs` wrote of `files/`, and what it left out.

/// Every file of `files/` but its notes, sorted by path.
pub static FILES: &[(&str, &[u8])] = shuyi_embed::files!("files");

/// The notes `build.rs` left out.
pub static LEFT_OUT: &[&str] = shuyi_embed::skipped!("files");
