//! What a player can ask for: `UiAction`'s six moves and Undo, every one
//! rebindable, their default keys and buttons in `files/keymap.ron`.

use serde::{Deserialize, Serialize};
use shuyi::assets::keymap::{ActionSet, Chord, Key, UiAction};

// ANCHOR: action
/// Everything a player can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Action {
    /// Walk up, or move a menu's focus up.
    Up,
    /// Walk down, or move a menu's focus down.
    Down,
    /// Walk left.
    Left,
    /// Walk right.
    Right,
    /// Choose a menu item.
    Confirm,
    /// Go back.
    Cancel,
    /// Take back the last move.
    Undo,
}

/// The keys every keymap has on top of its own: plain `Escape` cancels.
const FIXED_KEYS: [(Chord, Action); 1] = [(Chord::plain(Key::Escape), Action::Cancel)];

impl ActionSet for Action {
    /// No two actions share a key.
    type Group = ();

    const ALL: &'static [Self] = &[
        Self::Up,
        Self::Down,
        Self::Left,
        Self::Right,
        Self::Confirm,
        Self::Cancel,
        Self::Undo,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Confirm => "Confirm",
            Self::Cancel => "Cancel",
            Self::Undo => "Undo",
        }
    }

    /// The four moves and Undo repeat while held.
    fn is_repeatable(self) -> bool {
        !matches!(self, Self::Confirm | Self::Cancel)
    }

    /// Every action must keep a key: the game can't be played without one.
    fn is_required(self) -> bool {
        true
    }

    fn is_rebindable(self) -> bool {
        true
    }

    fn group(self) {}

    /// What it means to the engine's widgets: Undo means nothing to them.
    fn ui(self) -> Option<UiAction> {
        match self {
            Self::Up => Some(UiAction::Up),
            Self::Down => Some(UiAction::Down),
            Self::Left => Some(UiAction::Left),
            Self::Right => Some(UiAction::Right),
            Self::Confirm => Some(UiAction::Confirm),
            Self::Cancel => Some(UiAction::Cancel),
            Self::Undo => None,
        }
    }

    fn fixed_keys() -> &'static [(Chord, Self)] {
        &FIXED_KEYS
    }
}
// ANCHOR_END: action
