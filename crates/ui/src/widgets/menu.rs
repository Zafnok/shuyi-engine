//! A vertical list menu: a title screen's, an action menu, a list of
//! replies.
//!
//! The menu holds its items and its focus (a [`ListCursor`]). What it
//! shows is
//! [`Menu::view`], plain data, and [`glyph::paint`] draws that view as
//! glyphs: the screens' own skins call it for their menu.
//!
//! Each item stands for a value of the game's type, which the menu gives
//! back when it is chosen, so a screen needs no list beside the menu to
//! know what was picked. Its label is a [`Label`]: a text key the skin
//! puts in the player's language when it paints, or text as it is, so a
//! menu is built once, by logic that doesn't know the language.
//!
//! ```
//! use shuyi_assets::keymap::sample::Action;
//! use shuyi_ui::widgets::{Menu, MenuEvent, MenuItem};
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! enum Seed {
//!     Turnip,
//!     Melon,
//! }
//!
//! let mut seeds = Menu::new(vec![
//!     MenuItem::keyed("seed.turnip").with_value(Seed::Turnip),
//!     MenuItem::keyed("seed.melon").with_value(Seed::Melon),
//! ]);
//! seeds.handle(Action::CursorDown);
//! assert_eq!(
//!     seeds.handle(Action::Confirm),
//!     Some(MenuEvent::Chosen(Seed::Melon))
//! );
//! ```

pub mod glyph;
pub mod view;

use crate::prelude::*;
pub use view::{MenuItemView, MenuView, SuffixKind};

use core::fmt::Display;

use shuyi_assets::keymap::{ActionSet, UiAction};

use super::list::{ListCursor, Scroll};
use crate::audio::{AudioQueue, MenuSound};
pub use crate::text::Label;

/// One menu entry, standing for a `value` of the game's type (`()` for
/// a menu whose screen only needs to know an item was chosen).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem<T = ()> {
    /// What the item stands for: what [`MenuEvent::Chosen`] gives back.
    pub value: T,
    /// Text shown: a text key the skin puts in the player's language, or
    /// text as it is.
    pub label: Label,
    /// Disabled items are drawn dim, skipped by the cursor and can't be chosen.
    pub enabled: bool,
    /// Text shown after the label (e.g. `(broken)`), and what it is.
    pub suffix: Option<(String, SuffixKind)>,
    /// What the item does, shown under the list while it has the focus
    /// (a description in a list).
    pub note: Option<String>,
}

impl MenuItem {
    /// An enabled item showing `label` as it is.
    pub fn new(label: impl Into<String>) -> Self {
        Self::labelled(Label::Text(label.into()))
    }

    /// An enabled item showing the text for `key` in the player's
    /// language. Give its placeholders values with
    /// [`with_arg`](Self::with_arg).
    pub fn keyed(key: impl Into<String>) -> Self {
        Self::labelled(Label::key(key))
    }

    /// A disabled item showing `label` as it is.
    pub fn disabled(label: impl Into<String>) -> Self {
        Self::new(label).with_enabled(false)
    }

    /// An enabled item showing `label`.
    pub fn labelled(label: Label) -> Self {
        Self {
            value: (),
            label,
            enabled: true,
            suffix: None,
            note: None,
        }
    }
}

impl<T> MenuItem<T> {
    /// The same item standing for `value`.
    #[must_use]
    pub fn with_value<U>(self, value: U) -> MenuItem<U> {
        MenuItem {
            value,
            label: self.label,
            enabled: self.enabled,
            suffix: self.suffix,
            note: self.note,
        }
    }

    /// The same item with the placeholder `{name}` of its keyed label
    /// filled by `value` (a label of text as it is stays as it is).
    #[must_use]
    pub fn with_arg(mut self, name: impl Into<String>, value: impl Display) -> Self {
        self.label = self.label.with_arg(name, value);
        self
    }

    /// The same item, enabled or not.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
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
}

/// What a menu reports back to its screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuEvent<T = ()> {
    /// Confirm on an enabled item: the value it stands for. (Its index is
    /// still the menu's [`focus`](Menu::focus).)
    Chosen(T),
    /// Cancel was pressed.
    Cancelled,
}

/// A vertical list of items in a single-line box. `CursorDown`/`CursorUp`
/// move the focus (wrapping, skipping disabled items), `Confirm` chooses the
/// focused item, `Cancel` cancels (unless the menu is
/// [`without_cancel`](Self::without_cancel)). A menu
/// [`showing`](Self::showing) fewer rows than it has scrolls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu<T = ()> {
    items: Vec<MenuItem<T>>,
    cursor: ListCursor,
    /// How many rows the box shows (all of them if `None`).
    rows_shown: Option<usize>,
    /// Whether `Cancel` reports [`MenuEvent::Cancelled`].
    cancellable: bool,
    /// Whether the focus also stops on disabled items, so their notes can
    /// be read (they still can't be chosen).
    browse_disabled: bool,
}

impl<T> Menu<T> {
    /// What a menu reacts to ([`handle`](Self::handle)), for the screen
    /// that shows it to declare ([`Screen::actions`](crate::screen::Screen::actions)).
    pub const ACTIONS: &'static [UiAction] = &[
        UiAction::Up,
        UiAction::Down,
        UiAction::Confirm,
        UiAction::Cancel,
    ];

    /// A menu focused on its first enabled item (or the first item, if none
    /// is enabled).
    pub fn new(items: Vec<MenuItem<T>>) -> Self {
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
    pub fn items(&self) -> &[MenuItem<T>] {
        &self.items
    }

    /// The value item `index` stands for, if it exists.
    pub fn value(&self, index: usize) -> Option<&T> {
        self.items.get(index).map(|i| &i.value)
    }

    /// Whether the focus may stop on `item`.
    fn stops_on(&self, item: &MenuItem<T>) -> bool {
        item.enabled || self.browse_disabled
    }

    /// Index of the focused item.
    pub fn focus(&self) -> usize {
        self.cursor.focus()
    }

    /// Handles one action, by what the game [takes it for](ActionSet::ui).
    /// Returns an event for [`UiAction::Confirm`] on an enabled item (with
    /// a copy of its value) and for [`UiAction::Cancel`]; moves the focus
    /// for [`UiAction::Down`] and [`UiAction::Up`]; ignores everything else.
    pub fn handle<A: ActionSet>(&mut self, action: A) -> Option<MenuEvent<T>>
    where
        T: Clone,
    {
        match action.ui() {
            Some(UiAction::Down) => self.step(true),
            Some(UiAction::Up) => self.step(false),
            Some(UiAction::Confirm) => {
                let focus = self.focus();
                return self
                    .items
                    .get(focus)
                    .filter(|i| i.enabled)
                    .map(|i| MenuEvent::Chosen(i.value.clone()));
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
    ) -> Option<MenuEvent<T>>
    where
        T: Clone,
    {
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
            // The cursor keeps its window inside the list.
            Some(rows) => (self.cursor.first_shown(), rows.max(1).min(n)),
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
