//! `cargo xtask eval-grade <task.ron> <project> [--base <dir>] [--out
//! <file>]`: a game graded against a task (ticket 2524). Every check runs
//! through the worked-example runner ([`crate::script::run`]) in the
//! game's folder, and the grader goes on after a failure. With a base, a
//! fresh `shuyi new <template> --name <name>`, the game's lines over it are
//! counted with `git diff --no-index`. The result is a JSON file.
//!
//! It reads no file of the game but `shuyi.ron` (for `web.dist`, left out
//! of the count) and the listing of the folder an export made, and writes
//! nothing into the game: every other input is a command's answer, and
//! what a step saves goes in a folder of the grader's own.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use serde_json::{Value, json};

use super::task::{After, Planned, Task};
use crate::script::{self, Runner};

/// The result file, by default, in the current folder.
pub const OUT: &str = "eval-result.json";

/// The folders and files of the game left out of its lines: what builds
/// and tools write, never an author.
const LEFT_OUT: [&str; 3] = ["target/", "dist/", ".git/"];

/// `eval-grade`'s options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// The task's file.
    pub task: PathBuf,
    /// The game's folder.
    pub project: PathBuf,
    /// `--base`: a fresh `shuyi new` of the task's template and name.
    pub base: Option<PathBuf>,
    /// `--out`: the result file.
    pub out: PathBuf,
}

impl Options {
    /// The options `args` give, or why they are wrong.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut free = Vec::new();
        let (mut base, mut out) = (None, PathBuf::from(OUT));
        let mut rest = args.iter();
        while let Some(arg) = rest.next() {
            match arg.as_str() {
                "--base" => base = Some(PathBuf::from(rest.next().ok_or("--base needs a folder")?)),
                "--out" => out = PathBuf::from(rest.next().ok_or("--out needs a file")?),
                other if other.starts_with("--") => return Err(format!("unknown option {other}")),
                other => free.push(PathBuf::from(other)),
            }
        }
        let [task, project] = <[PathBuf; 2]>::try_from(free)
            .map_err(|_| "a task file and a project folder are needed".to_owned())?;
        Ok(Self {
            task,
            project,
            base,
            out,
        })
    }
}

/// How one check went.
#[derive(Debug, Clone, PartialEq)]
pub struct Graded {
    /// Its kind and place in the task (`Scene 2`).
    pub name: String,
    /// Whether it passed.
    pub passed: bool,
    /// How long it took.
    pub took: Duration,
    /// Why it failed: the answer's error, a failed command's last lines,
    /// or what was equal that should have differed.
    pub detail: Option<String>,
}

/// A game's lines over its base.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Lines {
    /// Lines added.
    pub added: u64,
    /// Lines removed.
    pub removed: u64,
    /// Files changed, added or removed, binary ones among them.
    pub files: u64,
}

/// What grading found: the result file's JSON, and whether every check
/// passed.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The result, as `--out` holds it.
    pub result: Value,
    /// Whether every check passed.
    pub passed: bool,
    /// Each check, as people read it.
    pub report: String,
}

/// What a grade exits with: every check passed.
pub const PASSED: u8 = 0;
/// What a grade exits with: a check failed.
pub const FAILED: u8 = 1;
/// What a grade exits with: it couldn't grade (no such game, a bad task
/// file, a task that waits).
pub const UNGRADED: u8 = 2;

/// `eval-grade <args>` in the engine checkout at `root`: its exit code.
pub fn command(root: &Path, args: &[String], runner: &dyn Runner) -> u8 {
    let options = match Options::parse(args) {
        Ok(options) => options,
        Err(e) => {
            eprintln!("eval-grade: {e}");
            return UNGRADED;
        }
    };
    let task = match Task::read(&options.task) {
        Ok(task) => task,
        Err(e) => {
            eprintln!("eval-grade: {e}");
            return UNGRADED;
        }
    };
    let graded = to_file(
        &task,
        &options.project,
        options.base.as_deref(),
        root,
        &options.out,
        runner,
    );
    match graded {
        Ok(outcome) => {
            eprint!("{}", outcome.report);
            if outcome.passed { PASSED } else { FAILED }
        }
        Err(e) => {
            eprintln!("eval-grade: {e}");
            UNGRADED
        }
    }
}

/// Grades `game` against `task`, over `base` if given, on the engine
/// checkout at `root`, and writes the result to `out`; or why it couldn't.
pub fn to_file(
    task: &Task,
    game: &Path,
    base: Option<&Path>,
    root: &Path,
    out: &Path,
    runner: &dyn Runner,
) -> Result<Outcome, String> {
    if !task.waits_for.is_empty() {
        return Err(format!(
            "{} waits for {}: it isn't graded",
            task.id,
            task.waits_for.join(", ")
        ));
    }
    if !game.join("shuyi.ron").is_file() {
        return Err(format!("{}: no game (no shuyi.ron)", game.display()));
    }
    if let Some(base) = base
        && !base.join("shuyi.ron").is_file()
    {
        return Err(format!("--base {}: no game (no shuyi.ron)", base.display()));
    }
    let head = runner.run(root, "git", &["rev-parse".to_owned(), "HEAD".to_owned()])?;
    let engine = head.stdout.trim().to_owned();
    let scratch =
        std::env::temp_dir().join(format!("shuyi-eval-{}-{}", task.id, std::process::id()));
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).map_err(|e| format!("{}: {e}", scratch.display()))?;
    let outcome = grade(task, game, base, &engine, &scratch, runner);
    let _ = fs::remove_dir_all(&scratch);
    if let Some(parent) = out.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(&outcome.result).map_err(|e| e.to_string())?;
    fs::write(out, text + "\n").map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(outcome)
}

