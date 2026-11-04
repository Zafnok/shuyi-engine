//! A toy [`TickSim`] for the engine's tests (ticket 2137): a [`Dot`] that
//! moves a pixel a step while a direction is held, one more pixel a step
//! for every 8 steps it is held, up to 4, and stops when let go. Stopping
//! is its one event, [`DotEvent::Stopped`]. Integers only.
//!
//! Its directions are the first four actions of the sample game
//! (`shuyi_assets::keymap::sample`), by their ids: [`LEFT`], [`DOWN`],
//! [`UP`] and [`RIGHT`].
//!
//! Built for this crate's tests and, with the `toy` feature, for other
//! engine crates' tests.

use alloc::vec::Vec;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::grid::Dir;
use crate::hash::state_hash;
use crate::tick::{ActionId, TickInput, TickSim};
use crate::timer::Every;

/// The sample game's `CursorLeft`.
pub const LEFT: ActionId = ActionId(0);
/// The sample game's `CursorDown`.
pub const DOWN: ActionId = ActionId(1);
/// The sample game's `CursorUp`.
pub const UP: ActionId = ActionId(2);
/// The sample game's `CursorRight`.
pub const RIGHT: ActionId = ActionId(3);

/// The directions, by their actions, in the order a held one is picked.
const DIRS: [(ActionId, Dir); 4] = [
    (LEFT, Dir::Left),
    (DOWN, Dir::Down),
    (UP, Dir::Up),
    (RIGHT, Dir::Right),
];

/// How many steps a direction is held for each pixel more a step.
pub const FASTER_EVERY: u32 = 8;

/// The most pixels a step.
pub const TOP_SPEED: i32 = 4;

/// What happened in a step of a [`Dot`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DotEvent {
    /// It stopped here, its direction let go.
    Stopped {
        /// Where, across.
        x: i32,
        /// Where, down.
        y: i32,
    },
}

/// A dot on an endless plane, in pixels, and how it moves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dot {
    /// Where it is, across.
    pub x: i32,
    /// Where it is, down.
    pub y: i32,
    /// The way it moves, while it does.
    heading: Option<Dir>,
    /// Pixels a step: 0 while it is still.
    speed: i32,
    /// When it moves a pixel a step faster.
    faster: Every,
}

impl Dot {
    /// A dot standing still at `(x, y)`.
    pub const fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            heading: None,
            speed: 0,
            faster: Every::new(FASTER_EVERY),
        }
    }

    /// The way it moves, while it does.
    pub fn heading(&self) -> Option<Dir> {
        self.heading
    }

    /// Pixels a step: 0 while it is still.
    pub fn speed(&self) -> i32 {
        self.speed
    }

    /// Where it goes in a step given `input`: the first direction pressed
    /// in it, else the one it moves in while that is held, else the first
    /// held.
    fn wants(&self, input: &TickInput) -> Option<Dir> {
        let seat = input.seat(0)?;
        let pressed = seat.pressed.iter().find_map(|&id| dir_of(id));
        let kept = self
            .heading
            .filter(|&dir| DIRS.iter().any(|&(id, d)| d == dir && seat.is_held(id)));
        let held = DIRS
            .iter()
            .find(|&&(id, _)| seat.is_held(id))
            .map(|&(_, dir)| dir);
        pressed.or(kept).or(held)
    }
}

impl Default for Dot {
    /// A dot standing still at `(0, 0)`.
    fn default() -> Self {
        Self::new(0, 0)
    }
}

/// The direction of the action `id`, if it is one.
fn dir_of(id: ActionId) -> Option<Dir> {
    DIRS.iter().find(|&&(a, _)| a == id).map(|&(_, dir)| dir)
}

impl TickSim for Dot {
    type Event = DotEvent;

    fn step(&mut self, input: &TickInput) -> Vec<DotEvent> {
        let Some(dir) = self.wants(input) else {
            if self.heading.take().is_none() {
                return Vec::new();
            }
            self.speed = 0;
            self.faster.reset();
            let (x, y) = (self.x, self.y);
            return [DotEvent::Stopped { x, y }].into();
        };
        if self.heading == Some(dir) {
            let faster = i32::try_from(self.faster.advance(1)).unwrap_or(TOP_SPEED);
            self.speed = self.speed.saturating_add(faster).min(TOP_SPEED);
        } else {
            self.heading = Some(dir);
            self.speed = 1;
            self.faster.reset();
        }
        let (dx, dy) = dir.delta();
        self.x = self.x.saturating_add(dx * self.speed);
        self.y = self.y.saturating_add(dy * self.speed);
        Vec::new()
    }

    fn hash(&self) -> u64 {
        // Integers and an enum: nothing here refuses to serialize.
        state_hash(self).map_or(0, |hash| hash.0)
    }
}

impl fmt::Display for Dot {
    /// `at 14, 0, heading Right at speed 2` (pixels a step), or `at 0, 0,
    /// still`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at {}, {}", self.x, self.y)?;
        match self.heading {
            Some(dir) => write!(f, ", heading {dir:?} at speed {}", self.speed),
            None => f.write_str(", still"),
        }
    }
}
