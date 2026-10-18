use std::collections::BTreeMap;

use super::*;
use crate::font_atlas::{BitmapFont, build_atlas};
use crate::project::GlyphTile;

/// A font of 2 × 3 cells: `a` all on, `b` only its top-left pixel.
fn font() -> (FontAtlasDef, u32, Vec<u8>) {
    let glyphs = BTreeMap::from([
        ('a', vec![true; 6]),
        ('b', vec![true, false, false, false, false, false]),
    ]);
    let atlas = build_atlas(&BitmapFont {
        cell_w: 2,
        cell_h: 3,
        glyphs,
    });
    (atlas.def, atlas.width, atlas.rgba)
}

fn palette() -> PaletteDef {
    PaletteDef {
        colors: BTreeMap::from([
            ("ink".to_owned(), [200, 0, 0]),
            ("paper".to_owned(), [0, 0, 100]),
        ]),
    }
}

fn tile(glyphs: &str, bg: Option<&str>) -> GlyphTile {
    GlyphTile {
        glyphs: glyphs.to_owned(),
        fg: "ink".to_owned(),
        bg: bg.map(str::to_owned),
    }
}

/// Tiles of 6 px, `columns` to a row.
fn rules(columns: u32, tiles: Vec<GlyphTile>) -> GlyphTiles {
    GlyphTiles {
        tile: 6,
        columns,
        tiles,
        ..GlyphTiles::default()
    }
}

fn draw(rules: &GlyphTiles) -> Result<Tileset, String> {
    let (atlas, width, rgba) = font();
    render(rules, &atlas, (width, &rgba), &palette())
}

const INK: [u8; 4] = [200, 0, 0, 255];
const PAPER: [u8; 4] = [0, 0, 100, 255];
const CLEAR: [u8; 4] = [0; 4];

#[test]
fn tiles_go_left_to_right_then_down() {
    let r = rules(2, vec![tile("a", None); 3]);
    assert_eq!(tile_origin(&r, 0), (0, 0));
    assert_eq!(tile_origin(&r, 1), (6, 0));
    assert_eq!(tile_origin(&r, 2), (0, 6));
    assert_eq!(tile_origin(&r, 3), (6, 6));
    let tiles = draw(&r).unwrap();
    assert_eq!((tiles.width, tiles.height), (12, 12));
    assert_eq!(tiles.rgba.len(), 12 * 12 * 4);
}

#[test]
fn a_tiles_glyphs_are_centred_on_its_square() {
    // Two glyphs of 2 × 3 in a 6 px tile: from (1, 1) to (4, 3).
    let tiles = draw(&rules(1, vec![tile("ab", Some("paper"))])).unwrap();
    for y in 0..6 {
        for x in 0..6 {
            let a = (1..3).contains(&x) && (1..4).contains(&y);
            let b = (x, y) == (3, 1);
            let want = if a || b { INK } else { PAPER };
            assert_eq!(tiles.pixel(x, y), want, "({x}, {y})");
        }
    }
    // Outside the picture: clear.
    assert_eq!(tiles.pixel(6, 0), CLEAR);
    assert_eq!(tiles.pixel(0, 6), CLEAR);
}

#[test]
fn glyphs_that_just_fit_are_drawn_and_an_odd_tile_rounds_down() {
    // Three glyphs of 2 px fill a 6 px tile; a 3 px glyph fills a 3 px one.
    assert!(draw(&rules(1, vec![tile("aaa", None)])).is_ok());
    let mut tall = rules(1, vec![tile("a", None)]);
    tall.tile = 3;
    assert!(draw(&tall).is_ok());
    // In a 7 px tile, `a` (2 × 3) goes at (2, 2): the 5 and 4 px left over
    // halved, rounded down.
    let mut odd = rules(1, vec![tile("a", Some("paper"))]);
    odd.tile = 7;
    let tiles = draw(&odd).unwrap();
    for y in 0..7 {
        for x in 0..7 {
            let on = (2..4).contains(&x) && (2..5).contains(&y);
            let want = if on { INK } else { PAPER };
            assert_eq!(tiles.pixel(x, y), want, "({x}, {y})");
        }
    }
}

#[test]
fn a_tile_with_no_background_is_clear_but_for_its_glyphs() {
    let tiles = draw(&rules(2, vec![tile("a", Some("paper")), tile("b", None)])).unwrap();
    // The second tile, at x 6: one glyph of 2 × 3, from (2, 1).
    assert_eq!(tiles.pixel(6 + 2, 1), INK);
    for (x, y) in [(6, 0), (6 + 3, 1), (6 + 2, 2), (11, 5)] {
        assert_eq!(tiles.pixel(x, y), CLEAR, "({x}, {y})");
    }
    assert_eq!(tiles.pixel(0, 0), PAPER);
}

