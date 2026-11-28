//! `cargo xtask tutorial-steps` (ticket 2858, ADR-0168): the book's
//! project written once.
//!
//! The project's first step, `site/tutorial/01-new/`, is written by hand.
//! Every later step is the one before it plus one patch,
//! `site/tutorial-steps/patches/<step>.patch`: the unified diff from the
//! step before, lock files left out. Each step's `Cargo.lock` is one of
//! the few kept in `site/tutorial-steps/locks/`, as [`LOCK_GROUPS`] says.
//! The step folders stay checked in, where the book, the exercises, CI and
//! the Codespace read them, but they are made from that source:
//!
//! - `--write` makes them: a copy of `01-new` with each patch applied in
//!   turn (`git apply`, in `target/tutorial-steps/`), each step's lock put
//!   in;
//! - `--check` fails on a folder that isn't what `--write` makes, naming
//!   its first file that differs;
//! - `--record <NN>` writes the patches of step NN and of the step after
//!   it from the folders as edited: edit a step's folder, then record it;
//! - `--locks` keeps the locks the steps' folders hold after a refresh
//!   (`cargo metadata` in each), when the steps of each group agree.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The steps' folders, from the root.
pub const STEPS: &str = "site/tutorial";
/// The steps' source: the patches and the locks, from the root.
pub const SOURCE: &str = "site/tutorial-steps";
/// One patch a step after the first, `<step>.patch`.
const PATCHES: &str = "site/tutorial-steps/patches";
/// One folder a group of [`LOCK_GROUPS`], holding its `Cargo.lock`.
const LOCKS: &str = "site/tutorial-steps/locks";
/// Where the steps are made, from the root.
const SCRATCH: &str = "target/tutorial-steps";
/// The step written by hand.
const FIRST: &str = "01-new";
/// A step's lock, which no patch holds.
const LOCK: &str = "Cargo.lock";
/// What a step's folder holds that is no part of it: its builds, and the
/// pictures a test writes beside the ones it keeps (its `.gitignore`).
const SKIP: [&str; 3] = ["target/**", "dist/**", "**/*.png.new"];
/// The command that records a step's edit, before its number.
const RECORD: &str = "cargo xtask tutorial-steps --record";

/// Which steps take which lock: the lock's folder under
/// `site/tutorial-steps/locks/`, named after its first step, and the steps
/// whose `Cargo.lock` it is. A step whose dependencies change moves to a
/// group of its own, with its lock.
pub const LOCK_GROUPS: &[(&str, &[&str])] = &[
    ("01-new", &["01-new"]),
    (
        "02-board",
        &["02-board", "03-walk", "04-push", "05-undo", "06-levels"],
    ),
    ("07-save", &["07-save", "08-polish", "09-test", "10-ship"]),
];

/// A folder's files: each path from the folder, `/`-separated, and its
/// bytes.
pub type Tree = BTreeMap<String, Vec<u8>>;

/// What the command does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    /// `--write`: make the folders.
    Write,
    /// `--check`: compare them with what `--write` makes.
    Check,
    /// `--record <NN>`: write step NN's patch and the next one's.
    Record(String),
    /// `--locks`: keep the folders' locks.
    Locks,
}

impl Mode {
    /// `--write`, `--check`, `--record <NN>` or `--locks`; `None` for
    /// anything else.
    pub fn parse(args: &[String]) -> Option<Self> {
        match args {
            [flag] if flag == "--write" => Some(Self::Write),
            [flag] if flag == "--check" => Some(Self::Check),
            [flag] if flag == "--locks" => Some(Self::Locks),
            [flag, number]
                if flag == "--record"
                    && number.len() == 2
                    && number.bytes().all(|b| b.is_ascii_digit()) =>
            {
                Some(Self::Record(number.clone()))
            }
            _ => None,
        }
    }
}

