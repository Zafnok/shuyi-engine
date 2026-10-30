//! The look of the screens: each paints its screen's view (plain data)
//! with glyphs in the palette's colors. The map itself is painted by the
//! engine's skins ([`crate::map`]); this paints the lines round it.

use crate::prelude::*;

use shuyi::ui::dialogue::TextView;
use shuyi::ui::dialogue::text_box::{self, glyph::BoxPlace};
use shuyi::ui::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use shuyi::ui::frame::{EngineColor, Palette, Rgb};
use shuyi::ui::widgets::menu::{self, MenuView};
use shuyi::ui::{Ctx, GameData};

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
}

/// What a yes-or-no question over the stage shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionView {
    /// The question.
    pub question: String,
    /// Its answers, with their keys.
    pub answers: String,
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
/// Its labels are in the player's language.
fn centred_menu<G: GameData>(ctx: &Ctx<G>, view: &MenuView, buf: &mut GlyphBuffer, y: i32) {
    let (w, _) = menu::glyph::size(ctx, view);
    let x = (i32::from(buf.width()) - w).max(0) / 2;
    menu::glyph::paint(ctx, view, buf, x, y);
}

/// How far up a screen's lines move on a short console, so the menu clears
/// the help on the last row: on the Playdate's 15 rows, three.
fn raised(buf: &GlyphBuffer) -> i32 {
    (24 - i32::from(buf.height())).clamp(0, 3)
}

/// The title: the name in a box, the line under it, the menu, the help.
pub fn paint_title<G: GameData>(ctx: &Ctx<G>, view: &TitleView, buf: &mut GlyphBuffer) {
    let palette = &ctx.palette;
    let bg = clear(palette, buf);
    let c = |e| palette.engine(e);
    let width = i32::try_from(view.name.chars().count()).unwrap_or(0) + 6;
    let x = (i32::from(buf.width()) - width).max(0) / 2;
    let up = raised(buf);
    buf.draw_box(
        Rect::new(x, 3 - up, width, 3),
        BoxStyle::Double,
        c(EngineColor::Border),
        bg,
    );
    line(buf, 4 - up, &view.name, c(EngineColor::Highlight), bg);
    line(buf, 7 - up, &view.subtitle, c(EngineColor::TextDim), bg);
    centred_menu(ctx, &view.menu, buf, 10 - up);
    help(palette, buf, &view.help, bg);
}

/// Where the scene's text box goes in `buf`: the bottom seven rows.
pub fn talk_box(buf: &GlyphBuffer) -> BoxPlace {
    let (width, height) = (i32::from(buf.width()), i32::from(buf.height()));
    BoxPlace::new(Rect::new(0, height - 7, width, 7), 2, height - 5)
}

/// A scene's text box, with the speaker's name.
pub fn paint_talk<G: GameData>(
    ctx: &Ctx<G>,
    view: &TextView,
    speaker: Option<&str>,
    buf: &mut GlyphBuffer,
) {
    let place = talk_box(buf);
    text_box::glyph::paint(ctx, buf, place, view, speaker);
}

/// The lines round the stage's map: the turn and what is happening on the
/// top row, the help on the bottom one.
pub fn paint_stage(palette: &Palette, view: &StageView, buf: &mut GlyphBuffer) {
    let c = |e| palette.engine(e);
    let bg = c(EngineColor::Panel);
    // The top and bottom rows are the cleared frame's: the panel's color.
    let shown = buf.print(1, 0, &view.turn, c(EngineColor::Highlight), bg);
    let x = 1 + i32::from(shown) + 3;
    buf.print(x, 0, &view.message, c(EngineColor::Text), bg);
    help(palette, buf, &view.help, bg);
}

/// A question over the stage, on the bottom row in place of its help.
pub fn paint_question(palette: &Palette, view: &QuestionView, buf: &mut GlyphBuffer) {
    let bg = palette.engine(EngineColor::Panel);
    let y = i32::from(buf.height()) - 1;
    buf.fill_rect(
        Rect::new(0, y, i32::from(buf.width()), 1),
        Cell::new(' ', bg, bg),
    );
    let text = format!("{}  {}", view.question, view.answers);
    line(buf, y, &text, palette.engine(EngineColor::Warning), bg);
}

/// The end of a stage: the headline, the menu, the help.
pub fn paint_end<G: GameData>(ctx: &Ctx<G>, view: &EndView, buf: &mut GlyphBuffer) {
    let palette = &ctx.palette;
    let bg = clear(palette, buf);
    let color = if view.won {
        EngineColor::Accent
    } else {
        EngineColor::Warning
    };
    let up = raised(buf);
    line(buf, 6 - up, &view.headline, palette.engine(color), bg);
    centred_menu(ctx, &view.menu, buf, 10 - up);
    help(palette, buf, &view.help, bg);
}

#[cfg(test)]
mod tests {
    use shuyi::ui::frame::ConsoleSpec;

    use super::*;
    use crate::rules::Outcome;
    use crate::screens::End;
    use crate::{PLAYDATE, PLAYDATE_CONSOLE, ctx_on};

    fn buffer(spec: ConsoleSpec) -> GlyphBuffer {
        let cell = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
        GlyphBuffer::with_spec(spec, cell)
    }

    /// Row `y` of `buf`.
    fn row(buf: &GlyphBuffer, y: i32) -> String {
        (0..i32::from(buf.width()))
            .filter_map(|x| buf.get(x, y).map(|cell| cell.glyph))
            .collect()
    }

    #[test]
    fn the_talk_box_is_the_bottom_seven_rows_of_any_console() {
        let place = talk_box(&buffer(PLAYDATE_CONSOLE));
        assert_eq!(place.frame, Rect::new(0, 8, 50, 7));
        assert_eq!((place.text_x, place.text_y), (2, 10));
        let place = talk_box(&buffer(crate::CONSOLE));
        assert_eq!(place.frame, Rect::new(0, 17, 48, 7));
        assert_eq!(place.text_y, 19);
    }

    #[test]
    fn the_end_clears_the_help_on_a_short_console() {
        let ctx = ctx_on(PLAYDATE, 0);
        let view = End::new(Outcome::Won { turn: 3 }).view(&ctx);
        let mut buf = buffer(PLAYDATE_CONSOLE);
        paint_end(&ctx, &view, &mut buf);
        assert!(row(&buf, 3).contains(&view.headline), "{:?}", row(&buf, 3));
        assert_ne!(row(&buf, 7).trim(), "", "the menu's top");
        assert_eq!(row(&buf, 6).trim(), "", "just above it");
    }
}
