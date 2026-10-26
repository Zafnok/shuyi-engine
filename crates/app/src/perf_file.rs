//! The trace file of a running desktop build (ADR-0046 §7): with
//! `SHUYI_PERF=<path>` in a build with debug tools, each frame's record
//! is written to the file as a JSON line, a [`PerfLine::Frame`], and
//! flushed as the frame ends, so an agent reads the file while it grows.
//! Where the heap is counted by tag, it also takes a heap snapshot on its
//! first frame and every [`SNAPSHOT_EVERY`] frames after, each a
//! [`PerfLine::Snapshot`] and, after the first, the [`PerfLine::Diff`]
//! from the one before, so the file shows what grows as a person plays.
//! A file that can't be created or written is one warning in the log, and
//! the game goes on.

use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::PathBuf;

use shuyi_ui::perf::{FrameRecord, HeapSnapshot, PerfLine};

/// The variable that names the trace file.
pub(crate) const VAR: &str = "SHUYI_PERF";

/// How many frames apart the trace takes its heap snapshots: ten seconds
/// at 60 a second.
pub(crate) const SNAPSHOT_EVERY: u32 = 600;

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
    snapshots: Snapshots,
}

/// When the trace takes its heap snapshots, and the last it took, to diff
/// the next from.
#[derive(Debug, Default)]
pub(crate) struct Snapshots {
    /// The frame the next is due on: the first frame written, then
    /// [`SNAPSHOT_EVERY`] after the last taken.
    due: u32,
    last: Option<HeapSnapshot>,
}

impl Snapshots {
    /// The lines after `frame`'s records: if a snapshot is due, the one
    /// `take` gives, and its diff from the last. A frame that gives none
    /// (the heap isn't counted by tag) leaves it due.
    fn lines(&mut self, frame: u32, take: impl FnOnce() -> Option<HeapSnapshot>) -> Vec<PerfLine> {
        if frame < self.due {
            return Vec::new();
        }
        let Some(snapshot) = take() else {
            return Vec::new();
        };
        self.due = snapshot.frame.saturating_add(SNAPSHOT_EVERY);
        let diff = self.last.as_ref().map(|last| last.diff(&snapshot));
        self.last = Some(snapshot.clone());
        let mut lines = vec![PerfLine::Snapshot(snapshot)];
        lines.extend(diff.map(PerfLine::Diff));
        lines
    }
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
                    snapshots: Snapshots::default(),
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
    /// Writes `records`, then a heap snapshot (`take`'s) if one is due
    /// after `frame`, and flushes, or logs why it couldn't and gives itself
    /// up: a file that can't be written stops the trace, not the game.
    pub(crate) fn write(
        mut self,
        records: &[FrameRecord],
        frame: u32,
        take: impl FnOnce() -> Option<HeapSnapshot>,
    ) -> Option<Self> {
        let lines = self.snapshots.lines(frame, take);
        match write_lines(&mut self.out, records, &lines) {
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

/// `records` written to `out` as trace lines, oldest first, then `lines`,
/// then flushed.
pub(crate) fn write_lines(
    out: &mut impl Write,
    records: &[FrameRecord],
    lines: &[PerfLine],
) -> io::Result<()> {
    let frames = records.iter().map(|r| PerfLine::Frame(r.clone()));
    for line in frames.chain(lines.iter().cloned()) {
        serde_json::to_writer(&mut *out, &line)?;
        out.write_all(b"\n")?;
    }
    out.flush()
}

#[cfg(test)]
mod tests {
    use shuyi_ui::perf::{SpanEvent, TagClass, TagRow};

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
        write_lines(&mut out, &records, &[]).unwrap();
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
        write_lines(&mut out, &[], &[]).unwrap();
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
        let error = write_lines(&mut Refusing, &[FrameRecord::new(1)], &[]).unwrap_err();
        assert_eq!(error.to_string(), "the disk is full");
    }

    /// A snapshot of `frame` whose `update` tag holds 64 bytes a frame.
    fn snapshot_of(frame: u32) -> HeapSnapshot {
        let bytes = 64 * u64::from(frame);
        let rows = vec![TagRow::new(TagClass::Phase, "update", bytes, frame)];
        let mut snapshot = HeapSnapshot::new(bytes, bytes, rows);
        snapshot.frame = frame;
        snapshot
    }

    #[test]
    fn the_writer_takes_a_snapshot_on_the_first_frame_then_every_600_with_its_diff() {
        let mut snapshots = Snapshots::default();
        let mut out = Vec::new();
        let mut taken = 0;
        for frame in 1..=1201 {
            let lines = snapshots.lines(frame, || {
                taken += 1;
                Some(snapshot_of(frame))
            });
            write_lines(&mut out, &[FrameRecord::new(frame)], &lines).unwrap();
        }
        assert_eq!(taken, 3, "a snapshot is taken only when due");
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<PerfLine> = text
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(lines.len(), 1201 + 3 + 2);
        let heap: Vec<String> = lines
            .iter()
            .filter_map(|line| match line {
                PerfLine::Frame(_) => None,
                PerfLine::Snapshot(s) => Some(format!("snapshot {}", s.frame)),
                PerfLine::Diff(d) => Some(format!("diff {} {} {}", d.from, d.to, d.live_bytes)),
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(
            heap,
            [
                "snapshot 1",
                "snapshot 601",
                "diff 1 601 38400",
                "snapshot 1201",
                "diff 601 1201 38400",
            ]
        );
        // Each snapshot follows its frame's record.
        assert_eq!(lines[1], PerfLine::Snapshot(snapshot_of(1)));
    }

    #[test]
    fn a_frame_without_a_snapshot_leaves_it_due() {
        let mut snapshots = Snapshots::default();
        let none: Vec<PerfLine> = Vec::new();
        assert_eq!(snapshots.lines(1, || None), none);
        assert_eq!(snapshots.lines(2, || None), none);
        let lines = snapshots.lines(3, || Some(snapshot_of(3)));
        assert_eq!(lines, [PerfLine::Snapshot(snapshot_of(3))]);
        assert_eq!(snapshots.lines(602, || panic!("not due")), none);
        assert_eq!(snapshots.lines(603, || Some(snapshot_of(603))).len(), 2);
    }
}
