//! Dialogue: a scene of the `.dlg` format played one text box at a time
//! ([`player`], with the game's own lines and tokens through
//! [`DialogueHooks`]) and the text box it is read in ([`text_box`]: its
//! pages, typing and look). A game's dialogue screen puts them together
//! with its portraits and whatever it draws behind the talk. A [`preview`]
//! plays a folder's scenes in that screen for a writer.

pub mod player;
pub mod preview;
pub mod text_box;

pub use player::{DialogueHooks, DialoguePlayer, Named, Plain, Portrait, View};
pub use text_box::{SkipView, Step, TextBox, TextView, WaitView};
