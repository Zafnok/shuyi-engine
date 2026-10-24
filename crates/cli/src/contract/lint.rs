//! `lint`: everything wrong with the shape of the game's code, in one
//! answer, each finding with its file, its line and what to do (ADR-0005
//! §1):
//!
//! - `check-keys`' findings: a key named in the game's code or text
//!   ([`crate::check_keys`], [`codes::HARD_CODED_KEY`]);
//! - `check-text`'s: screen text written as literals, when there are more
//!   than the project allows, and a data name read by a screen
//!   ([`crate::check_text`], [`codes::SCREEN_TEXT`]);
//! - the rules crate's fence (ADR-0032 §4): `cargo clippy` on the package
//!   `shuyi.ron`'s `rules` names, read for the lints its `clippy.toml` and
//!   its `lib.rs` set, a hash map, a float, a clock, file I/O or outside
//!   luck ([`FENCE_LINTS`], [`codes::RULES_FENCE`]);
//! - its layers: `cargo tree` of that package, as the template's
//!   `tests/layers.rs` reads it, each crate the rules never depend on
//!   ([`NEVER`], [`codes::RULES_DEPENDENCY`]) with the `cargo tree -i`
//!   that shows the way in.
//!
//! The two checks run in this process; clippy and `cargo tree` run as
//! child processes from the project's root, through a [`Cargo`], so the
//! tests can give any output. What isn't checked (a part `shuyi.ron`
//! lacks, a cargo that fails) is in the answer's limits. It passes when
//! nothing is found.

use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Answer, Op, codes};
use crate::check_keys::relative;
use crate::project::{FILE, Project};
use crate::{check_keys, check_text};

/// The clippy lints of the rules crate's fence.
pub const FENCE_LINTS: [&str; 3] = [
    "clippy::disallowed_types",
    "clippy::disallowed_methods",
    "clippy::float_arithmetic",
];

/// The crates the rules never depend on, even through another crate, as
/// the grid template's `crates/rules/tests/layers.rs` lists them (a test
/// keeps the two the same). A trailing `*` is a prefix.
pub const NEVER: &[&str] = &[
    "shuyi",
    "shuyi-ui",
    "shuyi-platform",
    "shuyi-app",
    "shuyi-cli",
    "macroquad",
    "miniquad",
    "quad-*",
    "gilrs",
    "gilrs-core",
    "cpal",
    "rodio",
    "kira",
    "alsa",
    "alsa-sys",
    "coreaudio-rs",
    "coreaudio-sys",
    "oboe",
    "symphonia",
    "lewton",
    "hound",
    "rand",
    "getrandom",
];

/// What `lint` checks: everything (nothing to choose yet).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LintInput {}

/// Which check found a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum LintCheck {
    /// `check-keys`: a key named in code or text.
    CheckKeys,
    /// `check-text`: screen text as a literal, or a data name read.
    CheckText,
    /// The rules crate's clippy fence.
    RulesFence,
    /// The rules crate's dependencies.
    RulesLayers,
}

/// One thing wrong, with where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Finding {
    /// The check that found it.
    pub check: LintCheck,
    /// The file, from the project's root (`crates/rules/src/lib.rs`).
    pub file: String,
    /// Its 1-based line, if known.
    pub line: Option<u32>,
    /// Its 1-based column, if known.
    pub column: Option<u32>,
    /// What is wrong, and for the fence, what to use instead.
    pub message: String,
    /// Its problem code (`SH2101`).
    pub code: String,
    /// How to fix it, with the code.
    pub help: Option<String>,
    /// A command that shows more: for a dependency, the `cargo tree -i`
    /// that shows the way in.
    pub command: Option<String>,
}

impl Finding {
    /// A finding of `check`, in `file`, with code `code` and its row's
    /// help.
    fn new(check: LintCheck, file: impl Into<String>, code: codes::Code, message: String) -> Self {
        Self {
            check,
            file: file.into(),
            line: None,
            column: None,
            message,
            code: code.0.to_owned(),
            help: codes::help_of(code.0).map(str::to_owned),
            command: None,
        }
    }
}

