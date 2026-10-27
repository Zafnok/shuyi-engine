//! [`Platform`]: what a backend gives the engine (ADR-0008 §2, ADR-0024),
//! and the plain data that crosses it: the time ([`Millis`]), the
//! display ([`DisplayInfo`]), a [`Frame`] to show and the system's
//! events ([`SystemEvent`]).

use crate::alloc_prelude::*;
use core::time::Duration;

use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageTable;
pub use shuyi_assets::target::Color;
use shuyi_ui::RawInputEvent;
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::frame_cap::FrameCap;
use shuyi_ui::perf::{HeapSnapshot, MemoryReading, MemoryScopes, PerfClock};
use shuyi_ui::storage::Storage;

use crate::app::Card;
use crate::audio::Audio;
use crate::text_input::TextInput;

/// A moment on the backend's clock, in whole milliseconds. Where the clock
/// counts from (the program's start, the device's, 1970) doesn't matter:
/// [`App`](crate::App) only takes differences. A clock that goes backwards
/// counts as no time passing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Millis(pub u64);

impl Millis {
    /// The time `secs` seconds into the clock: a backend whose clock gives
    /// seconds (`miniquad::date::now`). A negative, NaN or endless time is
    /// 0.
    pub fn from_secs_f64(secs: f64) -> Self {
        let ms = Duration::try_from_secs_f64(secs).map_or(0, |d| d.as_millis());
        Self(u64::try_from(ms).unwrap_or(u64::MAX))
    }

    /// The time since `earlier`, or none if `earlier` is later.
    pub fn since(self, earlier: Millis) -> Duration {
        Duration::from_millis(self.0.saturating_sub(earlier.0))
    }

    /// The time in seconds, for a sound player that keeps time in them.
    pub fn as_secs_f64(self) -> f64 {
        Duration::from_millis(self.0).as_secs_f64()
    }

    /// `self` moved on by `elapsed`, to the millisecond below.
    #[must_use]
    pub fn after(self, elapsed: Duration) -> Millis {
        let ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        Self(self.0.saturating_add(ms))
    }
}

/// What a backend's display is (ADR-0008 §2). One screen for now; a
/// device with more (the DS) reports them here once ticket 3023 adds
/// surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct DisplayInfo {
    /// Its width in device pixels.
    pub width: u32,
    /// Its height in device pixels.
    pub height: u32,
    /// How many colours it shows, in a target profile's terms
    /// ([`Color`], ADR-0008 §1): on `OneBit` the frame becomes 1-bit
    /// (ticket 3006).
    pub color: Color,
}

impl DisplayInfo {
    /// A full-colour display of `width` × `height` device pixels.
    pub const fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            color: Color::Rgba8,
        }
    }

    /// The same display in black and white.
    #[must_use]
    pub const fn one_bit(mut self) -> Self {
        self.color = Color::OneBit;
        self
    }
}

/// What a game's frame is painted with: the font atlas and the pictures in
/// the game's data. A backend makes its textures (or 1-bit glyphs) from
/// them the first time it sees them. Build it with [`Look::new`]
/// (ADR-0039).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Look<'a> {
    /// The font atlas the glyph cells are drawn from.
    pub font: &'a FontAtlasDef,
    /// The pictures the sprite items name.
    pub images: &'a ImageTable,
}

impl<'a> Look<'a> {
    /// The look of glyphs from `font` and pictures from `images`.
    #[must_use]
    pub const fn new(font: &'a FontAtlasDef, images: &'a ImageTable) -> Self {
        Self { font, images }
    }
}

