//! `trim-targets`: frees the disk the build output of a repository's idle
//! worktrees takes, and never removes, prunes or archives a worktree
//! (ADR-0121).
//!
//! It lists the repository's worktrees (`git worktree list --porcelain`),
//! and leaves alone the one it runs in, one with no `target/`, one whose
//! `target/` is a link, one where cargo holds a lock (a build is running)
//! and one built in the last [`Options::idle_days`]. In each other one, it
//! deletes the `incremental` folder of every cargo profile folder under one
//! of [`Options::roots`]: cargo still finds the build Fresh, and only the
//! first edit there is slower. With [`Options::merged`], it deletes the
//! whole content of `target/` of a worktree whose branch is merged. With
//! [`Options::stale_hashes`], it also deletes the older hashes of a crate
//! in those profile folders' `deps/` that no build read in days
//! (ADR-0123).
//!
//! Nothing outside `<worktree>/target/` is ever deleted, no link is
//! followed, no folder holding `.git` is entered, and deletes are
//! best-effort: a file in use is left and counted. `cargo xtask trim-targets` runs it on the engine's worktrees,
//! and `cargo xtask gates` at the end of a passing run.

use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use super::seed::LOCKS;

/// How many days without a build make a worktree idle, by default.
pub const DEFAULT_IDLE_DAYS: u64 = 3;

/// The [`Options::roots`] of a game's worktrees (`shuyi trim-targets`):
/// `target/` itself, so `target/debug`, `target/release`,
/// a triple's `target/<triple>/<profile>` and the web build's
/// `target/wasm32-unknown-unknown/release`. No folder of the engine's own
/// builds is named (its `gates/`, `tutorial/`, `skill-examples*`), and a
/// folder that is a target folder of its own is never read as a triple.
pub const GAME_ROOTS: [&str; 1] = [""];

/// The file cargo writes at the top of each target folder, and never in a
/// triple's folder: a folder holding it is another build's target folder.
const TARGET_MARK: &str = ".rustc_info.json";

/// How deep under `target/` a profile folder is looked for:
/// `compile-times/<game>/target/debug` is at 4.
const PROFILE_DEPTH: usize = 5;

/// Folders under `target/` never searched for a profile folder: cargo's
/// documentation, and the sources of the games the gates make there.
const NOT_SEARCHED: [&str; 3] = ["doc", "src", ".git"];

const DAY: Duration = Duration::from_hours(24);

/// What `trim-targets` deletes, and where it looks.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Options {
    /// The folders under `target/`, `/`-separated, whose profile folders
    /// (`<root>/<profile>` and `<root>/<triple>/<profile>`) lose their
    /// `incremental` folder. `""` is `target/` itself; a last part ending
    /// in `*` matches each folder that starts with what comes before it
    /// (`skill-examples*`).
    pub roots: Vec<String>,
    /// A worktree with no build in this many days is idle.
    pub idle_days: u64,
    /// Say what would be deleted, with each `target/`'s size, and delete
    /// nothing.
    pub dry_run: bool,
    /// Also delete all of `target/` of a worktree whose branch is merged
    /// into [`Options::base`], or whose pull request is merged (`gh`).
    pub merged: bool,
    /// The branch [`Options::merged`] compares with.
    pub base: String,
    /// Also delete, in the `deps/` of the same profile folders, the older
    /// hashes of a crate last accessed more than [`Options::idle_days`]
    /// before the worktree's last build, keeping the newest-accessed one
    /// of each crate and kind. Skipped, with a note, where
    /// the system doesn't update last access.
    pub stale_hashes: bool,
}

impl Options {
    /// The defaults for `roots`: idle after [`DEFAULT_IDLE_DAYS`], only
    /// `incremental` folders deleted, merged against `origin/main`.
    pub fn new(roots: Vec<String>) -> Self {
        Self {
            roots,
            idle_days: DEFAULT_IDLE_DAYS,
            dry_run: false,
            merged: false,
            base: "origin/main".to_owned(),
            stale_hashes: false,
        }
    }
}

