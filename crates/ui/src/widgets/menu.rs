//! A vertical list menu: a title screen's, an action menu, a list of
//! replies.
//!
//! The menu holds its items and its focus (a [`ListCursor`]). What it
//! shows is
//! [`Menu::view`], plain data, and [`glyph::paint`] draws that view as
//! glyphs: the screens' own skins call it for their menu.

pub mod glyph;
pub mod view;

use crate::prelude::*;
pub use view::{MenuItemView, MenuView, SuffixKind};

use shuyi_assets::keymap::{ActionSet, UiAction};

use super::list::{ListCursor, Scroll};
use crate::audio::{AudioQueue, MenuSound};

/// One menu entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    /// Text shown.
    pub label: String,
    /// Disabled items are drawn dim, skipped by the cursor and can't be chosen.
    pub enabled: bool,
    /// Text shown after the label (e.g. `(broken)`), and what it is.
    pub suffix: Option<(String, SuffixKind)>,
    /// What the item does, shown under the list while it has the focus
    /// (a description in a list).
    pub note: Option<String>,
}

impl MenuItem {
    /// An enabled item.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            suffix: None,
            note: None,
        }
    }

    /// The same item with `text` of `kind` after its label.
    #[must_use]
    pub fn with_suffix(mut self, text: impl Into<String>, kind: SuffixKind) -> Self {
        self.suffix = Some((text.into(), kind));
        self
    }

    /// The same item with `text` saying what it does, shown under the
    /// list while it has the focus.
    #[must_use]
    pub fn with_note(mut self, text: impl Into<String>) -> Self {
        self.note = Some(text.into());
        self
    }

    /// A disabled item.
    pub fn disabled(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: false,
            suffix: None,
            note: None,
        }
    }
}

/// What a menu reports back to its screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEvent {
    /// Confirm on the enabled item at this index.
    Chosen(usize),
    /// Cancel was pressed.
    Cancelled,
}

/// A vertical list of items in a single-line box. `CursorDown`/`CursorUp`
/// move the focus (wrapping, skipping disabled items), `Confirm` chooses the
/// focused item, `Cancel` cancels (unless the menu is
/// [`without_cancel`](Self::without_cancel)). A menu
/// [`showing`](Self::showing) fewer rows than it has scrolls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    items: Vec<MenuItem>,
    cursor: ListCursor,
    /// How many rows the box shows (all of them if `None`).
    rows_shown: Option<usize>,
    /// Whether `Cancel` reports [`MenuEvent::Cancelled`].
    cancellable: bool,
    /// Whether the focus also stops on disabled items, so their notes can
    /// be read (they still can't be chosen).
    browse_disabled: bool,
}

impl Menu {
    /// A menu focused on its first enabled item (or the first item, if none
    /// is enabled).
    pub fn new(items: Vec<MenuItem>) -> Self {
        let focus = items.iter().position(|i| i.enabled).unwrap_or(0);
        Self {
            cursor: ListCursor::new(items.len()).at(focus),
            items,
            rows_shown: None,
            cancellable: true,
            browse_disabled: false,
        }
    }

    /// The same menu where `Cancel` does nothing (and so plays nothing):
    /// for menus with nothing to back out of.
    #[must_use]
    pub fn without_cancel(mut self) -> Self {
        self.cancellable = false;
        self
    }

    /// The same menu where the focus also stops on disabled items, so the
    /// player can read what they do. Confirm on one still chooses nothing.
    #[must_use]
    pub fn browsing_disabled(mut self) -> Self {
        self.browse_disabled = true;
        self
    }

    /// The same menu showing at most `rows` rows: a longer one scrolls,
    /// its window following the focus by `scroll`.
    #[must_use]
    pub fn showing(mut self, rows: usize, scroll: Scroll) -> Self {
        self.rows_shown = Some(rows);
        self.cursor = self.cursor.scrolling(scroll);
        self.follow();
        self
    }