/// Runs `mode` over the steps under `root`: what failed, or nothing.
pub fn run(root: &Path, mode: &Mode) -> Vec<String> {
    let done = match mode {
        Mode::Write => write(root),
        Mode::Check => return check(root),
        Mode::Record(number) => record(root, number),
        Mode::Locks => keep_locks(root),
    };
    done.err().into_iter().collect()
}

/// The steps in order: [`FIRST`], then one a patch. Fails on a patch not
/// named `NN-<name>.patch`, and on a step folder (one with a `shuyi.ron`)
/// that is neither.
pub fn steps(root: &Path) -> Result<Vec<String>, String> {
    let mut steps = vec![FIRST.to_owned()];
    for name in names(&root.join(PATCHES))? {
        let step = name
            .strip_suffix(".patch")
            .filter(|step| is_step_name(step))
            .ok_or_else(|| format!("{PATCHES}/{name}: a patch is named NN-<step>.patch"))?;
        steps.push(step.to_owned());
    }
    steps.sort();
    if steps[1..].iter().any(|step| step == FIRST) {
        return Err(format!(
            "{PATCHES}/{FIRST}.patch: {FIRST} is written by hand, with no patch"
        ));
    }
    for name in names(&root.join(STEPS))? {
        if root.join(STEPS).join(&name).join("shuyi.ron").is_file() && !steps.contains(&name) {
            return Err(format!(
                "{STEPS}/{name}: a step with no patch: record it ({RECORD} {}) or remove the folder",
                number(&name)
            ));
        }
    }
    Ok(steps)
}

/// Whether `name` is a step's: two digits, `-`, then a name.
fn is_step_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() > 3 && bytes[..2].iter().all(u8::is_ascii_digit) && bytes[2] == b'-'
}

/// A step's number: the two digits its name starts with.
fn number(step: &str) -> &str {
    step.get(..2).unwrap_or(step)
}

/// The names in folder `dir`, sorted.
fn names(dir: &Path) -> Result<Vec<String>, String> {
    let read = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut names = Vec::new();
    for entry in read {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort();
    Ok(names)
}

/// The lock's folder under [`LOCKS`] of `step`, from [`LOCK_GROUPS`].
pub fn lock_of(step: &str) -> Option<&'static str> {
    LOCK_GROUPS
        .iter()
        .find(|(_, steps)| steps.contains(&step))
        .map(|(lock, _)| *lock)
}

/// The files of folder `dir`, but for [`SKIP`].
pub fn read_tree(dir: &Path) -> Result<Tree, String> {
    let skip: Vec<String> = SKIP.iter().map(|&s| s.to_owned()).collect();
    let walk = shuyi_embed::walk(dir, &skip).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut tree = Tree::new();
    for entry in walk.files {
        let bytes = fs::read(&entry.file).map_err(|e| format!("{}: {e}", entry.file.display()))?;
        tree.insert(entry.path, bytes);
    }
    Ok(tree)
}

/// Writes every file of `tree` under `dir`, making its folders.
fn write_tree(dir: &Path, tree: &Tree) -> Result<(), String> {
    for (path, bytes) in tree {
        let file = dir.join(path);
        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&file, bytes).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(())
}

/// Removes folder `dir` if it is there.
fn remove(dir: &Path) -> Result<(), String> {
    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    }
    Ok(())
}

/// `git` with none of the person's settings that change a patch or how
/// one is applied, and no repository found around the folder it runs in:
/// a `git apply` inside a repository's folder reads the patch's paths from
/// the repository's top, and leaves out, silently, those outside it.
fn git(dir: &Path) -> Command {
    let mut git = Command::new("git");
    git.current_dir(dir)
        .env("GIT_CEILING_DIRECTORIES", dir.parent().unwrap_or(dir))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_PREFIX")
        .args([
            "-c",
            "core.autocrlf=false",
            "-c",
            "core.safecrlf=false",
            "-c",
            "core.eol=lf",
            "-c",
            "core.quotePath=true",
            "-c",
            "diff.suppressBlankEmpty=false",
            "-c",
            "diff.noprefix=false",
            "-c",
            "diff.mnemonicPrefix=false",
            "-c",
            "diff.relative=false",
        ]);
    git
}

