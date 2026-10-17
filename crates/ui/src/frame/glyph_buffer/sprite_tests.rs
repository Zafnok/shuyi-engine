//! Sprite items (ticket 0231, ADR-0038): clipping, cutting and `blit`
//! follow the rules overlays have.

use std::collections::HashMap;
use std::rc::Rc;

use proptest::prelude::*;
use shuyi_assets::image::{ImageInfo, ImageTable};

use super::*;
use crate::frame::console::ConsoleSpec;

const FG: Rgb = Rgb::new(200, 200, 200);
const BG: Rgb = Rgb::new(0, 0, 0);
const RED: Rgb = Rgb::new(255, 0, 0);
const BLANK: Cell = Cell::new(' ', FG, BG);
/// The whole of the 16×16 test images.
const SRC: PxRect = Rect::new(0, 0, 16, 16);

fn buf(w: u16, h: u16) -> GlyphBuffer {
    GlyphBuffer::new(w, h, BLANK)
}

/// Two made-up 16×16 images.
fn images() -> [ImageId; 2] {
    let size = ImageInfo {
        width: 16,
        height: 16,
    };
    let table = ImageTable {
        images: [("a.png", size), ("b.png", size)].into(),
    };
    [table.id("a.png").unwrap(), table.id("b.png").unwrap()]
}

/// A sprite of image `a.png` drawn at `dest`.
fn at(x: i32, y: i32, w: i32, h: i32) -> Sprite {
    Sprite::new(images()[0], SRC, Rect::new(x, y, w, h), Layer::Over)
}

fn clips(b: &GlyphBuffer) -> Vec<PxRect> {
    b.sprites().iter().map(|s| s.clip).collect()
}

#[test]
fn new_sprite_is_solid_unflipped_and_shows_all_of_dest() {
    let [a, b] = images();
    assert_ne!(a, b);
    let dest = Rect::new(8, 8, 48, 48);
    let s = Sprite::new(b, Rect::new(1, 2, 3, 4), dest, Layer::Under);
    assert_eq!(
        s,
        Sprite {
            image: b,
            src: Rect::new(1, 2, 3, 4),
            dest,
            clip: dest,
            layer: Layer::Under,
            flip_x: false,
            opacity: 255,
            brightness: 255,
            paint: Paint::Image,
            base: None,
        }
    );
}

/// A 16 × 20 picture standing on the tile of cells (2, 1) and (3, 1): the
/// pixels 16..32 × 16..32, its feet 2 px above the tile's bottom, its head
/// 6 px into the cells above. `dx` moves it sideways (an outline's copy).
fn standing(dx: i32) -> Sprite {
    Sprite {
        base: Some(Rect::new(16, 16, 16, 16)),
        ..at(16 + dx, 10, 16, 20)
    }
}

