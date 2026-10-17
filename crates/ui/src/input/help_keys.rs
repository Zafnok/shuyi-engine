//! Help text that names keys and controller buttons. Names always come
//! from the active [`Keymap`], never string literals, because each layout
//! binds actions to different keys (ADR-0015) and the player can rebind
//! them (ADR-0031); and they follow the [`Device`] the player pressed last
//! (`docs/design/controls.md`, *Controller*): keys on the keyboard, the
//! pad's own button names on a controller.
//!
//! Generic over the game's [`ActionSet`] (ticket 1106).

use crate::prelude::*;
use core::ops::Deref;
use shuyi_assets::keymap::ActionSet;

use super::keymap::Keymap;
use super::state::Device;

/// The English name of the four arrow keys together: what
/// [`HelpKeys::new`] starts with, and what tests compare with. The game
/// shows the language file's word. Lives here because it names keys (only
/// `input` may).
// check-text: not player text (the default of `HelpKeys::new`; a game gives the language's)
pub const ARROWS: &str = "arrows";

/// What help text names keys from: the bindings, the device whose keys
/// or buttons to name, the two words a key's name can be
/// (`! not mapped`, `arrows`) and what joins a button chord's two buttons
/// (`A+B`). A game gives its screens one with the words
/// in the player's language ([`worded`](Self::worded)). Dereferences to the
/// [`Keymap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpKeys<'a, A: ActionSet> {
    keymap: &'a Keymap<A>,
    device: Device,
    not_mapped: &'a str,
    arrows: &'a str,
    chord_join: &'a str,
}

impl<'a, A: ActionSet> HelpKeys<'a, A> {
    /// `keymap`'s keys, or its buttons as a pad names them, by `device`,
    /// with the English words ([`NOT_MAPPED`], [`ARROWS`]).
    pub fn new(keymap: &'a Keymap<A>, device: Device) -> Self {
        Self {
            keymap,
            device,
            not_mapped: NOT_MAPPED,
            arrows: ARROWS,
            chord_join: CHORD_JOIN,
        }
    }

    /// These keys, named with `not_mapped` for an action with no key and
    /// `arrows` for the four arrow keys: the player's language's words.
    #[must_use]
    pub fn worded(self, not_mapped: &'a str, arrows: &'a str) -> Self {
        Self {
            not_mapped,
            arrows,
            ..self
        }
    }

    /// These keys, with `join` between a button chord's two buttons
    /// (`A+B`): the player's language's word.
    #[must_use]
    pub fn chord_joined(self, join: &'a str) -> Self {
        Self {
            chord_join: join,
            ..self
        }
    }

    /// These words for another set of bindings (a screen showing a keymap
    /// that isn't the active one).
    #[must_use]
    pub fn of(self, keymap: &'a Keymap<A>) -> Self {
        Self { keymap, ..self }
    }

    /// These bindings' keyboard keys, whatever the player is using.
    #[must_use]
    pub fn on_keyboard(self) -> Self {
        Self {
            device: Device::Keyboard,
            ..self
        }
    }

    /// What is shown for an action with no key, or no button on a
    /// controller (`docs/design/controls.md`, *Rebinding keys*).
    pub fn not_mapped(self) -> &'a str {
        self.not_mapped
    }

    /// `keymap`'s keyboard keys, whatever the player is using (a picture
    /// of the keyboard, a keyboard-only key).
    pub fn keyboard(keymap: &'a Keymap<A>) -> Self {
        Self::new(keymap, Device::Keyboard)
    }

    /// The device whose keys or buttons are named.
    pub fn device(self) -> Device {
        self.device
    }
}

impl<A: ActionSet> Deref for HelpKeys<'_, A> {
    type Target = Keymap<A>;

    fn deref(&self) -> &Keymap<A> {
        self.keymap
    }
}

/// Separator between the parts of a help line.
pub const SEPARATOR: &str = " · ";

/// The English of [`HelpKeys::not_mapped`]: what [`HelpKeys::new`] starts
/// with, and what tests compare with. The game shows
/// `keys.not_mapped` of the language file.
// check-text: not player text (the default of `HelpKeys::new`; a game gives the language's)
pub const NOT_MAPPED: &str = "! not mapped";

/// The English of what joins a button chord's two buttons (`A+B`): what
/// [`HelpKeys::new`] starts with, and what tests compare with. The game
/// shows the language file's.
// check-text: not player text (the default of `HelpKeys::new`; a game gives the language's)
pub const CHORD_JOIN: &str = "+";

/// The name of the key for `action` (its [`Keymap::primary`] chord), e.g.
/// `f` or `Shift+Space`; on a controller, of its button or button chord
/// ([`Keymap::primary_button`]) as that pad labels it, e.g. `A` or `A+B`.
/// [`HelpKeys::not_mapped`] if it has none.
pub fn key_name<A: ActionSet>(keys: HelpKeys<'_, A>, action: A) -> String {
    let name = match keys.device {
        Device::Keyboard => keys.primary(action).map(|c| c.to_string()),
        Device::Pad(kind) => {
            let button = keys.primary_button(action);
            button.map(|b| kind.chord_name(b, keys.chord_join))
        }
    };
    name.unwrap_or_else(|| keys.not_mapped.to_owned())
}

/// Every key for `action`, joined with `/` (e.g. `f/j/Enter/Space`): its
/// slots in [`Keymap::chords_for`] order, then its fixed keys
/// ([`Keymap::fixed_chords_for`]). On a controller, every button for it
/// ([`Keymap::buttons_for`]). [`HelpKeys::not_mapped`] if its slots are empty.
pub fn all_key_names<A: ActionSet>(keys: HelpKeys<'_, A>, action: A) -> String {
    let names: Vec<String> = match keys.device {
        Device::Keyboard => {
            let own = keys.chords_for(action);
            // The fixed keys alone don't count as a key of its own.
            let fixed = if own.is_empty() {
                Vec::new()
            } else {
                Keymap::<A>::fixed_chords_for(action)
            };
            own.iter().chain(&fixed).map(ToString::to_string).collect()
        }
        Device::Pad(kind) => {
            let buttons = keys.buttons_for(action);
            let name = |&b| kind.chord_name(b, keys.chord_join);
            buttons.iter().map(name).collect()
        }
    };
    if names.is_empty() {
        return keys.not_mapped.to_owned();
    }
    names.join("/")
}

/// What moves the cursor: `arrows` when the four cursor actions are on the
/// arrow keys, otherwise their keys in up-left-down-right order (`wasd`).
/// On a controller, `D-pad/L-stick` or whatever the cursor is on
/// ([`Keymap::cursor_buttons_name`]). [`HelpKeys::not_mapped`] if any
/// cursor action has no key. See [`Keymap::cursor_keys_name`].
pub fn cursor_keys_name<A: ActionSet>(keys: HelpKeys<'_, A>) -> String {
    let name = match keys.device {
        Device::Keyboard => keys.cursor_keys_name(keys.arrows),
        Device::Pad(kind) => keys.cursor_buttons_name(kind),
    };
    name.unwrap_or_else(|| keys.not_mapped.to_owned())
}

/// Joins `key label` hints with [`SEPARATOR`], leaving out hints whose key
/// is `None` (hidden by the caller, e.g. a key that does nothing here).
pub fn help_line(hints: &[(Option<String>, &str)]) -> String {
    hints
        .iter()
        .filter_map(|(key, label)| key.as_ref().map(|k| format!("{k} {label}")))
        .collect::<Vec<_>>()
        .join(SEPARATOR)
}

#[cfg(test)]
mod tests;