/// The patch from `old` to `new`: `git diff --no-index` between the two
/// written into `scratch` as folders `old_name` and `new_name`, so its
/// paths read `<old_name>/<file>` and `<new_name>/<file>`; [`clean`]ed.
pub fn diff(
    scratch: &Path,
    (old_name, old): (&str, &Tree),
    (new_name, new): (&str, &Tree),
) -> Result<String, String> {
    remove(scratch)?;
    write_tree(&scratch.join(old_name), old)?;
    write_tree(&scratch.join(new_name), new)?;
    let output = git(scratch)
        .args([
            "diff",
            "--no-index",
            "--no-prefix",
            "--binary",
            "--full-index",
            "--no-renames",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--unified=3",
            "--inter-hunk-context=0",
            "--diff-algorithm=myers",
            "--indent-heuristic",
            old_name,
            new_name,
        ])
        .output()
        .map_err(|e| format!("git diff: {e}"))?;
    // 1: the folders differ; 0: they don't.
    if !matches!(output.status.code(), Some(0 | 1)) {
        return Err(format!(
            "git diff {old_name} {new_name}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|_| format!("git diff {old_name} {new_name}: not UTF-8"))?;
    Ok(clean(&text))
}

/// `patch` without the `index` lines of its text files: they name blobs,
/// which only a binary file's patch needs.
pub fn clean(patch: &str) -> String {
    let mut out = String::with_capacity(patch.len());
    for section in sections(patch) {
        let binary = section.lines().any(|line| line == "GIT binary patch");
        let mut header = true;
        for line in section.split_inclusive('\n') {
            if line.starts_with("@@") {
                header = false;
            }
            if header && !binary && line.starts_with("index ") {
                continue;
            }
            out.push_str(line);
        }
    }
    out
}

/// The sections of `patch`, one a file, each starting `diff --git `.
fn sections(patch: &str) -> Vec<&str> {
    let mut starts: Vec<usize> = patch
        .match_indices("diff --git ")
        .map(|(at, _)| at)
        .filter(|&at| at == 0 || patch.as_bytes()[at - 1] == b'\n')
        .collect();
    if starts.first() != Some(&0) {
        starts.insert(0, 0);
    }
    starts.push(patch.len());
    starts
        .windows(2)
        .map(|pair| &patch[pair[0]..pair[1]])
        .filter(|s| !s.is_empty())
        .collect()
}

/// Applies the patch in file `patch` to folder `dir`.
pub fn apply(dir: &Path, patch: &Path) -> Result<(), String> {
    let output = git(dir)
        .args(["apply", "--whitespace=nowarn", "-p1"])
        .arg(patch)
        .output()
        .map_err(|e| format!("git apply: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{}: doesn't apply: {}",
            patch.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// The locks of [`LOCK_GROUPS`], by the folder they are in.
fn read_locks(root: &Path) -> Result<BTreeMap<&'static str, Vec<u8>>, String> {
    let mut locks = BTreeMap::new();
    for (lock, _) in LOCK_GROUPS {
        let file = root.join(LOCKS).join(lock).join(LOCK);
        let bytes = fs::read(&file).map_err(|e| format!("{LOCKS}/{lock}/{LOCK}: {e}"))?;
        locks.insert(*lock, bytes);
    }
    Ok(locks)
}

/// The steps [`make`] made, in order, and why it stopped before the rest:
/// a patch that no longer applies.
pub type Made = (Vec<(String, Tree)>, Option<String>);

/// Each of the first `count` steps as `--write` makes it: [`FIRST`]'s
/// folder, then each patch applied to the step before it; each with its
/// lock. It stops at a patch that doesn't apply, with the steps before it.
pub fn make(root: &Path, count: usize) -> Result<Made, String> {
    let steps = steps(root)?;
    let locks = read_locks(root)?;
    let scratch = root.join(SCRATCH);
    let work = scratch.join("work");
    remove(&scratch)?;
    let mut tree = read_tree(&root.join(STEPS).join(FIRST))?;
    tree.remove(LOCK);
    write_tree(&work, &tree)?;
    let mut made = Vec::new();
    for (index, step) in steps.iter().take(count).enumerate() {
        if index > 0 {
            if let Err(e) = apply(&work, &root.join(PATCHES).join(format!("{step}.patch"))) {
                remove(&scratch)?;
                let fix = format!(
                    "an edit before {step} changed lines its patch reads: make the edit in {STEPS}/{step} too (and in each later step that should have it), then {RECORD} {}",
                    number(step)
                );
                return Ok((
                    made,
                    Some(format!(
                        "{e}
{fix}"
                    )),
                ));
            }
            tree = read_tree(&work)?;
        }
        let lock = lock_of(step).ok_or_else(|| {
            format!("{STEPS}/{step}: no lock: add it to LOCK_GROUPS (crates/xtask/src/tutorial_steps.rs)")
        })?;
        let mut whole = tree.clone();
        whole.insert(LOCK.to_owned(), locks[lock].clone());
        made.push((step.clone(), whole));
    }
    remove(&scratch)?;
    Ok((made, None))
}

/// The first file where `actual` (the folder `dir`, from the root) isn't
/// `expected`, as an error, or `None` when they are the same.
pub fn first_difference(dir: &str, expected: &Tree, actual: &Tree) -> Option<String> {
    let paths: std::collections::BTreeSet<&String> = expected.keys().chain(actual.keys()).collect();
    let step = dir.rsplit('/').next().unwrap_or(dir);
    let fix = if step == FIRST {
        format!(
            "keep {LOCKS}/{}/{LOCK} (--locks)",
            lock_of(step).unwrap_or(FIRST)
        )
    } else {
        format!(
            "record the edit ({RECORD} {}) or make the step again (--write)",
            number(step)
        )
    };
    paths.into_iter().find_map(|path| {
        let what = match (expected.get(path), actual.get(path)) {
            (Some(want), Some(have)) if want == have => return None,
            (Some(_), Some(_)) => "not what the source makes",
            (Some(_), None) => "missing: the source makes it",
            (None, _) => "not made by the source",
        };
        let fix = if path == LOCK {
            format!(
                "keep {LOCKS}/{}/{LOCK} after a refresh (--locks), or put it back (--write)",
                lock_of(step).unwrap_or(step)
            )
        } else {
            fix.clone()
        };
        Some(format!("{dir}/{path}: {what}: {fix}"))
    })
}

/// `--check`: each step's folder against what `--write` makes.
fn check(root: &Path) -> Vec<String> {
    let (made, stopped) = match make(root, usize::MAX) {
        Ok(made) => made,
        Err(e) => return vec![e],
    };
    let mut errors: Vec<String> = stopped.into_iter().collect();
    for (step, expected) in made {
        let dir = format!("{STEPS}/{step}");
        let actual = match read_tree(&root.join(&dir)) {
            Ok(tree) => tree,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        // The first step's files are the source: only its lock is made.
        let expected = if step == FIRST {
            let mut tree = actual.clone();
            tree.insert(LOCK.to_owned(), expected[LOCK].clone());
            tree
        } else {
            expected
        };
        errors.extend(first_difference(&dir, &expected, &actual));
    }
    errors
}

/// `--write`: makes each step's folder what the source makes, up to a
/// patch that doesn't apply.
fn write(root: &Path) -> Result<(), String> {
    let (made, stopped) = make(root, usize::MAX)?;
    for (step, tree) in made {
        let dir = root.join(STEPS).join(&step);
        if step == FIRST {
            let file = dir.join(LOCK);
            fs::write(&file, &tree[LOCK]).map_err(|e| format!("{}: {e}", file.display()))?;
        } else {
            mirror(&dir, &tree)?;
        }
    }
    stopped.map_or(Ok(()), Err)
}

/// Makes folder `dir` hold `tree`, but for [`SKIP`]: removes the files
/// `tree` hasn't, and the folders that leaves empty, and writes those that
/// differ.
pub fn mirror(dir: &Path, tree: &Tree) -> Result<(), String> {
    let had = if dir.exists() {
        read_tree(dir)?
    } else {
        Tree::new()
    };
    for path in had.keys().filter(|path| !tree.contains_key(*path)) {
        let file = dir.join(path);
        fs::remove_file(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        // The folders it was in, while they are empty.
        let mut folder = file.parent();
        while let Some(parent) = folder.filter(|f| *f != dir) {
            if fs::remove_dir(parent).is_err() {
                break;
            }
            folder = parent.parent();
        }
    }
    let changed: Tree = tree
        .iter()
        .filter(|(path, bytes)| had.get(*path) != Some(*bytes))
        .map(|(path, bytes)| (path.clone(), bytes.clone()))
        .collect();
    write_tree(dir, &changed)
}

/// `--record <number>`: writes the patch of step `number` from the step
/// before it as `--write` makes it to its folder, and the next step's from
/// that folder to the next one's.
fn record(root: &Path, wanted: &str) -> Result<(), String> {
    let steps = steps(root)?;
    let index = steps
        .iter()
        .position(|step| number(step) == wanted)
        .ok_or_else(|| format!("--record {wanted}: no step {wanted} in {STEPS}"))?;
    let folder = |step: &str| -> Result<Tree, String> {
        let mut tree = read_tree(&root.join(STEPS).join(step))?;
        tree.remove(LOCK);
        Ok(tree)
    };
    let scratch = root.join(SCRATCH);
    let edited = folder(&steps[index])?;
    if index > 0 {
        let (mut made, stopped) = make(root, index)?;
        if let Some(e) = stopped {
            return Err(e);
        }
        let (before, mut made) = made.pop().ok_or("no step before it")?;
        made.remove(LOCK);
        write_patch(root, &scratch, (&before, &made), (&steps[index], &edited))?;
    }
    if let Some(next) = steps.get(index + 1) {
        write_patch(
            root,
            &scratch,
            (&steps[index], &edited),
            (next, &folder(next)?),
        )?;
    }
    remove(&scratch)
}

/// Writes the patch of step `new` (from step `old`).
fn write_patch(
    root: &Path,
    scratch: &Path,
    old: (&str, &Tree),
    new: (&str, &Tree),
) -> Result<(), String> {
    let patch = diff(scratch, old, new)?;
    let file = root.join(PATCHES).join(format!("{}.patch", new.0));
    fs::write(&file, patch).map_err(|e| format!("{}: {e}", file.display()))
}

/// `--locks`: keeps each group's lock from its steps' folders, when they
/// all hold the same one.
fn keep_locks(root: &Path) -> Result<(), String> {
    for (lock, group) in LOCK_GROUPS {
        let mut kept: Option<(&str, Vec<u8>)> = None;
        for step in *group {
            let file = root.join(STEPS).join(step).join(LOCK);
            let bytes = fs::read(&file).map_err(|e| format!("{STEPS}/{step}/{LOCK}: {e}"))?;
            match &kept {
                Some((first, held)) if *held != bytes => {
                    return Err(format!(
                        "{STEPS}/{step}/{LOCK} isn't {STEPS}/{first}/{LOCK}: the steps of one lock refresh to the same one; move {step} to a group of its own in LOCK_GROUPS (crates/xtask/src/tutorial_steps.rs)"
                    ));
                }
                Some(_) => {}
                None => kept = Some((step, bytes)),
            }
        }
        if let Some((_, bytes)) = kept {
            let dir = root.join(LOCKS).join(lock);
            fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
            let file: PathBuf = dir.join(LOCK);
            fs::write(&file, bytes).map_err(|e| format!("{}: {e}", file.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
