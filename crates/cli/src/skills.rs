//! The engine's skill pack (ADR-0005 §2, ADR-0043): `skills/<name>/SKILL.md`
//! folders in the open Agent Skills format, each with a worked example
//! (`example.ron`, a script of contract commands) unless it waits for work
//! still to come.
//!
//! - [`Pack`]: the skills of a folder, by name, with their files: read from
//!   a checkout of the engine ([`Pack::read`]) or the one this `shuyi`
//!   embeds ([`Pack::embedded`]). `shuyi new` copies a pack's skills into a
//!   new game's `.claude/skills/`, all but the modules' (`shuyi add` copies
//!   those);
//! - [`Frontmatter`]: what a `SKILL.md` says of itself, between its `---`
//!   lines: the open format's keys, and in `metadata` the pack's own,
//!   `version` and `waits_for`;
//! - [`check`]: `shuyi check-skills`' checks of a pack, each problem with
//!   its file and line: the frontmatter, the `shuyi` commands the skills
//!   name, the files they link, their length, what they wait for and their
//!   worked examples, and the problem codes they name.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path};

use include_dir::{Dir, DirEntry, include_dir};
use serde::Deserialize;

use crate::contract::Op;
use crate::contract::codes;
use crate::contract::new::ENGINE_GIT;

/// The skill pack, from the engine's root.
pub const PACK: &str = "skills";
/// A skill's own file, in its folder.
pub const FILE: &str = "SKILL.md";
/// A skill's worked example, in its folder.
pub const EXAMPLE: &str = "example.ron";
/// The most lines a `SKILL.md` may have: the open format keeps the rest in
/// files it loads on demand.
pub const MAX_LINES: usize = 500;
/// The longest a skill's name may be.
pub const MAX_NAME: usize = 64;
/// The longest a skill's description may be.
pub const MAX_DESCRIPTION: usize = 1024;
/// The longest a skill's `compatibility` may be.
pub const MAX_COMPATIBILITY: usize = 500;

/// The pack this `shuyi` was built with, embedded.
static EMBEDDED: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../skills");

/// A skill: its name (its folder's) and its files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skill {
    /// Its name, its folder's.
    pub name: String,
    /// Every file of its folder, by its path from there (`/`-separated),
    /// with its bytes.
    pub files: BTreeMap<String, Vec<u8>>,
}

impl Skill {
    /// Its `SKILL.md`, as text.
    pub fn text(&self) -> Option<String> {
        self.files
            .get(FILE)
            .map(|bytes| String::from_utf8_lossy(bytes).replace("\r\n", "\n"))
    }

    /// Its frontmatter, read; or what is wrong with it.
    pub fn frontmatter(&self) -> Result<Frontmatter, String> {
        let text = self.text().ok_or_else(|| format!("no {FILE}"))?;
        Frontmatter::parse(&text).map(|(frontmatter, _)| frontmatter)
    }

    /// Whether it has a worked example.
    pub fn has_example(&self) -> bool {
        self.files.contains_key(EXAMPLE)
    }
}

/// A skill pack: its skills, by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pack {
    /// Its skills, in name order.
    pub skills: Vec<Skill>,
    /// Its folders that hold no `SKILL.md`, by name.
    pub not_skills: Vec<String>,
}

impl Pack {
    /// The pack in folder `dir`: each folder in it with its files (a file
    /// beside them, a README, is no skill). A missing folder is an empty
    /// pack.
    pub fn read(dir: &Path) -> Result<Self, String> {
        let mut pack = Self::default();
        let Ok(entries) = fs::read_dir(dir) else {
            return Ok(pack);
        };
        for entry in entries {
            let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            let mut files = BTreeMap::new();
            read_files(&path, &path, &mut files)?;
            pack.add(name, files);
        }
        pack.sort();
        Ok(pack)
    }

    /// The pack this `shuyi` was built with.
    pub fn embedded() -> Self {
        let mut pack = Self::default();
        for entry in EMBEDDED.entries() {
            if let DirEntry::Dir(dir) = entry {
                let name = dir
                    .path()
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut files = BTreeMap::new();
                embedded_files(dir, dir.path(), &mut files);
                pack.add(name, files);
            }
        }
        pack.sort();
        pack
    }

    /// The skill named `name`.
    pub fn skill(&self, name: &str) -> Option<&Skill> {
        self.skills.iter().find(|s| s.name == name)
    }