#[test]
fn cells_replaced_over_a_base_take_what_stands_above_them() {
    let cut = |sprite: Sprite, cells: Rect| {
        let mut b = buf(8, 4);
        b.add_sprite(sprite);
        b.fill_rect(cells, BLANK);
        let mut left = clips(&b);
        left.sort_by_key(|r| (r.y, r.x));
        left
    };
    let tile = Rect::new(2, 1, 2, 1);
    // The whole tile: nothing is left, not even the head above it.
    assert!(cut(standing(0), tile).is_empty());
    // Without a base, the head stays (cells own only what is over them).
    assert_eq!(cut(at(16, 10, 16, 20), tile), [Rect::new(16, 10, 16, 6)]);
    // A copy 1 px left or right of the tile goes whole too.
    assert!(cut(standing(-1), tile).is_empty());
    assert!(cut(standing(1), tile).is_empty());
    // Half the tile: that half, and the head above it.
    let left_half = Rect::new(2, 1, 1, 1);
    assert_eq!(
        cut(standing(0), left_half),
        [Rect::new(24, 10, 8, 6), Rect::new(24, 16, 8, 14)]
    );
    // The copy 1 px left also loses its pixel beside the tile.
    assert_eq!(
        cut(standing(-1), left_half),
        [Rect::new(24, 10, 7, 6), Rect::new(24, 16, 7, 14)]
    );
    // The cells above only: the head, as for any item; the rest stays.
    let above = Rect::new(2, 0, 2, 1);
    assert_eq!(cut(standing(0), above), [Rect::new(16, 16, 16, 14)]);
    // Cells beside the tile, or below it: nothing of this sprite.
    for beside in [
        Rect::new(0, 1, 2, 1),
        Rect::new(4, 1, 2, 1),
        Rect::new(2, 2, 2, 1),
    ] {
        assert_eq!(cut(standing(0), beside), [Rect::new(16, 10, 16, 20)]);
    }
    // On the tile at the buffer's left edge, the copy 1 px right goes
    // whole too: nothing is left in the column past the tile.
    let first = Sprite {
        base: Some(Rect::new(0, 16, 16, 16)),
        ..at(1, 10, 16, 20)
    };
    assert!(cut(first, Rect::new(0, 1, 2, 1)).is_empty());
    // A base two cells tall, its top cell replaced: the head goes, and
    // what stands on the lower cell stays.
    let tall = Sprite {
        base: Some(Rect::new(16, 16, 16, 32)),
        ..at(16, 10, 16, 36)
    };
    assert_eq!(cut(tall, tile), [Rect::new(16, 32, 16, 14)]);
    // A sprite whose base is elsewhere is cut as any item.
    let elsewhere = Sprite {
        base: Some(Rect::new(48, 16, 16, 16)),
        ..at(16, 10, 16, 20)
    };
    assert_eq!(cut(elsewhere, tile), [Rect::new(16, 10, 16, 6)]);
    // A blit moves the base with the picture.
    let mut small = buf(4, 3);
    small.add_sprite(standing(0));
    let mut b = buf(8, 4);
    b.blit(&small, 2, 1);
    let moved = b.sprites()[0];
    assert_eq!(moved.dest, Rect::new(32, 26, 16, 20));
    assert_eq!(moved.base, Some(Rect::new(32, 32, 16, 16)));
}

#[test]
fn a_paint_colours_a_pixel_and_a_painted_sprite_is_otherwise_the_same() {
    let grey = Rgb::new(200, 200, 200);
    assert_eq!(Paint::default(), Paint::Image);
    assert_eq!(Paint::Image.apply(RED), RED);
    assert_eq!(Paint::Solid(grey).apply(RED), grey);
    assert_eq!(Paint::Dimmed.apply(RED), RED.dimmed());
    assert_ne!(RED.dimmed(), RED);
    // A light's own sum (ADR-0058); the plain light changes nothing.
    let dusk = shuyi_assets::lighting::Light::new(0.35, [0.62, 0.56, 0.46]);
    let shaded = Paint::Shaded(dusk).apply(RED);
    assert_eq!(shaded, dusk.apply([RED.r, RED.g, RED.b]).into());
    assert_ne!(shaded, RED);
    assert_eq!(
        Paint::Shaded(shuyi_assets::lighting::Light::PLAIN).apply(RED),
        RED
    );
    let s = at(8, 8, 48, 48);
    let solid = s.painted(Paint::Solid(RED));
    assert_eq!(solid.paint, Paint::Solid(RED));
    assert_eq!(solid.painted(Paint::Image), s);
    // The paint stays through clipping and cutting.
    let mut b = buf(4, 2);
    b.add_sprite(at(-8, 0, 32, 32).painted(Paint::Dimmed));
    b.fill_rect(Rect::new(1, 0, 1, 1), BLANK);
    assert!(b.sprites().len() > 1);
    assert!(b.sprites().iter().all(|s| s.paint == Paint::Dimmed));
}

#[test]
fn add_sprite_clips_only_the_clip() {
    let mut b = buf(3, 2);
    // Inside: kept as given.
    b.add_sprite(at(4, 8, 16, 16));
    // Over the right and bottom edges, and the left and top ones.
    b.add_sprite(at(16, 24, 16, 16));
    b.add_sprite(at(-8, -4, 16, 16));
    assert_eq!(
        b.sprites(),
        [
            at(4, 8, 16, 16),
            Sprite {
                clip: Rect::new(16, 24, 8, 8),
                ..at(16, 24, 16, 16)
            },
            Sprite {
                clip: Rect::new(0, 0, 8, 12),
                ..at(-8, -4, 16, 16)
            },
        ]
    );
    assert!(b.overlays().is_empty());
    assert_eq!(b.items().len(), 3);
}

