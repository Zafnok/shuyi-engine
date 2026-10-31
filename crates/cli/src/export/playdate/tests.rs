//! The Playdate exporter's commands and files, with no SDK.

use std::path::{Path, PathBuf};

use super::*;
use crate::export::find;

fn cx(release: bool) -> ExportCx {
    let root = PathBuf::from("/game");
    ExportCx {
        target: "playdate".to_owned(),
        profile: "playdate".to_owned(),
        workspace: root.clone(),
        target_dir: root.join("target"),
        out: root.join("dist/export/playdate"),
        shown: "dist/export/playdate".to_owned(),
        root,
        release,
        package: "moon-garden".to_owned(),
        bin: "moon-garden".to_owned(),
        version: "1.2.0".to_owned(),
        title: "Moon Garden".to_owned(),
        game: Vec::new(),
        web: None,
        app: None,
        features: vec![FEATURE.to_owned()],
    }
}

fn tools(simulator: Result<PathBuf, String>) -> Tools {
    Tools {
        sdk: PathBuf::from("/sdk"),
        arm_gcc: PathBuf::from("/bin/arm-none-eabi-gcc"),
        simulator,
        rustflags: ("device".to_owned(), "simulator".to_owned()),
    }
}

/// What each step of a plan is, in order.
fn names(steps: &[Step]) -> Vec<String> {
    steps
        .iter()
        .map(|step| match step {
            Step::Run { what, .. } => (*what).to_owned(),
            Step::Search(file) => format!(
                "search {}",
                file.file_name().unwrap_or_default().to_string_lossy()
            ),
        })
        .collect()
}

