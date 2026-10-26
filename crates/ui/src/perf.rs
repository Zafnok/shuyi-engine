//! Frame times and spans as data (ADR-0046), in debug builds: what the
//! platform's `App` measures of each frame and keeps in a [`PerfRing`] of
//! the last [`PERF_FRAMES`] [`FrameRecord`]s, for tests, tools and agents.
//!
//! Only a backend has a clock. It hands one over ([`PerfClock`]), the
//! platform's `App` moves it into the game's context, and the context reads
//! it: around the update and the drawing, and around each span a game
//! marks. With debug tools off ([`Ctx::debug_tools`]) nothing is measured
//! and the clock is never read.
//!
//! A game marks a span of its own around any part of an update:
//!
//! ```
//! use shuyi_ui::{Ctx, GameData};
//!
//! fn plan_turn<G: GameData>(ctx: &mut Ctx<G>) -> u32 {
//!     ctx.span("ai", |_ctx| 2 + 2)
//! }
//! ```
//!
//! [`Ctx::span`] calls the closure and gives back what it returns; in a
//! measured frame it also keeps the span's start, length and depth in the
//! frame's record. A time never reaches an update: nothing reads a record
//! to decide anything, so a run plays the same with the clock on and off.
//!
//! The heap is counted too, where the backend counts it (a debug build of
//! `shuyi-app` or of a game's tool program): each record's
//! [`heap`](FrameRecord::heap) holds the live bytes, their peak and the
//! allocations in that frame ([`HeapCounts`]), as the backend's
//! [`MemoryReading`] gave them at the frame's end. The counter counts the
//! whole process, every thread's allocations.
//!
//! Where the backend's counter has memory tags ([`MemoryScopes`]), it
//! also counts what each part of a frame allocates under a tag of its own
//! ([`TagClass`]): the frame's phases, each span a game marks, and each
//! asset kind the backend sees decoded. Entering a tag changes nothing a
//! game reads. A [`HeapSnapshot`] is what holds the heap at a frame, by
//! tag, and a [`HeapDiff`] what grew and shrank between two
//! ([`HeapSnapshot::diff`]); both are lines of a trace ([`PerfLine`]).
//!
//! [`Ctx::debug_tools`]: crate::Ctx::debug_tools
//! [`Ctx::span`]: crate::Ctx::span

use crate::prelude::*;
use alloc::borrow::Cow;
use alloc::collections::VecDeque;
use alloc::collections::vec_deque;

use serde::{Deserialize, Serialize};

/// How many frames' records a [`PerfRing`] keeps by default: ten seconds
/// at 60 a second.
pub const PERF_FRAMES: usize = 600;

/// How many spans a frame's record keeps: those past it are counted in
/// [`FrameRecord::spans_dropped`].
pub const MAX_SPANS: usize = 64;

/// A clock in whole microseconds, from a backend
/// (`shuyi_platform::Platform::clock`) or a test.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum PerfClock {
    /// The device's clock: microseconds from any fixed start, never going
    /// back.
    Real(fn() -> u64),
    /// A clock that moves by fixed steps on each read, so a test's records
    /// are the same on every run ([`FakeClock`]).
    Fake(FakeClock),
}

impl PerfClock {
    /// A [`FakeClock`] at 0.
    pub fn fake() -> Self {
        Self::Fake(FakeClock::new())
    }

    /// The time now, in microseconds.
    pub fn read(&mut self) -> u64 {
        match self {
            Self::Real(now) => now(),
            Self::Fake(fake) => fake.read(),
        }
    }
}

/// A clock for tests. It starts at 0, and its read number *n* (from 0)
/// adds 100 × (*n* mod 7 + 1) microseconds and gives the new time: 100,
/// 300, 600, 1000, …
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FakeClock {
    now: u64,
    reads: u64,
}

impl FakeClock {
    /// A clock at 0, never read.
    pub const fn new() -> Self {
        Self { now: 0, reads: 0 }
    }

    /// Moves on and gives the new time.
    pub fn read(&mut self) -> u64 {
        self.now += 100 * (self.reads % 7 + 1);
        self.reads += 1;
        self.now
    }

    /// How many times it has been read.
    pub fn reads(&self) -> u64 {
        self.reads
    }
}

