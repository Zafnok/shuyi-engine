//! The frame cap: the most frames a second the platform shows
//! ([`FrameCap`]), 60 unless the game or the player says otherwise, so a
//! simple game doesn't run as fast as it can and keep a CPU core busy
//! (fans, heat, the battery).
//!
//! The cap is the player's setting
//! ([`Settings::frame_cap`](crate::settings::Settings::frame_cap)), whose
//! default and choices are the game's
//! ([`GameSettings::DEFAULT_FRAME_CAP`](crate::settings::GameSettings::DEFAULT_FRAME_CAP),
//! [`FRAME_CAP_CHOICES`](crate::settings::GameSettings::FRAME_CAP_CHOICES)).
//! Only the platform can apply it, so a change is a request it confirms:
//! every change of the cap takes the next serial number, the platform is
//! handed the cap with its serial each frame, and before each frame it
//! reports what it applied ([`Pacing`], through
//! [`Game::set_pacing`](crate::Game::set_pacing)). A game that asks for a
//! cap ([`Ctx::request_frame_cap`](crate::Ctx::request_frame_cap)) gets a
//! [`CapRequest`] and checks it like a promise
//! ([`Ctx::frame_cap_status`](crate::Ctx::frame_cap_status)): pending until
//! the platform reports that serial, then applied, with the report.
//!
//! **With vsync and variable refresh (VRR).** The platform never waits
//! twice: when vsync, the display or a slow machine already keeps the
//! frames as far apart as the cap asks, it doesn't wait at all, so a cap
//! of 60 on a 60 Hz display with vsync on never misses a vblank. On a VRR
//! display (G-Sync, `FreeSync`, adaptive sync), a cap a few frames under the
//! display's refresh (141 on a 144 Hz display) keeps it in its variable
//! range, so the display follows the frames.
//!
//! **With fixed steps.** A tick stage steps 60 times a second. At a cap
//! above 60 each step shows for whole frames: evenly at 120 and 240, but 2
//! and 3 frames in turn at 144 and 165, which moves unevenly. A cap that
//! is a multiple of 60 avoids it.

use crate::prelude::*;

/// The lowest cap [`FrameCap::fps`] takes, in frames a second. *Tunable.*
pub const MIN_FPS: u16 = 15;

/// The highest cap [`FrameCap::fps`] takes, in frames a second. *Tunable.*
pub const MAX_FPS: u16 = 1000;

/// The choices an options screen offers when the game doesn't give its own
/// ([`GameSettings::FRAME_CAP_CHOICES`](crate::settings::GameSettings::FRAME_CAP_CHOICES)):
/// the common refresh rates, then no cap.
pub const CHOICES: [FrameCap; 7] = [
    FrameCap::Fps(30),
    FrameCap::Fps(60),
    FrameCap::Fps(120),
    FrameCap::Fps(144),
    FrameCap::Fps(165),
    FrameCap::Fps(240),
    FrameCap::Off,
];

/// Text key ([`Ctx::text`](crate::Ctx::text)) of a cap's name, with the
/// number of frames a second as `{n}` (`"{n} FPS"`).
pub const FPS_KEY: &str = "options.value.fps";

/// Text key of no cap's name (`"Unlimited"`).
pub const UNLIMITED_KEY: &str = "options.value.unlimited";

/// The most frames a second the platform shows.
///
/// 60 by default ([`FrameCap::DEFAULT`]): enough for a smooth picture on
/// most displays, and a game that does little uses little of the CPU.
/// Build one with [`FrameCap::fps`], which checks the range; a cap out of
/// the range ([`MIN_FPS`] to [`MAX_FPS`]), set as `Fps(n)` in code or read
/// from a file, is held to it when the platform applies it
/// ([`limit`](Self::limit)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum FrameCap {
    /// No cap: the platform shows frames as fast as vsync, the display or
    /// the machine let it, on purpose.
    Off,
    /// At most this many frames a second.
    Fps(u16),
}

impl Default for FrameCap {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl FrameCap {
    /// The engine's default: 60 frames a second.
    pub const DEFAULT: Self = Self::Fps(60);

    /// A cap of `n` frames a second; out of [`MIN_FPS`] to [`MAX_FPS`] is
    /// [`CapError::OutOfRange`].
    pub const fn fps(n: u16) -> Result<Self, CapError> {
        if n < MIN_FPS || n > MAX_FPS {
            return Err(CapError::OutOfRange);
        }
        Ok(Self::Fps(n))
    }

    /// The most frames a second it allows, held to [`MIN_FPS`] to
    /// [`MAX_FPS`]; `None` for no cap.
    pub fn limit(self) -> Option<u16> {
        match self {
            Self::Off => None,
            Self::Fps(n) => Some(n.clamp(MIN_FPS, MAX_FPS)),
        }
    }

    /// The text key of its name: [`FPS_KEY`] or [`UNLIMITED_KEY`].
    pub const fn key(self) -> &'static str {
        match self {
            Self::Off => UNLIMITED_KEY,
            Self::Fps(_) => FPS_KEY,
        }
    }

