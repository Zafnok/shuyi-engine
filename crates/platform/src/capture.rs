//! Plans: the input a backend's capture mode plays, and the frames whose
//! pixels it compares with the raster's (ADR-0052, ticket 2018). The
//! raster (`shuyi_ui::frame::raster`) is the reference picture of a frame;
//! a capture mode (`shuyi-app`'s `capture` feature) runs its game frame by
//! frame as a plan says, paints the frames the plan names with the raster
//! too, and compares the two within the plan's [`Tolerance`].
//!
//! A plan is RON, a test input of the backend, never an asset a game
//! loads:
//!
//! ```ron
//! (
//!     steps: [
//!         wait(0.5),
//!         capture("title"),
//!         keys("Enter Up"),
//!         pad("South+East"),
//!         capture("moved"),
//!     ],
//!     tolerance: (channel: 2, pixels: 0),
//! )
//! ```
//!
//! Its steps are `frame-png`'s, with the same names and meaning, and one
//! more, `capture`, which names the frame before it: the one on the
//! screen. Every frame is [`FRAME_DT`] long.

use crate::alloc_prelude::*;
use alloc::collections::BTreeMap;
use alloc::format;
use core::fmt;

use serde::Deserialize;
use shuyi_ui::RawInputEvent;
use shuyi_ui::frame::raster::Tolerance;
use shuyi_ui::input::Chord;

use crate::press;

/// How long each frame of a plan is: a 60th of a second, as the Harness's
/// frames are.
pub const FRAME_DT: f32 = 1.0 / 60.0;

/// The longest wait of one step, in seconds.
pub const MAX_WAIT: f32 = 600.0;

/// One step of a plan, played in the order written.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
// A set that grows: a step is added without a break (ADR-0039).
#[non_exhaustive]
pub enum Step {
    /// Each whitespace-separated chord (`"Enter Shift+Space"`) pressed in
    /// one frame and let go in the next, as `Harness::keys` does.
    Keys(String),
    /// Each whitespace-separated controller button (`"South"`; buttons
    /// joined by `+` go down together) pressed on a generic pad in one
    /// frame and let go in the next, as `Harness::pad` does.
    Pad(String),
    /// That many seconds of frames with no input ([`wait_frames`]).
    Wait(f32),
    /// The frame before it, the one on the screen, compared with the
    /// raster and saved under this name.
    Capture(String),
}

impl fmt::Display for Step {
    /// The step as a plan writes it: `keys("Enter")`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Step::Keys(script) => write!(f, "keys({script:?})"),
            Step::Pad(script) => write!(f, "pad({script:?})"),
            Step::Wait(seconds) => write!(f, "wait({seconds})"),
            Step::Capture(name) => write!(f, "capture({name:?})"),
        }
    }
}

/// How many frames a wait of `seconds` takes: the seconds at 60 a second,
/// rounded up, a product within a thousandth of a whole number counting
/// as that number (0.5 s is 30 frames, 0.51 s is 31). A wait above 0 is at
/// least one frame; a wait of 0, or one that isn't a number of seconds, is
/// none. At most [`MAX_WAIT`] seconds.
pub fn wait_frames(seconds: f32) -> u32 {
    if !(seconds.is_finite() && seconds > 0.0) {
        return 0;
    }
    let product = seconds.min(MAX_WAIT) * 60.0;
    // 0 to 36 000.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let whole = product as u32;
    #[allow(clippy::cast_precision_loss)] // at most 36 000
    let rest = product - whole as f32;
    let frames = if rest > 0.001 { whole + 1 } else { whole };
    frames.max(1)
}

/// A plan: its steps, checked, and the tolerance its captures are
/// compared within. Made by [`Plan::parse`] alone.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    steps: Vec<Step>,
    tolerance: Tolerance,
}

/// A plan's file, before its steps are checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    steps: Vec<Step>,
    #[serde(default)]
    tolerance: Tolerance,
}

/// One frame of a plan's run.
#[derive(Debug, Clone, PartialEq, Default)]
// Made by `Plan::frames`: it may grow (ADR-0039).
#[non_exhaustive]
pub struct PlannedFrame {
    /// The input the frame gets, in order.
    pub events: Vec<RawInputEvent>,
    /// The capture's name, if the frame is compared with the raster.
    pub capture: Option<String>,
}