    /// The same menu focused on item `index`, if it exists and the focus
    /// may stop on it (it is enabled, or the menu is
    /// [`browsing_disabled`](Self::browsing_disabled)).
    #[must_use]
    pub fn focused(mut self, index: usize) -> Self {
        if self.items.get(index).is_some_and(|i| self.stops_on(i)) {
            self.cursor = self.cursor.at(index);
            self.follow();
        }
        self
    }

    /// The items, in order.
    pub fn items(&self) -> &[MenuItem] {
        &self.items
    }

    /// Whether the focus may stop on `item`.
    fn stops_on(&self, item: &MenuItem) -> bool {
        item.enabled || self.browse_disabled
    }

    /// Index of the focused item.
    pub fn focus(&self) -> usize {
        self.cursor.focus()
    }

    /// Handles one action, by what the game [takes it for](ActionSet::ui).
    /// Returns an event for [`UiAction::Confirm`] on an enabled item and
    /// for [`UiAction::Cancel`]; moves the focus for [`UiAction::Down`] and
    /// [`UiAction::Up`]; ignores everything else.
    pub fn handle<A: ActionSet>(&mut self, action: A) -> Option<MenuEvent> {
        match action.ui() {
            Some(UiAction::Down) => self.step(true),
            Some(UiAction::Up) => self.step(false),
            Some(UiAction::Confirm) => {
                let focus = self.focus();
                return self
                    .items
                    .get(focus)
                    .filter(|i| i.enabled)
                    .map(|_| MenuEvent::Chosen(focus));
            }
            Some(UiAction::Cancel) if self.cancellable => return Some(MenuEvent::Cancelled),
            _ => {}
        }
        None
    }

    /// [`handle`](Self::handle), playing the [`MenuSound`]s on `audio`:
    /// Move when the focus moves, Select when an item is chosen, Cancel
    /// when cancelled, Denied for Confirm on a disabled item. Any other
    /// action that does nothing (a move with nowhere to go) plays nothing.
    pub fn handle_with_sound<A: ActionSet, V>(
        &mut self,
        action: A,
        audio: &mut AudioQueue<V>,
    ) -> Option<MenuEvent> {
        let before = self.focus();
        let event = self.handle(action);
        let sound = match event {
            Some(MenuEvent::Chosen(_)) => Some(MenuSound::Select),
            Some(MenuEvent::Cancelled) => Some(MenuSound::Cancel),
            None if self.focus() != before => Some(MenuSound::Move),
            None if action.ui() == Some(UiAction::Confirm) && !self.items.is_empty() => {
                Some(MenuSound::Denied)
            }
            None => None,
        };
        if let Some(sound) = sound {
            audio.menu(sound);
        }
        event
    }

    /// Moves the focus to the next (or previous) item it may stop on
    /// (enabled, unless the menu browses disabled ones too), wrapping.
    /// Stays put if there is none.
    fn step(&mut self, down: bool) {
        let (items, browse) = (&self.items, self.browse_disabled);
        if self.cursor.step(down, true, |i| items[i].enabled || browse) {
            self.follow();
        }
    }

    /// Moves the window to keep the focus in it, if the menu scrolls.
    fn follow(&mut self) {
        if let Some(rows) = self.rows_shown {
            self.cursor.window(rows);
        }
    }

    /// What the menu shows: every item, which one has the focus, and the
    /// window of them the box shows.
    pub fn view(&self) -> MenuView {
        let focus = self.focus();
        let items = self.items.iter().enumerate().map(|(i, item)| MenuItemView {
            label: item.label.clone(),
            suffix: item.suffix.clone(),
            note: item.note.clone(),
            enabled: item.enabled,
            focused: i == focus,
        });
        let n = self.items.len();
        let (first_shown, shown) = match self.rows_shown {
            Some(rows) => {
                let first = self.cursor.first_shown();
                (first, rows.max(1).min(n - first.min(n)))
            }
            None => (0, n),
        };
        MenuView {
            items: items.collect(),
            first_shown,
            shown,
        }
    }
}

#[cfg(test)]
mod tests;
