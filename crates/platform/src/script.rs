//! Input scripts: the steps of a player's input a tool plays (keys,
//! controller buttons, an action by name, waits in seconds, the crank, and
//! the debug view's channels turned on) and the one plan that turns them into frames of raw input events
//! ([`frames`]). The Harness plays a [`Script`] (`Script::play`, the
//! `harness` feature), the live link's server plays a `Play` request's
//! steps, and a capture plan's input steps are these (ticket 2566): a key
//! press is written once.
//!
//! A step's form is the same in JSON and in RON: `{"keys": "Enter Up"}`,
//! `{"pad": "South+East"}`, `{"act": "Confirm"}`, `{"wait": 0.5}`,
//! `{"crank": 30}`, `{"debug": "grid"}`; `keys("Enter Up")` and so on in
//! a plan's RON.
//!
//! Every frame is [`FRAME_DT`] long, and a wait is played as
//! [`wait_frames`] of them. How long one wait may be is the player's to
//! say, not the step's: `harness::MAX_WAIT`, `link::MAX_WAIT`, a capture
//! plan's `MAX_WAIT`.

use crate::alloc_prelude::*;
use alloc::{format, vec};
use core::fmt;

use serde::{Deserialize, Serialize};
use shuyi_ui::RawInputEvent;
use shuyi_ui::input::{ActionSet, Button, ButtonChord, Chord, Keymap, PadKind};

/// How long each frame of a script is: a 60th of a second.
pub const FRAME_DT: f32 = 1.0 / 60.0;

/// One step of a script, played in the order written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "kebab-case")]
// A set that grows: a step is added without a break (ADR-0039).
#[non_exhaustive]
pub enum Step {
    /// Each whitespace-separated key chord pressed in one frame and let go
    /// in the next, in turn (`"Enter Up Shift+Space"`).
    Keys(String),
    /// Each whitespace-separated controller button pressed in one frame
    /// and let go in the next, in turn (`"South"`; `"East+DpadDown"`:
    /// those buttons together).
    Pad(String),
    /// The game's action of that name (`Confirm`), pressed by the player's
    /// primary key for it, or its primary button if it has no key.
    Act(String),
    /// That many seconds of frames with no input, 60 a second, rounded up.
    Wait(f32),
    /// The crank wound that many degrees in one frame (backwards is
    /// negative), as a Playdate front end reports it.
    Crank(f32),
    /// Debug tools and the debug view's channels of these comma-separated
    /// ids turned on from here on (`"grid,collision"`; `"all"` turns on
    /// every channel, offered or not), and the frame drawn again: no frame
    /// passes (ADR-0084). Played by the Harness only.
    Debug(String),
}

/// Whether `id` can name a debug view channel in a [`Step::Debug`]: not
/// empty, and only `a`-`z`, `0`-`9` and `_`.
fn is_channel_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

impl Step {
    /// Whether it can be played: names `Chord::parse` and `Button::parse`
    /// know, an action named, a wait that is a number of seconds, 0 or
    /// more, and a crank's number of degrees. An action's name is looked
    /// up when it plays, in the game's keymap; the longest wait is the
    /// player's to check ([`check_waits`]).
    pub fn check(&self) -> Result<(), String> {
        match self {
            Step::Keys(script) => {
                for token in script.split_whitespace() {
                    Chord::parse(token).map_err(|e| format!("keys: {e}"))?;
                }
            }
            Step::Pad(script) => {
                for token in script.split_whitespace() {
                    buttons(token).map_err(|e| format!("pad: {e}"))?;
                }
            }
            Step::Act(name) => {
                if name.trim().is_empty() {
                    return Err("act: no action named".to_owned());
                }
            }
            Step::Wait(seconds) => {
                if !(seconds.is_finite() && *seconds >= 0.0) {
                    return Err(format!(
                        "wait: {seconds} is not a number of seconds, 0 or more"
                    ));
                }
            }
            Step::Crank(degrees) => {
                if !degrees.is_finite() {
                    return Err(format!("crank: {degrees} is not a number of degrees"));
                }
            }
            Step::Debug(channels) => {
                if let Some(bad) = channels
                    .split(',')
                    .map(str::trim)
                    .find(|id| !is_channel_id(id))
                {
                    return Err(format!(
                        "debug: {bad:?} is not a channel's id (a-z, 0-9 and _, comma-separated)"
                    ));
                }
            }
        }
        Ok(())
    }
}

impl fmt::Display for Step {
    /// The step as a plan's RON writes it: `keys("Enter")`, `wait(0.5)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Step::Keys(script) => write!(f, "keys({script:?})"),
            Step::Pad(script) => write!(f, "pad({script:?})"),
            Step::Act(name) => write!(f, "act({name:?})"),
            Step::Wait(seconds) => write!(f, "wait({seconds})"),
            Step::Crank(degrees) => write!(f, "crank({degrees})"),
            Step::Debug(channels) => write!(f, "debug({channels:?})"),
        }
    }
}

