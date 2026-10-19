//! What the layout picker shows, as plain data: no cells, glyphs, colours
//! or positions. The screen builds a [`LayoutPickerView`] each frame
//! ([`LayoutPickerScreen::view`](super::LayoutPickerScreen::view)) and a
//! skin paints it ([`super::glyph::paint`] today).

use crate::prelude::*;
use shuyi_assets::keymap::{Key, Layout};

/// The whole screen, as it is this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutPickerView {
    /// The screen's title.
    pub title: String,
    /// One entry per layout, in the order they are offered.
    pub layouts: Vec<LayoutView>,
    /// The help line: what the keys do here.
    pub help: String,
}

impl LayoutPickerView {
    /// The layout the cursor is on.
    pub fn focused(&self) -> Option<&LayoutView> {
        self.layouts.iter().find(|l| l.focused)
    }
}

/// One layout, with its keyboard and what its keys do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutView {
    /// Which layout it is.
    pub layout: Layout,
    /// Its name.
    pub name: String,
    /// Whether the cursor is on it.
    pub focused: bool,
    /// What its keys do, one line per thing the player does.
    pub legend: Vec<LegendRow>,
    /// Every key of the keyboard with its role in this layout. A skin draws
    /// the keys its picture has and where it puts them.
    pub keys: Vec<KeyCapView>,
}

impl LayoutView {
    /// The role of `key` in this layout ([`KeyRole::Unbound`] for a key
    /// the view doesn't list).
    pub fn role(&self, key: Key) -> KeyRole {
        let cap = self.keys.iter().find(|cap| cap.key == key);
        cap.map_or(KeyRole::Unbound, |cap| cap.role)
    }
}

/// One line of a legend: the keys, and what they do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegendRow {
    /// The keys' names (`! not mapped` if there are none).
    pub keys: String,
    /// What they do.
    pub what: String,
    /// Whether they move the cursor (a skin sets those apart).
    pub movement: bool,
}

/// One key of the keyboard and what it does in a layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyCapView {
    /// The key.
    pub key: Key,
    /// What it does.
    pub role: KeyRole,
}

/// What a key does in a layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyRole {
    /// Moves the cursor (a key that repeats while held).
    Movement,
    /// Does something else.
    Bound,
    /// Does nothing.
    Unbound,
}
