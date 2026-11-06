//! Counting a frame's instructions with Valgrind's Callgrind: the game's
//! tool program runs under it, which dumps its count (and starts counting
//! from zero again) each time a frame ends, so each dump is one frame.
//!
//! A frame ends where it reads the heap ([`MARKER`]: a measured frame
//! reads it once, at its end), and `bench-play` reads it once itself
//! before the first, so the first dump is the start. The last dump, at the
//! program's end, is left out. Callgrind counts the same instructions on
//! every run of the same program; the numbers are an x86-64 Linux
//! program's, a proxy for a device's, never its frame time.

use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::process::Command;

/// The program.
pub(super) const VALGRIND: &str = "valgrind";

/// The function whose every call ends a frame: Callgrind matches it by
/// its demangled name, and `*` stands for anything (a crate's
/// disambiguator, the symbol's hash).
pub(super) const MARKER: &str = "*contract::session::process_heap*";

/// The dumps' file name, in the run's folder; each dump adds `.<part>`.
const BASE: &str = "callgrind.out";

/// Whether Valgrind is there to run.
pub(super) fn check() -> Result<(), String> {
    match Command::new(VALGRIND).arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        _ => Err(
            "bench counts instructions with Valgrind's Callgrind, and `valgrind` doesn't run \
             here: install it (Linux: `sudo apt-get install valgrind`), or measure the heap \
             and the packages alone with --no-instructions"
                .to_owned(),
        ),
    }
}

/// Valgrind's arguments before the program, its dumps going in `dir`.
pub(super) fn args(dir: &Path) -> Vec<OsString> {
    let mut out = OsString::from("--callgrind-out-file=");
    out.push(dir.join(BASE));
    vec![
        "--tool=callgrind".into(),
        "--quiet".into(),
        out,
        format!("--dump-before={MARKER}").into(),
        "--dump-instr=no".into(),
        "--dump-line=no".into(),
        "--compress-strings=no".into(),
        "--compress-pos=no".into(),
    ]
}

/// One dump, read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Dump {
    /// Its number: dumps are numbered in the order taken.
    pub part: u32,
    /// Whether the program's end took it, not a frame's.
    pub at_exit: bool,
    /// The instructions counted since the dump before.
    pub instructions: u64,
}

/// Reads dump `text` (the file `name`).
pub(super) fn parse(name: &str, text: &str) -> Result<Dump, String> {
    let mut part = None;
    let mut at_exit = false;
    let mut totals = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("part:") {
            part = value.trim().parse().ok();
        } else if line.starts_with("desc: Trigger:") {
            at_exit = line.contains("Program termination");
        } else if let Some(value) = line
            .strip_prefix("totals:")
            .or_else(|| line.strip_prefix("summary:"))
        {
            totals = value.split_whitespace().next().and_then(|n| n.parse().ok());
        }
    }
    let instructions = totals.ok_or_else(|| format!("{name}: no instruction total"))?;
    Ok(Dump {
        part: part.unwrap_or(0),
        at_exit,
        instructions,
    })
}

/// The instructions of the dumps Callgrind wrote in `dir`, in order: the
/// start, then each frame. The program's end's dump is left out.
pub(super) fn read(dir: &Path) -> Result<Vec<u64>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("reading {}: {e}", dir.display()))?;
    let mut dumps = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.starts_with(BASE) {
            continue;
        }
        let text = fs::read_to_string(entry.path()).map_err(|e| format!("{name}: {e}"))?;
        // The file's own number orders dumps whose text has none.
        let suffix: u32 = name
            .rsplit('.')
            .next()
            .and_then(|n| n.parse().ok())
            .unwrap_or(0);
        dumps.push((parse(&name, &text)?, suffix));
    }
    dumps.sort_by_key(|(dump, suffix)| (dump.part, *suffix));
    let counts: Vec<u64> = dumps
        .iter()
        .filter(|(dump, _)| !dump.at_exit)
        .map(|(dump, _)| dump.instructions)
        .collect();
    if counts.is_empty() {
        return Err(format!(
            "Callgrind dumped nothing at a frame's end: it found no {MARKER}"
        ));
    }
    Ok(counts)
}
