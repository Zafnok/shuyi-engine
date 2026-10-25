//! `new <dir>`: a new game in `dir`, from the engine's template,
//! `templates/shuyi-default`, made of parts (ADR-0013 §3,
//! [`crate::template`]). It is embedded in the `shuyi` binary, so `new`
//! needs no checkout of the engine.
//!
//! The answers pick the parts: a short name of the template's
//! (`--template grid`, `--template blank`), then an answers file
//! (`--answers answers.ron`), then `--set <question>=<answer[,answer]>`,
//! each in place of the one before for the questions it answers. A
//! question none answers takes its default. The template's composer
//! ([`template::compose`]) makes the files with no I/O, and
//! [`template::apply`] writes them.
//!
//! A template's text files may hold these placeholders, and no others
//! (`docs/templates.md`):
//!
//! | Placeholder | Becomes |
//! | ----------- | ------- |
//! | `{{name}}` | the game's name, as given (`Moon Garden`) |
//! | `{{crate_name}}` | its crate's name (`moon-garden`) |
//! | `"{{engine.<crate>}}"` | the dependency on that engine crate (`shuyi`, `core`, `assets`, `ui`, `platform`, `app`, `cli`) in `Cargo.toml`: the engine's repository at `main` or at a commit, or a checkout on this machine |
//! | `{{engine_ref}}` | the engine's commit as a workflow's `uses: …@` names it: a 40-digit `--engine-rev`, else `main` |
//! | `{{answer.<question>}}` | that question's answers, joined by `,` |
//!
//! A GitHub Actions expression, `${{ … }}`, is left as it is. The game's
//! Rust code names its own crates `game` and `rules` (their `[lib]
//! name`), so its Rust holds a placeholder only in a string: the game's
//! name and its storage's name, each the whole of its own `const`
//! (`lib.rs`), so rustfmt lays the code out the same whatever the names'
//! length.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::codes::{self, Code};
use super::{Answer, Op};
use crate::catalog::Catalog;
use crate::template::{self, Answers, ComposeError, Fill, Plan, Template};

/// The engine's repository, which a new game depends on by git.
pub const ENGINE_GIT: &str = "https://github.com/Zafnok/shuyi-engine";

/// The game's workflow that calls the engine's reusable playtest
/// workflow, at `{{engine_ref}}` (the grid stage's part writes it).
const PLAYTEST_WORKFLOW: &str = ".github/workflows/playtest.yml";

/// The engine's crates a template may depend on, by their folder under
/// `crates/`: `shuyi`, the front door, and the default engine's.
pub const ENGINE_CRATES: [&str; 7] = ["shuyi", "core", "assets", "ui", "platform", "app", "cli"];

/// Where the new game gets the engine from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "from", rename_all = "kebab-case", deny_unknown_fields)]
// More places may come (ADR-0039).
#[non_exhaustive]
pub enum Engine {
    /// The engine's repository at its `main` branch.
    #[default]
    Main,
    /// The engine's repository at a commit.
    Rev {
        /// The commit, 7 to 40 hex digits.
        rev: String,
    },
    /// A checkout of the engine on this machine.
    Path {
        /// Its folder (the one holding `crates/`).
        path: String,
    },
}

/// What `new` makes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `NewInput::new` (a folder has no default) and its
// fields assigned, it may grow (ADR-0039).
#[non_exhaustive]
pub struct NewInput {
    /// The new game's folder, from the working folder: made if missing,
    /// and refused unless empty (or `force`).
    pub dir: String,
    /// A short name of the template's for a set of answers (`grid`,
    /// `blank`); none for none.
    #[serde(default)]
    pub template: Option<String>,
    /// An answers file (`answers.ron`), from the working folder: its
    /// answers in place of the short name's.
    #[serde(default)]
    pub answers: Option<String>,
    /// Answers by question, in place of the short name's and the file's
    /// (`--set perspective=top-down-grid`).
    #[serde(default)]
    pub set: Answers,
    /// The game's name; the folder's name if none.
    #[serde(default)]
    pub name: Option<String>,
    /// Where it gets the engine from.
    #[serde(default)]
    pub engine: Engine,
    /// Write into a folder that isn't empty, over the files there.
    #[serde(default)]
    pub force: bool,
}

