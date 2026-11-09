//! [`Driver`]: the body of the SDK's update callback, around
//! [`App::frame`], with the backend's device behind [`Sdk`] so the tests run
//! all of it on the fake.

use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use shuyi_assets::bundle::Watcher;
use shuyi_platform::{App, Millis, Start};
use shuyi_ui::log::{LogCategory, LogLevel};
use shuyi_ui::{Game, GameData};

use crate::backend::Playdate;
use crate::display::Showing;
use crate::report::{self, Now, Reads, RunReport, Stretch};
use crate::sdk::{Sdk, SdkEvent};

/// How long the frames a second are measured over before they are reported,
/// in milliseconds.
const FPS_WINDOW_MS: u64 = 1000;

/// A clock of whole milliseconds from a counter that wraps at `u32::MAX`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Clock {
    /// The wraps so far, in milliseconds.
    wrapped: u64,
    last: u32,
}

impl Clock {
    /// The time at the counter's reading `now`.
    fn at(&mut self, now: u32) -> Millis {
        if now < self.last {
            self.wrapped += 1 << 32;
        }
        self.last = now;
        Millis(self.wrapped + u64::from(now))
    }
}

/// The frames a second, measured over a window.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Fps {
    /// When the window began; `None` before the first frame.
    since: Option<Millis>,
    frames: u32,
    last: u16,
}

impl Fps {
    /// A frame ran at `now`.
    fn frame(&mut self, now: Millis) {
        let Some(since) = self.since else {
            self.since = Some(now);
            return;
        };
        self.frames += 1;
        let ms = now.0.saturating_sub(since.0);
        if ms >= FPS_WINDOW_MS {
            let rate = u64::from(self.frames) * 1000 / ms;
            self.last = u16::try_from(rate).unwrap_or(u16::MAX);
            self.since = Some(now);
            self.frames = 0;
        }
    }
}

/// The run report's state, while it is on.
#[derive(Debug)]
struct Reporter {
    report: RunReport,
    /// When the driver was made, in the timer's microseconds: `init`.
    init_us: u64,
    /// When the card was shown, after `init`.
    card_us: Option<u64>,
    /// When the game had loaded, after `init`.
    loaded_us: Option<u64>,
    /// Whether the first frame of the game was shown.
    first_shown: bool,
    /// Whether the bundle's reads are noted.
    reads: bool,
    /// Each update callback that ran the game's load: its time in the
    /// timer's microseconds, and the load's steps it ran.
    callbacks: Vec<(u64, u32)>,
    /// The start's lines, for the file, until the first frame is shown.
    start: Vec<String>,
    /// Every read's line, for the file, once the game has loaded.
    each: Vec<String>,
    /// When the stretch the running lines are of began: from the first
    /// frame shown.
    since: Option<Millis>,
    stretch: Stretch,
}

/// A game on the Playdate: its [`App`] and the backend.
pub struct Driver<G: GameData, S: Sdk> {
    app: App<G>,
    platform: Playdate<S>,
    started: bool,
    clock: Clock,
    fps: Fps,
    /// The run report, if the game turned it on.
    reporter: Option<Box<Reporter>>,
}

impl<G: GameData, S: Sdk> Driver<G, S> {
    /// The game `start` says on `sdk`.
    pub fn new(start: Start<G>, sdk: S) -> Self {
        Self {
            app: App::new(start),
            platform: Playdate::new(sdk),
            started: false,
            clock: Clock::default(),
            fps: Fps::default(),
            reporter: None,
        }
    }

    /// The game `start` says on `sdk`, saying in the console how its start
    /// went and how fast it runs, as `report` says ([`report`]).
    pub fn with_report(start: Start<G>, sdk: S, report: RunReport) -> Self {
        Self::reporting(start, sdk, report, report::watch_bundle)
    }

    /// [`with_report`](Self::with_report), setting the bundle's watcher
    /// with `watch`.
    pub(crate) fn reporting(
        start: Start<G>,
        sdk: S,
        report: RunReport,
        watch: fn(Watcher) -> Result<(), Watcher>,
    ) -> Self {
        let init_us = S::clock_us();
        let mut driver = Self::new(start, sdk);
        let reads = watch(report::noted).is_ok();
        let mut start = Vec::new();
        if reads {
            report::with_reads(|noted| *noted = Some(Reads::new(S::clock_us)));
        } else {
            driver.say(report::REFUSED);
            start.push(String::from(report::REFUSED));
        }
        driver.platform.measure(report.phases);
        driver.reporter = Some(Box::new(Reporter {
            report,
            init_us,
            card_us: None,
            loaded_us: None,
            first_shown: false,
            reads,
            callbacks: Vec::new(),
            start,
            each: Vec::new(),
            since: None,
            stretch: Stretch::default(),
        }));
        driver
    }

    /// One update of the SDK's: true when the screen changed.
    pub fn update(&mut self) -> bool {
        self.app.set_pacing(self.platform.pacing(self.fps.last));
        let now = self.clock.at(self.platform.sdk().millis());
        let steps = self.app.load_steps();
        let began = self.reporter.is_some().then(S::clock_us);
        // A game that has quit keeps its last frame: the player leaves
        // through the system's Home.
        let _ = self.app.frame(&mut self.platform, now);
        let timed = began.map(|began| (began, S::clock_us()));
        self.fps.frame(now);
        self.after_frame();
        let changed = self.platform.take_changed();
        if let Some((began, ended)) = timed {
            let steps = self.app.load_steps() - steps;
            self.report(began, ended, now, changed, steps);
        }
        changed
    }

