//! A question over the screen: a line of text and the answers to it, one
//! in focus ("Keep these changes? Keep · Discard · Back"), or a yes/no
//! answered by Confirm and Cancel alone, its keys named under it
//! ([`Ask::yes_no`]).
//!
//! The screen holds an [`Ask`] while the question is open and hands it
//! each action ([`Ask::handle`]): the cursor moves between the answers,
//! Confirm gives the focused one and Cancel the answer it stands for, if
//! any. What the question shows is [`Ask::view`], plain data, and
//! [`glyph::paint`] draws that view in a box over whatever is under it.
//! A farming game's "Sell all 12 turnips?" and a platformer's "Restart the
//! level?" are the same widget.

pub mod glyph;
pub mod view;

use crate::prelude::*;
use core::fmt::Display;
pub use view::{AnswerView, AskView};

use shuyi_assets::keymap::{ActionSet, UiAction};

use super::list::ListCursor;
use crate::audio::{AudioQueue, MenuSound};
use crate::ctx::{Ctx, GameData};
use crate::input::help_keys::HelpKeys;

/// A question and its answers, each a `T` the screen acts on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask<T> {
    /// The text key of the question.
    key: &'static str,
    /// The answers in the order shown, each with the text key of its name.
    answers: Vec<(T, &'static str)>,
    /// Which answer is in focus.
    cursor: ListCursor,
    /// What Cancel answers; `None`: Cancel does nothing.
    cancel: Option<T>,
    /// The text key of the line naming the keys that answer, shown instead
    /// of the answers (a yes/no).
    keys: Option<&'static str>,
}

impl<T: Copy + PartialEq> Ask<T> {
    /// What a question reacts to ([`handle`](Self::handle)), for the
    /// screen that shows it to declare
    /// ([`Screen::actions`](crate::screen::Screen::actions)).
    pub const ACTIONS: &'static [UiAction] = crate::input::NAVIGATION;

    /// The question of the text key `key` with `answers` (each an answer
    /// and the text key of its name), the first in focus. Cancel does
    /// nothing until [`cancelled_by`](Self::cancelled_by) names an answer.
    pub fn new(key: &'static str, answers: impl IntoIterator<Item = (T, &'static str)>) -> Self {
        let answers: Vec<_> = answers.into_iter().collect();
        Self {
            key,
            cursor: ListCursor::new(answers.len()),
            answers,
            cancel: None,
            keys: None,
        }
    }

    /// The same question focused on `answer` (on the first while it isn't
    /// one of the answers).
    #[must_use]
    pub fn focused_on(mut self, answer: T) -> Self {
        let i = self.answers.iter().position(|&(a, _)| a == answer);
        self.cursor = self.cursor.at(i.unwrap_or(0));
        self
    }

    /// The same question, which Cancel answers with `answer` (going back,
    /// say).
    #[must_use]
    pub fn cancelled_by(mut self, answer: T) -> Self {
        self.cancel = Some(answer);
        self
    }

    /// The text key of the question; also what names it.
    pub fn key(&self) -> &'static str {
        self.key
    }

    /// The answer in focus; `None` with no answers.
    pub fn focus(&self) -> Option<T> {
        self.answers.get(self.cursor.focus()).map(|&(a, _)| a)
    }

    /// What Cancel answers, if anything.
    pub fn cancel(&self) -> Option<T> {
        self.cancel
    }

    /// An action while the question is open. The cursor's keys move the
    /// focus to the next answer (right and down) or the one before (left
    /// and up), going round at the ends, with the menu's move sound on
    /// `audio`. Confirm gives the answer in focus, Cancel the answer it
    /// stands for ([`cancelled_by`](Self::cancelled_by)); anything else
    /// gives nothing. An answer's own sound is the screen's to play: what
    /// it does may still be refused.
    pub fn handle<A: ActionSet, V>(&mut self, action: A, audio: &mut AudioQueue<V>) -> Option<T> {
        match action.ui()? {
            UiAction::Up | UiAction::Left => self.step(false, audio),
            UiAction::Down | UiAction::Right => self.step(true, audio),
            UiAction::Confirm => return self.focus(),
            UiAction::Cancel => return self.cancel,
            UiAction::Debug => {}
        }
        None
    }

    /// Moves the focus one answer on (`forward`) or back, wrapping.
    fn step<V>(&mut self, forward: bool, audio: &mut AudioQueue<V>) {
        if self.cursor.step(forward, true, |_| true) {
            audio.menu(MenuSound::Move);
        }
    }

    /// The question as it is now, as plain data for a skin to paint
    /// ([`glyph::paint`]): its text and its answers, or the line naming
    /// its keys, in the player's language naming the active keys.
    pub fn view<G: GameData>(&self, ctx: &Ctx<G>) -> AskView<T> {
        self.view_with_keys(ctx, ctx.help_keys(), &[])
    }

    /// [`view`](Self::view), naming the keys `keys` has rather than the
    /// active ones, and filling the question's placeholders from `args`
    /// ([`Ctx::text_with_keys`]).
    pub fn view_with_keys<G: GameData>(
        &self,
        ctx: &Ctx<G>,
        keys: HelpKeys<'_, G::Action>,
        args: &[(&str, &dyn Display)],
    ) -> AskView<T> {
        let focus = self.cursor.focus();
        let answers = self
            .answers
            .iter()
            .enumerate()
            .map(|(i, &(answer, key))| AnswerView {
                answer,
                label: ctx.text(key).to_owned(),
                focused: i == focus,
            });
        AskView {
            text: ctx.text_with_keys(self.key, keys, args),
            answers: match self.keys {
                Some(_) => Vec::new(),
                None => answers.collect(),
            },
            keys: self.keys.map(|key| ctx.text_with_keys(key, keys, &[])),
        }
    }
}

impl Ask<bool> {
    /// A yes/no question of the text key `key`: Confirm answers yes
    /// (`true`) and Cancel no (`false`). It shows no answers to move
    /// between, but the line of the text key `keys`, which names the keys
    /// for yes and no.
    pub fn yes_no(key: &'static str, keys: &'static str) -> Self {
        Self {
            keys: Some(keys),
            ..Self::new(key, [(true, keys)]).cancelled_by(false)
        }
    }
}

#[cfg(test)]
mod tests;
