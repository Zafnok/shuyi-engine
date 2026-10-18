//! Screens any game can use, each with rows, entries or a legend the game
//! gives: the [`layout_picker`], the [`key_bindings`] screen and the
//! [`options`] screen. Each
//! says what it shows as plain data (a view) and a skin paints it
//! (`glyph` for glyphs).

pub mod key_bindings;
pub mod layout_picker;
pub mod options;

use crate::prelude::*;

use crate::ctx::{Ctx, GameData};
use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::fixed;

/// The text key of a text box's help line, with `{done}`, `{delete}` and
/// `{cancel}`: the text box's fixed keys.
pub const TEXT_BOX_HELP: &str = "text_box.help";

/// The help line of a text box: `Enter done · Backspace delete · Escape
/// cancel`, in `ctx`'s language.
pub fn text_keys_help<G: GameData>(ctx: &Ctx<G>) -> String {
    let [done, delete, cancel] = fixed::text_key_names();
    ctx.text_with(
        TEXT_BOX_HELP,
        &[("done", &done), ("delete", &delete), ("cancel", &cancel)],
    )
}

/// Column at which `width` cells are centred in `buf` (rounded left).
pub fn centre_x(buf: &GlyphBuffer, width: usize) -> i32 {
    let width = i32::try_from(width).unwrap_or(i32::MAX);
    (i32::from(buf.width()) - width) / 2
}

/// Prints `text` centred on row `y`.
pub fn print_centred(buf: &mut GlyphBuffer, y: i32, text: &str, fg: Rgb, bg: Rgb) {
    let x = centre_x(buf, text.chars().count());
    buf.print(x, y, text, fg, bg);
}

#[cfg(test)]
mod tests;