    /// The run report's part of an update whose `App::frame` ran from
    /// `began` to `ended` (the timer's microseconds), at `now`, and ran
    /// `steps` of the game's load.
    fn report(&mut self, began: u64, ended: u64, now: Millis, changed: bool, steps: u32) {
        let Some(mut r) = self.reporter.take() else {
            return;
        };
        let in_start = !r.first_shown;
        let at = ended.saturating_sub(r.init_us);
        let showing = self.platform.showing();
        let mut lines = Vec::new();
        if r.card_us.is_none() && showing == Showing::Card {
            r.card_us = Some(at);
            lines.push(report::shown_line("Loading card", at));
        }
        if steps > 0 {
            r.callbacks.push((ended.saturating_sub(began), steps));
            if self.app.is_loading() {
                // The time to the next callback is no file's.
                report::mark(report::BETWEEN);
            }
        }
        if r.loaded_us.is_none() && self.app.game().is_some() {
            r.loaded_us = Some(at);
            lines.push(report::load_line(at - r.card_us.unwrap_or(0)));
            lines.push(report::callbacks_line(&r.callbacks));
            let used = self.platform.sdk().stack_used();
            lines.push(report::stack_line(used, r.report.stack_bytes));
            if let Some(reads) = report::with_reads(Option::take).filter(|_| r.reads) {
                lines.extend(reads.summary(ended, r.report.slowest));
                r.each = reads.each(ended);
            }
        }
        if !r.first_shown && r.loaded_us.is_some() && changed && showing == Showing::Game {
            r.first_shown = true;
            lines.push(report::shown_line("First frame of the game", at));
            r.since = Some(now);
            let _ = self.platform.take_measured();
        } else if let Some(since) = r.since {
            r.stretch.frame.add(ended.saturating_sub(began));
            let ms = now.0.saturating_sub(since.0);
            if ms >= u64::from(r.report.every_seconds) * 1000 {
                lines.extend(self.running_lines(&r, ms, now));
                r.since = Some(now);
                r.stretch = Stretch::default();
            }
        }
        for line in &lines {
            self.say(line);
        }
        if in_start {
            r.start.extend(lines);
            if r.first_shown {
                self.write_file(&mut r);
            }
        }
        self.reporter = Some(r);
    }

    /// The running lines, for a stretch of `ms` milliseconds ending `now`.
    fn running_lines(&mut self, r: &Reporter, ms: u64, now: Millis) -> Vec<String> {
        let measured = self.platform.take_measured();
        let underran = self.platform.underran();
        let stack_used = self.platform.sdk().stack_used();
        let at = Now {
            screen: self.app.game().and_then(Game::top_screen),
            ms,
            stack_used,
            music: self.platform.music(now),
            underran,
        };
        r.stretch.lines(&at, &measured, &r.report)
    }

    /// Writes the start's lines and every read to the report's file, if it
    /// names one; the console says if it couldn't.
    fn write_file(&self, r: &mut Reporter) {
        let start = core::mem::take(&mut r.start);
        let each = core::mem::take(&mut r.each);
        let Some(path) = &r.report.file else {
            return;
        };
        let mut text = String::new();
        for line in start.iter().chain(&each) {
            text.push_str(line);
            text.push('\n');
        }
        if let Err(error) = self.platform.sdk().write_data(path, text.as_bytes()) {
            // check-text: not player text (for the console)
            self.say(&format!("Run report: {path} not written: {error}"));
        }
    }

    /// Logs `text` as the run report's line, as the log's lines are.
    fn say(&self, text: &str) {
        self.platform.sdk().log(&format!(
            "f{} {} {}: {text}",
            self.app.frame_number(),
            LogLevel::Info.name(),
            LogCategory::Platform.name(),
        ));
    }

    /// What follows a frame: the backend readied once the game has loaded,
    /// and the frame's log on the SDK's console.
    fn after_frame(&mut self) {
        if !self.started
            && let Some(game) = self.app.game()
        {
            self.platform.start(game);
            self.started = true;
        }
        for line in self.app.take_new_log() {
            self.platform.sdk().log(&format!(
                "f{} {} {}: {}",
                line.frame,
                line.level.name(),
                line.category.name(),
                line.text
            ));
        }
    }

    /// The SDK's event. What must be heard before the system stops the
    /// updates runs a frame now: `App` lets the game save and steps nothing.
    pub fn event(&mut self, event: SdkEvent) {
        if self.platform.event(event) {
            let now = self.clock.at(self.platform.sdk().millis());
            let _ = self.app.frame(&mut self.platform, now);
            self.after_frame();
        }
    }

    /// The player chose the system menu's item in `slot`.
    pub fn menu_chosen(&mut self, slot: u8) {
        self.platform.menu_chosen(slot);
    }

    /// The game's app, for the tests.
    pub fn app(&self) -> &App<G> {
        &self.app
    }

    /// The backend, for the tests.
    pub fn platform(&self) -> &Playdate<S> {
        &self.platform
    }
}

#[cfg(test)]
mod tests;
