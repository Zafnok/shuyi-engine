//! The worked-example runner (ticket 1134, ADR-0043): a script of contract
//! commands in, a verdict per step out. A skill of the engine's skill pack
//! (`skills/`) keeps its worked example as such a script, and `cargo xtask
//! skill-examples` ([`crate::skills`]) runs each one on a fresh `shuyi new`
//! game; anything else that grades a game through the contract runs its
//! checks through [`run`] too.
//!
//! A script is a list of [`Step`]s, run in order in a game's folder:
//!
//! - [`Step::Shuyi`]: `cargo shuyi <args> --json`, its answer read as JSON:
//!   its `ok` must be the step's, and each [`Expect`] must hold of it;
//! - [`Step::Cargo`]: `cargo <args>`, which must succeed or fail as `ok`
//!   says;
//! - [`Step::Write`] and [`Step::Edit`]: a file of the game written from the
//!   script's own files, or one piece of its text replaced;
//! - [`Step::Holds`]: a file of the game holds a piece of text.
//!
//! [`run`] goes on after a failed step, so the verdicts say everything that
//! failed, and reads the answers as [`serde_json::Value`]: the xtask doesn't
//! depend on `shuyi-cli`.

use std::fmt::Write as _;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::Value;

/// The last lines of a failed command's output a verdict keeps.
const TAIL: usize = 20;

/// One step of a script.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Step {
    /// `cargo shuyi <args> --json` in the game's folder: the answer's `ok`
    /// must be `ok` (and the exit code 0 exactly when it is), and each of
    /// `expect` must hold of it. With `save`, the value at a JSON pointer
    /// of the answer is written to a file of the game: a string as it is,
    /// anything else as JSON.
    Shuyi {
        /// The arguments after `cargo shuyi`, without `--json`.
        args: Vec<String>,
        /// The verdict the answer must give.
        #[serde(default = "yes")]
        ok: bool,
        /// What must hold of the answer.
        #[serde(default)]
        expect: Vec<Expect>,
        /// A JSON pointer of the answer, and the file of the game to write
        /// its value to.
        #[serde(default)]
        save: Option<(String, String)>,
    },
    /// `cargo <args>` in the game's folder: it must exit 0 when `ok`, and
    /// with another code when not.
    Cargo {
        /// The arguments after `cargo`.
        args: Vec<String>,
        /// Whether it must succeed.
        #[serde(default = "yes")]
        ok: bool,
    },
    /// The file `to` of the game written with the script's own file
    /// `from`, its folders made.
    Write {
        /// The file in the game, from its folder.
        to: String,
        /// The script's file, from the script's folder.
        from: String,
    },
    /// The text `find`, which must be in the game's file `file` exactly
    /// once, replaced by `replace`.
    Edit {
        /// The file in the game, from its folder.
        file: String,
        /// The text to replace.
        find: String,
        /// What replaces it.
        replace: String,
    },
    /// The game's file `file` exists and holds `text` (any file, when it
    /// is empty).
    Holds {
        /// The file in the game, from its folder.
        file: String,
        /// The text it holds.
        #[serde(default)]
        text: String,
    },
}

/// The default of a step's `ok`: it passes.
fn yes() -> bool {
    true
}

/// What must hold of a contract answer, each at a JSON pointer
/// (`/result/top_screen`, RFC 6901).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Expect {
    /// The value there equals this: a string compared without its quotes,
    /// anything else as the JSON this reads as (`3`, `true`, `null`).
    Is(String, String),
    /// There is a value there, and it isn't `null`.
    Has(String),
    /// The array there holds an item that [`Expect::Is`] this, or the
    /// string there contains it.
    Holds(String, String),
}

/// How one step went.
#[derive(Debug, Clone, PartialEq)]
pub struct Verdict {
    /// Its place in the script, from 1.
    pub step: usize,
    /// What it did, as people read it (`cargo shuyi run --frames 1`).
    pub what: String,
    /// Whether it passed.
    pub passed: bool,
    /// Why it failed: what didn't hold, the answer's error and code, or
    /// the last lines of the command's output.
    pub detail: Option<String>,
    /// How long it took.
    pub took: Duration,
}

/// What a command did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ran {
    /// Its exit code; `None` when a signal ended it.
    pub code: Option<i32>,
    /// What it wrote on stdout.
    pub stdout: String,
    /// What it wrote on stderr.
    pub stderr: String,
}

/// Runs a script's commands. The tests fake it.
pub trait Runner {
    /// Runs `program` with `args` in `dir`. An error says it could not
    /// start.
    fn run(&self, dir: &Path, program: &str, args: &[String]) -> Result<Ran, String>;
}