    fn add(&mut self, name: String, files: BTreeMap<String, Vec<u8>>) {
        if files.contains_key(FILE) {
            self.skills.push(Skill { name, files });
        } else {
            self.not_skills.push(name);
        }
    }

    fn sort(&mut self) {
        self.skills.sort_by(|a, b| a.name.cmp(&b.name));
        self.not_skills.sort();
    }
}

/// Every file under `dir` into `files`, by its path from `root`.
fn read_files(
    root: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if kind.is_dir() {
            read_files(root, &path, files)?;
        } else if kind.is_file() {
            let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            files.insert(shown(path.strip_prefix(root).unwrap_or(&path)), bytes);
        }
    }
    Ok(())
}

/// Every file of an embedded folder into `files`, by its path from `root`.
fn embedded_files(dir: &Dir<'_>, root: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
    for entry in dir.entries() {
        match entry {
            DirEntry::Dir(d) => embedded_files(d, root, files),
            DirEntry::File(f) => {
                let path = f.path().strip_prefix(root).unwrap_or(f.path());
                files.insert(shown(path), f.contents().to_vec());
            }
        }
    }
}

/// A path, `/`-separated.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// What a `SKILL.md` says of itself: the open Agent Skills format's keys,
/// and no others.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frontmatter {
    /// Its name: its folder's.
    pub name: String,
    /// What it does, when to use it and when not.
    pub description: String,
    /// The license of its files.
    #[serde(default)]
    pub license: Option<String>,
    /// What it needs to work.
    #[serde(default)]
    pub compatibility: Option<String>,
    /// The tools it may use without asking, as some agents read it.
    #[serde(default, rename = "allowed-tools")]
    pub allowed_tools: Option<String>,
    /// Its own keys, each a string: the pack's are `version` and
    /// `waits_for`.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
}

impl Frontmatter {
    /// The frontmatter of `SKILL.md`'s text, and the line its body starts
    /// on (from 1); or what is wrong with it.
    pub fn parse(text: &str) -> Result<(Self, usize), String> {
        let rest = text
            .strip_prefix("---\n")
            .ok_or("the file starts with no `---` line before its frontmatter")?;
        let end = rest
            .find("\n---\n")
            .or_else(|| rest.strip_suffix("\n---").map(str::len))
            .ok_or("the frontmatter has no closing `---` line")?;
        let yaml = &rest[..end];
        let frontmatter: Self = serde_norway::from_str(yaml).map_err(|e| {
            format!("the frontmatter doesn't read as the open format's keys, each metadata value a string: {e}")
        })?;
        Ok((frontmatter, yaml.lines().count() + 3))
    }

