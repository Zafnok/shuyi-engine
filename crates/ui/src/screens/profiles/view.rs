//! What the profiles screen shows, as plain data: no cells, glyphs,
//! colours or positions. The screen builds a [`ProfilesView`] each frame
//! ([`ProfilesScreen::view`](super::ProfilesScreen::view)) and a skin
//! paints it ([`super::glyph::paint`] today).

use super::{DeleteChoice, RowKind};
use crate::prelude::*;
use crate::screens::options::LeaveChoice;
pub use crate::widgets::name_entry::{GridView, TypingView};
use crate::widgets::{AskView, MenuView};

/// The whole screen, as it is this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfilesView {
    /// The screen's title.
    pub title: String,
    /// The profiles, in the order they were made, then the row that adds
    /// one.
    pub rows: Vec<RowView>,
    /// The words that mark the profile in use.
    pub current_mark: String,
    /// What can be done to the focused profile, while that menu is open.
    pub actions: Option<ActionsView>,
    /// The typing box, while a name is typed on the keyboard.
    pub typing: Option<TypingView>,
    /// The letter grid, while a name is spelled on a controller.
    pub grid: Option<GridView>,
    /// The question asked on backing out of a name box, while it is open.
    pub leaving: Option<AskView<LeaveChoice>>,
    /// The question asked before deleting, while it is open.
    pub deleting: Option<AskView<DeleteChoice>>,
    /// A message for the player (something couldn't be done).
    pub message: Option<String>,
    /// The help line: what the keys do here.
    pub help: String,
}

impl ProfilesView {
    /// The focused row.
    pub fn focused(&self) -> Option<&RowView> {
        self.rows.iter().find(|r| r.focused)
    }

    /// The row of the profile in use.
    pub fn current(&self) -> Option<&RowView> {
        self.rows.iter().find(|r| r.current)
    }
}

/// One row: a profile, or the row that adds one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowView {
    /// Which row it is.
    pub kind: RowKind,
    /// The profile's name, or what the row does.
    pub label: String,
    /// Whether it is the profile in use.
    pub current: bool,
    /// Whether the cursor is on it.
    pub focused: bool,
    /// Whether Confirm does something there (no adding past the most
    /// profiles).
    pub enabled: bool,
}

/// The menu of what can be done to a profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionsView {
    /// The profile's name.
    pub title: String,
    /// What can be done, top to bottom: the labels are text keys, put in
    /// the player's language when painted.
    pub menu: MenuView,
}