impl NewInput {
    /// A game in the folder `dir`, named for its folder, from the
    /// template's defaults, on the engine's `main` ([`Engine::Main`]).
    pub fn new(dir: impl Into<String>) -> Self {
        Self {
            dir: dir.into(),
            template: None,
            answers: None,
            set: Answers::default(),
            name: None,
            engine: Engine::Main,
            force: false,
        }
    }

    /// The same, from the template's short name `name` (`grid`).
    #[must_use]
    pub fn template(mut self, name: impl Into<String>) -> Self {
        self.template = Some(name.into());
        self
    }
}

/// What `new` made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NewOutput {
    /// The game's folder, as given.
    pub dir: String,
    /// The game's name.
    pub name: String,
    /// Its crate's name.
    pub crate_name: String,
    /// The template's id (`shuyi-default`).
    pub template: String,
    /// The answers it was made for, each question left out at its
    /// default.
    pub answers: Answers,
    /// The template's parts it was made of, in order.
    pub parts: Vec<String>,
    /// The files made, from its folder, `/`-separated, in order.
    pub files: Vec<String>,
    /// The files that were there already and were written over (with
    /// `force`).
    pub overwritten: Vec<String>,
    /// The engine's modules turned on, by crate.
    pub modules: Vec<String>,
    /// The targets it ships on.
    pub targets: Vec<String>,
    /// Its stages, in order.
    pub stages: Vec<String>,
    /// The engine's skills it holds in `.claude/skills/`, by name.
    pub skills: Vec<String>,
}

/// The crate name for game name `name`: lowercase ASCII letters and
/// digits, every other run of characters one `-`; it starts with a letter.
pub fn crate_name(name: &str) -> Result<String, String> {
    let mut out = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.starts_with(|c: char| c.is_ascii_lowercase()) {
        Ok(out)
    } else {
        Err(format!(
            "`{name}` makes no crate name: it needs a letter first, then letters, digits or spaces"
        ))
    }
}

/// Whether `name` can be a game's name: something to show, with no
/// quote, backslash, brace or control character, which the files it goes
/// into would have to escape.
fn check_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("the game's name is empty".to_owned());
    }
    match name
        .chars()
        .find(|c| c.is_control() || matches!(c, '"' | '\\' | '{' | '}'))
    {
        Some(c) => Err(format!("the game's name may not hold {c:?}")),
        None => Ok(()),
    }
}

/// The dependency on engine crate `krate` (a folder of `crates/`), as a
/// TOML inline table.
fn dependency(engine: &Engine, root: Option<&str>, krate: &str) -> String {
    match (engine, root) {
        (Engine::Path { .. }, Some(root)) => format!("{{ path = '{root}/crates/{krate}' }}"),
        (Engine::Rev { rev }, _) => format!("{{ git = \"{ENGINE_GIT}\", rev = \"{rev}\" }}"),
        _ => format!("{{ git = \"{ENGINE_GIT}\", branch = \"main\" }}"),
    }
}

/// The commit a workflow's `uses: …@` can name for `engine`: a full
/// 40-digit rev (GitHub takes no shorter one there).
fn engine_ref(engine: &Engine) -> Option<&str> {
    match engine {
        Engine::Rev { rev } if rev.len() == 40 => Some(rev),
        _ => None,
    }
}

/// The engine checkout at `path` (from `cwd`), as an absolute path with
/// `/`, or why it is none.
fn engine_root(cwd: &Path, path: &str) -> Result<String, String> {
    let root = std::path::absolute(cwd.join(path)).map_err(|e| format!("{path}: {e}"))?;
    if !root.join("crates/core/Cargo.toml").is_file() {
        return Err(format!(
            "{path} is no checkout of the engine: it has no crates/core/Cargo.toml"
        ));
    }
    let root = root.to_string_lossy().replace('\\', "/");
    if root.contains('\'') {
        return Err(format!("the engine's path may not hold a quote: {root}"));
    }
    Ok(root)
}

/// What fills the placeholders of a game named `name`, of crate `krate`,
/// on `engine` (at `root` if a checkout).
pub fn fill_for(name: &str, krate: &str, engine: &Engine, root: Option<&str>) -> Fill {
    Fill {
        name: name.to_owned(),
        crate_name: krate.to_owned(),
        engine: ENGINE_CRATES
            .iter()
            .map(|krate| ((*krate).to_owned(), dependency(engine, root, krate)))
            .collect(),
        engine_ref: engine_ref(engine).unwrap_or("main").to_owned(),
    }
}

