use std::collections::BTreeMap;

use shuyi_assets::font::FALLBACK_GLYPH;
use shuyi_assets::image::ImageInfo;
use shuyi_ui::frame::{Cell, Layer, Overlay, Rect, Sprite};

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
        wide: BTreeMap::new(),
        wide_sets: BTreeMap::new(),
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
            script: Script::new(vec![
                Step::Keys("F2 Up".into()),
                Step::Wait(0.5),
                Step::Pad("South East+DpadDown".into()),
                Step::Keys("f".into()),
            ]),
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
    let plain = super::usage("shuyi", &[], "");
    assert!(
        plain.ends_with("whatever the player's keymap binds to it."),
        "{plain}"
    );
    assert!(
        plain.contains("  --act Confirm      press the game's action"),
        "{plain}"
    );
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
    let wait = |seconds| Script::new(vec![Step::Wait(seconds)]);
    assert!(check(&wait(0.0)).is_ok());
    assert!(check(&wait(MAX_WAIT)).is_ok());
    for bad in [-0.5, f32::NAN, f32::INFINITY] {
        let error = check(&wait(bad)).unwrap_err();
        assert!(error.starts_with("step 1: wait: "), "{error}");
        assert!(
            error.ends_with("is not a number of seconds, 0 or more"),
            "{error}"
        );
    }
    // The Harness's longest wait: its 2000 frames.
    let error = check(&wait(MAX_WAIT + 0.1)).unwrap_err();
    assert!(
        error.starts_with("step 1: wait: at most 33.3 seconds"),
        "{error}"
    );
    let script = Script::new(vec![Step::Wait(1.0), Step::Keys("Nope".into())]);
    assert!(check(&script).unwrap_err().starts_with("step 2: keys: "));
    assert!(check(&Script::default()).is_ok());
}

#[test]
fn a_step_is_read_from_its_flag() {
    assert_eq!(
        parse_step("--keys", "Up"),
        Some(Ok(Step::Keys("Up".into())))
    );
    assert_eq!(
        parse_step("--script", "Up"),
        Some(Ok(Step::Keys("Up".into())))
    );
    assert_eq!(
        parse_step("--pad", "South"),
        Some(Ok(Step::Pad("South".into())))
    );
    assert_eq!(parse_step("--wait", "0.5"), Some(Ok(Step::Wait(0.5))));
    assert_eq!(
        parse_step("--act", "Confirm"),
        Some(Ok(Step::Act("Confirm".into())))
    );
    assert_eq!(
        parse_step("--act", ""),
        Some(Err("--act: no action named".into()))
    );
    assert_eq!(
        parse_step("--wait", "soon"),
        Some(Err("--wait: not a number of seconds: soon".into()))
    );
    let negative = parse_step("--wait", "-1").unwrap().unwrap_err();
    assert!(negative.starts_with("--wait: -1 is not"), "{negative}");
    // The longest wait is the Harness's, checked with the script.
    assert_eq!(parse_step("--wait", "100"), Some(Ok(Step::Wait(100.0))));
    let long = parse_args(&args(&["a.png", "--wait", "100"]), &[]).unwrap_err();
    assert!(long.starts_with("step 1: wait: at most 33.3"), "{long}");
    assert!(
        parse_step("--keys", "Nope")
            .unwrap()
            .unwrap_err()
            .starts_with("--keys: ")
    );
    assert_eq!(parse_step("--scale", "2"), None);
}

#[test]
fn a_crank_step_is_read_checked_and_kept_in_order() {
    for (text, degrees) in [("30", 30.0), ("-30", -30.0), ("0.5", 0.5)] {
        assert_eq!(parse_step("--crank", text), Some(Ok(Step::Crank(degrees))));
    }
    assert_eq!(
        parse_step("--crank", "far"),
        Some(Err("--crank: not a number of degrees: far".into()))
    );
    let nan = parse_step("--crank", "NaN").unwrap().unwrap_err();
    assert!(nan.starts_with("--crank: "), "{nan}");
    assert!(Step::Crank(f32::INFINITY).check().is_err());
    let parsed = parse_args(
        &args(&["a.png", "--pad", "South", "--crank", "-30", "--pad", "East"]),
        &[],
    )
    .unwrap();
    assert_eq!(
        parsed.script.steps,
        vec![
            Step::Pad("South".into()),
            Step::Crank(-30.0),
            Step::Pad("East".into())
        ]
    );
    assert!(parse_args(&args(&["a.png", "--crank", "far"]), &[]).is_err());
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
    assert_eq!(Script::new(steps.clone()).steps, steps);
}