#[test]
fn add_sprite_keeps_a_given_clip_inside_dest() {
    let mut b = buf(4, 2);
    // A clip partly outside `dest` is cut to it.
    b.add_sprite(Sprite {
        clip: Rect::new(0, 0, 12, 40),
        ..at(8, 8, 16, 16)
    });
    assert_eq!(clips(&b), [Rect::new(8, 8, 4, 16)]);
}

#[test]
fn add_sprite_drops_what_shows_nothing() {
    let mut b = buf(3, 2);
    // Wholly outside, on each side.
    b.add_sprite(at(24, 0, 16, 16));
    b.add_sprite(at(0, 32, 16, 16));
    b.add_sprite(at(-16, 0, 16, 16));
    b.add_sprite(at(0, -16, 16, 16));
    b.add_sprite(at(i32::MAX, i32::MIN, i32::MAX, 16));
    // Empty `dest`, `src` or `clip`, or a `clip` beside `dest`.
    b.add_sprite(at(0, 0, 0, 16));
    b.add_sprite(Sprite {
        src: Rect::new(0, 0, 16, 0),
        ..at(0, 0, 16, 16)
    });
    b.add_sprite(Sprite {
        src: Rect::new(0, 0, -1, 16),
        ..at(0, 0, 16, 16)
    });
    b.add_sprite(Sprite {
        clip: Rect::new(0, 0, 0, 16),
        ..at(0, 0, 16, 16)
    });
    b.add_sprite(Sprite {
        clip: Rect::new(16, 0, 8, 16),
        ..at(0, 0, 16, 16)
    });
    assert_eq!(b.items(), []);
    // An invisible sprite is still a sprite.
    b.add_sprite(Sprite {
        opacity: 0,
        ..at(0, 0, 16, 16)
    });
    assert_eq!(b.sprites().len(), 1);
}

#[test]
fn items_keep_the_order_they_were_added_in() {
    let mut b = buf(4, 2);
    let rect = Overlay::new(Rect::new(0, 0, 8, 2), RED, Layer::Under);
    let under = Sprite {
        layer: Layer::Under,
        ..at(8, 0, 16, 16)
    };
    b.add_sprite(at(0, 0, 16, 16));
    b.add_overlay(rect);
    b.add_sprite(under);
    assert_eq!(
        b.items(),
        [
            Item::Sprite(at(0, 0, 16, 16)),
            Item::Rect(rect),
            Item::Sprite(under)
        ]
    );
    assert_eq!(b.overlays(), [rect]);
    assert_eq!(b.sprites(), [at(0, 0, 16, 16), under]);
    let layers: Vec<Layer> = b.items().iter().map(Item::layer).collect();
    assert_eq!(layers, [Layer::Over, Layer::Under, Layer::Under]);
    let visible: Vec<PxRect> = b.items().iter().map(Item::visible).collect();
    assert_eq!(
        visible,
        [
            Rect::new(0, 0, 16, 16),
            Rect::new(0, 0, 8, 2),
            Rect::new(8, 0, 16, 16)
        ]
    );
}