/// How long the parts of one frame took, in whole microseconds. Build it
/// with [`FrameTimes::new`] (ADR-0039); a backend may add a part.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct FrameTimes {
    /// `Game::frame` from its start to the drawing: the input, the
    /// modules' `before_update` and the screens' update.
    pub update_us: u32,
    /// The drawing: the screens and the modules drawing the frame
    /// ([`Game::redraw`](crate::Game::redraw)).
    pub draw_us: u32,
    /// The platform's `present`: in `shuyi-app`, painting the frame into
    /// the window.
    pub paint_us: u32,
    /// From the end of the last frame to the start of this one, outside
    /// `App`: the backend's buffer swap and its wait for vsync or the frame
    /// cap. 0 on the first frame measured.
    pub present_us: u32,
    /// [`present_us`](Self::present_us) plus this frame from its start to
    /// its end: never less than the four above together. The rest is
    /// `App`'s own (the system's events, the audio, the record).
    pub frame_us: u32,
}

impl FrameTimes {
    /// Every part 0.
    pub const fn new() -> Self {
        Self {
            update_us: 0,
            draw_us: 0,
            paint_us: 0,
            present_us: 0,
            frame_us: 0,
        }
    }
}

/// A span a game marked ([`Ctx::span`](crate::Ctx::span)), as it ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SpanEvent {
    /// What the game called it.
    pub name: Cow<'static, str>,
    /// When it started, in microseconds from the frame's start.
    pub start_us: u32,
    /// How long it took, in microseconds.
    pub us: u32,
    /// How many spans it was inside: 0 for one inside no other.
    pub depth: u8,
}

impl SpanEvent {
    /// A span called `name` of `us` microseconds, `start_us` into the
    /// frame, inside `depth` others.
    pub fn new(name: impl Into<Cow<'static, str>>, start_us: u32, us: u32, depth: u8) -> Self {
        Self {
            name: name.into(),
            start_us,
            us,
            depth,
        }
    }
}

/// The heap's counts at a frame's end, in bytes asked for (never the
/// allocator's own overhead). Build it with [`HeapCounts::new`]
/// (ADR-0039).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct HeapCounts {
    /// The bytes allocated and not yet freed.
    pub live_bytes: u64,
    /// The most bytes live at once since the program started.
    pub peak_bytes: u64,
    /// The calls that allocated or reallocated since the last reading: one
    /// reading a frame, so the frame's.
    pub allocs: u32,
}

impl HeapCounts {
    /// `live_bytes` live, `peak_bytes` at the most, and `allocs` calls
    /// since the last reading.
    pub const fn new(live_bytes: u64, peak_bytes: u64, allocs: u32) -> Self {
        Self {
            live_bytes,
            peak_bytes,
            allocs,
        }
    }
}

/// What a backend reads of the device's memory at a frame's end
/// (`shuyi_platform::Platform::memory`). Build it with
/// [`MemoryReading::new`] (ADR-0039).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct MemoryReading {
    /// The heap's counts.
    pub heap: HeapCounts,
}

impl MemoryReading {
    /// A reading of the heap's counts `heap`.
    pub const fn new(heap: HeapCounts) -> Self {
        Self { heap }
    }
}

/// What a memory tag names (ADR-0046 §6): the heap counter counts each
/// block under the innermost tag entered when it was allocated. Its number
/// is the class a backend's counter keeps with each name
/// ([`MemoryScopes::tag`]); `Other` is 0, the class of the counter's own
/// `other` and `overflow`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
#[repr(u8)]
#[non_exhaustive]
pub enum TagClass {
    /// A phase of the frame: `load`, `update`, `draw`, `paint`, `perf`,
    /// and `app` for the rest of `App`'s frame.
    Phase = 1,
    /// A span the game marked ([`Ctx::span`](crate::Ctx::span)).
    Span = 2,
    /// An asset kind being decoded: `image`, `palette`, …
    Asset = 3,
    /// None of those: `other`, what is allocated outside any tag, and
    /// `overflow`, the names past the counter's table.
    Other = 0,
}

impl TagClass {
    /// The class numbered `n`, if there is one.
    pub const fn from_u8(n: u8) -> Option<Self> {
        match n {
            0 => Some(Self::Other),
            1 => Some(Self::Phase),
            2 => Some(Self::Span),
            3 => Some(Self::Asset),
            _ => None,
        }
    }
}