    /// The ticket ids it waits for: `metadata.waits_for`, split on spaces
    /// and commas.
    pub fn waits_for(&self) -> Vec<String> {
        self.metadata
            .get("waits_for")
            .map(|ids| {
                ids.split([' ', ','])
                    .filter(|id| !id.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// A problem `check` found: the file (from the pack's folder), its line
/// when it has one, and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    /// The file, from the pack's folder (`shuyi-gates/SKILL.md`).
    pub file: String,
    /// Its line, from 1.
    pub line: Option<usize>,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{line}: {}", self.file, self.message),
            None => write!(f, "{}: {}", self.file, self.message),
        }
    }
}

/// What `check` checks against, besides the pack.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Against {
    /// The `shuyi` commands a skill may name: the contract's operations,
    /// the MCP session's tools and the other commands.
    pub commands: BTreeSet<String>,
    /// The ids of the tickets still to come, if known: each id in a
    /// `waits_for` must be one.
    pub open_tickets: Option<BTreeSet<String>>,
}

impl Against {
    /// This `shuyi`'s commands ([`crate::COMMANDS`] and the contract's),
    /// and the ticket ids of the open tickets in `tickets` if given (each
    /// `<id>-….md` file there).
    pub fn this_shuyi(tickets: Option<&Path>) -> Result<Self, String> {
        let mut commands: BTreeSet<String> = Op::ALL
            .iter()
            .chain(Op::SESSION.iter())
            .map(|op| op.name().to_owned())
            .collect();
        commands.extend(crate::COMMANDS.iter().map(|c| (*c).to_owned()));
        let open_tickets = match tickets {
            None => None,
            Some(dir) => {
                let entries =
                    fs::read_dir(dir).map_err(|e| format!("--tickets {}: {e}", dir.display()))?;
                let ids = entries
                    .flatten()
                    .filter_map(|e| {
                        let name = e.file_name().to_string_lossy().into_owned();
                        let id = name.split_once('-')?.0.to_owned();
                        Path::new(&name)
                            .extension()
                            .is_some_and(|x| x == "md")
                            .then_some(id)
                    })
                    .collect();
                Some(ids)
            }
        };
        Ok(Self {
            commands,
            open_tickets,
        })
    }
}

/// Every problem of `pack`, checked against `against`, in order of skill
/// and file:
///
/// 1. its frontmatter reads, with its folder's `name` (lowercase letters,
///    digits and single `-`), a `description` that says when to use it
///    (`Use …`) and when not (`Not …`), and a `metadata.version` (`1.0`);
/// 2. every `shuyi <command>` in the code of its Markdown files is a
///    command of `shuyi`;
/// 3. every relative link of its Markdown files names a file of the pack,
///    and none names the engine's repository on `main` (a skill sends an
///    agent to the project's `AGENTS.md`, which names the engine's docs at
///    the version the game builds with);
/// 4. its name, description, compatibility and `SKILL.md` are within the
///    format's lengths ([`MAX_NAME`], [`MAX_DESCRIPTION`],
///    [`MAX_COMPATIBILITY`], [`MAX_LINES`]);
/// 5. every id in its `waits_for` is an open ticket (when they are
///    known), and a skill of the engine's pack ([`Pack::embedded`], by
///    name) that waits for nothing has a worked example. A project's own
///    skill, one the embedded pack doesn't have, needs none: it teaches
///    the project's own workflow, with nothing for a new game to run;
/// 6. every problem code (`SH2003`) in its files is a row of the table
///    (`shuyi explain`) that isn't retired.
///
/// A folder of the pack with no `SKILL.md` is a problem too.
pub fn check(pack: &Pack, against: &Against) -> Vec<Problem> {
    let mut problems: Vec<Problem> = pack
        .not_skills
        .iter()
        .map(|name| Problem {
            file: format!("{name}/"),
            line: None,
            message: format!("a folder of the pack with no {FILE}: every skill has one"),
        })
        .collect();
    let engines: BTreeSet<String> = Pack::embedded()
        .skills
        .into_iter()
        .map(|skill| skill.name)
        .collect();
    for skill in &pack.skills {
        let engines = engines.contains(&skill.name);
        check_skill(pack, skill, engines, against, &mut problems);
    }
    problems
}

/// The problems of one skill of `pack`, into `problems`; `engines` when
/// it is a skill of the engine's pack, which has a worked example.
fn check_skill(
    pack: &Pack,
    skill: &Skill,
    engines: bool,
    against: &Against,
    problems: &mut Vec<Problem>,
) {
    let main = format!("{}/{FILE}", skill.name);
    let problem = |file: &str, line: Option<usize>, message: String| Problem {
        file: file.to_owned(),
        line,
        message,
    };
    let text = skill.text().unwrap_or_default();
    match Frontmatter::parse(&text) {
        Err(e) => problems.push(problem(&main, Some(1), e)),
        Ok((frontmatter, _)) => {
            for message in frontmatter_problems(&skill.name, &frontmatter) {
                problems.push(problem(&main, Some(1), message));
            }
            let waits = frontmatter.waits_for();
            if let Some(open) = &against.open_tickets {
                for id in waits.iter().filter(|id| !open.contains(*id)) {
                    problems.push(problem(
                        &main,
                        Some(1),
                        format!(
                            "waits_for: {id} is no open ticket: if it is done, take its id out, \
                             and write the skill's worked example once nothing is left"
                        ),
                    ));
                }
            }
            if engines && waits.is_empty() && !skill.has_example() {
                problems.push(problem(
                    &main,
                    None,
                    format!(
                        "no worked example ({EXAMPLE}): a skill of the engine's pack that waits \
                         for nothing has one"
                    ),
                ));
            }
        }
    }
    let lines = text.lines().count();
    if lines > MAX_LINES {
        problems.push(problem(
            &main,
            None,
            format!(
                "{lines} lines, over {MAX_LINES}: move the detail into a file it links, which \
                 an agent reads when it needs it"
            ),
        ));
    }
    for (path, bytes) in &skill.files {
        let Ok(text) = std::str::from_utf8(bytes) else {
            continue;
        };
        let text = text.replace("\r\n", "\n");
        let file = format!("{}/{path}", skill.name);
        for (line, code) in codes_in(&text) {
            if let Some(message) = code_problem(&code, codes::entry(&code)) {
                problems.push(problem(&file, Some(line), message));
            }
        }
        let markdown = Path::new(path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if !markdown {
            continue;
        }
        for (line, command) in commands_in(&text) {
            if !against.commands.contains(&command) {
                problems.push(problem(
                    &file,
                    Some(line),
                    format!("`shuyi {command}`: no such command (`shuyi` alone lists them)"),
                ));
            }
        }
        for (line, target) in links_in(&text) {
            if let Some(message) = link_problem(pack, &skill.name, path, &target) {
                problems.push(problem(&file, Some(line), message));
            }
        }
    }
}

/// What is wrong with a skill naming problem code `code`, whose row in the
/// table is `entry`: none, or a retired one.
fn code_problem(code: &str, entry: Option<&codes::Entry>) -> Option<String> {
    match entry {
        None => Some(format!(
            "{code} is no problem code: `shuyi explain` lists them"
        )),
        Some(entry) if entry.retired => {
            Some(format!("{code} is retired: no error gives it any more"))
        }
        Some(_) => None,
    }
}

/// What is wrong with the frontmatter of the skill in folder `folder`.
fn frontmatter_problems(folder: &str, frontmatter: &Frontmatter) -> Vec<String> {
    let mut problems = Vec::new();
    let name = &frontmatter.name;
    if name != folder {
        problems.push(format!(
            "name: `{name}` is not its folder's name, `{folder}`"
        ));
    }
    let well_formed = !name.is_empty()
        && name.split('-').all(|w| {
            !w.is_empty()
                && w.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        });
    if !well_formed {
        problems.push(format!(
            "name: `{name}`: lowercase letters and digits in words joined by single `-`"
        ));
    }
    if name.chars().count() > MAX_NAME {
        problems.push(format!("name: longer than {MAX_NAME} characters"));
    }
    let description = &frontmatter.description;
    if !description.contains("Use ") {
        problems.push("description: say when to use it, in a sentence starting `Use …`".to_owned());
    }
    if !description.contains("Not ") {
        problems
            .push("description: say when not to use it, in a sentence starting `Not …`".to_owned());
    }
    let length = description.chars().count();
    if length > MAX_DESCRIPTION {
        problems.push(format!(
            "description: {length} characters, over {MAX_DESCRIPTION}"
        ));
    }
    if let Some(compatibility) = &frontmatter.compatibility
        && compatibility.chars().count() > MAX_COMPATIBILITY
    {
        problems.push(format!(
            "compatibility: longer than {MAX_COMPATIBILITY} characters"
        ));
    }
    let version = frontmatter.metadata.get("version");
    let is_version = version.is_some_and(|v| {
        !v.is_empty()
            && v.split('.')
                .all(|n| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()))
    });
    if !is_version {
        problems.push(
            "metadata.version: give the skill a version, a string of numbers and dots (\"1.0\")"
                .to_owned(),
        );
    }
    problems
}

/// Pieces of a text, each with the line it starts on (from 1).
type Pieces = Vec<(usize, String)>;

/// The code and the prose of `text`. A fenced block's lines are code, and
/// so is an inline span between backticks, which may wrap onto the next
/// lines of its paragraph; the rest of each line is prose.
fn split_code(text: &str) -> (Pieces, Pieces) {
    let mut code = Vec::new();
    let mut prose = Vec::new();
    let mut fenced = false;
    let mut span: Option<(usize, String)> = None;
    for (i, line) in text.lines().enumerate() {
        let trimmed = line.trim_start();
        let fence = trimmed.starts_with("```") || trimmed.starts_with("~~~");
        if fence || trimmed.is_empty() {
            // A span still open at a paragraph's end was a lone backtick.
            span = None;
            fenced ^= fence;
            continue;
        }
        if fenced {
            code.push((i + 1, line.to_owned()));
            continue;
        }
        let mut outside = String::new();
        for (n, part) in line.split('`').enumerate() {
            if n > 0 {
                span = match span.take() {
                    Some(closed) => {
                        code.push(closed);
                        None
                    }
                    None => Some((i + 1, String::new())),
                };
            }
            if let Some((_, text)) = &mut span {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(part);
            } else {
                outside.push_str(part);
                outside.push(' ');
            }
        }
        prose.push((i + 1, outside));
    }
    (code, prose)
}

/// Each `shuyi <command>` in the code of `text`, with its line: the word
/// after a `shuyi` that starts the code or follows a space.
fn commands_in(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (line, code) in split_code(text).0 {
        let words: Vec<&str> = code.split_whitespace().collect();
        for pair in words.windows(2) {
            let (word, next) = (pair[0], pair[1]);
            let starts = next.starts_with(|c: char| c.is_ascii_lowercase());
            if word == "shuyi" && starts {
                let command: String = next
                    .chars()
                    .take_while(|c| {
                        c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '_'
                    })
                    .collect();
                found.push((line, command));
            }
        }
    }
    found
}

/// Each problem code (`SH` and four digits) in `text`, with its line.
fn codes_in(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let mut at = 0;
        while let Some(offset) = line[at..].find("SH") {
            let start = at + offset;
            let digits = &bytes[start + 2..];
            let four = digits.len() >= 4 && digits[..4].iter().all(u8::is_ascii_digit);
            let fifth = digits.get(4).is_some_and(u8::is_ascii_alphanumeric);
            let before = start > 0 && bytes[start - 1].is_ascii_alphanumeric();
            if four && !fifth && !before {
                found.push((i + 1, line[start..start + 6].to_owned()));
            }
            at = start + 2;
        }
    }
    found
}

