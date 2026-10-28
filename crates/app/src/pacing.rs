//! The frame pacer (ADR-0050): how long [`run`](crate::run)'s loop waits
//! after a frame so frames come no faster than the frame cap, and on the
//! web which of the browser's frames run the game. It is given the times
//! the clock reads (seconds, as `f64`) and answers, so it is tested
//! without a clock; `run` reads its clock ([`seconds`]) and sleeps
//! ([`sleep`]).
//!
//! Its rules:
//!
//! - **Deadlines, not a period after each frame.** The next frame is due
//!   a period after the last was due; after a frame that ran late, the
//!   deadlines start again from it, so there's no burst of fast frames to
//!   catch up.
//! - **Never wait twice.** It keeps a rolling measure of the frame
//!   interval without its own waits. When that is already about the
//!   period (vsync, the display or a slow machine limiting the rate), it
//!   waits nothing, so a 60 cap on a 60 Hz display with vsync on never
//!   misses a vblank and drops to 30. When it waits, it leaves the time the
//!   swap takes (vsync's wait for the vblank) out of its own wait.
//! - **Off** never waits.
//! - **The web** can't sleep, and the browser's `requestAnimationFrame`
//!   sets the rate: a frame that comes before the due time skips the game
//!   ([`Pacer::web_frame`]), and the input it read is kept for the next
//!   frame that runs ([`Held`]).

use shuyi_platform::SystemEvent;
use shuyi_ui::RawInputEvent;
use shuyi_ui::frame_cap::{FrameCap, Pacing};

/// How far under the period the frame interval without the pacer's waits
/// may be and still count as limited already (by vsync, the display or the
/// machine): a share of the period, of a rolling average, so jitter
/// doesn't count. Under the 2 % between a 141 cap and a 144 Hz display, so
/// that cap paces a VRR display instead of leaving it to vsync. *Tunable.*
const TOLERANCE: f64 = 0.01;

/// How late a frame may start, as a share of the period, and still keep
/// its deadline: later, the deadlines start again from it. *Tunable.*
const LATE: f64 = 0.5;

/// How much each new measure weighs in the rolling averages. *Tunable.*
const WEIGHT: f64 = 0.1;

/// How long before the due time [`sleep`] stops sleeping and yields
/// instead, in seconds. A sleep overshot by 0.5 ms (median) to 0.8 ms
/// (most) on Windows 11 (measured 2026-10-27), and the yielding keeps a
/// core busy: 1 ms of it cost hello-grid about 3 % more of a core at 60
/// frames a second than 0.5 ms. A frame that starts up to 0.3 ms late
/// keeps its deadline, so the rate holds. *Tunable.*
pub(crate) const SPIN_S: f64 = 0.0005;

/// A rolling average: each new measure moves it by [`WEIGHT`] of the
/// difference; the first is the average.
#[derive(Debug, Clone, Copy, Default)]
struct Average(Option<f64>);

impl Average {
    fn add(&mut self, x: f64) {
        self.0 = Some(self.0.map_or(x, |avg| avg + (x - avg) * WEIGHT));
    }

    fn get(self) -> Option<f64> {
        self.0
    }
}

/// The pacer: the cap it applies, when the next frame is due, and what it
/// has measured.
#[derive(Debug, Clone)]
pub(crate) struct Pacer {
    /// The cap applied, and the serial it came with.
    cap: FrameCap,
    serial: u32,
    /// Whether the window asked for vsync.
    vsync: bool,
    /// When the next frame is due to start; `None` while nothing paces
    /// (no cap, the rate limited already, before the first frame).
    due: Option<f64>,
    /// When the frame running started (on the web, the last that ran).
    start: Option<f64>,
    /// When its work ended, and how long it then waited.
    work_end: Option<f64>,
    waited: f64,
    /// When its wait ended, while the swap hasn't returned.
    wait_end: Option<f64>,
    /// When the browser's last frame came (the web).
    tick: Option<f64>,
    /// The frame interval without the pacer's waits.
    unpaced: Average,
    /// The time from the end of a wait to the next frame's start: what
    /// the swap takes.
    swap: Average,
    /// The frame interval of the frames that ran, for the frames a second.
    interval: Average,
}

