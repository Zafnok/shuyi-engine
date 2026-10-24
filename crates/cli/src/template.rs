//! Templates as data (ADR-0013 §3): a folder with a `template.ron` and
//! the files of its parts, which `shuyi new` turns into a new game.
//!
//! - [`Template`]: the manifest, read and checked by [`Template::load`]
//!   (a folder) or [`Template::read`] (its files, already in memory);
//! - [`Part`]: a folder of files, with the modules, targets and stage it
//!   adds, and [`When`] it applies;
//! - [`Question`]: a question of the template's own, and its answers;
//! - [`Answers`]: the answers picked, by question, as `answers.ron` holds
//!   them;
//! - [`compose`]: the template and the answers made into a [`Plan`], the
//!   files and their bytes, with no I/O; [`apply`] writes a plan.
//!
//! The engine's own template, `templates/shuyi-default`, is embedded in
//! the `shuyi` binary ([`built_in`]). `docs/templates.md` describes the
//! format. No template runs code: a template is data, and `new` writes
//! files.

mod compose;
mod ron_merge;

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use include_dir::{Dir, DirEntry, include_dir};
use schemars::JsonSchema;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

pub use compose::{
    Applied, ComposeError, Fill, GENERATED_STAGES, Plan, apply, compose, fill, stages_rs,
};

/// The manifest of a template, in its folder.
pub const MANIFEST: &str = "template.ron";

/// The version of the template format this `shuyi` reads.
pub const FORMAT_VERSION: u32 = 1;

/// The engine's version, which a template's `engine` must match.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The engine's own template, embedded.
static BUILT_IN: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../../templates/shuyi-default");

/// Where the engine's own template is, in the engine's repository.
pub const BUILT_IN_ORIGIN: &str = "templates/shuyi-default";

/// A template's files, by their path from its folder (`/`-separated),
/// with their bytes.
pub type Files = BTreeMap<String, Vec<u8>>;

/// A template: what `template.ron` says, with the files of its folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    /// Its id (`shuyi-default`).
    pub id: String,
    /// How people call it.
    pub name: String,
    /// Its own version.
    pub version: Version,
    /// The engine versions it makes games for (`>=0.1, <0.2`).
    pub engine: VersionReq,
    /// Its license, an SPDX expression: the license of the files it
    /// writes.
    pub license: String,
    /// The template format it is written in: [`FORMAT_VERSION`].
    pub format_version: u32,
    /// Its own questions, for what a game picks that no concern of the
    /// engine's asks.
    #[serde(default)]
    pub questions: Vec<Question>,
    /// Short names for sets of answers (`grid`), which `new --template`
    /// takes.
    #[serde(default)]
    pub short_names: BTreeMap<String, Answers>,
    /// Its parts, in the order their files are written and merged.
    pub parts: Vec<Part>,
    /// Every file of its folder but `template.ron`.
    #[serde(skip)]
    pub files: Files,
    /// Where it was read from, as errors name it.
    #[serde(skip)]
    pub origin: String,
}

/// A part of a template: a folder of files, what they turn on, and when
/// they are written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Part {
    /// Its id, which `requires` and `conflicts` name.
    pub id: String,
    /// When it applies.
    pub when: When,
    /// Its folder, from the template's: its files are written from there
    /// into the game's folder.
    pub files: PathBuf,
    /// The engine's modules it turns on (`physics`), as `shuyi add` does.
    #[serde(default)]
    pub modules: Vec<String>,
    /// The targets it ships on (`web`).
    #[serde(default)]
    pub targets: Vec<String>,
    /// The stage it adds, if any (`top-down-grid`): its module is
    /// `crates/game/src/stages/<stage, with `_` for `-`>.rs`.
    #[serde(default)]
    pub stage: Option<String>,
    /// The parts it needs picked too.
    #[serde(default)]
    pub requires: Vec<String>,
    /// The parts it can't be picked with.
    #[serde(default)]
    pub conflicts: Vec<String>,
}

/// When a part applies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum When {
    /// In every game.
    Always,
    /// When the question (its id first) has the answer (its id) picked.
    Picked(String, String),
    /// When each of these holds.
    All(Vec<When>),
    /// When one of these holds.
    Any(Vec<When>),
}

/// A question of a template's own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Question {
    /// Its id, which `Picked` and the answers name.
    pub id: String,
    /// The words that ask it.
    pub text: String,
    /// How many answers a game picks.
    pub pick: Pick,
    /// Its answers.
    pub answers: Vec<Answer>,
    /// The answers picked when none is given: none if empty.
    #[serde(default)]
    pub default: Vec<String>,
}

