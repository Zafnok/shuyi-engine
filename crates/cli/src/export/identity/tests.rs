use std::fs;

use super::*;

/// The fixture game's identity: `tools/export-check/lune-cafe`'s `app`.
fn lune() -> Identity {
    Identity {
        name: "Lune Café".to_owned(),
        id: "io.github.zafnok.shuyi-engine.lune-cafe".to_owned(),
        version: "1.2.3".to_owned(),
        build: 4,
        publisher: "Shuyi Engine".to_owned(),
        icon: None,
        icon_sizes: BTreeMap::new(),
    }
}

/// A folder of the system's for one test, empty.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-identity-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// A PNG of `side` pixels a side, every pixel `rgba`, written at `path`.
fn png(path: &Path, side: u32, rgba: [u8; 4]) {
    let pixels: Vec<u8> = (0..side * side).flat_map(|_| rgba).collect();
    fs::write(path, codec::encode_png(side, side, &pixels).unwrap()).unwrap();
}

/// The fixture game's icon, `tools/export-check/lune-cafe/icon.png`: 1024
/// pixels a side, a few flat squares on a night blue, so it stays code and
/// not art.
fn fixture_icon() -> Vec<u8> {
    const SIDE: u32 = 1024;
    // Each square: left, top, right and bottom, then its colour.
    let squares = [
        ([192, 192, 576, 576], [240, 200, 80, 255]),
        ([640, 384, 832, 576], [236, 236, 236, 255]),
        ([256, 704, 832, 832], [60, 160, 150, 255]),
    ];
    let mut pixels = Vec::new();
    for y in 0..SIDE {
        for x in 0..SIDE {
            let colour = squares
                .iter()
                .find(|([l, t, r, b], _)| (*l..*r).contains(&x) && (*t..*b).contains(&y))
                .map_or([24, 28, 56, 255], |(_, colour)| *colour);
            pixels.extend(colour);
        }
    }
    codec::encode_png(SIDE, SIDE, &pixels).unwrap()
}

/// The fixture game's folder.
fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/export-check/lune-cafe")
}

#[test]
fn the_fixture_games_icon_is_the_helpers() {
    let file = fixture().join("icon.png");
    let made = fixture_icon();
    if std::env::var_os("SHUYI_BLESS").is_some() {
        fs::write(&file, &made).unwrap();
    }
    assert!(
        fs::read(&file).unwrap() == made,
        "{} isn't the helper's picture: run this test with SHUYI_BLESS=1",
        file.display()
    );
}

#[test]
fn of_takes_the_part_and_joins_its_files_to_the_root() {
    let mut part = AppPart::new("Lune Café", "io.github.zafnok.shuyi-engine.lune-cafe");
    part.build = 4;
    part.publisher = "Shuyi Engine".to_owned();
    part.icon = Some("icon.png".to_owned());
    part.icon_sizes.insert(16, "icon-16.png".to_owned());
    let identity = Identity::of(&part, Path::new("game"), "1.2.3");
    assert_eq!(identity.icon, Some(Path::new("game").join("icon.png")));
    assert_eq!(
        identity.icon_sizes[&16],
        Path::new("game").join("icon-16.png")
    );
    assert_eq!(
        Identity {
            icon: None,
            icon_sizes: BTreeMap::new(),
            ..identity
        },
        lune()
    );
}

#[test]
fn numbers_are_the_version_then_the_build_each_at_most_65535() {
    assert_eq!(lune().numbers(), Ok([1, 2, 3, 4]));
    let pre = Identity {
        version: "2.0.1-beta.1+abc".to_owned(),
        ..lune()
    };
    assert_eq!(pre.numbers(), Ok([2, 0, 1, 4]));
    let top = Identity {
        version: "65535.65535.65535".to_owned(),
        build: 65535,
        ..lune()
    };
    assert_eq!(top.numbers(), Ok([65535; 4]));
    for version in ["65536.0.0", "1.65536.0", "1.0.65536"] {
        let big = Identity {
            version: version.to_owned(),
            ..lune()
        };
        let error = big.numbers().unwrap_err();
        assert!(
            error.contains("`app.version`") && error.contains("65536"),
            "{error}"
        );
    }
    let build = Identity {
        build: 65536,
        ..lune()
    };
    let error = build.numbers().unwrap_err();
    assert!(error.contains("`app.build` holds 65536"), "{error}");
    let bad = Identity {
        version: "one".to_owned(),
        ..lune()
    };
    assert!(bad.numbers().unwrap_err().contains("isn't semver"));
}

#[test]
fn the_short_version_and_the_app_folder() {
    let pre = Identity {
        version: "1.2.3-rc.1".to_owned(),
        name: "Moon/Garden: Two".to_owned(),
        ..lune()
    };
    assert_eq!(pre.short_version().as_deref(), Ok("1.2.3"));
    assert_eq!(pre.app_folder(), "Moon-Garden- Two.app");
    assert_eq!(lune().app_folder(), "Lune Café.app");
}