/// What a run did, in all the worktrees.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize)]
#[non_exhaustive]
pub struct Report {
    /// The idle worktrees, trimmed (or that would be, dry), some with
    /// nothing to delete.
    pub idle: usize,
    /// The worktrees left alone: this one, missing, without `target/`,
    /// locked or built recently.
    pub kept: usize,
    /// The bytes deleted (or that would be), as the files' lengths: on a
    /// compressed `target/` the disk freed is less.
    pub freed: u64,
    /// The files that couldn't be deleted (in use), left where they are.
    pub skipped: u64,
    /// How many of the files deleted (or that would be) were stale hashes
    /// in `deps/` ([`Options::stale_hashes`]).
    pub stale: u64,
    /// Whether the run deleted nothing ([`Options::dry_run`]).
    pub dry_run: bool,
    /// What was skipped in the whole run, and why (`gh` missing).
    pub notes: Vec<String>,
}

impl Report {
    /// The run in one line, for the end of `cargo xtask gates`.
    pub fn summary(&self) -> String {
        let verb = if self.dry_run { "would free" } else { "freed" };
        let mut line = format!(
            "trim-targets: {verb} {} in {} idle worktree(s), left {} alone",
            bytes(self.freed),
            self.idle,
            self.kept
        );
        if self.stale > 0 {
            let _ = write!(line, ", {} stale hash file(s) among it", self.stale);
        }
        if self.skipped > 0 {
            let _ = write!(line, ", {} file(s) in use skipped", self.skipped);
        }
        for note in &self.notes {
            let _ = write!(line, "; {note}");
        }
        line
    }
}

/// Trims the worktrees of the repository `repo` is in, all but `repo`'s
/// own, writing a line for each to `out`.
///
/// Fails only when the worktrees can't be listed.
pub fn run(repo: &Path, options: &Options, out: &mut dyn Write) -> Result<Report, String> {
    let current = git(repo, &["rev-parse", "--show-toplevel"])?;
    let current = fs::canonicalize(current.trim()).map_err(|e| e.to_string())?;
    let worktrees = parse_porcelain(&git(repo, &["worktree", "list", "--porcelain"])?);
    let idle = DAY.saturating_mul(u32::try_from(options.idle_days).unwrap_or(u32::MAX));
    let cutoff = SystemTime::now()
        .checked_sub(idle)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut report = Report {
        dry_run: options.dry_run,
        ..Report::default()
    };
    let stale_hashes = options.stale_hashes
        && match stale::last_access_updates() {
            Ok(()) => true,
            Err(why) => {
                report
                    .notes
                    .push(format!("{why}: stale hashes not looked for"));
                false
            }
        };
    let mut gh_missing = false;
    for worktree in &worktrees {
        let mut line = format!("{}", worktree.path.display());
        if let Some(branch) = &worktree.branch {
            let _ = write!(line, " ({branch})");
        }
        let freed = trim_one(
            &Trim {
                repo,
                current: &current,
                options,
                cutoff,
                idle,
                stale_hashes,
            },
            worktree,
            &mut line,
            &mut gh_missing,
        );
        match freed {
            Some(freed) => {
                report.idle += 1;
                report.freed += freed.bytes;
                report.skipped += freed.skipped;
                report.stale += freed.stale;
            }
            None => report.kept += 1,
        }
        let _ = writeln!(out, "{line}");
    }
    if gh_missing {
        report
            .notes
            .push("`gh` didn't answer: merged pull requests not looked for".to_owned());
    }
    let _ = writeln!(out, "{}", report.summary());
    Ok(report)
}

/// What every worktree is trimmed against.
struct Trim<'a> {
    repo: &'a Path,
    current: &'a Path,
    options: &'a Options,
    cutoff: SystemTime,
    /// [`Options::idle_days`] as a duration.
    idle: Duration,
    /// [`Options::stale_hashes`], when the system updates last access.
    stale_hashes: bool,
}

