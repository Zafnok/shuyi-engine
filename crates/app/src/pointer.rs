//! The mouse and touches (ticket 2413, ADR-0134): reads macroquad's mouse
//! each frame and gives [`RawInputEvent`]'s pointer events, in console
//! pixels of the frame last drawn.
//!
//! The rules are pure, and tested without a window:
//! - **where** ([`console_px`]): macroquad's mouse is in logical pixels;
//!   times the DPI factor it is in the physical pixels the frame is placed
//!   in ([`placement`]), and the placement's inverse gives console pixels;
//! - **what** ([`PointerReader::events`]): the place when it changes, a
//!   press inside the frame and its release wherever it happens, then the
//!   wheel, and a lifted finger leaves;
//! - **how far** ([`Notches`]): the wheel's raw deltas in whole notches.
//!
//! [`read`] is the thin shell that asks macroquad.
//!
//! macroquad turns touches into the left button (its
//! `simulate_mouse_with_touch`, on by default on native and the web), and
//! moves the mouse to the touch, so a tap is a press and a release where
//! the finger was. It reports no pointer leaving the window (miniquad's
//! Windows backend has that code commented out): off the window the
//! pointer stays where it was last, which hover following motion only makes
//! harmless.

use macroquad::input::{
    MouseButton, is_mouse_button_down, is_mouse_button_pressed, is_mouse_button_released,
    mouse_position, mouse_wheel, touches,
};
use macroquad::window::{screen_dpi_scale, screen_height, screen_width};
use shuyi_ui::RawInputEvent;
use shuyi_ui::frame::console::{ConsoleSpec, ScaleMode, frame_of};

use crate::render::placement;

/// One wheel notch in the unit `mouse_wheel().1` comes in on this build's
/// platform, positive away from the player everywhere:
/// - the web: `-deltaY` (`crates/cli/web/mq_js_bundle.js`), in CSS pixels
///   since the bundle never reads `deltaMode` (Firefox gives lines only to
///   a page that does). Measured on 2026-11-22 in Chromium 156 on
///   Windows: 100 a notch (three notches gave one `deltaY` of 300, in
///   pixels). Firefox was not measured; it is said to give about 100 to
///   120 a notch. A trackpad gives small steps;
/// - Windows: `WM_MOUSEWHEEL`'s delta, 120 a notch (miniquad 0.4.11,
///   `src/native/windows.rs`);
/// - macOS: a wheel's lines times 10, and a trackpad's pixels
///   (`src/native/macos.rs`);
/// - Linux, on X11 as `shuyi-app` keeps it: 1 a notch, buttons 4 and 5
///   (`src/native/linux_x11.rs`).
const NOTCH: f32 = if cfg!(target_arch = "wasm32") {
    100.0
} else if cfg!(windows) {
    120.0
} else if cfg!(target_os = "macos") {
    10.0
} else {
    1.0
};

/// What macroquad says of the mouse in one frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Reading {
    /// Where the mouse is, in logical pixels (`mouse_position`).
    pub at: (f32, f32),
    /// The window's size in logical pixels.
    pub window: (f32, f32),
    /// Physical pixels per logical pixel.
    pub dpi: f32,
    /// Whether the left button (or a touch) went down this frame.
    pub pressed: bool,
    /// Whether it went up this frame.
    pub released: bool,
    /// Whether it is down now.
    pub down: bool,
    /// The fingers on the screen, or lifted this frame.
    pub touches: usize,
    /// The wheel's raw vertical delta this frame.
    pub wheel: f32,
}

/// The console pixel of a `spec` frame shown as `mode` under the mouse a
/// `reading` gives, placed as
/// [`Renderer::draw`](crate::render::Renderer::draw) places it; `None` in
/// the letterbox.
pub(crate) fn console_px(
    spec: ConsoleSpec,
    mode: ScaleMode,
    reading: &Reading,
) -> Option<(i32, i32)> {
    let dpi = reading.dpi;
    let physical = (reading.at.0 * dpi, reading.at.1 * dpi);
    placement(spec, mode, reading.window, dpi).frame_px(frame_of(spec), physical)
}

