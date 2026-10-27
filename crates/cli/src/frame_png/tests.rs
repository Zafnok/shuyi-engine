use std::collections::BTreeMap;

use shuyi_assets::font::FALLBACK_GLYPH;
use shuyi_assets::image::ImageInfo;
use shuyi_assets::lighting::Light;
use shuyi_ui::frame::onebit;
use shuyi_ui::frame::{Cell, Layer, Overlay, Paint, Rect, Sprite};

use super::*;
use crate::test_card;

/// The test card's path in the image table.
const TEST_CARD_PATH: &str = "images/test_card.png";

/// The console background in these tests.
const CLEAR: Rgb = Rgb::new(16, 16, 16);

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_string()).collect()
}

/// A 24×16 atlas of three 8×16 cells: ' ', then 'A' (a solid 4×4 block
/// in its top-left corner) and the fallback '?' (its left column).
fn tiny_atlas() -> (FontAtlasDef, Vec<u8>) {
    let def = FontAtlasDef {
        cell_w: 8,
        cell_h: 16,
        columns: 3,
        glyphs: BTreeMap::from([(' ', 0), ('A', 1), (FALLBACK_GLYPH, 2)]),
    };
    let mut rgba = vec![0; 24 * 16 * 4];
    let mut set = |x: usize, y: usize| rgba[(y * 24 + x) * 4..][..4].copy_from_slice(&[255; 4]);
    for y in 0..4 {
        for x in 8..12 {
            set(x, y);
        }
    }
    for y in 0..16 {
        set(16, y);
    }
    (def, encode_png(24, 16, &rgba).unwrap())
}

/// The test card as a PNG file.
fn card_png() -> Vec<u8> {
    let size = test_card::SIZE;
    encode_png(size, size, &test_card::pixels()).unwrap()
}

/// A painter with the tiny atlas and the test card, if `with_card`.
fn painter(with_card: bool) -> (Painter, ImageTable) {
    let (atlas, png) = tiny_atlas();
    let size = ImageInfo::new(test_card::SIZE, test_card::SIZE);
    let table = ImageTable::new([(TEST_CARD_PATH, size)].into());
    let card = card_png();
    let read = |path: &str| (with_card && path == TEST_CARD_PATH).then_some(card.as_slice());
    (
        Painter::new(atlas, &png, &table, read, CLEAR).unwrap(),
        table,
    )
}

/// 3×2 cells: an 'A' on a blue cell, a white one on a green cell below
/// right, a glyph the atlas lacks, a green
/// `Under` line through the 'A', a red `Over` dot on it, a white `Over`
/// square, and the test card, flipped, half-opaque and clipped to its
/// middle-top 8×8, over the bottom row.
fn hand_checked_frame(table: &ImageTable) -> GlyphBuffer {
    let mut buf = GlyphBuffer::new(3, 2, Cell::new(' ', CLEAR, CLEAR));
    buf.set(
        0,
        0,
        Cell::new('A', Rgb::new(200, 100, 50), Rgb::new(0, 0, 128)),
    );
    buf.set(1, 0, Cell::new('Z', Rgb::new(1, 2, 3), CLEAR));
    buf.set(
        2,
        1,
        Cell::new('A', Rgb::new(255, 255, 255), Rgb::new(0, 128, 0)),
    );
    let under = Overlay::new(Rect::new(0, 2, 8, 1), Rgb::new(0, 255, 0), Layer::Under);
    buf.add_overlay(under);
    buf.add_overlay(Overlay::new(
        Rect::new(3, 3, 1, 1),
        Rgb::new(255, 0, 0),
        Layer::Over,
    ));
    buf.add_overlay(Overlay::new(
        Rect::new(20, 0, 2, 2),
        Rgb::new(255, 255, 255),
        Layer::Over,
    ));
    let card = table.id(TEST_CARD_PATH).unwrap();
    let dest = Rect::new(0, 16, 16, 16);
    let mut sprite = Sprite::new(card, Rect::new(0, 0, 16, 16), dest, Layer::Over);
    sprite.clip = Rect::new(4, 16, 8, 8);
    sprite.flip_x = true;
    sprite.opacity = 128;
    buf.add_sprite(sprite);
    buf
}