/// Each link's target in the prose of `text` (`[words](target)`), with its
/// line, and every URL of the engine's repository anywhere in it.
fn links_in(text: &str) -> Vec<(usize, String)> {
    let mut found = Vec::new();
    for (line, prose) in split_code(text).1 {
        let mut rest = prose.as_str();
        while let Some(at) = rest.find("](") {
            let after = &rest[at + 2..];
            let Some(end) = after.find(')') else {
                break;
            };
            found.push((line, after[..end].trim().to_owned()));
            rest = &after[end..];
        }
    }
    for (i, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(at) = rest.find(ENGINE_GIT) {
            let url: String = rest[at..]
                .chars()
                .take_while(|c| !c.is_whitespace() && !matches!(c, ')' | '>' | '`' | '"'))
                .collect();
            if !found.iter().any(|(l, t)| *l == i + 1 && *t == url) {
                found.push((i + 1, url.clone()));
            }
            rest = &rest[at + ENGINE_GIT.len()..];
        }
    }
    found
}

/// What is wrong with a link to `target` in the file `path` of skill
/// `skill`, if anything.
fn link_problem(pack: &Pack, skill: &str, path: &str, target: &str) -> Option<String> {
    if target.starts_with(ENGINE_GIT) {
        let on_main = ["/blob/main/", "/tree/main/", "/blob/main", "/tree/main"]
            .iter()
            .any(|m| target[ENGINE_GIT.len()..].starts_with(m));
        return on_main.then(|| {
            format!(
                "{target}: a fixed page on the engine's `main`: send the agent to the project's \
                 `AGENTS.md`, which names the engine's docs at the version the game builds with"
            )
        });
    }
    let target = target.split(['#', '?']).next().unwrap_or(target);
    let external = target.contains("://") || target.starts_with("mailto:");
    if target.is_empty() || external {
        return None;
    }
    // The link from the pack's folder: the skill's folder, the file's own
    // folder in it, then the target.
    let mut at: Vec<String> = vec![skill.to_owned()];
    at.extend(path.split('/').map(str::to_owned));
    at.pop();
    for component in Path::new(target).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if at.pop().is_none() {
                    return Some(format!(
                        "{target}: the link leaves the skill pack, which is all a game gets: \
                         link a file of the pack, or name the file in words"
                    ));
                }
            }
            Component::Normal(name) => at.push(name.to_string_lossy().into_owned()),
            Component::RootDir | Component::Prefix(_) => {
                return Some(format!(
                    "{target}: a link from the root: give it from the file's folder"
                ));
            }
        }
    }
    let Some((first, rest)) = at.split_first() else {
        return Some(format!(
            "{target}: the link names the pack's folder, not a file"
        ));
    };
    let exists = pack.skill(first).is_some_and(|s| {
        let path = rest.join("/");
        path.is_empty()
            || s.files.contains_key(&path)
            || s.files.keys().any(|f| f.starts_with(&format!("{path}/")))
    });
    (!exists).then(|| format!("{target}: no such file in the skill pack"))
}

#[cfg(test)]
mod tests;
