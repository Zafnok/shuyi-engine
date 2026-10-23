use proptest::prelude::*;
use shuyi_assets::image::ImageTable;

use super::*;
use crate::frame::color::Rgb;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::Cell;

const BLANK: Cell = Cell::new(' ', Rgb::new(200, 200, 200), Rgb::new(0, 0, 0));
/// A 32 × 16-cell area of 8 × 16-pixel cells at cell (0, 0).
const FRAME: PxRect = Rect::new(0, 0, 256, 256);

fn size(width: u32, height: u32) -> ImageInfo {
    ImageInfo::new(width, height)
}

/// The one sprite of a `width × height` picture drawn in the 32 × 16
/// cells at `at` of a `cells`-sized buffer, if any of it shows.
fn drawn(
    (width, height): (u32, u32),
    cells: (u16, u16),
    at: (i32, i32),
    dim: f32,
    mirror: bool,
) -> Option<Sprite> {
    let table = ImageTable::new([("faces/t.png", size(width, height))].into());
    let image = table.id("faces/t.png").unwrap();
    let mut buf = GlyphBuffer::new(cells.0, cells.1, BLANK);
    let before = buf.clone();
    draw_fitted(
        &mut buf,
        at,
        (32, 16),
        (image, size(width, height)),
        dim,
        mirror,
    );
    // Only a sprite is added: no cell changes.
    for y in 0..i32::from(cells.1) {
        for x in 0..i32::from(cells.0) {
            assert_eq!(buf.get(x, y), before.get(x, y));
        }
    }
    assert_eq!(buf.items().len(), buf.sprites().len());
    assert!(buf.sprites().len() <= 1);
    buf.sprites().first().copied()
}

#[test]
fn a_small_picture_fills_the_area_at_4x() {
    let s = drawn((64, 64), (40, 20), (3, 2), 0.0, false).unwrap();
    assert_eq!(s.dest, Rect::new(24, 32, 256, 256));
    assert_eq!(s.clip, s.dest);
    assert_eq!(s.src, Rect::new(0, 0, 64, 64));
    assert_eq!(s.image.path(), "faces/t.png");
    assert_eq!(s.layer, Layer::Under);
    assert!(!s.flip_x);
    assert_eq!(s.opacity, 255);
}

#[test]
fn the_area_follows_the_cells_size() {
    let table = ImageTable::new([("faces/t.png", size(16, 16))].into());
    let image = table.id("faces/t.png").unwrap();
    // 8 × 8 cells: a 4 × 4-cell area is 32 × 32 pixels.
    let spec = ConsoleSpec {
        cell_h: 8,
        ..ConsoleSpec::text(10, 10)
    };
    let mut buf = GlyphBuffer::with_spec(spec, BLANK);
    draw_fitted(&mut buf, (1, 2), (4, 4), (image, size(16, 16)), 0.0, false);
    assert_eq!(buf.sprites()[0].dest, Rect::new(8, 16, 32, 32));
}

#[test]
fn any_size_gets_the_largest_whole_scale_that_fits() {
    let fit = |w, h| fit_whole_scale(size(w, h), FRAME);
    assert_eq!(fit(32, 32), Some(Rect::new(0, 0, 256, 256)));
    assert_eq!(fit(1, 1), Some(Rect::new(0, 0, 256, 256)));
    assert_eq!(fit(256, 256), Some(Rect::new(0, 0, 256, 256)));
    // The tighter side decides: 2× (3× would be 300 wide).
    assert_eq!(fit(100, 60), Some(Rect::new(28, 68, 200, 120)));
    assert_eq!(fit(60, 100), Some(Rect::new(68, 28, 120, 200)));
    // 129 px fits only once; the odd pixel left over is on the right.
    assert_eq!(fit(129, 128), Some(Rect::new(63, 64, 129, 128)));
    assert_eq!(fit(128, 129), Some(Rect::new(64, 63, 128, 129)));
    // A frame elsewhere, not square.
    let frame = Rect::new(10, 20, 100, 50);
    assert_eq!(
        fit_whole_scale(size(16, 16), frame),
        Some(Rect::new(10 + 26, 20 + 1, 48, 48))
    );
}

