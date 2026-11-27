//! The game's files: they load, and the generated ones are what the
//! engine's tools make from their sources and `shuyi.ron` (the font atlas
//! from the BDF, the tileset from the font and the palette, the preview
//! and the book's shots from their `frame-png` arguments), so an edit that
//! forgets to regenerate
//! them fails here.

use std::borrow::Cow;
use std::path::Path;

use hello_grid::map::{FOE_TILE, HERO_TILE, TERRAIN_LOOKS, TILE_COLUMNS, TILE_PX, piece_look};
use hello_grid::rules::Side;
use hello_grid::tools::HelloTools;
use hello_grid::{FONT_PNG, FONT_RON, Hello, REQUIRED_GLYPHS};
use shuyi::prelude::GameData;
use shuyi_cli::font_atlas::{AtlasChars, atlas_ron, build_atlas, read_font};
use shuyi_cli::prelude::{Project, decode_png};
use shuyi_cli::{frame_png, glyph_tiles};

/// The example's folder, which holds its `shuyi.ron`.
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn project() -> Project {
    Project::load(root()).unwrap_or_else(|e| panic!("{e}"))
}

fn embedded(path: &str) -> Cow<'static, [u8]> {
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
    // `font-atlas` runs the game, and keeps the characters its text uses.
    let game = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    let chars = AtlasChars::of_lang(game.lang(), &rules.required_glyphs);
    let atlas = build_atlas(&font, &chars);
    let (width, height, rgba) = decode_png(&embedded(FONT_PNG)).unwrap();
    assert_eq!((width, height), (atlas.image.width, atlas.image.height));
    assert!(rgba == atlas.image.rgba, "files/font/atlas.png is stale");
    let ron = atlas_ron(
        &atlas.def,
        "ter-u16n.bdf",
        config.command.as_deref().unwrap_or("shuyi"),
        rules.readme.as_deref(),
    )
    .unwrap();
    let committed = embedded(FONT_RON);
    let committed = std::str::from_utf8(&committed).unwrap();
    assert_eq!(committed, ron, "files/font/atlas.ron is stale");
    let found = atlas.def.problems(REQUIRED_GLYPHS);
    assert!(found.is_empty(), "{found:?}");
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
    let (width, height, rgba) = decode_png(&embedded("tiles.png")).unwrap();
    assert_eq!((width, height), (drawn.width, drawn.height));
    assert!(rgba == drawn.rgba, "files/tiles.png is stale");
}

#[test]
fn the_preview_is_what_frame_png_makes_of_its_part() {
    frame_png::preview::check(&project(), &HelloTools).unwrap_or_else(|e| panic!("{e}"));
}

/// The book's pictures (`shots/`), each what its line of `shots` makes,
/// and no other file there.
#[test]
fn the_shots_are_what_frame_png_makes_of_their_part() {
    let project = project();
    assert_eq!(
        project.config.shots.keys().collect::<Vec<_>>(),
        [
            "dialogue", "leave", "options", "playdate", "stage", "tiles", "title"
        ]
    );
    frame_png::kept::check_shots(&project, &HelloTools).unwrap_or_else(|e| panic!("{e}"));
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

/// The data read a file a step is the data read at once, and a load
/// finished early says how many files it didn't read.
#[test]
fn the_data_reads_a_file_a_step_and_says_what_it_did_not_read() {
    use hello_grid::DataLoad;
    use shuyi::prelude::Bundle;

    let bundle = Bundle::new(vec![hello_grid::layer()]);
    let formats = hello_grid::formats();
    let mut data = DataLoad::default();
    let read: Vec<u32> = (0..DataLoad::FILES + 2)
        .map(|_| data.step(&bundle, &formats))
        .collect();
    assert_eq!(read, [1, 2, 3, 4, 5, 6, 7, 7, 7], "none past the last");
    let stepped = data.finish().unwrap_or_else(|e| panic!("{e:?}"));
    let at_once = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(stepped.stage_map, at_once.stage_map);
    assert_eq!(stepped.dialogue.get("intro"), at_once.dialogue.get("intro"));
    let unread = |steps: u32| {
        let mut data = DataLoad::default();
        for _ in 0..steps {
            data.step(&bundle, &formats);
        }
        let errors = data.finish().err().unwrap_or_default();
        errors.iter().map(ToString::to_string).collect::<Vec<_>>()
    };
    assert!(
        unread(0)
            .iter()
            .any(|e| e.contains("7 of its files not read")),
        "{:?}",
        unread(0)
    );
    assert!(
        unread(6)
            .iter()
            .any(|e| e.contains("1 of its files not read")),
        "{:?}",
        unread(6)
    );
}

/// Where a load takes more than a frame (a slice a step long, as on a slow
/// device), the card counts the game's files as they are read, after those
/// of its first frame, which have all arrived at once ([`hello_grid::FIRST`]).
#[test]
fn the_card_counts_the_games_files_where_its_load_takes_more_than_a_frame() {
    use hello_grid::DataLoad;
    use shuyi::platform::headless::HeadlessPlatform;
    use shuyi::platform::{App, LoadSlice, Millis};
    use shuyi::ui::perf::PerfClock;

    let slice = LoadSlice::new(0, PerfClock::fake());
    let mut platform = HeadlessPlatform::new().with_load_slice(slice);
    let mut app = App::new(hello_grid::start());
    let mut counts = Vec::new();
    for frame in 0..20 {
        let _ = app.frame(&mut platform, Millis(16 * frame));
        if !app.is_loading() {
            break;
        }
        counts.extend(platform.card_count());
    }
    let first = u32::try_from(hello_grid::FIRST.len()).unwrap();
    let of = first + DataLoad::FILES;
    let expected: Vec<(u32, u32)> = core::iter::once(0)
        .chain(first + 1..=of)
        .map(|done| (done, of))
        .collect();
    assert_eq!(counts, expected, "a frame a file, after the card's");
    assert_eq!(app.errors(), None);
    assert_eq!(
        app.game().map(shuyi::prelude::Game::screens),
        Some(vec!["title"])
    );
}
