//! The engine's log (ADR-0030): lines with a level, a category and the
//! frame they were given in, kept in a [`LogRing`] of the last
//! [`LOG_LINES`].
//!
//! A screen or the engine logs through [`Ctx::log`](crate::Ctx::log); the
//! platform's `App` takes the lines each frame, stamps them with its frame
//! and keeps them for tests, tools and the backend's sinks. A log is
//! output only: nothing reads it to decide anything. Its lines are
//! English, never translated, and not player text.

use crate::prelude::*;
use alloc::collections::VecDeque;
use alloc::collections::vec_deque;

/// How serious a line is, from the most serious: `Error < Debug`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum LogLevel {
    /// Something failed: the game can't do what it was asked.
    Error,
    /// Something went wrong and the game made do (the defaults used).
    Warn,
    /// What happened, for anyone reading.
    Info,
    /// Detail for whoever is chasing a bug.
    Debug,
}

/// What part of the engine or game a line is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum LogCategory {
    /// The game's data files.
    Assets,
    /// Sounds, music and voices.
    Audio,
    /// Keys, controllers and their bindings.
    Input,
    /// The screens.
    Screens,
    /// Saves, settings and profiles.
    Storage,
    /// The platform: the load, the device.
    Platform,
    /// The game's own lines.
    Game,
}

impl LogLevel {
    /// Every level, from the most serious.
    pub const ALL: [Self; 4] = [Self::Error, Self::Warn, Self::Info, Self::Debug];

    /// Its name: `error`, `warn`, `info` or `debug`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }

    /// The level called `name`, if one is.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|level| level.name() == name)
    }
}

impl LogCategory {
    /// Every category.
    pub const ALL: [Self; 7] = [
        Self::Assets,
        Self::Audio,
        Self::Input,
        Self::Screens,
        Self::Storage,
        Self::Platform,
        Self::Game,
    ];

    /// Its name, in lowercase: `assets` to `game`.
    pub fn name(self) -> &'static str {
        match self {
            Self::Assets => "assets",
            Self::Audio => "audio",
            Self::Input => "input",
            Self::Screens => "screens",
            Self::Storage => "storage",
            Self::Platform => "platform",
            Self::Game => "game",
        }
    }

    /// The category called `name`, if one is.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|category| category.name() == name)
    }
}

/// One line of the log. Build it with [`LogLine::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LogLine {
    /// The frame it was given in (frames count from 1; 0 is before the
    /// first).
    pub frame: u32,
    /// How serious it is.
    pub level: LogLevel,
    /// What it is about.
    pub category: LogCategory,
    /// What it says, in English.
    pub text: String,
}

impl LogLine {
    /// A line given in `frame`.
    pub fn new(
        frame: u32,
        level: LogLevel,
        category: LogCategory,
        text: impl Into<String>,
    ) -> Self {
        Self {
            frame,
            level,
            category,
            text: text.into(),
        }
    }
}

/// How many lines a log keeps: older ones are dropped and counted.
pub const LOG_LINES: usize = 500;

/// The last lines of a log, oldest first, and how many older ones were
/// dropped. Each line pushed gets a sequence number, from 0 and never
/// reused, so a reader takes the lines [`since`](Self::since) the last it
/// took.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogRing {
    lines: VecDeque<LogLine>,
    capacity: usize,
    dropped: u32,
    /// The sequence number of the next line pushed.
    next: u64,
}

impl LogRing {
    /// An empty ring that keeps `capacity` lines.
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            capacity,
            dropped: 0,
            next: 0,
        }
    }

    /// Keeps `line`, dropping (and counting) the oldest when full.
    pub fn push(&mut self, line: LogLine) {
        self.next += 1;
        if self.capacity == 0 {
            self.add_dropped(1);
            return;
        }
        if self.lines.len() == self.capacity {
            self.lines.pop_front();
            self.add_dropped(1);
        }
        self.lines.push_back(line);
    }

    /// The kept lines, oldest first.
    pub fn lines(&self) -> vec_deque::Iter<'_, LogLine> {
        self.lines.iter()
    }

    /// How many lines were dropped to keep the newest.
    pub fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Counts `n` more lines dropped (elsewhere, before they got here).
    pub fn add_dropped(&mut self, n: u32) {
        self.dropped = self.dropped.saturating_add(n);
    }

    /// The sequence number the next line pushed gets.
    pub fn next_seq(&self) -> u64 {
        self.next
    }

    /// The kept lines whose sequence number is `seq` or more, oldest
    /// first: those after the last a reader took, if it kept
    /// [`next_seq`](Self::next_seq) then.
    pub fn since(&self, seq: u64) -> vec_deque::Iter<'_, LogLine> {
        let skip = skip_before(seq, self.next, self.lines.len());
        self.lines.range(skip..)
    }

    /// Takes every kept line and the dropped count, leaving the ring empty
    /// with none dropped. Sequence numbers go on from where they were.
    pub fn take(&mut self) -> (Vec<LogLine>, u32) {
        let lines = self.lines.drain(..).collect();
        (lines, core::mem::take(&mut self.dropped))
    }
}

/// How many of a ring's `kept` items, the newest numbered `next - 1`, are
/// numbered below `seq`.
pub(crate) fn skip_before(seq: u64, next: u64, kept: usize) -> usize {
    let kept_u64 = u64::try_from(kept).unwrap_or(u64::MAX);
    let skip = seq.saturating_sub(next.saturating_sub(kept_u64));
    usize::try_from(skip).unwrap_or(usize::MAX).min(kept)
}

/// Which lines a log keeps: a level for every category, and a level per
/// category over it. A line passes at its category's level or more
/// serious.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFilter {
    level: LogLevel,
    categories: Vec<(LogCategory, LogLevel)>,
}

impl Default for LogFilter {
    /// [`LogLevel::Info`] and more serious, in every category.
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

impl LogFilter {
    /// `level` and more serious, in every category.
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            categories: Vec::new(),
        }
    }

    /// The same filter, with `category` at `level`.
    #[must_use]
    pub fn with(mut self, category: LogCategory, level: LogLevel) -> Self {
        self.categories.retain(|(c, _)| *c != category);
        self.categories.push((category, level));
        self
    }

    /// Whether a line of `category` at `level` passes.
    pub fn allows(&self, category: LogCategory, level: LogLevel) -> bool {
        let at = self
            .categories
            .iter()
            .find(|(c, _)| *c == category)
            .map_or(self.level, |(_, l)| *l);
        level <= at
    }
}

#[cfg(test)]
mod tests;
