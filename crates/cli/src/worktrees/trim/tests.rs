//! `trim-targets` on folders made in the system's temporary folder: what
//! it finds, what it leaves, and a whole run on a repository of worktrees.

use std::fs::{self, File, FileTimes};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use super::*;

/// A folder of its own under the system's temporary folder, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-trim-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `text` to `path`, making its folders.
fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn days_ago(days: u64) -> SystemTime {
    SystemTime::now() - Duration::from_secs(days * 24 * 60 * 60)
}

/// Sets `path`'s last modification, and its last access, to `time`.
fn touch(path: &Path, time: SystemTime) {
    File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(FileTimes::new().set_modified(time).set_accessed(time))
        .unwrap();
}

/// A profile folder at `target/<at>`, with one unit's fingerprint built
/// `days` ago and an incremental cache of one file.
fn profile(target: &Path, at: &str, days: u64) {
    let dir = target.join(at);
    let fingerprint = dir.join(".fingerprint/foo-0123456789abcdef/lib-foo");
    write(&fingerprint, "hash");
    touch(&fingerprint, days_ago(days));
    write(&dir.join("incremental/foo-1/s-1/query-cache.bin"), "cache");
    write(&dir.join("deps/libfoo-0123456789abcdef.rlib"), "rlib");
}

fn engine_roots() -> Vec<String> {
    ["", "gates/games", "tutorial", "skill-examples*"]
        .map(str::to_owned)
        .to_vec()
}

