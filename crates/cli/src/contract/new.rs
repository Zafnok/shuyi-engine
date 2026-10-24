//! `new <template> <dir>`: a new game in `dir`, from one of the engine's
//! two templates, `templates/grid` (a turn-based grid stage) and
//! `templates/blank` (one screen). Each is a plain folder, embedded in the
//! `shuyi` binary, so `new` needs no checkout of the engine. Ticket 2501
//! turns them into templates made of parts (ADR-0013).
//!
//! A template's text files may hold these placeholders, and no others:
//!
//! | Placeholder | Becomes |
//! | ----------- | ------- |
//! | `{{name}}` | the game's name, as given (`Moon Garden`) |
//! | `{{crate}}` | its crate's name (`moon-garden`) |
//! | `"{{engine.<crate>}}"` | the dependency on that engine crate (`shuyi`, `core`, `assets`, `ui`, `platform`, `app`, `cli`) in `Cargo.toml`: the engine's repository at `main` or at a commit, or a checkout on this machine |
//!
//! The game's Rust code names its own crates `game` and `rules` (their
//! `[lib] name`), so its Rust holds a placeholder only in a string: the
//! game's name and its storage's name, each the whole of its own `const`
//! (`blank`'s `lib.rs`, `grid`'s `main.rs`), so rustfmt lays the code out
//! the same whatever the names' length. No other `{{` may be in a
//! template's text.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use include_dir::{Dir, DirEntry, include_dir};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::codes::{self, Code};
use super::{Answer, Op};

/// The engine's templates, embedded.
static TEMPLATES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../templates");

/// The engine's repository, which a new game depends on by git.
pub const ENGINE_GIT: &str = "https://github.com/Zafnok/shuyi-engine";

/// The engine's crates a template may depend on, by their folder under
/// `crates/`: `shuyi`, the front door, and the default engine's.
pub const ENGINE_CRATES: [&str; 7] = ["shuyi", "core", "assets", "ui", "platform", "app", "cli"];

/// A template of the engine's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Template {
    /// A turn-based grid stage: a rules crate over `TurnRules` and a game
    /// crate with one stage screen, a bot, records, and every tool of the
    /// contract wired.
    Grid,
    /// One screen and nothing else.
    Blank,
}

impl Template {
    /// Every template.
    pub const ALL: [Template; 2] = [Template::Grid, Template::Blank];

    /// Its name, and its folder's under `templates/`.
    pub fn name(self) -> &'static str {
        match self {
            Template::Grid => "grid",
            Template::Blank => "blank",
        }
    }

    /// The template named `name`.
    pub fn named(name: &str) -> Option<Template> {
        Template::ALL.into_iter().find(|t| t.name() == name)
    }

    /// Its embedded folder.
    fn dir(self) -> Option<&'static Dir<'static>> {
        TEMPLATES.get_dir(self.name())
    }
}

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
// An input: built with `NewInput::new` (a template and a folder have no
// default) and its fields assigned, it may grow (ADR-0039).
#[non_exhaustive]
pub struct NewInput {
    /// The template.
    pub template: Template,
    /// The new game's folder, from the working folder: made if missing,
    /// and refused unless empty.
    pub dir: String,
    /// The game's name; the folder's name if none.
    #[serde(default)]
    pub name: Option<String>,
    /// Where it gets the engine from.
    #[serde(default)]
    pub engine: Engine,
}

