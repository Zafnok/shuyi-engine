//! `cargo xtask eval-reference (<id> | --all) [--out <dir>]
//! [--template-only]`: a task's reference solution graded (ticket 3421).
//! Two games are made with `shuyi new` on this checkout, as the `skills`
//! job makes its games ([`skills::make_game`]): `target/eval/grade/<id>`,
//! which an `Overlay` reference is laid over (a `Project` one is graded
//! where it is), and `target/eval/base/<id>`, the base its lines are
//! counted over. They build in [`BUILD`] with no incremental cache, unless
//! `CARGO_TARGET_DIR` says where ([`skills::runners`]). The result is
//! `<out>/<id>.json`, [`grade::to_file`]'s.
//!
//! With `--template-only` the plain template is graded, to show that the
//! task's checks can fail; with `--all`, every task that has a reference
//! and waits for nothing, and the ones that wait are listed, with a
//! warning for each ticket they wait for that is done.

use std::fs;
use std::path::{Path, PathBuf};

use super::grade::{self, Base, FAILED, Over, PASSED, UNGRADED};
use super::task::{Reference, Task};
use crate::script::Runner;
use crate::skills;

/// Where the tasks are, from the repository's root.
pub const TASKS: &str = "evals/tasks";
/// Where the games are made, from the repository's root.
const GAMES: &str = "target/eval";
/// Where they build, from the repository's root, unless
/// `CARGO_TARGET_DIR` says.
pub const BUILD: &str = "target/eval-build";
/// The result folder, by default.
pub const OUT: &str = "eval-results";

/// `eval-reference`'s options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The task, by id; `None` with `--all`.
    pub id: Option<String>,
    /// `--out`: the folder of the results.
    pub out: PathBuf,
    /// `--template-only`: grade the plain template.
    pub template_only: bool,
}

impl Options {
    /// The options `args` give, or why they are wrong.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let (mut id, mut all, mut template_only) = (None, false, false);
        let mut out = PathBuf::from(OUT);
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--all" => all = true,
                "--template-only" => template_only = true,
                "--out" => out = PathBuf::from(rest.next().ok_or("--out needs a folder")?),
                other if other.starts_with("--") => return Err(format!("unknown option {other}")),
                other if id.is_none() => id = Some(other.to_owned()),
                other => return Err(format!("one task, not {other} too")),
            }
        }
        match (id.is_some(), all) {
            (true, true) => Err("a task id or --all, not both".to_owned()),
            (false, false) => Err("a task id or --all is needed".to_owned()),
            _ => Ok(Self {
                id,
                out,
                template_only,
            }),
        }
    }
}

/// Every task in `root`'s [`TASKS`], in id order, or the errors of reading
/// them (a file whose id isn't its name among them).
pub fn tasks(root: &Path) -> Result<Vec<Task>, Vec<String>> {
    let dir = root.join(TASKS);
    let entries = fs::read_dir(&dir).map_err(|e| vec![format!("{}: {e}", dir.display())])?;
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "ron"))
        .collect();
    files.sort();
    let (mut found, mut errors) = (Vec::new(), Vec::new());
    for file in files {
        match Task::read(&file) {
            Ok(task) if file.file_stem().is_some_and(|s| *s == *task.id) => found.push(task),
            Ok(task) => errors.push(format!(
                "{}: its id is {}, not its file's name",
                file.display(),
                task.id
            )),
            Err(e) => errors.push(e),
        }
    }
    if errors.is_empty() {
        Ok(found)
    } else {
        Err(errors)
    }
}

/// The tasks to grade of `all`: the one `id` names, or with none every one
/// with a reference that waits for nothing; or why there is none.
pub fn pick<'a>(all: &'a [Task], id: Option<&str>) -> Result<Vec<&'a Task>, String> {
    match id {
        Some(id) => (all.iter().find(|t| t.id == id))
            .map(|task| vec![task])
            .ok_or_else(|| format!("no task {id} in {TASKS}/")),
        None => Ok(all
            .iter()
            .filter(|t| t.waits_for.is_empty() && t.reference != Reference::None)
            .collect()),
    }
}

/// The tasks of `all` that wait, as people read them: `platformer (waits
/// for 1127)`.
pub fn waiting(all: &[Task]) -> Vec<String> {
    (all.iter())
        .filter(|t| !t.waits_for.is_empty())
        .map(|t| format!("{} (waits for {})", t.id, t.waits_for.join(", ")))
        .collect()
}

