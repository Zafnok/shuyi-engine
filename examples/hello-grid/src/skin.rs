//! The look of the screens: each paints its screen's view (plain data)
//! with glyphs in the palette's colors. The map itself is painted by the
//! engine's skins ([`crate::map`]); this paints the lines round it.

use shuyi_ui::dialogue::TextView;
use shuyi_ui::dialogue::text_box::{self, glyph::BoxPlace};
use shuyi_ui::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use shuyi_ui::frame::{EngineColor, Palette, Rgb};
use shuyi_ui::widgets::menu::{self, MenuView};

/// What the title shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleView {
    /// The game's name.
    pub name: String,
    /// The line under it.
    pub subtitle: String,
    /// The menu.
    pub menu: MenuView,
    /// The help line.
    pub help: String,
}

/// What the lines round the stage's map show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageView {
    /// The turn.
    pub turn: String,
    /// What is happening.
    pub message: String,
    /// The help line (empty while a piece walks).
    pub help: String,
    /// The question asked instead, with its answers, if one is.
    pub question: Option<(String, String)>,
}

/// What the end of a stage shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndView {
    /// How it ended.
    pub headline: String,
    /// Whether you won.
    pub won: bool,
    /// The menu.
    pub menu: MenuView,
    /// The help line.
    pub help: String,
}

/// The column that centres `text` in `buf`.
fn centred(buf: &GlyphBuffer, text: &str) -> i32 {
    let width = i32::from(buf.width());
    let len = i32::try_from(text.chars().count()).unwrap_or(width);
    (width - len).max(0) / 2
}

/// Fills `buf` with the background.
fn clear(palette: &Palette, buf: &mut GlyphBuffer) -> Rgb {
    let bg = palette.engine(EngineColor::Background);
    buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
    bg
}

/// `text` centred on row `y`.
fn line(buf: &mut GlyphBuffer, y: i32, text: &str, fg: Rgb, bg: Rgb) {
    let x = centred(buf, text);
    buf.print(x, y, text, fg, bg);
}

/// The help line, on the last row.
fn help(palette: &Palette, buf: &mut GlyphBuffer, text: &str, bg: Rgb) {
    let y = i32::from(buf.height()) - 1;
    line(buf, y, text, palette.engine(EngineColor::TextDim), bg);
}

/// `menu` centred across `buf`, its top on row `y`.
fn centred_menu(palette: &Palette, view: &MenuView, buf: &mut GlyphBuffer, y: i32) {
    let (w, _) = menu::glyph::size(view);
    let x = (i32::from(buf.width()) - w).max(0) / 2;
    menu::glyph::paint(palette, view, buf, x, y);
}

/// The title: the name in a box, the line under it, the menu, the help.
pub fn paint_title(palette: &Palette, view: &TitleView, buf: &mut GlyphBuffer) {
    let bg = clear(palette, buf);
    let c = |e| palette.engine(e);
    let width = i32::try_from(view.name.chars().count()).unwrap_or(0) + 6;
    let x = (i32::from(buf.width()) - width).max(0) / 2;
    buf.draw_box(
        Rect::new(x, 3, width, 3),
        BoxStyle::Double,
        c(EngineColor::Border),
        bg,
    );
    line(buf, 4, &view.name, c(EngineColor::Highlight), bg);
    line(buf, 7, &view.subtitle, c(EngineColor::TextDim), bg);
    centred_menu(palette, &view.menu, buf, 10);
    help(palette, buf, &view.help, bg);
}

/// Where the scene's text box goes: the bottom seven rows.
pub const TALK_BOX: BoxPlace = BoxPlace {
    frame: Rect::new(0, 25, 100, 7),
    text_x: 2,
    text_y: 27,
};

/// A scene's text box, with the speaker's name.
pub fn paint_talk(
    palette: &Palette,
    view: &TextView,
    speaker: Option<&str>,
    buf: &mut GlyphBuffer,
) {
    text_box::glyph::paint(palette, buf, TALK_BOX, view, speaker);
}

/// The lines round the stage's map: the turn and what is happening on the
/// top row, the help (or the question) on the bottom one.
pub fn paint_stage(palette: &Palette, view: &StageView, buf: &mut GlyphBuffer) {
    let c = |e| palette.engine(e);
    let bg = c(EngineColor::Panel);
    // The top and bottom rows are the cleared frame's: the panel's color.
    let shown = buf.print(1, 0, &view.turn, c(EngineColor::Highlight), bg);
    let x = 1 + i32::from(shown) + 3;
    buf.print(x, 0, &view.message, c(EngineColor::Text), bg);
    match &view.question {
        Some((question, answers)) => {
            let text = format!("{question}  {answers}");
            let y = i32::from(buf.height()) - 1;
            line(buf, y, &text, c(EngineColor::Warning), bg);
        }
        None => help(palette, buf, &view.help, bg),
    }
}

/// The end of a stage: the headline, the menu, the help.
pub fn paint_end(palette: &Palette, view: &EndView, buf: &mut GlyphBuffer) {
    let bg = clear(palette, buf);
    let color = if view.won {
        EngineColor::Accent
    } else {
        EngineColor::Warning
    };
    line(buf, 6, &view.headline, palette.engine(color), bg);
    centred_menu(palette, &view.menu, buf, 10);
    help(palette, buf, &view.help, bg);
}
