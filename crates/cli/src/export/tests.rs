//! The exporters' arguments and packages, on Hello, Grid's package with a
//! stand-in program (no build: `cargo metadata` alone, for the notices).

use std::io::Read as _;

use super::desktop::{self, MACOS_NOTE, NOTE, System, WINDOWS_NOTE};
use super::*;

/// A context on Hello, Grid with its export's folder under the temporary
/// folder, for test `name`.
fn cx(name: &str, target: &str) -> ExportCx {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-grid");
    let out = std::env::temp_dir().join(format!("shuyi-export-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    fs::create_dir_all(&out).unwrap();
    ExportCx {
        target: target.to_owned(),
        profile: "desktop".to_owned(),
        workspace: root.join("../.."),
        target_dir: root.join("../../target"),
        root,
        shown: out.display().to_string(),
        out,
        release: false,
        package: "hello-grid".to_owned(),
        bin: "hello-grid".to_owned(),
        version: "0.1.0".to_owned(),
        title: "Hello, Grid".to_owned(),
        game: Vec::new(),
        web: Some(Web {
            package: "hello-grid".to_owned(),
            bin: "hello-grid".to_owned(),
            title: "Hello, Grid".to_owned(),
            features: vec!["link".to_owned(), "capture".to_owned()],
            music: None,
            voice: None,
            dist: "dist/web".to_owned(),
        }),
        app: None,
        features: Vec::new(),
        turned_on: Vec::new(),
        beside: Vec::new(),
    }
}

#[test]
fn every_exporters_builds_turn_on_no_feature_and_never_the_live_link() {
    let mut cx = cx("args", "windows");
    for release in [false, true] {
        cx.release = release;
        for exporter in exporters() {
            let builds = exporter.cargo_args(&cx);
            assert_eq!(builds.len(), exporter.triples().len(), "{}", exporter.id());
            for (args, triple) in builds.iter().zip(exporter.triples()) {
                assert!(
                    args.windows(2)
                        .any(|w| w[0] == "--target" && w[1] == *triple)
                );
                // The Playdate's build turns on what makes the library the
                // game there, and nothing else.
                let features: Vec<&String> = args
                    .windows(2)
                    .filter(|w| w[0] == "--features")
                    .map(|w| &w[1])
                    .collect();
                if exporter.id() == "playdate" {
                    assert_eq!(features, [playdate::FEATURE], "{args:?}");
                } else {
                    assert!(
                        !args.iter().any(|a| a.contains("feature") || a == "-F"),
                        "{args:?}"
                    );
                }
                assert_eq!(args.iter().any(|a| a == "--release"), release, "{args:?}");
                assert_eq!(no_live_link(args), Ok(()));
            }
        }
    }
    let args = |items: &[&str]| items.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    for linked in [
        &["build", "--features", "link"][..],
        &["build", "--features", "capture,link"],
        &["build", "-F", "game/link"],
        &["build", "--features=link"],
    ] {
        let refused = no_live_link(&args(linked)).unwrap_err();
        assert_eq!(refused.code(), codes::PROJECT_FILE, "{linked:?}");
    }
    assert_eq!(
        no_live_link(&args(&["build", "--features", "linked"])),
        Ok(())
    );
    fs::remove_dir_all(&cx.out).unwrap();
}

/// What ships beside the program, as `shuyi.ron` writes it.
fn beside(path: &str, name: Option<&str>, optional: bool) -> Beside {
    let text = format!(
        "(export: (beside: [(path: {path:?}, name: {name}, optional: {optional})]))",
        name = name.map_or("None".to_owned(), |n| format!("Some({n:?})"))
    );
    crate::project::Config::parse(&text)
        .unwrap()
        .export
        .unwrap()
        .beside
        .remove(0)
}

#[test]
fn a_build_turns_on_the_export_parts_features_but_those_left_off() {
    let mut cx = cx("features", "windows");
    let part: ExportPart = crate::project::Config::parse(r#"(export: (features: ["a", "b"]))"#)
        .unwrap()
        .export
        .unwrap();
    cx.turned_on = turned_on(Some(&part), &[]).unwrap();
    for exporter in [
        &desktop::WINDOWS as &dyn Exporter,
        &desktop::MACOS,
        &web::WEB,
    ] {
        for args in exporter.cargo_args(&cx) {
            let on: Vec<&str> = args
                .windows(2)
                .filter(|w| w[0] == "--features")
                .flat_map(|w| w[1].split(','))
                .collect();
            assert_eq!(on, ["a", "b"], "{args:?}");
        }
    }
    assert_eq!(turned_on(Some(&part), &["a".to_owned()]).unwrap(), ["b"]);
    assert_eq!(turned_on(None, &[]).unwrap(), Vec::<String>::new());
    let unknown = turned_on(Some(&part), &["c".to_owned()]).unwrap_err();
    assert!(unknown.message().starts_with("--without c:"), "{unknown:?}");
    assert!(!playdate::PLAYDATE.takes_export_part());
    fs::remove_dir_all(&cx.out).unwrap();
}

#[test]
fn link_in_the_export_parts_features_is_refused() {
    let mut cx = cx("link-part", "windows");
    cx.turned_on = vec!["art".to_owned(), "link".to_owned()];
    for args in desktop::WINDOWS.cargo_args(&cx) {
        let refused = no_live_link(&args).unwrap_err();
        assert_eq!(refused.code(), codes::PROJECT_FILE);
    }
    fs::remove_dir_all(&cx.out).unwrap();
}

#[test]
fn a_missing_beside_path_refuses_the_export_by_name_unless_it_is_optional() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/hello-grid");
    let part = |item: Beside| {
        let mut part = crate::project::Config::parse("(export: ())")
            .unwrap()
            .export
            .unwrap();
        part.beside.push(item);
        part
    };
    let missing = part(beside("no-such-folder", None, false));
    let refused = present(&root, Some(&missing)).unwrap_err();
    assert_eq!(refused.code(), codes::PROJECT_FILE);
    assert!(refused.message().contains("no-such-folder"), "{refused:?}");
    let optional = part(beside("no-such-folder", None, true));
    assert_eq!(present(&root, Some(&optional)).unwrap(), []);
    let there = part(beside("capture", None, false));
    assert_eq!(present(&root, Some(&there)).unwrap().len(), 1);
    let path_name = part(beside("capture", Some("../up"), false));
    assert!(
        present(&root, Some(&path_name))
            .unwrap_err()
            .message()
            .contains("plain name")
    );
}

#[test]
fn the_exporters_are_the_targets_with_their_profiles() {
    assert_eq!(ids(), ["windows", "linux", "macos", "web", "playdate"]);
    let profiles: Vec<&str> = exporters().iter().map(|e| e.profile()).collect();
    assert_eq!(
        profiles,
        ["desktop", "desktop", "desktop", "web", "playdate"]
    );
    assert_eq!(find("macos").unwrap().triples().len(), 2);
    assert!(find("steam-deck").is_none());
}

#[test]
fn a_target_with_no_run_step_gives_the_default_answer() {
    let report = ExportReport::default();
    let out = Path::new(".");
    let refused = find("web")
        .unwrap()
        .run(&RunCx {
            out,
            report: &report,
        })
        .unwrap_err();
    assert_eq!(
        refused,
        ExportError::NoRunStep("no run step for web yet".to_owned())
    );
    assert_eq!(refused.code(), codes::NO_EXPORTER);
    assert_eq!(codes::exit_code(Some(refused.code().0)), 1);
    // Another system's desktop target has none on this one.
    let other = if cfg!(windows) { "linux" } else { "windows" };
    let refused = find(other)
        .unwrap()
        .run(&RunCx {
            out,
            report: &report,
        })
        .unwrap_err();
    assert_eq!(refused.code(), codes::NO_EXPORTER);
}

#[test]
fn the_program_is_the_one_named_as_the_package_or_the_only_one() {
    let bins = |names: &[&str]| names.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(program("g", &bins(&["tool", "g"])), Ok("g".to_owned()));
    assert_eq!(program("g", &bins(&["play"])), Ok("play".to_owned()));
    assert!(program("g", &bins(&[])).is_err());
    assert!(program("g", &bins(&["a", "b"])).is_err());
}

#[test]
fn the_version_is_the_app_parts_else_the_packages() {
    let package = || "0.1.0".to_owned();
    assert_eq!(version(None, package()), "0.1.0");
    let mut app = crate::project::AppPart::new("A game", "com.example.a-game");
    assert_eq!(version(Some(&app), package()), "0.1.0");
    app.version = Some("1.4.0".to_owned());
    assert_eq!(version(Some(&app), package()), "1.4.0");
}

#[test]
fn a_missing_target_says_how_to_add_it() {
    assert_eq!(
        missing_target("x86_64-apple-darwin").message(),
        "the Rust target x86_64-apple-darwin isn't installed: run `rustup target add \
         x86_64-apple-darwin`"
    );
    assert_eq!(missing_target("x").code(), codes::TOOLCHAIN);
}

#[test]
fn a_desktop_target_refuses_another_system_and_says_where_it_builds() {
    let cx = cx("system", "macos");
    let other = if cfg!(target_os = "macos") {
        "windows"
    } else {
        "macos"
    };
    let refused = find(other).unwrap().export(&cx).unwrap_err();
    assert_eq!(refused.code(), codes::TOOLCHAIN);
    assert!(refused.message().contains("let CI build it"), "{refused:?}");
    fs::remove_dir_all(&cx.out).unwrap();
}

/// A stand-in program in the export's folder.
fn program_file(cx: &ExportCx, name: &str) -> PathBuf {
    let path = cx.out.join("built").join(name);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, b"MZ a program").unwrap();
    path
}

#[test]
fn the_windows_package_is_a_zip_of_the_program_the_notices_and_the_players_note() {
    let cx = cx("windows", "windows");
    let program = program_file(&cx, "hello-grid.exe");
    let report = desktop::pack(&cx, System::Windows, "x86_64-pc-windows-gnu", &program).unwrap();
    assert_eq!(report.package, "hello-grid-0.1.0-windows.zip");
    assert_eq!(report.folder, "hello-grid-0.1.0-windows");
    assert_eq!(report.signing, Signing::None);
    assert_eq!(report.debug, None);
    let sizes: Vec<&str> = report.sizes.iter().map(|s| s.file.as_str()).collect();
    assert_eq!(
        sizes,
        [
            "hello-grid-0.1.0-windows.zip",
            "hello-grid-0.1.0-windows/hello-grid.exe"
        ]
    );
    let zip = fs::read(cx.out.join(&report.package)).unwrap();
    assert_eq!(report.sizes[0].bytes, zip.len() as u64);
    assert_eq!(
        archive::zip_names(&zip).unwrap(),
        [
            "hello-grid-0.1.0-windows/HOW-TO-OPEN.txt",
            "hello-grid-0.1.0-windows/NOTICES.txt",
            "hello-grid-0.1.0-windows/hello-grid.exe",
        ]
    );
    let folder = cx.out.join(&report.folder);
    assert_eq!(fs::read_to_string(folder.join(NOTE)).unwrap(), WINDOWS_NOTE);
    let notices = fs::read_to_string(folder.join(notices::FILE)).unwrap();
    for needle in [
        "Notices for Hello, Grid\n",
        "- macroquad ",
        "- gamecontrollerdb.txt, from SDL_GameControllerDB, compiled in by gilrs, Zlib",
        "This software is provided 'as-is'",
    ] {
        assert!(notices.contains(needle), "{needle} in {notices}");
    }
    fs::remove_dir_all(&cx.out).unwrap();
}

#[test]
fn what_ships_beside_goes_in_the_package_under_its_name_and_in_the_report() {
    let mut cx = cx("beside", "windows");
    cx.beside = vec![
        beside("capture", Some("plans"), false),
        beside("Cargo.toml", None, false),
    ];
    let program = program_file(&cx, "hello-grid.exe");
    let report = desktop::pack(&cx, System::Windows, "x86_64-pc-windows-gnu", &program).unwrap();
    let zip = fs::read(cx.out.join(&report.package)).unwrap();
    let names = archive::zip_names(&zip).unwrap();
    for name in ["Cargo.toml", "plans/frames.ron"] {
        let name = format!("hello-grid-0.1.0-windows/{name}");
        assert!(names.contains(&name), "{name} in {names:?}");
    }
    let shipped: Vec<&str> = report.beside.iter().map(|s| s.file.as_str()).collect();
    assert_eq!(
        shipped,
        [
            "hello-grid-0.1.0-windows/plans",
            "hello-grid-0.1.0-windows/Cargo.toml"
        ]
    );
    assert!(report.beside.iter().all(|size| size.bytes > 0));
    // The report's sizes stay the package, then its program.
    assert_eq!(report.sizes.len(), 2);
    let taken = ExportCx {
        beside: vec![beside("Cargo.toml", Some("NOTICES.txt"), false)],
        ..cx.clone()
    };
    let refused =
        desktop::pack(&taken, System::Windows, "x86_64-pc-windows-gnu", &program).unwrap_err();
    assert!(refused.message().contains("already holds"), "{refused:?}");
    fs::remove_dir_all(&cx.out).unwrap();
}

#[test]
fn the_linux_package_is_a_tar_gz_with_the_program_runnable_and_no_players_note() {
    let cx = cx("linux", "linux");
    let program = program_file(&cx, "hello-grid");
    let report = desktop::pack(&cx, System::Linux, "x86_64-unknown-linux-gnu", &program).unwrap();
    assert_eq!(report.package, "hello-grid-0.1.0-linux.tar.gz");
    let mut tar = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(cx.out.join(&report.package)).unwrap())
        .read_to_end(&mut tar)
        .unwrap();
    // Each header's name and mode, in order.
    let mut listed = Vec::new();
    let mut at = 0;
    while tar[at] != 0 {
        let name = String::from_utf8_lossy(&tar[at..at + 100])
            .trim_end_matches('\0')
            .to_owned();
        let mode = String::from_utf8_lossy(&tar[at + 100..at + 107]).into_owned();
        let size = usize::from_str_radix(
            String::from_utf8_lossy(&tar[at + 124..at + 135]).as_ref(),
            8,
        )
        .unwrap();
        listed.push(format!("{mode} {name}"));
        at += 512 + size.div_ceil(512) * 512;
    }
    assert_eq!(
        listed,
        [
            "0000644 hello-grid-0.1.0-linux/NOTICES.txt",
            "0000755 hello-grid-0.1.0-linux/hello-grid",
        ]
    );
    fs::remove_dir_all(&cx.out).unwrap();
}

