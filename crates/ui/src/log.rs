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
///
/// A backend's sink also reads lines that name no category: a target, such
/// as a library's module path (`gilrs_core::platform`). Those pass at the
/// other targets' level, [`LogLevel::Warn`] until
/// [`with_others`](Self::with_others) says otherwise
/// ([`allows_target`](Self::allows_target)), or at the level of the
/// longest prefix of theirs it was given ([`with_prefix`](Self::with_prefix)).
///
/// [`parse`](Self::parse) reads one from text, as `SHUYI_LOG` gives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogFilter {
    level: LogLevel,
    categories: Vec<(LogCategory, LogLevel)>,
    others: LogLevel,
    prefixes: Vec<(String, LogLevel)>,
}

impl Default for LogFilter {
    /// [`LogLevel::Info`] and more serious, in every category;
    /// [`LogLevel::Warn`] and more serious for every other target.
    fn default() -> Self {
        Self::new(LogLevel::Info)
    }
}

impl LogFilter {
    /// `level` and more serious, in every category; [`LogLevel::Warn`] and
    /// more serious for every other target.
    pub fn new(level: LogLevel) -> Self {
        Self {
            level,
            categories: Vec::new(),
            others: LogLevel::Warn,
            prefixes: Vec::new(),
        }
    }

    /// The filter `text` says, over [`LogFilter::default`]: items split by
    /// commas, each trimmed, empty ones skipped. An item is a level
    /// (`error`, `warn`, `info` or `debug`) for every category; a
    /// category's name, `=` and a level, for that category
    /// ([`with`](Self::with)); or any other name, `=` and a level, for
    /// every other target that starts with it
    /// ([`with_prefix`](Self::with_prefix)). A later item wins.
    ///
    /// `warn,audio=debug,gilrs=info` is `warn` and more serious, `debug`
    /// in the audio, and `info` for `gilrs` and `gilrs_core::…`.
    ///
    /// # Errors
    ///
    /// Why an item isn't one of those, in English: a level that isn't one
    /// (`loud`, `audio=loud`) or no name before `=` (`=debug`).
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut filter = Self::default();
        for item in text.split(',').map(str::trim).filter(|i| !i.is_empty()) {
            let Some((name, level)) = item.split_once('=') else {
                filter.level = level_named(item, item)?;
                continue;
            };
            let (name, level) = (name.trim(), level_named(level.trim(), item)?);
            if name.is_empty() {
                return Err(format!("`{item}` names no category or target before `=`"));
            }
            filter = match LogCategory::from_name(name) {
                Some(category) => filter.with(category, level),
                None => filter.with_prefix(name, level),
            };
        }
        Ok(filter)
    }

    /// The same filter, with `category` at `level`.
    #[must_use]
    pub fn with(mut self, category: LogCategory, level: LogLevel) -> Self {
        self.categories.retain(|(c, _)| *c != category);
        self.categories.push((category, level));
        self
    }

    /// The same filter, with every target that names no category at
    /// `level`.
    #[must_use]
    pub fn with_others(mut self, level: LogLevel) -> Self {
        self.others = level;
        self
    }

    /// The same filter, with every target that names no category and
    /// starts with `prefix` at `level` (`gilrs` covers `gilrs_core::…`
    /// too). Of two prefixes of a target, the longer wins; `prefix` given
    /// again keeps the last level.
    #[must_use]
    pub fn with_prefix(mut self, prefix: &str, level: LogLevel) -> Self {
        self.prefixes.retain(|(p, _)| p != prefix);
        self.prefixes.push((prefix.to_owned(), level));
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

    /// Whether a line of the target `target` at `level` passes: a
    /// category's name is read as that category ([`allows`](Self::allows)),
    /// one that starts with prefixes at the level of the longest, any other
    /// at the other targets' level.
    pub fn allows_target(&self, target: &str, level: LogLevel) -> bool {
        if let Some(category) = LogCategory::from_name(target) {
            return self.allows(category, level);
        }
        let longest = self
            .prefixes
            .iter()
            .filter(|(p, _)| target.starts_with(p.as_str()))
            .max_by_key(|(p, _)| p.len());
        level <= longest.map_or(self.others, |(_, l)| *l)
    }

    /// The most detailed level any line passes at: a sink asks for no
    /// line more detailed.
    pub fn most_detailed(&self) -> LogLevel {
        let categories = self.categories.iter().map(|(_, l)| *l);
        let prefixes = self.prefixes.iter().map(|(_, l)| *l);
        categories
            .chain(prefixes)
            .fold(self.level.max(self.others), LogLevel::max)
    }
}

/// The level called `name`, or why it isn't one, naming the `item` it was
/// in.
fn level_named(name: &str, item: &str) -> Result<LogLevel, String> {
    LogLevel::from_name(name)
        .ok_or_else(|| format!("`{item}`: `{name}` is not a level (error, warn, info or debug)"))
}

#[cfg(test)]
mod tests;
