//! Tests of see-through cells and the [`Backdrop`] (game ADR-0048).

use alloc::rc::Rc;

use super::*;

const FG: Rgb = Rgb::new(200, 200, 200);
const BG: Rgb = Rgb::new(10, 20, 30);
const BLANK: Cell = Cell::new(' ', FG, BG);

fn scene() -> Rc<GlyphBuffer> {
    Rc::new(GlyphBuffer::new(6, 4, Cell::new('s', FG, BG)))
}

/// A 10×5 buffer showing [`scene`] through `clip`.
fn with_backdrop(clip: Rect, origin_px: (f32, f32), zoom: u8) -> GlyphBuffer {
    let mut b = GlyphBuffer::new(10, 5, BLANK);
    b.set_backdrop(scene(), clip, origin_px, zoom);
    b
}

#[test]
fn a_cell_is_solid_unless_made_see_through() {
    assert!(!Cell::new('x', FG, BG).see_through);
    let hole = Cell::see_through('x', FG);
    assert!(hole.see_through);
    assert_eq!((hole.glyph, hole.fg), ('x', FG));
    assert_ne!(Cell::see_through(' ', FG), Cell::new(' ', FG, hole.bg));
}

#[test]
fn printing_a_whole_cell_makes_it_solid_and_print_fg_keeps_the_hole() {
    let mut b = GlyphBuffer::new(4, 1, Cell::see_through(' ', FG));
    b.print_fg(0, 0, "a", FG);
    b.print(1, 0, "b", FG, BG);
    b.set(2, 0, BLANK);
    assert_eq!(b.get(0, 0), Some(&Cell::see_through('a', FG)));
    assert_eq!(b.get(1, 0), Some(&Cell::new('b', FG, BG)));
    assert_eq!(b.get(2, 0), Some(&BLANK));
    assert_eq!(b.get(3, 0), Some(&Cell::see_through(' ', FG)));
    // A box and a blit over holes are solid too.
    let mut b = GlyphBuffer::new(3, 3, Cell::see_through(' ', FG));
    b.draw_box(b.bounds(), BoxStyle::Single, FG, BG);
    assert!(!b.get(0, 0).unwrap().see_through);
    assert!(b.get(1, 1).unwrap().see_through);
    b.blit(&GlyphBuffer::new(1, 1, BLANK), 1, 1);
    assert_eq!(b.get(1, 1), Some(&BLANK));
    // And a blit brings the source's holes along.
    b.blit(&GlyphBuffer::new(1, 1, Cell::see_through('h', FG)), 0, 0);
    assert_eq!(b.get(0, 0), Some(&Cell::see_through('h', FG)));
}

#[test]
fn a_backdrop_keeps_what_it_was_given() {
    let b = GlyphBuffer::new(10, 5, BLANK);
    assert!(b.backdrop().is_none());
    let b = with_backdrop(Rect::new(2, 1, 6, 3), (12.5, -3.25), 3);
    let d = b.backdrop().unwrap();
    assert_eq!(d.scene(), &*scene());
    assert_eq!(d.clip(), Rect::new(2, 1, 6, 3));
    assert_eq!(d.clip_px(), Rect::new(16, 16, 48, 48));
    assert_eq!(d.origin_px(), (12.5, -3.25));
    assert_eq!(d.zoom(), 3);
}

#[test]
fn the_clip_is_clipped_to_the_buffer_and_one_outside_sets_nothing() {
    let b = with_backdrop(Rect::new(-2, 3, 5, 9), (0.0, 0.0), 1);
    assert_eq!(b.backdrop().unwrap().clip(), Rect::new(0, 3, 3, 2));
    assert_eq!(b.backdrop().unwrap().clip_px(), Rect::new(0, 48, 24, 32));
    for clip in [
        Rect::new(10, 0, 3, 3),
        Rect::new(0, 5, 3, 3),
        Rect::new(2, 2, 0, 3),
        Rect::new(2, 2, 3, -1),
    ] {
        // Also removes one set before.
        let mut b = with_backdrop(Rect::new(0, 0, 2, 2), (0.0, 0.0), 1);
        b.set_backdrop(scene(), clip, (0.0, 0.0), 1);
        assert!(b.backdrop().is_none(), "{clip:?}");
    }
}

