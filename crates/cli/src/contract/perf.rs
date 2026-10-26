//! Perf traces as the contract hands them to agents (ADR-0046 §7): the
//! frames' records a session kept, read as a stream ([`PerfRead`]), summed
//! up ([`summary`]), and written as JSON lines ([`lines`]).
//!
//! `run --perf` answers a [`PerfSummary`], and with `--perf-out` writes
//! every kept frame as a line; the MCP server's `perf` tool answers the
//! records since a number.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_ui::perf::{FrameRecord, PERF_FRAMES, PerfLine, PerfRing, Spread, spread};

/// How many of the spans with the most time a [`PerfSummary`] names.
pub const TOP_SPANS: usize = 5;

/// The limit every answer with perf has: what its times are.
pub const TIMES: &str = "times are this machine's, from an unoptimized build unless it says \
                         otherwise: compare runs on the same machine";

/// The limit of a read whose `since` is past its `next`.
pub const RESET: &str = "the session was reset or started again: read from 0";

/// The records a session kept from a number on: a read of its stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct PerfRead {
    /// The kept records numbered `since` or more, oldest first, the first
    /// `max` of them.
    pub records: Vec<FrameRecord>,
    /// The number to read from next time: the one after the last record
    /// answered. Less than `since` when the session started again.
    pub next: u64,
    /// How many records numbered `since` or more were no longer kept.
    pub dropped: u64,
    /// Whether anything was measured.
    pub measured: bool,
}

impl PerfRead {
    /// A read of nothing, from `since`, with nothing measured: a session
    /// that doesn't measure.
    pub fn none(since: u64) -> Self {
        Self {
            records: Vec::new(),
            next: since,
            dropped: 0,
            measured: false,
        }
    }

    /// The first `max` records of `ring` numbered `since` or more. A
    /// `since` past the ring's next number (a ring of a game started again)
    /// reads nothing, with `next` 0.
    pub fn of(ring: &PerfRing, since: u64, max: u32) -> Self {
        let next_seq = ring.next_seq();
        let kept = u64::try_from(ring.records().len()).unwrap_or(u64::MAX);
        let first = next_seq.saturating_sub(kept);
        let measured = next_seq > 0;
        if since > next_seq {
            return Self {
                records: Vec::new(),
                next: 0,
                dropped: 0,
                measured,
            };
        }
        let max = usize::try_from(max).unwrap_or(usize::MAX);
        let records: Vec<FrameRecord> = ring.since(since).take(max).cloned().collect();
        let start = since.max(first);
        Self {
            next: start + u64::try_from(records.len()).unwrap_or(u64::MAX),
            dropped: first.saturating_sub(since),
            records,
            measured,
        }
    }
}

/// A span's name and how its time adds up over the records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct SpanSummary {
    /// What the game called it (`ctx.span("ai", …)`).
    pub name: String,
    /// How many times it ended.
    pub count: u32,
    /// Its time in all, in microseconds.
    pub total_us: u64,
    /// Its mean time, in microseconds, rounded down.
    pub mean_us: u32,
    /// Its longest time, in microseconds.
    pub worst_us: u32,
}

/// What a trace's records add up to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct PerfSummary {
    /// The records summed up: the frames kept.
    pub frames: u32,
    /// How many earlier frames' records were no longer kept.
    pub dropped: u64,
    /// How the whole frame's time spread (`frame_us`).
    pub frame: Spread,
    /// How the update's spread (`update_us`).
    pub update: Spread,
    /// How the drawing's spread (`draw_us`).
    pub draw: Spread,
    /// How painting into the window spread (`paint_us`).
    pub paint: Spread,
    /// How the backend's swap and wait spread (`present_us`).
    pub present: Spread,
    /// The spans with the most time in all, most first (ties by name), at
    /// most five.
    pub spans: Vec<SpanSummary>,
    /// The trace file written, its absolute path, with `--perf-out`.
    pub file: Option<String>,
}

/// What `records` add up to, or `None` for no records; nothing dropped and
/// no file.
pub fn summary(records: &[FrameRecord]) -> Option<PerfSummary> {
    let of = |part: fn(&FrameRecord) -> u32| spread(records.iter().map(part));
    let mut spans: BTreeMap<&str, (u32, u64, u32)> = BTreeMap::new();
    for span in records.iter().flat_map(|r| &r.spans) {
        let (count, total, worst) = spans.entry(&span.name).or_default();
        *count = count.saturating_add(1);
        *total += u64::from(span.us);
        *worst = (*worst).max(span.us);
    }
    let mut spans: Vec<SpanSummary> = spans
        .into_iter()
        .map(|(name, (count, total_us, worst_us))| SpanSummary {
            name: name.to_owned(),
            count,
            total_us,
            mean_us: u32::try_from(total_us / u64::from(count.max(1))).unwrap_or(u32::MAX),
            worst_us,
        })
        .collect();
    // A stable sort of spans in name order: ties stay by name.
    spans.sort_by_key(|span| std::cmp::Reverse(span.total_us));
    spans.truncate(TOP_SPANS);
    Some(PerfSummary {
        frames: u32::try_from(records.len()).unwrap_or(u32::MAX),
        dropped: 0,
        frame: of(|r| r.times.frame_us)?,
        update: of(|r| r.times.update_us)?,
        draw: of(|r| r.times.draw_us)?,
        paint: of(|r| r.times.paint_us)?,
        present: of(|r| r.times.present_us)?,
        spans,
        file: None,
    })
}

/// `records` as a trace: a [`PerfLine::Frame`] each, as JSON, a line each,
/// oldest first.
pub fn lines<'a>(records: impl IntoIterator<Item = &'a FrameRecord>) -> String {
    let mut out = String::new();
    for record in records {
        // A record of numbers and names: serializing can't fail.
        let line = serde_json::to_string(&PerfLine::Frame(record.clone())).unwrap_or_default();
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// The limit of a trace that dropped `dropped` earlier frames.
pub fn dropped_limit(dropped: u64) -> String {
    format!(
        "the trace keeps the last {PERF_FRAMES} frames: {dropped} earlier frame(s) were dropped"
    )
}

/// The limit of a session that measured nothing after `frames` frames.
pub fn unmeasured_limit(frames: u32) -> String {
    if frames == 0 {
        "nothing was measured: no frame ran".to_owned()
    } else {
        "nothing was measured: the game's tool program was built without debug tools".to_owned()
    }
}

/// `us` microseconds as milliseconds with one decimal, rounded to the
/// nearest tenth, in integers: `1250` is `1.3`.
pub fn millis(us: u32) -> String {
    let tenths = (u64::from(us) + 50) / 100;
    format!("{}.{}", tenths / 10, tenths % 10)
}

/// `summary`'s line for people: `perf: 600 frames, frame p50 1.2 ms p99
/// 3.4 ms worst 8.0 ms; span ai 0.8 ms mean`, then the trace file's.
pub fn text(summary: &PerfSummary) -> String {
    let frame = &summary.frame;
    let mut out = format!(
        "perf: {} frames, frame p50 {} ms p99 {} ms worst {} ms",
        summary.frames,
        millis(frame.p50),
        millis(frame.p99),
        millis(frame.worst)
    );
    if let Some(span) = summary.spans.first() {
        let _ = write!(out, "; span {} {} ms mean", span.name, millis(span.mean_us));
    }
    out.push('\n');
    if let Some(file) = &summary.file {
        let _ = writeln!(out, "trace: {file}");
    }
    out
}

#[cfg(test)]
mod tests;
