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
//!         act("Confirm"),
//!         capture("moved"),
//!     ],
//!     tolerance: (channel: 2, pixels: 0),
//! )
//! ```
//!
//! Its input steps are a script's ([`script::Step`], ticket 2566), with
//! the same names and meaning as `frame-png`'s, and one more, `capture`,
//! names the frame before it: the one on the screen. Every frame is
//! [`FRAME_DT`](script::FRAME_DT) long.

use crate::alloc_prelude::*;
use alloc::collections::BTreeMap;
use alloc::format;
use core::fmt;

use serde::de::value::{EnumAccessDeserializer, StringDeserializer};
use serde::de::{self, DeserializeSeed, EnumAccess, IntoDeserializer};
use serde::{Deserialize, Deserializer};
use shuyi_ui::RawInputEvent;
use shuyi_ui::frame::raster::Tolerance;
use shuyi_ui::input::{ActionSet, Keymap, PadKind};

use crate::script::{self, Planned, Step, wait_frames};

/// The longest wait of one step, in seconds.
pub const MAX_WAIT: f32 = 600.0;

/// One step of a plan, played in the order written.
#[derive(Debug, Clone, PartialEq)]
// A set that grows: a step is added without a break (ADR-0039).
#[non_exhaustive]
pub enum PlanStep {
    /// Input, as a script plays it: `keys("Enter")`, `act("Confirm")`,
    /// `wait(0.5)`.
    Input(Step),
    /// The frame before it, the one on the screen, compared with the
    /// raster and saved under this name.
    Capture(String),
}

impl fmt::Display for PlanStep {
    /// The step as a plan writes it: `keys("Enter")`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PlanStep::Input(step) => step.fmt(f),
            PlanStep::Capture(name) => write!(f, "capture({name:?})"),
        }
    }
}

impl<'de> Deserialize<'de> for PlanStep {
    /// `capture("name")`, or a script's step as it reads.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_enum("PlanStep", &[], PlanStepVisitor)
    }
}

/// Reads a [`PlanStep`]: its variant's name, then `capture`'s name or the
/// script's step of that name.
struct PlanStepVisitor;

impl<'de> de::Visitor<'de> for PlanStepVisitor {
    type Value = PlanStep;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("a plan's step: keys, pad, act, wait or capture")
    }

    fn visit_enum<E: EnumAccess<'de>>(self, data: E) -> Result<PlanStep, E::Error> {
        let (Ident(name), variant) = data.variant::<Ident>()?;
        if name == "capture" {
            return de::VariantAccess::newtype_variant(variant).map(PlanStep::Capture);
        }
        let again = Named { name, variant };
        Step::deserialize(EnumAccessDeserializer::new(again)).map(PlanStep::Input)
    }
}

/// A variant's name, read as an identifier.
struct Ident(String);

impl<'de> Deserialize<'de> for Ident {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Name;
        impl de::Visitor<'_> for Name {
            type Value = Ident;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a step's name")
            }

            fn visit_str<E: de::Error>(self, name: &str) -> Result<Ident, E> {
                Ok(Ident(name.to_owned()))
            }
        }
        deserializer.deserialize_identifier(Name)
    }
}

/// A variant whose name was read already, read again as a [`Step`]'s.
struct Named<V> {
    name: String,
    variant: V,
}

impl<'de, V: de::VariantAccess<'de>> EnumAccess<'de> for Named<V> {
    type Error = V::Error;
    type Variant = V;

    fn variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<(T::Value, V), V::Error> {
        let name: StringDeserializer<V::Error> = self.name.into_deserializer();
        Ok((seed.deserialize(name)?, self.variant))
    }
}

/// A plan: its steps, checked, and the tolerance its captures are
/// compared within. Made by [`Plan::parse`] alone.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    steps: Vec<PlanStep>,
    tolerance: Tolerance,
}