#[test]
fn an_image_that_does_not_fit_is_not_drawn() {
    let fit = |w, h| fit_whole_scale(size(w, h), FRAME);
    assert_eq!(fit(257, 8), None);
    assert_eq!(fit(8, 257), None);
    assert_eq!(fit(0, 8), None);
    assert_eq!(fit(8, 0), None);
    assert_eq!(drawn((257, 8), (40, 20), (0, 0), 0.0, false), None);
}

#[test]
fn dim_is_the_sprites_opacity() {
    assert_eq!(dim_opacity(0.0), 255);
    assert_eq!(dim_opacity(1.0), 0);
    assert_eq!(dim_opacity(0.5), 128);
    assert_eq!(dim_opacity(0.45), 140);
    assert_eq!(dim_opacity(0.25), 191);
    assert_eq!(dim_opacity(-1.0), 255);
    assert_eq!(dim_opacity(2.0), 0);
    let s = drawn((64, 64), (40, 20), (0, 0), 0.5, false).unwrap();
    assert_eq!(s.opacity, 128);
    assert!(!s.flip_x);
}

#[test]
fn mirror_is_the_sprites_flip() {
    let s = drawn((48, 48), (40, 20), (0, 0), 0.0, true).unwrap();
    assert!(s.flip_x);
    assert_eq!(s.opacity, 255);
    // Flipping doesn't move it.
    assert_eq!(s.dest, Rect::new(8, 8, 240, 240));
}

#[test]
fn clips_to_the_buffer_without_moving_the_picture() {
    // A 20×10-cell buffer is 160×160 px.
    let s = drawn((64, 64), (20, 10), (-2, -1), 0.0, false).unwrap();
    assert_eq!(s.dest, Rect::new(-16, -16, 256, 256));
    assert_eq!(s.clip, Rect::new(0, 0, 160, 160));
    let s = drawn((48, 48), (20, 10), (10, 5), 0.0, false).unwrap();
    assert_eq!(s.dest, Rect::new(88, 88, 240, 240));
    assert_eq!(s.clip, Rect::new(88, 88, 72, 72));
    // Wholly outside: nothing.
    assert_eq!(drawn((64, 64), (20, 10), (20, 0), 0.0, false), None);
    assert_eq!(drawn((64, 64), (20, 10), (0, -16), 0.0, false), None);
    assert_eq!(
        drawn((64, 64), (20, 10), (i32::MAX, i32::MIN), 0.0, false),
        None
    );
}

proptest! {
    /// Any image that fits is drawn inside the frame, at one whole scale
    /// both ways, the largest that fits, centred to within a pixel.
    #[test]
    fn a_fitting_image_lies_inside_the_frame_at_a_whole_scale(
        w in 1u32..=256,
        h in 1u32..=256,
        x in -500i32..500,
        y in -500i32..500,
    ) {
        let frame = Rect::new(x, y, 256, 256);
        let dest = fit_whole_scale(size(w, h), frame).unwrap();
        prop_assert_eq!(dest.intersect(&frame), Some(dest));
        let (w, h) = (i32::try_from(w).unwrap(), i32::try_from(h).unwrap());
        let scale = dest.w / w;
        prop_assert!(scale >= 1);
        prop_assert_eq!((dest.w, dest.h), (w * scale, h * scale));
        prop_assert!(w * (scale + 1) > 256 || h * (scale + 1) > 256);
        let (left, top) = (dest.x - x, dest.y - y);
        let (right, bottom) = (256 - dest.w - left, 256 - dest.h - top);
        prop_assert!(right - left == 0 || right - left == 1);
        prop_assert!(bottom - top == 0 || bottom - top == 1);
    }
}
