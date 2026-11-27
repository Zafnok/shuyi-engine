//! The nine-slice rule (ticket 3249), read from the sprites it makes.

use proptest::prelude::*;
use shuyi_assets::image::{ImageInfo, ImageRect, ImageTable, Picture};

use super::*;

/// A 12×9 picture at (4, 2) of a bigger image, cut 3, 2, 4, 1.
fn slice() -> NineSlice {
    let table = ImageTable::new([("frame.png", ImageInfo::new(32, 32))].into());
    let picture = Picture {
        image: table.id("frame.png").unwrap(),
        rect: ImageRect {
            x: 4,
            y: 2,
            w: 12,
            h: 9,
        },
    };
    let borders = Borders {
        left: 3,
        top: 2,
        right: 4,
        bottom: 1,
    };
    NineSlice::new(picture, borders)
}

const ALL: PxRect = Rect::new(-1000, -1000, 4000, 4000);

fn parts(slice: &NineSlice, dest: PxRect) -> Vec<Sprite> {
    nine_slice_sprites(slice, dest, ALL, Layer::Under)
}

/// `(src, dest, clip)` of each sprite.
fn shape(sprites: &[Sprite]) -> Vec<(PxRect, PxRect, PxRect)> {
    sprites.iter().map(|s| (s.src, s.dest, s.clip)).collect()
}

#[test]
fn a_wide_slice_has_four_corners_four_edges_and_a_middle() {
    let r = Rect::new;
    let got = shape(&parts(&slice(), r(10, 20, 30, 12)));
    assert_eq!(
        got,
        [
            // Row by row: the top-left corner, the top edge, the top-right
            // corner, from the picture's corners (it starts at 4, 2)...
            (r(4, 2, 3, 2), r(10, 20, 3, 2), r(10, 20, 3, 2)),
            (r(7, 2, 5, 2), r(13, 20, 23, 2), r(13, 20, 23, 2)),
            (r(12, 2, 4, 2), r(36, 20, 4, 2), r(36, 20, 4, 2)),
            // ...the left edge, the middle, the right edge...
            (r(4, 4, 3, 6), r(10, 22, 3, 9), r(10, 22, 3, 9)),
            (r(7, 4, 5, 6), r(13, 22, 23, 9), r(13, 22, 23, 9)),
            (r(12, 4, 4, 6), r(36, 22, 4, 9), r(36, 22, 4, 9)),
            // ...and the bottom row, all stretched.
            (r(4, 10, 3, 1), r(10, 31, 3, 1), r(10, 31, 3, 1)),
            (r(7, 10, 5, 1), r(13, 31, 23, 1), r(13, 31, 23, 1)),
            (r(12, 10, 4, 1), r(36, 31, 4, 1), r(36, 31, 4, 1)),
        ]
    );
}

#[test]
fn zoom_scales_every_part() {
    let got = parts(&slice().with_zoom(2), Rect::new(0, 0, 60, 30));
    assert_eq!(got[0].dest, Rect::new(0, 0, 6, 4));
    assert_eq!(got[8].dest, Rect::new(52, 28, 8, 2));
    assert_eq!(got[4].dest, Rect::new(6, 4, 46, 24));
}

#[test]
fn a_narrow_slice_cuts_its_corners_on_their_inner_side() {
    // 5 wide: corners of 3 and 4 don't fit; the left keeps
    // min(3, max(5 - 4, 3)) = 3, the right 2. 2 tall: the top keeps
    // min(2, max(2 - 1, 1)) = 1, the bottom 1.
    let got = parts(&slice(), Rect::new(0, 0, 5, 2));
    assert_eq!(got.len(), 4, "corners only: {got:?}");
    let r = Rect::new;
    assert_eq!((got[0].dest, got[0].clip), (r(0, 0, 3, 2), r(0, 0, 3, 1)));
    assert_eq!((got[1].dest, got[1].clip), (r(1, 0, 4, 2), r(3, 0, 2, 1)));
    assert_eq!((got[2].dest, got[2].clip), (r(0, 1, 3, 1), r(0, 1, 3, 1)));
    assert_eq!((got[3].dest, got[3].clip), (r(1, 1, 4, 1), r(3, 1, 2, 1)));
}

#[test]
fn tiled_edges_repeat_from_the_corner_and_cut_the_last() {
    let got = parts(
        &slice()
            .with_edges(EdgeFill::Tile)
            .with_centre(CentreFill::Empty),
        Rect::new(0, 0, 19, 10),
    );
    let r = Rect::new;
    // The top edge runs 3..15: tiles of 5 at 3 and 8, the third cut to 2.
    let top: Vec<_> = got.iter().filter(|s| s.src == r(7, 2, 5, 2)).collect();
    let top: Vec<_> = top.iter().map(|s| (s.dest, s.clip)).collect();
    assert_eq!(
        top,
        [
            (r(3, 0, 5, 2), r(3, 0, 5, 2)),
            (r(8, 0, 5, 2), r(8, 0, 5, 2)),
            (r(13, 0, 5, 2), r(13, 0, 2, 2)),
        ]
    );
    // The left edge runs 2..9: tiles of 6, the second cut to 1.
    let left: Vec<_> = got.iter().filter(|s| s.src == r(4, 4, 3, 6)).collect();
    assert_eq!(left.len(), 2);
    assert_eq!(left[1].clip, r(0, 8, 3, 1));
    // No middle.
    assert!(got.iter().all(|s| s.src != r(7, 4, 5, 6)));
}

