use alloc::collections::BTreeSet;

use insta::assert_snapshot;

use super::*;
use crate::frame::color::tests::engine_palette;
use crate::frame::glyph_buffer::Item;
use crate::frame::raster::MISSING_COLOR;

/// Every kind of thing a frame draws, as [`kinds`] names them.
const ALL: [&str; 8] = [
    "Item::Rect",
    "Item::Sprite",
    "Layer::Under",
    "Layer::Over",
    "Paint::Image",
    "Paint::Solid",
    "Paint::Dimmed",
    "Paint::Shaded",
];

/// What `item` is: its `Item` variant, its `Layer` and a sprite's `Paint`.
/// Every `match` here has no `_` arm, so a new variant doesn't compile
/// until it is named here; then [`ALL`] names it, and the card shows it.
fn kinds(item: &Item) -> Vec<&'static str> {
    let layer = match item.layer() {
        Layer::Under => "Layer::Under",
        Layer::Over => "Layer::Over",
    };
    match item {
        Item::Rect(_) => vec!["Item::Rect", layer],
        Item::Sprite(sprite) => {
            let paint = match sprite.paint {
                Paint::Image => "Paint::Image",
                Paint::Solid(_) => "Paint::Solid",
                Paint::Dimmed => "Paint::Dimmed",
                Paint::Shaded(_) => "Paint::Shaded",
            };
            vec!["Item::Sprite", layer, paint]
        }
    }
}

/// The card's items, and its backdrop scene's.
fn items(card: &RenderCard) -> Vec<Item> {
    let mut items = card.buffer.items().to_vec();
    if let Some(backdrop) = card.buffer.backdrop() {
        items.extend_from_slice(backdrop.scene().items());
    }
    items
}

/// The card's sprites, and its backdrop scene's.
fn sprites(card: &RenderCard) -> Vec<Sprite> {
    let sprite = |item: Item| match item {
        Item::Sprite(sprite) => Some(sprite),
        Item::Rect(_) => None,
    };
    items(card).into_iter().filter_map(sprite).collect()
}

#[test]
fn every_item_paint_and_layer_is_on_the_card() {
    let card = build();
    let mut count = BTreeMap::new();
    for kind in items(&card).iter().flat_map(kinds) {
        *count.entry(kind).or_insert(0) += 1;
    }
    let shown: BTreeSet<_> = count.keys().copied().collect();
    assert_eq!(shown, ALL.into_iter().collect(), "{count:?}");
    for (kind, n) in &count {
        assert!(*n >= 2, "{kind} is on the card {n} times, not twice");
    }
}

#[test]
fn the_card_fits_320_by_240() {
    let card = build();
    let bounds = card.buffer.pixel_bounds();
    assert_eq!((bounds.w, bounds.h), (256, 192));
    assert!(bounds.w <= 320 && bounds.h <= 240);
}

#[test]
fn its_sprites_take_every_setting() {
    let card = build();
    let sprites = sprites(&card);
    let values = |f: fn(&Sprite) -> u8| sprites.iter().map(f).collect::<BTreeSet<_>>();
    assert_eq!(values(|s| s.opacity), BTreeSet::from([128, 255]));
    assert_eq!(values(|s| s.brightness), BTreeSet::from([102, 255]));
    assert!(sprites.iter().any(|s| s.flip_x));
    assert!(sprites.iter().any(|s| s.flip_x && s.clip != s.dest));
    assert!(sprites.iter().filter(|s| s.clip != s.dest).count() >= 3);
    let missing = card.table.id(MISSING_PATH).unwrap();
    assert!(!card.images.contains_key(&missing));
    assert!(sprites.iter().any(|s| s.image == missing));
    let test_card = card.table.id(TEST_CARD_PATH).unwrap();
    assert!(card.images.contains_key(&test_card));
}