#[test]
fn fill_rect_over_half_a_sprite_leaves_the_rest() {
    let mut b = buf(4, 2);
    let whole = Sprite {
        flip_x: true,
        opacity: 100,
        ..at(0, 0, 32, 32)
    };
    b.add_sprite(whole);
    // The right half's cells are replaced.
    b.fill_rect(Rect::new(2, 0, 2, 2), BLANK);
    assert_eq!(
        b.sprites(),
        [Sprite {
            clip: Rect::new(0, 0, 16, 32),
            ..whole
        }]
    );
    // One cell in the middle of what's left: four parts, same picture.
    let mut b = buf(6, 3);
    let whole = at(0, 0, 48, 48);
    b.add_sprite(whole);
    b.fill_rect(Rect::new(2, 1, 1, 1), BLANK);
    let parts = [
        Rect::new(0, 0, 48, 16),
        Rect::new(0, 32, 48, 16),
        Rect::new(0, 16, 16, 16),
        Rect::new(24, 16, 24, 16),
    ];
    assert_eq!(b.sprites(), parts.map(|clip| Sprite { clip, ..whole }));
    // The parts cover exactly the rest: every pixel but the cell's, once.
    for y in 0..48 {
        for x in 0..48 {
            let n = parts.iter().filter(|p| p.contains(x, y)).count();
            let cut = Rect::new(16, 16, 8, 16).contains(x, y);
            assert_eq!(n, usize::from(!cut), "({x}, {y})");
        }
    }
    // Filling everything removes the sprite; fills beside it don't touch it.
    let mut b = buf(4, 2);
    b.add_sprite(at(0, 0, 16, 16));
    let before = b.clone();
    b.fill_rect(Rect::new(2, 0, 2, 2), BLANK);
    b.fill_rect(Rect::new(9, 0, 2, 2), BLANK);
    assert_eq!(b, before);
    b.fill_rect(b.bounds(), BLANK);
    assert_eq!(b.items(), []);
}

#[test]
fn blit_offsets_dest_and_clip_and_clips_the_clip() {
    let mut src = buf(2, 2);
    let s = Sprite {
        flip_x: true,
        opacity: 7,
        ..at(0, 8, 16, 16)
    };
    src.add_sprite(s);
    src.add_overlay(Overlay::new(Rect::new(0, 0, 16, 2), RED, Layer::Under));
    // Wholly inside: both rectangles move by one cell each way.
    let mut b = buf(4, 4);
    b.blit(&src, 1, 1);
    let moved = Sprite {
        dest: Rect::new(8, 24, 16, 16),
        clip: Rect::new(8, 24, 16, 16),
        ..s
    };
    assert_eq!(
        b.items(),
        [
            Item::Sprite(moved),
            Item::Rect(Overlay::new(Rect::new(8, 16, 16, 2), RED, Layer::Under)),
        ]
    );
    // Over the left and top edges: `dest` moves, `clip` is what's left.
    let mut b = buf(4, 4);
    b.blit(&src, -1, -1);
    assert_eq!(
        b.sprites(),
        [Sprite {
            dest: Rect::new(-8, -8, 16, 16),
            clip: Rect::new(0, 0, 8, 8),
            ..s
        }]
    );
    assert!(b.overlays().is_empty());
    // Over the right and bottom edges.
    let mut b = buf(4, 4);
    b.blit(&src, 3, 3);
    assert_eq!(
        b.sprites(),
        [Sprite {
            dest: Rect::new(24, 56, 16, 16),
            clip: Rect::new(24, 56, 8, 8),
            ..s
        }]
    );
    // Wholly off the buffer: nothing comes along.
    let mut b = buf(4, 4);
    b.blit(&src, 4, 0);
    b.blit(&src, i32::MAX, i32::MIN);
    b.blit(&src, i32::MIN, i32::MAX);
    assert_eq!(b.items(), []);
}

#[test]
fn blit_keeps_a_cut_sprites_parts_and_replaces_what_was_there() {
    // The source's sprite was cut before the blit: both parts come along.
    let mut src = buf(4, 1);
    src.add_sprite(at(0, 0, 32, 16));
    src.fill_rect(Rect::new(1, 0, 1, 1), BLANK);
    assert_eq!(
        clips(&src),
        [Rect::new(0, 0, 8, 16), Rect::new(16, 0, 16, 16)]
    );
    let mut b = buf(6, 2);
    b.add_sprite(Sprite {
        image: images()[1],
        ..at(0, 0, 48, 32)
    });
    b.blit(&src, 2, 1);
    let dest = Rect::new(16, 16, 32, 16);
    let [a, old] = images();
    let got: Vec<(ImageId, PxRect, PxRect)> = b
        .sprites()
        .iter()
        .map(|s| (s.image, s.dest, s.clip))
        .collect();
    let whole = Rect::new(0, 0, 48, 32);
    assert_eq!(
        got,
        [
            // What was there, minus the cells blitted over.
            (old, whole, Rect::new(0, 0, 48, 16)),
            (old, whole, Rect::new(0, 16, 16, 16)),
            // The source's two parts.
            (a, dest, Rect::new(16, 16, 8, 16)),
            (a, dest, Rect::new(32, 16, 16, 16)),
        ]
    );
}