/// How a backend lets the engine tag what it allocates (ADR-0046 §6):
/// three plain functions over its heap counter's tags
/// (`shuyi_platform::Platform::memory_scopes`), so neither this crate nor
/// the platform depends on the counter. Build it with
/// [`MemoryScopes::new`] (ADR-0039).
///
/// With scopes in its context ([`Ctx::set_memory_scopes`]), the game's
/// frame enters `update` and `draw`, each [`Ctx::span`] its own name, and
/// the platform's `App` its own phases. Entering a tag changes nothing a
/// game reads.
///
/// [`Ctx::set_memory_scopes`]: crate::Ctx::set_memory_scopes
/// [`Ctx::span`]: crate::Ctx::span
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct MemoryScopes {
    /// The tag of a name of a class ([`TagClass`] as a number), the same
    /// each time it is asked for.
    pub tag: fn(u8, &'static str) -> u16,
    /// Makes a tag the current one, and gives back the one before.
    pub enter: fn(u16) -> u16,
    /// Puts back the tag `enter` gave.
    pub leave: fn(u16),
}

impl MemoryScopes {
    /// Scopes that find a tag with `tag`, and `enter` and `leave` it.
    pub const fn new(
        tag: fn(u8, &'static str) -> u16,
        enter: fn(u16) -> u16,
        leave: fn(u16),
    ) -> Self {
        Self { tag, enter, leave }
    }

    /// Enters the tag of `class`'s `name`, and gives back the tag before,
    /// for [`leave`](Self::leave).
    pub fn enter(&self, class: TagClass, name: &'static str) -> u16 {
        (self.enter)((self.tag)(class as u8, name))
    }

    /// Puts back `before`, the tag [`enter`](Self::enter) gave.
    pub fn leave(&self, before: u16) {
        (self.leave)(before);
    }
}

/// Leaves a tag [`Ctx`](crate::Ctx) entered: the tag before it, with the
/// scopes that entered it.
pub(crate) fn leave_tag(entered: Option<(MemoryScopes, u16)>) {
    if let Some((scopes, before)) = entered {
        scopes.leave(before);
    }
}

/// `bytes` for people: under 1,024 as `1023 B`, then in `KB`, `MB` and
/// `GB` with one decimal, rounded down (`1.0 KB` is 1,024 bytes, `1.0 MB`
/// 1,048,576), in integers.
pub fn bytes_text(bytes: u64) -> String {
    const UNITS: [(u64, &str); 3] = [(1 << 30, "GB"), (1 << 20, "MB"), (1 << 10, "KB")];
    let Some(&(unit, name)) = UNITS.iter().find(|&&(unit, _)| bytes >= unit) else {
        return format!("{bytes} B");
    };
    let tenths = u128::from(bytes) * 10 / u128::from(unit);
    format!("{}.{} {name}", tenths / 10, tenths % 10)
}

/// What one frame measured. Build it with [`FrameRecord::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct FrameRecord {
    /// The frame (`App::frame_number`: frames count from 1).
    pub frame: u32,
    /// How long its parts took.
    pub times: FrameTimes,
    /// The spans that ended in it, in the order they ended (an inner one
    /// before its outer one), at most [`MAX_SPANS`].
    pub spans: Vec<SpanEvent>,
    /// The spans past [`MAX_SPANS`], counted and not kept.
    pub spans_dropped: u32,
    /// The heap's counts at the frame's end, where the backend counts it
    /// (a debug build of `shuyi-app` or a tool program); `None` where it
    /// doesn't, and left out of the JSON then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heap: Option<HeapCounts>,
}

impl FrameRecord {
    /// The record of `frame`, with nothing measured yet.
    pub fn new(frame: u32) -> Self {
        Self {
            frame,
            times: FrameTimes::new(),
            spans: Vec::new(),
            spans_dropped: 0,
            heap: None,
        }
    }
}

/// The last frames' records, oldest first, and how many older ones were
/// dropped: [`LogRing`](crate::log::LogRing)'s twin. Each record pushed
/// gets a sequence number, from 0 and never reused, so a reader takes the
/// records [`since`](Self::since) the last it took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfRing {
    records: VecDeque<FrameRecord>,
    capacity: usize,
    dropped: u64,
    /// The sequence number of the next record pushed.
    next: u64,
}

impl PerfRing {
    /// An empty ring that keeps `capacity` records.
    pub fn new(capacity: usize) -> Self {
        Self {
            records: VecDeque::new(),
            capacity,
            dropped: 0,
            next: 0,
        }
    }