#[test]
fn its_cells_take_every_setting() {
    let card = build();
    let buf = &card.buffer;
    let cells: Vec<Cell> = (0..i32::from(ROWS))
        .flat_map(|y| (0..i32::from(COLUMNS)).map(move |x| (x, y)))
        .filter_map(|(x, y)| buf.get(x, y).copied())
        .collect();
    let solid = || cells.iter().filter(|c| !c.see_through);
    let backgrounds: BTreeSet<_> = solid().map(|c| c.bg).collect();
    assert!(backgrounds.len() >= 6, "{backgrounds:?}");
    let glyphs = || cells.iter().filter(|c| c.glyph != ' ');
    assert!(glyphs().map(|c| c.fg).collect::<BTreeSet<_>>().len() >= 4);
    assert!(glyphs().any(|c| card.atlas.glyph_rect(c.glyph).is_none()));
    assert!(cells.iter().any(|c| c.see_through && c.glyph != ' '));
    assert!(solid().any(|c| c.bg == card.clear && c.glyph != ' '));
}

#[test]
fn its_backdrop_is_zoomed_and_offset_by_half_a_scene_pixel() {
    let card = build();
    let backdrop = card.buffer.backdrop().unwrap();
    assert_eq!(backdrop.zoom(), 2);
    assert_eq!(backdrop.origin_px(), (0.5, 0.5));
    // At the window's scale 3, half a scene pixel is 3 window pixels.
    assert_eq!(backdrop.scene_offset(3), (-3, -3));
}

#[test]
fn the_painter_shows_what_is_missing_in_the_missing_colour() {
    let card = build();
    let image = card.painter().render(&card.buffer, 1);
    assert_eq!((image.width, image.height), (256, 192));
    let Rgb { r, g, b } = MISSING_COLOR;
    // The missing picture, the seventh of the first row of sprites.
    assert_eq!(image.pixel(140, 40), Some([r, g, b, 255]));
    // The fallback glyph, a checkerboard, in cell (0, 1).
    assert_eq!(image.pixel(2, 18), Some([r, g, b, 255]));
    let [cr, cg, cb] = [CLEAR.r, CLEAR.g, CLEAR.b];
    assert_eq!(image.pixel(1, 18), Some([cr, cg, cb, 255]));
}

#[test]
fn the_card_is_the_same_every_time() {
    assert_eq!(build(), build());
    let card = build();
    let painter = card.painter();
    assert_eq!(
        painter.render(&card.buffer, 3),
        painter.render(&card.buffer, 3)
    );
}

/// The atlas's picture, a row of text per pixel row: `#` where a glyph's
/// pixel is set, `.` where it is see-through. Its cells: the space, `#`,
/// `o`, `-`, `|` and the fallback.
const ATLAS: &str = "\
...................................##...........
...................................##...........
.........######....................##.....#.#.#.
.........######..######............##....#.#.#..
.........######..#....#............##.....#.#.#.
.........######..#....#............##....#.#.#..
.........######..#....#............##.....#.#.#.
.........######..#....#.########...##....#.#.#..
.........######..#....#.########...##.....#.#.#.
.........######..#....#............##....#.#.#..
.........######..#....#............##.....#.#.#.
.........######..#....#............##....#.#.#..
.........######..######............##.....#.#.#.
.........######....................##....#.#.#..
...................................##...........
...................................##...........
";

#[test]
fn the_atlas_is_boxes_and_bars() {
    let card = build();
    let image = &card.atlas_image;
    let mut text = String::new();
    for y in 0..image.height {
        for x in 0..image.width {
            let [r, g, b, a] = image.pixel(x, y).unwrap();
            text.push(match [r, g, b, a] {
                [255, 255, 255, 255] => '#',
                [0, 0, 0, 0] => '.',
                _ => '?',
            });
        }
        text.push('\n');
    }
    assert_eq!(text, ATLAS);
    let at = |glyph| card.atlas.glyph_rect(glyph).map(|r| (r.x, r.y));
    assert_eq!(at(' '), Some((0, 0)));
    assert_eq!(at(FALLBACK_GLYPH), Some((40, 0)));
}

#[test]
fn the_card_is_as_its_snapshot_says() {
    // Every cell, item and backdrop of the card, so a change to what it
    // shows is seen in review.
    assert_snapshot!(build().buffer.to_snapshot(&engine_palette()));
}
