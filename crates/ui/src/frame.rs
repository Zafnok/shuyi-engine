//! The frame (game ADR-0003): the [`GlyphBuffer`] console every screen draws
//! into, [`ConsoleSpec`] cells big, with its pictures and rectangles
//! ([`Item`]s, in [`Layer`]s) and its [`Backdrop`]; colors by name
//! ([`Rgb`], [`Palette`]); the text form snapshot tests read
//! ([`snapshot`]); fitting the console to a window ([`console`]), a shot
//! to a backdrop ([`cinema`]) and a picture to an area ([`portrait`]);
//! pictures a skin paints at run time ([`painted`], ADR-0078); the debug
//! view's shapes ([`gizmo`], ADR-0085), drawn by the integer pixel rules
//! of [`shape`]; and
//! the one-bit renderer ([`onebit`]); with the `raster` feature, the
//! software painter whose picture every backend must match
//! ([`raster`]).
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
pub mod gizmo;
pub mod glyph_buffer;
pub mod onebit;
pub mod painted;
pub mod portrait;
#[cfg(any(test, feature = "raster"))]
pub mod raster;
pub mod shape;
pub mod snapshot;

pub use color::{ColorSet, EngineColor, Palette, Rgb};
pub use console::ConsoleSpec;
pub use gizmo::{GizmoItem, GizmoRole, GizmoShape, LabelText};
pub use glyph_buffer::{
    Backdrop, BoxStyle, Cell, GlyphBuffer, Item, Layer, Overlay, Paint, PxRect, Rect, Sprite,
};
pub use painted::{Painted, PaintedId, PaintedKind, PaintedSprite, PaintedTable};

#[cfg(test)]
mod tests;
