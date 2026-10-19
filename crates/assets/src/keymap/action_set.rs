//! What a game's input actions are to the input mechanism (ticket 1106):
//! [`ActionSet`], implemented by the game's own action enum, and
//! [`UiAction`], the few actions the generic widgets and screens react to.
//!
//! Generic over the game: nothing here names one game's actions.
use core::fmt::Debug;
use core::hash::Hash;

use super::Chord;

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