/// Trims one worktree, describing it on `line`. `None` when it is left
/// alone; otherwise what was deleted, which may be nothing.
fn trim_one(
    trim: &Trim,
    worktree: &Worktree,
    line: &mut String,
    gh_missing: &mut bool,
) -> Option<Freed> {
    let options = trim.options;
    let (target, profiles, last) = idle_target(trim, worktree, line)?;
    let verb = if options.dry_run {
        "would delete"
    } else {
        "deleted"
    };
    let mut freed = Freed::default();
    let merged = options.merged
        && worktree
            .branch
            .as_deref()
            .is_some_and(|branch| merged(trim.repo, branch, &options.base, gh_missing));
    if merged {
        delete_tree(&target, options.dry_run, &mut freed);
        let _ = write!(
            line,
            ", merged: {verb} all of target/ ({})",
            bytes(freed.bytes)
        );
        return Some(freed);
    }
    for dir in incremental_dirs(&target, &profiles, &options.roots) {
        let before = freed.bytes;
        delete_tree(&dir, options.dry_run, &mut freed);
        if !options.dry_run {
            let _ = fs::remove_dir(&dir);
        }
        let shown = dir.strip_prefix(&target).unwrap_or(&dir);
        let _ = write!(
            line,
            ", {verb} {} ({})",
            shown.display(),
            bytes(freed.bytes - before)
        );
    }
    if trim.stale_hashes {
        // Relative to the last build, not to now: an idle worktree's live
        // hashes were all read last when it was last built.
        let cutoff = last.map_or(trim.cutoff, |last| {
            last.checked_sub(trim.idle)
                .unwrap_or(SystemTime::UNIX_EPOCH)
        });
        let before = freed;
        delete_stale(&target, &profiles, cutoff, options, &mut freed);
        if freed.stale > before.stale {
            let _ = write!(
                line,
                ", {verb} {} stale hash file(s) in deps/ ({})",
                freed.stale - before.stale,
                bytes(freed.bytes - before.bytes)
            );
        }
    }
    if freed.bytes == 0 && freed.skipped == 0 && freed.stale == 0 {
        line.push_str(", nothing to delete");
    }
    if freed.skipped > 0 {
        let _ = write!(line, ", {} file(s) in use skipped", freed.skipped);
    }
    Some(freed)
}

/// The worktree's `target/`, its profile folders and its last build, when
/// it is to be trimmed; `None`, said on `line`, when it is left alone: this one, bare,
/// prunable, missing, without a `target/` of its own, locked, or built
/// after the cutoff.
fn idle_target(
    trim: &Trim,
    worktree: &Worktree,
    line: &mut String,
) -> Option<(PathBuf, Vec<PathBuf>, Option<SystemTime>)> {
    let options = trim.options;
    if worktree.bare || worktree.prunable || !worktree.path.is_dir() {
        line.push_str(": bare, prunable or missing, left alone");
        return None;
    }
    if fs::canonicalize(&worktree.path).is_ok_and(|path| path == trim.current) {
        line.push_str(": this worktree, left alone");
        return None;
    }
    let target = worktree.path.join("target");
    match fs::symlink_metadata(&target) {
        Err(_) => {
            line.push_str(": no target/");
            return None;
        }
        Ok(meta) if is_link(&meta) || !meta.is_dir() => {
            line.push_str(": target/ is a link or a file, left alone");
            return None;
        }
        Ok(_) => {}
    }
    if options.dry_run {
        let _ = write!(line, ": target {}", bytes(tree_size(&target)));
    }
    let profiles = profile_dirs(&target);
    if locked(&target, &profiles) {
        line.push_str(", locked (a cargo build is running), left alone");
        return None;
    }
    let last = last_build(&profiles);
    let age = |time: SystemTime| {
        SystemTime::now()
            .duration_since(time)
            .unwrap_or_default()
            .as_secs()
            / 3600
    };
    match last {
        Some(time) if time >= trim.cutoff => {
            let _ = write!(
                line,
                ", built {} hours ago (in the last {} days), left alone",
                age(time),
                options.idle_days
            );
            return None;
        }
        Some(time) => {
            let _ = write!(line, ", idle (last build {} days ago)", age(time) / 24);
        }
        None => line.push_str(", idle (never built)"),
    }
    Some((target, profiles, last))
}

/// A worktree, as `git worktree list --porcelain` lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Worktree {
    path: PathBuf,
    /// The branch checked out (`refs/heads/` taken off), `None` when the
    /// head is detached.
    branch: Option<String>,
    bare: bool,
    prunable: bool,
}

/// The worktrees of `git worktree list --porcelain`'s output: one block
/// of lines each, blocks apart by an empty line.
fn parse_porcelain(text: &str) -> Vec<Worktree> {
    let mut worktrees = Vec::new();
    let mut current: Option<Worktree> = None;
    for line in text.lines().chain(std::iter::once("")) {
        let (key, value) = line.split_once(' ').unwrap_or((line, ""));
        match key {
            "worktree" => {
                worktrees.extend(current.take());
                current = Some(Worktree {
                    path: PathBuf::from(value),
                    branch: None,
                    bare: false,
                    prunable: false,
                });
            }
            "" => worktrees.extend(current.take()),
            _ => {
                let Some(worktree) = current.as_mut() else {
                    continue;
                };
                match key {
                    "branch" => {
                        let name = value.strip_prefix("refs/heads/").unwrap_or(value);
                        worktree.branch = Some(name.to_owned());
                    }
                    "bare" => worktree.bare = true,
                    "prunable" => worktree.prunable = true,
                    _ => {}
                }
            }
        }
    }
    worktrees
}

