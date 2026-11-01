//! The gate on real packagers' output with a stand-in program: canaries in
//! raw packs, loose files, embedded strings, debug files and final archives,
//! and the same report for the same files.

use std::collections::BTreeMap as Files;

use proptest::prelude::*;
use shuyi_assets::pack::{self as assets_pack, Content, PackInfo};

use super::super::desktop::{self, System};
use super::super::tests::cx;
use super::super::{ExportReport, find as exporter, web};
use super::*;
use crate::project::{Config, Web};

/// A canary, in no file of the engine's but these tests'.
const CANARY: &str = "UNRELEASED-ACT-THREE";

/// The part in `shuyi.ron`'s words.
fn part(text: &str) -> Disclosure {
    Config::parse(&format!("(disclosure: {text})"))
        .unwrap()
        .disclosure
        .unwrap()
}

/// The part with [`CANARY`].
fn with_canary() -> Disclosure {
    part(&format!("(canaries: [{CANARY:?}])"))
}

/// A pack of `files`.
fn pack_of(files: &[(&str, &[u8])]) -> Vec<u8> {
    let info = PackInfo {
        stamp: "the game 1".to_owned(),
        ..PackInfo::default()
    };
    let files: Files<String, Content> = files
        .iter()
        .map(|(path, bytes)| ((*path).to_owned(), Content::Raw(bytes.to_vec())))
        .collect();
    assets_pack::write(&info, &files)
}

/// A stand-in program outside the export's folder, holding `extra`.
fn program(cx: &ExportCx, name: &str, extra: &[u8]) -> PathBuf {
    let dir = cx.out.with_extension("built");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    fs::write(&path, [b"MZ a program ", extra].concat()).unwrap();
    path
}

/// The Windows package of `extra` in the program, and what it shipped.
fn windows(name: &str, extra: &[u8]) -> (ExportCx, ExportReport, Shipped) {
    let cx = cx(name, "windows");
    let built = program(&cx, "hello-grid.exe", extra);
    let report = desktop::pack(&cx, System::Windows, "x86_64-pc-windows-gnu", &built).unwrap();
    let shipped = exporter("windows").unwrap().shipped(&cx, &report);
    (cx, report, shipped)
}

/// `inspect` with no build inputs.
fn inspected(cx: &ExportCx, part: Option<&Disclosure>, shipped: &Shipped) -> ExposureReport {
    inspect(
        &cx.out,
        part,
        shipped,
        MAX_FILE_BYTES,
        BuildInputs::default(),
    )
    .unwrap()
}

/// `(kind, path, entry)` of each finding.
fn kinds(report: &ExposureReport) -> Vec<(FindingKind, &str, Option<&str>)> {
    report
        .findings
        .iter()
        .map(|f| (f.kind, f.path.as_str(), f.entry.as_deref()))
        .collect()
}

fn clean(cx: &ExportCx) {
    let _ = fs::remove_dir_all(cx.out.with_extension("built"));
    let _ = fs::remove_dir_all(&cx.out);
}

#[test]
fn a_clean_desktop_package_passes_and_lists_every_file() {
    let (cx, _, shipped) = windows("disclosure-clean", b"");
    let report = inspected(&cx, Some(&with_canary()), &shipped);
    assert_eq!(report.findings, []);
    assert!(report.ok && report.declared);
    let listed: Vec<(&str, FileKind)> = report
        .files
        .iter()
        .map(|file| (file.path.as_str(), file.kind))
        .collect();
    assert_eq!(
        listed,
        [
            ("HOW-TO-OPEN.txt", FileKind::Written),
            ("NOTICES.txt", FileKind::Written),
            ("hello-grid-0.1.0-windows.zip", FileKind::Archive),
            ("hello-grid.exe", FileKind::Program),
        ]
    );
    clean(&cx);
}

#[test]
fn without_a_part_everything_is_listed_and_nothing_refused() {
    let (cx, _, shipped) = windows("disclosure-none", CANARY.as_bytes());
    fs::write(cx.out.join("future.pak"), "x").unwrap();
    let report = inspected(&cx, None, &shipped);
    assert!(report.ok && !report.declared);
    assert_eq!(report.findings, []);
    assert!(report.files.iter().any(|f| f.kind == FileKind::Beside));
    clean(&cx);
}