/// How many answers a question takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pick {
    /// One at most.
    One,
    /// Any number.
    Many,
}

/// One answer to a question.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// Its id.
    pub id: String,
    /// The words that say it.
    pub text: String,
}

/// The answers a game picked: by question id, the ids of its answers, as
/// `answers.ron` holds them (`{ "perspective": ["top-down-grid"] }`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(transparent)]
pub struct Answers(pub BTreeMap<String, Vec<String>>);

impl Answers {
    /// `answers.ron`'s text, read (`file` names it in an error).
    pub fn parse(file: &str, text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("{file}: {e}"))
    }

    /// As `answers.ron` writes them.
    pub fn to_ron(&self) -> String {
        let pretty = ron::ser::PrettyConfig::new();
        ron::ser::to_string_pretty(self, pretty).unwrap_or_default()
    }

    /// The answers of `--set <question>=<answer[,answer]>`.
    pub fn parse_set(set: &str) -> Result<(String, Vec<String>), String> {
        let (question, answers) = set
            .split_once('=')
            .ok_or_else(|| format!("--set {set}: not <question>=<answer[,answer]>"))?;
        let question = question.trim();
        if question.is_empty() {
            return Err(format!("--set {set}: no question before `=`"));
        }
        let answers = answers
            .split(',')
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .map(str::to_owned)
            .collect();
        Ok((question.to_owned(), answers))
    }

    /// These answers, with `other`'s in place of theirs for each question
    /// `other` answers.
    #[must_use]
    pub fn and(mut self, other: &Answers) -> Self {
        for (question, answers) in &other.0 {
            self.0.insert(question.clone(), answers.clone());
        }
        self
    }

    /// Whether `question` has `answer` picked.
    pub fn picked(&self, question: &str, answer: &str) -> bool {
        self.0
            .get(question)
            .is_some_and(|answers| answers.iter().any(|a| a == answer))
    }
}

impl When {
    /// Whether it holds for `answers`.
    pub fn holds(&self, answers: &Answers) -> bool {
        match self {
            When::Always => true,
            When::Picked(question, answer) => answers.picked(question, answer),
            When::All(all) => all.iter().all(|w| w.holds(answers)),
            When::Any(any) => any.iter().any(|w| w.holds(answers)),
        }
    }

    /// Why it holds for `answers`, in words: "always", or the answers that
    /// make it hold (`perspective` is `top-down-grid`).
    pub fn because(&self, answers: &Answers) -> String {
        match self {
            When::Always => "always".to_owned(),
            When::Picked(question, answer) => format!("`{question}` is `{answer}`"),
            When::All(all) => {
                let words: Vec<String> = all.iter().map(|w| w.because(answers)).collect();
                if words.is_empty() {
                    "always".to_owned()
                } else {
                    words.join(" and ")
                }
            }
            When::Any(any) => {
                let words: Vec<String> = any
                    .iter()
                    .filter(|w| w.holds(answers))
                    .map(|w| w.because(answers))
                    .collect();
                words.join(" or ")
            }
        }
    }

    /// Every `(question, answer)` it names, at any depth.
    pub fn picks(&self) -> Vec<(&str, &str)> {
        match self {
            When::Always => Vec::new(),
            When::Picked(question, answer) => vec![(question.as_str(), answer.as_str())],
            When::All(whens) | When::Any(whens) => whens.iter().flat_map(When::picks).collect(),
        }
    }
}

/// Why a template can't be read: the file, the field and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadError {
    /// The file (`templates/shuyi-default/template.ron`).
    pub file: String,
    /// The field (`parts[base].files`), or `""` for the whole file.
    pub field: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.field.is_empty() {
            write!(f, "{}: {}", self.file, self.message)
        } else {
            write!(f, "{}: {}: {}", self.file, self.field, self.message)
        }
    }
}

impl std::error::Error for LoadError {}

/// A part's folder as a prefix of the template's file paths (`base/`), or
/// why it is none.
pub(crate) fn folder_of(files: &Path) -> Result<String, String> {
    let text = files.to_string_lossy().replace('\\', "/");
    let trimmed = text.trim_matches('/');
    let escapes = trimmed
        .split('/')
        .any(|segment| segment == ".." || segment == "." || segment.is_empty());
    // An empty path is one empty segment, and an absolute one starts with
    // `/` here (a Windows drive's `C:` matches no file of the template).
    if text.starts_with('/') || escapes {
        return Err(format!(
            "`{text}` is no folder inside the template: give a path from its folder, with no \
             `.` or `..`"
        ));
    }
    Ok(format!("{trimmed}/"))
}

