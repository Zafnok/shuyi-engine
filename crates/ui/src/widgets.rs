//! Reusable pieces screens are built from: a [`menu`] and [`word_wrap`]
//! for text boxes. Each widget says what it shows as plain data (a view)
//! and a skin paints it (`menu::glyph` for glyphs).

pub mod menu;
pub mod wrap;

pub use menu::{Menu, MenuEvent, MenuItem, MenuItemView, MenuView};
pub use wrap::word_wrap;
