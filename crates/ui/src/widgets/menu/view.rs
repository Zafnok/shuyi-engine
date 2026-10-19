//! What a [`Menu`](super::Menu) shows, as plain data: its items with their
//! labels and which one is focused, no cells or positions. A screen puts
//! the [`MenuView`] in its own view and its skin paints it
//! ([`super::glyph::paint`] today; game ADR-0054).

use crate::prelude::*;

/// The whole menu, as it is this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuView {
    /// The items, top to bottom.
    pub items: Vec<MenuItemView>,
}

impl MenuView {
    /// The focused item, if there is one.
    pub fn focused(&self) -> Option<&MenuItemView> {
        self.items.iter().find(|i| i.focused)
    }
}

/// One item of a menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItemView {
    /// Its text.
    pub label: String,
    /// Text that follows the label (e.g. why it can't be chosen) and what
    /// it is.
    pub suffix: Option<(String, SuffixKind)>,
    /// What it does, shown under the list while it has the focus.
    pub note: Option<String>,
    /// Whether it can be chosen: a disabled item is dimmed (and skipped by
    /// the cursor, unless the menu browses disabled items).
    pub enabled: bool,
    /// Whether the cursor is on it.
    pub focused: bool,
}

/// What the text after an item's label is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuffixKind {
    /// A warning: why the item can't be chosen, or that it is broken.
    Warning,
    /// A detail aside, such as what it costs.
    Aside,
}
