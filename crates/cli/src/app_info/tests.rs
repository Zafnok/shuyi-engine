//! The `app` part's rules, the Rust `app-info` writes, and `--check`.

use std::path::{Path, PathBuf};

use super::*;

/// The part a new game starts with.
fn moon() -> AppPart {
    AppPart::new("Moon Garden", "io.github.someone.moon-garden")
}

/// The settings of `part`'s problems, each with its level.
fn found(part: &AppPart) -> Vec<(Level, String)> {
    fields(part)
        .into_iter()
        .map(|p| (p.level, p.setting))
        .collect()
}

fn error(setting: &str) -> Vec<(Level, String)> {
    vec![(Level::Error, setting.to_owned())]
}

/// A folder of its own under the system's temporary folder, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-app-info-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_good_part_has_no_problem() {
    assert_eq!(found(&moon()), []);
    let mut full = moon();
    full.version = Some("1.2.0-beta.1+42".to_owned());
    full.build = 3;
    full.storage = Some("Moon Garden_2.x".to_owned());
    assert_eq!(found(&full), []);
}

#[test]
fn the_name_is_not_empty_and_holds_no_control_character() {
    for name in ["", "  ", "Moon\nGarden", "Moon\tGarden"] {
        let mut part = moon();
        part.name = name.to_owned();
        assert_eq!(found(&part), error("app.name"), "{name:?}");
    }
    let mut part = moon();
    part.name = "Lune Café \"2\"".to_owned();
    assert_eq!(found(&part), []);
}

#[test]
fn the_id_is_reverse_dns_of_letters_digits_and_dashes() {
    for good in [
        "a.b",
        "com.example.moon-garden",
        "io.github.a-b.c2",
        "A.B-9",
    ] {
        assert_eq!(check_id(good), Ok(()), "{good}");
    }
    for bad in [
        "",
        "moon",
        "com.",
        ".com.a",
        "com..a",
        "com.2moon",
        "com.-moon",
        "com.moon_garden",
        "com.moon garden",
        "com.café",
    ] {
        assert!(check_id(bad).is_err(), "{bad}");
        let mut part = moon();
        part.id = bad.to_owned();
        let settings: Vec<String> = fields(&part).into_iter().map(|p| p.setting).collect();
        assert!(
            settings.contains(&"app.id".to_owned()),
            "{bad}: {settings:?}"
        );
    }
}

#[test]
fn every_crate_name_new_makes_passes_under_the_placeholder() {
    for name in ["Snake", "Moon Garden", "Lune Café", "a 2 b"] {
        let krate = crate::contract::new::crate_name(name).unwrap();
        assert_eq!(
            check_id(&format!("{PLACEHOLDER}{krate}")),
            Ok(()),
            "{krate}"
        );
    }
}

#[test]
fn a_placeholder_id_is_a_warning() {
    let part = AppPart::new("Snake", "com.example.snake");
    let problems = fields(&part);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(
        (problems[0].level, problems[0].setting.as_str()),
        (Level::Warning, "app.id")
    );
    assert!(
        problems[0].message.contains("share their saves"),
        "{problems:?}"
    );
}

#[test]
fn the_version_is_semver() {
    for good in [
        "0.1.0",
        "1.2.3",
        "1.0.0-alpha",
        "1.0.0-rc.1+build.5",
        "10.20.30",
    ] {
        assert_eq!(check_version(good), Ok(()), "{good}");
    }
    for bad in [
        "", "1", "1.2", "1.2.3.4", "v1.2.3", "01.2.3", "1.2.x", "1.2.3-",
    ] {
        assert!(check_version(bad).is_err(), "{bad}");
        let mut part = moon();
        part.version = Some(bad.to_owned());
        assert_eq!(found(&part), error("app.version"), "{bad}");
    }
}

#[test]
fn the_build_starts_at_one() {
    let mut part = moon();
    part.build = 0;
    assert_eq!(found(&part), error("app.build"));
}