/// A plan's file, before its steps are checked.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    steps: Vec<PlanStep>,
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
    /// its number and its text: an input step [`Step::check`] refuses, a
    /// wait of more than [`MAX_WAIT`] seconds, a crank (not in a plan
    /// yet), a capture with no frame before it (first, or after waits of 0
    /// only) or none since the last capture, and a capture's name used
    /// twice or with anything but `a`-`z`, `0`-`9`, `-` and `_` (names
    /// become file names). An action's name is looked up when the frames
    /// are planned ([`frames`](Self::frames)).
    pub fn parse(text: &str) -> Result<Plan, String> {
        let file: File = ron::from_str(text).map_err(|e| format!("not a plan: {e}"))?;
        // Each capture's name, and the step that named it.
        let mut names = BTreeMap::new();
        let mut frames = 0u64;
        let mut captured_at = None;
        for (i, step) in file.steps.iter().enumerate() {
            let at = |why: String| format!("step {} ({step}): {why}", i + 1);
            match step {
                PlanStep::Input(input) => {
                    input.check().map_err(at)?;
                    frames += u64::from(input_frames(input).map_err(at)?);
                }
                PlanStep::Capture(name) => {
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
    pub fn steps(&self) -> &[PlanStep] {
        &self.steps
    }

    /// Whether a step presses an action by name, so its frames need the
    /// game's keymap: the one it has once loaded.
    pub fn presses_actions(&self) -> bool {
        self.steps
            .iter()
            .any(|step| matches!(step, PlanStep::Input(Step::Act(_))))
    }

    /// How far a capture may be from the raster's picture and pass.
    pub fn tolerance(&self) -> Tolerance {
        self.tolerance
    }

    /// Every frame of the run, each [`FRAME_DT`](script::FRAME_DT) long: the input steps'
    /// frames as a script's ([`script::frames`]: an action pressed as
    /// `keymap` says, buttons on a generic pad), and each capture's name
    /// on the frame before it. An error names the step: an action `keymap`
    /// can't press.
    pub fn frames<A: ActionSet>(&self, keymap: &Keymap<A>) -> Result<Vec<PlannedFrame>, String> {
        let mut out: Vec<PlannedFrame> = Vec::new();
        for (i, step) in self.steps.iter().enumerate() {
            let input = match step {
                PlanStep::Input(input) => input,
                PlanStep::Capture(name) => {
                    if let Some(last) = out.last_mut() {
                        last.capture = Some(name.clone());
                    }
                    continue;
                }
            };
            let planned = script::frames(core::slice::from_ref(input), keymap, PadKind::default())
                .map_err(|e| format!("step {} ({step}): {}", i + 1, unnumbered(&e)))?;
            for frames in planned {
                match frames {
                    Planned::Events(events) => out.push(PlannedFrame {
                        events,
                        capture: None,
                    }),
                    Planned::Idle(n) => out.extend((0..n).map(|_| PlannedFrame::default())),
                }
            }
        }
        Ok(out)
    }
}

/// `error`, the plan of one step's, without its `step 1: `.
fn unnumbered(error: &str) -> &str {
    error.strip_prefix("step 1: ").unwrap_or(error)
}

/// How many frames the input step `step` takes: two for each key chord or
/// pad token, two for an action, a wait's [`wait_frames`]. An error for a
/// wait of more than [`MAX_WAIT`] seconds, and for a crank.
fn input_frames(step: &Step) -> Result<u32, String> {
    let tokens =
        |script: &str| u32::try_from(script.split_whitespace().count()).unwrap_or(u32::MAX);
    Ok(match step {
        Step::Keys(script) | Step::Pad(script) => tokens(script).saturating_mul(2),
        Step::Act(_) => 2,
        Step::Wait(seconds) => {
            if *seconds > MAX_WAIT {
                return Err(format!(
                    "wait: at most {MAX_WAIT} seconds in one step, not {seconds}"
                ));
            }
            wait_frames(*seconds)
        }
        Step::Crank(_) => return Err("crank: not in a capture plan yet".to_owned()),
    })
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
