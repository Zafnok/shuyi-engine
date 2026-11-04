//! A stage's rules in fixed steps (ADR-0003 §2): [`TickDriver`] steps a
//! [`TickSim`] as many times as a frame has steps due
//! ([`FrameInput::ticks`]), gives each step what the player did since the
//! last one, and hands each step's events on once. [`ActionIds`] turns a
//! game's action into the id a step's input holds, and back.
//!
//! A tick stage reads input from its steps' [`TickInput`], never from
//! [`FrameInput::actions`]: frames and steps don't line up (a 120 Hz
//! display runs a step every other frame; a late frame runs up to 4), so a
//! press read straight from a frame would be lost on a frame with no step
//! and seen again on a frame with several, and a key's repeats would count
//! as presses.
//!
//! ```
//! use shuyi_core::tick::TickRecord;
//! use shuyi_core::toy::{Dot, DotEvent};
//! use shuyi_ui::input::sample::Action;
//! use shuyi_ui::screen::FrameInput;
//! use shuyi_ui::tick::TickDriver;
//! # let made = shuyi_core::Made::new("dot", shuyi_core::ContentHash(0));
//!
//! let mut dot = Dot::default();
//! let mut driver = TickDriver::new().recording(TickRecord::new(made, 1));
//! // A tap on a frame with no step due: kept for the next step.
//! let tap = FrameInput::new(vec![Action::CursorRight], 0.004, vec![]);
//! assert!(driver.update(&mut dot, &tap).is_empty());
//! // Two steps due: the tap moves the dot in the first, and it stops in
//! // the second.
//! let late = FrameInput::new(vec![], 0.033, vec![]).with_ticks(2);
//! let events = driver.update(&mut dot, &late);
//! assert_eq!(events, [(2, DotEvent::Stopped { x: 1, y: 0 })]);
//! assert_eq!(driver.record().map(TickRecord::len), Some(2));
//! ```

use crate::prelude::*;

use shuyi_assets::keymap::ActionSet;
use shuyi_core::tick::{ActionId, SeatInput, TickInput, TickRecord, TickSim};

use crate::input::frame::FrameInput;

/// A game's action as a step's input holds it ([`ActionId`]): its place
/// in [`ActionSet::ALL`]. Every [`ActionSet`] has it.
pub trait ActionIds: ActionSet {
    /// Its id: its place in [`ActionSet::ALL`], from 0 (`u16::MAX` for an
    /// action that isn't there, or past it).
    fn id(self) -> ActionId;

    /// The action whose id is `id`, if there is one.
    fn from_id(id: ActionId) -> Option<Self>;
}

impl<A: ActionSet> ActionIds for A {
    fn id(self) -> ActionId {
        let place = Self::ALL.iter().position(|&action| action == self);
        ActionId(
            place
                .and_then(|p| u16::try_from(p).ok())
                .unwrap_or(u16::MAX),
        )
    }

    fn from_id(id: ActionId) -> Option<Self> {
        Self::ALL.get(usize::from(id.0)).copied()
    }
}

/// An action going down or coming up, waiting for a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    /// It went down.
    Press,
    /// It came up.
    Release,
}

/// What one seat did since the steps last took it: each press and release
/// read and not yet given to a step, in order.
#[derive(Debug, Clone)]
struct Latch<A> {
    /// The edges not given to a step yet, in the order they came.
    queue: Vec<(A, Edge)>,
    /// The actions whose last edge read is a press.
    down: Vec<A>,
    /// The actions whose last edge given to a step is a press.
    given: Vec<A>,
    /// The actions held at the end of the last frame read.
    held: Vec<A>,
}

impl<A: ActionSet> Latch<A> {
    fn new() -> Self {
        Self {
            queue: Vec::new(),
            down: Vec::new(),
            given: Vec::new(),
            held: Vec::new(),
        }
    }

