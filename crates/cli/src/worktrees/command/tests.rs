//! `trim-targets` and `seed-target` as `shuyi`'s commands: their flags,
//! their refusals' codes, and a trim of a game's worktrees in temporary
//! folders.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use super::*;

fn args(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_owned()).collect()
}

/// A folder of its own under the system's temporary folder, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-wtcmd-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `text` to `path`, making its folders.
fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Runs `git` in `dir`, which must succeed; its stdout.
fn git_ok(dir: &Path, words: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .args([
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "core.hooksPath=no-hooks",
        ])
        .args(words)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {words:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A game in git at `dir/game`: its `shuyi.ron` committed on `main`.
fn game_repo(dir: &Path) -> PathBuf {
    let game = dir.join("game");
    write(&game.join(project::FILE), "()");
    write(&game.join(".gitignore"), "/target/\n");
    git_ok(&game, &["init", "-q", "-b", "main"]);
    git_ok(&game, &["add", "."]);
    git_ok(&game, &["commit", "-q", "-m", "first"]);
    game
}

/// A profile folder at `target/<at>`, built 9 days ago, with an
/// incremental cache and a compiled crate.
fn profile(target: &Path, at: &str) {
    let dir = target.join(at);
    let fingerprint = dir.join(".fingerprint/foo-0123456789abcdef/lib-foo");
    write(&fingerprint, "hash");
    let old = SystemTime::now() - Duration::from_hours(9 * 24);
    fs::File::options()
        .write(true)
        .open(&fingerprint)
        .unwrap()
        .set_modified(old)
        .unwrap();
    write(&dir.join("incremental/foo-1/s-1/query-cache.bin"), "cache");
    write(&dir.join("deps/libfoo-0123456789abcdef.rlib"), "rlib");
}

#[test]
fn trim_targets_takes_its_flags_with_a_games_roots() {
    let Ok(Parsed::Run { what, json }) = parse_trim(&[]) else {
        panic!("no flags runs");
    };
    assert_eq!(what.roots, trim::GAME_ROOTS);
    assert_eq!(what.idle_days, trim::DEFAULT_IDLE_DAYS);
    assert!(!json && !what.dry_run && !what.stale_hashes && !what.merged);
    let all = args(&[
        "--idle-days",
        "7",
        "--dry-run",
        "--stale-hashes",
        "--merged",
        "--json",
    ]);
    let Ok(Parsed::Run { what, json }) = parse_trim(&all) else {
        panic!("every flag runs");
    };
    assert_eq!(what.idle_days, 7);
    assert!(json && what.dry_run && what.stale_hashes && what.merged);
    assert_eq!(
        parse_trim(&args(&["--dry-run", "--help"])),
        Ok(Parsed::Help)
    );
    for (wrong, says) in [
        (&["--idle-days"][..], "needs a number"),
        (&["--idle-days", "three"], "not a number"),
        (&["--force"], "unknown argument --force"),
    ] {
        let error = parse_trim(&args(wrong)).unwrap_err();
        assert!(error.contains(says), "{wrong:?}: {error}");
    }
}

#[test]
fn seed_target_takes_one_worktree() {
    assert_eq!(
        parse_seed(&args(&["--into", "../wt", "--json"])),
        Ok(Parsed::Run {
            what: PathBuf::from("../wt"),
            json: true
        })
    );
    assert_eq!(parse_seed(&args(&["--help"])), Ok(Parsed::Help));
    for (wrong, says) in [
        (&[][..], "--into <worktree> names"),
        (&["--into"], "needs the new worktree"),
        (&["--into", "a", "--into", "b"], "not two"),
        (&["../wt"], "unknown argument ../wt"),
    ] {
        let error = parse_seed(&args(wrong)).unwrap_err();
        assert!(error.contains(says), "{wrong:?}: {error}");
    }
}

#[test]
fn each_refusal_of_the_seed_has_its_code() {
    let path = || PathBuf::from("x");
    let cases = [
        (seed::Error::NoWorktree(path()), "SH0007"),
        (seed::Error::SameWorktree(path()), "SH0008"),
        (seed::Error::NoDonorBuild(path()), "SH1011"),
        (seed::Error::Locked(path()), "SH3016"),
        (
            seed::Error::OtherVolume {
                donor: path(),
                into: path(),
            },
            "SH3017",
        ),
        (
            seed::Error::Io {
                path: path(),
                error: io::Error::other("full"),
            },
            "SH3004",
        ),
    ];
    for (error, code) in cases {
        assert_eq!(seed_code(&error).0, code, "{error}");
        assert!(codes::entry(code).is_some(), "{code} has a row");
    }
    // Usage codes exit 2, setup ones 3, the others 1.
    let exits: Vec<u8> = ["SH0007", "SH1010", "SH3016"]
        .iter()
        .map(|code| codes::exit_code(Some(code)))
        .collect();
    assert_eq!(exits, [2, 3, 1]);
}

#[test]
fn a_game_in_no_repository_of_its_own_is_refused() {
    let dir = scratch("repo");
    // In no repository at all (the temporary folder is in none).
    let alone = dir.join("alone");
    write(&alone.join(project::FILE), "()");
    if Command::new("git")
        .current_dir(&alone)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .is_ok_and(|out| !out.status.success())
    {
        assert_eq!(repository(&alone).unwrap_err().code, codes::NO_REPOSITORY);
    }
    // Inside a repository that ignores it, as a game made in a target/.
    let outer = dir.join("outer");
    write(&outer.join(".gitignore"), "/target/\n");
    git_ok(&outer, &["init", "-q", "-b", "main"]);
    let inside = outer.join("target/games/moon");
    write(&inside.join(project::FILE), "()");
    let refused = repository(&inside).unwrap_err();
    assert_eq!(refused.code, codes::NO_REPOSITORY);
    assert!(refused.error.contains("ignores it"), "{}", refused.error);
    // In its own: that repository's worktree.
    let game = game_repo(&dir);
    assert!(same_folder(&repository(&game).unwrap(), &game));
    fs::remove_dir_all(&dir).unwrap();
}

fn same_folder(a: &Path, b: &Path) -> bool {
    a.canonicalize().unwrap() == b.canonicalize().unwrap()
}

#[test]
fn seed_target_refuses_a_folder_that_is_no_worktree() {
    let dir = scratch("seed");
    let game = game_repo(&dir);
    for into in [game.join("crates"), dir.join("nowhere")] {
        fs::create_dir_all(game.join("crates")).unwrap();
        let refused = seed_from(&game, &into).unwrap_err();
        assert_eq!(refused.code, codes::NOT_A_WORKTREE, "{}", refused.error);
        assert!(!into.join("target").exists(), "nothing written");
    }
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn trim_targets_deletes_only_the_idle_worktrees_incremental_caches() {
    let dir = scratch("trim");
    let game = game_repo(&dir);
    profile(&game.join("target"), "debug");
    let idle = dir.join("idle");
    git_ok(
        &game,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "idle",
            &idle.to_string_lossy(),
        ],
    );
    let target = idle.join("target");
    for at in [
        "debug",
        "release",
        "x86_64-pc-windows-gnu/debug",
        "wasm32-unknown-unknown/release",
    ] {
        profile(&target, at);
    }
    // A target folder of its own inside target/, as the engine's
    // `tutorial/`: not a triple, so left alone.
    profile(&target, "tutorial/debug");
    write(&target.join("tutorial/.rustc_info.json"), "{}");
    let listed = git_ok(&game, &["worktree", "list", "--porcelain"]);

    let Ok(Parsed::Run { what: options, .. }) = parse_trim(&[]) else {
        panic!("no flags runs");
    };
    let done = trim_from(&game, &options).unwrap();
    assert_eq!((done.report.idle, done.report.kept), (1, 1));
    assert_eq!(done.worktrees.len(), 2, "{:?}", done.worktrees);
    assert!(done.worktrees[0].ends_with("this worktree, left alone"));
    let gone = |at: &str| !target.join(at).join("incremental").exists();
    assert!(gone("debug") && gone("release"));
    assert!(gone("x86_64-pc-windows-gnu/debug") && gone("wasm32-unknown-unknown/release"));
    assert!(!gone("tutorial/debug"));
    assert!(
        target
            .join("debug/deps/libfoo-0123456789abcdef.rlib")
            .exists()
    );
    assert!(game.join("target/debug/incremental").exists(), "this one");
    assert_eq!(git_ok(&game, &["worktree", "list", "--porcelain"]), listed);
    // As JSON, the counts beside the lines.
    let json = serde_json::to_value(&done).unwrap();
    assert_eq!(json["idle"], 1);
    assert_eq!(json["kept"], 1);
    assert!(
        json["summary"]
            .as_str()
            .unwrap()
            .starts_with("trim-targets: freed")
    );
    git_ok(
        &game,
        &["worktree", "remove", "--force", &idle.to_string_lossy()],
    );
    fs::remove_dir_all(&dir).unwrap();
}