    /// Keeps `record`, dropping (and counting) the oldest when full.
    pub fn push(&mut self, record: FrameRecord) {
        self.next += 1;
        if self.capacity == 0 {
            self.dropped += 1;
            return;
        }
        if self.records.len() == self.capacity {
            self.records.pop_front();
            self.dropped += 1;
        }
        self.records.push_back(record);
    }

    /// The kept records, oldest first.
    pub fn records(&self) -> vec_deque::Iter<'_, FrameRecord> {
        self.records.iter()
    }

    /// How many records were dropped to keep the newest.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// The sequence number the next record pushed gets.
    pub fn next_seq(&self) -> u64 {
        self.next
    }

    /// The kept records whose sequence number is `seq` or more, oldest
    /// first: those after the last a reader took, if it kept
    /// [`next_seq`](Self::next_seq) then.
    pub fn since(&self, seq: u64) -> vec_deque::Iter<'_, FrameRecord> {
        let skip = crate::log::skip_before(seq, self.next, self.records.len());
        self.records.range(skip..)
    }
}

/// One memory tag's counts in a [`HeapSnapshot`], in bytes asked for.
/// Build it with [`TagRow::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct TagRow {
    /// What the tag names: a phase, a span, an asset kind, or `other`.
    pub class: TagClass,
    /// Its name: `update`, the span's (`ai`), the asset kind's (`image`),
    /// `other` or `overflow`.
    pub name: Cow<'static, str>,
    /// The bytes live under it: allocated, or last grown, under it and not
    /// yet freed.
    pub live_bytes: u64,
    /// The blocks live under it.
    pub live_blocks: u32,
    /// The bytes ever allocated under it: a block grown adds its new size.
    pub allocated_bytes: u64,
    /// The most bytes live under it at once.
    pub peak_bytes: u64,
}

impl TagRow {
    /// `class`'s `name` with `live_bytes` live in `live_blocks` blocks, as
    /// if just allocated: its bytes ever allocated and its peak are its
    /// live bytes (assign them otherwise).
    pub fn new(
        class: TagClass,
        name: impl Into<Cow<'static, str>>,
        live_bytes: u64,
        live_blocks: u32,
    ) -> Self {
        Self {
            class,
            name: name.into(),
            live_bytes,
            live_blocks,
            allocated_bytes: live_bytes,
            peak_bytes: live_bytes,
        }
    }
}

/// What holds the heap at a frame (ADR-0046 §6): its live bytes, and each
/// memory tag's counts. Build it with [`HeapSnapshot::new`] (ADR-0039),
/// which puts the rows in order; compare two with [`diff`](Self::diff).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct HeapSnapshot {
    /// The frame it was taken after (frames count from 1; 0 is before the
    /// first).
    pub frame: u32,
    /// The bytes allocated and not yet freed.
    pub live_bytes: u64,
    /// The most bytes live at once since the program started.
    pub peak_bytes: u64,
    /// Each tag with bytes live or ever allocated under it: the most live
    /// bytes first, then by class, then by name.
    pub tags: Vec<TagRow>,
    /// How many names had no tag of their own, past the counter's table,
    /// and count under `overflow`.
    pub overflowed: u32,
}

impl HeapSnapshot {
    /// A snapshot of frame 0 with `live_bytes` live, `peak_bytes` at the
    /// most, and the rows `tags`, put in order: the most live bytes first,
    /// then by class, then by name. None overflowed.
    pub fn new(live_bytes: u64, peak_bytes: u64, mut tags: Vec<TagRow>) -> Self {
        tags.sort_by(|a, b| {
            b.live_bytes
                .cmp(&a.live_bytes)
                .then_with(|| (a.class as u8).cmp(&(b.class as u8)))
                .then_with(|| a.name.cmp(&b.name))
        });
        Self {
            frame: 0,
            live_bytes,
            peak_bytes,
            tags,
            overflowed: 0,
        }
    }

