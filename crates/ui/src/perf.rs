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
}

impl FrameRecord {
    /// The record of `frame`, with nothing measured yet.
    pub fn new(frame: u32) -> Self {
        Self {
            frame,
            times: FrameTimes::new(),
            spans: Vec::new(),
            spans_dropped: 0,
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
