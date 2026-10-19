//! What the Options screen shows, as plain data: no cells, glyphs, colours
//! or positions. The screen builds an [`OptionsView`] each frame
//! ([`OptionsScreen::view`](super::OptionsScreen::view)) and a skin paints
//! it ([`super::glyph::paint`] today). Swapping the look (a bought UI
//! pack) means another painter of the same view; the screen's logic and
//! its tests of what happened don't change.

use super::LeaveChoice;
use crate::prelude::*;

/// The whole screen, as it is this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OptionsView {
    /// The screen's title.
    pub title: String,
    /// The rows, top to bottom.
    pub rows: Vec<RowView>,
    /// A message for the player (something was done, or couldn't be).
    pub message: Option<String>,
    /// The yes/no question open over the rows, if any.
    pub question: Option<QuestionView>,
    /// The question asked on leaving with changes, if it is open.
    pub leaving: Option<LeaveView>,
    /// The help line: what the keys do here.
    pub help: String,
}

impl OptionsView {
    /// The focused row.
    pub fn focused(&self) -> Option<&RowView> {
        self.rows.iter().find(|r| r.focused)
    }

    /// The view of the row whose key is `key`, if it is shown. A game may
    /// name its rows by its own type, turned into their keys.
    pub fn row(&self, key: impl Into<&'static str>) -> Option<&RowView> {
        let key = key.into();
        self.rows.iter().find(|r| r.key == key)
    }
}

/// One row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowView {
    /// Which row it is: the text key of its label.
    pub key: &'static str,
    /// Its name.
    pub label: String,
    /// What it shows beside its name.
    pub value: ValueView,
    /// Whether the cursor is on it.
    pub focused: bool,
    /// Whether it is the first row of a group (a skin sets groups apart).
    pub starts_group: bool,
}

/// What a row shows beside its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueView {
    /// Nothing: the row only does something.
    None,
    /// A value in words.
    Text {
        /// The words.
        text: String,
        /// Whether the cursor's left and right keys change it.
        adjustable: bool,
    },
    /// A volume, for a slider.
    Volume {
        /// The volume, 0 to `max`.
        level: u8,
        /// The loudest.
        max: u8,
    },
    /// The box that takes a volume's exact number, open on this row.
    NumberBox {
        /// The digits in it so far.
        digits: String,
        /// Whether it is being typed in (a caret shows where); otherwise
        /// the cursor steps the number.
        typing: bool,
    },
}

impl ValueView {
    /// The value in words, if it is one.
    pub fn text(&self) -> Option<&str> {
        match self {
            ValueView::Text { text, .. } => Some(text),
            _ => None,
        }
    }
}

/// A yes/no question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionView {
    /// The question.
    pub text: String,
    /// How to answer: the keys for yes and no.
    pub answers: String,
}

/// The question asked on leaving with changes: keep them, throw them away
/// or stay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveView {
    /// The question.
    pub text: String,
    /// The answers, in order.
    pub choices: Vec<ChoiceView>,
}

impl LeaveView {
    /// The answer in focus.
    pub fn focused(&self) -> Option<LeaveChoice> {
        self.choices.iter().find(|c| c.focused).map(|c| c.choice)
    }
}

/// One answer to a question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChoiceView {
    /// Which answer it is.
    pub choice: LeaveChoice,
    /// Its name.
    pub label: String,
    /// Whether the cursor is on it.
    pub focused: bool,
}
