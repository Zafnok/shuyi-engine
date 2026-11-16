//! `trim-targets`: frees the disk the build output of a repository's idle
//! worktrees takes, and never removes, prunes or archives a worktree
//! (ticket 2076, ADR-0125).
//!
//! It lists the repository's worktrees (`git worktree list --porcelain`),
//! and leaves alone the one it runs in, one with no `target/`, one whose
//! `target/` is a link, one where cargo holds a lock (a build is running)
//! and one built in the last [`Options::idle_days`]. In each other one, it
//! deletes the `incremental` folder of every cargo profile folder under one
//! of [`Options::roots`]: cargo still finds the build Fresh, and only the
//! first edit there is slower. With [`Options::stale_hashes`], it also
//! deletes the older hashes of a crate in `deps/` that no build read for
//! as long; with [`Options::merged`], the whole content of `target/` of a
//! worktree whose branch is merged.
//!
//! Nothing outside `<worktree>/target/` is ever deleted, no link is
//! followed, no folder holding `.git` is entered, and deletes are best-effort: a file in use is left and
//! counted. `cargo xtask trim-targets` runs it on the engine's worktrees,
//! and `cargo xtask gates` at the end of a passing run.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

/// How many days without a build make a worktree idle, by default.
pub const DEFAULT_IDLE_DAYS: u64 = 3;

/// The files cargo locks while it builds in a profile folder: cargo
/// 1.99 adds the last two beside `.cargo-lock`.
const LOCK_FILES: [&str; 3] = [".cargo-lock", ".cargo-build-lock", ".cargo-artifact-lock"];

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
    /// Also delete, in each `deps/`, the older hashes of a crate that no
    /// build read in [`Options::idle_days`] (by the files' last access).
    pub stale_hashes: bool,
    /// Also delete all of `target/` of a worktree whose branch is merged
    /// into [`Options::base`], or whose pull request is merged (`gh`).
    pub merged: bool,
    /// The branch [`Options::merged`] compares with.
    pub base: String,
}

impl Options {
    /// The defaults for `roots`: idle after [`DEFAULT_IDLE_DAYS`], only
    /// `incremental` folders deleted, merged against `origin/main`.
    pub fn new(roots: Vec<String>) -> Self {
        Self {
            roots,
            idle_days: DEFAULT_IDLE_DAYS,
            dry_run: false,
            stale_hashes: false,
            merged: false,
            base: "origin/main".to_owned(),
        }
    }
}

/// What a run did, in all the worktrees.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Report {
    /// The worktrees something was deleted in (or would be, dry).
    pub trimmed: usize,
    /// The worktrees left alone: this one, missing, without `target/`,
    /// locked or built recently.
    pub kept: usize,
    /// The bytes deleted (or that would be), as the files' lengths: on a
    /// compressed `target/` the disk freed is less.
    pub freed: u64,
    /// The files that couldn't be deleted (in use), left where they are.
    pub skipped: u64,
    /// Whether the run deleted nothing ([`Options::dry_run`]).
    pub dry_run: bool,
    /// What was skipped in the whole run, and why (`--stale-hashes` with
    /// last-access updates off, `gh` missing).
    pub notes: Vec<String>,
}

