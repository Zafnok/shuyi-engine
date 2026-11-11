#![cfg(feature = "std")]
//! The files a script's steps read from the working folder: a
//! [`Step::Drop`](super::Step::Drop)'s.

use crate::alloc_prelude::*;
use alloc::format;

/// The bytes of the file at `path`; an error names it.
pub(crate) fn read(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("drop: {path}: {e}"))
}