#[test]
fn the_storage_names_one_folder() {
    for bad in [
        "", "a/b", "a\\b", "a:b", ".hidden", "trail.", " lead", "trail ", "café",
    ] {
        assert!(check_storage(bad).is_err(), "{bad:?}");
        let mut part = moon();
        part.storage = Some(bad.to_owned());
        assert_eq!(found(&part), error("app.storage"), "{bad:?}");
    }
    for good in [
        "moon_garden",
        "Moon Garden",
        "shuyi-hello-grid",
        "com.example.a",
    ] {
        assert_eq!(check_storage(good), Ok(()), "{good}");
    }
}

#[test]
fn the_rust_file_holds_the_defaults_alone() {
    let mut part = AppPart::new("Moon Garden", "com.example.moon-garden");
    part.code = Some("crates/game/src/app_info.rs".to_owned());
    insta::assert_snapshot!("rust_defaults", rust(&part, "cargo shuyi"));
}

#[test]
fn the_rust_file_sets_each_field_off_its_default() {
    let mut part = AppPart::new("Hello, Grid", "io.github.someone.hello-grid");
    part.version = Some("1.2.0-beta.1".to_owned());
    part.storage = Some("shuyi-hello-grid".to_owned());
    part.build = 12;
    part.publisher = "Someone \"S\" \\ Co".to_owned();
    insta::assert_snapshot!("rust_every_field", rust(&part, "shuyi"));
}

#[test]
fn the_rust_file_keeps_a_name_beyond_ascii_as_it_is() {
    let part = AppPart::new("Lune Café", "com.example.lune-caf");
    let text = rust(&part, "shuyi");
    assert!(
        text.contains("AppInfo::new(\"Lune Café\", \"com.example.lune-caf\", "),
        "{text}"
    );
    assert_eq!(quoted("a\u{7}b"), "\"a\\u{7}b\"");
    assert!(
        text.starts_with("// Generated by `shuyi app-info`"),
        "{text}"
    );
    assert!(
        text.lines().next().unwrap().ends_with("Do not edit."),
        "{text}"
    );
}

