use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use shuyi_assets::corners::{Rim, RimSide, Wobble, generate, layer, sheet_layer};
use shuyi_assets::image::{ImageInfo, ImageTable};
use shuyi_assets::tileset::{Layer, MIXES};
use shuyi_core::Pos;
use shuyi_ui::frame::raster::Painter;
use shuyi_ui::frame::{Cell, GlyphBuffer, Rect, Rgb};
use shuyi_ui::grid::{GridSkin, MapScene, NoUnits};
use shuyi_ui::toy::grid::{
    BRIDGE, GRASS, Kit, Piece, TILES_SIZE, WATER, layered, sprite, tile_at, tiles,
};

use super::*;
use crate::codec::decode_png;
use crate::font_atlas::{AtlasChars, build_atlas, read_font};

/// A new empty folder of its own for test `name`.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-corners-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A `w × h` picture whose every pixel is opaque and its own.
fn pattern(w: u32, h: u32) -> Image {
    let byte = |v: u32| u8::try_from(v % 256).unwrap();
    let rgba = (0..h)
        .flat_map(|y| (0..w).flat_map(move |x| [byte(x * 9), byte(y * 5), byte(x + y), 255]))
        .collect();
    Image {
        width: w,
        height: h,
        rgba,
    }
}

const RECIPE: &str = r##"(
    make: Generated(
        tile: (image: "tiles/terrain.png", at: (24, 0), size: (24, 24)),
        wobble: Some((strength: 30, cells: 4, seed: 7)),
        rim: Some((colour: "#E8F4FF", width: 2, side: Outside)),
    ),
    out: "tiles/water-corners.png",
)
"##;

/// A game's folder with a recipe at `art/water.corners.ron` and the
/// picture it names, and the recipe's path.
fn game(name: &str, recipe: &str) -> (PathBuf, String) {
    let dir = temp_dir(name);
    fs::create_dir_all(dir.join("art/tiles")).unwrap();
    let terrain = pattern(48, 24);
    let png = encode_png(terrain.width, terrain.height, &terrain.rgba).unwrap();
    fs::write(dir.join("art/tiles/terrain.png"), png).unwrap();
    let file = dir.join("art/water.corners.ron");
    fs::write(&file, recipe).unwrap();
    (dir, file.to_string_lossy().into_owned())
}