#[test]
fn the_web_package_is_a_folder_and_its_zip_with_index_html_at_the_root() {
    let (cx, rules, dist) = web_build("web");
    let report = web::pack(&cx, &rules, &dist).unwrap();
    assert_eq!(report.package, "hello-grid-0.1.0-web.zip");
    let zip = fs::read(cx.out.join(&report.package)).unwrap();
    assert_eq!(report.sizes[0].bytes, zip.len() as u64);
    assert_eq!(report.sizes[1].file, "hello-grid-0.1.0-web/hello-grid.wasm");
    assert_eq!(archive::zip_names(&zip).unwrap(), WEB_FILES);
    assert!(cx.out.join("hello-grid-0.1.0-web/index.html").is_file());
    fs::remove_dir_all(&cx.out).unwrap();
}

/// What a web build of [`web_build`]'s game makes.
const WEB_FILES: [&str; 8] = [
    "NOTICES.txt",
    "gamepad.js",
    "hello-grid.wasm",
    "index.html",
    "mq_js_bundle.js",
    "music/a.ogg",
    "quad-storage.js",
    "sapp_jsutils.js",
];

/// A context on a game in the export's folder with a stand-in binary and
/// a track, built once with the web packager: the context, the web rules
/// and the build's folder.
fn web_build(name: &str) -> (ExportCx, Web, PathBuf) {
    let mut cx = cx(name, "web");
    cx.root = cx.out.join("game");
    cx.out = cx.out.join("export");
    let mut rules = cx.web.clone().unwrap();
    rules.music = Some("music".to_owned());
    fs::create_dir_all(cx.root.join("music")).unwrap();
    fs::write(cx.root.join("music/a.ogg"), "a").unwrap();
    let wasm = cx.root.join("target/wasm32-unknown-unknown/debug");
    fs::create_dir_all(&wasm).unwrap();
    fs::write(wasm.join("hello-grid.wasm"), "wasm").unwrap();
    build_web(&cx, &rules);
    let dist = cx.root.join(&rules.dist);
    (cx, rules, dist)
}