/// Steps played in order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
// Built with `Script::new` or `default()`: it may grow (ADR-0039).
#[non_exhaustive]
pub struct Script {
    /// The steps, in order.
    pub steps: Vec<Step>,
}

impl Script {
    /// The script of `steps`, in order.
    pub fn new(steps: Vec<Step>) -> Self {
        Self { steps }
    }

    /// Whether every step can be played ([`Step::check`]); an error names
    /// the step (`step 3: keys: …`).
    pub fn check(&self) -> Result<(), String> {
        for (i, step) in self.steps.iter().enumerate() {
            step.check().map_err(|e| format!("step {}: {e}", i + 1))?;
        }
        Ok(())
    }

    /// Plays the steps in `h`, each frame [`FRAME_DT`] long: the frames of
    /// [`frames`] with the game's keymap and the Harness's pad, and a crank
    /// step with `Harness::crank`.
    ///
    /// # Panics
    ///
    /// On a step [`Step::check`] refuses, an action the game has no key or
    /// button for, and a wait longer than `harness::MAX_WAIT`.
    #[cfg(any(test, feature = "harness"))]
    pub fn play<G: shuyi_ui::GameData>(&self, h: &mut crate::harness::Harness<G>) {
        for (i, step) in self.steps.iter().enumerate() {
            let pad = h.pad_kind();
            let planned = match step {
                Step::Crank(degrees) => step.check().map(|()| {
                    h.crank(*degrees);
                    Vec::new()
                }),
                Step::Debug(channels) => step.check().map(|()| {
                    h.show_debug(channels);
                    Vec::new()
                }),
                // Only an action needs the game: keys play while it loads.
                Step::Act(_) => step_frames(step, h.game().input().keymap(), pad),
                _ => plain_frames(step, pad),
            };
            let planned = planned.unwrap_or_else(|e| panic!("Harness: step {}: {e}", i + 1));
            h.play_planned(&planned);
        }
    }
}

/// Whether every wait of `steps` is at most `max` seconds, the most the
/// player plays in one step; an error names the step.
pub fn check_waits(steps: &[Step], max: f32) -> Result<(), String> {
    for (i, step) in steps.iter().enumerate() {
        if let Step::Wait(seconds) = step
            && *seconds > max
        {
            return Err(format!(
                "step {}: wait: at most {max:.1} seconds in one step, not {seconds}",
                i + 1
            ));
        }
    }
    Ok(())
}

/// How many frames a wait of `seconds` takes: the seconds at 60 a second,
/// rounded up, a product within a thousandth of a whole number counting
/// as that number (0.5 s is 30 frames, 1/60 s is 1, 0.51 s is 31). A wait
/// above 0 is at least one frame; a wait of 0, or one that isn't a number
/// of seconds 0 or more, is none.
pub fn wait_frames(seconds: f32) -> u32 {
    if !(seconds.is_finite() && seconds > 0.0) {
        return 0;
    }
    // Thousandths of a frame, to the nearest: one over a whole number of
    // frames rounds down, two round up. `as` saturates past `u64::MAX`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let thousandths = (f64::from(seconds) * 60_000.0 + 0.5) as u64;
    let frames = thousandths.saturating_add(998) / 1000;
    u32::try_from(frames).unwrap_or(u32::MAX).max(1)
}

/// What presses an action: a key chord, or a button (or two together).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Press {
    /// The action's primary key chord.
    Keys(Chord),
    /// Its primary button, or button chord.
    Pad(ButtonChord),
}

/// What presses the game's action named `name` in `keymap`: its primary
/// key ([`Keymap::primary`]), else its primary button
/// ([`Keymap::primary_button`]). The one lookup of an action by name, so
/// every tool presses an action as a player would. An error names the
/// action: the game has none of that name, or it has neither a key nor a
/// button.
pub fn press_for<A: ActionSet>(keymap: &Keymap<A>, name: &str) -> Result<Press, String> {
    let Some(action) = A::from_name(name) else {
        let names: Vec<&str> = A::ALL.iter().map(|a| a.name()).collect();
        return Err(format!(
            "act: the game has no action {name:?}; its actions are {}",
            names.join(", ")
        ));
    };
    if let Some(chord) = keymap.primary(action) {
        return Ok(Press::Keys(chord));
    }
    keymap
        .primary_button(action)
        .map(Press::Pad)
        .ok_or_else(|| {
            format!("act: {name} has neither a key nor a button in the player's keymap now")
        })
}