/// The template's short names, joined by commas.
fn short_names(template: &Template) -> String {
    let names: Vec<&str> = template.short_names.keys().map(String::as_str).collect();
    names.join(", ")
}

/// The answers `input` gives, from the working folder `cwd`: its short
/// name's, then its file's, then its `set`, each in place of the one
/// before for the questions it answers.
pub fn answers_of(cwd: &Path, template: &Template, input: &NewInput) -> Result<Answers, String> {
    let mut answers = Answers::default();
    if let Some(name) = &input.template {
        let named = template.short_name(name).ok_or_else(|| {
            format!(
                "no template {name}: the templates are {}",
                short_names(template)
            )
        })?;
        answers = answers.and(named);
    }
    if let Some(file) = &input.answers {
        let text = fs::read_to_string(cwd.join(file)).map_err(|e| format!("{file}: {e}"))?;
        answers = answers.and(&Answers::parse(file, &text)?);
    }
    Ok(answers.and(&input.set))
}

/// The problem code of a template that makes no game: the answers'
/// fault ([`codes::WRONG_ARGUMENTS`]) or the template's
/// ([`codes::FILE`]).
fn code_of(error: &ComposeError) -> Code {
    match error {
        ComposeError::Answer { .. }
        | ComposeError::Requires { .. }
        | ComposeError::Conflict { .. }
        | ComposeError::Module { .. } => codes::WRONG_ARGUMENTS,
        _ => codes::FILE,
    }
}

/// The game `template` makes for `input` (named `name`, of crate
/// `krate`) under `cwd`, with no file written: its plan, and its limits.
pub fn plan(
    cwd: &Path,
    template: &Template,
    input: &NewInput,
    name: &str,
    krate: &str,
) -> Result<(Plan, Vec<String>), (Code, String)> {
    let refused = |e| (codes::WRONG_ARGUMENTS, e);
    let mut limits = Vec::new();
    let root = match &input.engine {
        Engine::Main => {
            limits.push(
                "the game follows the engine's main branch, not a commit: pass --engine-rev \
                 <commit> to pin it"
                    .to_owned(),
            );
            None
        }
        Engine::Rev { rev } => {
            let hex = rev.chars().all(|c| c.is_ascii_hexdigit());
            if !(hex && (7..=40).contains(&rev.len())) {
                return Err(refused(format!("--engine-rev: not a commit: {rev}")));
            }
            None
        }
        Engine::Path { path } => {
            let root = engine_root(cwd, path).map_err(refused)?;
            limits.push(format!(
                "the game builds with the engine checkout at {root}: it moves with that folder"
            ));
            Some(root)
        }
    };
    let catalog = match &root {
        Some(root) => Catalog::read(Path::new(root)),
        None => Catalog::embedded(),
    }
    .map_err(|e| (codes::ENGINE_DEPENDENCY, e))?;
    let answers = answers_of(cwd, template, input).map_err(refused)?;
    let fill = fill_for(name, krate, &input.engine, root.as_deref());
    let plan = template::compose(template, &answers, &catalog, &fill)
        .map_err(|e| (code_of(&e), e.to_string()))?;
    limits.extend(plan.limits.iter().cloned());
    if matches!(input.engine, Engine::Rev { .. })
        && engine_ref(&input.engine).is_none()
        && plan.files.contains_key(Path::new(PLAYTEST_WORKFLOW))
    {
        limits.push(format!(
            "{PLAYTEST_WORKFLOW} calls the engine's playtest workflow at main: a workflow names \
             a commit by its 40 digits, so pass the whole commit to --engine-rev to pin it too"
        ));
    }
    Ok((plan, limits))
}

/// `new`: the game `input` asks for, made under `cwd`.
pub fn new(cwd: &Path, input: &NewInput) -> Answer<NewOutput> {
    match make(cwd, input) {
        Ok((output, limits)) => Answer::new(Op::New, true, output, limits),
        Err((code, e)) => Answer::failed(Op::New, code, e),
    }
}