/// The web packager on `cx`'s game, in debug, with no works.
fn build_web(cx: &ExportCx, rules: &Web) {
    let target = cx.root.join("target");
    let options = crate::web::Options::default();
    crate::web::package(
        &crate::web::RealWasmOpt,
        rules,
        &cx.root,
        &target,
        &options,
        &[],
    )
    .unwrap();
}

/// A stale file in the build's folder, nested and not, is gone from the
/// second build, its zip and the published folder.
#[test]
fn a_second_web_export_holds_no_stale_file() {
    let (cx, rules, dist) = web_build("web-stale");
    web::pack(&cx, &rules, &dist).unwrap();
    fs::create_dir_all(dist.join("old")).unwrap();
    fs::write(dist.join("canary.txt"), "stale").unwrap();
    fs::write(dist.join("old/canary.txt"), "stale").unwrap();
    build_web(&cx, &rules);
    let report = web::pack(&cx, &rules, &dist).unwrap();
    let zip = fs::read(cx.out.join(&report.package)).unwrap();
    assert_eq!(archive::zip_names(&zip).unwrap(), WEB_FILES);
    assert!(!dist.join("canary.txt").exists());
    assert!(!cx.out.join(&report.folder).join("canary.txt").exists());
    assert!(!cx.out.join(&report.folder).join("old").exists());
    fs::remove_dir_all(cx.root.parent().unwrap()).unwrap();
}

