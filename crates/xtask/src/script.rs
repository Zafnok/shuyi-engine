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
//! - [`Step::Holds`]: a file of the game holds a piece of text;
//! - [`Step::Mcp`]: `cargo shuyi mcp` started once, and its session tools
//!   (`start`, `step`, `perf`, …) called in order, each [`McpCall`]'s
//!   `isError` against its `ok` and its `structuredContent` read with the
//!   same [`Expect`]s (ticket 2563).
//!
//! [`run`] goes on after a failed step, so the verdicts say everything that
//! failed, and reads the answers as [`serde_json::Value`]: the xtask doesn't
//! depend on `shuyi-cli`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::Deserialize;
use serde_json::{Value, json};

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
    /// `cargo shuyi mcp` started once in the game's folder: `initialize`,
    /// then a `tools/call` per call, in order, and its input closed. Each
    /// call's `isError` must agree with its `ok`, and each of its `expect`
    /// must hold of its `structuredContent` (the contract answer).
    Mcp {
        /// The tools called, in order.
        calls: Vec<McpCall>,
    },
}

/// One tool called in a [`Step::Mcp`].
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpCall {
    /// The tool's name (`start`, `perf`).
    pub tool: String,
    /// Its arguments, a map (`{"frames": 30}`); none by default.
    #[serde(default = "no_input")]
    pub input: Value,
    /// The verdict its answer must give.
    #[serde(default = "yes")]
    pub ok: bool,
    /// What must hold of its `structuredContent`.
    #[serde(default)]
    pub expect: Vec<Expect>,
}

/// The default of a step's `ok`: it passes.
fn yes() -> bool {
    true
}

/// The default of a call's `input`: no arguments.
fn no_input() -> Value {
    json!({})
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

    /// Runs `program` with `args` in `dir`, `input` written to its stdin,
    /// which then closes. A runner that can't says so.
    fn run_with_input(
        &self,
        dir: &Path,
        program: &str,
        args: &[String],
        input: &str,
    ) -> Result<Ran, String> {
        let _ = (dir, args, input);
        Err(format!("this runner can't write to `{program}`'s input"))
    }
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
        Ok(ran_of(&output))
    }

    fn run_with_input(
        &self,
        dir: &Path,
        program: &str,
        args: &[String],
        input: &str,
    ) -> Result<Ran, String> {
        let mut command = Command::new(program);
        command
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(target_dir) = &self.target_dir {
            command.env("CARGO_TARGET_DIR", target_dir);
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("`{program}` did not run ({e})"))?;
        // Written from a thread while the output is read, so no pipe fills;
        // a program that ends early leaves the rest unread, and its output
        // says why.
        let writer = child.stdin.take().map(|mut stdin| {
            let input = input.to_owned();
            std::thread::spawn(move || {
                let _ = stdin.write_all(input.as_bytes());
            })
        });
        let output = child
            .wait_with_output()
            .map_err(|e| format!("`{program}` did not end ({e})"))?;
        if let Some(writer) = writer {
            let _ = writer.join();
        }
        Ok(ran_of(&output))
    }
}

/// What a command whose output is `output` did.
fn ran_of(output: &std::process::Output) -> Ran {
    Ran {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
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
        Step::Mcp { calls } => {
            let tools: Vec<String> = calls
                .iter()
                .map(|c| {
                    let fails = if c.ok { "" } else { " (fails)" };
                    format!("{}{fails}", c.tool)
                })
                .collect();
            format!("cargo shuyi mcp: {}", tools.join(", "))
        }
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
        Step::Mcp { calls } => {
            let args = ["shuyi".to_owned(), "mcp".to_owned()];
            let ran = runner.run_with_input(game, "cargo", &args, &mcp_input(calls))?;
            mcp_verdict(calls, &ran)
        }
    }
}

/// The JSON-RPC lines a [`Step::Mcp`] writes: `initialize` (id 0), its
/// `notifications/initialized`, and a `tools/call` per call, the `n`th
/// with id `n` (from 1).
pub fn mcp_input(calls: &[McpCall]) -> String {
    let initialize = json!({ "jsonrpc": "2.0", "id": 0, "method": "initialize",
        "params": { "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": { "name": "cargo xtask skill-examples", "version": "1" } } });
    let mut lines = vec![
        initialize.to_string(),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }).to_string(),
    ];
    lines.extend(calls.iter().zip(1u64..).map(|(call, id)| {
        json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": { "name": call.tool, "arguments": call.input } })
        .to_string()
    }));
    let mut input = lines.join("\n");
    input.push('\n');
    input
}

/// Nothing if every call's answer in what the server `ran` wrote holds,
/// or what didn't: a line per call that failed, named by its place and
/// tool.
fn mcp_verdict(calls: &[McpCall], ran: &Ran) -> Result<(), String> {
    let responses: BTreeMap<u64, Value> = ran
        .stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|message| Some((message.get("id")?.as_u64()?, message)))
        .collect();
    let mut failed = Vec::new();
    for (call, id) in calls.iter().zip(1u64..) {
        let Some(response) = responses.get(&id) else {
            // The server ended: no later call has an answer either.
            failed.push(format!(
                "call {id} ({}): no answer; the server exited with {}\n{}",
                call.tool,
                exit_shown(ran.code),
                tail(ran)
            ));
            break;
        };
        if let Err(why) = call_holds(call, response) {
            failed.push(format!("call {id} ({}): {why}", call.tool));
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("\n"))
    }
}

/// Nothing if `call`'s `ok` and `expect` hold of its JSON-RPC `response`,
/// or what doesn't.
fn call_holds(call: &McpCall, response: &Value) -> Result<(), String> {
    if let Some(error) = response.get("error") {
        return Err(format!("a JSON-RPC error: {error}"));
    }
    let result = &response["result"];
    let answer = &result["structuredContent"];
    let is_error = result["isError"].as_bool();
    if is_error != Some(!call.ok) {
        let mut why = format!(
            "its isError is {}, not {}",
            is_error.map_or_else(|| "missing".to_owned(), |e| e.to_string()),
            !call.ok
        );
        for field in ["error", "code", "help"] {
            if let Some(Value::String(text)) = answer.get(field) {
                // Writing to a `String` can't fail.
                let _ = write!(why, "; {field}: {text}");
            }
        }
        return Err(why);
    }
    let failed: Vec<String> = call
        .expect
        .iter()
        .filter_map(|e| holds(answer, e).err())
        .collect();
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("; "))
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
