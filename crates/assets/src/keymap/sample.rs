//! A sample game's actions and keymap file, for the tests of the keymap
//! format and of the input mechanism built on it (ticket 1107): a tactics
//! game with four cursor moves, Confirm and Cancel, a few map actions, a
//! developer Debug key no player can rebind, two optional actions and a
//! dialogue log in a [`KeyGroup`] of its own. Its keymap file
//! ([`keymap`]) has two layouts, [`RIGHT_HANDED`] and [`LEFT_HANDED`].
//!
//! Built for this crate's tests and, with the `sample-actions` feature, for
//! other engine crates' tests.
use core::fmt;

use super::{ActionSet, Chord, Key, KeymapDef, Layout, UiAction};

/// The sample keymap file's text.
pub const SOURCE: &str = include_str!("sample.ron");

/// The right-handed layout: arrow keys steer; the left hand acts from the
/// `A S D F` row.
pub const RIGHT_HANDED: Layout = Layout::named("RightHanded");

/// The left-handed layout: `W A S D` steer; the right hand acts from the
/// `J K L ;` row.
pub const LEFT_HANDED: Layout = Layout::named("LeftHanded");

/// The sample keymap file, validated.
///
/// # Panics
///
/// If [`SOURCE`] doesn't load, which its own tests rule out.
pub fn keymap() -> KeymapDef<Action> {
    KeymapDef::from_source("sample.ron", SOURCE).unwrap_or_else(|e| panic!("{e:?}"))
}

/// The keys every keymap has on top of its own bindings, whatever the
/// player binds: plain `Escape` cancels.
const FIXED_KEYS: [(Chord, Action); 1] = [(Chord::plain(Key::Escape), Action::Cancel)];

/// Everything a player of the sample game can ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    /// Move the cursor one tile left.
    CursorLeft,
    /// Move the cursor one tile down.
    CursorDown,
    /// Move the cursor one tile up.
    CursorUp,
    /// Move the cursor one tile right.
    CursorRight,
    /// Confirm / select.
    Confirm,
    /// Cancel / back. Screens with nothing to cancel open the menu instead.
    Cancel,
    /// Unit info / details.
    Info,
    /// Toggle the enemy danger zone.
    DangerZone,
    /// Jump to the next ready unit.
    NextUnit,
    /// Jump to the previous ready unit.
    PrevUnit,
    /// End the player phase (asks for confirmation).
    EndTurn,
    /// Open the map menu.
    Menu,
    /// Debug overlay (developer).
    Debug,
    /// Toggle auto-end of the player phase.
    ToggleAutoEnd,
    /// Open the turn rewind.
    Rewind,
    /// Pick a unit, tile or target on the map with the cursor. Optional:
    /// with no key, Confirm does it.
    Select,
    /// Accept the end-turn prompt. Optional: with no key, End turn pressed
    /// again does it.
    ConfirmEndTurn,
    /// Switch the battle map between its two zooms. Optional.
    ZoomMap,
    /// Open the dialogue log while a scene plays. Optional, and in a
    /// [`KeyGroup`] of its own.
    DialogueLog,
}

/// Which actions may share a key or button: only actions of the same group clash, since actions of
/// different groups are used in different places. A key is on at most one
/// action of each group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyGroup {
    /// The map, the menus and everything else.
    General,
    /// The dialogue log, opened while a scene plays.
    DialogueLog,
}

impl KeyGroup {
    /// Every group, in the order the Key bindings screen lists them.
    pub const ALL: [KeyGroup; 2] = [Self::General, Self::DialogueLog];
}

impl Action {
    /// Every action, in declaration order.
    pub const ALL: [Action; 19] = [
        Self::CursorLeft,
        Self::CursorDown,
        Self::CursorUp,
        Self::CursorRight,
        Self::Confirm,
        Self::Cancel,
        Self::Info,
        Self::DangerZone,
        Self::NextUnit,
        Self::PrevUnit,
        Self::EndTurn,
        Self::Menu,
        Self::Debug,
        Self::ToggleAutoEnd,
        Self::Rewind,
        Self::Select,
        Self::ConfirmEndTurn,
        Self::ZoomMap,
        Self::DialogueLog,
    ];

    /// The name used in keymap files (the variant name).
    pub fn name(self) -> &'static str {
        const NAMES: [&str; 19] = [
            "CursorLeft",
            "CursorDown",
            "CursorUp",
            "CursorRight",
            "Confirm",
            "Cancel",
            "Info",
            "DangerZone",
            "NextUnit",
            "PrevUnit",
            "EndTurn",
            "Menu",
            "Debug",
            "ToggleAutoEnd",
            "Rewind",
            "Select",
            "ConfirmEndTurn",
            "ZoomMap",
            "DialogueLog",
        ];
        NAMES[self as usize]
    }

    /// Looks an action up by its [`name`](Self::name) (exact match).
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|a| a.name() == name)
    }

    /// Whether holding the key re-emits the action (the four cursor moves).
    pub fn is_repeatable(self) -> bool {
        matches!(
            self,
            Self::CursorLeft | Self::CursorDown | Self::CursorUp | Self::CursorRight
        )
    }

    /// Whether the player must always keep at least one key on this action:
    /// the four cursor moves, Confirm, Cancel and End turn. The rest may be
    /// left with no key.
    pub fn is_required(self) -> bool {
        matches!(
            self,
            Self::CursorLeft
                | Self::CursorDown
                | Self::CursorUp
                | Self::CursorRight
                | Self::Confirm
                | Self::Cancel
                | Self::EndTurn
        )
    }

    /// The [`KeyGroup`] whose actions this one's keys may not be shared
    /// with.
    pub fn group(self) -> KeyGroup {
        match self {
            Self::DialogueLog => KeyGroup::DialogueLog,
            _ => KeyGroup::General,
        }
    }

    /// Whether the player can rebind this action. Only the developer Debug
    /// key can't.
    pub fn is_rebindable(self) -> bool {
        self != Self::Debug
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl ActionSet for Action {
    type Group = KeyGroup;

    const ALL: &'static [Self] = &Action::ALL;

    fn name(self) -> &'static str {
        Action::name(self)
    }

    fn is_repeatable(self) -> bool {
        Action::is_repeatable(self)
    }

    fn is_required(self) -> bool {
        Action::is_required(self)
    }

    fn is_rebindable(self) -> bool {
        Action::is_rebindable(self)
    }

    fn group(self) -> KeyGroup {
        Action::group(self)
    }

    fn ui(self) -> Option<UiAction> {
        match self {
            Self::CursorUp => Some(UiAction::Up),
            Self::CursorDown => Some(UiAction::Down),
            Self::CursorLeft => Some(UiAction::Left),
            Self::CursorRight => Some(UiAction::Right),
            Self::Confirm => Some(UiAction::Confirm),
            Self::Cancel => Some(UiAction::Cancel),
            Self::Debug => Some(UiAction::Debug),
            Self::Info
            | Self::DangerZone
            | Self::NextUnit
            | Self::PrevUnit
            | Self::EndTurn
            | Self::Menu
            | Self::ToggleAutoEnd
            | Self::Rewind
            | Self::Select
            | Self::ConfirmEndTurn
            | Self::ZoomMap
            | Self::DialogueLog => None,
        }
    }

    fn fixed_keys() -> &'static [(Chord, Self)] {
        &FIXED_KEYS
    }
}