impl Report {
    /// The run in one line, for the end of `cargo xtask gates`.
    pub fn summary(&self) -> String {
        let (verb, freed) = if self.dry_run {
            ("would trim", "would free")
        } else {
            ("trimmed", "freed")
        };
        let mut line = format!(
            "trim-targets: {verb} {} idle worktree(s), {freed} {}, left {} alone",
            self.trimmed,
            bytes(self.freed),
            self.kept
        );
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
    let cutoff = SystemTime::now()
        .checked_sub(DAY.saturating_mul(u32::try_from(options.idle_days).unwrap_or(u32::MAX)))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let mut report = Report {
        dry_run: options.dry_run,
        ..Report::default()
    };
    let last_access = if options.stale_hashes {
        let on = last_access_updates();
        if let Err(why) = &on {
            report.notes.push(format!("--stale-hashes skipped: {why}"));
        }
        on.is_ok()
    } else {
        false
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
                last_access,
            },
            worktree,
            &mut line,
            &mut gh_missing,
        );
        match freed {
            Some(freed) => {
                report.trimmed += 1;
                report.freed += freed.bytes;
                report.skipped += freed.skipped;
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
    last_access: bool,
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
    let (target, profiles) = idle_target(trim, worktree, line)?;
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
    if options.stale_hashes && trim.last_access {
        let before = freed.bytes;
        let files = delete_stale(trim, &target, &profiles, &mut freed);
        let _ = write!(
            line,
            ", {verb} {files} stale hash file(s) ({})",
            bytes(freed.bytes - before)
        );
    }
    if freed.bytes == 0 && freed.skipped == 0 {
        line.push_str(", nothing to delete");
    }
    if freed.skipped > 0 {
        let _ = write!(line, ", {} file(s) in use skipped", freed.skipped);
    }
    Some(freed)
}

/// The worktree's `target/` and its profile folders, when it is to be
/// trimmed; `None`, said on `line`, when it is left alone: this one, bare,
/// prunable, missing, without a `target/` of its own, locked, or built
/// after the cutoff.
fn idle_target(
    trim: &Trim,
    worktree: &Worktree,
    line: &mut String,
) -> Option<(PathBuf, Vec<PathBuf>)> {
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
                ", built {} h ago (in the last {} days), left alone",
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
    Some((target, profiles))
}

/// Deletes the stale hashes of `deps/` in each of `profiles` under the
/// roots (dry, counts them), adding to `freed`; how many files.
fn delete_stale(trim: &Trim, target: &Path, profiles: &[PathBuf], freed: &mut Freed) -> usize {
    let options = trim.options;
    let mut files = 0;
    for profile in profiles
        .iter()
        .filter(|p| under_roots(target, p, &options.roots))
    {
        for (file, len) in stale_files(&profile.join("deps"), trim.cutoff) {
            files += 1;
            if options.dry_run || fs::remove_file(&file).is_ok() {
                freed.bytes += len;
            } else {
                freed.skipped += 1;
            }
        }
    }
    files
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

/// Whether cargo holds a lock in `target`: on one of [`LOCK_FILES`] in
/// `target/` itself or in one of its profile folders.
fn locked(target: &Path, profiles: &[PathBuf]) -> bool {
    std::iter::once(target)
        .chain(profiles.iter().map(PathBuf::as_path))
        .flat_map(|dir| LOCK_FILES.iter().map(move |name| dir.join(name)))
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
/// `<root>/<triple>/<profile>` for one of `roots`.
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
        matches!(extra, Some(1 | 2))
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

/// The length of the files under `dir`, links not followed.
fn tree_size(dir: &Path) -> u64 {
    let mut freed = Freed::default();
    delete_tree(dir, true, &mut freed);
    freed.bytes
}

/// A `deps/` file's crate and kind: `libfoo-0123456789abcdef.rlib` is
/// (`libfoo`, `.rlib`). `None` for a name without a 16-digit hash.
fn split_hash(name: &str) -> Option<(&str, &str)> {
    name.match_indices('-').find_map(|(at, _)| {
        let rest = name.get(at + 1..)?;
        let hash = rest.get(..16)?;
        let after = rest.get(16..)?;
        (hash.bytes().all(|b| b.is_ascii_hexdigit())
            && (after.is_empty() || after.starts_with('.')))
        .then(|| (name.get(..at).unwrap_or_default(), after))
    })
}

/// The files of one crate and kind in `deps/`: each one's path, last
/// access and length.
type Group = Vec<(PathBuf, SystemTime, u64)>;

/// The files of `deps` whose crate and kind are there under more than one
/// hash, last read before `cutoff`, with their lengths. The most recently
/// read hash of each crate and kind is always kept.
fn stale_files(deps: &Path, cutoff: SystemTime) -> Vec<(PathBuf, u64)> {
    let Ok(entries) = fs::read_dir(deps) else {
        return Vec::new();
    };
    let mut groups: BTreeMap<(String, String), Group> = BTreeMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        let Ok(accessed) = meta.accessed() else {
            continue;
        };
        let name = entry.file_name().to_string_lossy().into_owned();
        if !meta.is_file() || is_link(&meta) {
            continue;
        }
        if let Some((stem, kind)) = split_hash(&name) {
            groups
                .entry((stem.to_owned(), kind.to_owned()))
                .or_default()
                .push((path, accessed, meta.len()));
        }
    }
    let mut stale = Vec::new();
    for mut files in groups.into_values().filter(|files| files.len() > 1) {
        files.sort_by_key(|(_, accessed, _)| *accessed);
        files.pop();
        stale.extend(
            files
                .into_iter()
                .filter(|(_, accessed, _)| *accessed < cutoff)
                .map(|(path, _, len)| (path, len)),
        );
    }
    stale
}

/// Whether the file system updates the files' last access, which
/// `--stale-hashes` reads; `Err` says why it can't be trusted. Asked of
/// `fsutil` on Windows; elsewhere assumed (Linux's `relatime` updates it
/// at least daily).
fn last_access_updates() -> Result<(), String> {
    if !cfg!(windows) {
        return Ok(());
    }
    let output = Command::new("fsutil")
        .args(["behavior", "query", "disablelastaccess"])
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("fsutil did not start ({e})"))?;
    match parse_disable_last_access(&String::from_utf8_lossy(&output.stdout)) {
        Some(true) => Ok(()),
        Some(false) => Err("NTFS last-access updates are off".to_owned()),
        None => Err("fsutil didn't say whether NTFS last-access updates are on".to_owned()),
    }
}

/// Whether `fsutil behavior query disablelastaccess`'s output says the
/// updates are on: its words (`… Updates ENABLED)`) or, without them, its
/// value, whose low bit is "disabled" (0 and 2 on, 1 and 3 off).
fn parse_disable_last_access(text: &str) -> Option<bool> {
    let upper = text.to_ascii_uppercase();
    if upper.contains("UPDATES ENABLED") {
        return Some(true);
    }
    if upper.contains("UPDATES DISABLED") {
        return Some(false);
    }
    let value = upper.split_once("DISABLELASTACCESS")?.1;
    let digits: String = value
        .trim_start_matches([' ', '='])
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse::<u32>().ok().map(|value| value & 1 == 0)
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

#[cfg(test)]
mod tests;