/// The wheel's raw deltas in whole notches of `unit`, the rest kept for
/// the next, so a trackpad's small steps scroll too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Notches {
    unit: f32,
    rest: f32,
}

impl Notches {
    /// Notches of `unit` raw steps (more than 0).
    pub(crate) fn new(unit: f32) -> Self {
        Self { unit, rest: 0.0 }
    }

    /// The whole notches `delta` completes, towards zero; what is left
    /// waits for the next delta.
    pub(crate) fn turn(&mut self, delta: f32) -> i32 {
        if !delta.is_finite() {
            return 0;
        }
        self.rest += delta;
        let whole = (self.rest / self.unit).trunc();
        self.rest -= whole * self.unit;
        // Saturating: no real delta comes near.
        #[allow(clippy::cast_possible_truncation)]
        let notches = whole as i32;
        notches
    }
}

/// What the reader keeps between frames.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PointerReader {
    /// The place last sent, `None` before the first and after it left.
    place: Option<(i32, i32)>,
    /// Whether a press was sent and its release not yet.
    held: bool,
    /// Whether the press held came from a finger.
    touch: bool,
    notches: Notches,
}

impl Default for PointerReader {
    fn default() -> Self {
        Self::new(NOTCH)
    }
}

impl PointerReader {
    /// A reader of a wheel whose notch is `notch` raw steps.
    pub(crate) fn new(notch: f32) -> Self {
        Self {
            place: None,
            held: false,
            touch: false,
            notches: Notches::new(notch),
        }
    }

    /// The pointer's events in a frame `reading` gives, over the last frame
    /// drawn, whose console was `spec` shown as its mode (none before the
    /// first game frame).
    /// In order (ADR-0134 §2): the place, if it changed (`PointerLeft` into
    /// the letterbox), the press, the release, the wheel, and after a
    /// finger's release, `PointerLeft`.
    ///
    /// A press outside the frame is dropped; every press sent gets its
    /// release, wherever it happens, even one macroquad never reported
    /// (the button is up and no release came).
    pub(crate) fn events(
        &mut self,
        drawn: Option<(ConsoleSpec, ScaleMode)>,
        reading: &Reading,
    ) -> Vec<RawInputEvent> {
        let mut out = Vec::new();
        let Some((spec, mode)) = drawn else {
            return out;
        };
        let place = console_px(spec, mode, reading);
        if place != self.place {
            out.push(match place {
                Some((x, y)) => RawInputEvent::PointerMoved { x, y },
                None => RawInputEvent::PointerLeft,
            });
            self.place = place;
        }
        let mut lifted = false;
        let mut up = |reader: &mut Self, out: &mut Vec<RawInputEvent>| {
            out.push(RawInputEvent::PointerUp);
            reader.held = false;
            lifted |= reader.touch;
        };
        // A press held from before, let go before this frame's press.
        if self.held && reading.pressed && reading.released {
            up(self, &mut out);
        }
        if reading.pressed && place.is_some() && !self.held {
            out.push(RawInputEvent::PointerDown);
            self.held = true;
            self.touch = reading.touches > 0;
        }
        if self.held && !reading.down {
            up(self, &mut out);
        }
        let notches = self.notches.turn(reading.wheel);
        if notches != 0 {
            out.push(RawInputEvent::Wheel(notches));
        }
        // A finger has no hover: once lifted, it is nowhere.
        if lifted && !self.held && self.place.is_some() {
            out.push(RawInputEvent::PointerLeft);
            self.place = None;
        }
        out
    }
}

/// This frame's mouse, from macroquad.
pub(crate) fn read() -> Reading {
    Reading {
        at: mouse_position(),
        window: (screen_width(), screen_height()),
        dpi: screen_dpi_scale(),
        pressed: is_mouse_button_pressed(MouseButton::Left),
        released: is_mouse_button_released(MouseButton::Left),
        down: is_mouse_button_down(MouseButton::Left),
        touches: touches().len(),
        wheel: mouse_wheel().1,
    }
}

#[cfg(test)]
mod tests {
    use RawInputEvent::{PointerDown, PointerLeft, PointerUp, Wheel};

