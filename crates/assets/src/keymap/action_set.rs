//! What a game's input actions are to the input mechanism (ticket 1106):
//! [`ActionSet`], implemented by the game's own action enum, and
//! [`UiAction`], the few actions the generic widgets and screens react to.
//! `UiAction` is an action set too, ready-made for a game that plays with
//! the six moves alone (ADR-0036).
//!
//! Generic over the game: nothing here names one game's actions.
use core::fmt::Debug;
use core::hash::Hash;

use super::{Chord, Key};

/// What the generic widgets (a menu), the Key bindings screen and the debug
/// hook react to. A game maps some of its actions onto these with
/// [`ActionSet::ui`]; the rest are its own business.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UiAction {
    /// Move up (a menu's focus, a cursor).
    Up,
    /// Move down.
    Down,
    /// Move left.
    Left,
    /// Move right.
    Right,
    /// Confirm / select.
    Confirm,
    /// Cancel / back.
    Cancel,
    /// Open the developer's debug screens.
    Debug,
}

/// Everything a player can ask for in one game: the game's action enum.
/// The keymap file, the keymap, held keys and repeat, the player's saved
/// bindings and help text are all built on it, never on one game's
/// actions.
pub trait ActionSet: Copy + Ord + Eq + Hash + Debug + 'static {
    /// Which actions may share a key or button: a key is on at most one
    /// action of each group (game ADR-0068). `()` for a game where no two
    /// actions share one.
    type Group: Copy + Ord + Eq + Debug;

    /// Every action, in the order the game lists them (the order a repair
    /// of the saved bindings keeps the first of two claims in).
    const ALL: &'static [Self];

    /// The name used in keymap files and the saved bindings.
    fn name(self) -> &'static str;

    /// Looks an action up by its [`name`](Self::name) (exact match).
    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|a| a.name() == name)
    }

    /// Whether holding its key or button re-emits it.
    fn is_repeatable(self) -> bool;

    /// Whether the player must always keep at least one key (and one
    /// button) on it.
    fn is_required(self) -> bool;

    /// Whether the player can rebind it. One that can't keeps the keymap
    /// file's keys and has no controller button.
    fn is_rebindable(self) -> bool;

    /// Its [`Group`](Self::Group).
    fn group(self) -> Self::Group;

    /// What a generic widget or screen takes it for, if anything.
    fn ui(self) -> Option<UiAction>;

    /// The keys every keymap has on top of its own bindings, which the
    /// player can't move or remove (this game: plain `Escape` cancels).
    /// Shown after an action's own keys in help text, but never counted as
    /// a key of its own. A game with none returns `&[]`.
    fn fixed_keys() -> &'static [(Chord, Self)];

    /// The first action that [`ui`](Self::ui) maps to `ui`, if any.
    fn for_ui(ui: UiAction) -> Option<Self> {
        Self::ALL.iter().copied().find(|a| a.ui() == Some(ui))
    }
}

/// The keys every keymap of [`UiAction`]s has on top of its own: plain
/// `Escape` cancels.
const UI_FIXED_KEYS: [(Chord, UiAction); 1] = [(Chord::plain(Key::Escape), UiAction::Cancel)];

/// The ready-made action set of a game that plays with the six moves: its
/// keymap file names `Up`, `Down`, `Left`, `Right`, `Confirm` and `Cancel`,
/// every one required and rebindable. [`UiAction::Debug`] is no action of
/// the set: a game with a debug key writes its own action enum. A
/// `UiAction` added later stays out of [`ALL`](ActionSet::ALL) too, so no
/// game's keymap file must name it (ADR-0036).
impl ActionSet for UiAction {
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
            Self::Debug => "Debug",
        }
    }

    /// The four moves repeat while held.
    fn is_repeatable(self) -> bool {
        matches!(self, Self::Up | Self::Down | Self::Left | Self::Right)
    }

    /// The six must keep a key: the game can't be played without one.
    fn is_required(self) -> bool {
        self != Self::Debug
    }

    fn is_rebindable(self) -> bool {
        self != Self::Debug
    }

    fn group(self) {}

    fn ui(self) -> Option<UiAction> {
        Some(self)
    }

    fn fixed_keys() -> &'static [(Chord, Self)] {
        &UI_FIXED_KEYS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::KeymapDef;

    /// A keymap file of the six moves, and `extra` lines in its layout and
    /// its pad.
    fn source(extra: &str) -> String {
        format!(
            r#"(
    layouts: [(
        id: "Arrows",
        name: "layout.arrows",
        keys: {{
            "Up": ["Up"], "Down": ["Down"], "Left": ["Left"], "Right": ["Right"],
            "Confirm": ["Enter"], "Cancel": ["Backspace"],{extra}
        }},
    )],
    layout_picker: {{ "Up": ["Up"], "Down": ["Down"], "Confirm": ["Enter"] }},
    pad: {{
        "Up": ["DpadUp"], "Down": ["DpadDown"], "Left": ["DpadLeft"],
        "Right": ["DpadRight"], "Confirm": ["South"], "Cancel": ["East"],
    }},
    targets: {{}},
    repeat: (delay_ms: 300, interval_ms: 70),
    stick: (press_percent: 50, release_percent: 35),
)"#
        )
    }

    #[test]
    fn the_six_moves_are_the_set_and_debug_is_not() {
        assert_eq!(
            UiAction::ALL,
            [
                UiAction::Up,
                UiAction::Down,
                UiAction::Left,
                UiAction::Right,
                UiAction::Confirm,
                UiAction::Cancel,
            ]
        );
        for &action in UiAction::ALL {
            assert_eq!(UiAction::from_name(action.name()), Some(action));
            assert_eq!(action.ui(), Some(action));
            assert_eq!(UiAction::for_ui(action), Some(action));
            assert!(action.is_required() && action.is_rebindable());
        }
        let repeating: Vec<UiAction> = UiAction::ALL
            .iter()
            .copied()
            .filter(|a| a.is_repeatable())
            .collect();
        assert_eq!(
            repeating,
            [
                UiAction::Up,
                UiAction::Down,
                UiAction::Left,
                UiAction::Right
            ]
        );
        assert_eq!(UiAction::Debug.name(), "Debug");
        assert_eq!(UiAction::from_name("Debug"), None);
        assert_eq!(UiAction::for_ui(UiAction::Debug), None);
        assert!(!UiAction::Debug.is_required() && !UiAction::Debug.is_rebindable());
        assert!(!UiAction::Debug.is_repeatable());
        assert_eq!(
            UiAction::fixed_keys(),
            [(Chord::plain(Key::Escape), UiAction::Cancel)]
        );
    }

    #[test]
    fn a_keymap_file_of_the_six_moves_loads_and_one_naming_debug_is_refused() {
        let keymap = KeymapDef::<UiAction>::from_source("keymap.ron", &source(""))
            .unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(
            keymap
                .first_layout()
                .map(|l| l.name().to_owned())
                .as_deref(),
            Some("Arrows")
        );
        let errors =
            KeymapDef::<UiAction>::from_source("keymap.ron", &source(r#" "Debug": ["F3"],"#))
                .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| e.to_string().contains("unknown action \"Debug\"")),
            "{errors:?}"
        );
    }
}
