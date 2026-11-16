//! The seed on temp folders: what is linked and what copied, the times
//! kept, the root rewritten, the path packages left out, and each refusal.

use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::*;

/// The time every donor file and folder is given, so a kept time is told
/// apart from the time of the copy.
fn old() -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(1_600_000_000)
}

/// A fresh folder for one test, holding a `donor` and an `into` worktree.
fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-seed-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("donor")).unwrap();
    fs::create_dir_all(dir.join("into")).unwrap();
    dir
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// The donor's `target/debug/` of a small workspace: a third-party crate
/// `dep` with a build script, and the path packages `shuyi` and `game`,
/// with folders outside what is seeded. Every file and folder is [`old`].
fn donor(root: &Path) -> PathBuf {
    let profile = root.join("target").join("debug");
    let root_text = root.display().to_string();
    let files = [
        ("deps/libdep-0123abcd.rlib", "rlib".to_owned()),
        ("deps/libdep-0123abcd.rmeta", "rmeta".to_owned()),
        ("deps/dep-0123abcd.d", "dep info".to_owned()),
        ("deps/tool-0123abcd.exe", "exe".to_owned()),
        ("deps/macro-0123abcd.dll", "dll".to_owned()),
        ("deps/libnative.a", "archive".to_owned()),
        ("deps/dep-0123abcd.dep.0.rcgu.o", "object".to_owned()),
        ("deps/native.lib", "import library".to_owned()),
        ("deps/libshuyi-89ab.rlib", "the donor's shuyi".to_owned()),
        (
            "build/dep-4567cdef/build-script-build.exe",
            "script".to_owned(),
        ),
        (
            "build/dep-4567cdef/build_script_build-4567cdef.d",
            "dep info".to_owned(),
        ),
        (
            "build/dep-89abcdef/output",
            format!(
                "cargo:rustc-link-search=native={root_text}/target/debug/build/dep-89abcdef/out\ncargo:rerun-if-changed={root_text}-old/file.txt\n"
            ),
        ),
        (
            "build/dep-89abcdef/root-output",
            format!("{root_text}\\target\\debug\\build\\dep-89abcdef\\out"),
        ),
        ("build/dep-89abcdef/stderr", String::new()),
        (
            "build/dep-89abcdef/out/generated.rs",
            "pub const GENERATED: u8 = 1;".to_owned(),
        ),
        (".fingerprint/dep-0123abcd/lib-dep", "0123".to_owned()),
        (".fingerprint/dep-0123abcd/lib-dep.json", "{}".to_owned()),
        (
            ".fingerprint/dep-0123abcd/invoked.timestamp",
            "This file has an mtime".to_owned(),
        ),
        (
            ".fingerprint/shuyi-0123456789abcdef/lib-shuyi",
            "1".to_owned(),
        ),
        (
            ".fingerprint/shuyi-core-0123456789abcdef/lib-shuyi-core",
            "1".to_owned(),
        ),
        (
            ".fingerprint/game-fedcba9876543210/bin-game",
            "1".to_owned(),
        ),
        (
            ".fingerprint/game-extra-fedcba9876543210/lib-game-extra",
            "1".to_owned(),
        ),
        (
            "incremental/shuyi-1/s-1/dep-graph.bin",
            "incremental".to_owned(),
        ),
        ("examples/hello.exe", "example".to_owned()),
        ("shuyi.exe", "uplifted".to_owned()),
        (".cargo-lock", String::new()),
    ];
    for (file, text) in files {
        write(&profile.join(file), &text);
    }
    write(
        &root.join("target/gates/games/debug/deps/libgame.rlib"),
        "gate",
    );
    make_old(&profile);
    profile
}

/// Sets every file and folder under `dir`, and `dir`, to [`old`].
fn make_old(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            make_old(&path);
        } else {
            set_modified(&path, old()).unwrap();
        }
    }
    set_modified(dir, old()).unwrap();
}