#[test]
fn the_command_writes_a_sheet_that_check_then_passes() {
    let (dir, file) = game("writes", RECIPE);
    assert_eq!(main("shuyi", std::slice::from_ref(&file)), 0);
    let out = dir.join("art/tiles/water-corners.png");
    let (w, h, rgba) = decode_png(&fs::read(&out).unwrap()).unwrap();
    let recipe = recipe::parse(&file, RECIPE).unwrap();
    let want = make(&recipe, beside(&file)).unwrap();
    assert_eq!((w, h, rgba), (192, 48, want.rgba));
    assert_eq!(run_check(&file), Ok(()));
    assert_eq!(main("shuyi", &["--check".to_owned(), file.clone()]), 0);
    // `check` as a game's test calls it, reading from the recipe's folder.
    let folder = dir.join("art");
    let read = |path: &str| fs::read(folder.join(path)).map_err(|e| e.to_string());
    assert_eq!(check("art/water.corners.ron", RECIPE, read), Ok(()));
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_stale_or_missing_sheet_fails_the_check_and_says_how_to_remake_it() {
    let (dir, file) = game("stale", RECIPE);
    assert_eq!(main("shuyi", std::slice::from_ref(&file)), 0);
    fs::write(&file, RECIPE.replace("seed: 7", "seed: 8")).unwrap();
    let remake = format!("remake it: cargo shuyi corners {file}");
    let e = run_check(&file).unwrap_err();
    let stale = "tiles/water-corners.png is stale: its pixels differ";
    assert!(e.starts_with(stale) && e.ends_with(&remake), "{e}");
    assert_eq!(main("shuyi", &["--check".to_owned(), file.clone()]), 1);
    // Remade, it passes again.
    assert_eq!(main("shuyi", std::slice::from_ref(&file)), 0);
    assert_eq!(run_check(&file), Ok(()));
    // Another size.
    fs::write(&file, RECIPE.replace("size: (24, 24)", "size: (24, 12)")).unwrap();
    let e = run_check(&file).unwrap_err();
    let sized = e.contains("it is 192×48 pixels, and") && e.contains("makes 192×24");
    assert!(sized, "{e}");
    fs::remove_file(dir.join("art/tiles/water-corners.png")).unwrap();
    let e = run_check(&file).unwrap_err();
    let missing = e.starts_with("tiles/water-corners.png: ") && e.ends_with(&remake);
    assert!(missing, "{e}");
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_bad_recipe_fails_with_its_file_and_line() {
    let (dir, file) = game("bad", &RECIPE.replace("#E8F4FF", "white"));
    for e in [write(&file).unwrap_err(), run_check(&file).unwrap_err()] {
        let colour = e.starts_with(&format!("{file}:5:")) && e.contains("#RRGGBB");
        assert!(colour, "{e}");
    }
    assert_eq!(main("shuyi", std::slice::from_ref(&file)), 1);
    fs::write(&file, RECIPE.replace("at: (24, 0)", "at: (40, 0)")).unwrap();
    let e = write(&file).unwrap_err();
    assert!(e.starts_with(&format!("{file}:3: the tile,")), "{e}");
    assert_eq!(main("shuyi", &[]), 2);
    assert_eq!(main("shuyi", &["--force".to_owned()]), 2);
    let gone = dir.join("gone.corners.ron").to_string_lossy().into_owned();
    assert_eq!(main("shuyi", &[gone]), 1);
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_layer_of_a_sheet_is_laid_out_as_the_toys_shore() {
    let kit = Kit::new();
    let shore = layered(kit.tiles_image());
    let Some(Layer::Corners(drawn)) = shore.look.layers.first() else {
        panic!("{:?}", shore.look.layers);
    };
    let made = layer(BTreeSet::from([WATER, BRIDGE]), (0, 96), (24, 24));
    assert_eq!(made.tiles[1..MIXES], drawn.tiles[1..MIXES]);
    assert_eq!(made, *drawn);
}

/// The committed picture of the look snapshot.
const SHORE: &str = "tests/fixtures/corners/shore.png";

/// The made sheet's path in the snapshot's image table.
const SHEET: &str = "tiles/water-corners.png";

/// The toy's tiles picture in its size, grass and water tiles of their
/// own on row 0; and the shore made from the water's tile, a sheet of
/// its own.
fn pictures() -> (Image, Image) {
    let mut picture = Image::filled(TILES_SIZE.width, TILES_SIZE.height, [0; 4]);
    let paint = |picture: &mut Image, x0: u32, y0: u32, from: &Image| {
        for y in 0..from.height {
            for x in 0..from.width {
                let i = (((y0 + y) * picture.width + x0 + x) * 4) as usize;
                picture.rgba[i..i + 4].copy_from_slice(&from.pixel(x, y).unwrap());
            }
        }
    };
    let tile = |on: [u8; 4], off: [u8; 4], dotted: fn(u32, u32) -> bool| {
        let pixel = move |x, y| if dotted(x, y) { on } else { off };
        let rgba = (0..24).flat_map(|y| (0..24).flat_map(move |x| pixel(x, y)));
        Image {
            width: 24,
            height: 24,
            rgba: rgba.collect(),
        }
    };
    let grass = tile([70, 140, 60, 255], [90, 170, 70, 255], |x, y| {
        (x * 3 + y * 5) % 7 == 0
    });
    let water = tile([70, 120, 210, 255], [40, 80, 170, 255], |x, y| {
        (x + 2 * y) % 8 < 2
    });
    let at = |id: shuyi_core::TerrainId| tile_at(u32::from(id.0), 0);
    paint(&mut picture, at(GRASS).x, 0, &grass);
    paint(&mut picture, at(WATER).x, 0, &water);
    let wobble = Wobble::new(3).with_strength(40);
    let rim = Rim::new([232, 244, 255], RimSide::Outside).with_width(2);
    let shore = generate(&water, Some(wobble), Some(rim)).unwrap();
    (picture, shore)
}

/// A 4 × 3 map with a lake and a bay, its shore made from the water's
/// tile, painted by the grid kit's sprite skin and the raster: the same
/// as its committed picture. The water tiles' own picture is grass, so
/// the layer draws all the water, rounded. The layer names the made
/// sheet as an image of its own: nothing is pasted into the tiles'. When it changes on purpose,
/// the test writes the new picture beside the old (`shore.new.png`): look
/// at it, then move it over the old one.
#[test]
fn a_made_shore_is_painted_as_its_committed_picture() {
    let kit = Kit::new();
    let mut terrain = tiles(kit.tiles_image());
    terrain
        .look
        .tiles
        .insert(WATER, tile_at(u32::from(GRASS.0), 0));
    let sheets = ImageTable::new([(SHEET, ImageInfo::new(192, 48))].into());
    let sheet = sheets.id(SHEET).unwrap();
    terrain.look.layers = vec![Layer::Corners(sheet_layer(
        BTreeSet::from([WATER]),
        sheet,
        (24, 24),
    ))];
    let skin = sprite(Some(terrain));
    let rows = ["~~..", "~~~.", ".~.."];
    let mut scene = MapScene::<Piece>::new(Pos::new(0, 0), (4, 3));
    scene.set_terrain(|pos| {
        let row = rows[usize::try_from(pos.y.clamp(0, 2)).ok()?];
        let c = row.as_bytes()[usize::try_from(pos.x.clamp(0, 3)).ok()?];
        Some(if c == b'~' { WATER } else { GRASS })
    });
    // 12 × 5 cells of 8 × 16: 96 × 80 pixels, the 4 × 3 tiles from (0, 4).
    let clear = Rgb::new(0, 0, 0);
    let mut buf = GlyphBuffer::new(12, 5, Cell::new(' ', clear, clear));
    skin.paint(
        &kit.looks(),
        &scene,
        Rect::new(0, 0, 12, 5),
        &NoUnits,
        &mut buf,
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let font =
        read_font(&root.join("../../templates/shuyi-default/base/files-src/ter-u16n.bdf")).unwrap();
    let atlas = build_atlas(&font, &AtlasChars::default());
    let (tiles, shore) = pictures();
    let images = BTreeMap::from([(kit.tiles_image(), tiles), (sheet, shore)]);
    let image = Painter::new(atlas.def, atlas.image, images, clear).render(&buf, 2);
    assert_eq!((image.width, image.height), (192, 160));
    let path = root.join(SHORE);
    let matches = fs::read(&path)
        .ok()
        .and_then(|bytes| decode_png(&bytes).ok())
        .is_some_and(|(w, h, rgba)| (w, h, &rgba) == (image.width, image.height, &image.rgba));
    if !matches {
        let new = path.with_extension("new.png");
        fs::create_dir_all(new.parent().unwrap()).unwrap();
        fs::write(
            &new,
            encode_png(image.width, image.height, &image.rgba).unwrap(),
        )
        .unwrap();
        panic!(
            "the shore isn't its picture {SHORE}: the new picture is {}",
            new.display()
        );
    }
}