    /// Reads one frame's `presses`, `released` actions and the actions
    /// `held` at its end. An action's presses and releases alternate for
    /// the steps: a press of an action already down (another of its keys)
    /// and a release of one whose press no step saw (held since before the
    /// stage) are dropped, and an action down but no longer held is let
    /// go.
    fn read(&mut self, presses: &[A], released: &[A], held: &[A]) {
        // The actions this frame moved, in the order they came, then those
        // down that it didn't name.
        let mut actions: Vec<A> = Vec::new();
        for &action in presses.iter().chain(released).chain(&self.down) {
            if !actions.contains(&action) {
                actions.push(action);
            }
        }
        let mut lists: Vec<(A, Vec<Edge>)> = Vec::new();
        for action in actions {
            let edges = self.edges(action, presses, released, held);
            if !edges.is_empty() {
                lists.push((action, edges));
            }
        }
        // Round by round, each action's first edge before any one's second,
        // so presses keep their order across actions.
        let rounds = lists.iter().map(|(_, edges)| edges.len()).max();
        for round in 0..rounds.unwrap_or(0) {
            for (action, edges) in &lists {
                if let Some(&edge) = edges.get(round) {
                    self.queue.push((*action, edge));
                }
            }
        }
        self.held = held.to_vec();
    }

    /// `action`'s edges in a frame of `presses` and `released` that ends
    /// with `held`, in order; it notes whether the action is down after
    /// them.
    fn edges(&mut self, action: A, presses: &[A], released: &[A], held: &[A]) -> Vec<Edge> {
        let count = |list: &[A]| list.iter().filter(|&&a| a == action).count();
        let mut ups = count(released);
        let (was_held, is_held) = (self.held.contains(&action), held.contains(&action));
        // The presses that put it down: one for each release, one more if
        // it ends held and one less if it started held. Any other is
        // another key of an action that was down already.
        let mut downs = (ups + usize::from(is_held))
            .saturating_sub(usize::from(was_held))
            .min(count(presses));
        let mut seen = self.down.contains(&action);
        let mut down = was_held || seen;
        let mut edges = Vec::new();
        loop {
            if down {
                if ups == 0 {
                    break;
                }
                ups -= 1;
                // A release of a press no step saw is dropped.
                if core::mem::take(&mut seen) {
                    edges.push(Edge::Release);
                }
            } else {
                if downs == 0 {
                    break;
                }
                downs -= 1;
                edges.push(Edge::Press);
                seen = true;
            }
            down = !down;
        }
        if seen && !is_held {
            edges.push(Edge::Release);
            seen = false;
        }
        self.down.retain(|&a| a != action);
        if seen {
            self.down.push(action);
        }
        edges
    }

    /// Gives one step its presses and releases, in order: for each action,
    /// a release of a press an earlier step had, then a press. Whatever an
    /// action has beyond waits for the next step, so nothing is lost: a
    /// press always lands in a step before its release's, and an action
    /// pressed again before a step shows its release and the press again
    /// in the next.
    fn step(&mut self) -> (Vec<A>, Vec<A>) {
        let mut pressed: Vec<A> = Vec::new();
        let mut released: Vec<A> = Vec::new();
        let mut kept = Vec::new();
        for (action, edge) in self.queue.drain(..) {
            let pressed_now = pressed.contains(&action);
            match edge {
                Edge::Release if !pressed_now && !released.contains(&action) => {
                    released.push(action);
                    self.given.retain(|&a| a != action);
                }
                Edge::Press if !pressed_now => {
                    pressed.push(action);
                    self.given.push(action);
                }
                Edge::Release | Edge::Press => kept.push((action, edge)),
            }
        }
        self.queue = kept;
        (pressed, released)
    }

    /// Forgets every press and release no step has had yet. A press a step
    /// had stays down, so its release still comes.
    fn clear(&mut self) {
        self.queue.clear();
        self.down.clone_from(&self.given);
    }
}