#[test]
fn tiles_are_their_part_times_the_zoom_and_keep_to_their_grid() {
    let tiled = slice()
        .with_zoom(2)
        .with_edges(EdgeFill::Tile)
        .with_centre(CentreFill::Tile);
    // Seen from (100, 100) on: the tiles before it are skipped, the rest
    // laid from the middle's corner (6, 4) in steps of 10 by 12.
    let within = Rect::new(100, 100, 30, 30);
    let got = nine_slice_sprites(&tiled, Rect::new(0, 0, 400, 400), within, Layer::Under);
    let middle: Vec<_> = got
        .iter()
        .filter(|s| s.src == Rect::new(7, 4, 5, 6))
        .collect();
    assert_ne!(middle.len(), 0);
    for s in &middle {
        assert_eq!((s.dest.w, s.dest.h), (10, 12));
        assert_eq!(((s.dest.x - 6) % 10, (s.dest.y - 4) % 12), (0, 0), "{s:?}");
    }
    let first = middle.iter().map(|s| (s.dest.y, s.dest.x)).min().unwrap();
    assert_eq!(first, (100, 96));
    assert!(middle.iter().any(|s| s.clip.contains(100, 100)));
}

#[test]
fn a_tiled_middle_makes_only_the_tiles_within_shows() {
    let tiled = slice().with_centre(CentreFill::Tile);
    let within = Rect::new(100, 100, 7, 5);
    let got = nine_slice_sprites(
        &tiled,
        Rect::new(0, 0, 100_000, 100_000),
        within,
        Layer::Over,
    );
    assert_ne!(got.len(), 0);
    assert!(got.len() <= 6, "{} sprites", got.len());
    for s in &got {
        assert!(within.intersect(&s.clip) == Some(s.clip), "{s:?}");
        assert_eq!(s.layer, Layer::Over);
    }
}

#[test]
fn nothing_for_an_empty_dest_or_within() {
    assert_eq!(parts(&slice(), Rect::new(0, 0, 0, 10)).len(), 0);
    let none = nine_slice_sprites(
        &slice(),
        Rect::new(0, 0, 10, 10),
        Rect::new(0, 0, 0, 0),
        Layer::Under,
    );
    assert_eq!(none.len(), 0);
    // Wholly outside `within`: nothing either.
    let far = nine_slice_sprites(
        &slice(),
        Rect::new(0, 0, 10, 10),
        Rect::new(50, 50, 9, 9),
        Layer::Under,
    );
    assert_eq!(far.len(), 0);
}

#[test]
fn borders_that_meet_leave_out_the_empty_parts() {
    // Borders of 6 + 6 across a 12-wide picture: no middle column.
    let wide = NineSlice::new(
        slice().picture,
        Borders {
            left: 6,
            top: 2,
            right: 6,
            bottom: 1,
        },
    );
    let got = parts(&wide, Rect::new(0, 0, 40, 40));
    // Four corners and the left and right edges; no top, bottom or middle.
    assert_eq!(got.len(), 6, "{got:?}");
}

#[test]
fn the_buffer_adds_them_cut_to_itself() {
    let mut buf = GlyphBuffer::new(
        4,
        2,
        crate::frame::Cell::new(
            ' ',
            crate::frame::Rgb::new(0, 0, 0),
            crate::frame::Rgb::new(0, 0, 0),
        ),
    );
    buf.nine_slice(&slice(), Rect::new(-5, -5, 100, 100), Layer::Under);
    let bounds = buf.pixel_bounds();
    let sprites = buf.sprites();
    assert_ne!(sprites.len(), 0);
    assert!(
        sprites
            .iter()
            .all(|s| bounds.intersect(&s.clip) == Some(s.clip))
    );
}

proptest! {
    /// However big the rectangle, the clips cover it exactly once with a
    /// stretched middle: no gap and no overlap.
    #[test]
    fn the_parts_tile_the_rectangle(w in 1i32..80, h in 1i32..60, zoom in 0u8..4, tile in any::<bool>()) {
        let mut s = slice().with_zoom(zoom);
        if tile {
            s = s.with_edges(EdgeFill::Tile).with_centre(CentreFill::Tile);
        }
        let dest = Rect::new(3, -2, w, h);
        let got = parts(&s, dest);
        let area: i64 = got.iter().map(|s| i64::from(s.clip.w) * i64::from(s.clip.h)).sum();
        prop_assert_eq!(area, i64::from(w) * i64::from(h));
        for (i, a) in got.iter().enumerate() {
            prop_assert!(dest.intersect(&a.clip) == Some(a.clip));
            prop_assert!(a.dest.intersect(&a.clip) == Some(a.clip));
            for b in &got[i + 1..] {
                prop_assert!(a.clip.intersect(&b.clip).is_none(), "{:?} and {:?}", a, b);
            }
        }
    }
}