/// Pixels of the hand-checked frame at scale 1, worked out by hand.
const EXPECTED: [((u32, u32), [u8; 3]); 17] = [
    // The 'A': its block in fg, the rest of the cell its bg.
    ((0, 0), [200, 100, 50]),
    ((2, 1), [200, 100, 50]),
    ((5, 5), [0, 0, 128]),
    ((16, 16), [255, 255, 255]),
    ((21, 20), [0, 128, 0]),
    // The `Under` line shows only beside the glyph.
    ((0, 2), [200, 100, 50]),
    ((5, 2), [0, 255, 0]),
    // The `Over` dot covers the glyph.
    ((3, 3), [255, 0, 0]),
    ((20, 1), [255, 255, 255]),
    // The missing 'Z' draws the fallback glyph in magenta.
    ((8, 7), [255, 0, 255]),
    ((9, 7), [16, 16, 16]),
    // The sprite, flipped: its left shows the card's right (green), its
    // right the card's left (red); 128/255 over the clear colour.
    ((4, 17), [34, 93, 42]),
    ((11, 17), [110, 34, 34]),
    ((6, 16), [136, 136, 136]),
    // Clipped: nothing beside or below the clip.
    ((3, 17), [16, 16, 16]),
    ((12, 17), [16, 16, 16]),
    ((6, 24), [16, 16, 16]),
];

#[test]
fn matches_a_hand_checked_frame() {
    let (painter, table) = painter(true);
    let buf = hand_checked_frame(&table);
    for scale in [1, 2] {
        let image = painter.render(&buf, scale);
        assert_eq!((image.width, image.height), (24 * scale, 32 * scale));
        for ((x, y), [r, g, b]) in EXPECTED {
            // Every pixel of the console pixel's square.
            for (dx, dy) in [(0, 0), (scale - 1, scale - 1)] {
                let at = (x * scale + dx, y * scale + dy);
                assert_eq!(
                    image.pixel(at.0, at.1),
                    Some([r, g, b, 255]),
                    "({x}, {y}) at scale {scale}"
                );
            }
        }
    }
}

#[test]
fn an_unflipped_solid_sprite_shows_the_card_as_is() {
    let (painter, table) = painter(true);
    let mut buf = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, CLEAR));
    let card = table.id(TEST_CARD_PATH).unwrap();
    let full = Rect::new(0, 0, 16, 16);
    buf.add_sprite(Sprite::new(card, full, full, Layer::Under));
    let image = painter.render(&buf, 1);
    for y in 0..16 {
        for x in 0..16 {
            let want = test_card::pixels()[((y * 16 + x) * 4) as usize..][..4].to_vec();
            assert_eq!(image.pixel(x, y).unwrap().to_vec(), want, "({x}, {y})");
        }
    }
}

#[test]
fn a_painted_sprite_is_one_colour_or_dimmed_where_the_image_is_solid() {
    let (painter, table) = painter(true);
    let card = table.id(TEST_CARD_PATH).unwrap();
    let full = Rect::new(0, 0, 16, 16);
    let render = |paint, opacity| {
        let mut buf = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, CLEAR));
        let mut sprite = Sprite::new(card, full, full, Layer::Over).painted(paint);
        sprite.opacity = opacity;
        buf.add_sprite(sprite);
        painter.render(&buf, 1)
    };
    // The card: a white border, then a red quadrant from (1, 1).
    let [red, ..] = test_card::QUADRANTS;
    let red = Rgb::new(red[0], red[1], red[2]);
    let white = Rgb::new(255, 255, 255);
    let rgba = |c: Rgb| Some([c.r, c.g, c.b, 255]);
    // One colour everywhere, whatever the image's.
    let blue = Rgb::new(10, 20, 200);
    let solid = render(Paint::Solid(blue), 255);
    assert_eq!(solid.pixel(0, 0), rgba(blue));
    assert_eq!(solid.pixel(1, 1), rgba(blue));
    assert_eq!(solid.pixel(14, 14), rgba(blue));
    // Half see-through: half way from the clear colour (16) to it.
    let half = render(Paint::Solid(blue), 128);
    assert_eq!(half.pixel(1, 1), Some([13, 18, 108, 255]));
    // Dimmed: each pixel grey and darker, by its own colour.
    let dimmed = render(Paint::Dimmed, 255);
    assert_eq!(dimmed.pixel(0, 0), rgba(white.dimmed()));
    assert_eq!(dimmed.pixel(1, 1), rgba(red.dimmed()));
    assert_ne!(red.dimmed(), red);
    // Shaded: each pixel as it is in the light (game ADR-0058).
    let dusk = Light::new(0.35, [0.62, 0.56, 0.46]);
    let shaded = render(Paint::Shaded(dusk), 255);
    assert_eq!(shaded.pixel(0, 0), Some([158, 143, 117, 255]));
    assert_eq!(shaded.pixel(1, 1), rgba(Paint::Shaded(dusk).apply(red)));
    assert_ne!(Paint::Shaded(dusk).apply(red), red);
    // The image's own colours, as before.
    let plain = render(Paint::Image, 255);
    assert_eq!(plain.pixel(1, 1), rgba(red));
}

