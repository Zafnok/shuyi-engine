//! [`Driver`]: the body of the SDK's update callback, around
//! [`App::frame`], with the backend's device behind [`Sdk`] so the tests run
//! all of it on the fake.

use alloc::format;

use shuyi_platform::{App, Millis, Start};
use shuyi_ui::GameData;

use crate::backend::Playdate;
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

/// A game on the Playdate: its [`App`] and the backend.
pub struct Driver<G: GameData, S: Sdk> {
    app: App<G>,
    platform: Playdate<S>,
    started: bool,
    clock: Clock,
    fps: Fps,
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
        }
    }

    /// One update of the SDK's: true when the screen changed.
    pub fn update(&mut self) -> bool {
        self.app.set_pacing(self.platform.pacing(self.fps.last));
        let now = self.clock.at(self.platform.sdk().millis());
        // A game that has quit keeps its last frame: the player leaves
        // through the system's Home.
        let _ = self.app.frame(&mut self.platform, now);
        self.fps.frame(now);
        self.after_frame();
        self.platform.take_changed()
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