    /// What changed from this snapshot to `later`: the tags whose live
    /// bytes or blocks changed, a tag missing from one counting as 0
    /// there.
    pub fn diff(&self, later: &HeapSnapshot) -> HeapDiff {
        let mut tags: Vec<TagDelta> = Vec::new();
        for row in &self.tags {
            let after = later.row(row.class, &row.name);
            let (bytes, blocks) = after.map_or((0, 0), |a| (a.live_bytes, a.live_blocks));
            tags.push(TagDelta::new(
                row.class,
                row.name.clone(),
                change(row.live_bytes, bytes),
                i64::from(blocks) - i64::from(row.live_blocks),
            ));
        }
        for row in &later.tags {
            if self.row(row.class, &row.name).is_none() {
                tags.push(TagDelta::new(
                    row.class,
                    row.name.clone(),
                    change(0, row.live_bytes),
                    i64::from(row.live_blocks),
                ));
            }
        }
        tags.retain(|t| t.live_bytes != 0 || t.live_blocks != 0);
        tags.sort_by(|a, b| {
            b.live_bytes
                .unsigned_abs()
                .cmp(&a.live_bytes.unsigned_abs())
                .then_with(|| (a.class as u8).cmp(&(b.class as u8)))
                .then_with(|| a.name.cmp(&b.name))
        });
        HeapDiff {
            from: self.frame,
            to: later.frame,
            live_bytes: change(self.live_bytes, later.live_bytes),
            tags,
        }
    }

    /// The row of `class`'s `name`, if it has one.
    fn row(&self, class: TagClass, name: &str) -> Option<&TagRow> {
        self.tags
            .iter()
            .find(|r| r.class == class && r.name == name)
    }
}

/// `to` less `from`, at most [`i64`]'s range.
fn change(from: u64, to: u64) -> i64 {
    let change = i128::from(to) - i128::from(from);
    let fits = change.clamp(i128::from(i64::MIN), i128::from(i64::MAX));
    i64::try_from(fits).unwrap_or_default()
}

/// How one memory tag changed between two snapshots. Build it with
/// [`TagDelta::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct TagDelta {
    /// What the tag names.
    pub class: TagClass,
    /// Its name.
    pub name: Cow<'static, str>,
    /// How many more bytes are live under it: less than 0 for fewer.
    pub live_bytes: i64,
    /// How many more blocks are live under it: less than 0 for fewer.
    pub live_blocks: i64,
}

impl TagDelta {
    /// `class`'s `name`, with `live_bytes` more bytes live in
    /// `live_blocks` more blocks.
    pub fn new(
        class: TagClass,
        name: impl Into<Cow<'static, str>>,
        live_bytes: i64,
        live_blocks: i64,
    ) -> Self {
        Self {
            class,
            name: name.into(),
            live_bytes,
            live_blocks,
        }
    }
}

/// What grew and shrank in the heap between two snapshots
/// ([`HeapSnapshot::diff`]). Build an empty one with [`HeapDiff::new`]
/// (ADR-0039).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct HeapDiff {
    /// The earlier snapshot's frame.
    pub from: u32,
    /// The later snapshot's frame.
    pub to: u32,
    /// How many more bytes are live: less than 0 for fewer.
    pub live_bytes: i64,
    /// The tags whose live bytes or blocks changed: the largest change in
    /// bytes first (more or fewer), then by class, then by name.
    pub tags: Vec<TagDelta>,
}

impl HeapDiff {
    /// Nothing changed from frame `from` to frame `to`.
    pub fn new(from: u32, to: u32) -> Self {
        Self {
            from,
            to,
            live_bytes: 0,
            tags: Vec::new(),
        }
    }
}

/// One line of a perf trace, as agents read it (ADR-0046 §7): a JSON
/// object tagged by `kind`, one a line (`serde_json::to_string(&line)` and
/// a newline).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "kebab-case")]
#[non_exhaustive]
pub enum PerfLine {
    /// What one frame measured: `{"kind":"frame","frame":12,…}`.
    Frame(FrameRecord),
    /// What held the heap at a frame:
    /// `{"kind":"snapshot","frame":600,"live_bytes":…,"tags":[…],…}`.
    Snapshot(HeapSnapshot),
    /// What changed between two snapshots:
    /// `{"kind":"diff","from":1,"to":600,"live_bytes":98304,"tags":[…]}`.
    Diff(HeapDiff),
}

/// How a set of times spread, in whole microseconds: the median, the 99th
/// percentile (nearest rank), the worst and the mean rounded down.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct Spread {
    /// The median: half the times are this or less.
    pub p50: u32,
    /// 99 in a hundred times are this or less.
    pub p99: u32,
    /// The longest.
    pub worst: u32,
    /// The mean, rounded down.
    pub mean: u32,
}