/// A darkened sprite (game ADR-0075) is its colours times its brightness, after
/// its paint, and as solid as before: nothing under it shows through.
#[test]
fn a_darkened_sprite_is_darker_not_see_through() {
    let (painter, table) = painter(true);
    let card = table.id(TEST_CARD_PATH).unwrap();
    let full = Rect::new(0, 0, 16, 16);
    let render = |paint, brightness| {
        let mut buf = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, CLEAR));
        let mut sprite = Sprite::new(card, full, full, Layer::Over).painted(paint);
        sprite.brightness = brightness;
        buf.add_sprite(sprite);
        painter.render(&buf, 1)
    };
    let [red, ..] = test_card::QUADRANTS;
    let red = Rgb::new(red[0], red[1], red[2]);
    let rgba = |c: Rgb| Some([c.r, c.g, c.b, 255]);
    let dark = render(Paint::Image, 102);
    assert_eq!(dark.pixel(1, 1), rgba(red.scale(0.4)));
    assert_eq!(dark.pixel(0, 0), rgba(Rgb::new(102, 102, 102)));
    let blue = Rgb::new(10, 20, 200);
    let solid = render(Paint::Solid(blue), 102);
    assert_eq!(solid.pixel(1, 1), rgba(blue.scale(0.4)));
    // Black where it shows; full brightness as it is.
    assert_eq!(render(Paint::Image, 0).pixel(1, 1), rgba(Rgb::new(0, 0, 0)));
    assert_eq!(render(Paint::Image, 255).pixel(1, 1), rgba(red));
}

#[test]
fn a_missing_image_draws_magenta_where_the_sprite_shows() {
    let (painter, table) = painter(false);
    let image = painter.render(&hand_checked_frame(&table), 1);
    assert_eq!(image.pixel(4, 17), Some([255, 0, 255, 255]));
    assert_eq!(image.pixel(11, 23), Some([255, 0, 255, 255]));
    assert_eq!(image.pixel(12, 17), Some([16, 16, 16, 255]));
}

#[test]
fn a_bad_atlas_image_is_an_error() {
    let (atlas, _) = tiny_atlas();
    let table = ImageTable::default();
    let err = Painter::new(atlas, b"not a png", &table, |_| None, CLEAR).err();
    assert!(err.unwrap().starts_with("font atlas image"));
}

/// A game's options: one flag, one with a value.
const GAME: [GameOption; 2] = [
    GameOption {
        flag: "--map",
        value: None,
        help: "the map alone",
    },
    GameOption {
        flag: "--light",
        value: Some("NAME"),
        help: "fight in this light\nwhatever the battle says",
    },
];

