//! The game's identity in the desktop packages: what each
//! exporter writes from `shuyi.ron`'s `app` part, with a stand-in program,
//! and the arguments of the tools it runs.

use super::super::identity::{HICOLOR_SIDES, NONE};
use super::*;
use crate::codec;
use crate::contract::target::{Level, TargetCheck};
use crate::project::AppPart;

/// The fixture game's part, with an icon of `side` pixels at an absolute
/// path in `dir`.
fn lune(dir: &Path, side: u32) -> AppPart {
    let mut part = AppPart::new("Lune Café", "io.github.zafnok.shuyi-engine.lune-cafe");
    part.version = Some("1.2.3".to_owned());
    part.build = 4;
    part.publisher = "Shuyi Engine".to_owned();
    let icon = dir.join("icon.png");
    let pixels: Vec<u8> = (0..side * side).flat_map(|_| [240, 200, 80, 255]).collect();
    fs::write(&icon, codec::encode_png(side, side, &pixels).unwrap()).unwrap();
    part.icon = Some(icon.display().to_string());
    part
}

#[test]
fn a_linux_package_carries_its_desktop_entry_and_hicolor_icons() {
    let mut cx = cx("identity-linux", "linux");
    cx.app = Some(lune(&cx.out, 512));
    let built = cx.out.with_extension("built");
    fs::create_dir_all(&built).unwrap();
    fs::write(built.join("hello-grid"), "a program").unwrap();
    let report = desktop::pack(
        &cx,
        System::Linux,
        "x86_64-unknown-linux-gnu",
        &built.join("hello-grid"),
    )
    .unwrap();
    let folder = cx.out.join(&report.folder);
    let id = "io.github.zafnok.shuyi-engine.lune-cafe";
    let entry =
        fs::read_to_string(folder.join(format!("share/applications/{id}.desktop"))).unwrap();
    assert!(
        entry.contains("Name=Lune Café\nExec=hello-grid\n"),
        "{entry}"
    );
    for side in HICOLOR_SIDES {
        let icon = folder.join(format!("share/icons/hicolor/{side}x{side}/apps/{id}.png"));
        let (w, h, _) = codec::decode_png(&fs::read(&icon).unwrap()).unwrap();
        assert_eq!((w, h), (side, side));
    }
    let shipped = find("linux").unwrap().shipped(&cx, &report);
    let written = |path: String| shipped.sources.get(&path).map(|(kind, _)| *kind);
    let kind = Some(disclosure::FileKind::Written);
    assert_eq!(written(format!("share/applications/{id}.desktop")), kind);
    assert_eq!(
        written(format!("share/icons/hicolor/512x512/apps/{id}.png")),
        kind
    );
    let _ = fs::remove_dir_all(&built);
    let _ = fs::remove_dir_all(&cx.out);
}

#[test]
fn a_linux_package_without_an_app_part_has_no_share_folder_and_says_so() {
    let cx = cx("identity-none", "linux");
    let built = cx.out.with_extension("built");
    fs::create_dir_all(&built).unwrap();
    fs::write(built.join("hello-grid"), "a program").unwrap();
    let report = desktop::pack(
        &cx,
        System::Linux,
        "x86_64-unknown-linux-gnu",
        &built.join("hello-grid"),
    )
    .unwrap();
    assert!(!cx.out.join(&report.folder).join("share").exists());
    let warning = desktop::no_identity(&cx).unwrap();
    assert_eq!(
        (warning.level, warning.check, warning.message.as_str()),
        (Level::Warning, TargetCheck::App, NONE)
    );
    let mut with = cx.clone();
    with.app = Some(AppPart::new(
        "Lune Café",
        "io.github.zafnok.shuyi-engine.lune-cafe",
    ));
    assert_eq!(desktop::no_identity(&with), None);
    let _ = fs::remove_dir_all(&built);
    let _ = fs::remove_dir_all(&cx.out);
}