/// The real commands, with `CARGO_TARGET_DIR` set to `target_dir` when
/// there is one, so games share what they build.
#[derive(Debug, Clone, Default)]
pub struct System {
    /// The target folder every command's cargo builds in.
    pub target_dir: Option<PathBuf>,
}

impl Runner for System {
    fn run(&self, dir: &Path, program: &str, args: &[String]) -> Result<Ran, String> {
        let mut command = Command::new(program);
        command.args(args).current_dir(dir);
        if let Some(target_dir) = &self.target_dir {
            command.env("CARGO_TARGET_DIR", target_dir);
        }
        let output = command
            .output()
            .map_err(|e| format!("`{program}` did not run ({e})"))?;
        Ok(Ran {
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Runs `steps` in the game at `game`, writing files from `files` (the
/// script's folder), with `runner`'s commands: a verdict per step, in
/// order. It goes on after a failure.
pub fn run(steps: &[Step], files: &Path, game: &Path, runner: &dyn Runner) -> Vec<Verdict> {
    steps
        .iter()
        .enumerate()
        .map(|(i, step)| {
            let started = Instant::now();
            let outcome = run_step(step, files, game, runner);
            Verdict {
                step: i + 1,
                what: what(step),
                passed: outcome.is_ok(),
                detail: outcome.err(),
                took: started.elapsed(),
            }
        })
        .collect()
}

/// A step, as people read it.
pub fn what(step: &Step) -> String {
    match step {
        Step::Shuyi { args, ok, .. } => {
            let fails = if *ok { "" } else { " (fails)" };
            format!("cargo shuyi {} --json{fails}", shown(args))
        }
        Step::Cargo { args, ok } => {
            let fails = if *ok { "" } else { " (fails)" };
            format!("cargo {}{fails}", shown(args))
        }
        Step::Write { to, from } => format!("write {to} from {from}"),
        Step::Edit { file, .. } => format!("edit {file}"),
        Step::Holds { file, text } if text.is_empty() => format!("{file} exists"),
        Step::Holds { file, .. } => format!("{file} holds its text"),
    }
}

/// Arguments as a shell would show them: one with a space quoted.
fn shown(args: &[String]) -> String {
    let shown: Vec<String> = args
        .iter()
        .map(|a| {
            if a.is_empty() || a.contains(' ') {
                format!("\"{a}\"")
            } else {
                a.clone()
            }
        })
        .collect();
    shown.join(" ")
}

/// Runs one step: nothing, or why it failed.
fn run_step(step: &Step, files: &Path, game: &Path, runner: &dyn Runner) -> Result<(), String> {
    match step {
        Step::Shuyi {
            args,
            ok,
            expect,
            save,
        } => {
            let mut full = vec!["shuyi".to_owned()];
            full.extend(args.iter().cloned());
            full.push("--json".to_owned());
            let ran = runner.run(game, "cargo", &full)?;
            let answer = answer_of(&ran, *ok)?;
            let failed: Vec<String> = expect
                .iter()
                .filter_map(|e| holds(&answer, e).err())
                .collect();
            if !failed.is_empty() {
                return Err(failed.join("; "));
            }
            if let Some((pointer, file)) = save {
                let value = answer
                    .pointer(pointer)
                    .ok_or_else(|| format!("save: no value at {pointer}"))?;
                let text = match value {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                write_file(&inside(game, file)?, text.as_bytes())?;
            }
            Ok(())
        }
        Step::Cargo { args, ok } => {
            let ran = runner.run(game, "cargo", args)?;
            if (ran.code == Some(0)) == *ok {
                Ok(())
            } else {
                let wanted = if *ok { "succeed" } else { "fail" };
                Err(format!(
                    "it exited with {}, and should {wanted}\n{}",
                    exit_shown(ran.code),
                    tail(&ran)
                ))
            }
        }
        Step::Write { to, from } => {
            let source = inside(files, from)?;
            let bytes = fs::read(&source).map_err(|e| format!("{from}: {e}"))?;
            write_file(&inside(game, to)?, &bytes)
        }
        Step::Edit {
            file,
            find,
            replace,
        } => {
            let path = inside(game, file)?;
            let text = fs::read_to_string(&path).map_err(|e| format!("{file}: {e}"))?;
            match text.matches(find.as_str()).count() {
                1 => write_file(&path, text.replacen(find, replace, 1).as_bytes()),
                0 => Err(format!("{file} doesn't hold the text to replace: {find:?}")),
                n => Err(format!(
                    "{file} holds the text to replace {n} times, not once: {find:?}"
                )),
            }
        }
        Step::Holds { file, text } => {
            let path = inside(game, file)?;
            if text.is_empty() {
                // Any file, text or not.
                return if path.is_file() {
                    Ok(())
                } else {
                    Err(format!("{file}: no such file"))
                };
            }
            let held = fs::read_to_string(&path).map_err(|e| format!("{file}: {e}"))?;
            if held.contains(text.as_str()) {
                Ok(())
            } else {
                Err(format!("{file} doesn't hold {text:?}"))
            }
        }
    }
}

/// The JSON answer of a contract command that `ran`, whose `ok` must be
/// `ok`; or why it is none, or the wrong one.
fn answer_of(ran: &Ran, ok: bool) -> Result<Value, String> {
    let answer: Value = serde_json::from_str(ran.stdout.trim()).map_err(|e| {
        format!(
            "no JSON answer ({e}); it exited with {}\n{}",
            exit_shown(ran.code),
            tail(ran)
        )
    })?;
    let said = answer.get("ok").and_then(Value::as_bool);
    if said != Some(ok) {
        let mut why = format!(
            "the answer's ok is {}, not {ok}",
            said.map_or_else(|| "missing".to_owned(), |s| s.to_string())
        );
        for field in ["error", "code", "help"] {
            if let Some(Value::String(text)) = answer.get(field) {
                // Writing to a `String` can't fail.
                let _ = write!(why, "; {field}: {text}");
            }
        }
        if let Some(Value::Array(limits)) = answer.get("limits")
            && !limits.is_empty()
        {
            let _ = write!(why, "; limits: {}", Value::Array(limits.clone()));
        }
        return Err(why);
    }
    if (ran.code == Some(0)) != ok {
        return Err(format!(
            "the answer's ok is {ok}, but it exited with {}",
            exit_shown(ran.code)
        ));
    }
    Ok(answer)
}

/// Nothing if `expect` holds of `answer`, or what doesn't.
pub fn holds(answer: &Value, expect: &Expect) -> Result<(), String> {
    let at = |pointer: &str| {
        answer
            .pointer(pointer)
            .ok_or_else(|| format!("no value at {pointer}"))
    };
    match expect {
        Expect::Is(pointer, want) => {
            let value = at(pointer)?;
            if same(value, want) {
                Ok(())
            } else {
                Err(format!("{pointer} is {value}, not {want}"))
            }
        }
        Expect::Has(pointer) => match at(pointer)? {
            Value::Null => Err(format!("{pointer} is null")),
            _ => Ok(()),
        },
        Expect::Holds(pointer, want) => match at(pointer)? {
            Value::Array(items) if items.iter().any(|item| same(item, want)) => Ok(()),
            Value::String(text) if text.contains(want.as_str()) => Ok(()),
            value @ (Value::Array(_) | Value::String(_)) => {
                Err(format!("{pointer} doesn't hold {want}: it is {value}"))
            }
            value => Err(format!(
                "{pointer} is no array or string to hold {want}: it is {value}"
            )),
        },
    }
}

/// Whether `value` is `want`: a string without its quotes, anything else
/// as the JSON `want` reads as.
fn same(value: &Value, want: &str) -> bool {
    match value {
        Value::String(text) => text == want,
        other => serde_json::from_str::<Value>(want).is_ok_and(|want| want == *other),
    }
}

/// `path` under `root`, or why it may not be: a path that leaves it.
fn inside(root: &Path, path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(path);
    let stays = relative
        .components()
        .all(|c| matches!(c, Component::Normal(_) | Component::CurDir));
    if path.is_empty() || !stays {
        return Err(format!(
            "{path:?}: a path from the folder, with no `..`, is needed"
        ));
    }
    Ok(root.join(relative))
}

/// Writes `bytes` to `path`, its folders made.
fn write_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// An exit code as people read it.
fn exit_shown(code: Option<i32>) -> String {
    code.map_or_else(|| "no code (a signal)".to_owned(), |c| c.to_string())
}

/// The last [`TAIL`] lines of what `ran` wrote: stderr, then stdout.
fn tail(ran: &Ran) -> String {
    let all = format!("{}{}", ran.stderr, ran.stdout);
    let lines: Vec<&str> = all.lines().collect();
    lines[lines.len().saturating_sub(TAIL)..].join("\n")
}

/// The verdicts as lines for people: each step, `ok` or `FAILED` and its
/// time, and why a failed one failed.
pub fn report(verdicts: &[Verdict]) -> String {
    let mut out = String::new();
    for verdict in verdicts {
        let mark = if verdict.passed { "ok" } else { "FAILED" };
        // Writing to a `String` can't fail.
        let _ = writeln!(
            out,
            "  {:>2}. {mark:<6} {} ({:.1} s)",
            verdict.step,
            verdict.what,
            verdict.took.as_secs_f64()
        );
        if let Some(detail) = &verdict.detail {
            for line in detail.lines() {
                let _ = writeln!(out, "             {line}");
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
