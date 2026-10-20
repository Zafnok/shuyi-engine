//! [`FrameInput`]: one frame's input as a screen sees it. Generic over the
//! game's [`ActionSet`] (ticket 1106), and [`frame_seconds`], which makes
//! a frame time a platform reports safe to add up.

use crate::prelude::*;
use shuyi_assets::keymap::{ActionSet, Button, Chord};

/// `dt` as seconds that can pass: itself when finite and at least 0, else
/// 0 (a NaN, an infinity or a negative frame time is a platform's
/// nonsense, and no time passes).
pub fn frame_seconds(dt: f32) -> f32 {
    if dt.is_finite() { dt.max(0.0) } else { 0.0 }
}

/// One frame's input as a screen sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameInput<A: ActionSet> {
    /// Actions this frame, in order: key presses, then key repeats.
    pub actions: Vec<A>,
    /// Seconds since the previous frame: finite and at least 0
    /// ([`frame_seconds`]).
    dt: f32,
    /// Actions whose key is currently held down.
    held: Vec<A>,
    /// The chords pressed this frame, in order.
    pressed: Vec<Chord>,
    /// The characters typed this frame, in order.
    text: Vec<char>,
    /// Whether a controller button went down this frame.
    pad: bool,
    /// The controller buttons that went down this frame, in order.
    buttons_down: Vec<Button>,
    /// The controller buttons that went up this frame, in order.
    buttons_up: Vec<Button>,
}

impl<A: ActionSet> FrameInput<A> {
    /// Input for one frame; `held` lists the actions whose keys are down.
    /// No [`pressed_chords`](Self::pressed_chords). A `dt` that is not
    /// finite or is negative counts as 0 ([`frame_seconds`]).
    pub fn new(actions: Vec<A>, dt: f32, held: Vec<A>) -> Self {
        Self {
            actions,
            dt: frame_seconds(dt),
            held,
            pressed: Vec::new(),
            text: Vec::new(),
            pad: false,
            buttons_down: Vec::new(),
            buttons_up: Vec::new(),
        }
    }

    /// The same input with the controller buttons that went `down` and
    /// `up` this frame (and so [`pad_pressed`](Self::pad_pressed) if any
    /// went down).
    #[must_use]
    pub fn with_buttons(mut self, down: Vec<Button>, up: Vec<Button>) -> Self {
        self.pad = !down.is_empty();
        self.buttons_down = down;
        self.buttons_up = up;
        self
    }

    /// Seconds since the previous frame: always finite and at least 0, so
    /// a screen adds it up as it is.
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// The controller buttons that went down this frame (bound or not; by
    /// binding position, stick directions included), for the Key bindings
    /// screen, which captures the button for a slot. Anything else reacts
    /// to [`actions`](Self::actions).
    pub fn pressed_buttons(&self) -> &[Button] {
        &self.buttons_down
    }

    /// The controller buttons that went up this frame, as
    /// [`pressed_buttons`](Self::pressed_buttons).
    pub fn released_buttons(&self) -> &[Button] {
        &self.buttons_up
    }

    /// The same input with whether a controller button went down this
    /// frame.
    #[must_use]
    pub fn with_pad(mut self, pad: bool) -> Self {
        self.pad = pad;
        self
    }

    /// Whether this frame's input came from a controller (a button went
    /// down, or a [button chord](super::ButtonChord)'s button gave its own
    /// action as it was let go): the player is on a controller, so a
    /// screen that needs text shows a letter grid instead of asking them to
    /// type. A game sets it with [`with_pad`](Self::with_pad);
    /// [`with_buttons`](Self::with_buttons) sets it for a button that
    /// went down.
    pub fn pad_pressed(&self) -> bool {
        self.pad
    }

    /// The same input with the chords `pressed` and the characters `text`
    /// typed this frame.
    #[must_use]
    pub fn with_typing(mut self, pressed: Vec<Chord>, text: Vec<char>) -> Self {
        self.pressed = pressed;
        self.text = text;
        self
    }

    /// The chords pressed this frame (bound or not; presses only, no
    /// repeats), for the few screens that read keys themselves: a text
    /// box's fixed keys ([`text_key`](super::fixed::text_key)) and the Key bindings
    /// screen, which captures the key for a slot.
    /// Anything else reacts to [`actions`](Self::actions).
    pub fn pressed_chords(&self) -> &[Chord] {
        &self.pressed
    }

    /// The characters typed this frame (printable only), for text boxes.
    pub fn text(&self) -> &[char] {
        &self.text
    }

    /// Whether a key bound to `action` is held (e.g. hold Confirm to
    /// fast-forward).
    pub fn is_held(&self, action: A) -> bool {
        self.held.contains(&action)
    }
}