#[test]
fn app_info_writes_the_file_and_check_finds_it_stale() {
    let root = scratch("run");
    let mut part = moon();
    part.code = Some("crates/game/src/app_info.rs".to_owned());
    let path = root.join("crates/game/src/app_info.rs");
    // Missing: stale, and out of date in the problems.
    assert_eq!(
        stale(&part, &root, "shuyi"),
        ["crates/game/src/app_info.rs"]
    );
    let stale_found = problems(&part, &root, "shuyi");
    assert_eq!(stale_found.len(), 1, "{stale_found:?}");
    assert_eq!(stale_found[0].setting, "app.code");
    assert_eq!(
        stale_found[0].message,
        "crates/game/src/app_info.rs is out of date: run `shuyi app-info`"
    );
    let error = run(&part, &root, "shuyi", true).unwrap_err();
    assert!(error.contains("crates/game/src/app_info.rs"), "{error}");
    // Written, then the same bytes again.
    assert_eq!(
        run(&part, &root, "shuyi", false),
        Ok("app-info: wrote crates/game/src/app_info.rs".to_owned())
    );
    let first = fs::read(&path).unwrap();
    run(&part, &root, "shuyi", false).unwrap();
    assert_eq!(fs::read(&path).unwrap(), first);
    assert_eq!(first, rust(&part, "shuyi").into_bytes());
    assert_eq!(
        run(&part, &root, "shuyi", true),
        Ok("app-info: up to date".to_owned())
    );
    let none = problems(&part, &root, "shuyi");
    assert!(none.is_empty(), "{none:?}");
    // Line endings don't count.
    let crlf = String::from_utf8(first).unwrap().replace('\n', "\r\n");
    fs::write(&path, crlf).unwrap();
    let none = stale(&part, &root, "shuyi");
    assert!(none.is_empty(), "{none:?}");
    // A change to the part makes it stale.
    part.build = 2;
    assert_eq!(
        stale(&part, &root, "shuyi"),
        ["crates/game/src/app_info.rs"]
    );
    assert!(run(&part, &root, "shuyi", true).is_err());
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn app_info_writes_nothing_without_code_or_with_an_error() {
    let root = scratch("none");
    assert_eq!(
        run(&moon(), &root, "shuyi", false),
        Ok("app-info: shuyi.ron's `app` names no `code`: no file to write".to_owned())
    );
    let none = stale(&moon(), &root, "shuyi");
    assert!(none.is_empty(), "{none:?}");
    let mut bad = moon();
    bad.id = "moon".to_owned();
    bad.code = Some("app_info.rs".to_owned());
    let error = run(&bad, &root, "shuyi", false).unwrap_err();
    assert!(error.contains("app.id: "), "{error}");
    assert!(!root.join("app_info.rs").exists());
    fs::remove_dir_all(&root).unwrap();
}

/// A `side` × `side` picture whose pixels follow their place and `seed`.
fn picture(side: u32, seed: u8) -> Vec<u8> {
    let mut out = Vec::new();
    for y in 0..side {
        for x in 0..side {
            let byte = |n: u32| u8::try_from(n % 256).unwrap();
            out.extend([byte(x), byte(y), byte(x + y + u32::from(seed)), byte(x * 3)]);
        }
    }
    out
}

/// Writes a PNG file of `width` × `height` at `path` under `root`.
fn write_png(root: &Path, path: &str, width: u32, height: u32) {
    let rgba = vec![200; 4 * width as usize * height as usize];
    let full = root.join(path);
    fs::create_dir_all(full.parent().unwrap()).unwrap();
    fs::write(full, codec::encode_png(width, height, &rgba).unwrap()).unwrap();
}

/// The settings of `part`'s problems in `root`, fields and icon.
fn found_in(part: &AppPart, root: &Path) -> Vec<(Level, String)> {
    checked(part, root)
        .into_iter()
        .map(|p| (p.level, p.setting))
        .collect()
}

#[test]
fn downscale_of_one_colour_gives_that_colour() {
    let red = [250, 10, 30, 255].repeat(512 * 512);
    for side in WINDOW_ICON_SIDES {
        assert_eq!(
            downscale(512, 512, &red, side),
            [250, 10, 30, 255].repeat((side * side) as usize),
            "{side}"
        );
    }
    let faint = [9, 99, 199, 40].repeat(6 * 6);
    assert_eq!(downscale(6, 6, &faint, 4), [9, 99, 199, 40].repeat(16));
}

#[test]
fn downscale_of_a_checkerboard_is_its_mean_rounded_half_up() {
    let board = [
        [0, 0, 0, 255],
        [255, 255, 255, 255],
        [255, 255, 255, 255],
        [0, 0, 0, 255],
    ]
    .concat();
    // 127.5 rounds up.
    assert_eq!(downscale(2, 2, &board, 1), [128, 128, 128, 255]);
    // Alpha's mean rounds half up too: (0 + 255 + 0 + 255) / 4 = 127.5.
    let half = [
        [10, 20, 30, 0],
        [10, 20, 30, 255],
        [10, 20, 30, 255],
        [10, 20, 30, 0],
    ]
    .concat();
    assert_eq!(downscale(2, 2, &half, 1), [10, 20, 30, 128]);
}

#[test]
fn a_transparent_pixel_adds_nothing_to_its_neighbours_colour() {
    // A red pixel beside a transparent green one: still red, half seen.
    let edge = [[255, 0, 0, 255], [0, 255, 0, 0]].concat();
    assert_eq!(downscale(2, 1, &edge, 1), [255, 0, 0, 128]);
    // All transparent: nothing.
    assert_eq!(downscale(1, 1, &[7, 8, 9, 0], 1), [0, 0, 0, 0]);
    // Colour weighed by alpha: (200 × 255 + 100 × 85) / 340 = 175.
    let weighed = [[200, 0, 0, 255], [100, 0, 0, 85]].concat();
    assert_eq!(downscale(2, 1, &weighed, 1), [175, 0, 0, 170]);
}

#[test]
fn downscale_squares_follow_x_times_width_over_size() {
    // 3 columns to 2: [0, 1) and [1, 3). The one row in stands for both
    // rows out, since a square is never empty.
    let row = [[10, 0, 0, 255], [20, 0, 0, 255], [41, 0, 0, 255]].concat();
    assert_eq!(
        downscale(3, 1, &row, 2),
        [[10, 0, 0, 255], [31, 0, 0, 255]].concat().repeat(2)
    );
    let picture = picture(512, 0);
    let small = downscale(512, 512, &picture, 16);
    assert_eq!(small.len(), 16 * 16 * 4);
    assert_eq!(downscale(512, 512, &picture, 16), small, "the same bytes");
}

#[test]
fn the_icon_is_a_square_png_of_512_pixels_or_more() {
    let root = scratch("icon");
    let mut part = moon();
    part.icon = Some("art/icon.png".to_owned());
    // Missing.
    assert_eq!(found_in(&part, &root), error("app.icon"));
    assert!(
        icon(&part, &root)
            .unwrap_err()
            .starts_with("art/icon.png can't be read")
    );
    // Not a PNG.
    fs::create_dir_all(root.join("art")).unwrap();
    fs::write(root.join("art/icon.png"), b"not a picture").unwrap();
    assert_eq!(found_in(&part, &root), error("app.icon"));
    assert!(icon(&part, &root).unwrap_err().contains("isn't a PNG file"));
    // Not square.
    write_png(&root, "art/icon.png", 600, 512);
    assert_eq!(found_in(&part, &root), error("app.icon"));
    assert_eq!(
        icon(&part, &root).unwrap_err(),
        "art/icon.png is 600 × 512 pixels: an icon is square"
    );
    // Too small.
    write_png(&root, "art/icon.png", 256, 256);
    assert_eq!(found_in(&part, &root), error("app.icon"));
    assert!(
        icon(&part, &root)
            .unwrap_err()
            .starts_with("art/icon.png is 256 pixels a side: an icon is 512 or more")
    );
    // Good, at 512 and above.
    for side in [512, 520] {
        write_png(&root, "art/icon.png", side, side);
        assert_eq!(found_in(&part, &root), []);
        assert_eq!(icon(&part, &root).unwrap().unwrap().side, side);
    }
    assert_eq!(icon(&moon(), &root), Ok(None));
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn an_icon_with_code_needs_window_icons() {
    let mut part = moon();
    part.icon = Some("art/icon.png".to_owned());
    // The icon alone is for the packages: no rule of the part's fields.
    assert_eq!(found(&part), []);
    part.code = Some("crates/game/src/app_info.rs".to_owned());
    assert_eq!(found(&part), error("app.window_icons"));
    part.window_icons = Some("crates/game/icon".to_owned());
    assert_eq!(found(&part), []);
}

#[test]
fn include_bytes_names_the_icons_from_the_rust_files_folder() {
    assert_eq!(
        relative("crates/game/src/app_info.rs", "crates/game/icon/16.png"),
        "../icon/16.png"
    );
    assert_eq!(
        relative("src/app_info.rs", "src/icon/16.png"),
        "icon/16.png"
    );
    assert_eq!(relative("app_info.rs", "./icon/16.png"), "icon/16.png");
    assert_eq!(relative("a\\b\\c.rs", "x/y/16.png"), "../../x/y/16.png");
}

#[test]
fn the_rust_file_reads_the_window_icons_last() {
    let mut part = AppPart::new("Moon Garden", "com.example.moon-garden");
    part.code = Some("crates/game/src/app_info.rs".to_owned());
    part.build = 2;
    part.icon = Some("art/icon.png".to_owned());
    part.window_icons = Some("crates/game/icon/".to_owned());
    insta::assert_snapshot!("rust_window_icon", rust(&part, "cargo shuyi"));
    // Without `window_icons`, the Rust has no icon.
    part.window_icons = None;
    assert!(!rust(&part, "shuyi").contains("WindowIcon"));
}

#[test]
fn app_info_writes_the_window_icons_and_check_finds_them_stale() {
    let root = scratch("icons");
    let mut part = moon();
    part.code = Some("crates/game/src/app_info.rs".to_owned());
    part.icon = Some("art/icon.png".to_owned());
    part.window_icons = Some("crates/game/icon".to_owned());
    let icon_png = codec::encode_png(512, 512, &picture(512, 0)).unwrap();
    fs::create_dir_all(root.join("art")).unwrap();
    fs::write(root.join("art/icon.png"), &icon_png).unwrap();
    assert_eq!(
        run(&part, &root, "shuyi", false),
        Ok(
            "app-info: wrote crates/game/src/app_info.rs, crates/game/icon/16.png, \
            crates/game/icon/32.png, crates/game/icon/64.png"
                .to_owned()
        )
    );
    let read = |side: u32| fs::read(root.join(format!("crates/game/icon/{side}.png"))).unwrap();
    let first: Vec<Vec<u8>> = WINDOW_ICON_SIDES.iter().map(|&s| read(s)).collect();
    for (side, bytes) in WINDOW_ICON_SIDES.iter().zip(&first) {
        let (width, height, rgba) = codec::decode_png(bytes).unwrap();
        assert_eq!((width, height), (*side, *side));
        assert_eq!(rgba, downscale(512, 512, &picture(512, 0), *side));
    }
    // Twice: the same bytes.
    run(&part, &root, "shuyi", false).unwrap();
    let again: Vec<Vec<u8>> = WINDOW_ICON_SIDES.iter().map(|&s| read(s)).collect();
    assert_eq!(again, first);
    assert_eq!(
        run(&part, &root, "shuyi", true),
        Ok("app-info: up to date".to_owned())
    );
    let none = problems(&part, &root, "shuyi");
    assert!(none.is_empty(), "{none:?}");
    // A change to the icon makes its window icons stale.
    let changed = codec::encode_png(512, 512, &picture(512, 1)).unwrap();
    fs::write(root.join("art/icon.png"), changed).unwrap();
    let error = run(&part, &root, "shuyi", true).unwrap_err();
    assert!(error.contains("crates/game/icon/16.png"), "{error}");
    assert!(!error.contains("app_info.rs"), "{error}");
    let stale_found = problems(&part, &root, "shuyi");
    assert!(
        stale_found
            .iter()
            .all(|p| p.setting == "app.window_icons" && p.message.contains("crates/game/icon/")),
        "{stale_found:?}"
    );
    // A window icon with CR bytes added is stale: a PNG file is compared whole.
    fs::write(root.join("art/icon.png"), &icon_png).unwrap();
    let mut crlf = read(16);
    crlf.insert(0, b'\r');
    fs::write(root.join("crates/game/icon/16.png"), crlf).unwrap();
    assert_eq!(stale(&part, &root, "shuyi"), ["crates/game/icon/16.png"]);
    // An icon that doesn't load: app-info fails with the icon's rule, and
    // the window icons aren't called stale.
    fs::write(root.join("art/icon.png"), b"broken").unwrap();
    let error = run(&part, &root, "shuyi", false).unwrap_err();
    assert!(
        error.contains("app.icon: art/icon.png isn't a PNG file"),
        "{error}"
    );
    let none = stale(&part, &root, "shuyi");
    assert!(none.is_empty(), "{none:?}");
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn app_info_writes_the_window_icons_without_code() {
    let root = scratch("icons-alone");
    let mut part = moon();
    part.icon = Some("icon.png".to_owned());
    part.window_icons = Some("icon".to_owned());
    write_png(&root, "icon.png", 512, 512);
    assert_eq!(
        run(&part, &root, "shuyi", false),
        Ok("app-info: wrote icon/16.png, icon/32.png, icon/64.png".to_owned())
    );
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn app_info_takes_check_or_nothing() {
    assert_eq!(parse_args(&[]), Ok(false));
    assert_eq!(parse_args(&["--check".to_owned()]), Ok(true));
    assert!(parse_args(&["--write".to_owned()]).is_err());
}
