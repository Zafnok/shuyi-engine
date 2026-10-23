//! The tool contract (ADR-0005 §1, ADR-0022): the operations an agent, CI
//! or a person runs on a game, each a library function with a typed input
//! and output that serde reads and writes and JSON schemas describe. The
//! CLI (`shuyi <op> [--json]`) is one surface over them; the MCP server
//! (`shuyi mcp`, [`crate::mcp`]) is another, where each is a tool and a
//! session keeps one game running between calls.
//!
//! | Op | Does | Module |
//! | -- | ---- | ------ |
//! | `new <template> <dir>` | a new game from `templates/grid` or `templates/blank` | [`new`] |
//! | `check [--baked]` | every error of every validator of the game's content; with `--baked`, every way its pack differs from its sources | [`check`] |
//! | `check --target <id>` | whether the game's frame, text and controls fit a target | [`check`], [`target`] |
//! | `lint` | the game's code: `check-keys`, `check-text` and the rules crate's fence, each finding with file and line | [`lint`] |
//! | `run` | plays a key script headless: the top screen, the scene, events, sounds | [`run`] |
//! | `scene` | the scene or stage on screen after a script | [`run`] |
//! | `replay <record>` | a state hash per step, and the first step where two runs differ | [`replay`] |
//! | `bots <n>` | the engine's bots play n games: outcomes and measures | [`bots`] |
//! | `playtest` | the game's named bots each play K tries on every core, timed and judged against its bands | [`playtest`] |
//! | `autobalance` | the game's knobs change a stage until its named bots land in their bands: the changes, the reports before and after; with `--apply`, the stage written | [`autobalance`] |
//! | `import <file>` | a file read through the format registry, or its errors | [`import`] |
//! | `bake [--target <id>]` | the game's sources baked into one pack: sizes and times | [`bake`] |
//! | `doctor` | the toolchain: what is missing and how to install it | [`doctor`] |
//! | `explain [<code>]` | every problem code, or one code's page | [`explain`] |
//!
//! Every answer is an [`Answer`]: the contract's [`VERSION`], the
//! operation, whether it passed, its result or the error that stopped it,
//! and its **limits**, what it could not determine ("no scene: the top
//! screen is no stage"). An error carries a problem code and a help line
//! ([`codes`]), and the code's class sets the exit code: 0 passed, 1
//! failed, 2 wrong arguments, 3 fix the setup, then run it again.
//!
//! The MCP server's session tools (`start`, `step`, `scene`, `frame_png`,
//! `events`, `reset`, `stop`; [`Op::SESSION`]) answer the same way.
//!
//! The schemas of every input and answer are generated from these types
//! ([`schemas`]) and checked in under `crates/cli/schemas/`. A test fails
//! when they change and [`VERSION`] doesn't.

pub mod autobalance;
pub mod bake;
pub mod bots;
pub mod check;
pub mod cli;
pub mod codes;
pub mod doctor;
pub mod explain;
#[cfg(test)]
pub(crate) mod fake;
pub mod import;
pub mod lint;
pub mod new;
pub mod playtest;
pub mod replay;
pub mod run;
pub mod session;
pub mod target;

use std::fmt::Write as _;

use schemars::{JsonSchema, Schema, schema_for};
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;

use codes::Code;

/// The contract's version, in every answer. Raise it whenever an input
/// or an answer changes shape (the schemas' test says when), and add the
/// new schemas' fingerprint to `schemas/versions.json`.
pub const VERSION: u32 = 13;

/// An operation of the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Op {
    /// A new game from a template.
    New,
    /// Every error of the game's content.
    Check,
    /// What is wrong with the shape of the game's code.
    Lint,
    /// A key script played headless.
    Run,
    /// The scene on screen.
    Scene,
    /// A record replayed.
    Replay,
    /// Bots playing the game.
    Bots,
    /// Named bots' tries judged against the game's checks.
    Playtest,
    /// A stage changed until the game's named bots land in their bands.
    Autobalance,
    /// A file read through the format registry.
    Import,
    /// The game's sources baked into a pack.
    Bake,
    /// The toolchain checked.
    Doctor,
    /// A problem code's page.
    Explain,
    /// The MCP session's game started headless.
    Start,
    /// The MCP session's game played on: a script, then idle frames.
    Step,
    /// A picture of the MCP session's frame.
    #[serde(rename = "frame_png")]
    FramePng,
    /// What happened in the MCP session's game.
    Events,
    /// The MCP session's game started again.
    Reset,
    /// The MCP session's game stopped.
    Stop,
}

