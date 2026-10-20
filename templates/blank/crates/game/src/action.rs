//! What a player can ask for: six actions, every one rebindable, their
//! default keys and buttons in `files/keymap.ron`.

use shuyi_assets::keymap::{ActionSet, Chord, Key, UiAction};

/// Everything a player can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    /// Step up.
    Up,
    /// Step down.
    Down,
    /// Step left.
    Left,
    /// Step right.
    Right,
    /// Choose.
    Confirm,
    /// Leave the game.
    Cancel,
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
    ];

    fn name(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Confirm => "Confirm",
            Self::Cancel => "Cancel",
        }
    }

    /// The four moves repeat while held.
    fn is_repeatable(self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::Left | Self::Right)
    }

    /// Every action must keep a key: the game can't be played without one.
    fn is_required(self) -> bool {
        true
    }

    fn is_rebindable(self) -> bool {
        true
    }

    fn group(self) {}

    fn ui(self) -> Option<UiAction> {
        Some(match self {
            Self::Up => UiAction::Up,
            Self::Down => UiAction::Down,
            Self::Left => UiAction::Left,
            Self::Right => UiAction::Right,
            Self::Confirm => UiAction::Confirm,
            Self::Cancel => UiAction::Cancel,
        })
    }

    fn fixed_keys() -> &'static [(Chord, Self)] {
        &FIXED_KEYS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_action_has_its_own_name_and_widget_meaning() {
        for &action in Action::ALL {
            assert_eq!(Action::from_name(action.name()), Some(action));
            assert_eq!(action.ui().and_then(Action::for_ui), Some(action));
            assert!(action.is_required() && action.is_rebindable());
        }
        let repeating: Vec<Action> = Action::ALL
            .iter()
            .copied()
            .filter(|a| a.is_repeatable())
            .collect();
        assert_eq!(
            repeating,
            [Action::Up, Action::Down, Action::Left, Action::Right]
        );
        assert_eq!(
            Action::fixed_keys(),
            &[(Chord::plain(Key::Escape), Action::Cancel)]
        );
    }
}