/// Runs a stage's [`TickSim`] in fixed steps: as many a frame as
/// [`FrameInput::ticks`] says are due (counted by the platform's `App`, 60
/// a second, at most 4 a frame). It keeps no clock of its own.
///
/// # Input reaches the steps
///
/// It keeps what each frame read until a step takes it, so the frame rate
/// never changes which presses a simulation sees:
///
/// - **A press is pressed in one step**: the first run in or after the
///   frame that read it, the first of a frame's steps when it has several.
///   A frame that runs no step (less than a step's time passed, an action
///   handed in by [`Game::act`](crate::Game::act), a slowed or stopped
///   game) keeps it for the next.
/// - **A release is released in one step**, after its press's: a tap
///   between two steps is pressed in one and released in the next, so it
///   is never lost.
/// - **Held is the frame's**: every step a frame runs sees the actions
///   held at its end. A tap is pressed without being held.
/// - **Key repeats never reach a step** ([`FrameInput::presses`]).
/// - **An action pressed twice before a step** isn't merged: the second
///   press waits. The next step has the first press, the one after the
///   release and the press again, and so on, so every press and release
///   reaches a step, in order. An action is pressed when it goes down: a
///   second key on an action already held presses nothing.
/// - **Covered or resumed, it forgets.** A stage calls
///   [`clear`](Self::clear) from [`Screen::covered`], and a frame after a
///   suspend ([`FrameInput::resumed`]) clears it, so a press made before a
///   pause screen or a suspend never fires after it. A press a step had
///   stays down, and its release still comes.
///
/// Each step's input is one seat, seat 0, from the game's input. With a
/// [`recording`](Self::recording) it is kept in a [`TickRecord`]: what each
/// step saw, so the record replays the same whatever frame rate made it.
///
/// # Events are handed on once
///
/// [`update`](Self::update) hands on each step's events with the step's
/// number (counted from 1), in order, a frame catching up included. It
/// keeps the highest step it has handed on: after a rewind
/// ([`rewind_to`](Self::rewind_to)), the steps that run again are stepped
/// for their state, but their events aren't handed on a second time. A
/// simulation asks for sound only through its events, so no sound plays
/// twice.
///
/// [`Screen::covered`]: crate::Screen::covered
#[derive(Debug, Clone)]
pub struct TickDriver<A: ActionSet> {
    latch: Latch<A>,
    /// The steps run: the number of the last.
    steps: u64,
    /// The highest step whose events were handed on.
    handed: u64,
    /// What each step saw, while recording.
    record: Option<TickRecord>,
}

impl<A: ActionSet> TickDriver<A> {
    /// A driver that has run no step, and records nothing.
    pub fn new() -> Self {
        Self {
            latch: Latch::new(),
            steps: 0,
            handed: 0,
            record: None,
        }
    }

    /// The same driver recording each step's input from now on into
    /// `record` ([`record`](Self::record)).
    #[must_use]
    pub fn recording(mut self, record: TickRecord) -> Self {
        self.record = Some(record);
        self
    }

    /// Reads `input`'s presses and releases and runs `input.ticks()` steps
    /// of `sim`, each with its seat's [`TickInput`] ([type docs](Self)).
    /// Gives back the events of the steps not handed on before, each with
    /// its step's number, in order.
    pub fn update<S: TickSim>(
        &mut self,
        sim: &mut S,
        input: &FrameInput<A>,
    ) -> Vec<(u64, S::Event)> {
        if input.resumed() {
            self.latch.clear();
        }
        self.latch
            .read(input.presses(), input.released(), input.held());
        let mut handed = Vec::new();
        for _ in 0..input.ticks() {
            let (pressed, released) = self.latch.step();
            let ids = |actions: &[A]| actions.iter().map(|&a| a.id()).collect();
            let seat = SeatInput::new(ids(&pressed), ids(&self.latch.held), ids(&released));
            let tick = TickInput::one(seat);
            let events = sim.step(&tick);
            if let Some(record) = &mut self.record {
                record.push(tick);
            }
            self.steps += 1;
            if self.steps > self.handed {
                self.handed = self.steps;
                let step = self.steps;
                handed.extend(events.into_iter().map(|event| (step, event)));
            }
        }
        handed
    }

    /// Forgets every press and release no step has had yet: from
    /// [`Screen::covered`](crate::Screen::covered), so a press made just
    /// before a pause screen never fires after it.
    pub fn clear(&mut self) {
        self.latch.clear();
    }

    /// Sets the steps run back to `step`, when the game has loaded its state
    /// at that step (a rollback): the steps after it run again for their
    /// state, and their events, handed on before, aren't handed on again.
    /// The record forgets the steps after it. A `step` at or past the steps
    /// run changes nothing.
    pub fn rewind_to(&mut self, step: u64) {
        if step >= self.steps {
            return;
        }
        self.steps = step;
        if let Some(record) = &mut self.record {
            record.truncate(step);
        }
    }

    /// The steps run: the number of the last one.
    pub fn steps(&self) -> u64 {
        self.steps
    }

    /// The highest step whose events were handed on.
    pub fn handed(&self) -> u64 {
        self.handed
    }

    /// The record of each step's input, while recording.
    pub fn record(&self) -> Option<&TickRecord> {
        self.record.as_ref()
    }

    /// Stops recording, handing back the record.
    pub fn take_record(&mut self) -> Option<TickRecord> {
        self.record.take()
    }
}

impl<A: ActionSet> Default for TickDriver<A> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