#[test]
fn the_bundle_takes_its_info_plist_and_icns_and_warns_under_1024_pixels() {
    let mut cx = cx("identity-macos", "macos");
    cx.app = Some(lune(&cx.out, 512));
    cx.version = "1.2.3".to_owned();
    let contents = cx.out.join("Lune Café.app").join("Contents");
    fs::create_dir_all(&contents).unwrap();
    let warnings = desktop::bundle_identity(&cx, &contents).unwrap();
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].message.contains("1024"), "{warnings:?}");
    let plist = fs::read_to_string(contents.join("Info.plist")).unwrap();
    assert!(plist.contains("<string>io.github.zafnok.shuyi-engine.lune-cafe</string>"));
    assert!(plist.contains("<key>CFBundleIconFile</key>"));
    let icns = fs::read(contents.join("Resources/AppIcon.icns")).unwrap();
    assert_eq!(icns[..4], *b"icns");
    assert_eq!(desktop::app_folder(&cx), "Lune Café.app");
    cx.app = Some(lune(&cx.out, 1024));
    assert_eq!(desktop::bundle_identity(&cx, &contents), Ok(Vec::new()));
    cx.app = None;
    assert_eq!(desktop::bundle_identity(&cx, &contents), Ok(Vec::new()));
    let plist = fs::read_to_string(contents.join("Info.plist")).unwrap();
    assert!(!plist.contains("CFBundleIdentifier"), "{plist}");
    assert_eq!(desktop::app_folder(&cx), "hello-grid.app");
    let _ = fs::remove_dir_all(&cx.out);
}

#[test]
fn a_version_number_over_65535_refuses_windows_before_any_tool_runs() {
    let mut cx = cx("identity-windows", "windows");
    let mut part = AppPart::new("Lune Café", "io.github.zafnok.shuyi-engine.lune-cafe");
    part.build = 70_000;
    cx.app = Some(part);
    let refused = desktop::windows_resource(&cx).unwrap_err();
    assert_eq!(refused.code(), codes::PROJECT_FILE);
    assert!(
        refused.message().contains("`app.build` holds 70000"),
        "{refused:?}"
    );
    cx.version = "1.70000.0".to_owned();
    let refused = desktop::windows_resource(&cx).unwrap_err();
    assert!(refused.message().contains("`app.version`"), "{refused:?}");
    cx.app = None;
    assert_eq!(desktop::windows_resource(&cx), Ok(None));
    let _ = fs::remove_dir_all(&cx.out);
}

#[test]
fn the_windows_build_links_the_resource_into_the_game_alone() {
    let mut cx = cx("identity-args", "windows");
    cx.release = true;
    cx.turned_on = vec!["packaged".to_owned()];
    let object = Path::new("target/shuyi-export/hello-grid-0.1.0-windows/app.o");
    assert_eq!(
        cx.rustc_args("x86_64-pc-windows-gnu", object),
        [
            "rustc",
            "-p",
            "hello-grid",
            "--bin",
            "hello-grid",
            "--target",
            "x86_64-pc-windows-gnu",
            "--release",
            "--features",
            "packaged",
            "--",
            "-C",
            &format!("link-arg={}", object.display()),
        ]
    );
    assert_eq!(
        cx.work(),
        cx.target_dir
            .join("shuyi-export")
            .join("hello-grid-0.1.0-windows")
    );
    assert_eq!(
        desktop::WINDRES_ARGS.join(" "),
        "--codepage=65001 -O coff -i app.rc -o app.o"
    );
    assert_eq!(windres(&|_| true), "x86_64-w64-mingw32-windres");
    assert_eq!(windres(&|_| false), "windres");
    assert!(desktop::windres_help(Some(System::Windows)).contains("mingw-w64-x86_64-gcc"));
    assert!(desktop::windres_help(Some(System::Linux)).contains("binutils-mingw-w64-x86-64"));
    let _ = fs::remove_dir_all(&cx.out);
}
