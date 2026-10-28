//! Chapter 4, the frame and skins: what the garden shows as plain data
//! (its view), and the skin that paints that view in glyphs. The rules
//! and the view never name a color or a glyph; only the skin does.

use shuyi::core::TurnRules;
use shuyi::prelude::*;

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
}
