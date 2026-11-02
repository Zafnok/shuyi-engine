//! The system's own keyboard for a text box (ticket 3003): a console's, or
//! Steam's on-screen one on the Steam Deck. A screen asks for it through
//! [`Ctx::system_keyboard`](crate::Ctx::system_keyboard), the platform's
//! `App` opens it (`shuyi_platform::TextInput`), and the player's answer
//! comes back in a later frame. Where the platform has none, a screen reads
//! typed keys ([`FrameInput::text`](crate::FrameInput::text)) or spells
//! on a letter grid, as before.
//!
//! The handshake, one frame at a time:
//!
//! 1. before each frame the platform says whether it has a keyboard
//!    ([`set_present`](SystemKeyboard::set_present));
//! 2. a screen [`ask`](SystemKeyboard::ask)s;
//! 3. after the frame the platform takes the ask
//!    ([`take_ask`](SystemKeyboard::take_ask)) and opens its keyboard;
//! 4. once the player closes it, the platform hands back the answer
//!    ([`answer`](SystemKeyboard::answer)) before a frame, and the screen
//!    takes it ([`take_answer`](SystemKeyboard::take_answer)).

use crate::prelude::*;

/// What a text box asks the system keyboard for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAsk {
    /// What the text is for, shown with the keyboard ("Name"), already in
    /// the player's language.
    pub prompt: String,
    /// The text so far, which the player starts from.
    pub text: String,
    /// The most characters the box takes.
    pub max_chars: u16,
}

/// What the player did with the system keyboard.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TextAnswer {
    /// They typed this and confirmed it: the whole text, not a change.
    Typed(String),
    /// They closed it without confirming, or it couldn't open.
    Cancelled,
}

/// The system keyboard as the screens see it: whether there is one, the
/// ask waiting for the platform and the answer waiting for the screen.
/// Absent until the platform says otherwise.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemKeyboard {
    present: bool,
    /// Asked for, not yet taken by the platform.
    asked: Option<TextAsk>,
    /// Taken by the platform, not yet answered.
    open: bool,
    /// Answered, not yet taken by the screen.
    answer: Option<TextAnswer>,
}

impl SystemKeyboard {
    /// Whether the platform has a system keyboard.
    pub fn is_present(&self) -> bool {
        self.present
    }

    /// Asks for the keyboard with `ask`. Without one nothing is asked and
    /// this is `false`: the screen takes typed keys instead. Asking again
    /// before the platform opened it replaces the ask; asking while it is
    /// open does nothing, and is `false`.
    pub fn ask(&mut self, ask: TextAsk) -> bool {
        if !self.present || self.open {
            return false;
        }
        self.asked = Some(ask);
        true
    }

    /// Whether an ask waits for the player: asked for and not yet
    /// answered.
    pub fn is_waiting(&self) -> bool {
        self.asked.is_some() || self.open
    }

    /// The player's answer, once: `None` until they close the keyboard.
    pub fn take_answer(&mut self) -> Option<TextAnswer> {
        self.answer.take()
    }

    /// The platform says whether it has a keyboard, before each frame. One
    /// that goes away takes an ask with it: the screen hears
    /// [`TextAnswer::Cancelled`].
    pub fn set_present(&mut self, present: bool) {
        self.present = present;
        if !present && self.is_waiting() {
            self.answer(TextAnswer::Cancelled);
        }
    }

    /// The ask to open, once, for the platform: from then on the keyboard
    /// is open until [`answer`](Self::answer).
    pub fn take_ask(&mut self) -> Option<TextAsk> {
        let ask = self.asked.take()?;
        self.open = true;
        Some(ask)
    }

    /// The platform hands back what the player did, which closes the
    /// keyboard.
    pub fn answer(&mut self, answer: TextAnswer) {
        self.asked = None;
        self.open = false;
        self.answer = Some(answer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn name(text: &str) -> TextAsk {
        TextAsk {
            prompt: "Name".into(),
            text: text.into(),
            max_chars: 12,
        }
    }

    #[test]
    fn without_a_keyboard_nothing_is_asked() {
        let mut keyboard = SystemKeyboard::default();
        assert!(!keyboard.is_present());
        assert!(!keyboard.ask(name("")));
        assert!(!keyboard.is_waiting());
        assert_eq!(keyboard.take_ask(), None);
    }

    #[test]
    fn an_ask_goes_to_the_platform_and_its_answer_back_once() {
        let mut keyboard = SystemKeyboard::default();
        keyboard.set_present(true);
        assert!(keyboard.is_present());
        assert!(keyboard.ask(name("A")));
        // Asked again before the platform took it: the later ask wins.
        assert!(keyboard.ask(name("Ab")));
        assert!(keyboard.is_waiting());
        assert_eq!(keyboard.take_ask(), Some(name("Ab")));
        assert_eq!(keyboard.take_ask(), None);
        // Open: a new ask waits for this one's answer.
        assert!(keyboard.is_waiting());
        assert!(!keyboard.ask(name("C")));
        assert_eq!(keyboard.take_answer(), None);
        keyboard.answer(TextAnswer::Typed("Abe".into()));
        assert!(!keyboard.is_waiting());
        assert_eq!(
            keyboard.take_answer(),
            Some(TextAnswer::Typed("Abe".into()))
        );
        assert_eq!(keyboard.take_answer(), None);
        // Closed: it can be asked for again.
        assert!(keyboard.ask(name("Abe")));
    }

    #[test]
    fn a_keyboard_that_goes_away_cancels_what_waits() {
        for open in [false, true] {
            let mut keyboard = SystemKeyboard::default();
            keyboard.set_present(true);
            assert!(keyboard.ask(name("")));
            if open {
                assert!(keyboard.take_ask().is_some());
            }
            keyboard.set_present(false);
            assert!(!keyboard.is_waiting(), "open: {open}");
            assert_eq!(keyboard.take_ask(), None, "open: {open}");
            assert_eq!(keyboard.take_answer(), Some(TextAnswer::Cancelled));
        }
        // With nothing waiting, nothing is answered.
        let mut keyboard = SystemKeyboard::default();
        keyboard.set_present(true);
        keyboard.set_present(false);
        assert_eq!(keyboard.take_answer(), None);
    }
}