    /// Its name on an options screen, in `ctx`'s language: `"60 FPS"`
    /// ([`FPS_KEY`], with the number as `{n}`) or `"Unlimited"`
    /// ([`UNLIMITED_KEY`]).
    pub fn name<G: crate::GameData>(self, ctx: &crate::Ctx<G>) -> String {
        match self {
            Self::Off => ctx.text(UNLIMITED_KEY).to_owned(),
            Self::Fps(n) => ctx.text_with(FPS_KEY, &[("n", &n)]),
        }
    }
}

/// Why [`FrameCap::fps`] refused a cap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum CapError {
    /// Fewer than [`MIN_FPS`] or more than [`MAX_FPS`] frames a second.
    #[error("a frame cap is {MIN_FPS} to {MAX_FPS} frames a second, or off")]
    OutOfRange,
}

/// A request for a frame cap
/// ([`Ctx::request_frame_cap`](crate::Ctx::request_frame_cap)): a handle to
/// check it with ([`Ctx::frame_cap_status`](crate::Ctx::frame_cap_status)),
/// as a promise is, without waiting on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapRequest {
    serial: u32,
}

impl CapRequest {
    /// The request with `serial`, the serial its change took.
    pub(crate) const fn new(serial: u32) -> Self {
        Self { serial }
    }

    /// The serial number its change took: the platform reports it once it
    /// has applied the cap ([`Pacing::serial`]).
    pub const fn serial(self) -> u32 {
        self.serial
    }
}

/// Where a [`CapRequest`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapStatus {
    /// The platform hasn't reported applying it yet. A platform that never
    /// reports (one whose rate is fixed and that doesn't say so) leaves
    /// every request here.
    Pending,
    /// The platform applied it: its report.
    Applied(Pacing),
    /// A later change of the cap (another request, the options screen, a
    /// profile switched to) took the next serial: the cap asked for is no
    /// longer the one in force, whether or not it had applied.
    Superseded,
}

/// What the platform reports of its pacing, before each frame
/// ([`Game::set_pacing`](crate::Game::set_pacing)): the cap it applied and
/// its serial, whether it asked for vsync, and the frames a second it
/// measured. Built with [`Pacing::new`]; the rest is assigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Pacing {
    /// The cap applied.
    pub cap: FrameCap,
    /// The serial the cap came with: the latest the platform was handed.
    pub serial: u32,
    /// Whether the platform asked the display's driver for vsync (the
    /// driver may not give it).
    pub vsync: bool,
    /// The frames a second it measured: a rolling average, 0 before it has
    /// measured.
    pub fps: u16,
}

impl Pacing {
    /// A report of `cap` applied, which came with `serial`: no vsync and
    /// nothing measured until assigned.
    pub const fn new(cap: FrameCap, serial: u32) -> Self {
        Self {
            cap,
            serial,
            vsync: false,
            fps: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cap_is_15_to_1000_frames_a_second() {
        assert_eq!(FrameCap::fps(14), Err(CapError::OutOfRange));
        assert_eq!(FrameCap::fps(1001), Err(CapError::OutOfRange));
        assert_eq!(FrameCap::fps(15), Ok(FrameCap::Fps(15)));
        assert_eq!(FrameCap::fps(1000), Ok(FrameCap::Fps(1000)));
        assert_eq!(FrameCap::default(), FrameCap::Fps(60));
        assert!(CapError::OutOfRange.to_string().contains("15 to 1000"));
    }

    #[test]
    fn a_cap_out_of_range_is_held_to_it() {
        assert_eq!(FrameCap::Off.limit(), None);
        assert_eq!(FrameCap::Fps(60).limit(), Some(60));
        assert_eq!(FrameCap::Fps(0).limit(), Some(MIN_FPS));
        assert_eq!(FrameCap::Fps(5000).limit(), Some(MAX_FPS));
    }

    #[test]
    fn a_cap_is_named_by_its_text_key() {
        assert_eq!(FrameCap::Off.key(), "options.value.unlimited");
        assert_eq!(FrameCap::Fps(60).key(), "options.value.fps");
        assert_eq!(
            (UNLIMITED_KEY, FPS_KEY),
            (FrameCap::Off.key(), FrameCap::Fps(1).key())
        );
    }

    #[test]
    fn the_choices_are_in_range_and_end_with_no_cap() {
        for cap in CHOICES {
            let checked = cap.limit().map(FrameCap::fps);
            assert_eq!(checked, cap.limit().map(|_| Ok(cap)));
        }
        assert_eq!(CHOICES.last(), Some(&FrameCap::Off));
        assert!(CHOICES.contains(&FrameCap::DEFAULT));
    }

    #[test]
    fn a_cap_is_saved_by_name() {
        assert_eq!(ron::to_string(&FrameCap::Fps(60)).unwrap(), "Fps(60)");
        assert_eq!(ron::from_str::<FrameCap>("Off"), Ok(FrameCap::Off));
    }

    #[test]
    fn a_report_starts_unmeasured() {
        let pacing = Pacing::new(FrameCap::Off, 3);
        assert_eq!((pacing.cap, pacing.serial), (FrameCap::Off, 3));
        assert!(!pacing.vsync);
        assert_eq!(pacing.fps, 0);
        assert_eq!(CapRequest::new(4).serial(), 4);
    }
}