/// The committed reference picture of a line of wide and narrow glyphs.
const WIDE_REFERENCE: &str = "tests/fixtures/fonts/wide-line.png";

/// A line of narrow glyphs from the template's font and wide ones from our
/// fixture font, in two colours and on a coloured cell, drawn by the
/// raster as `frame-png` draws it: the same as its committed reference.
/// When it changes on purpose, the test writes the new picture beside the
/// old (`wide-line.new.png`): look at it, then move it over the old one.
#[test]
fn a_line_of_wide_and_narrow_glyphs_matches_its_reference() {
    use crate::font_atlas::{AtlasChars, build_wide_atlas, read_font};
    use shuyi_assets::font::CellWidths;
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let narrow =
        read_font(&root.join("../../templates/shuyi-default/base/files-src/ter-u16n.bdf")).unwrap();
    let wide = read_font(&root.join("tests/fixtures/fonts/wide-16.bdf")).unwrap();
    let chars = AtlasChars::default().with_wide("口田日一");
    let atlas = build_wide_atlas(&narrow, Some(&wide), &chars);
    let png = encode_png(atlas.width, atlas.height, &atlas.rgba).unwrap();
    let widths = CellWidths::of(&atlas.def);
    let table = ImageTable::new(BTreeMap::new());
    let painter = Painter::new(atlas.def, &png, &table, |_| None, CLEAR).unwrap();
    let mut buf = GlyphBuffer::new(14, 1, Cell::new(' ', CLEAR, CLEAR)).with_widths(widths);
    let (gold, white) = (Rgb::new(250, 200, 60), Rgb::new(255, 255, 255));
    let x = buf.print(0, 0, "A口b田", white, CLEAR);
    let x = i32::from(x) + i32::from(buf.print(i32::from(x), 0, "日", gold, Rgb::new(40, 60, 120)));
    buf.print(x, 0, "1一!", white, CLEAR);
    let image = painter.render(&buf, 2);
    assert_eq!((image.width, image.height), (14 * 8 * 2, 16 * 2));
    assert_matches_reference(&image, WIDE_REFERENCE);
}

/// Panics unless `image` is the committed picture at `reference` (from the
/// crate's folder), writing the new picture beside it
/// (`<name>.new.png`) to look at and move over the old one when the
/// change is meant.
fn assert_matches_reference(image: &Image, reference: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(reference);
    let matches = fs::read(&path)
        .ok()
        .and_then(|bytes| decode_png(&bytes).ok())
        .is_some_and(|(w, h, rgba)| {
            (w, h, rgba) == (image.width, image.height, image.rgba.clone())
        });
    if !matches {
        let new = path.with_extension("new.png");
        fs::write(
            &new,
            encode_png(image.width, image.height, &image.rgba).unwrap(),
        )
        .unwrap();
        panic!(
            "the picture isn't its reference {reference}: the new picture is {}",
            new.display()
        );
    }
}

/// The committed pictures of one line drawn in Japanese and in Chinese.
const SETS_REFERENCES: [&str; 2] = [
    "tests/fixtures/fonts/wide-set-ja.png",
    "tests/fixtures/fonts/wide-set-zh.png",
];