/// Whether `meta` (read without following) is a symbolic link or, on
/// Windows, any reparse point: a junction too.
fn is_link(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        if meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    meta.file_type().is_symlink()
}

/// The folders under `dir` that are folders, not links.
fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir() && !is_link(&meta))
        })
        .collect();
    dirs.sort();
    dirs
}

/// The cargo profile folders under `target` (those holding a
/// `.fingerprint/`), at any depth to [`PROFILE_DEPTH`], never through a
/// link.
fn profile_dirs(target: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
        for sub in subdirs(dir) {
            if sub.join(".fingerprint").is_dir() {
                out.push(sub);
            } else if depth < PROFILE_DEPTH
                && !sub
                    .file_name()
                    .is_some_and(|name| NOT_SEARCHED.iter().any(|n| name == *n))
            {
                walk(&sub, depth + 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(target, 1, &mut out);
    out
}

/// When cargo last built a unit in any of `profiles`: the newest
/// modification of a file directly in a `.fingerprint/<unit>/` folder,
/// which cargo rewrites each time it builds the unit. `None` when there is
/// none.
fn last_build(profiles: &[PathBuf]) -> Option<SystemTime> {
    let mut newest = None;
    for profile in profiles {
        for unit in subdirs(&profile.join(".fingerprint")) {
            let Ok(entries) = fs::read_dir(&unit) else {
                continue;
            };
            for entry in entries.flatten() {
                let Ok(meta) = fs::symlink_metadata(entry.path()) else {
                    continue;
                };
                if meta.is_file()
                    && let Ok(modified) = meta.modified()
                {
                    newest = newest.max(Some(modified));
                }
            }
        }
    }
    newest
}

/// Whether cargo holds a lock in `target`: on one of [`LOCKS`] (the seed's) in
/// `target/` itself or in one of its profile folders.
fn locked(target: &Path, profiles: &[PathBuf]) -> bool {
    std::iter::once(target)
        .chain(profiles.iter().map(PathBuf::as_path))
        .flat_map(|dir| LOCKS.iter().map(move |name| dir.join(name)))
        .any(|file| is_locked(&file))
}

/// Whether another process holds a lock on `file`: tried, and released at
/// once. A file that can't be opened counts as locked; a missing one not.
fn is_locked(file: &Path) -> bool {
    if !file.is_file() {
        return false;
    }
    let Ok(handle) = fs::File::open(file) else {
        return true;
    };
    match handle.try_lock() {
        Ok(()) => {
            let _ = handle.unlock();
            false
        }
        Err(_) => true,
    }
}

/// Whether `profile`, a folder under `target`, is `<root>/<profile>` or
/// `<root>/<triple>/<profile>` for one of `roots`. A `<triple>` folder
/// holding [`TARGET_MARK`] is another build's target folder (the engine's
/// `target/tutorial/`), not a triple's.
fn under_roots(target: &Path, profile: &Path, roots: &[String]) -> bool {
    let Ok(relative) = profile.strip_prefix(target) else {
        return false;
    };
    let parts: Vec<String> = relative
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    roots.iter().any(|root| {
        let pattern: Vec<&str> = root.split('/').filter(|p| !p.is_empty()).collect();
        let extra = parts.len().checked_sub(pattern.len());
        let triple_ok = || {
            profile
                .parent()
                .is_some_and(|triple| !triple.join(TARGET_MARK).exists())
        };
        (extra == Some(1) || (extra == Some(2) && triple_ok()))
            && pattern.iter().zip(&parts).all(|(want, part)| {
                want.strip_suffix('*')
                    .map_or(*want == part.as_str(), |prefix| part.starts_with(prefix))
            })
    })
}

/// The `incremental` folders to delete: one in each profile folder under
/// one of `roots`, when it is a folder and not a link.
fn incremental_dirs(target: &Path, profiles: &[PathBuf], roots: &[String]) -> Vec<PathBuf> {
    profiles
        .iter()
        .filter(|profile| under_roots(target, profile, roots))
        .map(|profile| profile.join("incremental"))
        .filter(|dir| fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir() && !is_link(&m)))
        .collect()
}

/// What a delete freed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Freed {
    /// The deleted files' lengths.
    bytes: u64,
    /// The files and folders left: in use, or links.
    skipped: u64,
    /// How many of the files deleted were stale hashes in `deps/`.
    stale: u64,
}

/// Deletes what is in `dir` (not `dir` itself, which keeps its NTFS
/// compression), best-effort: a file that won't go is counted in
/// `skipped`, and a link is never followed nor removed. A folder holding
/// `.git` (a worktree made inside a `target/`, or a repository) is left
/// whole and counted too. Dry, only counts.
fn delete_tree(dir: &Path, dry_run: bool, freed: &mut Freed) {
    let Ok(entries) = fs::read_dir(dir) else {
        freed.skipped += 1;
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            freed.skipped += 1;
            continue;
        };
        if is_link(&meta) || (meta.is_dir() && path.join(".git").exists()) {
            freed.skipped += 1;
        } else if meta.is_dir() {
            delete_tree(&path, dry_run, freed);
            if !dry_run {
                let _ = fs::remove_dir(&path);
            }
        } else if dry_run || fs::remove_file(&path).is_ok() {
            freed.bytes += meta.len();
        } else {
            freed.skipped += 1;
        }
    }
}