#[test]
fn offset_moves_or_fails() {
    let r = Rect::new(3, -4, 5, 6);
    assert_eq!(offset(r, 10, 20), Some(Rect::new(13, 16, 5, 6)));
    assert_eq!(offset(r, -10, -20), Some(Rect::new(-7, -24, 5, 6)));
    assert_eq!(offset(r, i64::from(i32::MAX), 0), None);
    assert_eq!(offset(r, 0, i64::from(i32::MIN)), None);
    let far = Rect::new(i32::MIN, i32::MAX, 1, 1);
    assert_eq!(
        offset(far, 5, -5),
        Some(Rect::new(i32::MIN + 5, i32::MAX - 5, 1, 1))
    );
}

#[test]
fn offset_clipped_cases() {
    let bounds = Rect::new(0, 0, 32, 48);
    let r = Rect::new(4, 6, 8, 10);
    assert_eq!(
        offset_clipped(r, 2, 3, bounds),
        Some(Rect::new(6, 9, 8, 10))
    );
    assert_eq!(
        offset_clipped(r, -8, -10, bounds),
        Some(Rect::new(0, 0, 4, 6))
    );
    assert_eq!(
        offset_clipped(r, 26, 40, bounds),
        Some(Rect::new(30, 46, 2, 2))
    );
    // Touching an edge from outside shows nothing.
    assert_eq!(offset_clipped(r, -12, 0, bounds), None);
    assert_eq!(offset_clipped(r, 0, -16, bounds), None);
    assert_eq!(offset_clipped(r, 28, 0, bounds), None);
    assert_eq!(offset_clipped(r, 0, 42, bounds), None);
    assert_eq!(offset_clipped(r, i64::MAX / 2, i64::MIN / 2, bounds), None);
}

/// Asserts `sprite`'s [`Sprite::clipped_src`] is `expected`, to within a
/// thousandth of an image pixel.
#[track_caller]
fn assert_clipped_src(sprite: Sprite, expected: [f32; 4]) {
    let got = sprite.clipped_src();
    let close = got.iter().zip(expected).all(|(g, e)| (g - e).abs() < 1e-3);
    assert!(close, "{got:?}, expected {expected:?}");
}

#[test]
fn clipped_src_is_the_part_of_src_under_clip() {
    // The whole sprite: all of `src`.
    let whole = Sprite::new(
        images()[0],
        Rect::new(2, 4, 8, 12),
        Rect::new(10, 20, 40, 36),
        Layer::Over,
    );
    assert_clipped_src(whole, [2.0, 4.0, 8.0, 12.0]);
    // 5× wide, 3× tall; the clip starts 10 px right and 6 px down.
    let part = Sprite {
        clip: Rect::new(20, 26, 15, 9),
        ..whole
    };
    assert_clipped_src(part, [4.0, 6.0, 3.0, 3.0]);
    // Flipped, the same clip shows the other side of `src`: it ends 15 px
    // before `dest`'s right edge, so starts 3 image pixels in.
    let flipped = Sprite {
        flip_x: true,
        ..part
    };
    assert_clipped_src(flipped, [5.0, 6.0, 3.0, 3.0]);
    let whole_flipped = Sprite {
        flip_x: true,
        ..whole
    };
    assert_clipped_src(whole_flipped, [2.0, 4.0, 8.0, 12.0]);
    // A clip through the middle of a scaled pixel gives fractions.
    let half = Sprite {
        clip: Rect::new(12, 20, 1, 36),
        ..whole
    };
    assert_clipped_src(half, [2.4, 4.0, 0.2, 12.0]);
    // Shrunk to half size.
    let small = Sprite {
        clip: Rect::new(2, 0, 4, 8),
        ..Sprite::new(images()[0], SRC, Rect::new(0, 0, 8, 8), Layer::Over)
    };
    assert_clipped_src(small, [4.0, 0.0, 8.0, 16.0]);
}

