//! What the Key bindings screen shows, as plain data: no cells, glyphs,
//! colours or positions. The screen builds a [`KeyBindingsView`] each frame
//! ([`KeyBindingsScreen::view`](super::KeyBindingsScreen::view)) and a skin
//! paints it ([`super::glyph::paint`] today).

use super::Side;
use crate::prelude::*;

/// The whole screen, as it is this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBindingsView<A> {
    /// The screen's title.
    pub title: String,
    /// The keyboard / controller switch; `None` with no keyboard, when the
    /// controller side is the only one.
    pub switch: Option<SwitchView>,
    /// The headings over the slot columns, left to right.
    pub columns: Vec<String>,
    /// The actions, grouped: the required ones, then the optional ones.
    pub groups: Vec<GroupView<A>>,
    /// The row under the actions that puts the shown side's defaults back.
    pub restore: RestoreView,
    /// The note on a row with nothing in any slot (`! not mapped`).
    pub not_mapped: String,
    /// A message for the player (a key can't be used, leaving is blocked).
    pub message: Option<String>,
    /// The yes/no question open over the rows, if any.
    pub question: Option<QuestionView>,
    /// The choices open under the focused slot, if any (the controller
    /// side's change / clear).
    pub choices: Option<ChoicesView>,
    /// The help line: what the keys do here.
    pub help: String,
}

impl<A: Copy + PartialEq> KeyBindingsView<A> {
    /// Every action's row, top to bottom.
    pub fn rows(&self) -> impl Iterator<Item = &RowView<A>> {
        self.groups.iter().flat_map(|g| g.rows.iter())
    }

    /// The row of `action`, if it is shown.
    pub fn row(&self, action: A) -> Option<&RowView<A>> {
        self.rows().find(|r| r.action == action)
    }

    /// The focused row, if the focus is on an action.
    pub fn focused(&self) -> Option<&RowView<A>> {
        self.rows().find(|r| r.focus.is_some())
    }
}

/// The switch between the keyboard and the controller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchView {
    /// The name of the keyboard side, with the layout it edits.
    pub keyboard: String,
    /// The name of the controller side.
    pub controller: String,
    /// The side shown.
    pub shown: Side,
    /// Whether the cursor is on the switch.
    pub focused: bool,
}

/// A heading and the actions under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupView<A> {
    /// What the group is.
    pub heading: String,
    /// Whether these are the required actions (each must have a key or a
    /// button before the screen can be left).
    pub required: bool,
    /// Its actions, top to bottom.
    pub rows: Vec<RowView<A>>,
}

/// One action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowView<A> {
    /// Which action it is.
    pub action: A,
    /// Its name.
    pub label: String,
    /// The keys it also has that can't be changed (keyboard side only).
    pub fixed: Vec<String>,
    /// What each of its slots holds.
    pub slots: Vec<SlotView>,
    /// The slot the cursor is on, if the cursor is on this row.
    pub focus: Option<usize>,
    /// Whether nothing is in any of its slots.
    pub unmapped: bool,
    /// Whether it is unmapped and so blocks leaving the screen (a required
    /// action with nothing on the shown side).
    pub blocks_leaving: bool,
    /// Whether it just lost its key to another slot (it stays highlighted
    /// for a moment).
    pub lost_key: bool,
}

impl<A> RowView<A> {
    /// Whether the cursor is on this row.
    pub fn focused(&self) -> bool {
        self.focus.is_some()
    }
}

/// What one slot holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlotView {
    /// Nothing.
    Empty,
    /// A key or button, by name.
    Bound(String),
    /// Waiting for the key or button to put in it: what it shows meanwhile.
    Capturing(String),
}

/// The restore defaults row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreView {
    /// Its name.
    pub label: String,
    /// Whether the cursor is on it.
    pub focused: bool,
}

/// A yes/no question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionView {
    /// The question.
    pub text: String,
    /// How to answer: the keys for yes and no.
    pub answers: String,
}

/// The choices a slot offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoicesView {
    /// The choices, top to bottom.
    pub lines: Vec<String>,
    /// The one the cursor is on.
    pub focused: usize,
    /// The slot they belong to (on the focused row).
    pub slot: usize,
}
