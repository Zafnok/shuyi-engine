//! The frame (game ADR-0003): the [`GlyphBuffer`] console every screen draws
//! into, [`ConsoleSpec`] cells big, with its pictures and rectangles
//! ([`Item`]s, in [`Layer`]s) and its [`Backdrop`]; colors by name
//! ([`Rgb`], [`Palette`]); the text form snapshot tests read
//! ([`snapshot`]); fitting the console to a window ([`console`]), a shot
//! to a backdrop ([`cinema`]) and a picture to an area ([`portrait`]); and
//! the one-bit renderer ([`onebit`]).
//!
//! The drawing helpers skins share ([`draw`]): [`GlyphBuffer::panel`] (a
//! box with a title in its border), [`GlyphBuffer::meter`] (a bar that
//! shows how full something is), [`GlyphBuffer::print_centred`] and
//! [`GlyphBuffer::print_cut`] (a line centred in a rectangle, or cut to a
//! width), next to [`GlyphBuffer::print`], [`GlyphBuffer::fill_rect`] and
//! [`GlyphBuffer::draw_box`].
//!
//! The game picks the console's size when it starts; nothing here knows
//! it.

pub mod cinema;
pub mod color;
pub mod console;
pub mod draw;
pub mod glyph_buffer;
pub mod onebit;
pub mod portrait;
pub mod snapshot;

pub use color::{ColorSet, EngineColor, Palette, Rgb};
pub use console::ConsoleSpec;
pub use glyph_buffer::{
    Backdrop, BoxStyle, Cell, GlyphBuffer, Item, Layer, Overlay, Paint, PxRect, Rect, Sprite,
};

#[cfg(test)]
mod tests;