fn packages() -> Vec<String> {
    vec![
        "game".to_owned(),
        "shuyi".to_owned(),
        "shuyi-core".to_owned(),
    ]
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

/// Appends to `path` (in the seeded target), and answers whether the
/// donor's `donor` changed with it: the two are one file, linked.
fn shares(path: &Path, donor: &Path) -> bool {
    let before = fs::read(donor).unwrap();
    File::options()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(b"+")
        .unwrap();
    fs::read(donor).unwrap() != before
}

#[test]
fn compiled_output_is_linked_and_the_rest_copied() {
    let dir = temp("links");
    let from = donor(&dir.join("donor"));
    let report = seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap();
    let to = dir.join("into/target/debug");
    for linked in [
        "deps/libdep-0123abcd.rlib",
        "deps/libdep-0123abcd.rmeta",
        "deps/tool-0123abcd.exe",
        "deps/macro-0123abcd.dll",
        "deps/libnative.a",
        "deps/dep-0123abcd.dep.0.rcgu.o",
        "deps/native.lib",
        "build/dep-4567cdef/build-script-build.exe",
    ] {
        assert!(
            shares(&to.join(linked), &from.join(linked)),
            "{linked} is not linked"
        );
    }
    for copied in [
        "deps/dep-0123abcd.d",
        "build/dep-4567cdef/build_script_build-4567cdef.d",
        "build/dep-89abcdef/output",
        "build/dep-89abcdef/stderr",
        "build/dep-89abcdef/out/generated.rs",
        ".fingerprint/dep-0123abcd/lib-dep",
        ".fingerprint/dep-0123abcd/lib-dep.json",
        ".fingerprint/dep-0123abcd/invoked.timestamp",
    ] {
        assert!(to.join(copied).is_file(), "{copied} is missing");
        assert!(
            !shares(&to.join(copied), &from.join(copied)),
            "{copied} is linked"
        );
    }
    assert_eq!(report.linked, 9, "{report}");
    assert_eq!(report.copied, 10, "{report}");
    assert_eq!(
        report.linked_bytes,
        "rlibrmetaexedllarchiveobjectimport librarythe donor's shuyiscript".len() as u64
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn only_the_debug_profiles_three_folders_are_seeded() {
    let dir = temp("folders");
    donor(&dir.join("donor"));
    seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap();
    let target = dir.join("into/target");
    let mut top: Vec<String> = fs::read_dir(&target)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    top.sort();
    assert_eq!(top, ["debug"]);
    let mut profile: Vec<String> = fs::read_dir(target.join("debug"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    profile.sort();
    // The lock files are the seed's own, held while it seeds, as cargo
    // makes them.
    assert_eq!(
        profile,
        [
            ".cargo-artifact-lock",
            ".cargo-build-lock",
            ".cargo-lock",
            ".fingerprint",
            "build",
            "deps"
        ]
    );
    assert_eq!(
        fs::read(target.join("debug/.cargo-lock")).unwrap(),
        b"",
        "nothing of the donor's"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn files_and_folders_keep_their_modification_times() {
    let dir = temp("times");
    donor(&dir.join("donor"));
    seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap();
    let to = dir.join("into/target/debug");
    for path in [
        "",
        "deps",
        "build",
        "build/dep-89abcdef",
        "build/dep-89abcdef/out",
        "build/dep-89abcdef/out/generated.rs",
        "build/dep-89abcdef/output",
        "build/dep-89abcdef/root-output",
        ".fingerprint",
        ".fingerprint/dep-0123abcd",
        ".fingerprint/dep-0123abcd/invoked.timestamp",
        "deps/dep-0123abcd.d",
        "deps/libdep-0123abcd.rlib",
    ] {
        assert_eq!(modified(&to.join(path)), old(), "{path:?}");
    }
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_build_scripts_output_names_the_new_root() {
    let dir = temp("rewrite");
    donor(&dir.join("donor"));
    let into = dir.join("into");
    let report = seed(&dir.join("donor"), &into, &packages()).unwrap();
    let unit = into.join("target/debug/build/dep-89abcdef");
    let donor = dir.join("donor").display().to_string();
    let output = fs::read_to_string(unit.join("output")).unwrap();
    // The fixture writes the root as the platform does, and the new root
    // keeps the separator the match was written with.
    let new = std::path::absolute(&into).unwrap().display().to_string();
    assert_eq!(
        output,
        format!(
            "cargo:rustc-link-search=native={new}/target/debug/build/dep-89abcdef/out\n\
             cargo:rerun-if-changed={donor}-old/file.txt\n"
        )
    );
    let root_output = fs::read_to_string(unit.join("root-output")).unwrap();
    assert_eq!(
        root_output,
        format!("{new}\\target\\debug\\build\\dep-89abcdef\\out")
    );
    assert_eq!(report.rewritten, 2);
    assert_eq!(
        fs::read_to_string(unit.join("out/generated.rs")).unwrap(),
        "pub const GENERATED: u8 = 1;",
        "only output and root-output are rewritten"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_path_packages_fingerprints_are_left_out_and_the_others_kept() {
    let dir = temp("fingerprints");
    donor(&dir.join("donor"));
    let report = seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap();
    let fingerprints = dir.join("into/target/debug/.fingerprint");
    let mut kept: Vec<String> = fs::read_dir(&fingerprints)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    kept.sort();
    // `game-extra` is a third-party crate whose name begins with `game`.
    assert_eq!(kept, ["dep-0123abcd", "game-extra-fedcba9876543210"]);
    assert_eq!(
        report.rebuilt,
        [
            "game-fedcba9876543210",
            "shuyi-0123456789abcdef",
            "shuyi-core-0123456789abcdef"
        ]
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_donors_files_are_unchanged() {
    let dir = temp("donor-kept");
    let from = donor(&dir.join("donor"));
    let before = snapshot(&from);
    seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap();
    assert_eq!(snapshot(&from), before);
    let _ = fs::remove_dir_all(&dir);
}

/// Every file under `dir`, with its bytes and time.
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>, SystemTime)> {
    let mut files = Vec::new();
    let mut folders = vec![dir.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                folders.push(path);
            } else {
                files.push((path.clone(), fs::read(&path).unwrap(), modified(&path)));
            }
        }
    }
    files.sort();
    files
}

#[test]
fn a_partly_built_target_keeps_what_it_has() {
    let dir = temp("partly");
    donor(&dir.join("donor"));
    // What a session's MCP server built before the seed: `dep`'s library
    // and its build script's run, and the path package `shuyi`.
    let to = dir.join("into/target/debug");
    write(&to.join("deps/libdep-0123abcd.rlib"), "its own rlib");
    write(&to.join(".fingerprint/dep-0123abcd/lib-dep"), "its own");
    write(&to.join("build/dep-89abcdef/output"), "its own output");
    write(
        &to.join(".fingerprint/shuyi-0123456789abcdef/lib-shuyi"),
        "its own shuyi",
    );
    // A unit a seed stopped part-way left under its temporary name.
    write(
        &to.join(".fingerprint/game-extra-fedcba9876543210.seeding/half"),
        "",
    );
    let fingerprints_time = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    set_modified(&to.join(".fingerprint"), fingerprints_time).unwrap();
    let report = seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap();
    let read = |path: &str| fs::read_to_string(to.join(path)).unwrap();
    assert_eq!(read("deps/libdep-0123abcd.rlib"), "its own rlib");
    assert_eq!(read(".fingerprint/dep-0123abcd/lib-dep"), "its own");
    assert!(
        !to.join(".fingerprint/dep-0123abcd/lib-dep.json").exists(),
        "a kept unit is whole"
    );
    assert_eq!(read("build/dep-89abcdef/output"), "its own output");
    assert!(
        !to.join("build/dep-89abcdef/root-output").exists(),
        "a kept unit is whole"
    );
    assert_eq!(
        read(".fingerprint/shuyi-0123456789abcdef/lib-shuyi"),
        "its own shuyi"
    );
    assert_eq!(
        read("deps/libdep-0123abcd.rmeta"),
        "rmeta",
        "what it lacks is added"
    );
    assert_eq!(
        read(".fingerprint/game-extra-fedcba9876543210/lib-game-extra"),
        "1"
    );
    assert!(
        !to.join(".fingerprint/game-extra-fedcba9876543210.seeding")
            .exists()
    );
    assert_eq!(
        modified(&to.join(".fingerprint")),
        fingerprints_time,
        "its own folders' times"
    );
    assert_eq!(
        modified(&to.join("build/dep-4567cdef")),
        old(),
        "the donor's folders' times"
    );
    // The rlib, the unit of `dep` and the build script's unit.
    assert_eq!(report.kept, 3, "{report}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_target_with_a_build_running_is_refused() {
    let dir = temp("target-locked");
    donor(&dir.join("donor"));
    write(&dir.join("into/target/debug/.cargo-build-lock"), "");
    let held = File::open(dir.join("into/target/debug/.cargo-build-lock")).unwrap();
    held.lock().unwrap();
    let error = seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap_err();
    assert!(
        matches!(&error, Error::Locked(path) if path.ends_with(".cargo-build-lock")),
        "{error}"
    );
    assert!(
        !dir.join("into/target/debug/deps").exists(),
        "nothing is written"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn an_empty_or_absent_target_is_seeded() {
    let dir = temp("empty");
    donor(&dir.join("donor"));
    fs::create_dir(dir.join("into/target")).unwrap();
    seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap();
    assert!(dir.join("into/target/debug/deps").is_dir());
    assert!(
        !dir.join("into/target/.seed-probe").exists(),
        "the probe link is removed"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_donor_with_a_build_running_is_refused() {
    let dir = temp("locked");
    let from = donor(&dir.join("donor"));
    let held = File::open(from.join(".cargo-lock")).unwrap();
    held.lock().unwrap();
    let error = seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap_err();
    assert!(
        matches!(&error, Error::Locked(path) if path.ends_with(".cargo-lock")),
        "{error}"
    );
    assert!(!dir.join("into/target").exists(), "nothing is written");
    held.unlock().unwrap();
    seed(&dir.join("donor"), &dir.join("into"), &packages()).unwrap();
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_link_across_volumes_is_refused_as_another_volume() {
    let crossing = io::Error::from(io::ErrorKind::CrossesDevices);
    let error = link_refusal(crossing, Path::new("C:/donor"), Path::new("D:/into"));
    assert!(matches!(error, Error::OtherVolume { .. }), "{error}");
    assert!(error.to_string().contains("different volumes"), "{error}");
    let denied = io::Error::from(io::ErrorKind::PermissionDenied);
    let error = link_refusal(denied, Path::new("C:/donor"), Path::new("D:/into"));
    assert!(matches!(error, Error::Io { .. }), "{error}");
}

#[test]
fn the_donor_itself_and_a_donor_without_a_build_are_refused() {
    let dir = temp("same");
    donor(&dir.join("donor"));
    let error = seed(&dir.join("donor"), &dir.join("donor/."), &packages()).unwrap_err();
    assert!(matches!(error, Error::SameWorktree(_)), "{error}");
    fs::create_dir(dir.join("other")).unwrap();
    let error = seed(&dir.join("into"), &dir.join("other"), &packages()).unwrap_err();
    assert!(matches!(error, Error::NoDonorBuild(_)), "{error}");
    let error = seed(&dir.join("donor"), &dir.join("missing"), &packages()).unwrap_err();
    assert!(matches!(error, Error::NoWorktree(_)), "{error}");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn the_root_is_found_with_either_separator_and_only_where_its_name_ends() {
    let from = Path::new("D:/work/engine");
    let to = Path::new("D:/work/engine/.claude/worktrees/new");
    let text = b"D:\\work\\engine\\target D:/work/engine/target D:/work/engine-old D:/work/engine";
    let out = rewrite_root(text, from, to, false).unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "D:\\work\\engine\\.claude\\worktrees\\new\\target D:/work/engine/.claude/worktrees/new/target \
         D:/work/engine-old D:/work/engine/.claude/worktrees/new"
    );
    assert_eq!(rewrite_root(b"nothing here", from, to, false), None);
    assert_eq!(rewrite_root(b"d:/WORK/engine/x", from, to, false), None);
    assert_eq!(
        rewrite_root(b"d:/WORK/engine/x", from, to, true).unwrap(),
        b"D:/work/engine/.claude/worktrees/new/x"
    );
    assert_eq!(rewrite_root(b"D:/work/engine.bak", from, to, false), None);
    assert_eq!(
        rewrite_root(b"D:/work/engine\"", from, to, false).unwrap(),
        b"D:/work/engine/.claude/worktrees/new\""
    );
}

#[test]
fn a_fingerprint_names_its_package_then_a_hex_hash() {
    let packages = packages();
    assert!(is_path_unit("shuyi-0123456789abcdef", &packages));
    assert!(is_path_unit("shuyi-core-0123456789abcdef", &packages));
    assert!(!is_path_unit("shuyi-cli-0123456789abcdef", &packages));
    assert!(
        !is_path_unit("game-extra", &packages),
        "extra is not a hash"
    );
    assert!(!is_path_unit("game-", &packages));
    assert!(!is_path_unit("game", &packages));
}

#[test]
fn compiled_output_is_known_by_its_extension() {
    for linked in [
        "a.rlib",
        "a.rmeta",
        "a.exe",
        "a.dll",
        "liba.a",
        "a.o",
        "a.lib",
        "liba.so",
        "liba.dylib",
        "A.DLL",
    ] {
        assert!(is_linked(Path::new(linked)), "{linked}");
    }
    for copied in [
        "a.d",
        "output",
        "root-output",
        "lib-a.json",
        "invoked.timestamp",
        "a.pdb",
        "a.rs",
    ] {
        assert!(!is_linked(Path::new(copied)), "{copied}");
    }
}

#[test]
fn path_packages_are_those_without_a_source() {
    let metadata = serde_json::json!({
        "packages": [
            {"name": "shuyi-core", "source": null},
            {"name": "serde", "source": "registry+https://github.com/rust-lang/crates.io-index"},
            {"name": "game", "source": null},
            {"name": "patched", "source": "git+https://example.com/patched"},
            {"name": "game", "source": null},
        ]
    });
    assert_eq!(path_packages(&metadata), ["game", "shuyi-core"]);
    assert_eq!(path_packages(&serde_json::json!({})), Vec::<String>::new());
}

#[test]
fn the_report_says_what_was_done() {
    let report = Report {
        linked: 3,
        linked_bytes: 5 * 1024 * 1024,
        copied: 2,
        copied_bytes: 10,
        rewritten: 1,
        kept: 4,
        rebuilt: vec!["shuyi-0".to_owned()],
    };
    assert_eq!(
        report.to_string(),
        "linked 3 files (5 MiB), copied 2 files (0 MiB), rewrote the root in 1 build outputs, \
         kept 4 units and files already there, left out 1 fingerprints of path packages (they \
         rebuild)"
    );
}
