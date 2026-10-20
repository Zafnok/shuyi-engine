//! What a question shows, as plain data: no cells, glyphs, colours or
//! positions. [`Ask::view`](super::Ask::view) builds it and a skin paints
//! it ([`super::glyph::paint`] today).

use crate::prelude::*;

/// A question, as it is this frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskView<T> {
    /// The question.
    pub text: String,
    /// The answers, in order; none for a question answered by its keys
    /// alone ([`keys`](Self::keys)).
    pub answers: Vec<AnswerView<T>>,
    /// The line naming the keys that answer (a yes/no's "f yes / d no"),
    /// shown instead of the answers.
    pub keys: Option<String>,
}

impl<T: Copy> AskView<T> {
    /// The answer in focus, if answers are shown.
    pub fn focused(&self) -> Option<T> {
        self.answers.iter().find(|a| a.focused).map(|a| a.answer)
    }
}

/// One answer to a question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerView<T> {
    /// Which answer it is.
    pub answer: T,
    /// Its name.
    pub label: String,
    /// Whether the cursor is on it.
    pub focused: bool,
}
