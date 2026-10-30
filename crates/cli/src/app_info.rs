//! Who the game is: `shuyi.ron`'s `app` part ([`AppPart`], ADR-0059), its
//! rules ([`problems`], which `check --target` reports), and `app-info`,
//! which writes it as the Rust constant the game's binary compiles
//! ([`rust`], [`run`]).
//!
//! The rules:
//!
//! - `name`: not empty, with no control character;
//! - `id`: reverse-DNS, two or more parts between dots, each of ASCII
//!   letters, digits and `-`, starting with a letter; an id under
//!   `com.example.` is a warning, since two games with it share their
//!   saves;
//! - `version`, when given: semver, `MAJOR.MINOR.PATCH` then an optional
//!   `-pre-release` and `+build`;
//! - `build`: 1 or more;
//! - `storage`, or the id when it is unset: one folder name;
//! - the file `app-info` writes (`code`) is the one on disk.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::contract::target::Level;
use crate::project::{AppPart, FILE};

/// What `app-info` and the rules call the part.
const PART: &str = "app";

/// The id's start that marks it as a placeholder.
pub const PLACEHOLDER: &str = "com.example.";

/// A problem of the `app` part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppProblem {
    /// An error or a warning.
    pub level: Level,
    /// The field it is about: `app.id`.
    pub setting: String,
    /// What is wrong.
    pub message: String,
}

impl AppProblem {
    fn new(level: Level, field: &str, message: impl Into<String>) -> Self {
        Self {
            level,
            setting: format!("{PART}.{field}"),
            message: message.into(),
        }
    }

    fn error(field: &str, message: impl Into<String>) -> Self {
        Self::new(Level::Error, field, message)
    }
}

/// Every problem of `part`, in the project at `root` whose tool is called
/// `command`: its fields by the rules, and the file `app-info` writes when
/// it isn't the one on disk.
pub fn problems(part: &AppPart, root: &Path, command: &str) -> Vec<AppProblem> {
    let mut out = fields(part);
    for path in stale(part, root, command) {
        out.push(AppProblem::error(
            "code",
            format!("{path} is out of date: run `{command} app-info`"),
        ));
    }
    out
}

/// The problems of `part`'s fields, without reading a file.
fn fields(part: &AppPart) -> Vec<AppProblem> {
    let mut out = Vec::new();
    if let Err(e) = check_name(&part.name) {
        out.push(AppProblem::error("name", e));
    }
    match check_id(&part.id) {
        Err(e) => out.push(AppProblem::error("id", e)),
        Ok(()) if part.id.starts_with(PLACEHOLDER) => out.push(AppProblem::new(
            Level::Warning,
            "id",
            format!(
                "\"{}\" is a placeholder: two games with this id share their saves on one \
                 computer and on one web origin; set your own before you hand out a build, and \
                 before the first release, since the save folder follows it",
                part.id
            ),
        )),
        Ok(()) => {}
    }
    if let Some(version) = &part.version
        && let Err(e) = check_version(version)
    {
        out.push(AppProblem::error("version", e));
    }
    if part.build == 0 {
        out.push(AppProblem::error("build", "the build is 0: it starts at 1"));
    }
    let field = if part.storage.is_some() {
        "storage"
    } else {
        "id"
    };
    if let Err(e) = check_storage(part.storage()) {
        out.push(AppProblem::error(field, e));
    }
    out
}

/// Whether `name` can be the name players see: not empty, with no
/// control character.
fn check_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() {
        return Err("the name is empty".to_owned());
    }
    match name.chars().find(|c| c.is_control()) {
        Some(c) => Err(format!("the name may not hold {c:?}")),
        None => Ok(()),
    }
}

/// Whether `id` is a reverse-DNS id: two or more parts between dots, each
/// of ASCII letters, digits and `-`, starting with a letter. These are the
/// characters Apple allows in a bundle id, with a letter first as Android
/// asks.
pub fn check_id(id: &str) -> Result<(), String> {
    let parts: Vec<&str> = id.split('.').collect();
    if parts.len() < 2 {
        return Err(format!(
            "\"{id}\" isn't reverse-DNS: it needs two or more parts between dots, as \
             com.example.moon-garden"
        ));
    }
    for part in parts {
        if !part.starts_with(|c: char| c.is_ascii_alphabetic()) {
            return Err(format!(
                "\"{id}\": each part between dots starts with a letter, and \"{part}\" doesn't"
            ));
        }
        if let Some(c) = part
            .chars()
            .find(|&c| !(c.is_ascii_alphanumeric() || c == '-'))
        {
            return Err(format!(
                "\"{id}\" may hold only ASCII letters, digits, `-` and dots, not {c:?}"
            ));
        }
    }
    Ok(())
}

/// Whether `version` is semver: `MAJOR.MINOR.PATCH` in digits, then an
/// optional `-pre-release` and `+build`.
pub fn check_version(version: &str) -> Result<(), String> {
    semver::Version::parse(version).map(|_| ()).map_err(|e| {
        format!("\"{version}\" isn't semver (MAJOR.MINOR.PATCH, as 1.2.0 or 1.2.0-beta.1): {e}")
    })
}