/// Deletes the stale hashes of the `deps/` of each profile folder under
/// [`Options::roots`] ([`stale::stale_files`]), best-effort as
/// [`delete_tree`]. Dry, only counts.
fn delete_stale(
    target: &Path,
    profiles: &[PathBuf],
    cutoff: SystemTime,
    options: &Options,
    freed: &mut Freed,
) {
    let deps = profiles
        .iter()
        .filter(|profile| under_roots(target, profile, &options.roots))
        .map(|profile| profile.join("deps"));
    for deps in deps {
        for file in stale::stale_files(&deps, cutoff) {
            if options.dry_run || fs::remove_file(&file.path).is_ok() {
                freed.bytes += file.len;
                freed.stale += 1;
            } else {
                freed.skipped += 1;
            }
        }
    }
}

/// The length of the files under `dir`, links not followed.
fn tree_size(dir: &Path) -> u64 {
    let mut freed = Freed::default();
    delete_tree(dir, true, &mut freed);
    freed.bytes
}

/// Whether `branch` is merged: an ancestor of `base`, or the head of a
/// merged pull request (`gh pr list`). `base`'s own branch is never.
fn merged(repo: &Path, branch: &str, base: &str, gh_missing: &mut bool) -> bool {
    if base.rsplit('/').next() == Some(branch) {
        return false;
    }
    let local = format!("refs/heads/{branch}");
    let ancestor = Command::new("git")
        .current_dir(repo)
        .args(["merge-base", "--is-ancestor", &local, base])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if ancestor {
        return true;
    }
    let output = Command::new("gh")
        .current_dir(repo)
        .args(["pr", "list", "--state", "merged", "--head", branch])
        .args(["--json", "number", "--jq", "length"])
        .stderr(Stdio::null())
        .output();
    match output {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim() != "0"
        }
        _ => {
            *gh_missing = true;
            false
        }
    }
}

/// The folders of the worktrees of the repository `repo` is in, as `git
/// worktree list --porcelain` names them.
///
/// # Errors
///
/// git didn't start, or didn't list them.
pub(crate) fn worktree_paths(repo: &Path) -> Result<Vec<PathBuf>, String> {
    let listed = git(repo, &["worktree", "list", "--porcelain"])?;
    Ok(parse_porcelain(&listed)
        .into_iter()
        .map(|worktree| worktree.path)
        .collect())
}

/// Runs `git` in `dir`, its stdout as text.
fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("git did not start ({e})"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "git {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// `bytes` for a person: in GB (10⁹) with a tenth, or MB under one GB.
fn bytes(bytes: u64) -> String {
    const MB: u64 = 1_000_000;
    if bytes >= 1000 * MB {
        let tenths = bytes / (100 * MB);
        format!("{}.{} GB", tenths / 10, tenths % 10)
    } else {
        format!("{} MB", bytes.div_ceil(MB))
    }
}

mod stale;

#[cfg(test)]
mod tests;
