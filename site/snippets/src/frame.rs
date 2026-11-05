//! Chapter 4, the frame and skins: what the garden shows as plain data
//! (its view), and the skin that paints that view in glyphs. The rules
//! and the view never name a color or a glyph; only the skin does. And a
//! skin that paints a picture's pixels itself, a plasma moving with the
//! steps.

use shuyi::core::TurnRules;
use shuyi::prelude::*;
use shuyi::ui::frame::{Layer, Painted, PaintedId, PaintedSprite};

use crate::Data;
use crate::rules::Outcome;
use crate::screens::GardenScreen;

// ANCHOR: view
/// What the garden shows: plain data, built from the rules' state and
/// the game's text, which tests read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GardenView {
    /// Each plot, left to right.
    pub plots: Vec<Plot>,
    /// The plot the player stands at.
    pub at: usize,
    /// The seeds left, in words.
    pub seeds: String,
    /// The help line, or how the game ended.
    pub help: String,
}

/// A plot as the garden shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Plot {
    /// Bare soil.
    Soil,
    /// Something grows.
    Sprout,
}

impl GardenScreen {
    /// What the garden shows now.
    pub fn view(&self, ctx: &Ctx<Data>) -> GardenView {
        let garden = &self.garden;
        let plots = garden.plots().iter();
        let help = match garden.outcome() {
            None => "garden.help",
            Some(Outcome::Won) => "garden.won",
            Some(Outcome::Lost) => "garden.lost",
        };
        GardenView {
            plots: plots
                .map(|&grows| if grows { Plot::Sprout } else { Plot::Soil })
                .collect(),
            at: garden.at(),
            seeds: ctx.text_with("garden.seeds", &[("seeds", &garden.seeds())]),
            help: ctx.text_with(help, &[]),
        }
    }
}
// ANCHOR_END: view

// ANCHOR: skin
/// The glyph skin: paints a garden's view into the frame's cells, its
/// colors by name from the game's palette.
pub fn paint_garden(view: &GardenView, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
    let c = |color| ctx.palette.engine(color);
    let own = |name| ctx.palette.lookup(name).unwrap_or(c(EngineColor::Text));
    let bg = c(EngineColor::Background);
    buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
    let count = i32::try_from(view.plots.len()).unwrap_or(0);
    let left = (i32::from(buf.width()) - count * 4) / 2;
    for (x, plot) in (left..).step_by(4).zip(&view.plots) {
        let (glyph, fg) = match plot {
            Plot::Soil => ('·', own("soil")),
            Plot::Sprout => ('▲', own("sprout")),
        };
        buf.set(x + 1, 10, Cell::new(glyph, fg, bg));
    }
    let at = left + i32::try_from(view.at).unwrap_or(0) * 4;
    buf.draw_box(
        Rect::new(at, 9, 3, 3),
        BoxStyle::Single,
        c(EngineColor::Focus),
        bg,
    );
    print_centred(buf, 14, &view.seeds, c(EngineColor::Text), bg);
    let last = i32::from(buf.height()) - 1;
    print_centred(buf, last, &view.help, c(EngineColor::TextDim), bg);
}
// ANCHOR_END: skin

// ANCHOR: plasma
/// The plasma's name among the frame's pictures.
pub const PLASMA: PaintedId = PaintedId::new("plasma");