impl Plan {
    /// The plan `text` writes (RON), its tolerance
    /// [`Tolerance::default`] unless it says. An error names the step by
    /// its number and its text: a key or button name `Chord::parse` or
    /// `Button::parse` doesn't know, a wait that isn't from 0 to
    /// [`MAX_WAIT`] seconds, a capture with no frame before it (first, or
    /// after waits of 0 only) or none since the last capture, and a
    /// capture's name used twice or with anything but `a`-`z`, `0`-`9`,
    /// `-` and `_` (names become file names).
    pub fn parse(text: &str) -> Result<Plan, String> {
        let file: File = ron::from_str(text).map_err(|e| format!("not a plan: {e}"))?;
        // Each capture's name, and the step that named it.
        let mut names = BTreeMap::new();
        let mut frames = 0u64;
        let mut captured_at = None;
        for (i, step) in file.steps.iter().enumerate() {
            let at = |why: String| format!("step {} ({step}): {why}", i + 1);
            match step {
                Step::Keys(script) => {
                    for token in script.split_whitespace() {
                        Chord::parse(token).map_err(at)?;
                        frames += 2;
                    }
                }
                Step::Pad(script) => {
                    for token in script.split_whitespace() {
                        press::buttons(token).map_err(at)?;
                        frames += 2;
                    }
                }
                Step::Wait(seconds) => {
                    if !(seconds.is_finite() && (0.0..=MAX_WAIT).contains(seconds)) {
                        return Err(at(format!("not a number of seconds from 0 to {MAX_WAIT}")));
                    }
                    frames += u64::from(wait_frames(*seconds));
                }
                Step::Capture(name) => {
                    check_name(name).map_err(at)?;
                    if frames == 0 {
                        return Err(at("no frame comes before it to capture".to_owned()));
                    }
                    if captured_at == Some(frames) {
                        return Err(at(
                            "no frame since the last capture: they would name one frame".to_owned(),
                        ));
                    }
                    if let Some(first) = names.insert(name.as_str(), i + 1) {
                        return Err(at(format!("step {first} already uses that name")));
                    }
                    captured_at = Some(frames);
                }
            }
        }
        Ok(Plan {
            steps: file.steps,
            tolerance: file.tolerance,
        })
    }

    /// Its steps, in order.
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// How far a capture may be from the raster's picture and pass.
    pub fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    /// Every frame of the run, each [`FRAME_DT`] long: a key chord or a
    /// pad token down in one frame and up in the next, as the Harness
    /// presses them; a wait's [`wait_frames`] with no input; and each
    /// capture's name on the frame before it.
    pub fn frames(&self) -> Vec<PlannedFrame> {
        let mut out: Vec<PlannedFrame> = Vec::new();
        let input = |events: [Vec<RawInputEvent>; 2]| {
            events.map(|events| PlannedFrame {
                events,
                capture: None,
            })
        };
        for step in &self.steps {
            match step {
                Step::Keys(script) => {
                    // Checked by `parse`.
                    for chord in script
                        .split_whitespace()
                        .filter_map(|t| Chord::parse(t).ok())
                    {
                        out.extend(input(press::keys(chord)));
                    }
                }
                Step::Pad(script) => {
                    for token in script.split_whitespace() {
                        if let Ok(buttons) = press::buttons(token) {
                            out.extend(input(press::pad(&buttons)));
                        }
                    }
                }
                Step::Wait(seconds) => {
                    let idle = (0..wait_frames(*seconds)).map(|_| PlannedFrame::default());
                    out.extend(idle);
                }
                Step::Capture(name) => {
                    if let Some(last) = out.last_mut() {
                        last.capture = Some(name.clone());
                    }
                }
            }
        }
        out
    }
}

/// Whether `name` can name a capture: not empty, and only `a`-`z`, `0`-`9`,
/// `-` and `_`.
fn check_name(name: &str) -> Result<(), String> {
    let allowed = |c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_';
    if name.is_empty() || !name.chars().all(allowed) {
        return Err(
            "a capture's name is one or more of a-z, 0-9, - and _: it names its files".to_owned(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