#[test]
fn a_canary_in_a_program_fails_as_utf8_and_as_utf16() {
    let wide: Vec<u8> = CANARY.encode_utf16().flat_map(u16::to_le_bytes).collect();
    for (name, extra) in [
        ("disclosure-canary-8", CANARY.as_bytes().to_vec()),
        ("disclosure-canary-16", wide),
    ] {
        let (cx, _, shipped) = windows(name, &extra);
        let report = inspected(&cx, Some(&with_canary()), &shipped);
        assert!(!report.ok);
        let finding = &report.findings[0];
        assert_eq!(finding.kind, FindingKind::Canary);
        assert_eq!(finding.path, "hello-grid.exe");
        assert_eq!(finding.source, "the built program");
        assert_eq!(finding.canary, Some(0));
        clean(&cx);
    }
}

#[test]
fn a_canary_in_a_debug_file_fails_and_the_debug_file_is_listed() {
    let (cx, _, mut shipped) = windows("disclosure-debug", b"");
    let debug = cx.out.join("hello-grid-0.1.0-windows.debug");
    fs::write(&debug, format!("\0line tables {CANARY}\0")).unwrap();
    shipped.debug = Some(debug);
    let report = inspected(&cx, Some(&with_canary()), &shipped);
    assert_eq!(
        kinds(&report),
        [(FindingKind::Canary, "hello-grid-0.1.0-windows.debug", None)]
    );
    let listed = report
        .files
        .iter()
        .find(|f| f.path == "hello-grid-0.1.0-windows.debug");
    assert_eq!(listed.unwrap().kind, FileKind::DebugCompanion);
    clean(&cx);
}

#[test]
fn a_build_input_is_read_for_what_a_compressed_program_hides() {
    let (cx, _, mut shipped) = windows("disclosure-input", b"");
    let elf = program(&cx, "pdex.elf", CANARY.as_bytes());
    shipped
        .inputs
        .push(("pdex.elf".to_owned(), "what it is".to_owned(), elf));
    // One a build didn't make isn't missed.
    let missing = cx.out.join("no-such-file");
    shipped
        .inputs
        .push(("pdex.dll".to_owned(), "what it is".to_owned(), missing));
    let report = inspected(&cx, Some(&with_canary()), &shipped);
    assert_eq!(kinds(&report), [(FindingKind::Canary, "pdex.elf", None)]);
    assert_eq!(report.findings[0].source, "what it is");
    clean(&cx);
}