/// A skin that paints a 64 × 48 plasma from the step count, pixel by
/// pixel, and shows it twice its size with its top-left at `(x, y)`.
pub fn paint_plasma(step: u32, (x, y): (i32, i32), buf: &mut GlyphBuffer) {
    // Up from 0 to 248 and back down over 64 steps of `v`.
    let wave = |v: u32| u8::try_from((v % 64).abs_diff(32) * 31 / 4).unwrap_or(u8::MAX);
    let mut picture = Painted::rgba(64, 48);
    let pixels = picture.pixels_mut().as_chunks_mut::<4>().0;
    for (i, pixel) in (0u32..).zip(pixels) {
        let (px, py) = (i % 64, i / 64);
        let red = wave(px + step);
        let green = wave(py + 2 * step);
        let blue = wave(px + py + 3 * step);
        *pixel = [red, green, blue, 255];
    }
    buf.pictures_mut().update(PLASMA, picture);
    let whole = Rect::new(0, 0, 64, 48);
    let place = Rect::new(x, y, 128, 96);
    buf.add_painted(PaintedSprite::new(PLASMA, whole, place, Layer::Over));
}
// ANCHOR_END: plasma

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game;

    // ANCHOR: test_view
    #[test]
    fn planting_shows_in_the_view() {
        let mut h = game().harness();
        h.keys("Enter Enter");
        let garden = h.screen::<GardenScreen>().unwrap();
        let view = garden.view(h.ctx());
        assert_eq!(view.at, 0);
        assert_eq!(view.seeds, "8 seeds left");
        // A test of what happened reads the view: a new glyph or color
        // doesn't break it.
        assert_eq!(view.plots.len(), 6);
    }
    // ANCHOR_END: test_view

    // ANCHOR: test_look
    #[test]
    fn the_garden_looks_like_a_row_of_plots() {
        let mut h = game().harness();
        h.keys("Enter");
        // A test of a look reads the frame, here as text.
        let frame = h.snapshot();
        assert!(frame.contains("│·│  ·   ·   ·   ·   ·"), "{frame}");
        assert!(frame.contains("9 seeds left"), "{frame}");
    }
    // ANCHOR_END: test_look

    // ANCHOR: test_plasma
    #[test]
    fn the_plasma_moves_with_the_steps() {
        let black = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
        let mut buf = GlyphBuffer::new(48, 24, black);
        let mut revision = |step| {
            buf.fill_rect(buf.bounds(), black);
            paint_plasma(step, (16, 16), &mut buf);
            buf.pictures().get(PLASMA).map(Painted::revision)
        };
        let first = revision(0);
        // The same step paints the same picture: nothing to upload.
        assert_eq!(revision(0), first);
        // The next one moves it.
        assert_ne!(revision(1), first);
    }
    // ANCHOR_END: test_plasma

    #[test]
    fn the_plasmas_pixels_are_its_waves() {
        let black = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
        let pixel = |step: u32, (x, y): (usize, usize)| {
            let mut buf = GlyphBuffer::new(48, 24, black);
            paint_plasma(step, (0, 0), &mut buf);
            let picture = buf.pictures().get(PLASMA).unwrap();
            let at = (y * 64 + x) * 4;
            picture.pixels()[at..at + 4].to_vec()
        };
        // A wave is `|v % 64 - 32| × 31 / 4`.
        assert_eq!(pixel(0, (0, 0)), [248, 248, 248, 255]);
        // Red wave(10 + 5), green wave(3 + 10), blue wave(13 + 15).
        assert_eq!(pixel(5, (10, 3)), [131, 147, 31, 255]);
        // wave(163), wave(247), wave(410).
        assert_eq!(pixel(100, (63, 47)), [23, 178, 46, 255]);
    }

    #[test]
    fn the_row_is_centred_the_cursor_boxes_its_plot_and_the_help_is_last() {
        let mut h = game().harness();
        h.keys("Enter Right");
        let frame = h.snapshot();
        let lines: Vec<&str> = frame.lines().collect();
        // Six plots four cells apart from column 12, the second one boxed.
        assert_eq!(
            lines[10].trim_end(),
            "             ·  │·│  ·   ·   ·   ·",
            "{frame}"
        );
        assert_eq!(lines[9].trim(), "┌─┐", "{frame}");
        assert_eq!(lines[9].find('┌'), Some(16), "{frame}");
        assert!(lines[23].contains("Enter plant"), "{frame}");
    }
}
