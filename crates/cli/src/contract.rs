//! The tool contract (ADR-0005 §1, ADR-0022): the operations an agent, CI
//! or a person runs on a game, each a library function with a typed input
//! and output that serde reads and writes and JSON schemas describe. The
//! CLI (`shuyi <op> [--json]`) is one surface over them; the MCP server
//! (ticket 1133) and the skills (ticket 1134) are the others.
//!
//! | Op | Does | Module |
//! | -- | ---- | ------ |
//! | `new <template> <dir>` | a new game from `templates/grid` or `templates/blank` | [`new`] |
//! | `check [--baked]` | every error of every validator of the game's content; with `--baked`, every way its pack differs from its sources | [`check`] |
//! | `check --target <id>` | whether the game's frame, text and controls fit a target | [`check`], [`target`] |
//! | `run` | plays a key script headless: the top screen, the scene, events, sounds | [`run`] |
//! | `scene` | the scene or stage on screen after a script | [`run`] |
//! | `replay <record>` | a state hash per step, and the first step where two runs differ | [`replay`] |
//! | `bots <n>` | the engine's bots play n games: outcomes and measures | [`bots`] |
//! | `import <file>` | a file read through the format registry, or its errors | [`import`] |
//! | `bake [--target <id>]` | the game's sources baked into one pack: sizes and times | [`bake`] |
//! | `doctor` | the toolchain: what is missing and how to install it | [`doctor`] |
//!
//! Every answer is an [`Answer`]: the contract's [`VERSION`], the
//! operation, whether it passed, its result or the error that stopped it,
//! and its **limits**, what it could not determine ("no scene: the top
//! screen is no stage").
//!
//! The schemas of every input and answer are generated from these types
//! ([`schemas`]) and checked in under `crates/cli/schemas/`. A test fails
//! when they change and [`VERSION`] doesn't.

pub mod bake;
pub mod bots;
pub mod check;
pub mod cli;
pub mod doctor;
#[cfg(test)]
mod fake;
pub mod import;
pub mod new;
pub mod replay;
pub mod run;
pub mod session;
pub mod target;

use std::fmt::Write as _;

use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;

/// The contract's version, in every answer. Raise it whenever an input
/// or an answer changes shape (the schemas' test says when), and add the
/// new schemas' fingerprint to `schemas/versions.json`.
pub const VERSION: u32 = 5;

/// An operation of the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Op {
    /// A new game from a template.
    New,
    /// Every error of the game's content.
    Check,
    /// A key script played headless.
    Run,
    /// The scene on screen.
    Scene,
    /// A record replayed.
    Replay,
    /// Bots playing the game.
    Bots,
    /// A file read through the format registry.
    Import,
    /// The game's sources baked into a pack.
    Bake,
    /// The toolchain checked.
    Doctor,
}

impl Op {
    /// Every operation, in the order the usage lists them.
    pub const ALL: [Op; 9] = [
        Op::New,
        Op::Check,
        Op::Run,
        Op::Scene,
        Op::Replay,
        Op::Bots,
        Op::Import,
        Op::Bake,
        Op::Doctor,
    ];

    /// Its name on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Op::New => "new",
            Op::Check => "check",
            Op::Run => "run",
            Op::Scene => "scene",
            Op::Replay => "replay",
            Op::Bots => "bots",
            Op::Import => "import",
            Op::Bake => "bake",
            Op::Doctor => "doctor",
        }
    }

    /// The operation named `name` on the command line.
    pub fn named(name: &str) -> Option<Op> {
        Op::ALL.into_iter().find(|op| op.name() == name)
    }
}

/// What an operation answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Answer<T> {
    /// The contract's version ([`VERSION`]).
    pub contract: u32,
    /// The operation.
    pub op: Op,
    /// Whether it passed: nothing found by a check, runs that agree, a
    /// game made, a toolchain complete. False too when it could not run.
    pub ok: bool,
    /// What it found or made; absent when it could not run.
    pub result: Option<T>,
    /// Why it could not run, if it could not.
    pub error: Option<String>,
    /// What it could not determine, in plain words.
    pub limits: Vec<String>,
}

impl<T> Answer<T> {
    /// Operation `op`'s `result`, passed if `ok`, with `limits`.
    pub fn new(op: Op, ok: bool, result: T, limits: Vec<String>) -> Self {
        Self {
            contract: VERSION,
            op,
            ok,
            result: Some(result),
            error: None,
            limits,
        }
    }

    /// Operation `op`, which could not run because of `error`.
    pub fn failed(op: Op, error: impl Into<String>) -> Self {
        Self {
            contract: VERSION,
            op,
            ok: false,
            result: None,
            error: Some(error.into()),
            limits: Vec::new(),
        }
    }

