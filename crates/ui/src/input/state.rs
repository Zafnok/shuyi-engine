//! [`InputState`]: turns key and controller-button presses and releases
//! plus frame time into a game's actions, with repeat for held repeatable
//! keys and buttons (ADR-0015, ADR-0034). Generic over the game's
//! [`ActionSet`] (ticket 1106).

use crate::prelude::*;
use core::time::Duration;
use shuyi_assets::keymap::{ActionSet, Button, Chord, Key, RepeatDef};

use super::keymap::Keymap;
use super::pad::PadKind;

/// At most this many repeats are emitted by one [`InputState::update`], so a
/// lag spike can't teleport the cursor across the map.
pub const MAX_REPEATS_PER_UPDATE: u64 = 5;

/// What the player is playing with: whatever they pressed last
/// (`docs/design/controls.md`, *Switching between keyboard and
/// controller*). Help text names keys or this pad's buttons to match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Device {
    /// The keyboard.
    #[default]
    Keyboard,
    /// A controller of this kind.
    Pad(PadKind),
}

/// Something the player can hold down: a keyboard key or a controller
/// button (a stick direction counts as a button).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Input {
    /// A keyboard key.
    Key(Key),
    /// A controller button.
    Pad(Button),
}

/// The repeat currently running for the most recently pressed repeatable
/// key or button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Repeat<A> {
    pub(super) input: Input,
    pub(super) action: A,
    /// Time held so far, in microseconds.
    held_us: u64,
    /// Repeats accounted for so far (emitted or dropped by the cap).
    counted: u64,
}

impl<A> Repeat<A> {
    fn start(input: Input, action: A) -> Self {
        Self {
            input,
            action,
            held_us: 0,
            counted: 0,
        }
    }
}

/// Turns key and controller-button events and elapsed time into actions.
///
/// Feed it every key press ([`key_down`](Self::key_down)) and release
/// ([`key_up`](Self::key_up)), and every controller button press
/// ([`pad_down`](Self::pad_down)) and release ([`pad_up`](Self::pad_up)),
/// then call [`update`](Self::update) once per frame with the frame time.
/// Keys and buttons follow the same rules. A press emits its action once.
/// While a repeatable action's key or button is held, it re-emits after the
/// repeat delay and then every interval. The most recently pressed
/// repeatable one is the one that repeats; releasing it hands repeating
/// back to the most recent repeatable one still held, which waits a full
/// delay before repeating.
#[derive(Debug, Clone)]
pub struct InputState<A: ActionSet> {
    keymap: Keymap<A>,
    /// Actions from presses since the last update.
    pending: Vec<A>,
    /// Bound keys and buttons currently down with the actions they
    /// triggered (one entry each), oldest first.
    pub(super) held: Vec<(Input, A)>,
    pub(super) repeat: Option<Repeat<A>>,
    /// Where the last bound press came from.
    device: Device,
}

impl<A: ActionSet> InputState<A> {
    /// Input handling with the given bindings and repeat timings.
    pub fn new(keymap: Keymap<A>) -> Self {
        Self {
            keymap,
            pending: Vec::new(),
            held: Vec::new(),
            repeat: None,
            device: Device::default(),
        }
    }

    /// Starts as if `device` had been pressed last (a platform whose only
    /// input is its pad).
    #[must_use]
    pub fn starting_on(mut self, device: Device) -> Self {
        self.device = device;
        self
    }

    /// What the player pressed last: the keyboard, or a pad of some kind.
    /// Only presses that do something count (a bound key or button going
    /// down), so an unbound key or a resting stick never switches it.
    /// The keyboard until anything is pressed, unless the game started it
    /// on the platform's own pad.
    pub fn device(&self) -> Device {
        self.device
    }

    /// The bindings in use.
    pub fn keymap(&self) -> &Keymap<A> {
        &self.keymap
    }

    /// Switches to other bindings (the player picked a layout). Forgets held
    /// keys, pending presses and any running repeat, so nothing pressed
    /// under the old bindings leaks into the new ones; a key still down is
    /// ignored until it is pressed again. The [`device`](Self::device)
    /// stays.
    pub fn set_keymap(&mut self, keymap: Keymap<A>) {
        *self = Self {
            device: self.device,
            ..Self::new(keymap)
        };
    }

