//! Reusable pieces screens are built from: a [`menu`], the [`ListCursor`]
//! that moves its focus (and any other list's), a question over the screen
//! ([`Ask`]), a name typed or spelled on a letter grid
//! ([`name_entry`]) and [`word_wrap`] for text boxes. Each widget says what it
//! shows as plain data (a view) and a skin paints it (`menu::glyph` and
//! `ask::glyph` for glyphs).

pub mod ask;
pub mod list;
pub mod menu;
pub mod name_entry;
pub mod wrap;

pub use ask::{AnswerView, Ask, AskView};
pub use list::{ListCursor, Scroll};
pub use menu::{Menu, MenuEvent, MenuItem, MenuItemView, MenuView};
pub use name_entry::{NameBox, NameEntry, NameEvent, NameRules};
pub use wrap::word_wrap;