#[test]
fn a_file_added_to_the_folder_fails_by_its_source_and_path_and_differs_from_the_archive() {
    let (cx, report_, shipped) = windows("disclosure-loose", b"");
    fs::write(cx.out.join(&report_.folder).join("future.txt"), "soon").unwrap();
    let report = inspected(&cx, Some(&with_canary()), &shipped);
    assert_eq!(
        kinds(&report),
        [
            (FindingKind::UndeclaredFile, "future.txt", None),
            (FindingKind::ArchiveDiffers, "future.txt", None),
        ]
    );
    assert_eq!(report.findings[0].source, NOT_MADE);
    // Allowed, it is the game's file; it still isn't in the archive.
    let allowed = part(r#"(allowed_files: ["future.txt"])"#);
    let report = inspected(&cx, Some(&allowed), &shipped);
    assert_eq!(
        kinds(&report),
        [(FindingKind::ArchiveDiffers, "future.txt", None)]
    );
    clean(&cx);
}

#[test]
fn a_canary_in_the_archive_alone_fails() {
    let (cx, report_, shipped) = windows("disclosure-archive", b"");
    let inside = cx.out.join("inside");
    fs::create_dir_all(&inside).unwrap();
    fs::write(inside.join("hello-grid.exe"), CANARY).unwrap();
    let entries = archive::entries(&inside, "hello-grid-0.1.0-windows/", &|_| false).unwrap();
    archive::zip(&cx.out.join(&report_.package), &entries).unwrap();
    fs::remove_dir_all(&inside).unwrap();
    let report = inspected(&cx, Some(&with_canary()), &shipped);
    let found = kinds(&report);
    // The folder's other files are gone from it too.
    for expected in [
        (FindingKind::Canary, "hello-grid.exe", None),
        (FindingKind::ArchiveDiffers, "hello-grid.exe", None),
        (FindingKind::ArchiveDiffers, "NOTICES.txt", None),
    ] {
        assert!(found.contains(&expected), "{expected:?} in {found:?}");
    }
    clean(&cx);
}

#[test]
fn a_linux_tar_gz_is_read_like_a_zip() {
    let cx = cx("disclosure-tar", "linux");
    let built = program(&cx, "hello-grid", CANARY.as_bytes());
    let report_ = desktop::pack(&cx, System::Linux, "x86_64-unknown-linux-gnu", &built).unwrap();
    let shipped = exporter("linux").unwrap().shipped(&cx, &report_);
    let report = inspected(&cx, Some(&with_canary()), &shipped);
    assert_eq!(
        kinds(&report),
        [(FindingKind::Canary, "hello-grid", None)],
        "the folder and the archive agree"
    );
    assert!(report.files.iter().any(|f| f.kind == FileKind::Archive));
    clean(&cx);
}

#[test]
fn a_file_beside_the_package_fails_with_where_it_is() {
    let (cx, _, shipped) = windows("disclosure-beside", b"");
    fs::create_dir_all(cx.out.join("extras")).unwrap();
    fs::write(cx.out.join("extras/future.pak"), CANARY).unwrap();
    let report = inspected(&cx, Some(&with_canary()), &shipped);
    assert_eq!(
        kinds(&report),
        [
            (FindingKind::Canary, "extras/future.pak", None),
            (FindingKind::Beside, "extras/future.pak", None),
        ]
    );
    assert_eq!(report.findings[1].source, NOT_MADE);
    // The export's own reports are no such file.
    fs::remove_dir_all(cx.out.join("extras")).unwrap();
    fs::write(cx.out.join(REPORT), "{}").unwrap();
    fs::write(cx.out.join(EXPORT_REPORT), "{}").unwrap();
    assert_eq!(inspected(&cx, Some(&with_canary()), &shipped).findings, []);
    clean(&cx);
}

/// A folder that a pack file was added to, as `pdc` leaves a `.pdx`.
fn pack_package(name: &str, pack: &[u8]) -> (ExportCx, Shipped) {
    let cx = cx(name, "playdate");
    let tree = cx.out.join("game.pdx");
    fs::create_dir_all(&tree).unwrap();
    fs::write(tree.join("game.pack"), pack).unwrap();
    let mut shipped = Shipped::new(tree, None, "");
    shipped.source("game.pack", FileKind::Pack, "the game's bundle, baked");
    (cx, shipped)
}

#[test]
fn a_pack_entry_is_listed_and_fails_unless_it_is_allowed() {
    let pack = pack_of(&[("maps/first.ron", b"a map"), ("maps/future.ron", b"soon")]);
    let (cx, shipped) = pack_package("disclosure-pack", &pack);
    let manifest = part(r#"(allowed_pack_entries: ["maps/first.ron"])"#);
    let report = inspected(&cx, Some(&manifest), &shipped);
    assert_eq!(
        kinds(&report),
        [(
            FindingKind::UndeclaredPackEntry,
            "game.pack",
            Some("maps/future.ron")
        )]
    );
    assert_eq!(report.findings[0].source, "the game's bundle, baked");
    let listed: Vec<&str> = report
        .pack_entries
        .iter()
        .map(|e| e.path.as_str())
        .collect();
    assert_eq!(listed, ["maps/first.ron", "maps/future.ron"]);
    assert_eq!(
        report.pack_entries[0].sha256,
        sha256::hex(b"a map"),
        "the hash is of what the file unpacks to"
    );
    let both = part(r#"(allowed_pack_entries: ["maps/first.ron", "maps/future.ron"])"#);
    assert_eq!(inspected(&cx, Some(&both), &shipped).findings, []);
    clean(&cx);
}

#[test]
fn a_canary_a_pack_compresses_is_found_in_what_it_unpacks_to() {
    // Its one first copy runs across what LZ4 keeps and what it refers back to.
    let canary = "THREE-ACT-THREE";
    let text = "ACT-THREE-".repeat(50);
    let pack = pack_of(&[("story.txt", text.as_bytes())]);
    assert!(
        find(&pack, canary.as_bytes(), 0).is_none(),
        "the pack keeps it compressed"
    );
    let (cx, shipped) = pack_package("disclosure-pack-canary", &pack);
    let manifest = part(&format!(
        r#"(allowed_pack_entries: ["story.txt"], canaries: [{canary:?}])"#
    ));
    let report = inspected(&cx, Some(&manifest), &shipped);
    assert_eq!(
        kinds(&report),
        [(FindingKind::Canary, "game.pack", Some("story.txt"))]
    );
    clean(&cx);
}

#[test]
fn a_pack_embedded_in_a_program_is_read_and_two_are_told_apart() {
    let first = pack_of(&[("maps/first.ron", b"a map")]);
    let second = pack_of(&[("maps/future.ron", b"soon")]);
    let inside = [&b"code "[..], &first, b" more code ", &second, b" the end"].concat();
    let (cx, _, shipped) = windows("disclosure-embedded", &inside);
    let manifest = part(r#"(allowed_pack_entries: ["maps/first.ron"])"#);
    let report = inspected(&cx, Some(&manifest), &shipped);
    assert_eq!(
        kinds(&report),
        [(
            FindingKind::UndeclaredPackEntry,
            "hello-grid.exe",
            Some("maps/future.ron")
        )]
    );
    let offsets: Vec<u64> = report.pack_entries.iter().map(|e| e.offset).collect();
    assert!(offsets[0] > 0 && offsets[1] > offsets[0], "{offsets:?}");
    clean(&cx);
}

#[test]
fn a_pack_that_doesnt_read_fails_and_the_magic_bytes_alone_dont() {
    let mut broken = pack_of(&[("a.txt", &[9; 200])]);
    let last = broken.len() - 1;
    broken[last] ^= 0xff;
    let (cx, shipped) = pack_package("disclosure-broken", &broken);
    let report = inspected(&cx, Some(&part("()")), &shipped);
    assert_eq!(
        kinds(&report),
        [(FindingKind::UnreadablePack, "game.pack", None)]
    );
    clean(&cx);
    // In a program, the constant that reads packs is no pack.
    let (cx, _, shipped) = windows("disclosure-magic", b"SHUYIPAK and the rest of the program");
    assert_eq!(inspected(&cx, Some(&part("()")), &shipped).findings, []);
    clean(&cx);
}

#[test]
fn a_forbidden_path_fails_as_a_file_as_a_pack_entry_and_as_a_programs_text() {
    let manifest = part(
        r#"(forbidden_paths: ["maps/future.ron"], allowed_files: ["maps/first.ron"],
            allowed_pack_entries: ["maps/first.ron"])"#,
    );
    // As the text `include_dir!` keeps in a program, with either slash.
    for (name, text) in [
        ("disclosure-path-text", &b"maps/future.ron"[..]),
        ("disclosure-path-back", b"maps\\future.ron"),
    ] {
        let (cx, _, shipped) = windows(name, text);
        let report = inspected(&cx, Some(&manifest), &shipped);
        assert_eq!(
            kinds(&report),
            [(FindingKind::ForbiddenPath, "hello-grid.exe", None)]
        );
        clean(&cx);
    }
    let pack = pack_of(&[("maps/future.ron", b"soon")]);
    let (cx, shipped) = pack_package("disclosure-path-pack", &pack);
    let found = inspected(&cx, Some(&manifest), &shipped);
    assert!(kinds(&found).contains(&(
        FindingKind::ForbiddenPath,
        "game.pack",
        Some("maps/future.ron")
    )));
    // A file there; a text that mentions the path is only a mention.
    let tree = shipped.tree.clone();
    fs::create_dir_all(tree.join("maps")).unwrap();
    fs::write(tree.join("maps/future.ron"), "soon").unwrap();
    fs::write(tree.join("maps/first.ron"), "maps/future.ron").unwrap();
    let found = inspected(&cx, Some(&manifest), &shipped);
    let found = kinds(&found);
    assert!(found.contains(&(FindingKind::ForbiddenPath, "maps/future.ron", None)));
    assert!(!found.iter().any(|(_, path, _)| *path == "maps/first.ron"));
    clean(&cx);
}

#[test]
fn a_file_over_the_limit_fails_unread() {
    let (cx, _, shipped) = windows("disclosure-big", b"");
    let report = inspect(
        &cx.out,
        Some(&part("()")),
        &shipped,
        4,
        BuildInputs::default(),
    );
    // The archive unpacks past it too: it can't be read at all.
    let error = report.unwrap_err();
    assert!(error.contains("over 4 bytes unpacked"), "{error}");
    let mut folder_only = shipped.clone();
    folder_only.archive = None;
    let report = inspect(
        &cx.out,
        Some(&part("()")),
        &folder_only,
        4,
        BuildInputs::default(),
    )
    .unwrap();
    assert!(
        kinds(&report).contains(&(FindingKind::TooBig, "hello-grid.exe", None)),
        "{report:?}"
    );
    clean(&cx);
}

#[test]
fn a_line_a_tool_writes_each_build_changes_no_hash_in_the_report() {
    let hashed = |time: &str| {
        let cx = cx("disclosure-varying", "playdate");
        let tree = cx.out.join("game.pdx");
        fs::create_dir_all(&tree).unwrap();
        let info = format!("name=A game\nbuildtime={time}\nversion=1\n");
        fs::write(tree.join("pdxinfo"), info).unwrap();
        let archive_file = cx.out.join("game.pdx.zip");
        let entries = archive::entries(&tree, "game.pdx/", &|_| false).unwrap();
        archive::zip(&archive_file, &entries).unwrap();
        let mut shipped = Shipped::new(tree, Some(archive_file), "game.pdx/");
        shipped.source("pdxinfo", FileKind::Written, "written by the export");
        shipped
            .varying
            .insert("pdxinfo".to_owned(), "buildtime=".to_owned());
        let report = inspected(&cx, Some(&part("()")), &shipped);
        clean(&cx);
        report
    };
    let first = hashed("846825562");
    assert_eq!(first, hashed("846825599"));
    assert_eq!(first.findings, []);
    let info = first.files.iter().find(|f| f.path == "pdxinfo").unwrap();
    assert_eq!(info.sha256, sha256::hex(b"name=A game\nversion=1\n"));
    assert_eq!(
        info.varies.as_deref(),
        Some("the line that starts `buildtime=`")
    );
    let zip = first.files.iter().find(|f| f.kind == FileKind::Archive);
    assert_eq!(
        zip.unwrap().sha256,
        sha256::hex(format!("{}  pdxinfo\n", info.sha256).as_bytes())
    );
    assert!(zip.unwrap().varies.is_some());
    assert_eq!(zip.unwrap().bytes, info.bytes, "its files, unpacked");
}

#[test]
fn a_refused_export_leaves_the_report_and_no_package() {
    let (cx, report_, shipped) = windows("disclosure-gate", CANARY.as_bytes());
    let debug = cx.out.join("hello-grid-0.1.0-windows.debug");
    fs::write(&debug, "tables").unwrap();
    fs::write(cx.out.join(EXPORT_REPORT), "the last export").unwrap();
    let mut shipped = shipped;
    shipped.debug = Some(debug.clone());
    let refused = gate(
        &cx,
        exporter("windows").unwrap(),
        Some(&with_canary()),
        &shipped,
    )
    .unwrap_err();
    assert_eq!(refused.0, codes::DISCLOSURE);
    assert!(
        refused
            .1
            .contains("hello-grid.exe (the built program) holds disclosure.canaries[0]"),
        "{}",
        refused.1
    );
    assert!(!refused.1.contains(CANARY), "{}", refused.1);
    assert!(!cx.out.join(&report_.package).exists());
    assert!(!cx.out.join(&report_.folder).exists());
    assert!(!debug.exists());
    assert!(!cx.out.join(EXPORT_REPORT).exists());
    let written = fs::read_to_string(cx.out.join(REPORT)).unwrap();
    assert!(!written.contains(CANARY), "{written}");
    let report: ExposureReport = serde_json::from_str(&written).unwrap();
    assert!(!report.ok);
    assert_eq!(report.inputs.target, "windows");
    clean(&cx);
}

#[test]
fn a_passing_gate_leaves_the_package_and_a_report_of_its_build_inputs() {
    let (cx, report_, shipped) = windows("disclosure-pass", b"");
    let target = exporter("windows").unwrap();
    gate(&cx, target, Some(&with_canary()), &shipped).unwrap();
    let first = fs::read_to_string(cx.out.join(REPORT)).unwrap();
    let report: ExposureReport = serde_json::from_str(&first).unwrap();
    assert!(report.ok);
    assert!(cx.out.join(&report_.package).is_file());
    let inputs = &report.inputs;
    assert_eq!(
        (inputs.package.as_str(), inputs.release),
        ("hello-grid", false)
    );
    assert!(inputs.default_features);
    assert!(inputs.features.iter().any(|f| f == "default"), "{inputs:?}");
    assert!(inputs.toolchain.starts_with("rustc "), "{inputs:?}");
    assert!(inputs.project_file.is_some() && inputs.cargo_lock.is_some());
    // No path of this machine goes in, and a second run writes the same.
    assert!(!first.contains(&*cx.out.to_string_lossy()));
    assert!(!first.contains("Users") && !first.contains("home/"));
    gate(&cx, target, Some(&with_canary()), &shipped).unwrap();
    assert_eq!(fs::read_to_string(cx.out.join(REPORT)).unwrap(), first);
    clean(&cx);
}

#[test]
fn a_web_export_keeps_its_shell_declares_its_music_and_empties_dist_when_refused() {
    let root = std::env::temp_dir().join(format!("shuyi-disclosure-web-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    let dist = root.join("dist/web");
    fs::create_dir_all(dist.join("music")).unwrap();
    fs::create_dir_all(root.join("music")).unwrap();
    fs::write(root.join("music/future.ogg"), "ogg").unwrap();
    let mut cx = cx("disclosure-web", "web");
    let rules = Web {
        package: "hello-grid".to_owned(),
        bin: "hello-grid".to_owned(),
        title: "A game".to_owned(),
        music: Some("music".to_owned()),
        dist: "dist/web".to_owned(),
        ..Web::default()
    };
    cx.web = Some(rules.clone());
    cx.root.clone_from(&root);
    // What `shuyi web` leaves, without its build.
    fs::write(root.join("Cargo.toml"), "").unwrap();
    for file in crate::web::current_files(&rules, &root).unwrap() {
        fs::write(dist.join(&file), "the file").unwrap();
    }
    let report_ = web::pack(&cx, &rules, &dist).unwrap();
    let target = exporter("web").unwrap();
    let shipped = target.shipped(&cx, &report_);
    let allowed = part(r#"(allowed_files: ["music/future.ogg"])"#);
    assert_eq!(inspected(&cx, Some(&allowed), &shipped).findings, []);
    let report = inspected(&cx, Some(&part("()")), &shipped);
    assert_eq!(
        kinds(&report),
        [(FindingKind::UndeclaredFile, "music/future.ogg", None)]
    );
    assert_eq!(report.findings[0].source, "web.music: music/future.ogg");
    let wasm = report.files.iter().find(|f| f.path == "hello-grid.wasm");
    assert_eq!(wasm.unwrap().kind, FileKind::Program);
    remove_package(&cx.out, &shipped);
    assert!(!cx.out.join(&report_.package).exists());
    assert_eq!(fs::read_dir(&dist).unwrap().count(), 0, "emptied, not gone");
    clean(&cx);
    fs::remove_dir_all(&root).unwrap();
}

#[test]
fn every_target_says_what_it_ships() {
    let mut cx = cx("disclosure-targets", "macos");
    cx.web = None;
    let report = ExportReport {
        package: "g".to_owned(),
        folder: "g".to_owned(),
        ..ExportReport::default()
    };
    let shipped = exporter("macos").unwrap().shipped(&cx, &report);
    assert_eq!(shipped.archive, None, "the macOS package is its folder");
    for file in [
        "NOTICES.txt",
        "HOW-TO-OPEN.txt",
        "hello-grid.app/Contents/Info.plist",
        "hello-grid.app/Contents/MacOS/hello-grid",
        "hello-grid.app/Contents/_CodeSignature/CodeResources",
    ] {
        assert!(shipped.sources.contains_key(file), "{file}");
    }
    let linux = exporter("linux").unwrap().shipped(&cx, &report);
    assert!(!linux.sources.contains_key("HOW-TO-OPEN.txt"));
    let web_ = exporter("web").unwrap().shipped(&cx, &report);
    assert!(web_.sources.is_empty(), "no web part, nothing known");
    let report = ExportReport {
        package: "g.pdx".to_owned(),
        folder: "g".to_owned(),
        ..report
    };
    let playdate = exporter("playdate").unwrap().shipped(&cx, &report);
    assert_eq!(playdate.tree, cx.out.join("g.pdx"));
    assert_eq!(playdate.archive, Some(cx.out.join("g.pdx.zip")));
    assert_eq!(playdate.prefix, "g.pdx/");
    assert_eq!(playdate.inputs[0].0, "pdex.elf");
    clean(&cx);
}

#[test]
fn the_parts_lines_that_cant_match_are_refused_by_name() {
    assert_eq!(problems(&part("()")), Vec::<String>::new());
    let cases = [
        (r#"(allowed_files: ["a\\b.ron"])"#, "write it with `/`"),
        (r#"(allowed_files: ["maps/*.ron"])"#, "with no pattern"),
        (
            r#"(forbidden_paths: ["/a.ron"])"#,
            "from the package's root",
        ),
        (
            r#"(allowed_pack_entries: ["C:/a.ron"])"#,
            "from the package's root",
        ),
        (r#"(allowed_files: ["a/../b.ron"])"#, "`..`"),
        (r#"(allowed_files: [""])"#, "an empty part"),
        (r#"(allowed_files: ["a.ron", "a.ron"])"#, "listed twice"),
        (
            r#"(allowed_files: ["a.ron"], forbidden_paths: ["a.ron"])"#,
            "in both disclosure.allowed_files and disclosure.forbidden_paths",
        ),
        (r#"(canaries: ["short"])"#, "canaries[0]: under 6 bytes"),
        (
            r#"(canaries: ["long enough", "long enough"])"#,
            "canaries[1]: listed twice",
        ),
    ];
    for (text, expected) in cases {
        let found = problems(&part(text));
        assert!(
            found.iter().any(|p| p.contains(expected)),
            "{text}: {found:?}"
        );
    }
    let long = "x".repeat(MAX_CANARY + 1);
    let found = problems(&part(&format!("(canaries: [{long:?}])")));
    assert_eq!(found, ["disclosure.canaries[0]: over 256 bytes"]);
    // The texts stay out of what is printed.
    assert!(!problems(&part(r#"(canaries: ["short"])"#))[0].contains("short\""));
    assert!(Config::parse("(disclosure: (allow: []))").is_err());
}

#[test]
fn the_features_are_those_asked_with_what_they_turn_on() {
    let map: BTreeMap<String, Vec<String>> = [
        ("default".to_owned(), vec!["a".to_owned()]),
        ("a".to_owned(), vec!["b".to_owned(), "dep:x".to_owned()]),
        ("b".to_owned(), vec!["a".to_owned()]),
        ("c".to_owned(), Vec::new()),
    ]
    .into();
    assert_eq!(
        resolved_features(&map, true, &[]),
        ["a", "b", "default", "dep:x"]
    );
    assert_eq!(resolved_features(&map, false, &["c"]), ["c"]);
    assert_eq!(resolved_features(&BTreeMap::new(), true, &[]), [""; 0]);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// However the files came to be on disk, the same files give the same
    /// report, byte for byte, sorted by path.
    #[test]
    fn the_same_files_give_the_same_report(
        files in proptest::collection::btree_map("[a-z]{1,6}(/[a-z]{1,6})?", any::<Vec<u8>>(), 0..8),
    ) {
        // A path that is also a folder's name can't be written.
        let names: Vec<&String> = files.keys().collect();
        let files: Files<&String, &Vec<u8>> = files
            .iter()
            .filter(|(name, _)| {
                !names.iter().any(|other| other.starts_with(&format!("{name}/")))
            })
            .collect();
        let dir = std::env::temp_dir()
            .join(format!("shuyi-disclosure-same-{}", std::process::id()));
        let write = |tree: &Path, reversed: bool| {
            let _ = fs::remove_dir_all(tree);
            fs::create_dir_all(tree).unwrap();
            let mut listed: Vec<_> = files.iter().collect();
            if reversed {
                listed.reverse();
            }
            for (name, bytes) in listed {
                let path = tree.join(name.as_str());
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, bytes).unwrap();
            }
        };
        let tree = dir.join("package");
        let report = |reversed: bool| {
            write(&tree, reversed);
            let shipped = Shipped::new(tree.clone(), None, "");
            let report =
                inspect(&dir, Some(&part("()")), &shipped, MAX_FILE_BYTES, BuildInputs::default())
                    .unwrap();
            serde_json::to_string(&report).unwrap()
        };
        let first = report(false);
        prop_assert_eq!(&first, &report(true));
        let read: ExposureReport = serde_json::from_str(&first).unwrap();
        let paths: Vec<&String> = read.files.iter().map(|file| &file.path).collect();
        let mut sorted = paths.clone();
        sorted.sort();
        prop_assert_eq!(paths, sorted);
        let _ = fs::remove_dir_all(&dir);
    }
}
