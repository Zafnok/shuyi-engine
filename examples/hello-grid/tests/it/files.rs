//! The game's files: they load, and the generated ones are what the
//! engine's tools make from their sources and `shuyi.ron` (the font atlas
//! from the BDF, the tileset from the font and the palette), so an edit
//! that forgets to regenerate them fails here.

use std::path::Path;

use hello_grid::map::{FOE_TILE, HERO_TILE, TERRAIN_LOOKS, TILE_COLUMNS, TILE_PX, piece_look};
use hello_grid::rules::Side;
use hello_grid::{FONT_PNG, FONT_RON, Hello, REQUIRED_GLYPHS};
use shuyi_cli::Project;
use shuyi_cli::codec::decode_png;
use shuyi_cli::font_atlas::{atlas_ron, build_atlas, read_font};
use shuyi_cli::glyph_tiles;

/// The example's folder, which holds its `shuyi.ron`.
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn project() -> Project {
    Project::load(root()).unwrap_or_else(|e| panic!("{e}"))
}

fn embedded(path: &str) -> &'static [u8] {
    hello_grid::file(path).unwrap_or_else(|| panic!("no {path}"))
}

#[test]
fn the_games_files_load() {
    let game = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    assert!(game.dialogue.get("intro").is_some());
    assert_eq!(game.stage_map.name, "Hello, Grid");
    assert_eq!(
        (game.stage_map.tiles.width(), game.stage_map.tiles.height()),
        (12, 10)
    );
    assert!(game.images.id("tiles.png").is_some());
}

#[test]
fn the_font_atlas_is_what_font_atlas_makes_of_the_bdf() {
    let config = project().config;
    let rules = config.font_atlas.unwrap();
    assert_eq!(rules.required_glyphs, REQUIRED_GLYPHS);
    let font = read_font(&root().join("files-src/ter-u16n.bdf")).unwrap();
    let atlas = build_atlas(&font);
    let (width, height, rgba) = decode_png(embedded(FONT_PNG)).unwrap();
    assert_eq!((width, height), (atlas.width, atlas.height));
    assert!(rgba == atlas.rgba, "files/font/atlas.png is stale");
    let ron = atlas_ron(
        &atlas.def,
        "ter-u16n.bdf",
        config.command.as_deref().unwrap_or("shuyi"),
        rules.readme.as_deref(),
    )
    .unwrap();
    let committed = std::str::from_utf8(embedded(FONT_RON)).unwrap();
    assert_eq!(committed, ron, "files/font/atlas.ron is stale");
    assert!(atlas.def.problems(REQUIRED_GLYPHS).is_empty());
}

#[test]
fn the_tileset_is_what_glyph_tiles_makes_of_the_codes_looks() {
    let rules = project().config.glyph_tiles.unwrap();
    assert_eq!((rules.tile, rules.columns), (TILE_PX, TILE_COLUMNS));
    assert_eq!(rules.out, "files/tiles.png");
    // The terrains first, with the glyph skin's glyphs and colors.
    for (i, &(glyphs, fg, bg)) in TERRAIN_LOOKS.iter().enumerate() {
        let tile = &rules.tiles[i];
        assert_eq!(tile.glyphs, glyphs.iter().collect::<String>(), "tile {i}");
        assert_eq!((tile.fg.as_str(), tile.bg.as_deref()), (fg, Some(bg)));
    }
    // Then the pieces, each on a clear tile in its color.
    for side in [Side::Hero, Side::Foe] {
        let (letter, color, index) = piece_look(side);
        let tile = &rules.tiles[index];
        assert_eq!(tile.glyphs, letter.to_string(), "{side:?}");
        assert_eq!((tile.fg.as_str(), tile.bg.as_deref()), (color, None));
    }
    assert_eq!((HERO_TILE, FOE_TILE), (3, 4));
    assert_eq!(rules.tiles.len(), 5);
    let drawn = glyph_tiles::render_files(&rules, root()).unwrap();
    let (width, height, rgba) = decode_png(embedded("tiles.png")).unwrap();
    assert_eq!((width, height), (drawn.width, drawn.height));
    assert!(rgba == drawn.rgba, "files/tiles.png is stale");
}

#[test]
fn the_web_build_is_the_games_binary() {
    let web = project().config.web.unwrap();
    assert_eq!(
        (web.package.as_str(), web.bin.as_str()),
        ("hello-grid", "hello-grid")
    );
    assert_eq!(web.dist, "dist/web");
}
