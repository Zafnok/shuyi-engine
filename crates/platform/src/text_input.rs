//! [`TextInput`]: the system's own keyboard, for a device where the player
//! doesn't type on keys (a console's software keyboard, Steam's on-screen
//! one on the Steam Deck, which is the game's to fill: its ticket 0903).
//!
//! A screen asks through its context
//! ([`Ctx::system_keyboard`](shuyi_ui::Ctx::system_keyboard)); `App` opens
//! the keyboard after the frame that asked and hands the answer back
//! before a later one ([`shuyi_ui::system_keyboard`] has the handshake).

pub use shuyi_ui::system_keyboard::{TextAnswer, TextAsk};

/// A system keyboard a backend offers ([`Platform::text_input`]).
///
/// [`Platform::text_input`]: crate::Platform::text_input
pub trait TextInput {
    /// Shows the keyboard for `ask`: its prompt, the text to start from and
    /// the most characters. `false` if it can't now (the system refused);
    /// the screen then hears [`TextAnswer::Cancelled`].
    fn open(&mut self, ask: &TextAsk) -> bool;

    /// What the player did, once they closed the keyboard; `None` while it
    /// is open, or when none is.
    fn poll(&mut self) -> Option<TextAnswer>;
}