/// The arguments of the step that does `what`, as text.
fn args_of(steps: &[Step], what: &str) -> Vec<String> {
    steps
        .iter()
        .find_map(|step| match step {
            Step::Run {
                what: this, args, ..
            } if *this == what => Some(
                args.iter()
                    .map(|a| a.to_string_lossy().into_owned())
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no step {what}: {steps:?}"))
}

fn dirs() -> Dirs {
    Dirs {
        source: PathBuf::from("/build/Source"),
        obj: PathBuf::from("/build/obj"),
    }
}

#[test]
fn a_release_builds_the_device_and_the_simulator_then_packs_them() {
    let (cx, tools) = (cx(true), tools(Ok(PathBuf::from("/bin/gcc"))));
    let steps = plan(&cx, &tools, &dirs(), Path::new("/out/game.pdx"));
    assert_eq!(
        names(&steps),
        [
            "cargo rustc (device)",
            "compile shim.c (device)",
            "compile setup.c (device)",
            "link pdex.elf (device)",
            "search pdex.elf",
            "cargo rustc (Simulator)",
            "link pdex.dll (Simulator)",
            "search pdex.dll",
            "pdc",
        ]
    );
    let device = args_of(&steps, "cargo rustc (device)");
    assert_eq!(
        device,
        [
            "rustc",
            "-p",
            "moon-garden",
            "--lib",
            "--crate-type",
            "staticlib",
            "--no-default-features",
            "--features",
            "playdate",
            "--target",
            "thumbv7em-none-eabihf",
            "--release",
        ]
    );
    let pdc = args_of(&steps, "pdc");
    assert_eq!(pdc[..2], ["-sdkpath".to_owned(), "/sdk".to_owned()]);
    assert_eq!(pdc[3], "/out/game.pdx");
}

/// `pdc` crashes on a big unoptimized ELF of more than 65,280 sections.
#[test]
fn a_debug_build_drops_the_elfs_per_function_unwind_sections() {
    let steps = plan(
        &cx(false),
        &tools(Err("none".to_owned())),
        &dirs(),
        Path::new("/g.pdx"),
    );
    let names = names(&steps);
    let link = names.iter().position(|n| n == "link pdex.elf (device)");
    let drop = names
        .iter()
        .position(|n| n == "drop unwind index sections (device)");
    assert_eq!(drop, link.map(|at| at + 1), "{names:?}");
    let args = args_of(&steps, "drop unwind index sections (device)");
    assert_eq!(
        args[..2],
        [
            "--remove-section=.ARM.exidx*".to_owned(),
            "--remove-section=.rel.ARM.exidx*".to_owned()
        ]
    );
    assert!(args[2].ends_with("pdex.elf"), "{args:?}");
    assert!(!args_of(&steps, "cargo rustc (device)").contains(&"--release".to_owned()));
    let release = plan(
        &cx(true),
        &tools(Err("none".to_owned())),
        &dirs(),
        Path::new("/g.pdx"),
    );
    let release = self::names(&release);
    assert!(
        !release.iter().any(|n| n.starts_with("drop")),
        "{release:?}"
    );
}

#[test]
fn with_no_simulator_part_only_the_device_is_built() {
    let steps = plan(
        &cx(true),
        &tools(Err("no MinGW".to_owned())),
        &dirs(),
        Path::new("/g.pdx"),
    );
    assert!(!names(&steps).iter().any(|n| n.contains("Simulator")));
}

#[test]
fn the_device_build_keeps_nothing_of_the_machine_in_its_c() {
    let steps = plan(
        &cx(true),
        &tools(Err("none".to_owned())),
        &dirs(),
        Path::new("/g.pdx"),
    );
    for what in ["compile shim.c (device)", "compile setup.c (device)"] {
        let args = args_of(&steps, what);
        assert!(
            !args.iter().any(|a| a.starts_with("-g")),
            "{what}: {args:?}"
        );
    }
}

#[test]
fn a_build_aborts_on_panic_and_sets_its_flags() {
    let steps = plan(
        &cx(true),
        &tools(Err("none".to_owned())),
        &dirs(),
        Path::new("/g.pdx"),
    );
    let Some(Step::Run { envs, .. }) = steps.first() else {
        panic!("{steps:?}");
    };
    let get = |key: &str| envs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str());
    assert_eq!(get("CARGO_PROFILE_RELEASE_PANIC"), Some("abort"));
    assert_eq!(get("CARGO_PROFILE_RELEASE_LTO"), Some("fat"));
    assert_eq!(get(paths::ENCODED), Some("device"));
    assert_eq!(profile_env(false, "x")[0].0, "CARGO_PROFILE_DEV_PANIC");
    assert_eq!(with_flags("", &["-a"]), "-a");
    assert_eq!(
        with_flags("-x\x1f-y", &["-a", "-b"]),
        "-x\x1f-y\x1f-a\x1f-b"
    );
}

#[test]
fn the_exporter_is_found_by_its_target_and_checks_its_profile() {
    let exporter = find("playdate").unwrap_or_else(|| panic!());
    assert_eq!(exporter.profile(), "playdate");
    assert_eq!(exporter.triples(), [DEVICE, SIMULATOR]);
    assert!(!exporter.builds_a_program());
    assert_eq!(exporter.cargo_args(&cx(false)).len(), 2);
}

#[test]
fn a_package_with_no_playdate_feature_is_told_what_to_add() {
    let mut cx = cx(false);
    cx.features.clear();
    let project = Project::new(cx.root.clone(), crate::project::Config::default());
    let refused = PLAYDATE.prepare(&cx, &project, None).unwrap_err();
    assert!(
        refused.message().contains("no `playdate` feature"),
        "{refused:?}"
    );
    cx.features = vec![FEATURE.to_owned()];
    let refused = PLAYDATE.prepare(&cx, &project, None).unwrap_err();
    assert!(refused.message().contains("no `game` part"), "{refused:?}");
}

#[test]
fn a_game_with_no_app_part_is_told() {
    let refused = PLAYDATE.export(&cx(false)).unwrap_err();
    assert!(refused.message().contains("no `app` part"), "{refused:?}");
}

#[test]
fn the_sdk_is_looked_for_in_the_variable_then_the_home_folder() {
    let candidates = sdk_candidates(
        Some(OsString::from("/opt/sdk")),
        Some(PathBuf::from("/home/me")),
    );
    assert_eq!(
        candidates,
        [
            PathBuf::from("/opt/sdk"),
            Path::new("/home/me").join("Documents").join("PlaydateSDK"),
        ]
    );
    assert_eq!(
        sdk_candidates(Some(OsString::new()), None),
        [] as [PathBuf; 0]
    );
}

/// The setup steps of the ticket's *Nick input*.
#[test]
fn a_missing_sdk_says_how_to_install_it() {
    let refused = find_sdk(&[PathBuf::from("/no/such/sdk")]).unwrap_err();
    assert_eq!(refused.code(), crate::contract::codes::TOOLCHAIN);
    let message = refused.message();
    for words in [
        "/no/such/sdk",
        "https://play.date/dev/",
        "license",
        "PLAYDATE_SDK_PATH",
    ] {
        assert!(message.contains(words), "{words}: {message}");
    }
    let nowhere = find_sdk(&[]).unwrap_err();
    assert!(nowhere.message().contains("nowhere"), "{nowhere:?}");
}

#[test]
fn the_sdk_is_where_its_header_is() {
    let dir = std::env::temp_dir().join(format!("shuyi-playdate-sdk-{}", std::process::id()));
    fs::create_dir_all(dir.join("C_API")).unwrap_or_else(|e| panic!("{e}"));
    fs::write(dir.join("C_API").join("pd_api.h"), "").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        find_sdk(&[PathBuf::from("/no"), dir.clone()]).ok(),
        Some(dir.clone())
    );
    fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_pdxinfo_is_the_app_part() {
    let mut part = AppPart::new("Moon Garden", "com.example.moon-garden");
    part.publisher = "Example Games".to_owned();
    part.build = 7;
    assert_eq!(
        pdxinfo(&part, "1.2.0", true),
        "name=Moon Garden\nauthor=Example Games\nbundleID=com.example.moon-garden\n\
         version=1.2.0\nbuildNumber=7\nimagePath=launcher\n"
    );
    assert!(!pdxinfo(&part, "1.2.0", false).contains("imagePath"));
}

/// A checker of black and transparent, `side` a side.
fn checker(side: u32) -> Vec<u8> {
    (0..side * side)
        .flat_map(|i| {
            if (i + i / side).is_multiple_of(2) {
                [0, 0, 0, 255]
            } else {
                [255, 255, 255, 0]
            }
        })
        .collect()
}

#[test]
fn a_launchers_images_are_black_and_white_of_the_right_sizes() {
    let (width, height, rgba) =
        codec::decode_png(&card_from_icon(512, &checker(512)).unwrap_or_else(|e| panic!("{e}")))
            .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((width, height), CARD_SIZE);
    assert!(
        rgba.chunks(4)
            .all(|p| { p[3] == 255 && p[0] == p[1] && p[1] == p[2] && (p[0] == 0 || p[0] == 255) })
    );
    assert!(rgba[..4] == [255, 255, 255, 255], "white round the icon");
    let small = app_info::downscale(512, 512, &checker(512), ICON_SIDE);
    let icon =
        dithered(ICON_SIDE, small, OneBitSettings::default()).unwrap_or_else(|e| panic!("{e}"));
    let (width, height, _) = codec::decode_png(&icon).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((width, height), (ICON_SIDE, ICON_SIDE));
}

#[test]
fn a_part_with_no_pictures_has_no_launcher_images() {
    let part = AppPart::new("Moon Garden", "com.example.moon-garden");
    let images = launcher_images(&part, Path::new(".")).unwrap_or_else(|e| panic!("{e}"));
    assert!(images.is_empty(), "{images:?}");
}

#[test]
fn the_files_a_game_names_are_taken_as_they_are() {
    let dir = std::env::temp_dir().join(format!("shuyi-playdate-images-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    fs::write(dir.join("i.png"), "icon").unwrap_or_else(|e| panic!("{e}"));
    fs::write(dir.join("c.png"), "card").unwrap_or_else(|e| panic!("{e}"));
    let mut part = AppPart::new("Moon Garden", "com.example.moon-garden");
    part.playdate_icon = Some("i.png".to_owned());
    part.playdate_card = Some("c.png".to_owned());
    let images = launcher_images(&part, &dir).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(images["icon.png"], b"icon");
    assert_eq!(images["card.png"], b"card");
    part.playdate_card = Some("gone.png".to_owned());
    assert!(launcher_images(&part, &dir).is_err());
    fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_pack_is_where_the_backend_reads_it() {
    let backend = include_str!("../../../../playdate/src/sdk.rs");
    let line = format!("pub const PACK: &str = \"{}\";", playdate_pack());
    assert!(backend.contains(&line), "{line}");
}

#[test]
fn the_shim_is_the_backends() {
    assert!(SHIM.contains("eventHandler"));
    assert!(SHIM.contains("shuyi_playdate_update"));
}