/// How `values` spread, or `None` for no values. The percentiles are
/// nearest-rank: the value at rank ⌈p × n / 100⌉ of the n values sorted,
/// all in integers.
pub fn spread(values: impl IntoIterator<Item = u32>) -> Option<Spread> {
    let mut sorted: Vec<u32> = values.into_iter().collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_unstable();
    let n = sorted.len();
    let rank = |p: usize| (p * n).div_ceil(100).clamp(1, n) - 1;
    let sum: u64 = sorted.iter().map(|&v| u64::from(v)).sum();
    let count = u64::try_from(n).unwrap_or(u64::MAX);
    Some(Spread {
        p50: sorted[rank(50)],
        p99: sorted[rank(99)],
        worst: sorted[n - 1],
        mean: u32::try_from(sum / count).unwrap_or(u32::MAX),
    })
}

/// The microseconds from `from` to `to`: none if `to` is earlier, and at
/// most `u32::MAX`.
pub(crate) fn micros(from: u64, to: u64) -> u32 {
    u32::try_from(to.saturating_sub(from)).unwrap_or(u32::MAX)
}

/// What the context measures of a frame: its clock, and while a frame is
/// measured, its start, the spans open and those ended, and the update's
/// and the drawing's times.
#[derive(Debug, Default)]
pub(crate) struct PerfState {
    clock: Option<PerfClock>,
    /// The backend's memory tags, if it gave them.
    pub(crate) scopes: Option<MemoryScopes>,
    /// The frame's start, while one is measured.
    start: Option<u64>,
    /// How many spans are open.
    depth: u8,
    record: Option<FrameRecord>,
}

impl PerfState {
    pub(crate) fn clock(&self) -> Option<PerfClock> {
        self.clock
    }

    /// Takes `clock` in place of any; `None` stops measuring, and a frame
    /// being measured ends with nothing.
    pub(crate) fn set_clock(&mut self, clock: Option<PerfClock>) {
        self.clock = clock;
        if clock.is_none() {
            self.start = None;
            self.record = None;
        }
    }

    /// The time, if there is a clock.
    pub(crate) fn now(&mut self) -> Option<u64> {
        self.clock.as_mut().map(PerfClock::read)
    }

    /// The time, in a measured frame only.
    pub(crate) fn now_in_frame(&mut self) -> Option<u64> {
        self.start?;
        self.now()
    }

    /// Starts measuring a frame at the time now, if there is a clock.
    pub(crate) fn begin_frame(&mut self) -> Option<u64> {
        let start = self.now()?;
        self.start = Some(start);
        self.depth = 0;
        self.record = Some(FrameRecord::new(0));
        Some(start)
    }

    /// Ends the frame being measured: its update's and drawing's times and
    /// its spans, as a record of frame 0 for `App` to finish.
    pub(crate) fn end_frame(&mut self) -> Option<FrameRecord> {
        self.start = None;
        self.depth = 0;
        self.record.take()
    }

    /// Keeps the update's and the drawing's times: the update from `start`
    /// to `drawing`, the drawing from there to `drawn`.
    pub(crate) fn set_frame_times(
        &mut self,
        start: Option<u64>,
        drawing: Option<u64>,
        drawn: Option<u64>,
    ) {
        if let (Some(record), Some(start), Some(drawing), Some(drawn)) =
            (&mut self.record, start, drawing, drawn)
        {
            record.times.update_us = micros(start, drawing);
            record.times.draw_us = micros(drawing, drawn);
        }
    }

    /// Opens a span, in a measured frame: its start and its depth.
    pub(crate) fn open_span(&mut self) -> Option<(u64, u8)> {
        let start = self.now_in_frame()?;
        let depth = self.depth;
        self.depth = depth.saturating_add(1);
        Some((start, depth))
    }

    /// Closes the span `open_span` opened, keeping it if the frame is still
    /// measured.
    pub(crate) fn close_span(&mut self, name: &'static str, (start, depth): (u64, u8)) {
        self.depth = depth;
        let Some(end) = self.now_in_frame() else {
            return;
        };
        let (Some(frame_start), Some(record)) = (self.start, &mut self.record) else {
            return;
        };
        if record.spans.len() < MAX_SPANS {
            let at = micros(frame_start, start);
            record
                .spans
                .push(SpanEvent::new(name, at, micros(start, end), depth));
        } else {
            record.spans_dropped = record.spans_dropped.saturating_add(1);
        }
    }
}

#[cfg(test)]
mod tests;