    use super::*;

    /// 100 × 32 text cells: 800 × 512 console pixels.
    const WIDE: ConsoleSpec = ConsoleSpec::text(100, 32);

    /// `WIDE`, drawn at a whole scale.
    const DRAWN: Option<(ConsoleSpec, ScaleMode)> = Some((WIDE, ScaleMode::Integer));

    fn moved(x: i32, y: i32) -> RawInputEvent {
        RawInputEvent::PointerMoved { x, y }
    }

    /// The mouse at `at` logical pixels over a 1920 × 1080 window at a DPI
    /// factor of 1: `WIDE` at 2x, bars of 160 left and right, 28 above and
    /// below.
    fn over(at: (f32, f32)) -> Reading {
        Reading {
            at,
            window: (1920.0, 1080.0),
            dpi: 1.0,
            ..Reading::default()
        }
    }

    #[test]
    fn the_mouse_inside_the_frame_is_in_console_pixels() {
        assert_eq!(
            console_px(WIDE, ScaleMode::Integer, &over((160.0, 28.0))),
            Some((0, 0))
        );
        assert_eq!(
            console_px(WIDE, ScaleMode::Integer, &over((161.9, 29.9))),
            Some((0, 0))
        );
        assert_eq!(
            console_px(WIDE, ScaleMode::Integer, &over((162.0, 30.0))),
            Some((1, 1))
        );
        assert_eq!(
            console_px(WIDE, ScaleMode::Integer, &over((1759.0, 1051.0))),
            Some((799, 511))
        );
    }

    #[test]
    fn a_frame_that_fills_the_window_maps_by_its_fraction() {
        // 1280 × 720: `WIDE` at 720 / 512 = 1.40625, 1125 × 720 at (77, 0).
        let filled = |at: (f32, f32)| Reading {
            at,
            window: (1280.0, 720.0),
            dpi: 1.0,
            ..Reading::default()
        };
        let px = |at| console_px(WIDE, ScaleMode::Fit, &filled(at));
        assert_eq!(px((77.0, 0.0)), Some((0, 0)));
        assert_eq!(px((78.5, 1.5)), Some((1, 1)));
        assert_eq!(px((640.0, 360.0)), Some((400, 256)));
        assert_eq!(px((1201.9, 719.9)), Some((799, 511)));
        assert_eq!(px((76.0, 360.0)), None);
        assert_eq!(px((1202.0, 360.0)), None);
        // The same window at a whole scale: 1×, bars of 240 and 104.
        let whole = console_px(WIDE, ScaleMode::Integer, &filled((640.0, 360.0)));
        assert_eq!(whole, Some((400, 256)));
        assert_eq!(
            console_px(WIDE, ScaleMode::Integer, &filled((100.0, 50.0))),
            None
        );
    }

    #[test]
    fn the_letterbox_is_outside_the_frame() {
        for at in [
            (159.0, 500.0),
            (1760.0, 500.0),
            (900.0, 27.0),
            (900.0, 1052.0),
        ] {
            assert_eq!(
                console_px(WIDE, ScaleMode::Integer, &over(at)),
                None,
                "{at:?}"
            );
        }
    }

    #[test]
    fn at_a_dpi_factor_of_1_75_the_mouse_is_placed_in_physical_pixels() {
        // A 2560 × 1440 screen at 175 %: 1462.857… × 822.857… logical
        // pixels. Physically `WIDE` fits at 2x: 1600 × 1024, bars of 480
        // and 208.
        let scaled = |at: (f32, f32)| Reading {
            at,
            window: (2560.0 / 1.75, 1440.0 / 1.75),
            dpi: 1.75,
            ..Reading::default()
        };
        assert_eq!(
            console_px(
                WIDE,
                ScaleMode::Integer,
                &scaled((480.5 / 1.75, 208.5 / 1.75))
            ),
            Some((0, 0))
        );
        // The middle of the window is the middle of the frame.
        assert_eq!(
            console_px(
                WIDE,
                ScaleMode::Integer,
                &scaled((1280.5 / 1.75, 720.5 / 1.75))
            ),
            Some((400, 256))
        );
        // The last physical pixel of the frame, and the first past it.
        let last = (2079.5 / 1.75, 1231.5 / 1.75);
        assert_eq!(
            console_px(WIDE, ScaleMode::Integer, &scaled(last)),
            Some((799, 511))
        );
        assert_eq!(
            console_px(WIDE, ScaleMode::Integer, &scaled((2080.5 / 1.75, 700.0))),
            None
        );
        assert_eq!(
            console_px(WIDE, ScaleMode::Integer, &scaled((479.5 / 1.75, 400.0))),
            None
        );
    }