/// What a frame shows.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Show<'a> {
    /// The loading card, while the game's data loads: drawn with what the
    /// backend has of its own (its font, its picture decoder), since the
    /// game's look isn't loaded yet.
    Card(&'a Card),
    /// Why the game couldn't start (a broken asset, a wrong command line),
    /// in the backend's own font. It stays until the player quits.
    Errors(&'a str),
    /// The game's console: its cells and pictures in full colour (the
    /// 1-bit frame is ticket 3006's).
    #[non_exhaustive]
    Game {
        /// The cells and the sprite items, as the screens drew them.
        buffer: &'a GlyphBuffer,
        /// The colour behind the console and around it: that of the cell
        /// the frame is cleared to.
        background: Rgb,
        /// What it is painted with; `None` when the game didn't say (an
        /// `App` a test made over a running game).
        look: Option<Look<'a>>,
    },
}

/// One frame for the backend to show ([`Platform::present`]).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Frame<'a> {
    /// What it shows.
    pub show: Show<'a>,
    /// Whether the game should fill the screen (the player's Fullscreen
    /// setting): a windowed backend switches when it changes; a device
    /// that is always full screen ignores it.
    pub fullscreen: bool,
    /// The most frames a second to show (the player's frame cap): a
    /// backend that paces its frames paces the next ones to it, and reports
    /// what it applied before the next frame
    /// ([`App::set_pacing`](crate::App::set_pacing)).
    pub frame_cap: FrameCap,
    /// The serial of the cap's latest change: the backend reports it with
    /// the cap it applied.
    pub frame_cap_serial: u32,
}

impl<'a> Frame<'a> {
    /// A frame showing `show`, windowed, at the engine's default cap
    /// ([`FrameCap::DEFAULT`]) with serial 0.
    pub fn new(show: Show<'a>) -> Self {
        Self {
            show,
            fullscreen: false,
            frame_cap: FrameCap::DEFAULT,
            frame_cap_serial: 0,
        }
    }
}

/// Something the system tells the game, between frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SystemEvent {
    /// The game is about to stop running for a while: the device sleeps or
    /// locks, the system's home menu opens, the app goes to the
    /// background. [`App`](crate::App) lets the game save and stops
    /// stepping it until [`Resume`](Self::Resume).
    Suspend,
    /// The game runs again. [`App`](crate::App) forgets the time it was
    /// away, so the sleep doesn't become a burst of catch-up steps.
    Resume,
    /// The game's window lost the player's attention (another window, an
    /// overlay over it). The game keeps running; its hook may pause it.
    FocusLost,
    /// The game's window has the player's attention again.
    FocusGained,
    /// The player asked the system to close the game (the window's close
    /// button). [`App`](crate::App) lets the game save, then quits.
    QuitRequested,
    /// The battery is nearly empty: the game may save in case the device
    /// turns off.
    LowBattery,
    /// The window's own fullscreen shortcut was pressed (F11, Alt+Enter):
    /// [`App`](crate::App) flips the Fullscreen setting, so the next frame
    /// asks for the other window mode and an options screen shows it.
    ToggleFullscreen,
}

/// What a backend gives the engine: a display to [`present`] frames on,
/// the player's input, the system's events, storage, a sound device and,
/// if the device has one, its own keyboard for text and, in debug builds,
/// a clock to measure frames with.
///
/// The backend owns the loop (macroquad's, the Playdate SDK's update
/// callback, a console's main loop) and calls [`App::frame`] once per
/// frame with its clock; `App` calls these methods from inside it, in the
/// order `docs/platform-backends.md` gives. Only a backend touches a device
/// (`AGENTS.md` rule 5): the engine's other crates see this trait.
///
/// [`present`]: Self::present
/// [`App::frame`]: crate::App::frame
pub trait Platform {
    /// The display, as it is now (a window may be resized between frames).
    fn display(&self) -> DisplayInfo;