/// A file that reached the build's folder after the build is refused by
/// its path, and neither the folder nor an older zip stays to be released.
#[test]
fn an_unexpected_file_in_the_web_export_is_refused_by_its_path() {
    let (cx, rules, dist) = web_build("web-unexpected");
    let report = web::pack(&cx, &rules, &dist).unwrap();
    fs::create_dir_all(dist.join("old")).unwrap();
    fs::write(dist.join("old/canary.txt"), "stale").unwrap();
    let refused = web::pack(&cx, &rules, &dist).unwrap_err();
    assert_eq!(refused.code(), codes::PACKAGE);
    assert!(refused.message().contains("old/canary.txt"), "{refused:?}");
    assert!(!cx.out.join(&report.package).exists());
    assert!(!cx.out.join(&report.folder).exists());
    fs::remove_dir_all(cx.root.parent().unwrap()).unwrap();
}

/// `web.dist` holding the export's folder would delete it each build.
#[test]
fn an_export_folder_inside_the_web_build_is_refused() {
    let mut cx = cx("web-nested", "web");
    let rules = cx.web.as_mut().unwrap();
    rules.dist = "dist".to_owned();
    cx.root = cx.out.clone();
    cx.out = cx.root.join("dist/export/web");
    let refused = web::WEB.export(&cx);
    // Where wasm32 isn't installed the export stops sooner, at the toolchain.
    if crate::export::installed(crate::web::TARGET).is_ok() {
        let refused = refused.unwrap_err();
        assert_eq!(refused.code(), codes::PROJECT_FILE, "{refused:?}");
        assert!(refused.message().contains("`web.dist`"), "{refused:?}");
    }
    fs::remove_dir_all(&cx.root).unwrap();
}

#[test]
fn the_players_notes_say_how_to_open_an_unsigned_program() {
    assert!(WINDOWS_NOTE.contains("More info, then Run\nanyway"));
    assert!(MACOS_NOTE.contains("Privacy & Security"));
    assert!(MACOS_NOTE.contains("Open Anyway"));
    let plist = desktop::info_plist("game");
    assert!(plist.contains("<key>CFBundleExecutable</key>\n\t<string>game</string>"));
    assert!(plist.contains("<key>CFBundlePackageType</key>\n\t<string>APPL</string>"));
}

#[test]
fn a_release_package_with_a_build_path_is_refused_by_name() {
    let mut cx = cx("search", "linux");
    let file = cx.out.join("program");
    fs::write(&file, format!("at {}/src/main.rs", cx.root.display())).unwrap();
    assert_eq!(cx.search(&file), Ok(()), "a debug build isn't searched");
    cx.release = true;
    let refused = cx.search(&file).unwrap_err();
    assert_eq!(refused.code(), codes::BUILD_PATH);
    fs::write(&file, "at hello-grid/src/main.rs and /cargo/registry").unwrap();
    assert_eq!(cx.search(&file), Ok(()));
    fs::remove_dir_all(&cx.out).unwrap();
}