impl Op {
    /// Every operation, in the order the usage lists them.
    pub const ALL: [Op; 13] = [
        Op::New,
        Op::Check,
        Op::Lint,
        Op::Run,
        Op::Scene,
        Op::Replay,
        Op::Bots,
        Op::Playtest,
        Op::Autobalance,
        Op::Import,
        Op::Bake,
        Op::Doctor,
        Op::Explain,
    ];

    /// The MCP server's session tools ([`crate::mcp`]), in the order it
    /// lists them: `scene` is the session's, not the one-off operation.
    pub const SESSION: [Op; 7] = [
        Op::Start,
        Op::Step,
        Op::Scene,
        Op::FramePng,
        Op::Events,
        Op::Reset,
        Op::Stop,
    ];

    /// Its name on the command line, or as an MCP tool.
    pub fn name(self) -> &'static str {
        match self {
            Op::New => "new",
            Op::Check => "check",
            Op::Lint => "lint",
            Op::Run => "run",
            Op::Scene => "scene",
            Op::Replay => "replay",
            Op::Bots => "bots",
            Op::Playtest => "playtest",
            Op::Autobalance => "autobalance",
            Op::Import => "import",
            Op::Bake => "bake",
            Op::Doctor => "doctor",
            Op::Explain => "explain",
            Op::Start => "start",
            Op::Step => "step",
            Op::FramePng => "frame_png",
            Op::Events => "events",
            Op::Reset => "reset",
            Op::Stop => "stop",
        }
    }

    /// The operation named `name` on the command line (not a session
    /// tool).
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
    /// The problem code of its error (`SH1002`), or of its failure when
    /// it ran and failed on the setup (`doctor`); `shuyi explain <code>`
    /// prints its page.
    pub code: Option<String>,
    /// How to fix it, with the code.
    pub help: Option<String>,
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
            code: None,
            help: None,
            limits,
        }
    }

    /// Operation `op`, which could not run because of `error`, a problem
    /// of code `code`, with its row's help.
    pub fn failed(op: Op, code: Code, error: impl Into<String>) -> Self {
        Self {
            contract: VERSION,
            op,
            ok: false,
            result: None,
            error: Some(error.into()),
            code: None,
            help: None,
            limits: Vec::new(),
        }
        .with_code(code)
    }

    /// This answer with problem code `code` and its row's help: for an
    /// answer that ran and failed on the setup.
    #[must_use]
    pub fn with_code(mut self, code: Code) -> Self {
        self.code = Some(code.0.to_owned());
        self.help = codes::help_of(code.0).map(str::to_owned);
        self
    }

    /// This answer with `help` instead of its code's: a usage, or what
    /// to fix first.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// This answer with `limit` added to its limits.
    #[must_use]
    pub fn limited(mut self, limit: impl Into<String>) -> Self {
        self.limits.push(limit.into());
        self
    }

    /// The process exit code for it: 0 if it passed, else its code's
    /// class's ([`codes::Class::exit_code`]), 1 with no code.
    pub fn exit_code(&self) -> u8 {
        if self.ok {
            0
        } else {
            codes::exit_code(self.code.as_deref())
        }
    }

    /// The answer as JSON, one document, pretty-printed.
    pub fn to_json(&self) -> String
    where
        T: Serialize,
    {
        // A struct of strings, numbers and lists: serializing can't fail.
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// The answer for people: the result as `text` writes it, then the
    /// error with its code and help ([`codes::said`]), then the limits.
    pub fn to_text(&self, text: impl FnOnce(&T) -> String) -> String {
        let mut out = self.result.as_ref().map(text).unwrap_or_default();
        match (&self.code, &self.error) {
            (Some(code), error) => {
                let title = codes::entry(code).map_or("", |entry| entry.title);
                let message = error.as_deref().unwrap_or(title);
                out.push_str(&codes::said(
                    self.op.name(),
                    code,
                    message,
                    self.help.as_deref(),
                ));
            }
            (None, Some(error)) => {
                // Writing to a `String` can't fail.
                let _ = writeln!(out, "{}: {error}", self.op.name());
            }
            (None, None) => {}
        }
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
    /// Its problem code (`SH2001`), if its class has one.
    pub code: Option<String>,
    /// How to fix it, with the code.
    pub help: Option<String>,
}

impl From<&ContentError> for Problem {
    fn from(e: &ContentError) -> Self {
        Self {
            file: e.file.clone(),
            line: e.line,
            column: e.column,
            message: e.message.clone(),
            code: e.code.map(|code| code.0.to_owned()),
            help: e
                .code
                .and_then(|code| codes::help_of(code.0))
                .map(str::to_owned),
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
        if let Some(code) = &self.code {
            write!(f, ": error[{code}]")?;
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

/// The schemas of an input `I` and an answer `Answer<O>`, named `name`
/// (`run.input.json`, `run.output.json`).
fn pair<I: JsonSchema, O: JsonSchema>(name: &str) -> [SchemaFile; 2] {
    [
        file(format!("{name}.input.json"), &schema_for!(I)),
        file(format!("{name}.output.json"), &schema_for!(Answer<O>)),
    ]
}

/// The schemas of `op`'s input and answer: as the CLI and the MCP tool
/// of its name take it, or, if `session`, as the MCP server's session
/// tool does (named `session.<op>`; [`Op::SESSION`]).
pub fn op_schemas(op: Op, session: bool) -> [SchemaFile; 2] {
    use crate::mcp::{
        EventsOutput, FramePngInput, FramePngOutput, NoInput, StartInput, StartOutput, StepInput,
        StopOutput,
    };
    let name = if session {
        format!("session.{}", op.name())
    } else {
        op.name().to_owned()
    };
    let name = name.as_str();
    match (op, session) {
        (Op::New, _) => pair::<new::NewInput, new::NewOutput>(name),
        (Op::Check, _) => pair::<check::CheckInput, check::CheckOutput>(name),
        (Op::Lint, _) => pair::<lint::LintInput, lint::LintOutput>(name),
        (Op::Run, _) => pair::<run::RunInput, run::RunOutput>(name),
        (Op::Scene, false) => pair::<run::RunInput, run::SceneOutput>(name),
        (Op::Scene, true) => pair::<NoInput, run::SceneOutput>(name),
        (Op::Replay, _) => pair::<replay::ReplayInput, replay::ReplayOutput>(name),
        (Op::Bots, _) => pair::<bots::BotsInput, bots::BotsOutput>(name),
        (Op::Playtest, _) => pair::<playtest::PlaytestInput, Vec<playtest::PlaytestOutput>>(name),
        (Op::Autobalance, _) => {
            pair::<autobalance::AutobalanceInput, autobalance::AutobalanceOutput>(name)
        }
        (Op::Import, _) => pair::<import::ImportInput, import::ImportOutput>(name),
        (Op::Bake, _) => pair::<bake::BakeInput, bake::BakeOutput>(name),
        (Op::Doctor, _) => pair::<doctor::DoctorInput, doctor::DoctorOutput>(name),
        (Op::Explain, _) => pair::<explain::ExplainInput, explain::ExplainOutput>(name),
        (Op::Start, _) => pair::<StartInput, StartOutput>(name),
        (Op::Step, _) => pair::<StepInput, run::RunOutput>(name),
        (Op::FramePng, _) => pair::<FramePngInput, FramePngOutput>(name),
        (Op::Events, _) => pair::<NoInput, EventsOutput>(name),
        (Op::Reset, _) => pair::<NoInput, StartOutput>(name),
        (Op::Stop, _) => pair::<NoInput, StopOutput>(name),
    }
}

/// Every operation's input and output schema, in [`Op::ALL`]'s order,
/// then the MCP server's session tools' ([`Op::SESSION`]).
pub fn schemas() -> Vec<SchemaFile> {
    let ops = Op::ALL.into_iter().flat_map(|op| op_schemas(op, false));
    let session = Op::SESSION.into_iter().flat_map(|op| op_schemas(op, true));
    ops.chain(session).collect()
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