/// The game `input` asks for, made under `cwd`, with its limits; or the
/// problem code and why not: an input refused
/// ([`codes::WRONG_ARGUMENTS`]), or a template or files that can't be
/// read, composed or written ([`codes::FILE`]).
fn make(cwd: &Path, input: &NewInput) -> Result<(NewOutput, Vec<String>), (Code, String)> {
    let refused = |e| (codes::WRONG_ARGUMENTS, e);
    let dir = cwd.join(&input.dir);
    let folder = std::path::absolute(&dir)
        .ok()
        .and_then(|d| d.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default();
    let name = input.name.clone().unwrap_or(folder);
    check_name(&name).map_err(refused)?;
    let krate = crate_name(&name).map_err(refused)?;
    let template = template::built_in().map_err(|e| (codes::FILE, e.to_string()))?;
    let (plan, mut limits) = plan(cwd, &template, input, &name, &krate)?;
    let applied = template::apply(&plan, &dir, input.force).map_err(|e| {
        if e.starts_with("is not empty") {
            refused(format!("{} {e}", input.dir))
        } else {
            (codes::FILE, e)
        }
    })?;
    limits.push(format!(
        "the game was made, not built: run `cargo test` in {}",
        input.dir
    ));
    let output = NewOutput {
        dir: input.dir.clone(),
        name,
        crate_name: krate,
        template: template.id.clone(),
        answers: plan.answers,
        parts: plan.parts,
        files: applied.written,
        overwritten: applied.overwritten,
        modules: plan.modules,
        targets: plan.targets,
        stages: plan.stages,
        skills: plan.skills,
    };
    Ok((output, limits))
}

/// `new`'s usage.
pub fn usage(command: &str) -> String {
    let names = template::built_in().map_or_else(|_| String::new(), |t| short_names(&t));
    format!(
        "usage: {command} new <dir> [--template NAME] [--answers FILE] [--set QUESTION=ANSWER[,ANSWER]]...\n\
         \x20          [--name NAME] [--engine-rev COMMIT | --engine-path DIR] [--force] [--json]\n\n\
         Makes a new game in <dir> (missing or empty, or any with --force) from the engine's\n\
         template, its parts picked by the answers: --template's short name ({names}),\n\
         then --answers' file, then each --set. The game depends on the engine's repository\n\
         at main, at --engine-rev's commit, or on the checkout at --engine-path."
    )
}

/// Reads `new`'s arguments (not `--json`). `new <template> <dir>`, the
/// older form, is `new <dir> --template <template>`.
pub fn parse_args(args: &[String]) -> Result<NewInput, String> {
    let mut positional = Vec::new();
    let mut input = NewInput::new("");
    let mut set = BTreeMap::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        match arg.as_str() {
            "--template" => input.template = Some(value()?),
            "--answers" => input.answers = Some(value()?),
            "--set" => {
                let (question, answers) = Answers::parse_set(&value()?)?;
                set.insert(question, answers);
            }
            "--name" => input.name = Some(value()?),
            "--engine-rev" => input.engine = Engine::Rev { rev: value()? },
            "--engine-path" => input.engine = Engine::Path { path: value()? },
            "--force" => input.force = true,
            flag if flag.starts_with("--") => return Err(format!("unknown option: {flag}")),
            _ => positional.push(arg.clone()),
        }
    }
    input.set = Answers(set);
    match positional.as_slice() {
        [dir] => input.dir.clone_from(dir),
        [template, dir] if input.template.is_none() => {
            input.template = Some(template.clone());
            input.dir.clone_from(dir);
        }
        _ => return Err("a <dir>, nothing else".to_owned()),
    }
    Ok(input)
}

/// What `new` made, for people.
pub fn text(output: &NewOutput) -> String {
    let mut out = format!(
        "new: {} ({}) in {}, {} file(s), from {}: {}\n",
        output.name,
        output.crate_name,
        output.dir,
        output.files.len(),
        output.template,
        output.parts.join(", ")
    );
    // Writing to a `String` can't fail.
    if !output.modules.is_empty() {
        let _ = writeln!(out, "modules on: {}", output.modules.join(", "));
    }
    if !output.skills.is_empty() {
        let _ = writeln!(out, "skills: {}", output.skills.join(", "));
    }
    if !output.overwritten.is_empty() {
        let _ = writeln!(out, "written over: {}", output.overwritten.join(", "));
    }
    for file in &output.files {
        let _ = writeln!(out, "  {file}");
    }
    out
}

#[cfg(test)]
mod tests;