#[test]
fn icons_leave_out_sizes_above_the_source_and_take_the_drawn_ones() {
    let dir = folder("icons");
    png(&dir.join("icon.png"), 512, [200, 40, 40, 255]);
    png(&dir.join("16.png"), 16, [0, 0, 255, 255]);
    let identity = Identity {
        icon: Some(dir.join("icon.png")),
        icon_sizes: BTreeMap::from([(16, dir.join("16.png"))]),
        ..lune()
    };
    let made = icons(&identity, &[16, 32, 512, 1024]).unwrap();
    let sides: Vec<u32> = made.iter().map(|(side, _)| *side).collect();
    assert_eq!(sides, [16, 32, 512]);
    let pixel = |png: &[u8]| codec::decode_png(png).unwrap().2[..4].to_vec();
    assert_eq!(pixel(&made[0].1), [0, 0, 255, 255], "16 is the drawn one");
    assert_eq!(pixel(&made[1].1), [200, 40, 40, 255], "32 is downscaled");
    assert_eq!(icons(&lune(), &[16]), Ok(Vec::new()));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_bad_icon_is_an_error_naming_its_path() {
    let dir = folder("bad-icon");
    let file = dir.join("icon.png");
    fs::write(&file, "not a picture").unwrap();
    let identity = Identity {
        icon: Some(file.clone()),
        ..lune()
    };
    let error = icons(&identity, &[16]).unwrap_err();
    assert!(error.contains(&file.display().to_string()), "{error}");
    png(&file, 256, [0; 4]);
    let error = icons(&identity, &[16]).unwrap_err();
    assert!(error.contains("an icon is 512 or more"), "{error}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_ico_has_its_header_its_entries_and_the_pngs_at_their_offsets() {
    let pictures = vec![(16, vec![1; 10]), (256, vec![2; 20])];
    let ico = ico(&pictures);
    assert_eq!(ico[..6], [0, 0, 1, 0, 2, 0]);
    let entry = |at: usize| &ico[6 + 16 * at..6 + 16 * (at + 1)];
    assert_eq!(
        entry(0),
        [16, 16, 0, 0, 1, 0, 32, 0, 10, 0, 0, 0, 38, 0, 0, 0]
    );
    assert_eq!(
        entry(1),
        [0, 0, 0, 0, 1, 0, 32, 0, 20, 0, 0, 0, 48, 0, 0, 0],
        "256 is written 0"
    );
    assert_eq!(ico[38..48], [1; 10]);
    assert_eq!(ico[48..], [2; 20]);
    assert_eq!(ico.len(), 68);
}

#[test]
fn the_rc_holds_the_icon_and_every_version_string() {
    let rc = rc(&lune(), "lune-cafe", [1, 2, 3, 4], true);
    assert_eq!(
        rc,
        "#pragma code_page(65001)\n\
         1 ICON \"app.ico\"\n\
         1 VERSIONINFO\n\
         FILEVERSION 1,2,3,4\n\
         PRODUCTVERSION 1,2,3,4\n\
         FILEOS 0x40004\n\
         FILETYPE 0x1\n\
         BEGIN\n\
         \x20 BLOCK \"StringFileInfo\"\n\
         \x20 BEGIN\n\
         \x20   BLOCK \"040904B0\"\n\
         \x20   BEGIN\n\
         \x20     VALUE \"CompanyName\", \"Shuyi Engine\"\n\
         \x20     VALUE \"FileDescription\", \"Lune Café\"\n\
         \x20     VALUE \"FileVersion\", \"1.2.3\"\n\
         \x20     VALUE \"InternalName\", \"lune-cafe\"\n\
         \x20     VALUE \"OriginalFilename\", \"lune-cafe.exe\"\n\
         \x20     VALUE \"ProductName\", \"Lune Café\"\n\
         \x20     VALUE \"ProductVersion\", \"1.2.3\"\n\
         \x20   END\n\
         \x20 END\n\
         \x20 BLOCK \"VarFileInfo\"\n\
         \x20 BEGIN\n\
         \x20   VALUE \"Translation\", 0x409, 1200\n\
         \x20 END\n\
         END\n"
    );
}

#[test]
fn the_rc_without_a_publisher_or_an_icon_and_with_quotes() {
    let identity = Identity {
        name: "The \"Moon\" \\ Garden".to_owned(),
        publisher: String::new(),
        ..lune()
    };
    let rc = rc(&identity, "moon", [1, 2, 3, 4], false);
    assert!(!rc.contains("CompanyName") && !rc.contains("ICON"), "{rc}");
    assert!(
        rc.contains("VALUE \"ProductName\", \"The \"\"Moon\"\" \\\\ Garden\"\n"),
        "{rc}"
    );
}

#[test]
fn the_info_plist_says_who_the_game_is_escaped() {
    let identity = Identity {
        name: "Fish & <Chips>".to_owned(),
        ..lune()
    };
    let plist = info_plist("lune-cafe", Some((&identity, "1.2.3")), true);
    for line in [
        "<key>CFBundleIdentifier</key>\n\t<string>io.github.zafnok.shuyi-engine.lune-cafe</string>",
        "<key>CFBundleName</key>\n\t<string>Fish &amp; &lt;Chips&gt;</string>",
        "<key>CFBundleDisplayName</key>\n\t<string>Fish &amp; &lt;Chips&gt;</string>",
        "<key>CFBundleShortVersionString</key>\n\t<string>1.2.3</string>",
        "<key>CFBundleVersion</key>\n\t<string>4</string>",
        "<key>CFBundleExecutable</key>\n\t<string>lune-cafe</string>",
        "<key>CFBundleIconFile</key>\n\t<string>AppIcon</string>",
        "<key>CFBundlePackageType</key>\n\t<string>APPL</string>",
        "<key>CFBundleInfoDictionaryVersion</key>\n\t<string>6.0</string>",
        "<key>NSHighResolutionCapable</key>\n\t<true/>",
        "<key>LSApplicationCategoryType</key>\n\t<string>public.app-category.games</string>",
    ] {
        assert!(plist.contains(line), "{line} in {plist}");
    }
    let plain = info_plist("lune-cafe", Some((&lune(), "1.2.3")), false);
    assert!(!plain.contains("CFBundleIconFile"), "{plain}");
}

#[test]
fn the_info_plist_without_an_identity_is_the_minimal_one() {
    assert_eq!(
        info_plist("hello-grid", None, false),
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>CFBundleExecutable</key>\n\
         \t<string>hello-grid</string>\n\
         \t<key>CFBundlePackageType</key>\n\
         \t<string>APPL</string>\n\
         </dict>\n\
         </plist>\n"
    );
}

#[test]
fn the_icns_has_its_length_and_the_entries_a_512_pixel_source_gives() {
    assert_eq!(icns_sides(), [32, 64, 128, 256, 512, 1024]);
    let pictures: Vec<(u32, Vec<u8>)> = icns_sides()
        .into_iter()
        .filter(|&side| side <= 512)
        .map(|side| (side, vec![7; 3]))
        .collect();
    let icns = icns(&pictures);
    assert_eq!(icns[..4], *b"icns");
    assert_eq!(
        u32::from_be_bytes(icns[4..8].try_into().unwrap()) as usize,
        icns.len()
    );
    let mut kinds = Vec::new();
    let mut at = 8;
    while at < icns.len() {
        kinds.push(String::from_utf8(icns[at..at + 4].to_vec()).unwrap());
        let length = u32::from_be_bytes(icns[at + 4..at + 8].try_into().unwrap()) as usize;
        assert_eq!(length, 8 + 3);
        assert_eq!(icns[at + 8..at + length], [7; 3]);
        at += length;
    }
    assert_eq!(
        kinds,
        ["ic11", "ic12", "ic07", "ic08", "ic13", "ic09", "ic14"]
    );
}

#[test]
fn the_desktop_entry() {
    assert_eq!(
        desktop_entry(&lune(), "lune-cafe", true),
        "[Desktop Entry]\nType=Application\nName=Lune Café\nExec=lune-cafe\n\
         Icon=io.github.zafnok.shuyi-engine.lune-cafe\nCategories=Game;\nTerminal=false\n"
    );
    let bare = Identity {
        name: "Back\\slash".to_owned(),
        ..lune()
    };
    assert_eq!(
        desktop_entry(&bare, "lune-cafe", false),
        "[Desktop Entry]\nType=Application\nName=Back\\\\slash\nExec=lune-cafe\n\
         Categories=Game;\nTerminal=false\n"
    );
}

#[test]
fn the_linux_files_are_the_entry_and_the_hicolor_icons() {
    let dir = folder("linux");
    png(&dir.join("icon.png"), 512, [9, 9, 9, 255]);
    let identity = Identity {
        icon: Some(dir.join("icon.png")),
        ..lune()
    };
    let files = linux_files(&identity, "lune-cafe").unwrap();
    let paths: Vec<&str> = files.iter().map(|(path, _)| path.as_str()).collect();
    let id = "io.github.zafnok.shuyi-engine.lune-cafe";
    let mut expected = vec![format!("share/applications/{id}.desktop")];
    for side in HICOLOR_SIDES {
        expected.push(format!("share/icons/hicolor/{side}x{side}/apps/{id}.png"));
    }
    assert_eq!(paths, expected);
    let without = linux_files(&lune(), "lune-cafe").unwrap();
    assert_eq!(without.len(), 1);
    assert!(!String::from_utf8_lossy(&without[0].1).contains("Icon="));
    let _ = fs::remove_dir_all(&dir);
}