impl NewInput {
    /// A game from `template` in the folder `dir`, named for its folder,
    /// on the engine's `main` ([`Engine::Main`]).
    pub fn new(template: Template, dir: impl Into<String>) -> Self {
        Self {
            template,
            dir: dir.into(),
            name: None,
            engine: Engine::Main,
        }
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
    /// The files made, from its folder, `/`-separated, in order.
    pub files: Vec<String>,
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

/// `text` with the placeholders replaced, or the first one unknown (in
/// file `file`, at its line).
fn fill(
    file: &str,
    text: &str,
    name: &str,
    krate: &str,
    deps: &[(String, String)],
) -> Result<String, String> {
    let mut out = text.replace("{{name}}", name).replace("{{crate}}", krate);
    for (placeholder, dependency) in deps {
        out = out.replace(placeholder, dependency);
    }
    if let Some(at) = out.find("{{") {
        let line = out[..at].lines().count().max(1);
        let rest = &out[at..];
        let shown = rest.find("}}").map_or(rest, |end| &rest[..end + 2]);
        return Err(format!(
            "the template's {file}:{line} has an unknown placeholder {shown}"
        ));
    }
    Ok(out)
}

/// Every file of `dir`, at any depth.
fn files_of(dir: &'static Dir<'static>) -> Vec<&'static include_dir::File<'static>> {
    let mut out = Vec::new();
    let mut pending = vec![dir];
    while let Some(dir) = pending.pop() {
        for entry in dir.entries() {
            match entry {
                DirEntry::Dir(d) => pending.push(d),
                DirEntry::File(f) => out.push(f),
            }
        }
    }
    out
}

/// The files of `template` filled for game `name` with crate `krate`
/// and `engine` (at `root` if a checkout): each path from the game's
/// folder (`/`-separated) and its bytes, in order.
pub fn files(
    template: Template,
    name: &str,
    krate: &str,
    engine: &Engine,
    root: Option<&str>,
) -> Result<Vec<(String, Vec<u8>)>, String> {
    let dir = template
        .dir()
        .ok_or_else(|| format!("the template {} is missing", template.name()))?;
    let deps: Vec<(String, String)> = ENGINE_CRATES
        .iter()
        .map(|krate| {
            (
                format!("\"{{{{engine.{krate}}}}}\""),
                dependency(engine, root, krate),
            )
        })
        .collect();
    let mut out = Vec::new();
    for file in files_of(dir) {
        let path = file
            .path()
            .strip_prefix(template.name())
            .unwrap_or(file.path())
            .to_string_lossy()
            .replace('\\', "/");
        let bytes = match std::str::from_utf8(file.contents()) {
            Ok(text) => fill(&path, text, name, krate, &deps)?.into_bytes(),
            Err(_) => file.contents().to_vec(),
        };
        out.push((path, bytes));
    }
    out.sort();
    Ok(out)
}

/// Whether `dir` is missing or empty.
fn is_empty_dir(dir: &Path) -> bool {
    fs::read_dir(dir).map_or(true, |mut entries| entries.next().is_none())
}

/// Writes `files` into `dir`.
fn write(dir: &Path, files: &[(String, Vec<u8>)]) -> Result<(), String> {
    for (path, bytes) in files {
        let to: PathBuf = dir.join(path);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
    }
    Ok(())
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
/// ([`codes::WRONG_ARGUMENTS`]), or files that can't be filled or
/// written ([`codes::FILE`]).
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
    if !is_empty_dir(&dir) {
        return Err(refused(format!("{} is not empty", input.dir)));
    }
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
    let files = files(
        input.template,
        &name,
        &krate,
        &input.engine,
        root.as_deref(),
    )
    .map_err(|e| (codes::FILE, e))?;
    write(&dir, &files).map_err(|e| (codes::FILE, e))?;
    limits.push(format!(
        "the game was made, not built: run `cargo test` in {}",
        input.dir
    ));
    let output = NewOutput {
        dir: input.dir.clone(),
        name,
        crate_name: krate,
        files: files.into_iter().map(|(path, _)| path).collect(),
    };
    Ok((output, limits))
}

/// `new`'s usage.
pub fn usage(command: &str) -> String {
    let names: Vec<&str> = Template::ALL.iter().map(|t| t.name()).collect();
    format!(
        "usage: {command} new <template> <dir> [--name NAME] [--engine-rev COMMIT | --engine-path DIR] [--json]\n\n\
         Makes a new game in <dir> (missing or empty) from a template: {}.\n\
         The game depends on the engine's repository at main, at --engine-rev's\n\
         commit, or on the checkout at --engine-path.",
        names.join(", ")
    )
}

/// Reads `new`'s arguments (not `--json`).
pub fn parse_args(args: &[String]) -> Result<NewInput, String> {
    let mut positional = Vec::new();
    let mut name = None;
    let mut engine = Engine::Main;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        match arg.as_str() {
            "--name" => name = Some(value()?),
            "--engine-rev" => engine = Engine::Rev { rev: value()? },
            "--engine-path" => engine = Engine::Path { path: value()? },
            flag if flag.starts_with("--") => return Err(format!("unknown option: {flag}")),
            _ => positional.push(arg.clone()),
        }
    }
    let [template, dir] = positional.as_slice() else {
        return Err("a <template> and a <dir>, nothing else".to_owned());
    };
    let template = Template::named(template).ok_or_else(|| {
        let names: Vec<&str> = Template::ALL.iter().map(|t| t.name()).collect();
        format!(
            "no template {template}: the templates are {}",
            names.join(", ")
        )
    })?;
    Ok(NewInput {
        template,
        dir: dir.clone(),
        name,
        engine,
    })
}

/// What `new` made, for people.
pub fn text(output: &NewOutput) -> String {
    let mut out = format!(
        "new: {} ({}) in {}, {} file(s)\n",
        output.name,
        output.crate_name,
        output.dir,
        output.files.len()
    );
    for file in &output.files {
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "  {file}");
    }
    out
}

#[cfg(test)]
mod tests;