/// What `lint` found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LintOutput {
    /// Every finding: `check-keys`', `check-text`'s, the fence's, then the
    /// layers'.
    pub findings: Vec<Finding>,
}

/// What cargo printed and whether it succeeded.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ran {
    /// Whether it exited with 0.
    pub success: bool,
    /// Its stdout.
    pub stdout: String,
    /// Its stderr.
    pub stderr: String,
}

/// How `lint` runs cargo.
pub trait Cargo {
    /// Cargo with `args`, run in `root`; an error if it doesn't start.
    fn run(&self, root: &Path, args: &[&str]) -> Result<Ran, String>;
}

/// The machine's cargo: the one running this program if cargo started it
/// (`CARGO`), else `cargo` on the `PATH`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ThisMachine;

impl Cargo for ThisMachine {
    fn run(&self, root: &Path, args: &[&str]) -> Result<Ran, String> {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let out = Command::new(&cargo)
            .args(args)
            .current_dir(root)
            .output()
            .map_err(|e| format!("cannot run {}: {e}", cargo.to_string_lossy()))?;
        Ok(Ran {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

/// `lint` on `project`, running cargo with `cargo`.
pub fn lint(cargo: &dyn Cargo, project: &Project, _input: &LintInput) -> Answer<LintOutput> {
    let config = &project.config;
    let root = &project.root;
    let mut findings = Vec::new();
    let mut limits = Vec::new();
    match &config.check_keys {
        Some(rules) => findings.extend(
            check_keys::run(rules, root)
                .iter()
                .map(|e| reported(LintCheck::CheckKeys, codes::HARD_CODED_KEY, e)),
        ),
        None => limits.push(format!("check-keys wasn't run: {FILE} has no `check_keys`")),
    }
    match &config.check_text {
        Some(rules) => findings.extend(text_findings(rules, root)),
        None => limits.push(format!("check-text wasn't run: {FILE} has no `check_text`")),
    }
    match &config.rules {
        Some(package) => match fence(cargo, root, package, &mut limits) {
            Ok(found) => findings.extend(found),
            Err(e) => return Answer::failed(Op::Lint, codes::TOOLCHAIN, e),
        },
        None => limits.push(format!(
            "the rules crate's fence wasn't checked: {FILE} names no rules package (`rules`)"
        )),
    }
    let ok = findings.is_empty();
    Answer::new(Op::Lint, ok, LintOutput { findings }, limits)
}

/// `check-text`'s findings: every literal if there are more than the
/// rules allow, and every data name read.
fn text_findings(rules: &crate::project::CheckText, root: &Path) -> Vec<Finding> {
    let (hits, errors) = check_text::run(rules, root);
    let mut found: Vec<Finding> = errors
        .iter()
        .map(|e| reported(LintCheck::CheckText, codes::SCREEN_TEXT, e))
        .collect();
    if hits.len() > rules.max_literals {
        found.extend(hits.iter().map(|hit| {
            let message = format!(
                "screen text written as a literal, {:?} ({} in {}, at most {} allowed) — {}",
                hit.text,
                hits.len(),
                rules.dir,
                rules.max_literals,
                rules.hint
            );
            Finding {
                line: u32::try_from(hit.line).ok(),
                ..Finding::new(LintCheck::CheckText, &hit.file, codes::SCREEN_TEXT, message)
            }
        }));
    }
    found.extend(
        check_text::run_names(rules, root)
            .iter()
            .map(|e| reported(LintCheck::CheckText, codes::SCREEN_TEXT, e)),
    );
    found
}

/// A finding of `check` from a line a check reports (`file:line: what`,
/// or `file: what`), with `code`; one that couldn't read a file has
/// [`codes::FILE`].
pub fn reported(check: LintCheck, code: codes::Code, error: &str) -> Finding {
    let (place, message) = error.split_once(": ").unwrap_or(("", error));
    let (file, line) = match place.rsplit_once(':') {
        Some((file, line)) if line.parse::<u32>().is_ok() => (file, line.parse().ok()),
        _ => (place, None),
    };
    let code = if message.starts_with("cannot read") {
        codes::FILE
    } else {
        code
    };
    Finding {
        line,
        ..Finding::new(check, file, code, message.to_owned())
    }
}

/// The rules crate's fence: clippy's findings, then `cargo tree`'s, for
/// `package`, run in `root`. What couldn't be checked goes in `limits`;
/// an error if cargo doesn't start.
fn fence(
    cargo: &dyn Cargo,
    root: &Path,
    package: &str,
    limits: &mut Vec<String>,
) -> Result<Vec<Finding>, String> {
    let clippy = cargo.run(
        root,
        &[
            "clippy",
            "--quiet",
            "-p",
            package,
            "--message-format=json",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    let mut found = clippy_findings(root, &clippy.stdout);
    if let Some(error) = compile_error(root, &clippy.stdout) {
        limits.push(format!(
            "the rules crate's fence was checked only in part: {package} doesn't compile ({error})"
        ));
    } else if !clippy.success && found.is_empty() && !has_messages(&clippy.stdout) {
        limits.push(format!(
            "the rules crate's fence wasn't checked: cargo clippy -p {package} failed: {}",
            last_line(&clippy.stderr)
        ));
    }
    let tree = cargo.run(
        root,
        &[
            "tree",
            "-p",
            package,
            "--edges",
            "normal,build",
            "--target",
            "all",
            "--prefix",
            "none",
            "--format",
            "{p}",
        ],
    )?;
    if tree.success {
        found.extend(tree_findings(root, package, &tree.stdout));
    } else {
        limits.push(format!(
            "the rules crate's layers weren't checked: cargo tree -p {package} failed: {}",
            last_line(&tree.stderr)
        ));
    }
    Ok(found)
}

/// The last line of `text` that says something.
fn last_line(text: &str) -> &str {
    text.lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no output")
}

/// The compiler messages of cargo's JSON output, `stdout`.
fn messages(stdout: &str) -> impl Iterator<Item = Value> + '_ {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|line| line["reason"] == "compiler-message")
        .map(|mut line| line["message"].take())
}

/// Whether cargo's JSON output has any compiler message.
fn has_messages(stdout: &str) -> bool {
    messages(stdout).next().is_some()
}

/// `path` (from `root`, or a path in it) as the project shows it: from
/// `root`, with `/`.
fn shown(root: &Path, path: &str) -> String {
    relative(root, Path::new(path))
}

/// The file, line and column of `message`'s primary span, if it has one.
fn place(root: &Path, message: &Value) -> (String, Option<u32>, Option<u32>) {
    let spans = message["spans"].as_array().map_or(&[][..], Vec::as_slice);
    let Some(span) = spans
        .iter()
        .find(|span| span["is_primary"] == true)
        .or_else(|| spans.first())
    else {
        return (String::new(), None, None);
    };
    let number = |key: &str| span[key].as_u64().and_then(|n| u32::try_from(n).ok());
    (
        shown(root, span["file_name"].as_str().unwrap_or_default()),
        number("line_start"),
        number("column_start"),
    )
}

/// The reason a disallowed type's or method's message gives: the note
/// with no span that isn't clippy's own (a link, the `-D` it came from).
fn reason(message: &Value) -> Option<&str> {
    const CLIPPYS: [&str; 4] = [
        "for further information",
        "`-D ",
        "to override",
        "the lint level",
    ];
    message["children"]
        .as_array()?
        .iter()
        .filter(|child| child["level"] == "note")
        .filter(|child| child["spans"].as_array().is_none_or(Vec::is_empty))
        .filter_map(|child| child["message"].as_str())
        .find(|text| !CLIPPYS.iter().any(|clippy| text.starts_with(clippy)))
}

/// The fence's findings in clippy's JSON output, `stdout` (cargo's
/// `--message-format=json`), files shown from `root`: each message of a
/// lint of [`FENCE_LINTS`], once, its reason after it. Any other lint is
/// left out.
pub fn clippy_findings(root: &Path, stdout: &str) -> Vec<Finding> {
    let mut found: Vec<Finding> = Vec::new();
    for message in messages(stdout) {
        let name = message["code"]["code"].as_str().unwrap_or_default();
        if !FENCE_LINTS.contains(&name) {
            continue;
        }
        let (file, line, column) = place(root, &message);
        let text = message["message"].as_str().unwrap_or_default();
        let text = match reason(&message) {
            Some(reason) => format!("{text}: {reason}"),
            None => text.to_owned(),
        };
        let finding = Finding {
            line,
            column,
            ..Finding::new(LintCheck::RulesFence, file, codes::RULES_FENCE, text)
        };
        if !found.contains(&finding) {
            found.push(finding);
        }
    }
    found
}

/// The first error in clippy's JSON output that is the compiler's, not a
/// lint's (`error[E0425]`, or one with no code), with where, if any.
fn compile_error(root: &Path, stdout: &str) -> Option<String> {
    messages(stdout).find_map(|message| {
        let code = message["code"]["code"].as_str();
        let compiler = code.is_none_or(|code| {
            code.strip_prefix('E')
                .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))
        });
        (message["level"] == "error" && compiler).then(|| {
            let text = message["message"].as_str().unwrap_or_default();
            match place(root, &message) {
                (file, Some(line), _) => format!("{file}:{line}: {text}"),
                _ => text.to_owned(),
            }
        })
    })
}

/// Whether crate `name` is one of [`NEVER`].
fn is_never(name: &str) -> bool {
    NEVER.iter().any(|never| match never.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == *never,
    })
}

/// The layers' findings in `cargo tree -p package --prefix none --format
/// {p}`'s output, `stdout`: each crate of [`NEVER`] in it, once, in the
/// package's `Cargo.toml` (from `root`, read off the tree's first line),
/// with the `cargo tree -i` that shows the way in.
pub fn tree_findings(root: &Path, package: &str, stdout: &str) -> Vec<Finding> {
    let manifest = stdout
        .lines()
        .next()
        .and_then(|first| first.rsplit_once(" ("))
        .and_then(|(_, path)| path.strip_suffix(')'))
        .map_or_else(
            || "Cargo.toml".to_owned(),
            |dir| {
                let dir = shown(root, dir);
                if dir.is_empty() {
                    "Cargo.toml".to_owned()
                } else {
                    format!("{dir}/Cargo.toml")
                }
            },
        );
    let mut names: Vec<&str> = stdout
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| is_never(name))
        .collect();
    names.sort_unstable();
    names.dedup();
    names
        .into_iter()
        .map(|name| {
            let message = format!(
                "{package} depends on {name}: the rules stay pure, so screens, devices and tools \
                 go in crates/game, and the only luck is the SimRng kept in the state"
            );
            Finding {
                command: Some(format!("cargo tree -p {package} -i {name}")),
                ..Finding::new(
                    LintCheck::RulesLayers,
                    manifest.clone(),
                    codes::RULES_DEPENDENCY,
                    message,
                )
            }
        })
        .collect()
}

/// `lint`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!("usage: {command} lint [--json]")
}

/// What `lint` found, for people.
pub fn text(output: &LintOutput) -> String {
    if output.findings.is_empty() {
        return "lint: OK\n".to_owned();
    }
    let mut out = format!("lint: {} finding(s)\n", output.findings.len());
    for finding in &output.findings {
        // Writing to a `String` can't fail.
        let _ = write!(out, "  {}", finding.file);
        if let Some(line) = finding.line {
            let _ = write!(out, ":{line}");
            if let Some(column) = finding.column {
                let _ = write!(out, ":{column}");
            }
        }
        let _ = writeln!(out, ": error[{}]: {}", finding.code, finding.message);
        if let Some(command) = &finding.command {
            let _ = writeln!(out, "    see the way in: {command}");
        }
    }
    let mut seen: Vec<&str> = Vec::new();
    for finding in &output.findings {
        if let Some(help) = &finding.help
            && !seen.contains(&finding.code.as_str())
        {
            seen.push(&finding.code);
            let _ = writeln!(out, "help[{}]: {help}", finding.code);
        }
    }
    out
}

#[cfg(test)]
mod tests;