/// Whether `storage` can name a folder on every system: ASCII letters,
/// digits, `.`, `-`, `_` and spaces, with no `.` or space first or last.
pub fn check_storage(storage: &str) -> Result<(), String> {
    if storage.is_empty() {
        return Err("the storage's name is empty".to_owned());
    }
    if let Some(c) = storage
        .chars()
        .find(|&c| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ')))
    {
        return Err(format!(
            "the storage's name \"{storage}\" names a folder: ASCII letters, digits, `.`, `-`, \
             `_` and spaces only, not {c:?}"
        ));
    }
    let ends = [storage.chars().next(), storage.chars().last()];
    if ends.iter().flatten().any(|&c| matches!(c, '.' | ' ')) {
        return Err(format!(
            "the storage's name \"{storage}\" names a folder: no `.` or space first or last"
        ));
    }
    Ok(())
}

/// `text` as a Rust string literal: in quotes, with only `"`, `\` and
/// control characters escaped.
fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            // Writing to a `String` can't fail.
            c if c.is_control() => {
                let _ = write!(out, "\\u{{{:x}}}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The Rust file `app-info` writes for `part`, with `command` as the
/// tool's name: one `pub const APP: AppInfo`, kept from `rustfmt`, so
/// `app-info --check` is its only judge.
pub fn rust(part: &AppPart, command: &str) -> String {
    let version = part
        .version
        .as_deref()
        .map_or_else(|| "env!(\"CARGO_PKG_VERSION\")".to_owned(), quoted);
    let mut app = format!(
        "AppInfo::new({}, {}, {version})",
        quoted(&part.name),
        quoted(&part.id)
    );
    if let Some(storage) = &part.storage {
        let _ = write!(app, "\n    .with_storage({})", quoted(storage));
    }
    if part.build != 1 {
        let _ = write!(app, "\n    .with_build({})", part.build);
    }
    if !part.publisher.is_empty() {
        let _ = write!(app, "\n    .with_publisher({})", quoted(&part.publisher));
    }
    format!(
        "// Generated by `{command} {COMMAND}` from {FILE}'s `{PART}`. Do not edit.\n\
         //! Who the game is, as `{FILE}`'s `{PART}` says: its name, id and version, and the\n\
         //! name its saves are kept under.\n\
         \n\
         use shuyi::platform::AppInfo;\n\
         \n\
         /// Who the game is: the window's title, the loading card and the storage read it.\n\
         // check-text: not player text, who the game is (`{FILE}`'s `{PART}`)\n\
         #[rustfmt::skip]\n\
         pub const APP: AppInfo = {app};\n"
    )
}

/// The command's name.
pub const COMMAND: &str = "app-info";

/// The files `app-info` writes for `part`: each path from the project's
/// root and its bytes.
pub fn files(part: &AppPart, command: &str) -> Vec<(String, Vec<u8>)> {
    part.code
        .iter()
        .map(|path| (path.clone(), rust(part, command).into_bytes()))
        .collect()
}

/// The files `app-info` would write for `part` that differ from those in
/// `root` (a missing file differs), by their paths. A text file's line
/// endings don't count.
pub fn stale(part: &AppPart, root: &Path, command: &str) -> Vec<String> {
    files(part, command)
        .into_iter()
        .filter(|(path, bytes)| {
            fs::read(root.join(path)).map_or(true, |disk| {
                let lf: Vec<u8> = disk.into_iter().filter(|&b| b != b'\r').collect();
                lf != *bytes
            })
        })
        .map(|(path, _)| path)
        .collect()
}

/// `app-info [--check]` on `part` in the project at `root`: writes its
/// files, or with `check` writes nothing and fails naming each file that
/// differs. A part with an error writes nothing and fails with them.
pub fn run(part: &AppPart, root: &Path, command: &str, check: bool) -> Result<String, String> {
    let errors: Vec<String> = fields(part)
        .into_iter()
        .filter(|p| p.level == Level::Error)
        .map(|p| format!("{}: {}", p.setting, p.message))
        .collect();
    if !errors.is_empty() {
        return Err(format!(
            "{FILE}'s `{PART}` has {} error(s):\n  {}",
            errors.len(),
            errors.join("\n  ")
        ));
    }
    if part.code.is_none() {
        return Ok(format!(
            "{COMMAND}: {FILE}'s `{PART}` names no `code`: no file to write"
        ));
    }
    if check {
        let stale = stale(part, root, command);
        if stale.is_empty() {
            return Ok(format!("{COMMAND}: up to date"));
        }
        return Err(format!(
            "{} file(s) out of date: {}: run `{command} {COMMAND}`",
            stale.len(),
            stale.join(", ")
        ));
    }
    let mut wrote = Vec::new();
    for (path, bytes) in files(part, command) {
        let full = root.join(&path);
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
        }
        fs::write(&full, bytes).map_err(|e| format!("write {path}: {e}"))?;
        wrote.push(path);
    }
    Ok(format!("{COMMAND}: wrote {}", wrote.join(", ")))
}

/// `app-info`'s arguments: none, or `--check`. `Ok(true)` for `--check`.
pub fn parse_args(args: &[String]) -> Result<bool, String> {
    match args {
        [] => Ok(false),
        [flag] if flag == "--check" => Ok(true),
        _ => Err("no arguments, or --check".to_owned()),
    }
}

#[cfg(test)]
mod tests;