/// The `incremental` folders found, relative to `target`, `/`-separated.
fn found(target: &Path) -> Vec<String> {
    let mut found: Vec<String> = incremental_dirs(target, &profile_dirs(target), &engine_roots())
        .iter()
        .map(|dir| {
            dir.strip_prefix(target)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect();
    found.sort();
    found
}

#[test]
fn the_porcelain_lists_branches_detached_heads_bare_and_prunable_worktrees() {
    let text = "\
worktree D:/repo
HEAD 4eae83e6714ad5e49434a0647af5b920d02201ce
branch refs/heads/main

worktree C:/Users/someone/.codex/worktrees/521a/repo
HEAD d74659847ae03d46b3c865f2dc096dedc679165c
detached

worktree D:/bare.git
bare

worktree D:/gone
HEAD 47b22db2df7cc5dcfad7e5e103d5ade3242c6660
branch refs/heads/t2001-old
prunable gitdir file points to non-existent location
";
    let worktree = |path: &str, branch: Option<&str>, bare, prunable| Worktree {
        path: PathBuf::from(path),
        branch: branch.map(str::to_owned),
        bare,
        prunable,
    };
    assert_eq!(
        parse_porcelain(text),
        [
            worktree("D:/repo", Some("main"), false, false),
            worktree(
                "C:/Users/someone/.codex/worktrees/521a/repo",
                None,
                false,
                false
            ),
            worktree("D:/bare.git", None, true, false),
            worktree("D:/gone", Some("t2001-old"), false, true),
        ]
    );
    assert_eq!(parse_porcelain(""), []);
}

#[test]
fn incremental_is_found_in_each_profile_folder_under_the_roots() {
    let target = scratch("depths").join("target");
    profile(&target, "debug", 9);
    profile(&target, "release", 9);
    profile(&target, "gates/games/debug", 9);
    profile(&target, "x86_64-pc-windows-gnu/debug", 9);
    profile(&target, "tutorial/debug", 9);
    profile(&target, "skill-examples-build/debug", 9);
    assert_eq!(
        found(&target),
        [
            "debug/incremental",
            "gates/games/debug/incremental",
            "release/incremental",
            "skill-examples-build/debug/incremental",
            "tutorial/debug/incremental",
            "x86_64-pc-windows-gnu/debug/incremental",
        ]
    );
}

#[test]
fn a_folder_named_incremental_elsewhere_is_left_alone() {
    let target = scratch("elsewhere").join("target");
    profile(&target, "debug", 9);
    write(&target.join("debug/deps/incremental/a"), "x");
    write(&target.join("debug/build/foo-1/out/incremental/a"), "x");
    write(&target.join("doc/incremental/a"), "x");
    // Not a profile folder: no `.fingerprint/` beside it.
    write(&target.join("notes/incremental/a"), "x");
    // A profile folder, but under no root.
    profile(&target, "compile-times/grid/target/debug", 9);
    assert_eq!(found(&target), ["debug/incremental"]);
}

#[test]
fn a_root_ending_in_a_star_matches_each_folder_starting_so() {
    let target = Path::new("t");
    let roots = ["skill-examples*".to_owned()];
    let under = |at: &str| under_roots(target, &target.join(at), &roots);
    assert!(under("skill-examples-build/debug"));
    assert!(under("skill-examples/wasm32-unknown-unknown/release"));
    assert!(!under("skill/debug"));
    assert!(!under("skill-examples"), "the root itself is no profile");
    assert!(!under("skill-examples-build/a/b/debug"), "too deep");
}

/// A link (a junction on Windows) to `to`, at `at`.
fn link(to: &Path, at: &Path) {
    fs::create_dir_all(at.parent().unwrap()).unwrap();
    #[cfg(windows)]
    {
        let made = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(at.to_string_lossy().replace('/', "\\"))
            .arg(to.to_string_lossy().replace('/', "\\"))
            .stdout(std::process::Stdio::null())
            .status()
            .unwrap();
        assert!(made.success(), "mklink /J");
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(to, at).unwrap();
}

#[test]
fn a_link_is_never_followed_nor_deleted_through() {
    let dir = scratch("links");
    let outside = dir.join("outside");
    write(&outside.join("keep.txt"), "mine");
    let target = dir.join("target");
    profile(&target, "debug", 9);
    // A link as `incremental` itself, and one inside a real one.
    fs::create_dir_all(target.join("gates/games/debug/.fingerprint")).unwrap();
    link(&outside, &target.join("gates/games/debug/incremental"));
    link(&outside, &target.join("debug/incremental/foo-1/linked"));
    // A link where a profile folder would be.
    link(&target.join("debug"), &target.join("tutorial"));

    assert_eq!(found(&target), ["debug/incremental"]);
    let mut freed = Freed::default();
    delete_tree(&target.join("debug/incremental"), false, &mut freed);
    assert_eq!(
        freed,
        Freed {
            bytes: 5,
            skipped: 1
        },
        "the link is skipped"
    );
    assert_eq!(
        fs::read_to_string(outside.join("keep.txt")).unwrap(),
        "mine"
    );
}

#[test]
fn a_worktree_inside_a_target_is_never_deleted() {
    let target = scratch("nested").join("target");
    profile(&target, "debug", 9);
    write(&target.join("api-base/.git"), "gitdir: elsewhere");
    write(&target.join("api-base/src/lib.rs"), "mine");
    let mut freed = Freed::default();
    delete_tree(&target, false, &mut freed);
    assert_eq!(
        freed,
        Freed {
            bytes: 13,
            skipped: 1
        }
    );
    assert!(target.join("api-base/src/lib.rs").exists());
    assert!(!target.join("debug").exists());
}

#[test]
fn the_last_build_is_the_newest_fingerprint() {
    let target = scratch("idle").join("target");
    assert_eq!(last_build(&profile_dirs(&target)), None, "never built");
    profile(&target, "debug", 9);
    profile(&target, "gates/games/debug", 5);
    let last = last_build(&profile_dirs(&target)).unwrap();
    let age = SystemTime::now().duration_since(last).unwrap().as_secs() / 3600;
    assert_eq!(age, 5 * 24, "the newer of the two");
}

#[test]
fn a_held_cargo_lock_makes_the_target_locked() {
    let target = scratch("lock").join("target");
    profile(&target, "debug", 9);
    let profiles = profile_dirs(&target);
    assert!(!locked(&target, &profiles), "no lock file");
    for name in LOCK_FILES {
        let file = target.join("debug").join(name);
        write(&file, "");
        assert!(!locked(&target, &profiles), "{name} there, not held");
        let held = File::open(&file).unwrap();
        held.lock().unwrap();
        assert!(locked(&target, &profiles), "{name} held");
        held.unlock().unwrap();
        assert!(!locked(&target, &profiles), "{name} released");
    }
}

#[test]
fn sizes_read_in_gb_or_mb() {
    assert_eq!(bytes(0), "0 MB");
    assert_eq!(bytes(1), "1 MB");
    assert_eq!(bytes(999_000_000), "999 MB");
    assert_eq!(bytes(1_000_000_000), "1.0 GB");
    assert_eq!(bytes(25_340_000_000), "25.3 GB");
}

#[test]
fn the_summary_names_the_freed_space_and_the_notes() {
    let report = Report {
        idle: 2,
        kept: 5,
        freed: 30_500_000_000,
        skipped: 3,
        dry_run: false,
        notes: vec!["`gh` didn't answer: merged pull requests not looked for".to_owned()],
    };
    assert_eq!(
        report.summary(),
        "trim-targets: freed 30.5 GB in 2 idle worktree(s), left 5 alone, \
         3 file(s) in use skipped; `gh` didn't answer: merged pull requests not looked for"
    );
    let dry = Report {
        dry_run: true,
        ..Report::default()
    };
    assert_eq!(
        dry.summary(),
        "trim-targets: would free 0 MB in 0 idle worktree(s), left 0 alone"
    );
}

/// Runs `git` in `dir`, which must succeed; its stdout.
fn git_ok(dir: &Path, args: &[&str]) -> String {
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
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A worktree of `repo` at `dir/<name>` on a branch of that name, with a
/// commit of its own when `commit`, and a `target/` built `days` ago.
fn worktree(repo: &Path, dir: &Path, name: &str, commit: bool, days: u64) -> PathBuf {
    let path = dir.join(name);
    git_ok(
        repo,
        &["worktree", "add", "-q", "-b", name, &path.to_string_lossy()],
    );
    if commit {
        write(&path.join("work.txt"), name);
        git_ok(&path, &["add", "work.txt"]);
        git_ok(&path, &["commit", "-q", "-m", name]);
    }
    profile(&path.join("target"), "debug", days);
    path
}

#[test]
fn a_run_trims_idle_worktrees_only_and_keeps_every_worktree() {
    let dir = scratch("run");
    let repo = dir.join("repo");
    fs::create_dir_all(&repo).unwrap();
    git_ok(&repo, &["init", "-q", "-b", "main"]);
    write(&repo.join("readme.txt"), "repo");
    git_ok(&repo, &["add", "readme.txt"]);
    git_ok(&repo, &["commit", "-q", "-m", "first"]);
    profile(&repo.join("target"), "debug", 9);
    let idle = worktree(&repo, &dir, "idle", true, 9);
    let busy = worktree(&repo, &dir, "busy", true, 1);
    let held = worktree(&repo, &dir, "held", true, 9);
    let merged = worktree(&repo, &dir, "merged", false, 9);
    write(&held.join("target/debug/.cargo-lock"), "");
    let lock = File::open(held.join("target/debug/.cargo-lock")).unwrap();
    lock.lock().unwrap();
    let listed = git_ok(&repo, &["worktree", "list", "--porcelain"]);
    let incremental = |worktree: &Path| worktree.join("target/debug/incremental").exists();

    let mut options = Options::new(engine_roots());
    options.base = "main".to_owned();
    options.dry_run = true;
    let mut out = Vec::new();
    let report = run(&repo, &options, &mut out).unwrap();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains(": this worktree, left alone"), "{text}");
    assert!(
        text.contains("(idle): target 1 MB, idle (last build 9 days ago), would delete"),
        "{text}"
    );
    assert!(text.contains("(held): target 1 MB, locked"), "{text}");
    assert!(
        text.contains("(busy): target 1 MB, built 24 hours ago"),
        "{text}"
    );
    assert_eq!((report.idle, report.kept, report.freed), (2, 3, 10));
    assert!(
        [&repo, &idle, &busy, &held, &merged]
            .iter()
            .all(|w| incremental(w)),
        "dry"
    );

    options.dry_run = false;
    let report = run(&repo, &options, &mut std::io::sink()).unwrap();
    assert_eq!((report.idle, report.kept, report.freed), (2, 3, 10));
    assert!(!incremental(&idle) && !incremental(&merged));
    assert!(incremental(&repo) && incremental(&busy) && incremental(&held));
    assert!(
        idle.join("target/debug/deps/libfoo-0123456789abcdef.rlib")
            .exists()
    );

    // `--merged` empties the target/ of a branch merged into main, and
    // keeps the worktree and target/ itself.
    options.merged = true;
    let report = run(&repo, &options, &mut std::io::sink()).unwrap();
    assert_eq!(report.freed, 4 + 4, "foo's rlib and fingerprint");
    assert!(
        fs::read_dir(merged.join("target"))
            .unwrap()
            .next()
            .is_none()
    );
    assert!(merged.join("readme.txt").exists(), "the worktree is kept");
    assert!(
        idle.join("target/debug/deps/libfoo-0123456789abcdef.rlib")
            .exists()
    );
    assert_eq!(git_ok(&repo, &["worktree", "list", "--porcelain"]), listed);
    lock.unlock().unwrap();
}