#[test]
fn zoom_is_a_whole_step_from_1_to_4() {
    assert_eq!(MAX_ZOOM, 4);
    let zoom = |z| with_backdrop(Rect::new(0, 0, 4, 4), (0.0, 0.0), z);
    let zooms: Vec<u8> = [0, 1, 2, 3, 4, 5, u8::MAX]
        .into_iter()
        .map(|z| zoom(z).backdrop().unwrap().zoom())
        .collect();
    assert_eq!(zooms, [1, 1, 2, 3, 4, 4, 4]);
}

#[test]
fn an_origin_that_is_not_a_number_counts_as_zero() {
    let b = with_backdrop(Rect::new(0, 0, 4, 4), (f32::NAN, f32::NEG_INFINITY), 1);
    assert_eq!(b.backdrop().unwrap().origin_px(), (0.0, 0.0));
    let b = with_backdrop(Rect::new(0, 0, 4, 4), (f32::INFINITY, 2.0), 1);
    assert_eq!(b.backdrop().unwrap().origin_px(), (0.0, 2.0));
}

#[test]
fn the_scene_is_placed_on_whole_window_pixels() {
    let offset = |origin, zoom, scale| {
        with_backdrop(Rect::new(1, 1, 4, 2), origin, zoom)
            .backdrop()
            .unwrap()
            .scene_offset(scale)
    };
    assert_eq!(offset((0.0, 0.0), 1, 1), (0, 0));
    assert_eq!(offset((10.0, 4.0), 1, 1), (-10, -4));
    // A scene pixel is zoom × scale window pixels.
    assert_eq!(offset((10.0, 4.0), 3, 1), (-30, -12));
    assert_eq!(offset((10.0, 4.0), 3, 2), (-60, -24));
    // The window may start left of or above the scene.
    assert_eq!(offset((-7.0, -1.5), 2, 1), (14, 3));
    // Part of a scene pixel moves by whole window pixels: at 2 × 2, in
    // quarters.
    assert_eq!(offset((0.25, 0.75), 2, 2), (-1, -3));
    assert_eq!(offset((0.3, 0.6), 2, 2), (-1, -2));
    // At 1 × 1 a fraction rounds to the nearest pixel.
    assert_eq!(offset((0.4, 0.6), 1, 1), (0, -1));
    assert_eq!(offset((1e30, -1e30), 4, 8), (i64::MIN, i64::MAX));
}

#[test]
fn clearing_the_frame_removes_the_backdrop() {
    let clip = Rect::new(2, 1, 6, 3);
    let hole = Cell::see_through(' ', FG);
    // Marking the window, and filling part of the buffer, keep it.
    let mut b = with_backdrop(clip, (0.0, 0.0), 2);
    b.fill_rect(clip, hole);
    b.fill_rect(b.bounds(), hole);
    b.fill_rect(Rect::new(0, 0, 10, 4), BLANK);
    b.fill_rect(Rect::new(1, 0, 10, 5), BLANK);
    b.blit(&GlyphBuffer::new(10, 5, BLANK), 0, 0);
    assert!(b.backdrop().is_some());
    // A solid fill of the whole buffer (or more) removes it.
    b.fill_rect(b.bounds(), BLANK);
    assert!(b.backdrop().is_none());
    let mut b = with_backdrop(clip, (0.0, 0.0), 2);
    b.fill_rect(Rect::new(-1, -1, 20, 20), BLANK);
    assert!(b.backdrop().is_none());
    let mut b = with_backdrop(clip, (0.0, 0.0), 2);
    b.clear_backdrop();
    assert!(b.backdrop().is_none());
}

#[test]
fn buffers_differ_by_their_backdrop() {
    let clip = Rect::new(2, 1, 6, 3);
    let a = with_backdrop(clip, (1.0, 2.0), 2);
    assert_eq!(a, a.clone());
    assert_ne!(a, GlyphBuffer::new(10, 5, BLANK));
    assert_ne!(a, with_backdrop(clip, (1.5, 2.0), 2));
    assert_ne!(a, with_backdrop(clip, (1.0, 2.0), 3));
    assert_ne!(a, with_backdrop(Rect::new(2, 1, 6, 2), (1.0, 2.0), 2));
}

/// A frame with a cell, a rectangle and two sprites, over a scene with
/// the same.
fn busy() -> GlyphBuffer {
    busy_sharing().0
}

