//! The frame on a console of a size no game of the engine's uses yet:
//! 40 × 20 cells of 8 × 8 pixels.

use alloc::rc::Rc;

use insta::assert_snapshot;
use shuyi_assets::image::{ImageInfo, ImageTable};

use super::color::tests::engine_palette;
use super::*;

/// 40 × 20 cells of 8 × 8 pixels: 320 × 160.
const SMALL: ConsoleSpec = ConsoleSpec {
    cols: 40,
    rows: 20,
    cell_w: 8,
    cell_h: 8,
};

#[test]
fn a_40_by_20_console_of_8_by_8_cells() {
    let p = engine_palette();
    let text = p.engine(EngineColor::Text);
    let panel = p.engine(EngineColor::Panel);
    let size = ImageInfo::new(16, 16);
    let images = ImageTable::new([("images/face.png", size)].into());
    let face = images.id("images/face.png").unwrap();

    let mut buf = GlyphBuffer::with_spec(SMALL, Cell::new(' ', text, panel));
    assert_eq!(buf.spec(), SMALL);
    assert_eq!((buf.width(), buf.height()), (40, 20));
    assert_eq!(buf.pixel_bounds(), Rect::new(0, 0, 320, 160));

    // Text, in a box.
    buf.draw_box(
        Rect::new(0, 0, 40, 4),
        BoxStyle::Single,
        p.engine(EngineColor::Border),
        panel,
    );
    buf.print(
        2,
        1,
        "Forty by twenty",
        p.engine(EngineColor::Highlight),
        panel,
    );
    buf.print_fg(2, 2, "cells of 8x8", p.engine(EngineColor::TextDim));

    // A sprite: placed in pixels, its cells' size decides where.
    portrait::draw_fitted(&mut buf, (30, 5), (8, 8), (face, size), 0.0, false);
    let sprite = buf.sprites()[0];
    assert_eq!(sprite.dest, Rect::new(240, 40, 64, 64));
    // A rectangle past the edge is cut to the 320 × 160 console.
    buf.add_overlay(Overlay::new(
        Rect::new(300, 150, 40, 20),
        p.engine(EngineColor::Warning),
        Layer::Over,
    ));
    assert_eq!(buf.overlays()[0].rect, Rect::new(300, 150, 20, 10));

    // A backdrop: a scene of the same cells behind see-through ones, at 2×.
    let mut scene = GlyphBuffer::with_spec(SMALL.sized(8, 4), Cell::new('"', text, panel));
    scene.print(1, 1, "grass", p.lookup("grass").unwrap(), panel);
    for y in 5..9 {
        for x in 2..12 {
            buf.set(x, y, Cell::see_through(' ', text));
        }
    }
    buf.set_backdrop(Rc::new(scene), Rect::new(2, 5, 10, 4), (4.0, 2.0), 2);
    let backdrop = buf.backdrop().unwrap();
    assert_eq!(backdrop.clip(), Rect::new(2, 5, 10, 4));
    assert_eq!(backdrop.clip_px(), Rect::new(16, 40, 80, 32));
    assert_eq!(backdrop.zoom(), 2);

    let snap = buf.to_snapshot(&p);
    assert_eq!(
        snap.lines().next().map(str::chars).map(Iterator::count),
        Some(40)
    );
    assert_snapshot!(snap);
}
