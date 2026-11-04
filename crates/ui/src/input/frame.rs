//! [`FrameInput`]: one frame's input as a screen sees it. Generic over the
//! game's [`ActionSet`] (ticket 1106), and [`frame_seconds`], which makes
//! a frame time a platform reports safe to add up.
//!
//! In tests, a screen sees only the actions it declares
//! ([`Screen::actions`](crate::screen::Screen::actions), ADR-0008 §3):
//! [`declared`](FrameInput::declared) filters a frame's input down to
//! them, and asking whether an action it didn't declare is held fails the
//! test.

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
    /// Actions this frame, in order: key presses, then key repeats. A fixed
    /// step reads [`presses`](Self::presses) instead, which have no repeats.
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
    /// The fixed simulation steps due this frame.
    ticks: u32,
    /// The actions pressed this frame, in order, without the key repeats.
    presses: Vec<A>,
    /// The actions let go this frame, in order.
    released: Vec<A>,
    /// Whether the game came back from a suspend just before this frame.
    resumed: bool,
    /// The screen this input was [filtered](Self::declared) for, and the
    /// actions it declares: asking about any other is a bug in the
    /// screen's declaration.
    declared: Option<(&'static str, Vec<A>)>,
}

impl<A: ActionSet> FrameInput<A> {
    /// Input for one frame; `held` lists the actions whose keys are down.
    /// No [`pressed_chords`](Self::pressed_chords). A `dt` that is not
    /// finite or is negative counts as 0 ([`frame_seconds`]). Each of
    /// `actions` is a press ([`presses`](Self::presses)), and one that
    /// isn't held is a tap, let go too ([`released`](Self::released)),
    /// until [`with_presses`](Self::with_presses) says otherwise.
    pub fn new(actions: Vec<A>, dt: f32, held: Vec<A>) -> Self {
        let mut released: Vec<A> = Vec::new();
        for &action in &actions {
            if !held.contains(&action) && !released.contains(&action) {
                released.push(action);
            }
        }
        Self {
            presses: actions.clone(),
            released,
            actions,
            dt: frame_seconds(dt),
            held,
            pressed: Vec::new(),
            text: Vec::new(),
            pad: false,
            buttons_down: Vec::new(),
            buttons_up: Vec::new(),
            ticks: 0,
            resumed: false,
            declared: None,
        }
    }

    /// This input as `screen` sees it when it declares that it reacts to
    /// `declared` alone ([`Screen::actions`](crate::screen::Screen::actions)):
    /// only those actions, with the actions held back (in order, as often
    /// as they came). Asking the result whether an action outside
    /// `declared` is [held](Self::is_held) panics: the screen reads an
    /// action it didn't declare. Its [`presses`](Self::presses),
    /// [`released`](Self::released) and [`held`](Self::held) actions are
    /// `declared`'s alone. Keys, buttons and text stay as they were.
    pub fn declared(&self, screen: &'static str, declared: &[A]) -> (Self, Vec<A>) {
        let (actions, missed) = self.actions.iter().partition(|a| declared.contains(a));
        let only = |list: &[A]| {
            list.iter()
                .copied()
                .filter(|a| declared.contains(a))
                .collect()
        };
        let input = Self {
            actions,
            presses: only(&self.presses),
            released: only(&self.released),
            held: only(&self.held),
            declared: Some((screen, declared.to_vec())),
            ..self.clone()
        };
        (input, missed)
    }

    /// The same input with the actions `presses`, pressed this frame in
    /// order without the key repeats, and `released`, let go this frame in
    /// order, as [`InputState`](super::InputState) gives them.
    #[must_use]
    pub fn with_presses(mut self, presses: Vec<A>, released: Vec<A>) -> Self {
        self.presses = presses;
        self.released = released;
        self
    }

    /// The actions pressed this frame, in order, without the key repeats
    /// that [`actions`](Self::actions) has after them: what a fixed step
    /// reads. A tick stage reads them through its
    /// [`TickDriver`](crate::tick::TickDriver), which gives each to one
    /// step.
    pub fn presses(&self) -> &[A] {
        &self.presses
    }

    /// The actions let go this frame, in order: each when no key or button
    /// holds it any more.
    pub fn released(&self) -> &[A] {
        &self.released
    }

    /// The actions whose key or button is down at the end of this frame, in
    /// the game's order ([`ActionSet::ALL`]).
    pub fn held(&self) -> &[A] {
        &self.held
    }

    /// The same input with whether the game came back from a suspend just
    /// before this frame.
    #[must_use]
    pub fn with_resumed(mut self, resumed: bool) -> Self {
        self.resumed = resumed;
        self
    }

    /// Whether the game came back from a suspend just before this frame
    /// (the platform's `SystemEvent::Resume`): a tick stage forgets the
    /// presses it kept from before ([`TickDriver`](crate::tick::TickDriver)
    /// does).
    pub fn resumed(&self) -> bool {
        self.resumed
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
    /// a screen adds it up as it is. It is for what only looks: rules count
    /// [`ticks`](Self::ticks) or turns, with `shuyi_core::timer`.
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// The same input with `ticks` fixed simulation steps due.
    #[must_use]
    pub fn with_ticks(mut self, ticks: u32) -> Self {
        self.ticks = ticks;
        self
    }

    /// How many fixed simulation steps are due this frame (ADR-0003 §2: 60
    /// a second, at most 4 a frame): a stage with a tick simulation steps
    /// it this many times. The platform's `App` counts them from its clock
    /// (`shuyi_platform::tick`); 0 while the game is suspended, in an
    /// update with no time passing ([`Game::act`](crate::Game::act)) and
    /// in a frame no `App` ran. Rules time things in these steps, with
    /// `shuyi_core::timer`, and never with [`dt`](Self::dt).
    pub fn ticks(&self) -> u32 {
        self.ticks
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
    ///
    /// # Panics
    ///
    /// On input [`declared`](Self::declared) for a screen that doesn't
    /// declare `action`.
    pub fn is_held(&self, action: A) -> bool {
        if let Some((screen, declared)) = &self.declared {
            assert!(
                declared.contains(&action),
                "screen \"{screen}\" asks whether {action:?} is held, but doesn't declare it \
                 (Screen::actions)"
            );
        }
        self.held.contains(&action)
    }
}
