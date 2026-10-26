//! Perf traces as the contract hands them to agents (ADR-0046 §7): the
//! frames' records a session kept, read as a stream ([`PerfRead`]), summed
//! up ([`summary`]), and written as JSON lines ([`lines`]).
//!
//! `run --perf` answers a [`PerfSummary`], and with `--perf-out` writes
//! every kept frame as a line; the MCP server's `perf` tool answers the
//! records since a number.
//!
//! In a debug build of a game's tool program each record has the heap's
//! counts too (`FrameRecord::heap`), from the program's own heap counter
//! (`shuyi_alloc_count::global`), and the summary sums them up
//! ([`HeapSummary`]). The counter counts the whole program: every thread's
//! allocations. Where it counts by memory tag, `run --perf` takes a heap
//! snapshot before its script and one after its last frame, and the
//! summary has what held the heap and what grew ([`add_heap`]); the trace
//! file ends with the two snapshots and their diff ([`heap_lines`]).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_ui::perf::{
    FrameRecord, HeapDiff, HeapSnapshot, PERF_FRAMES, PerfLine, PerfRing, Spread, TagDelta, TagRow,
    bytes_text, spread,
};

/// How many of the spans with the most time a [`PerfSummary`] names.
pub const TOP_SPANS: usize = 5;

/// How many of a snapshot's rows, and of a diff's, a [`PerfSummary`]
/// keeps.
pub const HEAP_ROWS: usize = 10;

/// How many of a diff's changes the line for people names.
pub const TEXT_CHANGES: usize = 3;

/// The limit every answer with perf has: what its times are.
pub const TIMES: &str = "times are this machine's, from an unoptimized build unless it says \
                         otherwise: compare runs on the same machine";

/// The limit of a read whose `since` is past its `next`.
pub const RESET: &str = "the session was reset or started again: read from 0";

/// The limit of records measured with no heap counts.
pub const HEAP_UNCOUNTED: &str =
    "the heap isn't counted: the game's tool program is a release build, or opted out";

/// The limit of heap counts with no snapshot by tag: the heap was counted,
/// but not by memory tag.
pub const HEAP_UNTAGGED: &str =
    "the heap isn't counted by memory tag, so no heap snapshot was taken";

/// The limit of heap counts from this program's own counter while it ran
/// another thread beside the game's.
pub const HEAP_THREADS: &str = "the heap counts the whole tool program: another thread ran beside \
                                the game, and its allocations count too";

/// Whether this program started a thread of its own beside the game's
/// (the MCP server's reader): the heap counter counts its allocations too.
static OTHER_THREADS: AtomicBool = AtomicBool::new(false);

/// Notes that this program runs a thread beside the game's, from now on.
pub fn note_other_thread() {
    OTHER_THREADS.store(true, Ordering::Relaxed);
}

/// Whether this program ran a thread beside the game's
/// ([`note_other_thread`]).
pub fn other_threads() -> bool {
    OTHER_THREADS.load(Ordering::Relaxed)
}

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

/// How the heap's counts add up over a trace's records, in bytes asked for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An answer's, read field by field: it may grow (ADR-0039).
#[non_exhaustive]
pub struct HeapSummary {
    /// The most bytes live at once, the most of the records' peaks.
    pub peak_bytes: u64,
    /// The bytes live at the last record's end.
    pub live_bytes: u64,
    /// How the allocations per frame spread.
    pub allocs: Spread,
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
    /// The heap's counts over the records that have them; null where none
    /// has (a release build of the tool program, or one opted out).
    pub heap: Option<HeapSummary>,
    /// What held the heap after the last frame: the snapshot's first ten
    /// rows by tag, the most live bytes first. Empty without a snapshot.
    pub heap_tags: Vec<TagRow>,
    /// What grew and shrank from the snapshot before the run to the one
    /// after: the diff, with its first ten rows, the largest change first.
    /// Null without snapshots.
    pub heap_growth: Option<HeapDiff>,
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
        heap: heap_summary(records),
        heap_tags: Vec::new(),
        heap_growth: None,
        file: None,
    })
}