    #[test]
    fn before_the_first_game_frame_there_is_no_pointer() {
        let mut reader = PointerReader::new(120.0);
        let press = Reading {
            pressed: true,
            down: true,
            wheel: 120.0,
            ..over((900.0, 500.0))
        };
        assert_eq!(reader.events(None, &press), []);
    }

    #[test]
    fn the_place_is_sent_when_its_console_pixel_changes() {
        let mut reader = PointerReader::new(120.0);
        let mut at = |x, y| reader.events(DRAWN, &over((x, y)));
        assert_eq!(at(161.0, 29.0), [moved(0, 0)]);
        assert_eq!(at(161.5, 29.5), [], "the same console pixel");
        assert_eq!(at(165.0, 29.0), [moved(2, 0)]);
        assert_eq!(at(10.0, 10.0), [PointerLeft], "into the letterbox");
        assert_eq!(at(20.0, 10.0), [], "it left once");
        assert_eq!(at(165.0, 29.0), [moved(2, 0)], "back where it was");
    }

    #[test]
    fn a_press_and_a_release_in_one_frame_give_the_place_the_press_the_release() {
        let mut reader = PointerReader::new(120.0);
        let click = Reading {
            pressed: true,
            released: true,
            wheel: -120.0,
            ..over((961.0, 541.0))
        };
        assert_eq!(
            reader.events(DRAWN, &click),
            [moved(400, 256), PointerDown, PointerUp, Wheel(-1)]
        );
    }

    #[test]
    fn a_press_is_held_until_its_release_wherever_that_happens() {
        let mut reader = PointerReader::new(120.0);
        let press = Reading {
            pressed: true,
            down: true,
            ..over((961.0, 541.0))
        };
        assert_eq!(reader.events(DRAWN, &press), [moved(400, 256), PointerDown]);
        let held = Reading {
            down: true,
            ..over((10.0, 10.0))
        };
        assert_eq!(reader.events(DRAWN, &held), [PointerLeft]);
        let release = Reading {
            released: true,
            ..over((10.0, 10.0))
        };
        assert_eq!(
            reader.events(DRAWN, &release),
            [PointerUp],
            "in the letterbox"
        );
    }

    #[test]
    fn a_press_outside_the_frame_is_dropped_and_so_is_its_release() {
        let mut reader = PointerReader::new(120.0);
        let press = Reading {
            pressed: true,
            down: true,
            ..over((10.0, 10.0))
        };
        assert_eq!(reader.events(DRAWN, &press), []);
        let release = Reading {
            released: true,
            ..over((961.0, 541.0))
        };
        assert_eq!(reader.events(DRAWN, &release), [moved(400, 256)]);
    }

    #[test]
    fn a_release_never_reported_is_sent_once_the_button_is_up() {
        let mut reader = PointerReader::new(120.0);
        let press = Reading {
            pressed: true,
            down: true,
            ..over((961.0, 541.0))
        };
        let _ = reader.events(DRAWN, &press);
        assert_eq!(reader.events(DRAWN, &over((961.0, 541.0))), [PointerUp]);
        assert_eq!(reader.events(DRAWN, &over((961.0, 541.0))), []);
    }