/// One step of the property test below.
#[derive(Debug, Clone)]
enum Op {
    Sprite { dest: PxRect, clip: PxRect },
    Overlay(PxRect),
    Fill(Rect),
    Blit { x: i32, y: i32, dest: PxRect },
}

fn coord() -> impl Strategy<Value = i32> {
    prop_oneof![8 => -40..200i32, 1 => any::<i32>()]
}

fn rect() -> impl Strategy<Value = Rect> {
    (coord(), coord(), coord(), coord()).prop_map(|(x, y, w, h)| Rect::new(x, y, w, h))
}

fn cell_coord() -> impl Strategy<Value = i32> {
    prop_oneof![8 => -4..24i32, 1 => any::<i32>()]
}

fn cell_rect() -> impl Strategy<Value = Rect> {
    (cell_coord(), cell_coord(), cell_coord(), cell_coord())
        .prop_map(|(x, y, w, h)| Rect::new(x, y, w, h))
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (rect(), rect()).prop_map(|(dest, clip)| Op::Sprite { dest, clip }),
        rect().prop_map(|dest| Op::Sprite { dest, clip: dest }),
        rect().prop_map(Op::Overlay),
        cell_rect().prop_map(Op::Fill),
        (cell_coord(), cell_coord(), rect()).prop_map(|(x, y, dest)| Op::Blit { x, y, dest }),
    ]
}

/// A sprite tagged `tag` (in `src.x`), so its parts can be told from other
/// sprites' after cuts and blits.
fn tagged(tag: i32, dest: PxRect, clip: PxRect) -> Sprite {
    Sprite {
        clip,
        ..Sprite::new(images()[0], Rect::new(tag, 0, 16, 16), dest, Layer::Over)
    }
}

/// The pixels of the cells of `rect` on a `w × h`-cell buffer.
fn cells_px(rect: Rect, b: &GlyphBuffer) -> Option<PxRect> {
    let r = rect.intersect(&b.bounds())?;
    Some(Rect::new(r.x * 8, r.y * 16, r.w * 8, r.h * 16))
}

proptest! {
    #[test]
    fn items_stay_inside_and_off_cells_filled_after_them(
        w in 0u16..24, h in 0u16..12, ops in prop::collection::vec(op(), 0..12),
    ) {
        let mut b = buf(w, h);
        // Per sprite tag: the pixels of cells replaced after it was added.
        let mut replaced: HashMap<i32, Vec<PxRect>> = HashMap::new();
        let mut next_tag = 0;
        for op in ops {
            match op {
                Op::Sprite { dest, clip } => {
                    b.add_sprite(tagged(next_tag, dest, clip));
                    replaced.insert(next_tag, Vec::new());
                    next_tag += 1;
                }
                Op::Overlay(rect) => b.add_overlay(Overlay::new(rect, RED, Layer::Under)),
                Op::Fill(rect) => {
                    b.fill_rect(rect, BLANK);
                    for holes in replaced.values_mut() {
                        holes.extend(cells_px(rect, &b));
                    }
                }
                Op::Blit { x, y, dest } => {
                    let mut src = buf(5, 3);
                    src.add_sprite(tagged(next_tag, dest, dest));
                    src.add_overlay(Overlay::new(dest, RED, Layer::Over));
                    b.blit(&src, x, y);
                    for holes in replaced.values_mut() {
                        holes.extend(cells_px(Rect::new(x, y, 5, 3), &b));
                    }
                    // The blitted sprite arrives after its cells did.
                    replaced.insert(next_tag, Vec::new());
                    next_tag += 1;
                }
            }
        }
        let px = b.pixel_bounds();
        for item in b.items() {
            let visible = item.visible();
            prop_assert_eq!(visible.intersect(&px), Some(visible));
        }
        for s in b.sprites() {
            // A clip never leaves its picture's place.
            prop_assert_eq!(s.clip.intersect(&s.dest), Some(s.clip));
            for hole in &replaced[&s.src.x] {
                prop_assert_eq!(s.clip.intersect(hole), None, "{:?} over {:?}", s, hole);
            }
        }
    }

    #[test]
    fn cutting_and_clipping_never_change_src_or_dest(
        w in 1u16..24, h in 1u16..12, dest in rect(), fill in cell_rect(),
    ) {
        let mut b = buf(w, h);
        let s = tagged(7, dest, dest);
        b.add_sprite(s);
        b.fill_rect(fill, BLANK);
        for part in b.sprites() {
            prop_assert_eq!(Sprite { clip: s.clip, ..part }, s);
        }
        // Nothing is drawn twice: the parts don't overlap.
        let parts = b.sprites();
        for (i, a) in parts.iter().enumerate() {
            for other in &parts[i + 1..] {
                prop_assert_eq!(a.clip.intersect(&other.clip), None);
            }
        }
    }
}