/// The names of `keymap`'s actions that have a key or a button, in
/// [`ActionSet::ALL`]'s order: those an [`Step::Act`] can press.
pub fn pressable<A: ActionSet>(keymap: &Keymap<A>) -> Vec<String> {
    A::ALL
        .iter()
        .filter(|&&a| keymap.primary(a).is_some() || keymap.primary_button(a).is_some())
        .map(|a| a.name().to_owned())
        .collect()
}

/// Frames of a script's plan.
#[derive(Debug, Clone, PartialEq)]
// What a plan gives: it may grow (ADR-0039).
#[non_exhaustive]
pub enum Planned {
    /// One frame with these events, in order.
    Events(Vec<RawInputEvent>),
    /// That many frames with no input.
    Idle(u32),
}

/// The frames `steps` give, each [`FRAME_DT`] long, with the game's
/// `keymap` (for an action by name) and buttons pressed on a pad of kind
/// `pad`: a key chord or a pad token down in one frame and up in the next,
/// an action as its key or its button, a wait as [`wait_frames`] frames
/// with no input (none for a wait of 0). An error names the step
/// (`step 3: act: …`): one [`Step::check`] refuses, an action that can't be
/// pressed, or a crank, which isn't played this way (the Harness winds it
/// itself; the live link can't yet).
pub fn frames<A: ActionSet>(
    steps: &[Step],
    keymap: &Keymap<A>,
    pad: PadKind,
) -> Result<Vec<Planned>, String> {
    let mut out = Vec::new();
    for (i, step) in steps.iter().enumerate() {
        let planned = step_frames(step, keymap, pad).map_err(|e| format!("step {}: {e}", i + 1))?;
        out.extend(planned);
    }
    Ok(out)
}

/// The frames of `step`, as [`frames`] gives them; an error doesn't name
/// the step.
fn step_frames<A: ActionSet>(
    step: &Step,
    keymap: &Keymap<A>,
    pad: PadKind,
) -> Result<Vec<Planned>, String> {
    let Step::Act(name) = step else {
        return plain_frames(step, pad);
    };
    step.check()?;
    Ok(match press_for(keymap, name)? {
        Press::Keys(chord) => key_press(chord).to_vec(),
        Press::Pad(chord) => {
            let buttons = match chord.buttons() {
                Some((a, b)) => vec![a, b],
                None => vec![chord.first()],
            };
            pad_press(&buttons, pad).to_vec()
        }
    })
}

/// The frames of `step`, a step that presses no action, as [`frames`]
/// gives them.
fn plain_frames(step: &Step, pad: PadKind) -> Result<Vec<Planned>, String> {
    step.check()?;
    Ok(match step {
        Step::Keys(script) => key_frames(script)?,
        Step::Pad(script) => pad_frames(script, pad)?,
        Step::Wait(seconds) => match wait_frames(*seconds) {
            0 => Vec::new(),
            n => vec![Planned::Idle(n)],
        },
        Step::Act(_) => return Err("act: pressed with the game's keymap".to_owned()),
        Step::Crank(_) => return Err("crank: not over the live link yet".to_owned()),
        Step::Debug(_) => return Err("debug: not over the live link yet".to_owned()),
    })
}

/// The frames of `script`'s key chords, each down in one frame and up in
/// the next; an error is [`Chord::parse`]'s.
pub(crate) fn key_frames(script: &str) -> Result<Vec<Planned>, String> {
    let mut out = Vec::new();
    for token in script.split_whitespace() {
        out.extend(key_press(Chord::parse(token)?));
    }
    Ok(out)
}

/// The frames of `script`'s pad tokens on a pad of kind `pad`, each
/// token's buttons down together in one frame and up in the next; an
/// error is [`Button::parse`]'s.
pub(crate) fn pad_frames(script: &str, pad: PadKind) -> Result<Vec<Planned>, String> {
    let mut out = Vec::new();
    for token in script.split_whitespace() {
        out.extend(pad_press(&buttons(token)?, pad));
    }
    Ok(out)
}

/// The buttons of one pad token, `"South"` or `"South+East"`.
fn buttons(token: &str) -> Result<Vec<Button>, String> {
    token.split('+').map(Button::parse).collect()
}

/// `chord` down in one frame and its key up in the next.
fn key_press(chord: Chord) -> [Planned; 2] {
    [
        Planned::Events(vec![RawInputEvent::Down(chord)]),
        Planned::Events(vec![RawInputEvent::Up(chord.key)]),
    ]
}

/// `buttons` down together on a pad of kind `pad` in one frame, in the
/// order given, and up in the next.
fn pad_press(buttons: &[Button], pad: PadKind) -> [Planned; 2] {
    [
        Planned::Events(
            buttons
                .iter()
                .map(|&b| RawInputEvent::PadDown(b, pad))
                .collect(),
        ),
        Planned::Events(buttons.iter().map(|&b| RawInputEvent::PadUp(b)).collect()),
    ]
}

#[cfg(test)]
mod tests;