/// Adds the heap snapshots `first` (before a run) and `last` (after it)
/// to `summary`: `last`'s first [`HEAP_ROWS`] rows, and the diff from
/// `first` to `last` with its first [`HEAP_ROWS`].
pub fn add_heap(summary: &mut PerfSummary, first: &HeapSnapshot, last: &HeapSnapshot) {
    summary.heap_tags = last.tags.iter().take(HEAP_ROWS).cloned().collect();
    let mut growth = first.diff(last);
    growth.tags.truncate(HEAP_ROWS);
    summary.heap_growth = Some(growth);
}

/// How the heap's counts add up over the records that have them, or
/// `None` for none.
pub fn heap_summary(records: &[FrameRecord]) -> Option<HeapSummary> {
    let heaps: Vec<_> = records.iter().filter_map(|r| r.heap).collect();
    let last = heaps.last()?;
    Some(HeapSummary {
        peak_bytes: heaps.iter().map(|h| h.peak_bytes).max().unwrap_or(0),
        live_bytes: last.live_bytes,
        allocs: spread(heaps.iter().map(|h| h.allocs))?,
    })
}

/// The limits of `records`' heap counts: none counted, or counted by this
/// program's own counter (`process`) while another thread ran
/// (`other_threads`).
pub fn heap_limits(records: &[FrameRecord], process: bool, other_threads: bool) -> Vec<String> {
    if records.is_empty() {
        Vec::new()
    } else if records.iter().all(|r| r.heap.is_none()) {
        vec![HEAP_UNCOUNTED.to_owned()]
    } else if process && other_threads {
        vec![HEAP_THREADS.to_owned()]
    } else {
        Vec::new()
    }
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

/// The heap snapshots `first` and `last` as a trace's last lines: a
/// [`PerfLine::Snapshot`] each, then the [`PerfLine::Diff`] between them.
pub fn heap_lines(first: &HeapSnapshot, last: &HeapSnapshot) -> String {
    let lines = [
        PerfLine::Snapshot(first.clone()),
        PerfLine::Snapshot(last.clone()),
        PerfLine::Diff(first.diff(last)),
    ];
    let mut out = String::new();
    for line in lines {
        // Numbers and names: serializing can't fail.
        out.push_str(&serde_json::to_string(&line).unwrap_or_default());
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

/// `bytes` with its sign: `+96.0 KB`, `-512 B`.
fn signed_bytes(bytes: i64) -> String {
    let sign = if bytes < 0 { '-' } else { '+' };
    format!("{sign}{}", bytes_text(bytes.unsigned_abs()))
}

/// The change `delta` for people: its name and its bytes, signed.
fn delta_text(delta: &TagDelta) -> String {
    format!("{} {}", delta.name, signed_bytes(delta.live_bytes))
}

/// `diff`'s line for people: `heap grew 120.0 KB from frame 0 to 600:
/// update +96.0 KB, image +24.0 KB, ai +4.0 KB`, its [`TEXT_CHANGES`]
/// largest changes; `heap shrank …` for fewer bytes, `heap held its size
/// …` for as many; `None` when nothing changed.
pub fn heap_text(diff: &HeapDiff) -> Option<String> {
    if diff.live_bytes == 0 && diff.tags.is_empty() {
        return None;
    }
    let size = bytes_text(diff.live_bytes.unsigned_abs());
    let mut out = match diff.live_bytes.signum() {
        1 => format!("heap grew {size}"),
        -1 => format!("heap shrank {size}"),
        _ => "heap held its size".to_owned(),
    };
    let _ = write!(out, " from frame {} to {}", diff.from, diff.to);
    let changes: Vec<String> = diff
        .tags
        .iter()
        .take(TEXT_CHANGES)
        .map(delta_text)
        .collect();
    if !changes.is_empty() {
        let _ = write!(out, ": {}", changes.join(", "));
    }
    Some(out)
}

/// `summary`'s line for people: `perf: 600 frames, frame p50 1.2 ms p99
/// 3.4 ms worst 8.0 ms; span ai 0.8 ms mean; heap peak 4.1 MB`, then what
/// grew in the heap ([`heap_text`]) and the trace file's.
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
    if let Some(heap) = &summary.heap {
        let _ = write!(out, "; heap peak {}", bytes_text(heap.peak_bytes));
    }
    out.push('\n');
    if let Some(line) = summary.heap_growth.as_ref().and_then(heap_text) {
        out.push_str(&line);
        out.push('\n');
    }
    if let Some(file) = &summary.file {
        let _ = writeln!(out, "trace: {file}");
    }
    out
}

#[cfg(test)]
mod tests;