/// Ticket 1030: the frame takes the console's size from a [`ConsoleSpec`].
#[test]
fn a_playdate_console_is_50_by_15_cells_of_400_by_240_pixels() {
    let mut b = GlyphBuffer::with_spec(ConsoleSpec::text(50, 15), BLANK);
    assert_eq!(b.spec(), ConsoleSpec::text(50, 15));
    assert_eq!((b.width(), b.height()), (50, 15));
    assert_eq!(b.bounds(), Rect::new(0, 0, 50, 15));
    assert_eq!(b.pixel_bounds(), Rect::new(0, 0, 400, 240));

    // Text stops at the 50th column.
    b.print(45, 14, "corner!", RED, BG);
    let last: String = (40..50).map(|x| b.get(x, 14).unwrap().glyph).collect();
    assert_eq!(last, "     corne");
    assert_eq!(b.get(50, 14), None);

    // A sprite over the bottom-right corner is clipped to 400 × 240.
    b.add_sprite(at(392, 232, 16, 16));
    assert_eq!(clips(&b), [Rect::new(392, 232, 8, 8)]);

    // A backdrop's window is counted in the console's cells.
    let scene = GlyphBuffer::with_spec(ConsoleSpec::text(50, 15).sized(10, 4), BLANK);
    b.set_backdrop(Rc::new(scene), Rect::new(40, 10, 20, 20), (0.0, 0.0), 1);
    let backdrop = b.backdrop().unwrap();
    assert_eq!(backdrop.clip(), Rect::new(40, 10, 10, 5));
    assert_eq!(backdrop.clip_px(), Rect::new(320, 160, 80, 80));
}

/// The cell size comes from the spec too: pixels, cutting and `blit`
/// count in a 4 × 6 pixel cell here.
#[test]
fn every_pixel_rule_uses_the_specs_cell_size() {
    let small = ConsoleSpec {
        cols: 10,
        rows: 5,
        cell_w: 4,
        cell_h: 6,
    };
    let mut b = GlyphBuffer::with_spec(small, BLANK);
    assert_eq!(b.pixel_bounds(), Rect::new(0, 0, 40, 30));

    // Filling cell (1, 1) cuts the sprite's pixels 4..8 × 6..12.
    b.add_sprite(at(0, 0, 12, 12));
    b.fill_rect(Rect::new(1, 1, 1, 1), BLANK);
    let mut cut = clips(&b);
    cut.sort_by_key(|r| (r.y, r.x));
    assert_eq!(
        cut,
        [
            Rect::new(0, 0, 12, 6),
            Rect::new(0, 6, 4, 6),
            Rect::new(8, 6, 4, 6),
        ]
    );

    // A blit at cell (2, 1) moves the other buffer's sprite by 8 × 6 px.
    let mut src = GlyphBuffer::with_spec(small.sized(3, 2), BLANK);
    src.add_sprite(at(0, 0, 4, 4));
    let mut dest = GlyphBuffer::with_spec(small, BLANK);
    dest.blit(&src, 2, 1);
    assert_eq!(clips(&dest), [Rect::new(8, 6, 4, 4)]);

    let scene = Rc::new(GlyphBuffer::with_spec(small.sized(2, 2), BLANK));
    b.set_backdrop(scene, Rect::new(1, 2, 3, 2), (0.0, 0.0), 1);
    assert_eq!(b.backdrop().unwrap().clip_px(), Rect::new(4, 12, 12, 12));
}