impl Pacer {
    /// A pacer at the engine's default cap (until the first frame says),
    /// in a window that asked for `vsync` or not.
    pub(crate) fn new(vsync: bool) -> Self {
        Self {
            cap: FrameCap::DEFAULT,
            serial: 0,
            vsync,
            due: None,
            start: None,
            work_end: None,
            waited: 0.0,
            wait_end: None,
            tick: None,
            unpaced: Average::default(),
            swap: Average::default(),
            interval: Average::default(),
        }
    }

    /// Applies `cap`, which came with `serial`, from the next frame.
    pub(crate) fn set_cap(&mut self, cap: FrameCap, serial: u32) {
        self.cap = cap;
        self.serial = serial;
    }

    /// The time between frames at the cap, in seconds; `None` for no cap.
    fn period(&self) -> Option<f64> {
        self.cap.limit().map(|fps| 1.0 / f64::from(fps))
    }

    /// When the next frame is due, if anything paces.
    #[cfg(test)]
    fn due(&self) -> Option<f64> {
        self.due
    }

    /// A frame that runs starts at `now`: its interval since the last is
    /// measured, and the next one's deadline set.
    fn ran(&mut self, now: f64) {
        if let Some(last) = self.start {
            self.interval.add((now - last).max(0.0));
        }
        self.start = Some(now);
        self.due = self.period().map(|period| {
            let this = match self.due {
                Some(due) if now <= due + period * LATE => due,
                // Late, or nothing paced it: the deadlines start again.
                _ => now,
            };
            this + period
        });
    }

    /// A frame starts at `now`, on a backend that waits (native): what the
    /// last took without its wait, and what its swap took, are measured.
    pub(crate) fn start(&mut self, now: f64) {
        if let Some(last) = self.start {
            self.unpaced.add((now - last - self.waited).max(0.0));
        }
        if let Some(end) = self.wait_end.take() {
            self.swap.add((now - end).max(0.0));
        }
        self.work_end = None;
        self.waited = 0.0;
        self.ran(now);
    }

    /// Whether something else already keeps the frames a period apart:
    /// vsync, the display or the machine.
    fn limited(&self, period: f64) -> bool {
        self.unpaced
            .get()
            .is_some_and(|unpaced| unpaced >= period * (1.0 - TOLERANCE))
    }

    /// The frame's work is done at `now`: how long to wait, in seconds,
    /// before the next frame. Nothing with no cap, before a frame has been
    /// measured, or when the rate is limited already.
    pub(crate) fn wait(&mut self, now: f64) -> f64 {
        self.work_end = Some(now);
        let Some(period) = self.period() else {
            return 0.0;
        };
        let Some(due) = self.due.filter(|_| self.unpaced.get().is_some()) else {
            self.due = None;
            return 0.0;
        };
        if self.limited(period) {
            // The next frame keeps its own time: its deadline is its start.
            self.due = None;
            return 0.0;
        }
        let swap = self.swap.get().unwrap_or(0.0);
        (due - swap - now).max(0.0)
    }

    /// The wait after the frame's work ended at `now`.
    pub(crate) fn waited(&mut self, now: f64) {
        let work_end = self.work_end.unwrap_or(now);
        self.waited = (now - work_end).max(0.0);
        self.wait_end = Some(now);
    }

    /// On the web, the browser's frame comes at `now`: whether the game's
    /// frame runs in it. With a cap, it runs once its due time is near
    /// (within half the browser's interval), so the deadlines keep the
    /// rate on average; it always runs when the browser's own rate is no
    /// faster than the cap. The game's time is the time since the last
    /// frame that ran, skipped frames and all: `App` counts it from its
    /// clock.
    pub(crate) fn web_frame(&mut self, now: f64) -> WebFrame {
        if let Some(last) = self.tick {
            self.unpaced.add((now - last).max(0.0));
        }
        self.tick = Some(now);
        let run = match (self.period(), self.due, self.unpaced.get()) {
            (Some(period), Some(due), Some(tick)) => {
                self.limited(period) || now >= due - tick / 2.0
            }
            _ => true,
        };
        if run {
            self.ran(now);
            WebFrame::Run
        } else {
            WebFrame::Skip
        }
    }