#[test]
fn parses_steps_in_order_and_the_games_options() {
    let parsed = parse_args(
        &args(&[
            "out.png",
            "--keys",
            "F2 Up",
            "--wait",
            "0.5",
            "--pad",
            "South East+DpadDown",
            "--light",
            "dusk",
            "--keys",
            "f",
            "--scale",
            "3",
            "--map",
        ]),
        &GAME,
    )
    .unwrap();
    let game = |flag: &str, value: Option<&str>| GameArg {
        flag: flag.to_owned(),
        value: value.map(str::to_owned),
    };
    assert_eq!(
        parsed,
        Options {
            out: PathBuf::from("out.png"),
            script: Script {
                steps: vec![
                    Step::Keys("F2 Up".into()),
                    Step::Wait(0.5),
                    Step::Pad("South East+DpadDown".into()),
                    Step::Keys("f".into()),
                ],
            },
            scale: Some(3),
            target: None,
            game: vec![game("--light", Some("dusk")), game("--map", None)],
        }
    );
    let on = parse_args(&args(&["a.png", "--target", "playdate"]), &GAME).unwrap();
    assert_eq!(on.target.as_deref(), Some("playdate"));
    assert!(parse_args(&args(&["a.png", "--target"]), &GAME).is_err());
    let zero = parse_args(&args(&["a.png", "--wait", "0", "--scale", "8"]), &[]).unwrap();
    assert_eq!(
        (zero.script.steps, zero.scale),
        (vec![Step::Wait(0.0)], Some(8))
    );
    let plain = parse_args(&args(&["a.png"]), &GAME).unwrap();
    assert_eq!((plain.scale, plain.game), (None, Vec::new()));
    assert!(plain.script.steps.is_empty(), "{:?}", plain.script.steps);
}

#[test]
fn rejects_bad_arguments() {
    for bad in [
        &[][..],
        &["a.png", "b.png"],
        &["a.png", "--keys"],
        &["a.png", "--keys", "NoSuchKey"],
        &["a.png", "--pad", "Nope"],
        &["a.png", "--pad", "South+Nope"],
        &["a.png", "--wait", "-1"],
        &["a.png", "--wait", "soon"],
        &["a.png", "--scale", "0"],
        &["a.png", "--scale", "9"],
        &["a.png", "--light"],
        &["a.png", "--bogus"],
        &["--bogus"],
    ] {
        assert!(parse_args(&args(bad), &GAME).is_err(), "{bad:?}");
    }
    // An option of no game's is unknown.
    let error = parse_args(&args(&["a.png", "--map"]), &[]).unwrap_err();
    assert_eq!(error, "unknown option: --map");
}

#[test]
fn the_usage_lists_the_games_options_and_notes() {
    let usage = usage("tools", &GAME, "Pictures of bought art stay private.");
    assert!(usage.starts_with("usage: tools frame-png <out.png> [steps] [options]\n"));
    assert!(usage.contains("  --scale N            whole-number scale, 1 to 8"));
    assert!(
        usage.contains("\n  --map                the map alone\n"),
        "{usage}"
    );
    assert!(
        usage.contains(
            "\n  --light NAME         fight in this light\n                       \
             whatever the battle says\n"
        ),
        "{usage}"
    );
    assert!(usage.ends_with("\n\nPictures of bought art stay private."));
    assert!(super::usage("shuyi", &[], "").ends_with("doesn't apply to them."));
}

/// A game that shows a 2 × 1 picture and says what it was given.
struct Snapshot;

impl GameTools for Snapshot {
    fn frame_options(&self) -> &[GameOption] {
        &GAME
    }

    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        if options.iter().any(|o| o.flag == "--map") {
            return Err("--map: no battle on screen (title)".to_owned());
        }
        let width = scale.unwrap_or(2);
        Ok(Shot {
            image: Image::filled(width, 1, Rgb::new(1, 2, 3)),
            shows: format!("{} steps", script.steps.len()),
            warning: options.first().map(|o| format!("{o:?}")),
        })
    }

    fn lang(&self) -> Result<crate::GameLang, String> {
        Err("no languages".to_owned())
    }
}

