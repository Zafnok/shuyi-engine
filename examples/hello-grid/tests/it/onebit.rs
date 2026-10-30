//! ADR-0008 §4 on Hello, Grid: its look on a black and white screen.
//! `files/palette@onebit.ron` gives every role of its palette patterns
//! and keeps apart what its colours tell apart, `frame-png --target
//! playdate` paints its title as the Playdate shows it, checked as a
//! snapshot (the `.png` beside the `.snap` is the picture), and `bake
//! --target playdate` makes every picture one-bit, its tileset from
//! `files@playdate/`, in a pack the game plays from on the headless
//! backend.

use std::fs;
use std::path::PathBuf;

use hello_grid::screens::StageScreen;
use hello_grid::tools::{FONT_1BIT, HelloTools, one_bit_skin};
use hello_grid::{Hello, LAYOUT, MapLook, PACK_STAMP};
use shuyi::assets::bitmap::Bitmap;
use shuyi::assets::bundle::Bundle;
use shuyi::assets::onebit::Pattern;
use shuyi::assets::pack::Pack;
use shuyi::platform::harness::Harness;
use shuyi::ui::frame::onebit::{Font, Images, PatternMap, to_one_bit};
use shuyi::ui::{Ctx, Game, GameHooks};
use shuyi_cli::contract::bake::{BakeInput, bake};
use shuyi_cli::contract::target::{Level, engine_profiles, one_bit};
use shuyi_cli::frame_png::{self, Script};
use shuyi_cli::project::Config;
use shuyi_cli::{GameTools, Project};

#[test]
fn every_role_has_patterns_and_the_pieces_and_the_ground_stay_apart() {
    let skin = one_bit_skin().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(skin.palette_file, "files/palette.ron");
    assert_eq!(skin.map_file, "files/palette@onebit.ron");
    assert_eq!(one_bit(&skin), [], "no error, no warning");
    let offered = HelloTools.one_bit("playdate");
    assert_eq!(offered, Some(Ok(skin.clone())));
    // The foe drawn white like your piece: the check names both.
    let mut alike = skin;
    let foe = alike.map.roles.get_mut("foe").unwrap_or_else(|| panic!());
    foe.fg = Pattern::WHITE;
    let threat = alike
        .map
        .roles
        .get_mut("threat")
        .unwrap_or_else(|| panic!());
    threat.fg = Pattern::WHITE;
    let problems = one_bit(&alike);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].level, Level::Error);
    assert_eq!(problems[0].roles, ["hero", "foe"]);
}

/// A fresh empty folder under the system's temp folder.
fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hello-grid-onebit-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    dir
}

#[test]
fn frame_png_on_the_playdate_paints_the_title_in_black_and_white() {
    let dir = temp_dir();
    let options = frame_png::parse_args(
        &[
            "title.png".to_owned(),
            "--target".to_owned(),
            "playdate".to_owned(),
        ],
        HelloTools.frame_options(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let profiles = engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
    let said = frame_png::run(
        &dir,
        &options,
        &HelloTools,
        Some(&profiles["playdate"].profile),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    // The Playdate's whole screen, though the frame is taller: the
    // README says why it doesn't fit.
    assert_eq!(said, "frame-png: title → title.png (400×240)");
    let png = fs::read(dir.join("title.png")).unwrap_or_else(|e| panic!("{e}"));
    let _ = fs::remove_dir_all(&dir);
    insta::assert_binary_snapshot!("title_playdate.png", png);
    // Twice the size with a scale of 2.
    let big = HelloTools
        .frame_one_bit("playdate", &[], &Script::default(), Some(2))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((big.image.width, big.image.height), (800, 480));
    assert_eq!(big.shows, "title");
}

#[test]
fn the_playdate_pack_holds_every_picture_one_bit_and_the_game_plays_from_it() {
    // Its own folder: the other test here empties `temp_dir`'s.
    let dir = std::env::temp_dir().join(format!("hello-grid-bake-onebit-{}", std::process::id()));
    let project = Project::new(dir.clone(), Config::default());
    let mut input = BakeInput::default();
    input.target = Some("playdate".to_owned());
    input.sheet = Some("sheet.png".to_owned());
    let answer = bake(&HelloTools, &project, &input);
    assert!(answer.ok, "{answer:?}");
    let made = answer
        .result
        .unwrap_or_else(|| panic!())
        .pack
        .unwrap_or_else(|| panic!());
    // The font's picture is dithered; the tileset is the one drawn for the
    // Playdate.
    assert_eq!(made.dithered, ["files/font/atlas.png"]);
    assert_eq!(made.replaced, ["files@playdate/tiles.png"]);
    assert!(
        made.kinds.contains(&"one-bit picture".to_owned()),
        "{made:?}"
    );
    let sheet = fs::read(dir.join("sheet.png")).unwrap_or_else(|e| panic!("{e}"));
    assert!(sheet.starts_with(b"\x89PNG"));
    let bytes: &'static [u8] = fs::read(dir.join(&made.file))
        .unwrap_or_else(|e| panic!("{e}"))
        .leak();
    let _ = fs::remove_dir_all(&dir);

    // Every picture in it is a one-bit picture at its PNG's path, and the
    // pack is proven equal to its sources.
    let pack = Pack::open(bytes, PACK_STAMP).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(pack.info().target.as_deref(), Some("playdate"));
    for path in ["tiles.png", "font/atlas.png"] {
        let file = pack.get(path).unwrap_or_default();
        assert!(Bitmap::is_bitmap(file), "{path}");
    }
    let tiles = Bitmap::parse(pack.get("tiles.png").unwrap_or_default());
    assert_eq!(tiles.map(|t| t.source_size()), Ok((192, 24)));
    let game = HelloTools
        .bake(Some("playdate"))
        .unwrap_or_else(|e| panic!("{e}"));
    let pack = Bundle::new(vec![pack.into_layer("pack")]);
    let mut formats = game.formats;
    game.bake
        .register(&mut formats)
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(game.bake.check(&formats, &game.bundle, &pack), []);

    // The game loads from it and plays on the headless backend as the
    // Playdate, its stage in tiles.
    let hello = Hello::load_from(&pack, &formats).unwrap_or_else(|e| panic!("{e:?}"));
    let mut ctx = Ctx::new(hello).unwrap_or_else(|missing| panic!("{missing:?}"));
    ctx.use_layout(LAYOUT);
    ctx.preview_settings(|s| s.game.look = MapLook::Tiles);
    let stage = StageScreen::new(&mut ctx).briefed();
    let profiles = engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
    let mut h = Harness::new(Game::new(ctx, Box::new(stage), GameHooks::default()))
        .as_target(&profiles["playdate"].profile);
    h.wait(0.5);
    assert_eq!(h.screens(), ["stage"]);
    assert_eq!(h.ctx().platform.target, "playdate");

    // Its sprites are drawn from the pack's one-bit pictures, where with
    // none they are grey rectangles.
    let tiles = h.ctx().game.tiles();
    assert!(pack.image(tiles).is_some());
    let skin = one_bit_skin().unwrap_or_else(|e| panic!("{e}"));
    let map = PatternMap::new(h.ctx().palette.iter(), &skin.map);
    let bits = pack.bytes(FONT_1BIT).unwrap_or_default();
    let font = Font::new(&h.ctx().game.font, bits).unwrap_or_else(|e| panic!("{e}"));
    let drawn = to_one_bit(h.game().buffer(), &map, &font, &pack);
    let grey = to_one_bit(h.game().buffer(), &map, &font, &());
    assert_ne!(drawn.bytes(), grey.bytes());
}