    /// Shows `frame`. Called once per frame the game isn't suspended,
    /// after its input, audio and update; the backend shows it as soon
    /// as it can and keeps it on the screen until the next.
    fn present(&mut self, frame: &Frame<'_>);

    /// Adds the keyboard and controller events since the last call to
    /// `out`, in the order they happened: releases before presses within
    /// one poll of a device, as [`RawInputEvent::from_pads`] gives them.
    fn poll_input(&mut self, out: &mut Vec<RawInputEvent>);

    /// Adds the system's events since the last call to `out`, in order.
    fn system_events(&mut self, out: &mut Vec<SystemEvent>);

    /// Hands over the game's storage: its saves, settings and key bindings,
    /// by key. `App` calls it once, as the game loads, and the game's
    /// context keeps it (screens save through it).
    fn storage(&mut self) -> Box<dyn Storage>;

    /// The sound device: it plays each frame's sounds, music and voices
    /// and says what music is sounding.
    fn audio(&mut self) -> &mut dyn Audio;

    /// The system's own keyboard for text (a console's, Steam's on-screen
    /// one), if the device has one; `None` where the player types on keys,
    /// which come through [`poll_input`](Self::poll_input).
    fn text_input(&mut self) -> Option<&mut dyn TextInput>;

    /// A clock in whole microseconds, to measure frames with in debug
    /// builds (ADR-0046): `App` takes it once, on its first frame, and only
    /// with [`DEBUG_TOOLS`](shuyi_ui::DEBUG_TOOLS) on. `None` (the
    /// default) for a backend without one: nothing is measured.
    fn clock(&self) -> Option<PerfClock> {
        None
    }

    /// The device's memory now, in debug builds (ADR-0046): the heap's
    /// live bytes, their peak and the allocations since the last call.
    /// `App` asks once a frame, at the end of a frame it keeps a record of
    /// (debug tools on, with a clock), and puts the heap's counts in the
    /// record. `None` (the default) for a backend that doesn't count:
    /// records have no heap counts.
    fn memory(&mut self) -> Option<MemoryReading> {
        None
    }

    /// The memory tags of the backend's heap counter, in debug builds
    /// (ADR-0046 §6): plain functions over its tags, so what the game
    /// allocates counts by the frame's phase and the game's spans. `App`
    /// takes them with the clock ([`clock`](Self::clock)), once, with
    /// [`DEBUG_TOOLS`](shuyi_ui::DEBUG_TOOLS) on, and moves them into the
    /// game's context when it runs. `None` (the default) for a backend
    /// whose counter has no tags. A backend with them also tags each
    /// asset decode when it starts, before the game's load
    /// (`shuyi_assets::format::watch_decodes`).
    fn memory_scopes(&self) -> Option<MemoryScopes> {
        None
    }

    /// What holds the heap now, by memory tag, in debug builds (ADR-0046
    /// §6): its live bytes and each tag's counts, its `frame` left 0 for
    /// [`App::heap_snapshot`](crate::App::heap_snapshot) to stamp. Taking
    /// one allocates the rows, so it is asked for now and then (a tool, a
    /// trace every 600 frames), never every frame. `None` (the default)
    /// for a backend whose counter has no tags.
    fn heap_snapshot(&mut self) -> Option<HeapSnapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clock_in_seconds_is_whole_milliseconds() {
        assert_eq!(Millis::from_secs_f64(1.2344), Millis(1234));
        assert_eq!(Millis::from_secs_f64(1.7e9), Millis(1_700_000_000_000));
        for nonsense in [-1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(Millis::from_secs_f64(nonsense), Millis(0), "{nonsense}");
        }
        assert!((Millis(1500).as_secs_f64() - 1.5).abs() < f64::EPSILON);
    }

    #[test]
    fn time_never_runs_backwards() {
        assert_eq!(Millis(1500).since(Millis(1000)), Duration::from_millis(500));
        assert_eq!(Millis(1000).since(Millis(1500)), Duration::ZERO);
        assert_eq!(Millis(10).after(Duration::from_micros(2900)), Millis(12));
        assert_eq!(
            Millis(u64::MAX).after(Duration::from_secs(1)),
            Millis(u64::MAX)
        );
    }

    #[test]
    fn a_display_is_full_colour_unless_said() {
        let screen = DisplayInfo::new(400, 240);
        assert_eq!((screen.width, screen.height), (400, 240));
        assert_eq!(screen.color, Color::Rgba8);
        assert_eq!(screen.one_bit().color, Color::OneBit);
        let frame = Frame::new(Show::Errors("x"));
        assert!(!frame.fullscreen);
        assert_eq!(
            (frame.frame_cap, frame.frame_cap_serial),
            (FrameCap::Fps(60), 0)
        );
    }

    #[test]
    fn a_new_look_is_the_literal() {
        let font = FontAtlasDef {
            cell_w: 8,
            cell_h: 8,
            columns: 16,
            glyphs: alloc::collections::BTreeMap::new(),
        };
        let images = ImageTable::default();
        let look = Look::new(&font, &images);
        let literal = Look {
            font: &font,
            images: &images,
        };
        assert!(core::ptr::eq(look.font, literal.font));
        assert!(core::ptr::eq(look.images, literal.images));
    }
}
