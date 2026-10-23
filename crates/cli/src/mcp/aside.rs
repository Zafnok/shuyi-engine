//! A running server's program moved aside, so the next build can write a
//! fresh one (ticket 2545).
//!
//! Windows won't let a running program's file be removed or rewritten,
//! and cargo must do both to relink; it does let the file be renamed. So
//! on Windows `shuyi mcp` and `mcp-host` move their own program, and the
//! hard-linked twin cargo wrote first (in `deps/`, or beside an example),
//! into `<exe dir>/.shuyi-running/`, and run on from there. A copy of each,
//! with its modified time, takes its place: no program runs from the copy,
//! so the next build can replace it, and until then the program is still
//! there to run (another session's `cargo run`, a test's
//! `CARGO_BIN_EXE_shuyi`) and cargo sees it fresh.
//!
//! Only a cargo build output has a twin: a `shuyi` installed with `cargo
//! install` has none and is never moved.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// The folder, beside the program, its moved files go to.
pub const RUNNING: &str = ".shuyi-running";

/// The most files one process moves under its pid.
const NAMES: u32 = 1000;

/// The length and modified time of `path`: what a hard link shares.
fn stamp(path: &Path) -> Option<(u64, SystemTime)> {
    let meta = fs::metadata(path).ok()?;
    Some((meta.len(), meta.modified().ok()?))
}

/// `exe`'s twins: each `<exe stem>-*` with `exe`'s extension, length and
/// modified time, in `<exe dir>/deps/` (a binary) or beside it (an
/// example).
fn twins(exe: &Path, stem: &str) -> Vec<PathBuf> {
    let (Some(dir), Some(stamp_of_exe)) = (exe.parent(), stamp(exe)) else {
        return Vec::new();
    };
    let prefix = format!("{stem}-");
    let mut twins: Vec<PathBuf> = [dir.join("deps"), dir.to_path_buf()]
        .iter()
        .filter_map(|dir| fs::read_dir(dir).ok())
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension() == exe.extension()
                && path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with(&prefix))
                && stamp(path) == Some(stamp_of_exe)
        })
        .collect();
    twins.sort();
    twins
}

/// Deletes every file in `dir` that can be deleted; a program still
/// running from one refuses, and is left.
pub fn sweep(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let _ = fs::remove_file(entry.path());
    }
}

/// Moves `from` to `to`, and a copy of it, with its modified time, to
/// `from`: the copy is no running program's, so a build can replace it,
/// and until one does the program is still where cargo left it.
fn put_aside(from: &Path, to: &Path) -> Result<(), String> {
    let at = |path: &Path, e: std::io::Error| format!("{}: {e}", path.display());
    let copy = to.with_extension("copy");
    fs::copy(from, &copy).map_err(|e| at(&copy, e))?;
    let modified = fs::metadata(from)
        .and_then(|meta| meta.modified())
        .map_err(|e| at(from, e))?;
    fs::File::options()
        .write(true)
        .open(&copy)
        .and_then(|file| file.set_modified(modified))
        .map_err(|e| at(&copy, e))?;
    fs::rename(from, to).map_err(|e| at(from, e))?;
    fs::rename(&copy, from).map_err(|e| at(from, e))
}

/// Moves `exe` and its twins into `<exe dir>/.shuyi-running/`,
/// each as `<stem>-<pid>-<n>` with `exe`'s extension, after sweeping that
/// folder, and leaves a copy of each in its place: the new paths, none if
/// `exe` has no twin. Or why not.
pub fn move_aside(exe: &Path, pid: u32) -> Result<Vec<PathBuf>, String> {
    let stem = exe
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| format!("{}: no file name", exe.display()))?;
    let dir = exe
        .parent()
        .ok_or_else(|| format!("{}: no folder", exe.display()))?;
    let twins = twins(exe, stem);
    if twins.is_empty() {
        return Ok(Vec::new());
    }
    let running = dir.join(RUNNING);
    sweep(&running);
    fs::create_dir_all(&running).map_err(|e| format!("{}: {e}", running.display()))?;
    // The names not taken, in order, each looked at only when it is due.
    let ext = exe.extension().and_then(|e| e.to_str());
    let mut names = (0..NAMES)
        .map(|n| match ext {
            Some(ext) => running.join(format!("{stem}-{pid}-{n}.{ext}")),
            None => running.join(format!("{stem}-{pid}-{n}")),
        })
        .filter(|to| !to.exists());
    let mut moved = Vec::new();
    for from in std::iter::once(exe.to_path_buf()).chain(twins) {
        let to = names
            .next()
            .ok_or_else(|| format!("{}: no free name for {pid}", running.display()))?;
        put_aside(&from, &to)?;
        moved.push(to);
    }
    Ok(moved)
}

/// Moves this process's own program aside, saying on stderr if it
/// couldn't (the server serves either way).
pub fn this_program() {
    let moved = std::env::current_exe()
        .map_err(|e| e.to_string())
        .and_then(|exe| move_aside(&exe, std::process::id()));
    if let Err(e) = moved {
        eprintln!("mcp: couldn't move the program aside: {e}");
    }
}

#[cfg(test)]
mod tests;