    /// A key went down (with its modifier state). Unbound chords and keys
    /// already held are ignored.
    pub fn key_down(&mut self, chord: Chord) {
        let actions = self.keymap.actions(chord).to_vec();
        self.press(Input::Key(chord.key), &actions, Device::Keyboard);
    }

    /// A key went down that the game kept for itself (it opened the layout
    /// picker): the keyboard is the [`device`](Self::device) pressed last,
    /// and nothing else happens.
    pub fn keyboard_used(&mut self) {
        self.device = Device::Keyboard;
    }

    /// A key went up. Stops its repeat, if it was the repeating one.
    pub fn key_up(&mut self, key: Key) {
        self.release(Input::Key(key));
    }

    /// A controller button went down, on a pad of `kind`
    /// ([`Pads`](super::pad::Pads) merges the pads and gives the button's
    /// binding position). Unbound buttons and buttons already held are
    /// ignored.
    pub fn pad_down(&mut self, button: Button, kind: PadKind) {
        let actions = self.keymap.pad_actions(button).to_vec();
        self.press(Input::Pad(button), &actions, Device::Pad(kind));
    }

    /// A controller button went up. Stops its repeat, if it was the
    /// repeating one.
    pub fn pad_up(&mut self, button: Button) {
        self.release(Input::Pad(button));
    }

    /// `input` went down on `device`, bound to `actions` (none if
    /// unbound).
    fn press(&mut self, input: Input, actions: &[A], device: Device) {
        if actions.is_empty() || self.held.iter().any(|&(i, _)| i == input) {
            return;
        }
        self.device = device;
        for &action in actions {
            self.held.push((input, action));
            self.pending.push(action);
            if action.is_repeatable() {
                self.repeat = Some(Repeat::start(input, action));
            }
        }
    }

    /// `input` went up.
    fn release(&mut self, input: Input) {
        self.held.retain(|&(i, _)| i != input);
        if self.repeat.is_some_and(|r| r.input == input) {
            self.repeat = self
                .held
                .iter()
                .rev()
                .find(|(_, a)| a.is_repeatable())
                .map(|&(i, a)| Repeat::start(i, a));
        }
    }

    /// Whether any held key or button is bound to `action` (e.g. hold to
    /// fast-forward).
    pub fn is_held(&self, action: A) -> bool {
        self.held.iter().any(|&(_, a)| a == action)
    }

    /// Advances time by `dt` seconds and returns the actions to handle this
    /// frame, in order: presses since the last update, then repeats (at most
    /// [`MAX_REPEATS_PER_UPDATE`]; repeats beyond the cap are dropped, not
    /// deferred). Negative or non-finite `dt` counts as zero.
    pub fn update(&mut self, dt: f32) -> Vec<A> {
        let mut actions = core::mem::take(&mut self.pending);
        let RepeatDef {
            delay_ms,
            interval_ms,
        } = self.keymap.repeat();
        if let Some(r) = &mut self.repeat {
            r.held_us = r.held_us.saturating_add(seconds_to_micros(dt));
            let delay_us = u64::from(delay_ms) * 1000;
            // A zero interval is rejected by the loader; guard anyway.
            let interval_us = u64::from(interval_ms).max(1) * 1000;
            let due = match r.held_us.checked_sub(delay_us) {
                Some(past_delay) => 1 + past_delay / interval_us,
                None => 0,
            };
            let new = due.saturating_sub(r.counted).min(MAX_REPEATS_PER_UPDATE);
            r.counted = due;
            for _ in 0..new {
                actions.push(r.action);
            }
        }
        actions
    }
}

/// Converts seconds to whole microseconds, rounding to nearest. Rounding
/// per frame keeps millisecond-exact timings exact despite `f32` error.
pub(super) fn seconds_to_micros(dt: f32) -> u64 {
    let Ok(d) = Duration::try_from_secs_f32(dt) else {
        return 0;
    };
    u64::try_from(d.as_nanos().saturating_add(500) / 1000).unwrap_or(u64::MAX)
}