    /// The report for the game ([`Pacing`]): the cap applied, its serial,
    /// vsync and the frames a second measured.
    pub(crate) fn pacing(&self) -> Pacing {
        let mut pacing = Pacing::new(self.cap, self.serial);
        pacing.vsync = self.vsync;
        pacing.fps = self.interval.get().map_or(0, fps);
        pacing
    }
}

/// Frames a second, rounded, from the time between them in seconds: 0 for
/// none measured.
fn fps(interval: f64) -> u16 {
    if interval <= 0.0 {
        return 0;
    }
    let fps = (1.0 / interval).round().clamp(0.0, f64::from(u16::MAX));
    // In 0..=u16::MAX after the clamp.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let fps = fps as u16;
    fps
}

/// Whether the browser's frame runs the game ([`Pacer::web_frame`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WebFrame {
    /// The game's frame runs.
    Run,
    /// It isn't due: the last frame is shown again, and the input kept.
    Skip,
}

/// The input of the web's skipped frames, kept in order for the next frame
/// that runs: macroquad forgets a frame's key presses and releases at its
/// end, so a frame that doesn't run the game must still read them.
#[derive(Debug, Default)]
pub(crate) struct Held {
    input: Vec<RawInputEvent>,
    system: Vec<SystemEvent>,
}

impl Held {
    /// Keeps a skipped frame's input and system events, after those kept
    /// before.
    pub(crate) fn keep(
        &mut self,
        input: impl IntoIterator<Item = RawInputEvent>,
        system: impl IntoIterator<Item = SystemEvent>,
    ) {
        self.input.extend(input);
        self.system.extend(system);
    }

    /// Moves the kept input into `out`, ahead of what comes after.
    pub(crate) fn take_input(&mut self, out: &mut Vec<RawInputEvent>) {
        out.append(&mut self.input);
    }

    /// Moves the kept system events into `out`, ahead of what comes after.
    pub(crate) fn take_system(&mut self, out: &mut Vec<SystemEvent>) {
        out.append(&mut self.system);
    }
}

/// Seconds on the loop's one clock, which the pacer, `App::frame`'s time
/// and the perf clock all read: an `Instant` on native (the system's time,
/// which `miniquad::date::now` reads there, can jump), the page's
/// `Date.now()` on the web.
pub(crate) fn seconds() -> f64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        static START: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
        START
            .get_or_init(std::time::Instant::now)
            .elapsed()
            .as_secs_f64()
    }
    #[cfg(target_arch = "wasm32")]
    {
        macroquad::miniquad::date::now()
    }
}