/// [`busy`], and its scene as someone else holds it.
fn busy_sharing() -> (GlyphBuffer, Rc<GlyphBuffer>) {
    use shuyi_assets::image::{ImageInfo, ImageTable};
    let size = ImageInfo {
        width: 16,
        height: 16,
    };
    let table = ImageTable {
        images: [("a.png", size)].into(),
    };
    let image = table.id("a.png").unwrap();
    let px = Rect::new(0, 0, 16, 16);
    let fill = |b: &mut GlyphBuffer| {
        b.add_overlay(Overlay::new(px, Rgb::new(100, 50, 201), Layer::Under));
        b.add_sprite(Sprite::new(image, px, px, Layer::Over));
        let mut faint = Sprite::new(image, px, px, Layer::Over);
        faint.opacity = 101;
        b.add_sprite(faint);
    };
    let mut scene = GlyphBuffer::new(6, 4, Cell::new('s', FG, BG));
    fill(&mut scene);
    let mut b = GlyphBuffer::new(10, 5, BLANK);
    b.set(1, 1, Cell::see_through('t', FG));
    fill(&mut b);
    let scene = Rc::new(scene);
    b.set_backdrop(Rc::clone(&scene), b.bounds(), (1.0, 2.0), 2);
    (b, scene)
}

#[test]
fn fade_darkens_cells_rectangles_sprites_and_the_scene_alike() {
    let (mut b, scene_before) = busy_sharing();
    b.fade(0.5);
    let check = |b: &GlyphBuffer, cell: Cell| {
        assert_eq!(b.get(0, 0), Some(&cell));
        assert_eq!(b.overlays()[0].color, Rgb::new(50, 25, 101));
        // Darker, as see-through as they were (game ADR-0075).
        let looks: Vec<(u8, u8)> = b
            .sprites()
            .iter()
            .map(|s| (s.brightness, s.opacity))
            .collect();
        assert_eq!(looks, [(128, 255), (128, 101)]);
    };
    let (fg, bg) = (Rgb::new(100, 100, 100), Rgb::new(5, 10, 15));
    check(&b, Cell::new(' ', fg, bg));
    // A hole stays a hole; its glyph darkens.
    assert_eq!(b.get(1, 1), Some(&Cell::see_through('t', fg)));
    let backdrop = b.backdrop().unwrap();
    check(backdrop.scene(), Cell::new('s', fg, bg));
    // Where the window is doesn't change, nor the scene others share.
    assert_eq!((backdrop.origin_px(), backdrop.zoom()), ((1.0, 2.0), 2));
    assert_eq!(backdrop.clip(), b.bounds());
    assert_eq!(scene_before.get(0, 0), Some(&Cell::new('s', FG, BG)));
    assert_eq!(scene_before.sprites()[0].brightness, 255);
    // Nothing else about an item changes.
    let (faded, plain) = (b.sprites()[0], busy().sprites()[0]);
    assert_eq!(
        Sprite {
            brightness: 255,
            ..faded
        },
        plain
    );
}

#[test]
fn fade_is_clamped_and_full_brightness_changes_nothing() {
    for same in [1.0, 1.5, f32::INFINITY, f32::NAN] {
        let (mut b, scene) = busy_sharing();
        b.fade(same);
        assert_eq!(b, busy(), "{same}");
        // Not even a copy of the scene is made.
        let shown = b.backdrop().map(Backdrop::scene);
        assert!(shown.is_some_and(|shown| core::ptr::eq(shown, &raw const *scene)));
    }
    for black in [0.0, -1.0, f32::NEG_INFINITY] {
        let mut b = busy();
        b.fade(black);
        let dark = Rgb::new(0, 0, 0);
        assert_eq!(b.get(0, 0), Some(&Cell::new(' ', dark, dark)), "{black}");
        assert_eq!(b.overlays()[0].color, dark);
        assert!(b.sprites().iter().all(|s| s.brightness == 0));
        let scene = b.backdrop().unwrap().scene();
        assert_eq!(scene.get(0, 0), Some(&Cell::new('s', dark, dark)));
        assert!(scene.sprites().iter().all(|s| s.brightness == 0));
    }
    // Without a backdrop there is only the frame to fade.
    let mut b = GlyphBuffer::new(2, 1, BLANK);
    b.fade(0.5);
    assert!(b.backdrop().is_none());
    assert_eq!(b.get(1, 0).unwrap().fg, Rgb::new(100, 100, 100));
}