/// Grades `game` against `task` with `runner`'s commands, its steps'
/// files saved in `scratch`; counts its lines over `base` if there is one;
/// `engine` is the engine's commit.
pub fn grade(
    task: &Task,
    game: &Path,
    base: Option<&Path>,
    engine: &str,
    scratch: &Path,
    runner: &dyn Runner,
) -> Outcome {
    let graded: Vec<Graded> = task
        .plan(scratch)
        .iter()
        .map(|planned| run_check(planned, game, runner))
        .collect();
    let mut limits = Vec::new();
    let lines = match base {
        Some(base) => match count_lines(base, game, runner) {
            Ok(lines) => Some(lines),
            Err(e) => {
                limits.push(format!("the lines weren't counted: {e}"));
                None
            }
        },
        None => {
            limits.push("no --base: the lines over the template weren't counted".to_owned());
            None
        }
    };
    let passed = graded.iter().filter(|g| g.passed).count();
    let seconds: f64 = graded.iter().map(|g| g.took.as_secs_f64()).sum();
    let checks: Vec<Value> = graded
        .iter()
        .map(|g| {
            json!({ "check": g.name, "passed": g.passed,
                "seconds": round(g.took.as_secs_f64()), "detail": g.detail })
        })
        .collect();
    let unmeasured: Vec<Value> = task
        .unmeasured
        .iter()
        .map(|(what, waits_for)| json!({ "what": what, "waits_for": waits_for }))
        .collect();
    let result = json!({
        "task": task.id,
        "engine": engine,
        "checks": checks,
        "passed": passed,
        "total": graded.len(),
        "lines_added": lines.map(|l| l.added),
        "lines_removed": lines.map(|l| l.removed),
        "files_changed": lines.map(|l| l.files),
        "seconds": round(seconds),
        "unmeasured": unmeasured,
        "limits": limits,
    });
    Outcome {
        result,
        passed: passed == graded.len(),
        report: report(&task.id, &graded, lines),
    }
}

/// Seconds to a tenth.
fn round(seconds: f64) -> f64 {
    (seconds * 10.0).round() / 10.0
}

/// Runs one check's steps in order, stopping at the first that fails, then
/// what it looks at after them.
fn run_check(planned: &Planned, game: &Path, runner: &dyn Runner) -> Graded {
    let mut took = Duration::ZERO;
    for step in &planned.steps {
        let verdict = script::run(std::slice::from_ref(step), game, game, runner).remove(0);
        took += verdict.took;
        if !verdict.passed {
            let detail = verdict.detail.unwrap_or_default();
            return Graded {
                name: planned.name.clone(),
                passed: false,
                took,
                detail: Some(format!("{}: {detail}", verdict.what)),
            };
        }
    }
    let after = look_after(&planned.after, game);
    Graded {
        name: planned.name.clone(),
        passed: after.is_ok(),
        took,
        detail: after.err(),
    }
}

/// Nothing if what `after` names is there in `game`, or what isn't.
fn look_after(after: &After, game: &Path) -> Result<(), String> {
    let saved = |file: &str| -> Result<Value, String> {
        let text = fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?;
        serde_json::from_str(&text).map_err(|e| format!("{file}: {e}"))
    };
    let field = |result: &Value, name: &str| {
        result[name]
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| format!("the export's answer has no {name}"))
    };
    match after {
        After::Nothing => Ok(()),
        After::WebFolder(file) => {
            let result = saved(file)?;
            let folder = Path::new(&field(&result, "out")?).join(field(&result, "folder")?);
            let names: Vec<String> = fs::read_dir(game.join(&folder))
                .map_err(|e| format!("{}: {e}", folder.display()))?
                .flatten()
                .map(|entry| entry.file_name().to_string_lossy().into_owned())
                .collect();
            web_folder_holds(&names).map_err(|e| format!("{}: {e}", folder.display()))
        }
        After::Package(file) => {
            let result = saved(file)?;
            let package = Path::new(&field(&result, "out")?).join(field(&result, "package")?);
            if game.join(&package).exists() {
                Ok(())
            } else {
                Err(format!("{}: the package isn't there", package.display()))
            }
        }
    }
}