/// A fresh empty directory under the system temp directory.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-frame-png-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn run_writes_the_games_picture_and_says_what_it_shows() {
    let dir = temp_dir("run");
    let options = parse_args(&args(&["out/a.png", "--keys", "f", "--scale", "3"]), &GAME).unwrap();
    let said = run(&dir, &options, &Snapshot, None).unwrap();
    assert_eq!(said, "frame-png: 1 steps → out/a.png (3×1)");
    let (w, h, rgba) = decode_png(&fs::read(dir.join("out/a.png")).unwrap()).unwrap();
    assert_eq!((w, h), (3, 1));
    assert_eq!(&rgba[..4], &[1, 2, 3, 255]);
    // The game's warning follows; its error is the command's.
    let options = parse_args(&args(&["b.png", "--light", "dusk"]), &GAME).unwrap();
    let said = run(&dir, &options, &Snapshot, None).unwrap();
    assert!(
        said.starts_with("frame-png: 0 steps → b.png (2×1)\nwarning: "),
        "{said}"
    );
    let options = parse_args(&args(&["c.png", "--map"]), &GAME).unwrap();
    let error = run(&dir, &options, &Snapshot, None).unwrap_err();
    assert_eq!(error, "--map: no battle on screen (title)");
    assert!(!dir.join("c.png").exists());
    // A folder that can't be made, or a file that can't be written.
    fs::write(dir.join("file"), "x").unwrap();
    let options = parse_args(&args(&["file/d.png"]), &GAME).unwrap();
    assert!(
        run(&dir, &options, &Snapshot, None)
            .unwrap_err()
            .starts_with("creating ")
    );
    fs::create_dir_all(dir.join("e.png")).unwrap();
    let options = parse_args(&args(&["e.png"]), &GAME).unwrap();
    assert!(
        run(&dir, &options, &Snapshot, None)
            .unwrap_err()
            .starts_with("writing ")
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_one_bit_target_is_painted_as_its_display_shows_it() {
    let dir = temp_dir("target");
    let profiles = crate::contract::target::engine_profiles().unwrap();
    let options = parse_args(&args(&["a.png", "--target", "playdate"]), &GAME).unwrap();
    let playdate = Some(&profiles["playdate"].profile);
    assert_eq!(
        run(&dir, &options, &Snapshot, playdate).unwrap_err(),
        "the game's tools don't offer frame-png --target on a one-bit target \
         (GameTools::frame_one_bit)"
    );
    assert!(!dir.join("a.png").exists());
    // A target in colour shows the game's own frame.
    let desktop = Some(&profiles["desktop"].profile);
    let said = run(&dir, &options, &Snapshot, desktop).unwrap();
    assert_eq!(said, "frame-png: 0 steps → a.png (2×1)");
    let usage = usage("shuyi", &GAME, "");
    assert!(usage.contains("  --target ID          as target ID's display shows it"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_summary_warns_about_bought_art() {
    let frame = Image {
        width: 2,
        height: 1,
        rgba: vec![0; 8],
    };
    assert_eq!(
        summary("title → a.png", &frame, None),
        "frame-png: title → a.png (2×1)"
    );
    assert_eq!(
        summary("title → a.png", &frame, Some("bought art")),
        "frame-png: title → a.png (2×1)\nwarning: bought art"
    );
}

#[test]
fn a_16_bit_picture_without_alpha_decodes_to_rgba8() {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, 1, 1);
        encoder.set_color(png::ColorType::Rgb);
        encoder.set_depth(png::BitDepth::Sixteen);
        let mut writer = encoder.write_header().unwrap();
        writer
            .write_image_data(&[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc])
            .unwrap();
    }
    assert_eq!(
        decode_png(&out).unwrap(),
        (1, 1, vec![0x12, 0x56, 0x9a, 255])
    );
}

const RED: Rgb = Rgb::new(200, 0, 0);
const BLUE: Rgb = Rgb::new(0, 0, 128);
const GREEN: Rgb = Rgb::new(0, 128, 0);
const WHITE: Rgb = Rgb::new(255, 255, 255);

/// A 6×3-cell console over a 2×1-cell scene (a red 'A' on blue, then a
/// green cell with a white `Over` dot at scene pixel (9, 1)). The scene
/// shows at 2× from scene pixel `origin` through the window of cells
/// (1, 1) to (4, 2): console pixels 8..40 × 16..48. In the window, cell
/// (2, 1) holds a white 'A' with no background and cell (4, 2) is solid,
/// in the clear colour.
fn backdrop_frame(origin: (f32, f32)) -> GlyphBuffer {
    let mut scene = GlyphBuffer::new(2, 1, Cell::new(' ', CLEAR, GREEN));
    scene.set(0, 0, Cell::new('A', RED, BLUE));
    scene.add_overlay(Overlay::new(Rect::new(9, 1, 1, 1), WHITE, Layer::Over));
    let mut buf = GlyphBuffer::new(6, 3, Cell::new(' ', CLEAR, CLEAR));
    let window = Rect::new(1, 1, 4, 2);
    buf.fill_rect(window, Cell::see_through(' ', CLEAR));
    buf.set_backdrop(std::rc::Rc::new(scene), window, origin, 2);
    buf.print_fg(2, 1, "A", WHITE);
    buf.set(4, 2, Cell::new(' ', CLEAR, CLEAR));
    buf
}

/// Pixels of [`backdrop_frame`] from origin (2, 1) at scale 1, worked out
/// by hand: scene pixel (sx, sy) is the 2×2 square at
/// (8 + (sx − 2) × 2, 16 + (sy − 1) × 2).
const BACKDROP_EXPECTED: [((u32, u32), Rgb); 16] = [
    // The scene's 'A' block (scene x and y under 4), then its cell's bg.
    ((8, 16), RED),
    ((11, 16), RED),
    ((12, 16), BLUE),
    ((10, 45), BLUE),
    // The console's own 'A' over the scene, and the scene beside it.
    ((16, 16), WHITE),
    ((19, 19), WHITE),
    ((20, 16), GREEN),
    // The scene's dot, scaled with it.
    ((22, 16), WHITE),
    ((23, 17), WHITE),
    ((22, 18), GREEN),
    // Clipped to the window: nothing left of or above it.
    ((7, 16), CLEAR),
    ((8, 15), CLEAR),
    // Past the scene's right and bottom edges: the clear colour.
    ((35, 16), GREEN),
    ((36, 16), CLEAR),
    ((10, 46), CLEAR),
    // The solid cell hides the scene, though it is in the clear colour.
    ((33, 33), CLEAR),
];

#[test]
fn a_backdrop_is_scaled_shifted_and_clipped_behind_the_see_through_cells() {
    let (painter, _) = painter(false);
    let buf = backdrop_frame((2.0, 1.0));
    for scale in [1, 2, 3] {
        let image = painter.render(&buf, scale);
        assert_eq!((image.width, image.height), (48 * scale, 48 * scale));
        for ((x, y), Rgb { r, g, b }) in BACKDROP_EXPECTED {
            for (dx, dy) in [(0, 0), (scale - 1, scale - 1)] {
                let at = (x * scale + dx, y * scale + dy);
                assert_eq!(
                    image.pixel(at.0, at.1),
                    Some([r, g, b, 255]),
                    "({x}, {y}) at scale {scale}"
                );
            }
        }
        // Beside the solid cell the scene still shows.
        let Rgb { r, g, b } = GREEN;
        assert_eq!(image.pixel(31 * scale, 33 * scale), Some([r, g, b, 255]));
    }
}

#[test]
fn half_a_scene_pixel_pans_by_one_window_pixel() {
    let (painter, _) = painter(false);
    let at = |origin, x| painter.render(&backdrop_frame(origin), 1).pixel(x, 16);
    let px = |Rgb { r, g, b }: Rgb| Some([r, g, b, 255]);
    // From (2, 1) the block ends before window pixel 12; from (2.5, 1),
    // one pixel sooner; from (3, 1), two.
    assert_eq!((at((2.0, 1.0), 10), at((2.0, 1.0), 11)), (px(RED), px(RED)));
    assert_eq!(
        (at((2.5, 1.0), 10), at((2.5, 1.0), 11)),
        (px(RED), px(BLUE))
    );
    assert_eq!((at((3.0, 1.0), 9), at((3.0, 1.0), 10)), (px(RED), px(BLUE)));
    // At 2 × 2 a quarter of a scene pixel is a window pixel.
    let at2 = |origin, x| painter.render(&backdrop_frame(origin), 2).pixel(x, 32);
    assert_eq!(
        (at2((2.0, 1.0), 23), at2((2.0, 1.0), 24)),
        (px(RED), px(BLUE))
    );
    assert_eq!(
        (at2((2.25, 1.0), 22), at2((2.25, 1.0), 23)),
        (px(RED), px(BLUE))
    );
    // A window that starts before the scene shows the clear colour first.
    assert_eq!(at((-1.0, 1.0), 9), px(CLEAR));
    assert_eq!(at((-1.0, 1.0), 10), px(RED));
}

#[test]
fn a_one_bit_frame_becomes_a_black_and_white_picture() {
    let mut frame = Box::new([0u8; onebit::FRAME_BYTES]);
    frame[0] = 0b0100_0000;
    let picture = one_bit_picture(&frame, 2);
    assert_eq!((picture.width, picture.height), (800, 480));
    assert_eq!(picture.pixel(0, 0), Some([0, 0, 0, 255]));
    assert_eq!(picture.pixel(2, 1), Some([255; 4]));
    assert_eq!(picture.pixel(3, 1), Some([255; 4]));
    assert_eq!(picture.pixel(4, 0), Some([0, 0, 0, 255]));
    assert_eq!(picture.pixel(2, 2), Some([0, 0, 0, 255]));
}

#[test]
fn a_step_the_harness_cant_play_is_refused() {
    assert!(Step::Keys("Enter Shift+Space".into()).check().is_ok());
    assert!(
        Step::Keys("Nope".into())
            .check()
            .unwrap_err()
            .starts_with("keys: ")
    );
    assert!(Step::Pad("South East+DpadDown".into()).check().is_ok());
    assert!(
        Step::Pad("South+Nope".into())
            .check()
            .unwrap_err()
            .starts_with("pad: ")
    );
    assert!(Step::Wait(0.0).check().is_ok());
    assert!(Step::Wait(MAX_WAIT).check().is_ok());
    for bad in [-0.5, MAX_WAIT + 0.1, f32::NAN, f32::INFINITY] {
        let error = Step::Wait(bad).check().unwrap_err();
        assert!(error.starts_with("wait: "), "{error}");
        assert!(error.ends_with("from 0 to 33.3"), "{error}");
    }
    let script = Script {
        steps: vec![Step::Wait(1.0), Step::Keys("Nope".into())],
    };
    assert!(script.check().is_err());
    assert!(Script::default().check().is_ok());
}

#[test]
fn a_step_is_read_from_its_flag() {
    assert_eq!(
        Script::parse_step("--keys", "Up"),
        Some(Ok(Step::Keys("Up".into())))
    );
    assert_eq!(
        Script::parse_step("--script", "Up"),
        Some(Ok(Step::Keys("Up".into())))
    );
    assert_eq!(
        Script::parse_step("--pad", "South"),
        Some(Ok(Step::Pad("South".into())))
    );
    assert_eq!(
        Script::parse_step("--wait", "0.5"),
        Some(Ok(Step::Wait(0.5)))
    );
    assert_eq!(
        Script::parse_step("--wait", "soon"),
        Some(Err("--wait: not a number of seconds: soon".into()))
    );
    let long = Script::parse_step("--wait", "100").unwrap().unwrap_err();
    assert!(long.starts_with("--wait: 100 is not"), "{long}");
    assert!(
        Script::parse_step("--keys", "Nope")
            .unwrap()
            .unwrap_err()
            .starts_with("--keys: ")
    );
    assert_eq!(Script::parse_step("--scale", "2"), None);
}

#[test]
fn a_new_shot_has_no_warning_and_a_new_script_its_steps() {
    let image = Image {
        width: 1,
        height: 1,
        rgba: vec![0, 0, 0, 255],
    };
    let literal = Shot {
        image: image.clone(),
        shows: "title".to_owned(),
        warning: None,
    };
    assert_eq!(Shot::new(image, "title"), literal);
    let steps = vec![Step::Keys("Enter".to_owned()), Step::Wait(0.5)];
    assert_eq!(Script::new(steps.clone()), Script { steps });
}