/// Whether `id` can be a stage's: lowercase ASCII letters and digits in
/// words joined by `-`, starting with a letter (it names a Rust module).
fn is_stage_id(id: &str) -> bool {
    id.starts_with(|c: char| c.is_ascii_lowercase())
        && id.split('-').all(|w| {
            !w.is_empty()
                && w.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

impl Template {
    /// The template in folder `dir`: its `template.ron` and every file
    /// under it, read and checked against this engine ([`ENGINE_VERSION`]).
    pub fn load(dir: &Path) -> Result<Self, LoadError> {
        let origin = dir.to_string_lossy().replace('\\', "/");
        let mut files = Files::new();
        read_dir(dir, dir, &mut files).map_err(|message| LoadError {
            file: origin.clone(),
            field: String::new(),
            message,
        })?;
        Self::read(&origin, files, &engine_version())
    }

    /// The template whose folder (`origin`, as errors name it) holds
    /// `files`, its `template.ron` among them, read and checked for an
    /// engine of version `engine`.
    pub fn read(origin: &str, mut files: Files, engine: &Version) -> Result<Self, LoadError> {
        let file = format!("{}/{MANIFEST}", origin.trim_end_matches('/'));
        let error = |field: &str, message: String| LoadError {
            file: file.clone(),
            field: field.to_owned(),
            message,
        };
        let bytes = files
            .remove(MANIFEST)
            .ok_or_else(|| error("", "missing: every template has one".to_owned()))?;
        let text =
            std::str::from_utf8(&bytes).map_err(|e| error("", format!("not UTF-8 text: {e}")))?;
        let mut template: Template = ron::from_str(text).map_err(|e| error("", e.to_string()))?;
        template.files = files;
        origin.clone_into(&mut template.origin);
        template
            .check(engine)
            .map_err(|(field, message)| error(&field, message))?;
        Ok(template)
    }

    /// The first thing wrong with it, as its field and what is wrong.
    fn check(&self, engine: &Version) -> Result<(), (String, String)> {
        if self.format_version != FORMAT_VERSION {
            return Err((
                "format_version".to_owned(),
                format!(
                    "{}: this shuyi reads templates of format {FORMAT_VERSION}",
                    self.format_version
                ),
            ));
        }
        if !self.engine.matches(engine) {
            return Err((
                "engine".to_owned(),
                format!(
                    "`{}` doesn't match this engine, {engine}: the template makes games for \
                     another engine",
                    self.engine
                ),
            ));
        }
        self.check_questions()?;
        for (name, answers) in &self.short_names {
            self.check_answers(answers)
                .map_err(|message| (format!("short_names[{name}]"), message))?;
        }
        self.check_parts()
    }

    /// Its questions: each id once, each answer id once, the defaults
    /// among the answers.
    fn check_questions(&self) -> Result<(), (String, String)> {
        let mut seen = Vec::new();
        for question in &self.questions {
            let field = format!("questions[{}]", question.id);
            if seen.contains(&question.id.as_str()) {
                return Err((field, "a second question with this id".to_owned()));
            }
            seen.push(&question.id);
            let mut ids = Vec::new();
            for answer in &question.answers {
                if ids.contains(&answer.id.as_str()) {
                    return Err((
                        format!("{field}.answers"),
                        format!("`{}` is in twice", answer.id),
                    ));
                }
                ids.push(&answer.id);
            }
            Self::check_picks(question, &question.default)
                .map_err(|message| (format!("{field}.default"), message))?;
        }
        Ok(())
    }

    /// Whether `picked` are answers of `question`, as many as it takes.
    fn check_picks(question: &Question, picked: &[String]) -> Result<(), String> {
        if let Some(unknown) = picked
            .iter()
            .find(|a| !question.answers.iter().any(|answer| &answer.id == *a))
        {
            return Err(format!(
                "`{unknown}` is no answer of `{}`: its answers are {}",
                question.id,
                ids(question.answers.iter().map(|a| a.id.as_str()))
            ));
        }
        if question.pick == Pick::One && picked.len() > 1 {
            return Err(format!(
                "`{}` takes one answer, not {}",
                question.id,
                picked.len()
            ));
        }
        Ok(())
    }

    /// Whether `answers` name its questions and their answers, as many as
    /// each takes.
    pub fn check_answers(&self, answers: &Answers) -> Result<(), String> {
        for (question, picked) in &answers.0 {
            let question = self.question(question).ok_or_else(|| {
                format!(
                    "no question `{question}`: the template asks {}",
                    ids(self.questions.iter().map(|q| q.id.as_str()))
                )
            })?;
            Self::check_picks(question, picked)?;
        }
        Ok(())
    }

    /// Its parts: each id once, a folder of files each, the parts and
    /// answers they name there.
    fn check_parts(&self) -> Result<(), (String, String)> {
        let mut seen = Vec::new();
        for part in &self.parts {
            let field = |name: &str| format!("parts[{}].{name}", part.id);
            if seen.contains(&part.id.as_str()) {
                return Err((
                    format!("parts[{}]", part.id),
                    "a second part with this id".to_owned(),
                ));
            }
            seen.push(&part.id);
            let folder = folder_of(&part.files).map_err(|m| (field("files"), m))?;
            if !self.files.keys().any(|path| path.starts_with(&folder)) {
                return Err((
                    field("files"),
                    format!("the folder `{folder}` holds no file"),
                ));
            }
            for (list, named) in [("requires", &part.requires), ("conflicts", &part.conflicts)] {
                if let Some(missing) = named
                    .iter()
                    .find(|id| !self.parts.iter().any(|p| &p.id == *id))
                {
                    return Err((field(list), format!("no part `{missing}`")));
                }
            }
            for (question, answer) in part.when.picks() {
                let q = self.question(question).ok_or_else(|| {
                    (
                        field("when"),
                        format!("`Picked` names no question of the template: `{question}`"),
                    )
                })?;
                if !q.answers.iter().any(|a| a.id == answer) {
                    return Err((
                        field("when"),
                        format!("`Picked` names no answer of `{question}`: `{answer}`"),
                    ));
                }
            }
            if let Some(stage) = &part.stage
                && !is_stage_id(stage)
            {
                return Err((
                    field("stage"),
                    format!(
                        "`{stage}`: a stage's id is lowercase letters and digits in words \
                         joined by `-`, starting with a letter"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// Its question `id`.
    pub fn question(&self, id: &str) -> Option<&Question> {
        self.questions.iter().find(|q| q.id == id)
    }

    /// The answers its short name `name` stands for.
    pub fn short_name(&self, name: &str) -> Option<&Answers> {
        self.short_names.get(name)
    }

    /// The answers picked: `answers`, and each question they leave out at
    /// its default.
    pub fn with_defaults(&self, answers: &Answers) -> Answers {
        let mut all = answers.clone();
        for question in &self.questions {
            all.0
                .entry(question.id.clone())
                .or_insert_with(|| question.default.clone());
        }
        all
    }
}

/// `ids` in backticks, joined by commas; `none` if there is none.
fn ids<'a>(ids: impl Iterator<Item = &'a str>) -> String {
    let ids: Vec<String> = ids.map(|id| format!("`{id}`")).collect();
    if ids.is_empty() {
        "none".to_owned()
    } else {
        ids.join(", ")
    }
}

/// The engine's version, parsed.
pub fn engine_version() -> Version {
    // The package's version is semver: cargo checks it.
    Version::parse(ENGINE_VERSION).unwrap_or_else(|_| Version::new(0, 0, 0))
}

/// Every file under `dir`, into `files` by its path from `root`.
fn read_dir(root: &Path, dir: &Path, files: &mut Files) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
        let path = entry.path();
        let kind = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if kind.is_dir() {
            read_dir(root, &path, files)?;
        } else if kind.is_file() {
            let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let name = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            files.insert(name, bytes);
        }
    }
    Ok(())
}

/// The files of an embedded folder, at any depth, by their path from it.
fn embedded_files(dir: &'static Dir<'static>) -> Files {
    let mut files = Files::new();
    let mut pending = vec![dir];
    while let Some(dir) = pending.pop() {
        for entry in dir.entries() {
            match entry {
                DirEntry::Dir(d) => pending.push(d),
                DirEntry::File(f) => {
                    let name = f.path().to_string_lossy().replace('\\', "/");
                    files.insert(name, f.contents().to_vec());
                }
            }
        }
    }
    files
}

/// The engine's own template, `templates/shuyi-default`, as the `shuyi`
/// binary embeds it.
pub fn built_in() -> Result<Template, LoadError> {
    Template::read(
        BUILT_IN_ORIGIN,
        embedded_files(&BUILT_IN),
        &engine_version(),
    )
}

#[cfg(test)]
mod tests;