    #[test]
    fn a_release_and_a_new_press_in_one_frame_end_the_old_press_first() {
        let mut reader = PointerReader::new(120.0);
        let press = Reading {
            pressed: true,
            down: true,
            ..over((961.0, 541.0))
        };
        let _ = reader.events(DRAWN, &press);
        let again = Reading {
            released: true,
            ..press
        };
        assert_eq!(reader.events(DRAWN, &again), [PointerUp, PointerDown]);
        let up = Reading {
            released: true,
            ..over((961.0, 541.0))
        };
        assert_eq!(reader.events(DRAWN, &up), [PointerUp]);
    }

    #[test]
    fn the_release_of_a_touch_is_followed_by_pointer_left() {
        let mut reader = PointerReader::new(120.0);
        let tap = Reading {
            pressed: true,
            released: true,
            touches: 1,
            wheel: 240.0,
            ..over((961.0, 541.0))
        };
        assert_eq!(
            reader.events(DRAWN, &tap),
            [
                moved(400, 256),
                PointerDown,
                PointerUp,
                Wheel(2),
                PointerLeft
            ]
        );
        // The next tap at the same place comes in again.
        assert_eq!(
            reader.events(DRAWN, &tap),
            [
                moved(400, 256),
                PointerDown,
                PointerUp,
                Wheel(2),
                PointerLeft
            ]
        );
        // A finger held, then lifted a frame later.
        let down = Reading {
            pressed: true,
            down: true,
            touches: 1,
            ..over((161.0, 29.0))
        };
        assert_eq!(reader.events(DRAWN, &down), [moved(0, 0), PointerDown]);
        let lifted = Reading {
            released: true,
            touches: 1,
            ..over((161.0, 29.0))
        };
        assert_eq!(reader.events(DRAWN, &lifted), [PointerUp, PointerLeft]);
    }

    #[test]
    fn a_mouse_release_keeps_its_hover() {
        let mut reader = PointerReader::new(120.0);
        let click = Reading {
            pressed: true,
            released: true,
            ..over((961.0, 541.0))
        };
        let _ = reader.events(DRAWN, &click);
        assert_eq!(reader.events(DRAWN, &over((961.0, 541.0))), []);
    }

    /// The notches of `deltas` one after the other, on a wheel of `unit`.
    fn notches(unit: f32, deltas: &[f32]) -> Vec<i32> {
        let mut wheel = Notches::new(unit);
        deltas.iter().map(|&d| wheel.turn(d)).collect()
    }

    #[test]
    fn the_wheel_turns_in_whole_notches_of_each_platforms_unit() {
        // Windows: 120 a notch, and a fine wheel's 40s.
        assert_eq!(
            notches(120.0, &[120.0, -240.0, 40.0, 40.0, 40.0]),
            [1, -2, 0, 0, 1]
        );
        // Linux on X11: 1 a notch.
        assert_eq!(notches(1.0, &[1.0, -1.0, 3.0]), [1, -1, 3]);
        // macOS: a wheel's lines times 10, a trackpad's pixels.
        assert_eq!(
            notches(10.0, &[10.0, 3.5, 3.5, 3.5, -20.5]),
            [1, 0, 0, 1, -2]
        );
        // The web: about 100 a notch, a trackpad's small steps.
        assert_eq!(
            notches(100.0, &[100.0, -100.0, 30.0, 30.0, 30.0, 30.0]),
            [1, -1, 0, 0, 0, 1]
        );
    }

    #[test]
    fn the_wheels_rest_is_kept_and_turning_back_takes_it_away() {
        assert_eq!(notches(120.0, &[100.0, -100.0, 100.0, 100.0]), [0, 0, 0, 1]);
        assert_eq!(notches(120.0, &[-100.0, -100.0, -30.0]), [0, -1, 0]);
        assert_eq!(notches(120.0, &[f32::NAN, f32::INFINITY, 120.0]), [0, 0, 1]);
    }

    #[test]
    fn this_builds_notch_is_its_platforms() {
        let expected = if cfg!(target_arch = "wasm32") {
            100.0
        } else if cfg!(windows) {
            120.0
        } else if cfg!(target_os = "macos") {
            10.0
        } else {
            1.0
        };
        assert!((NOTCH - expected).abs() < f32::EPSILON);
        assert_eq!(PointerReader::default(), PointerReader::new(expected));
    }
}