    /// This answer with `limit` added to its limits.
    #[must_use]
    pub fn limited(mut self, limit: impl Into<String>) -> Self {
        self.limits.push(limit.into());
        self
    }

    /// The process exit code for it: 0 if it passed, 1 if not.
    pub fn exit_code(&self) -> u8 {
        u8::from(!self.ok)
    }

    /// The answer as JSON, one document, pretty-printed.
    pub fn to_json(&self) -> String
    where
        T: Serialize,
    {
        // A struct of strings, numbers and lists: serializing can't fail.
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// The answer for people: the result as `text` writes it, or the
    /// error, then the limits.
    pub fn to_text(&self, text: impl FnOnce(&T) -> String) -> String {
        let mut out = match (&self.result, &self.error) {
            (Some(result), _) => text(result),
            (None, Some(error)) => format!("{}: {error}\n", self.op.name()),
            (None, None) => String::new(),
        };
        if !self.limits.is_empty() {
            out.push_str("limits:\n");
            for limit in &self.limits {
                // Writing to a `String` can't fail.
                let _ = writeln!(out, "  - {limit}");
            }
        }
        out
    }
}

/// A problem in a file: what a validator found, with where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Problem {
    /// The file, as the game shows it (`files/stage.map`).
    pub file: String,
    /// Its 1-based line, if known.
    pub line: Option<u32>,
    /// Its 1-based column, if known.
    pub column: Option<u32>,
    /// What is wrong.
    pub message: String,
}

impl From<&ContentError> for Problem {
    fn from(e: &ContentError) -> Self {
        Self {
            file: e.file.clone(),
            line: e.line,
            column: e.column,
            message: e.message.clone(),
        }
    }
}

impl std::fmt::Display for Problem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.file)?;
        if let Some(line) = self.line {
            write!(f, ":{line}")?;
            if let Some(column) = self.column {
                write!(f, ":{column}")?;
            }
        }
        write!(f, ": {}", self.message)
    }
}

/// One schema file: its name (`run.input.json`) and its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaFile {
    /// The file's name.
    pub name: String,
    /// Its JSON, pretty-printed, ending in a newline.
    pub text: String,
}

/// `schema` as a schema file named `name`.
fn file(name: String, schema: &Schema) -> SchemaFile {
    let mut text = serde_json::to_string_pretty(schema).unwrap_or_default();
    text.push('\n');
    SchemaFile { name, text }
}

/// The schemas of `op`'s input `I` and answer `Answer<O>`.
fn pair<I: JsonSchema, O: JsonSchema>(op: Op) -> [SchemaFile; 2] {
    [
        file(format!("{}.input.json", op.name()), &schema_for!(I)),
        file(
            format!("{}.output.json", op.name()),
            &schema_for!(Answer<O>),
        ),
    ]
}

/// Every operation's input and output schema, in [`Op::ALL`]'s order.
pub fn schemas() -> Vec<SchemaFile> {
    Op::ALL
        .into_iter()
        .flat_map(|op| match op {
            Op::New => pair::<new::NewInput, new::NewOutput>(op),
            Op::Check => pair::<check::CheckInput, check::CheckOutput>(op),
            Op::Run => pair::<run::RunInput, run::RunOutput>(op),
            Op::Scene => pair::<run::RunInput, run::SceneOutput>(op),
            Op::Replay => pair::<replay::ReplayInput, replay::ReplayOutput>(op),
            Op::Bots => pair::<bots::BotsInput, bots::BotsOutput>(op),
            Op::Import => pair::<import::ImportInput, import::ImportOutput>(op),
            Op::Bake => pair::<bake::BakeInput, bake::BakeOutput>(op),
            Op::Doctor => pair::<doctor::DoctorInput, doctor::DoctorOutput>(op),
        })
        .collect()
}

/// The fingerprint of `files`: a hash of their names and texts, which
/// `schemas/versions.json` keeps for each [`VERSION`].
pub fn fingerprint(files: &[SchemaFile]) -> String {
    let parts: Vec<(&str, &str)> = files
        .iter()
        .map(|f| (f.name.as_str(), f.text.as_str()))
        .collect();
    shuyi_core::state_hash(&parts)
        .map(|hash| hash.to_string())
        .unwrap_or_default()
}

/// One line of `schemas/versions.json`: a version and its schemas'
/// fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionLine {
    /// The contract's version.
    pub version: u32,
    /// Its schemas' [`fingerprint`].
    pub fingerprint: String,
}

#[cfg(test)]
mod tests;
