//! What a [`Menu`](super::Menu) shows, as plain data: its items with their
//! labels and which one is focused, no cells or positions. A screen puts
//! the [`MenuView`] in its own view and its skin paints it
//! ([`super::glyph::paint`] today; game ADR-0054).

use crate::prelude::*;

/// The whole menu, as it is this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuView {
    /// The items, top to bottom: all of them, also those scrolled out of
    /// the box.
    pub items: Vec<MenuItemView>,
    /// The first item the box shows (0 unless the menu has scrolled).
    pub first_shown: usize,
    /// How many items the box shows from `first_shown` (all of them unless
    /// the menu scrolls).
    pub shown: usize,
}

impl MenuView {
    /// A view of `items` that shows them all.
    pub fn new(items: Vec<MenuItemView>) -> Self {
        Self {
            shown: items.len(),
            items,
            first_shown: 0,
        }
    }

    /// The items the box shows, top to bottom.
    pub fn shown_items(&self) -> &[MenuItemView] {
        let first = self.first_shown.min(self.items.len());
        let end = (first + self.shown).min(self.items.len());
        &self.items[first..end]
    }

    /// Whether items are scrolled out above the box, so a skin can mark it.
    pub fn more_above(&self) -> bool {
        self.first_shown > 0
    }

    /// Whether items are scrolled out below the box.
    pub fn more_below(&self) -> bool {
        self.first_shown + self.shown < self.items.len()
    }

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
