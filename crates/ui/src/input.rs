//! Keyboard and controller input as a game's actions (game ADR-0006, game ADR-0034),
//! generic over its [`ActionSet`] (tickets 1106 and 1107): a data-driven
//! [`Keymap`] from chords and controller buttons to actions
//! ([`keymap`](mod@keymap)), and [`InputState`] ([`state`]), which turns
//! presses and releases plus frame time into the actions screens see,
//! including repeat for held repeatable keys and buttons. [`FrameInput`]
//! ([`frame`]) is one frame's input as a screen sees it.
//!
//! The player's own bindings (3 slots per action, per layout, and the
//! controller buttons) are in [`bindings`]; [`pad`] turns the controllers'
//! raw state into button presses, and [`crank`] a crank's turns; [`help_keys`]
//! names keys and buttons in help text.
//!
//! **Fixed keys** ([`fixed`], game ADR-0031): plain `Escape` backs out of
//! capturing a key and plain `Delete` empties a slot on a rebinding
//! screen, so neither can be bound ([`Chord::is_reserved`]). Which action a
//! fixed key also triggers in play is the game's
//! ([`ActionSet::fixed_keys`]). `Shift+Escape` and `Shift+Delete` are
//! ordinary chords.
//!
//! A screen declares the actions it reacts to
//! ([`Screen::actions`](crate::screen::Screen::actions)); [`ui_actions`]
//! gives them for an engine screen or widget, which reacts to
//! [`UiAction`]s. [`fit`] checks that the required ones fit a target's
//! buttons (ADR-0008 §3).
//!
//! [`Key`], [`Chord`], [`Button`] and [`Layout`] are the keymap file's
//! (`shuyi_assets::keymap`) and re-exported here.

use alloc::vec::Vec;

pub mod bindings;
pub mod crank;
pub mod fit;
pub mod fixed;
pub mod frame;
pub mod help_keys;
pub mod keymap;
pub mod pad;
pub mod state;

#[cfg(test)]
mod chord_tests;
#[cfg(any(test, feature = "toy"))]
pub mod sample;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod toy_tests;

pub use bindings::{BindError, ButtonSlots, LayoutBindings, PadBindings, PlayerKeys, Slots};
pub use crank::Crank;
pub use fixed::{
    TextKey, capture_abort_key_name, clear_slot_key_name, is_capture_abort, is_clear_slot,
    text_key, text_key_names,
};
pub use frame::{FrameInput, frame_seconds};
pub use help_keys::{ARROWS, HelpKeys};
pub use keymap::{ChordWait, Keymap};
pub use pad::{ButtonSet, PadId, PadKind, PadState, Pads};
pub use shuyi_assets::keymap::{
    ActionSet, Bindings, Button, ButtonChord, Chord, ChordDef, CrankDef, Key, KeymapDef,
    LAYOUT_NAME_MAX, Layout, LayoutKeys, PadKeys, RepeatDef, SLOTS, StickDef, UiAction,
};
pub use state::{Device, Input, InputState, MAX_REPEATS_PER_UPDATE};

/// What the engine's menu-like screens react to: the four directions,
/// Confirm and Cancel.
pub const NAVIGATION: &[UiAction] = &[
    UiAction::Up,
    UiAction::Down,
    UiAction::Left,
    UiAction::Right,
    UiAction::Confirm,
    UiAction::Cancel,
];

/// The game's actions that [`ActionSet::ui`] takes for one of `uis`, in
/// [`ActionSet::ALL`] order: what an engine screen or widget that reacts
/// to `uis` declares ([`Screen::actions`](crate::screen::Screen::actions)).
pub fn ui_actions<A: ActionSet>(uis: &[UiAction]) -> Vec<A> {
    A::ALL
        .iter()
        .copied()
        .filter(|a| a.ui().is_some_and(|ui| uis.contains(&ui)))
        .collect()
}