/// The warnings for `task`'s `waits_for` ids that `root`'s
/// `tickets/done/` holds.
pub fn done_waits(root: &Path, task: &Task) -> Vec<String> {
    let done: Vec<String> = fs::read_dir(root.join("tickets/done"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    (task.waits_for.iter())
        .filter(|id| done.iter().any(|f| f.starts_with(&format!("{id}-"))))
        .map(|id| {
            format!(
                "{id} is done: give `{}` a reference and empty its `waits_for`",
                task.id
            )
        })
        .collect()
}

/// Why the overlay folder `dir` may not be laid over a game, if it may
/// not: a `Cargo.toml` that says where the engine's crates are, which
/// `shuyi new --engine-path` says. One naming them as the workspace's
/// (`shuyi-core.workspace = true`) may be laid over.
pub fn refuse_overlay(dir: &Path) -> Option<String> {
    files_under(dir).into_iter().find_map(|file| {
        if file.file_name()? != "Cargo.toml" {
            return None;
        }
        let text = fs::read_to_string(&file).ok()?;
        places_engine_crate(&text).then(|| {
            format!(
                "{}: an overlay's Cargo.toml may not say where the engine's crates are \
                 (`shuyi new --engine-path` does)",
                file.display()
            )
        })
    })
}

/// Whether a manifest's `text` names an engine crate (`shuyi`,
/// `shuyi-…`) other than as the workspace's: with a path, a git
/// repository or a version, or as a table of its own.
pub fn places_engine_crate(text: &str) -> bool {
    text.lines().any(|line| {
        let line = line.trim_start();
        if line.starts_with('[') {
            return line.contains(".shuyi]") || line.contains(".shuyi-");
        }
        let key = line.trim_start_matches('"');
        let names = key.strip_prefix("shuyi").is_some_and(|rest| {
            if rest.starts_with(|c: char| c.is_ascii_alphanumeric()) {
                // `shuyiness`: another crate.
                return false;
            }
            let rest = rest
                .trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_');
            let rest = rest.trim_start_matches('"').trim_start();
            rest.starts_with('=') || rest.starts_with('.')
        });
        names && !line.contains("workspace = true")
    })
}

/// Every file under `dir`, in name order.
fn files_under(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .collect();
    entries.sort();
    let mut files = Vec::new();
    for path in entries {
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

/// Copies every file under `overlay` into `game` at its path.
fn lay_over(overlay: &Path, game: &Path) -> Result<(), String> {
    if !overlay.is_dir() {
        return Err(format!("{}: no such folder", overlay.display()));
    }
    for file in files_under(overlay) {
        let to = game.join(file.strip_prefix(overlay).map_err(|e| e.to_string())?);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::copy(&file, &to).map_err(|e| format!("{}: {e}", file.display()))?;
    }
    Ok(())
}

/// `eval-reference <args>` in the engine checkout at `root`, its games made
/// with `maker` and graded with `runner`: its exit code, the worst of the
/// tasks', [`PASSED`], [`FAILED`] or [`UNGRADED`].
pub fn command(root: &Path, args: &[String], maker: &dyn Runner, runner: &dyn Runner) -> u8 {
    let options = match Options::parse(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("eval-reference: {e}");
            return UNGRADED;
        }
    };
    let all = match tasks(root) {
        Ok(all) => all,
        Err(errors) => {
            for e in errors {
                eprintln!("eval-reference: {e}");
            }
            return UNGRADED;
        }
    };
    let picked = match pick(&all, options.id.as_deref()) {
        Ok(picked) => picked,
        Err(e) => {
            eprintln!("eval-reference: {e}");
            return UNGRADED;
        }
    };
    if options.id.is_none() {
        for task in waiting(&all) {
            eprintln!("waiting: {task}");
        }
    }
    for task in &all {
        for warning in done_waits(root, task) {
            eprintln!("warning: {warning}");
        }
    }
    let out = root.join(&options.out);
    (picked.into_iter())
        .map(|task| one(root, task, &out, options.template_only, maker, runner))
        .fold(PASSED, u8::max)
}

/// Grades `task`'s reference (or its plain template) into `out`: its exit
/// code.
fn one(
    root: &Path,
    task: &Task,
    out: &Path,
    template_only: bool,
    maker: &dyn Runner,
    runner: &dyn Runner,
) -> u8 {
    let fail = |e: String| {
        eprintln!("eval-reference: {}: {e}", task.id);
        UNGRADED
    };
    if !task.waits_for.is_empty() {
        return fail(format!("waits for {}", task.waits_for.join(", ")));
    }
    if let (Reference::Overlay(dir), false) = (&task.reference, template_only)
        && let Some(why) = refuse_overlay(&root.join(dir))
    {
        return fail(why);
    }
    let games = root.join(GAMES);
    let (game, base) = (
        games.join("grade").join(&task.id),
        games.join("base").join(&task.id),
    );
    let made = |dir: &Path| skills::make_game(root, dir, &task.template, &task.name, maker);
    if let Err(e) = made(&base) {
        return fail(format!("{}: {e}", base.display()));
    }
    let (graded, over) = match (&task.reference, template_only) {
        (Reference::Project(dir), false) => (root.join(dir), Over::GamesFiles),
        (Reference::None, false) => return fail("no reference to grade".to_owned()),
        (reference, _) => {
            let laid = made(&game).map_err(|e| format!("{}: {e}", game.display()));
            let laid = laid.and_then(|()| match (reference, template_only) {
                (Reference::Overlay(dir), false) => lay_over(&root.join(dir), &game),
                _ => Ok(()),
            });
            if let Err(e) = laid {
                return fail(e);
            }
            (game, Over::Everything)
        }
    };
    let file = out.join(format!("{}.json", task.id));
    eprintln!("{}: grading {}", task.id, graded.display());
    let base = Base { dir: &base, over };
    match grade::to_file(task, &graded, Some(base), root, &file, runner) {
        Ok(outcome) => {
            eprint!("{}", outcome.report);
            eprintln!("{}: {}", task.id, file.display());
            if outcome.passed { PASSED } else { FAILED }
        }
        Err(e) => fail(e),
    }
}
