//! The toy game's tick stage (ticket 2137): core's toy [`Dot`], stepped by
//! a [`TickDriver`] that records every step, drawn as an `@` (a pixel a
//! cell, from the console's middle), each stop played as the sound
//! [`STOP`]. Cancel opens the toy's [`Menu`] over it.

use core::any::Any;

use shuyi_core::StateHash;
use shuyi_core::made::{ContentHash, Made};
use shuyi_core::tick::{TickRecord, TickSim};
use shuyi_core::toy::{Dot, DotEvent};

use super::{Action, Menu, Toy, ToyCtx};
use crate::frame::color::EngineColor;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer};
use crate::prelude::*;
use crate::screen::{FrameInput, Screen, Transition};
use crate::tick::TickDriver;

/// The sound a stop plays.
pub const STOP: &str = "stop";

/// The seed its record names: the dot has no luck.
pub const SEED: u64 = 1;

/// The toy's stamp, which its record carries.
pub fn made() -> Made {
    Made::new("toy", ContentHash(0))
}

/// The dot, the driver stepping it, and Cancel to open the menu.
#[derive(Debug)]
pub struct DotStage {
    /// The rules.
    pub dot: Dot,
    /// What steps them, recording each step.
    pub driver: TickDriver<Action>,
}

impl DotStage {
    /// The dot at `(0, 0)`, no step run.
    pub fn new() -> Self {
        Self {
            dot: Dot::default(),
            driver: TickDriver::new().recording(TickRecord::new(made(), SEED)),
        }
    }

    /// What each step saw so far.
    pub fn record(&self) -> &TickRecord {
        // Made recording, and never taken.
        self.driver
            .record()
            .unwrap_or_else(|| panic!("the dot's driver records"))
    }

    /// Back to the dot's state at step `step`, rebuilt from its record's
    /// first `step` steps without a sound, and its driver rewound, so the
    /// steps after it run again without handing their events on twice.
    pub fn rewind_to(&mut self, step: u64) {
        let mut dot = Dot::default();
        let steps = usize::try_from(step).unwrap_or(usize::MAX);
        for input in self.record().inputs().take(steps) {
            dot.step(input);
        }
        self.dot = dot;
        self.driver.rewind_to(step);
    }
}

impl Default for DotStage {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen<Toy> for DotStage {
    fn name(&self) -> &'static str {
        "dot"
    }

    /// The four directions the dot moves in, and Cancel for the menu.
    fn actions(&self) -> Option<Vec<Action>> {
        Some(vec![
            Action::CursorLeft,
            Action::CursorDown,
            Action::CursorUp,
            Action::CursorRight,
            Action::Cancel,
        ])
    }

    fn update(&mut self, ctx: &mut ToyCtx, input: &FrameInput<Action>) -> Transition<Toy> {
        for (_, event) in self.driver.update(&mut self.dot, input) {
            match event {
                DotEvent::Stopped { .. } => ctx.audio.play_sound(STOP),
            }
        }
        if input.actions.contains(&Action::Cancel) {
            return Transition::Push(Box::new(Menu));
        }
        Transition::None
    }

    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let paper = ctx.palette.engine(EngineColor::Panel);
        let ink = ctx.palette.engine(EngineColor::Highlight);
        buf.fill_rect(buf.bounds(), Cell::new(' ', ink, paper));
        let (width, height) = (i32::from(buf.width()), i32::from(buf.height()));
        let x = (width / 2).saturating_add(self.dot.x).clamp(0, width - 1);
        let y = (height / 2).saturating_add(self.dot.y).clamp(0, height - 1);
        buf.set(x, y, Cell::new('@', ink, paper));
    }

    fn state_hash(&self) -> Option<StateHash> {
        Some(StateHash(self.dot.hash()))
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }

    /// A press it kept for a step that hasn't run never fires after the
    /// menu.
    fn covered(&mut self, _: &mut ToyCtx) {
        self.driver.clear();
    }
}
