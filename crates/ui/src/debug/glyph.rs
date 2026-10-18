//! The glyph look of the debug screens: the debug menu ([`paint`]), the
//! glyph sampler ([`sampler`]), the scene camera ([`paint_camera`]) and the
//! Debug key's hint ([`paint_hint`]).

use alloc::rc::Rc;

use super::camera::SceneCameraView;
use super::{DebugMenuView, SamplePanels};
use crate::ctx::{Ctx, GameData};
use crate::frame::color::{EngineColor, Palette};
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::prelude::*;
use crate::screens::{centre_x, print_centred};
use crate::widgets::menu;

/// Row of the debug menu's title.
const MENU_TITLE_ROW: i32 = 9;

/// Paints the debug menu's `view` over the whole of `buf`: the title, the
/// menu centred under it and the help line at the bottom.
pub fn paint<G: GameData>(ctx: &Ctx<G>, view: &DebugMenuView, buf: &mut GlyphBuffer) {
    let c = |e| ctx.palette.engine(e);
    let black = c(EngineColor::Background);
    buf.fill_rect(buf.bounds(), Cell::new(' ', c(EngineColor::Text), black));
    let hi = c(EngineColor::Highlight);
    print_centred(buf, MENU_TITLE_ROW, &view.title, hi, black);
    let (w, _) = menu::glyph::size(&view.menu);
    let x = centre_x(buf, usize::try_from(w).unwrap_or(0));
    menu::glyph::paint(&ctx.palette, &view.menu, buf, x, MENU_TITLE_ROW + 2);
    let bottom = i32::from(buf.height()) - 1;
    print_centred(buf, bottom, &view.help, c(EngineColor::TextDim), black);
}

/// Left column of the scene camera's title and the lines under it.
const CAMERA_LEFT: i32 = 2;

/// Paints the scene camera's `view` over the whole of `buf`: the title,
/// the zoom and the help line; the picture through the window in a frame,
/// the caption over it and the text box over its corner.
pub fn paint_camera(palette: &Palette, view: &SceneCameraView, buf: &mut GlyphBuffer) {
    let c = |e| palette.engine(e);
    let (black, text, hi) = (
        c(EngineColor::Background),
        c(EngineColor::Text),
        c(EngineColor::Highlight),
    );
    buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
    buf.print(CAMERA_LEFT, 0, &view.title, hi, black);
    let bottom = i32::from(buf.height()) - 1;
    let dim = c(EngineColor::TextDim);
    buf.print(CAMERA_LEFT, bottom, &view.help, dim, black);
    buf.print(CAMERA_LEFT, 2, &view.info, text, black);
    let Some((picture, frame)) = &view.shown else {
        return;
    };

    let window = view.layout.window;
    let border = Rect::new(window.x - 1, window.y - 1, window.w + 2, window.h + 2);
    buf.draw_box(border, BoxStyle::Single, c(EngineColor::Border), black);
    buf.fill_rect(window, Cell::see_through(' ', text));
    buf.set_backdrop(Rc::clone(picture), window, frame.origin_px, frame.zoom);
    // Glyphs over the scene, with no box behind them.
    buf.print_fg(window.x + 1, window.y, &view.caption, hi);

    let text_box = view.layout.text_box;
    let panel = c(EngineColor::Panel);
    buf.fill_rect(text_box, Cell::new(' ', text, panel));
    buf.draw_box(text_box, BoxStyle::Double, c(EngineColor::Focus), panel);
    for (row, line) in (text_box.y + 1..).zip(&view.box_text) {
        buf.print(text_box.x + 2, row, line, text, panel);
    }
}

/// Prints `hint` (a screen's view holds the [`hint`](super::hint))
/// right-aligned on row `y`, one cell in from the edge, unless those cells
/// or the one before them already hold text.
pub fn paint_hint(palette: &Palette, hint: &str, buf: &mut GlyphBuffer, y: i32) {
    let len = i32::try_from(hint.chars().count()).unwrap_or(i32::MAX);
    let x = i32::from(buf.width()) - 1 - len;
    let free = (x - 1..x + len).all(|cx| buf.get(cx, y).is_some_and(|c| c.glyph == ' '));
    if free {
        buf.print(
            x,
            y,
            hint,
            palette.engine(EngineColor::TextDim),
            palette.engine(EngineColor::Background),
        );
    }
}

