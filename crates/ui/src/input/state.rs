//! [`InputState`]: turns key and controller-button presses and releases
//! plus frame time into a game's actions, with repeat for held repeatable
//! keys and buttons (game ADR-0015, game ADR-0034), and button chords (ticket 1034).
//! Generic over the game's [`ActionSet`] (ticket 1106).
//!
//! **How a button chord fires.** A [button chord](ButtonChord) `X+Y` fires
//! when one of its buttons goes down while the other is down and still
//! holding back its own actions ([`Keymap::chord_wait`]): the chord's
//! actions instead of either button's. A button that holds back waits
//! for the chord window (`keymap.ron`'s `chord`) or until it is let go,
//! whichever is first, or, if a partner of its acts on press, until it is
//! let go; it then gives its own actions, unless a chord used it while it
//! was held (then nothing). With the Playdate's `A+B`, `B+←` and `B+→`:
//!
//! - `A` and `B` pressed within the window, in either order, give End turn
//!   and neither Confirm nor Cancel; `B` held, `A` any time gives it too.
//! - `B` held, `←` gives Previous unit, as often as `←` is pressed while
//!   `B` is held, and `←` held repeats it as it repeats the cursor;
//!   letting go of `B` then gives nothing.
//! - `B` tapped (or held) alone gives Cancel when it is let go; `A` alone
//!   gives Confirm after the window, or on release if that is sooner.
//!
//! Buttons in no chord, keys, and every pad whose bindings have no chords
//! act on press exactly as before.

use crate::prelude::*;
use core::time::Duration;
use shuyi_assets::keymap::{ActionSet, Button, ButtonChord, Chord, Key, RepeatDef};

use super::keymap::{ChordWait, Keymap};
use super::pad::{ButtonSet, PadKind};

/// At most this many repeats are emitted by one [`InputState::update`], so a
/// lag spike can't teleport the cursor across the map.
pub const MAX_REPEATS_PER_UPDATE: u64 = 5;

/// What the player is playing with: whatever they pressed last
/// (`docs/design/controls.md`, *Switching between keyboard and
/// controller*). Help text names keys or this pad's buttons to match.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
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

/// A key or button that is down with an action it triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Held<A> {
    input: Input,
    action: A,
    /// Whether holding it re-emits the action.
    repeats: bool,
    /// For a button chord's action: the chord's other button, whose
    /// release ends it too.
    partner: Option<Button>,
}

/// How much longer a [`Waiting`] button holds back its own actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Until {
    /// Until the chord window has passed (or it is let go): time waited,
    /// in microseconds, counted from the first update after the press.
    Window { waited_us: u64, counting: bool },
    /// Until it is let go.
    Release,
}