#[test]
fn a_partly_covered_pixel_blends_or_takes_the_cover_as_its_opacity() {
    let mut tiles = Tileset {
        width: 2,
        height: 1,
        rgba: [PAPER, CLEAR].concat(),
    };
    tiles.stamp(0, 0, [200, 0, 0], 51);
    tiles.stamp(1, 0, [200, 0, 0], 51);
    // 0 → 200 by a fifth: 40; 100 → 0 by a fifth: 80.
    assert_eq!(tiles.pixel(0, 0), [40, 0, 80, 255]);
    assert_eq!(tiles.pixel(1, 0), [200, 0, 0, 51]);
    assert_eq!(blend(0, 255, 255), 255);
    assert_eq!(blend(255, 0, 0), 255);
    assert_eq!(blend(0, 255, 128), 128);
}

#[test]
fn what_cant_be_drawn_is_refused_and_named() {
    let refused = |r: GlyphTiles| draw(&r).unwrap_err();
    assert_eq!(
        refused(rules(1, vec![tile("", None)])),
        "tile 0: 0 glyphs, not 1 to 3"
    );
    assert_eq!(
        refused(rules(1, vec![tile("a", None), tile("abab", None)])),
        "tile 1: 4 glyphs, not 1 to 3"
    );
    assert_eq!(
        refused(rules(1, vec![tile("c", None)])),
        "tile 0: the font has no 'c'"
    );
    assert_eq!(
        refused(rules(1, vec![tile("a", Some("sky"))])),
        "the palette has no color \"sky\""
    );
    let mut no_ink = tile("a", None);
    no_ink.fg = "sky".to_owned();
    assert_eq!(
        refused(rules(1, vec![no_ink])),
        "the palette has no color \"sky\""
    );
    let mut small = rules(1, vec![tile("aaa", None)]);
    small.tile = 5;
    assert_eq!(refused(small), "tile 0: \"aaa\" doesn't fit a 5 px tile");
    let mut short = rules(1, vec![tile("a", None)]);
    short.tile = 2;
    assert_eq!(refused(short), "tile 0: \"a\" doesn't fit a 2 px tile");
    for (tile_px, columns) in [(0, 1), (6, 0)] {
        let mut zero = rules(columns, vec![tile("a", None)]);
        zero.tile = tile_px;
        assert_eq!(refused(zero), "tile and columns must be above 0");
    }
}

#[test]
fn run_reads_the_font_and_palette_and_writes_the_picture() {
    let root = std::env::temp_dir().join(format!("shuyi-glyph-tiles-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("font")).unwrap();
    let (atlas, width, rgba) = font();
    let height = u32::try_from(rgba.len()).unwrap() / 4 / width;
    let png = encode_png(width, height, &rgba).unwrap();
    fs::write(root.join("font/atlas.png"), png).unwrap();
    let ron = crate::font_atlas::atlas_ron(&atlas, "tiny.bdf", "shuyi", None).unwrap();
    fs::write(root.join("font/atlas.ron"), ron).unwrap();
    fs::write(
        root.join("palette.ron"),
        r##"{ "ink": "#c80000", "paper": "#000064" }"##,
    )
    .unwrap();
    let mut r = rules(2, vec![tile("ab", Some("paper")), tile("b", None)]);
    r.font = "font".to_owned();
    r.palette = "palette.ron".to_owned();
    r.out = "out/tiles.png".to_owned();
    assert_eq!(
        run(&r, &root).unwrap(),
        "glyph-tiles: 2 tiles, 12×6 px → out/tiles.png"
    );
    let (w, h, written) = decode_png(&fs::read(root.join("out/tiles.png")).unwrap()).unwrap();
    assert_eq!((w, h), (12, 6));
    assert_eq!(written, draw(&r).unwrap().rgba);
    // A palette that doesn't read, then no font at all.
    fs::write(root.join("palette.ron"), "{ \"ink\": \"red\" }").unwrap();
    assert!(run(&r, &root).unwrap_err().contains("palette.ron"));
    fs::remove_dir_all(root.join("font")).unwrap();
    assert!(run(&r, &root).unwrap_err().starts_with("reading "));
    fs::remove_dir_all(&root).unwrap();
}