/// Nothing if a web build's folder of these file names holds `index.html`
/// and one `.wasm`, or what it lacks.
pub fn web_folder_holds(names: &[String]) -> Result<(), String> {
    let wasm = names.iter().filter(|n| n.ends_with(".wasm")).count();
    if !names.iter().any(|n| n == "index.html") {
        Err("no index.html".to_owned())
    } else if wasm != 1 {
        Err(format!("{wasm} .wasm files, not one"))
    } else {
        Ok(())
    }
}

/// The checks as lines for people, and the lines counted.
fn report(task: &str, graded: &[Graded], lines: Option<Lines>) -> String {
    let passed = graded.iter().filter(|g| g.passed).count();
    let mut out = format!("{task}: {passed} of {} check(s) passed\n", graded.len());
    for g in graded {
        let mark = if g.passed { "ok" } else { "FAILED" };
        // Writing to a `String` can't fail.
        let _ = writeln!(
            out,
            "  {mark:<6} {} ({:.1} s)",
            g.name,
            g.took.as_secs_f64()
        );
        for line in g.detail.iter().flat_map(|d| d.lines()) {
            let _ = writeln!(out, "         {line}");
        }
    }
    if let Some(l) = lines {
        let _ = writeln!(
            out,
            "  lines over the template: +{} -{}, {} file(s)",
            l.added, l.removed, l.files
        );
    }
    out
}

/// `shuyi.ron`'s part the grader reads: the web build's folder. Its other
/// parts are skipped.
#[derive(Debug, Default, Deserialize)]
struct ProjectFile {
    #[serde(default)]
    web: WebPart,
}

/// `shuyi.ron`'s `web`.
#[derive(Debug, Default, Deserialize)]
struct WebPart {
    #[serde(default)]
    dist: String,
}

/// The web build's folder `shuyi.ron`'s text names (`web.dist`), if any.
pub fn web_dist(shuyi_ron: &str) -> Option<String> {
    let file: ProjectFile = ron::from_str(shuyi_ron).ok()?;
    Some(file.web.dist).filter(|d| !d.is_empty())
}

/// `game`'s lines over `base`: `git diff --no-index --no-renames --numstat`,
/// less what [`left_out`] drops.
fn count_lines(base: &Path, game: &Path, runner: &dyn Runner) -> Result<Lines, String> {
    let dist = fs::read_to_string(game.join("shuyi.ron"))
        .ok()
        .and_then(|text| web_dist(&text));
    let shown = |p: &Path| p.to_string_lossy().replace('\\', "/");
    let (base, game_shown) = (shown(base), shown(game));
    let args: Vec<String> = [
        "-c",
        "core.autocrlf=false",
        "diff",
        "--no-index",
        "--no-renames",
        "--numstat",
        "-z",
        &base,
        &game_shown,
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    let ran = runner.run(game, "git", &args)?;
    // 1: they differ; 0: they don't.
    if !matches!(ran.code, Some(0 | 1)) {
        return Err(format!("git diff failed: {}", ran.stderr.trim()));
    }
    Ok(numstat(&ran.stdout, &base, &game_shown, dist.as_deref()))
}

/// The lines of `git diff --no-index --numstat -z`'s `out` between the
/// folders `base` and `game`, as given to it, less what [`left_out`] drops;
/// `dist` is the web build's folder.
pub fn numstat(out: &str, base: &str, game: &str, dist: Option<&str>) -> Lines {
    let mut lines = Lines::default();
    let mut fields = out.split('\0');
    // Each file: `added\tremoved\t`, then its path before and after, one
    // of them /dev/null when only one side has it.
    while let (Some(counts), Some(before), Some(after)) =
        (fields.next(), fields.next(), fields.next())
    {
        let path = if after == "/dev/null" {
            relative(before, base)
        } else {
            relative(after, game)
        };
        if left_out(path, dist) {
            continue;
        }
        let mut counts = counts.split('\t');
        let (added, removed) = (counts.next(), counts.next());
        lines.files += 1;
        // A binary file: `-` for both, a file changed only.
        lines.added += added.and_then(|n| n.parse().ok()).unwrap_or(0);
        lines.removed += removed.and_then(|n| n.parse().ok()).unwrap_or(0);
    }
    lines
}

/// `path` from the folder `root`, `/`-separated.
fn relative<'a>(path: &'a str, root: &str) -> &'a str {
    path.strip_prefix(root.trim_end_matches('/'))
        .map_or(path, |rest| rest.trim_start_matches('/'))
}

/// Whether the game's file `path` is left out of its lines: under
/// [`LEFT_OUT`] or the web build's folder `dist`, or a `Cargo.lock`.
pub fn left_out(path: &str, dist: Option<&str>) -> bool {
    let under = |folder: &str| {
        let folder = folder.trim_end_matches('/');
        path.strip_prefix(folder)
            .is_some_and(|rest| rest.starts_with('/'))
    };
    LEFT_OUT.iter().any(|folder| under(folder))
        || dist.is_some_and(under)
        || path == "Cargo.lock"
        || path.ends_with("/Cargo.lock")
}