/// Waits `secs` seconds precisely on `now`'s clock (seconds): a sleep
/// until [`SPIN_S`] before the end, then yields to other threads until it.
/// Native only: a web page can't sleep.
pub(crate) fn sleep(secs: f64, now: impl Fn() -> f64) {
    if secs <= 0.0 {
        return;
    }
    let until = now() + secs;
    if let Ok(nap) = std::time::Duration::try_from_secs_f64(secs - SPIN_S) {
        std::thread::sleep(nap);
    }
    while now() < until {
        std::thread::yield_now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A machine and its display on a fake clock: each frame's work takes
    /// `work` seconds (`slow` for the frames it names), and the swap
    /// returns at the next vblank of a display of `hz` with vsync on, at
    /// once with vsync off, or, on a VRR display, no sooner than a refresh
    /// after the last.
    struct Machine {
        now: f64,
        work: f64,
        display: Display,
        last_shown: f64,
    }

    #[derive(Clone, Copy)]
    enum Display {
        NoVsync,
        Vsync(f64),
        Vrr(f64),
    }

    /// What one frame did: the wait the pacer asked for, and its start.
    #[derive(Debug, Clone, Copy)]
    struct Frame {
        wait: f64,
        start: f64,
    }

    impl Machine {
        fn new(work: f64, display: Display) -> Self {
            Self {
                now: 0.0,
                work,
                display,
                last_shown: 0.0,
            }
        }

        /// One frame with `work` seconds of work, as `run` drives it.
        fn frame_of(&mut self, pacer: &mut Pacer, work: f64) -> Frame {
            let start = self.now;
            pacer.start(start);
            self.now += work;
            let wait = pacer.wait(self.now);
            self.now += wait;
            pacer.waited(self.now);
            self.now = match self.display {
                Display::NoVsync => self.now,
                Display::Vsync(hz) => ((self.now * hz) - 1e-9).ceil() / hz,
                Display::Vrr(hz) => self.now.max(self.last_shown + 1.0 / hz),
            };
            self.last_shown = self.now;
            Frame { wait, start }
        }

        /// `n` frames of the machine's own work.
        fn frames(&mut self, pacer: &mut Pacer, n: usize) -> Vec<Frame> {
            (0..n).map(|_| self.frame_of(pacer, self.work)).collect()
        }
    }

    fn pacer(cap: FrameCap) -> Pacer {
        let mut pacer = Pacer::new(true);
        pacer.set_cap(cap, 0);
        pacer
    }

    /// The intervals between the frames' starts.
    fn intervals(frames: &[Frame]) -> Vec<f64> {
        frames.windows(2).map(|w| w[1].start - w[0].start).collect()
    }

    fn near(a: f64, b: f64, by: f64) -> bool {
        (a - b).abs() <= by
    }

    const MS: f64 = 0.001;

    #[test]
    fn at_60_with_2_ms_of_work_it_waits_the_rest() {
        let mut pacer = pacer(FrameCap::Fps(60));
        let frames = Machine::new(2.0 * MS, Display::NoVsync).frames(&mut pacer, 120);
        for frame in &frames[2..] {
            assert!(near(frame.wait, 14.67 * MS, 0.05 * MS), "{frame:?}");
        }
        for interval in intervals(&frames[2..]) {
            assert!(near(interval, 1.0 / 60.0, 0.01 * MS), "{interval}");
        }
        assert_eq!(pacer.pacing().fps, 60);
    }

    #[test]
    fn with_60_hz_vsync_already_limiting_it_never_waits() {
        let mut pacer = pacer(FrameCap::Fps(60));
        let frames = Machine::new(2.0 * MS, Display::Vsync(60.0)).frames(&mut pacer, 120);
        assert!(frames.iter().all(|f| f.wait == 0.0), "{frames:?}");
        for interval in intervals(&frames) {
            assert!(near(interval, 1.0 / 60.0, 0.01 * MS), "{interval}");
        }
    }

    #[test]
    fn at_30_on_a_60_hz_vsync_display_it_waits_a_vblank_more() {
        let mut pacer = pacer(FrameCap::Fps(30));
        let frames = Machine::new(2.0 * MS, Display::Vsync(60.0)).frames(&mut pacer, 120);
        for frame in &frames[3..] {
            assert!(near(frame.wait, 16.67 * MS, 0.3 * MS), "{frame:?}");
        }
        // Every frame two vblanks apart: none misses one and takes three.
        for interval in intervals(&frames[2..]) {
            assert!(near(interval, 2.0 / 60.0, 0.01 * MS), "{interval}");
        }
        assert_eq!(pacer.pacing().fps, 30);
    }

    #[test]
    fn off_never_waits() {
        let mut pacer = pacer(FrameCap::Off);
        for display in [Display::NoVsync, Display::Vsync(60.0)] {
            let frames = Machine::new(2.0 * MS, display).frames(&mut pacer, 60);
            assert!(frames.iter().all(|f| f.wait == 0.0));
        }
        assert_eq!(pacer.due(), None);
    }

    #[test]
    fn after_a_late_frame_the_next_is_due_after_it_not_before() {
        let mut pacer = pacer(FrameCap::Fps(60));
        let mut machine = Machine::new(2.0 * MS, Display::NoVsync);
        machine.frames(&mut pacer, 30);
        machine.frame_of(&mut pacer, 100.0 * MS);
        let late = machine.frame_of(&mut pacer, 2.0 * MS);
        assert!(pacer.due().is_some_and(|due| due > late.start));
        // No burst to catch up: it waits as before, and the frames after
        // keep the period.
        assert!(near(late.wait, 14.67 * MS, 0.05 * MS), "{late:?}");
        let after = machine.frames(&mut pacer, 10);
        let mut starts = vec![late];
        starts.extend(after);
        for interval in intervals(&starts) {
            assert!(near(interval, 1.0 / 60.0, 0.01 * MS), "{interval}");
        }
    }

    #[test]
    fn on_a_vrr_display_a_cap_under_its_refresh_paces_and_one_over_lets_it() {
        // The first frames' swaps wait for the display's fastest refresh;
        // once the pacer's waits take that over, the display follows them.
        let mut pacer = pacer(FrameCap::Fps(141));
        let frames = Machine::new(2.0 * MS, Display::Vrr(144.0)).frames(&mut pacer, 300);
        assert!(frames[100..].iter().all(|f| f.wait > 0.0));
        for interval in intervals(&frames[100..]) {
            assert!(near(interval, 1.0 / 141.0, 0.01 * MS), "{interval}");
        }
        assert_eq!(pacer.pacing().fps, 141);
        let mut pacer = self::pacer(FrameCap::Fps(240));
        let frames = Machine::new(2.0 * MS, Display::Vrr(144.0)).frames(&mut pacer, 60);
        assert!(frames.iter().all(|f| f.wait == 0.0), "{frames:?}");
        assert_eq!(pacer.pacing().fps, 144);
    }

    #[test]
    fn a_new_cap_applies_from_the_next_frame_and_is_reported() {
        let mut pacer = Pacer::new(false);
        let mut machine = Machine::new(2.0 * MS, Display::NoVsync);
        machine.frames(&mut pacer, 30);
        assert_eq!(pacer.pacing().cap, FrameCap::Fps(60));
        pacer.set_cap(FrameCap::Fps(30), 7);
        let frames = machine.frames(&mut pacer, 120);
        for interval in intervals(&frames[2..]) {
            assert!(near(interval, 1.0 / 30.0, 0.01 * MS), "{interval}");
        }
        let report = pacer.pacing();
        assert_eq!((report.cap, report.serial), (FrameCap::Fps(30), 7));
        assert!(!report.vsync);
        assert_eq!(report.fps, 30);
        assert_eq!(Pacer::new(true).pacing().fps, 0);
        assert!(Pacer::new(true).pacing().vsync);
    }

    /// The browser's frames at `hz` for `secs` seconds: which ran the game.
    fn web(pacer: &mut Pacer, hz: f64, secs: f64) -> Vec<(f64, WebFrame)> {
        let mut ticks = 0;
        while f64::from(ticks) < secs * hz {
            ticks += 1;
        }
        (0..ticks)
            .map(|i| {
                let now = f64::from(i) / hz;
                (now, pacer.web_frame(now))
            })
            .collect()
    }

    fn runs(frames: &[(f64, WebFrame)]) -> Vec<f64> {
        let ran = frames.iter().filter(|(_, f)| *f == WebFrame::Run);
        ran.map(|&(now, _)| now).collect()
    }

    #[test]
    fn on_the_web_a_frame_before_its_time_is_skipped() {
        // A 144 Hz display at a 60 cap: 60 frames a second run, on
        // average, each a whole number of the browser's frames apart.
        let mut pacer = pacer(FrameCap::Fps(60));
        let frames = web(&mut pacer, 144.0, 10.0);
        let ran = runs(&frames);
        assert!(ran.len().abs_diff(600) <= 2, "{}", ran.len());
        assert!(frames.iter().any(|(_, f)| *f == WebFrame::Skip));
        for w in ran.windows(2) {
            let gap = (w[1] - w[0]) * 144.0;
            assert!(near(gap, gap.round(), 1e-6) && gap.round() >= 2.0, "{gap}");
        }
        // A rolling average of 13.9 ms and 20.8 ms in turn.
        assert!(pacer.pacing().fps.abs_diff(60) <= 1, "{:?}", pacer.pacing());
        // At 120 Hz, every other frame.
        let mut pacer = self::pacer(FrameCap::Fps(60));
        let frames = web(&mut pacer, 120.0, 1.0);
        let kinds: Vec<_> = frames.iter().map(|(_, f)| *f).collect();
        for pair in kinds[2..].chunks(2) {
            assert_eq!(pair, [WebFrame::Run, WebFrame::Skip], "{kinds:?}");
        }
    }

    #[test]
    fn on_the_web_a_browser_no_faster_than_the_cap_runs_every_frame() {
        let mut pacer = pacer(FrameCap::Fps(60));
        let frames = web(&mut pacer, 60.0, 2.0);
        assert!(frames.iter().all(|(_, f)| *f == WebFrame::Run));
        let mut pacer = self::pacer(FrameCap::Off);
        let frames = web(&mut pacer, 144.0, 1.0);
        assert!(frames.iter().all(|(_, f)| *f == WebFrame::Run));
    }

    /// The game's time on the web adds up the skipped frames: `App` counts
    /// it from its clock, from the last frame that ran.
    #[test]
    fn on_the_web_the_games_time_adds_up_the_skipped_frames() {
        use shuyi_platform::headless::HeadlessPlatform;
        use shuyi_platform::{App, FrameOutcome, Millis};
        let mut app = App::running(shuyi_ui::toy::game());
        let mut platform = HeadlessPlatform::new();
        let mut pacer = pacer(FrameCap::Fps(30));
        let mut last_run = None;
        for i in 0..60 {
            let now = f64::from(i) * 0.008;
            if pacer.web_frame(now) == WebFrame::Skip {
                continue;
            }
            let before = app.game().map_or(0.0, |g| g.ctx().clock_s);
            let outcome = app.frame(&mut platform, Millis::from_secs_f64(now));
            assert_eq!(outcome, FrameOutcome::Continue);
            let dt = app.game().map_or(0.0, |g| g.ctx().clock_s) - before;
            if let Some(last) = last_run {
                let since: f64 = now - last;
                assert!(near(dt, since, 0.002), "{dt} {since}");
                assert!(since > 0.02, "{since}");
            }
            last_run = Some(now);
        }
    }

    /// A key pressed and released on skipped frames reaches the next frame
    /// that runs, once and in order, before that frame's own.
    #[test]
    fn the_input_of_skipped_frames_reaches_the_next_that_runs_in_order() {
        use shuyi_ui::input::{Chord, Key};
        let down = |key| RawInputEvent::Down(Chord::plain(key));
        let mut held = Held::default();
        held.keep([down(Key::F)], []);
        held.keep(
            [RawInputEvent::Up(Key::F), down(Key::D)],
            [SystemEvent::ToggleFullscreen],
        );
        let mut out = Vec::new();
        held.take_input(&mut out);
        out.push(RawInputEvent::Up(Key::D));
        assert_eq!(
            out,
            [
                down(Key::F),
                RawInputEvent::Up(Key::F),
                down(Key::D),
                RawInputEvent::Up(Key::D)
            ]
        );
        let mut system = Vec::new();
        held.take_system(&mut system);
        assert_eq!(system, [SystemEvent::ToggleFullscreen]);
        // Once: the next frame gets only its own.
        let (mut out, mut system) = (Vec::new(), Vec::new());
        held.take_input(&mut out);
        held.take_system(&mut system);
        assert!(out.is_empty() && system.is_empty());
    }

    #[test]
    fn the_pacers_clock_never_goes_back() {
        let (a, b) = (seconds(), seconds());
        assert!(b >= a && a >= 0.0);
    }

    #[test]
    fn a_precise_sleep_ends_at_its_time() {
        let clock = std::time::Instant::now();
        let now = || clock.elapsed().as_secs_f64();
        sleep(0.004, now);
        assert!(now() >= 0.004);
        let before = now();
        sleep(0.0, now);
        sleep(-1.0, now);
        assert!(now() - before < 0.001);
    }

    #[test]
    fn frames_a_second_are_rounded_from_the_interval() {
        assert_eq!(fps(1.0 / 60.0), 60);
        assert_eq!(fps(0.0), 0);
        assert_eq!(fps(1e-9), u16::MAX);
    }
}