/// A chord button that is down and holding back its own actions
/// ([`ChordWait`]).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Waiting<A> {
    button: Button,
    /// Its own actions, given when it stops waiting, unless `used`.
    actions: Vec<A>,
    until: Until,
    /// A chord fired with it while it was held: it gives nothing.
    used: bool,
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
/// delay before repeating. Button chords: see the [module docs](self).
#[derive(Debug, Clone)]
pub struct InputState<A: ActionSet> {
    keymap: Keymap<A>,
    /// Actions from presses and releases since the last update.
    pending: Vec<A>,
    /// Bound keys and buttons currently down with the actions they
    /// triggered (one entry each), oldest first.
    held: Vec<Held<A>>,
    pub(super) repeat: Option<Repeat<A>>,
    /// Every controller button down since this keymap was set, bound or
    /// not.
    buttons_down: ButtonSet,
    /// Chord buttons holding back their own actions, oldest first.
    waiting: Vec<Waiting<A>>,
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
            buttons_down: ButtonSet::EMPTY,
            waiting: Vec::new(),
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
    /// keys, pending presses, waiting chord buttons and any running repeat,
    /// so nothing pressed under the old bindings leaks into the new ones; a
    /// key still down is ignored until it is pressed again. The
    /// [`device`](Self::device) stays.
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
        self.release(|h| h.input == Input::Key(key));
    }

    /// A controller button went down, on a pad of `kind`
    /// ([`Pads`](super::pad::Pads) merges the pads and gives the button's
    /// binding position). Unbound buttons and buttons already down are
    /// ignored. A button in a chord may fire the chord, or hold back its
    /// own actions (see the [module docs](self)).
    pub fn pad_down(&mut self, button: Button, kind: PadKind) {
        if self.buttons_down.contains(button) {
            return;
        }
        self.buttons_down.insert(button);
        let device = Device::Pad(kind);
        if self.fire_chord(button, device) {
            return;
        }
        let actions = self.keymap.pad_actions(button).to_vec();
        let until = match self.keymap.chord_wait(button) {
            ChordWait::None => return self.press(Input::Pad(button), &actions, device),
            ChordWait::Window => Until::Window {
                waited_us: 0,
                counting: false,
            },
            ChordWait::Release => Until::Release,
        };
        self.device = device;
        self.waiting.push(Waiting {
            button,
            actions,
            until,
            used: false,
        });
    }

    /// The controller button `button`, just pressed, was taken by the game
    /// for itself (it ended a "press any button" wait), so it does nothing
    /// else: if it is holding back its own actions for a chord, it gives
    /// none of them. (A button that acts on press has given its actions
    /// already, in this frame's update, for the game to ignore.)
    pub fn pad_taken(&mut self, button: Button) {
        if let Some(w) = self.waiting.iter_mut().find(|w| w.button == button) {
            w.used = true;
            w.until = Until::Release;
        }
    }

    /// Fires the chord of `button` (just pressed) and the most recently
    /// pressed waiting button it has a chord with, if any: true if one
    /// fired.
    fn fire_chord(&mut self, button: Button, device: Device) -> bool {
        let keymap = &self.keymap;
        let chord_of = |w: &Waiting<A>| ButtonChord::pair(button, w.button);
        let found = self.waiting.iter_mut().rev().find_map(|w| {
            let actions = keymap.chord_actions(chord_of(w)?);
            (!actions.is_empty()).then(|| (w, actions.to_vec()))
        });
        let Some((partner, actions)) = found else {
            return false;
        };
        // The partner gives nothing now, but stays a partner while held.
        partner.used = true;
        partner.until = Until::Release;
        let partner = partner.button;
        // Pressed with a button that repeats (the D-pad), the chord repeats
        // as that button would.
        let own = self.keymap.pad_actions(button);
        let repeats = own.iter().any(|a| a.is_repeatable());
        self.device = device;
        self.hold(Input::Pad(button), Some(partner), &actions, |a| {
            repeats || a.is_repeatable()
        });
        true
    }

    /// A controller button went up. A waiting chord button gives its own
    /// actions now, unless a chord used it. Stops its repeat, if it was the
    /// repeating one, and ends the chords it is in.
    pub fn pad_up(&mut self, button: Button) {
        self.buttons_down.remove(button);
        if let Some(i) = self.waiting.iter().position(|w| w.button == button) {
            let waited = self.waiting.remove(i);
            if !waited.used {
                self.pending.extend(waited.actions);
            }
        }
        // Its own actions, and those of the chords it is the partner in.
        self.release(|h| h.input == Input::Pad(button) || h.partner == Some(button));
    }

    /// `input` went down on `device`, bound to `actions` (none if
    /// unbound).
    fn press(&mut self, input: Input, actions: &[A], device: Device) {
        if actions.is_empty() || self.held.iter().any(|h| h.input == input) {
            return;
        }
        self.device = device;
        self.hold(input, None, actions, A::is_repeatable);
    }

    /// Emits `actions` for `input`'s press and holds them while it (and a
    /// chord's `partner`) is down; those `repeats` says yes to repeat.
    fn hold(
        &mut self,
        input: Input,
        partner: Option<Button>,
        actions: &[A],
        repeats: impl Fn(A) -> bool,
    ) {
        for &action in actions {
            let repeats = repeats(action);
            self.held.push(Held {
                input,
                action,
                repeats,
                partner,
            });
            self.pending.push(action);
            if repeats {
                self.repeat = Some(Repeat::start(input, action));
            }
        }
    }

    /// Lets go of the held actions that `gone` picks. If the repeating one
    /// is among them, repeating passes to the most recent repeating one
    /// still held.
    fn release(&mut self, gone: impl Fn(&Held<A>) -> bool) {
        let repeating = |h: &Held<A>| {
            self.repeat
                .is_some_and(|r| (r.input, r.action) == (h.input, h.action))
        };
        let stops = self.held.iter().any(|h| gone(h) && repeating(h));
        self.held.retain(|h| !gone(h));
        if stops {
            self.repeat = self
                .held
                .iter()
                .rev()
                .find(|h| h.repeats)
                .map(|h| Repeat::start(h.input, h.action));
        }
    }

    /// Every held key or button with the action it holds, oldest first.
    #[cfg(test)]
    pub(super) fn held_pairs(&self) -> Vec<(Input, A)> {
        self.held.iter().map(|h| (h.input, h.action)).collect()
    }

    /// Whether any held key or button is bound to `action` (e.g. hold to
    /// fast-forward). A chord button holding back its action doesn't hold
    /// it yet.
    pub fn is_held(&self, action: A) -> bool {
        self.held.iter().any(|h| h.action == action)
    }

    /// Advances time by `dt` seconds and returns the actions to handle this
    /// frame, in order: chord buttons whose window has just passed, presses
    /// and releases since the last update, then repeats (at most
    /// [`MAX_REPEATS_PER_UPDATE`]; repeats beyond the cap are dropped, not
    /// deferred). Negative or non-finite `dt` counts as zero.
    pub fn update(&mut self, dt: f32) -> Vec<A> {
        let dt_us = seconds_to_micros(dt);
        let mut actions = self.windows_passed(dt_us);
        actions.append(&mut self.pending);
        let RepeatDef {
            delay_ms,
            interval_ms,
        } = self.keymap.repeat();
        if let Some(r) = &mut self.repeat {
            r.held_us = r.held_us.saturating_add(dt_us);
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

    /// Counts `dt_us` towards the chord window of the buttons waiting for
    /// it (from the first update after each press, so a long frame never
    /// eats the window of a button pressed during it) and gives the own
    /// actions of those whose window has passed, which are held from then
    /// on. Their actions never repeat: a button whose own action repeats
    /// never waits.
    fn windows_passed(&mut self, dt_us: u64) -> Vec<A> {
        let window_us = u64::from(self.keymap.chord().window_ms) * 1000;
        let mut passed = Vec::new();
        self.waiting.retain_mut(|w| {
            let Until::Window {
                waited_us,
                counting,
            } = &mut w.until
            else {
                return true;
            };
            if *counting {
                *waited_us = waited_us.saturating_add(dt_us);
            }
            *counting = true;
            if *waited_us < window_us {
                return true;
            }
            passed.push((w.button, core::mem::take(&mut w.actions)));
            false
        });
        let mut actions = Vec::new();
        for (button, own) in passed {
            for action in own {
                self.held.push(Held {
                    input: Input::Pad(button),
                    action,
                    repeats: false,
                    partner: None,
                });
                actions.push(action);
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