/// Glyphs per sampler row; each glyph is followed by a blank cell.
pub const GLYPHS_PER_ROW: usize = 48;
/// Width of one palette swatch column (`██ name`).
pub const SWATCH_W: i32 = 19;
/// Longest colour name shown in a swatch column; longer names are cut.
const SWATCH_NAME_W: usize = SWATCH_W as usize - 5;
/// Swatch columns across the console.
pub const SWATCH_COLUMNS: i32 = 5;

/// Row of the "Palette" heading: right after the glyph grid's own title and
/// rows, no blank row between. Computed from `glyphs` (rather than a fixed
/// constant) so the bottom half doesn't waste rows the top half didn't use,
/// however many glyphs the font ends up with.
pub fn palette_top(glyphs: &[char]) -> i32 {
    let glyph_rows: i32 = glyphs
        .chunks(GLYPHS_PER_ROW)
        .count()
        .try_into()
        .unwrap_or(i32::MAX);
    1 + glyph_rows
}

/// Row where the sample panels start, for `palette`'s size and `glyphs`'
/// row count: the palette title, its swatch rows, then the sample sentence,
/// with no blank rows between (every row here is scarce once the palette is
/// large).
pub fn panels_top(palette: &Palette, glyphs: &[char]) -> i32 {
    panels_top_for(palette.iter().count(), glyphs)
}

/// [`panels_top`] for a palette of `colours` colours.
pub fn panels_top_for(colours: usize, glyphs: &[char]) -> i32 {
    let swatch_rows: i32 = colours
        .div_ceil(SWATCH_COLUMNS as usize)
        .try_into()
        .unwrap_or(i32::MAX);
    palette_top(glyphs) + 2 + swatch_rows
}

/// A buffer of a console of `spec`: the top half shows every glyph in
/// `glyphs` in a grid; the bottom half shows every palette colour as a
/// swatch plus its name, a line of sample text and the game's `panels`.
pub fn sampler(
    spec: ConsoleSpec,
    palette: &Palette,
    glyphs: &[char],
    panels: SamplePanels,
) -> GlyphBuffer {
    let c = |e| palette.engine(e);
    let (text, dim, black) = (
        c(EngineColor::Text),
        c(EngineColor::TextDim),
        c(EngineColor::Background),
    );
    let mut b = GlyphBuffer::with_spec(spec, Cell::new(' ', text, black));

    // check-text: not player text (debug tools are English only)
    let title = format!("Glyphs ({})", glyphs.len());
    b.print(1, 0, &title, c(EngineColor::Highlight), black);
    for (row, chunk) in (1..).zip(glyphs.chunks(GLYPHS_PER_ROW)) {
        for (col, &g) in (0..).zip(chunk) {
            b.print(2 + col * 2, row, &g.to_string(), text, black);
        }
    }

    let bottom = palette_top(glyphs);
    let colors: Vec<_> = palette.iter().collect();
    // check-text: not player text (debug tools are English only)
    let title = format!("Palette ({})", colors.len());
    b.print(1, bottom, &title, c(EngineColor::Highlight), black);
    for (i, &(name, rgb)) in (0..).zip(&colors) {
        let (x, y) = (
            1 + i % SWATCH_COLUMNS * SWATCH_W,
            bottom + 1 + i / SWATCH_COLUMNS,
        );
        b.print(x, y, "██", rgb, black);
        let name: String = name.chars().take(SWATCH_NAME_W).collect();
        b.print(x + 3, y, &name, dim, black);
    }

    let panels_top = panels_top(palette, glyphs);
    b.print(
        1,
        panels_top - 1,
        // check-text: not player text (debug tools are English only)
        "The quick brown fox jumps over the lazy dog. 0123456789 ÀÉÎÕÜ àéîõü ß ¿¡ «» ← ↑ → ↓",
        text,
        black,
    );
    panels(&mut b, palette, panels_top);
    b
}
