//! The trace file of a running desktop build (ADR-0046 §7): with
//! `SHUYI_PERF=<path>` in a build with debug tools, each frame's record
//! is written to the file as a JSON line, a [`PerfLine::Frame`], and
//! flushed as the frame ends, so an agent reads the file while it grows.
//! A file that can't be created or written is one warning in the log, and
//! the game goes on.

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use shuyi_ui::perf::{FrameRecord, PerfLine};

/// The variable that names the trace file.
pub(crate) const VAR: &str = "SHUYI_PERF";

/// What the build does with the variable's value.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Decision {
    /// Not set, or empty: no trace.
    Off,
    /// Set in a build without debug tools: no trace, and a line says why.
    NoDebugTools,
    /// Write the trace to this file.
    Write(PathBuf),
}

/// What to do with `SHUYI_PERF`'s value `var`, in a build with debug tools
/// if `debug_tools`.
fn decide(var: Option<OsString>, debug_tools: bool) -> Decision {
    match var {
        None => Decision::Off,
        Some(path) if path.is_empty() => Decision::Off,
        Some(_) if !debug_tools => Decision::NoDebugTools,
        Some(path) => Decision::Write(PathBuf::from(path)),
    }
}

/// The trace file being written.
pub(crate) struct PerfFile {
    path: PathBuf,
    out: BufWriter<File>,
}

/// The trace file `SHUYI_PERF` names, created now, or none: not set, no
/// debug tools, or a file that can't be created (each but the first
/// logged).
pub(crate) fn start() -> Option<PerfFile> {
    match decide(std::env::var_os(VAR), shuyi_ui::DEBUG_TOOLS) {
        Decision::Off => None,
        Decision::NoDebugTools => {
            macroquad::logging::warn!(
                "shuyi perf: {} is set, but this build has no debug tools (a release build \
                 without the `debug-tools` feature), so it writes no trace",
                VAR
            );
            None
        }
        Decision::Write(path) => match File::create(&path) {
            Ok(file) => {
                macroquad::logging::info!(
                    "shuyi perf: writing each frame to {}, a JSON line each",
                    path.display()
                );
                Some(PerfFile {
                    path,
                    out: BufWriter::new(file),
                })
            }
            Err(e) => {
                macroquad::logging::warn!(
                    "shuyi perf: can't create {}: {}, so it writes no trace",
                    path.display(),
                    e
                );
                None
            }
        },
    }
}

impl PerfFile {
    /// Writes `records` and flushes, or logs why it couldn't and gives
    /// itself up: a file that can't be written stops the trace, not the
    /// game.
    pub(crate) fn write(mut self, records: &[FrameRecord]) -> Option<Self> {
        match write_lines(&mut self.out, records) {
            Ok(()) => Some(self),
            Err(e) => {
                macroquad::logging::warn!(
                    "shuyi perf: can't write {}: {}, so the trace stops here",
                    self.path.display(),
                    e
                );
                None
            }
        }
    }
}

/// `records` written to `out` as trace lines, oldest first, then flushed.
pub(crate) fn write_lines(out: &mut impl Write, records: &[FrameRecord]) -> io::Result<()> {
    for record in records {
        serde_json::to_writer(&mut *out, &PerfLine::Frame(record.clone()))?;
        out.write_all(b"\n")?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use shuyi_ui::perf::SpanEvent;

    use super::*;

    #[test]
    fn the_variable_turns_the_trace_on_in_a_build_with_debug_tools_only() {
        let path = || Some(OsString::from("perf.jsonl"));
        assert_eq!(decide(None, true), Decision::Off);
        assert_eq!(decide(Some(OsString::new()), true), Decision::Off);
        assert_eq!(decide(path(), false), Decision::NoDebugTools);
        assert_eq!(
            decide(path(), true),
            Decision::Write(PathBuf::from("perf.jsonl"))
        );
    }

    #[test]
    fn the_writer_writes_lines_that_parse_back_to_the_records() {
        let mut first = FrameRecord::new(1);
        first.times.frame_us = 16_700;
        first.spans = vec![SpanEvent::new("ai", 40, 250, 0)];
        let records = [first, FrameRecord::new(2)];
        let mut out = Vec::new();
        write_lines(&mut out, &records).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 2);
        assert!(text.ends_with('\n'));
        let back: Vec<FrameRecord> = text
            .lines()
            .map(|line| match serde_json::from_str(line).unwrap() {
                PerfLine::Frame(record) => record,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(back, records);
    }

    #[test]
    fn no_records_write_nothing() {
        let mut out = Vec::new();
        write_lines(&mut out, &[]).unwrap();
        assert_eq!(out, b"");
    }

    /// A writer that refuses every byte.
    struct Refusing;

    impl Write for Refusing {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("the disk is full"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_writer_that_refuses_is_an_error() {
        let error = write_lines(&mut Refusing, &[FrameRecord::new(1)]).unwrap_err();
        assert_eq!(error.to_string(), "the disk is full");
    }
}
