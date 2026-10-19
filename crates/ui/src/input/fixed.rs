//! The keys no player can rebind (`docs/design/controls.md`, *Rebinding
//! keys*; game ADR-0031): plain `Escape` backs out of the Key bindings screen's
//! "Press a key…" and plain `Delete` empties a slot there, so neither can
//! be bound ([`Chord::is_reserved`]); and a text box's own `Enter`,
//! `Backspace` and `Escape`. Which action a fixed key also triggers in
//! play is the game's data ([`ActionSet::fixed_keys`]).
//!
//! Generic over the game (ticket 1106).
//!
//! [`ActionSet::fixed_keys`]: shuyi_assets::keymap::ActionSet::fixed_keys

use crate::prelude::*;
use shuyi_assets::keymap::{Chord, Key};

/// The chord that backs out of the Key bindings screen's "Press a key…"
/// without changing anything.
const CAPTURE_ABORT: Chord = Chord::plain(Key::Escape);

/// The chord that empties the highlighted slot on the Key bindings screen.
const CLEAR_SLOT: Chord = Chord::plain(Key::Delete);

/// Whether `chord` backs out of the Key bindings screen's "Press a key…"
/// (`docs/design/controls.md`, *Rebinding keys*). The screen asks this
/// rather than naming the key (the `keyboard-input` skill).
pub fn is_capture_abort(chord: Chord) -> bool {
    chord == CAPTURE_ABORT
}

/// Whether `chord` empties the highlighted slot on the Key bindings screen.
pub fn is_clear_slot(chord: Chord) -> bool {
    chord == CLEAR_SLOT
}

/// The name of the [`is_capture_abort`] key, for help text.
pub fn capture_abort_key_name() -> String {
    CAPTURE_ABORT.to_string()
}

/// The name of the [`is_clear_slot`] key, for help text.
pub fn clear_slot_key_name() -> String {
    CLEAR_SLOT.to_string()
}

/// A text box's own keys (the lead's name, 0801): fixed like `Escape` for
/// Cancel, since while a player types, every letter key is a letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKey {
    /// `Enter`: keep the text.
    Done,
    /// `Backspace`: delete the last character.
    Delete,
    /// `Escape`: close the box, text unchanged.
    Cancel,
}

/// The text-box key `chord` is, if any (Shift or not).
pub fn text_key(chord: Chord) -> Option<TextKey> {
    match chord.key {
        Key::Enter => Some(TextKey::Done),
        Key::Backspace => Some(TextKey::Delete),
        Key::Escape => Some(TextKey::Cancel),
        _ => None,
    }
}

/// The names of the text-box keys, for its help line: done, delete and
/// cancel.
pub fn text_key_names() -> [String; 3] {
    [Key::Enter, Key::Backspace, Key::Escape].map(|key| Chord::plain(key).to_string())
}
