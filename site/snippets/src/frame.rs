//! Chapter 2, the frame and skins: what the garden shows as plain data
//! (its view), and the skin that paints that view in glyphs. The rules
//! and the view never name a color or a glyph; only the skin does. And a
//! skin that paints a picture's pixels itself, a plasma moving with the
//! steps. And the garden's own channel of the debug view: where each plot
//! is, drawn over the running game.

use shuyi::core::{Ease, Family, TurnRules, Tween};
use shuyi::prelude::*;
use shuyi::ui::debug::gizmo::{Channel, Gizmo, GizmoRole, Gizmos};
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

// ANCHOR: tween
/// How a new sprout comes up: from 8 pixels below its plot to its place in
/// 300 milliseconds, a little past and back, eased out with a "back".
pub const SPROUT_RISE: Tween<i32> = Tween::new(8, 0, 300).with_ease(Ease::Out(Family::Back));

/// How many pixels below its plot a sprout planted `ms` milliseconds ago is
/// drawn: worked out from the time alone, so nothing has to update it.
pub fn sprout_offset(ms: u32) -> i32 {
    SPROUT_RISE.value_at(ms)
}
// ANCHOR_END: tween

// ANCHOR: plasma
/// The plasma's name among the frame's pictures.
pub const PLASMA: PaintedId = PaintedId::new("plasma");

/// A skin that paints a 64 × 48 plasma from the step count, pixel by
/// pixel, and shows it twice its size with its top-left at `(x, y)`.
pub fn paint_plasma(step: u32, (x, y): (i32, i32), buf: &mut GlyphBuffer) {
    // Up from 0 to 248 and back down over 64 steps of `v`.
    let wave = |v: u32| u8::try_from((v % 64).abs_diff(32) * 31 / 4).unwrap_or(u8::MAX);
    let mut picture = Painted::rgba(64, 48);
    let rows = picture.pixels_mut().as_chunks_mut::<4>().0.chunks_mut(64);
    for (py, row) in (0u32..).zip(rows) {
        for (px, pixel) in (0u32..).zip(row) {
            let red = wave(px + step);
            let green = wave(py + 2 * step);
            let blue = wave(px + py + 3 * step);
            *pixel = [red, green, blue, 255];
        }
    }
    buf.pictures_mut().update(PLASMA, picture);
    let whole = Rect::new(0, 0, 64, 48);
    let place = Rect::new(x, y, 128, 96);
    buf.add_painted(PaintedSprite::new(PLASMA, whole, place, Layer::Over));
}
// ANCHOR_END: plasma

// ANCHOR: gizmos
/// The garden's channel of the debug view.
pub const PLOTS: Channel = Channel::new("plots", "Garden plots");

/// Hands in a box round each plot's cell, and a cross on the one the
/// player stands at, in console pixels: where the skin put them. The
/// garden screen's `debug_draw` calls it with its view.
pub fn plot_gizmos(view: &GardenView, ctx: &Ctx<Data>, out: &mut Gizmos) {
    if !out.channel_on(PLOTS) {
        return;
    }
    let (cell_w, cell_h) = (i32::from(ctx.console.cell_w), i32::from(ctx.console.cell_h));
    let count = i32::try_from(view.plots.len()).unwrap_or(0);
    let left = (i32::from(ctx.console.cols) - count * 4) / 2;
    for (i, x) in (0..view.plots.len()).zip((left + 1..).step_by(4)) {
        let rect = Rect::new(x * cell_w, 10 * cell_h, cell_w, cell_h);
        out.push(PLOTS, GizmoRole::Bounds, Gizmo::Rect { rect });
        if i == view.at {
            let at = (rect.x + cell_w / 2, rect.y + cell_h / 2);
            out.push(PLOTS, GizmoRole::Note, Gizmo::Cross { at, size: 3 });
        }
    }
}
// ANCHOR_END: gizmos

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

    // ANCHOR: test_tween
    #[test]
    fn a_sprout_rises_past_its_place_and_settles() {
        assert_eq!(sprout_offset(0), 8);
        // Over half the way up in a sixth of the time, then a pixel past.
        assert_eq!(sprout_offset(50), 3);
        assert_eq!(sprout_offset(200), -1);
        assert_eq!(sprout_offset(300), 0);
        assert_eq!(sprout_offset(5000), 0);
        // "It landed": true in exactly one frame, given the last frame's
        // time and this one's.
        let frames = [0, 16, 33, 150, 290, 307, 323];
        let landed = frames
            .windows(2)
            .filter(|w| SPROUT_RISE.finished_between(w[0], w[1]))
            .count();
        assert_eq!(landed, 1);
    }
    // ANCHOR_END: test_tween

    // ANCHOR: test_gizmos
    #[test]
    fn the_plots_channel_boxes_each_plot_and_marks_the_players() {
        let mut h = game().harness();
        h.keys("Enter Right");
        let garden = h.screen::<GardenScreen>().unwrap();
        let view = garden.view(h.ctx());
        // Off: nothing is built, but the channel is offered.
        let mut off = Gizmos::new();
        plot_gizmos(&view, h.ctx(), &mut off);
        assert_eq!(off.len(), 0);
        assert_eq!(off.offered(), [PLOTS]);
        // On: six boxes, and a cross after the second.
        let mut on = Gizmos::with_on(["plots"]);
        plot_gizmos(&view, h.ctx(), &mut on);
        let roles: Vec<GizmoRole> = on.drawn().iter().map(|d| d.role).collect();
        assert_eq!(roles.len(), 7);
        assert_eq!(roles[2], GizmoRole::Note);
    }
    // ANCHOR_END: test_gizmos

    #[test]
    fn the_plots_gizmos_sit_on_the_plots_the_skin_drew() {
        let mut h = game().harness();
        h.keys("Enter Right");
        // The garden screen hands them in once the channel is on.
        assert_eq!(h.game().buffer().gizmos().len(), 0);
        h.ctx_mut().debug_view.set("plots", true);
        h.wait(0.1);
        let shapes: Vec<Gizmo> = {
            let garden = h.screen::<GardenScreen>().unwrap();
            let mut on = Gizmos::with_on(["plots"]);
            plot_gizmos(&garden.view(h.ctx()), h.ctx(), &mut on);
            on.drawn().iter().map(|d| d.gizmo.clone()).collect()
        };
        assert_eq!(h.game().buffer().gizmos().len(), shapes.len());
        // Six plots four cells apart from column 13, on row 10, each cell
        // 8 × 16 pixels; the cross in the middle of the second.
        let boxed = |column: i32| Gizmo::Rect {
            rect: Rect::new(column * 8, 160, 8, 16),
        };
        let cross = Gizmo::Cross {
            at: (140, 168),
            size: 3,
        };
        let want = [
            boxed(13),
            boxed(17),
            cross,
            boxed(21),
            boxed(25),
            boxed(29),
            boxed(33),
        ];
        assert_eq!(shapes, want);
        // The skin's plots are in those cells.
        let frame = h.snapshot();
        let row: Vec<char> = frame.lines().nth(10).unwrap().chars().collect();
        assert_eq!((row[13], row[33]), ('·', '·'));
    }

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