/// An atlas with two wide sets from our fixture fonts, `ja` and
/// `zh-Hans`, each with its own `直`, made by `font-atlas` for a Japanese
/// pack and a Chinese one with its punctuation wide: `直` drawn in each
/// language is the shape of its set, and the Chinese line's quotes and
/// ellipsis are wide, each picture the same as its committed reference.
#[test]
fn a_character_draws_from_the_set_its_language_names() {
    use crate::font_atlas::{AtlasChars, build_atlas_with_sets, read_font};
    use shuyi_assets::font::CellWidths;
    use shuyi_assets::lang::{Lang, LangCode, LangFile, PackRules, from_files};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let narrow =
        read_font(&root.join("../../templates/shuyi-default/base/files-src/ter-u16n.bdf")).unwrap();
    let fixture = |name: &str| read_font(&root.join("tests/fixtures/fonts").join(name)).unwrap();
    let (ja, zh) = (fixture("wide-16-ja.bdf"), fixture("wide-16-zh.bdf"));
    let (japanese, chinese) = ("直日", "“直简”……");
    let ui = |text: &str| format!(r#"[(key: "a.line", source: "Line", text: "{text}")]"#);
    let (ja_ui, zh_ui) = (ui(japanese), ui(chinese));
    let files = [
        ("en/ui.ron", r#"{"a.line": "Line"}"#),
        (
            "ja/lang.ron",
            r#"(name: "日", made_by: Machine, wide_font: Some("ja"))"#,
        ),
        ("ja/ui.ron", ja_ui.as_str()),
        (
            "zh-Hans/lang.ron",
            r#"(name: "简", made_by: Machine, wide_font: Some("zh-Hans"), wide_punctuation: true)"#,
        ),
        ("zh-Hans/ui.ron", zh_ui.as_str()),
    ]
    .map(|(path, source)| LangFile {
        path,
        source: Some(source),
    });
    let rules = PackRules {
        data_problem: |_, _| None,
        tokens: &shuyi_assets::dialogue::NoTokens,
        limits: shuyi_assets::dialogue::DialogueLimits {
            box_width: 40,
            box_lines: 3,
            max_text: 200,
            max_option: 30,
            min_options: 2,
            max_options: 4,
            max_reaction_texts: 3,
        },
        widths: &CellWidths::NARROW,
        variants: &[],
    };
    let lang: Lang = from_files(&files, None, str::to_owned, &rules).unwrap();
    let chars = AtlasChars::of_lang(&lang, "?");
    let atlas = build_atlas_with_sets(
        &narrow,
        &[(Some("ja"), &ja), (Some("zh-Hans"), &zh)],
        &chars,
    );
    assert_eq!(shuyi_assets::font::undrawn(&atlas.def, &lang), Vec::new());
    let png = encode_png(atlas.width, atlas.height, &atlas.rgba).unwrap();
    let base = CellWidths::of(&atlas.def);
    let table = ImageTable::new(BTreeMap::new());
    let painter = Painter::new(atlas.def, &png, &table, |_| None, CLEAR).unwrap();
    let white = Rgb::new(255, 255, 255);
    let mut images = Vec::new();
    for (code, text) in [("ja", japanese), ("zh-Hans", chinese)] {
        let info = &lang.pack(&LangCode::new(code).unwrap()).unwrap().info;
        let mut buf =
            GlyphBuffer::new(14, 1, Cell::new(' ', CLEAR, CLEAR)).with_widths(info.widths(&base));
        let x = buf.print(0, 0, text, white, CLEAR);
        buf.print(i32::from(x), 0, "ok", white, CLEAR);
        images.push(painter.render(&buf, 2));
    }
    // 直 starts the Japanese line, in cells 0 and 1, and follows a wide `“`
    // in the Chinese one, in cells 2 and 3: its two shapes differ.
    let glyph = |image: &Image, cell: u32| -> Vec<Option<[u8; 4]>> {
        let (from, to) = (cell * 8 * 2, (cell + 2) * 8 * 2);
        (0..image.height)
            .flat_map(|y| (from..to).map(move |x| image.pixel(x, y)))
            .collect()
    };
    assert_ne!(glyph(&images[0], 0), glyph(&images[1], 2));
    for (image, reference) in images.iter().zip(SETS_REFERENCES) {
        assert_matches_reference(image, reference);
    }
}
