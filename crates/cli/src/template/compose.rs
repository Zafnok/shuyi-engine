//! [`compose`]: a template and a game's answers made into a [`Plan`], the
//! files of the new game and their bytes, with no I/O; and [`apply`],
//! which writes a plan into a folder.
//!
//! The parts whose `when` holds are picked in the template's order. Each
//! writes its folder's files, with the placeholders filled ([`fill`]).
//! A path more than one part writes is merged when it is a `Cargo.toml`
//! (key by key, [`toml_edit`]) or a `.ron` file (field by field, entry by
//! entry, item by item, keeping each file's comments); any other is
//! [`ComposeError::SameFile`]. The modules the parts turn on become the
//! `shuyi` crate's features in the root `Cargo.toml`, through `shuyi
//! add`'s code ([`catalog::turn_on`]). When the template has a part with a
//! stage, `shuyi new` writes the flow's list of stages,
//! [`GENERATED_STAGES`] ([`stages_rs`]). Every game gets the engine's skill
//! pack (the catalog's [`Catalog::skills`]) in `.claude/skills/`, all but
//! the modules' skills, which `shuyi add` copies when a game turns its
//! module on.

use std::collections::BTreeMap;
use std::fmt::{self, Write as _};
use std::fs;
use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Table, Value};

use super::{Answers, Part, Template, folder_of, ron_merge};
use crate::catalog::{self, Catalog};

/// The flow's list of stages, which `shuyi new` writes from the parts'
/// stages.
pub const GENERATED_STAGES: &str = "crates/game/src/stages.rs";

/// The files the parts write, by path, each with the parts that write it
/// and their bytes.
type Written = BTreeMap<String, Vec<(String, Vec<u8>)>>;

/// Who writes [`GENERATED_STAGES`], as errors name it.
const COMPOSER: &str = "shuyi new";

/// What fills a template's placeholders, but the answers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fill {
    /// `{{name}}`: the game's name, as given (`Moon Garden`).
    pub name: String,
    /// `{{crate_name}}`: its crate's name (`moon-garden`).
    pub crate_name: String,
    /// `"{{engine.<crate>}}"`: by an engine crate's folder under
    /// `crates/` (`core`), the dependency on it, a TOML inline table.
    pub engine: BTreeMap<String, String>,
}

/// What a template makes for a set of answers: the new game's files and
/// what its parts turn on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Every file, by its path from the game's folder (`/`-separated),
    /// with its bytes.
    pub files: BTreeMap<PathBuf, Vec<u8>>,
    /// The parts picked, in the template's order.
    pub parts: Vec<String>,
    /// The engine's modules turned on, by crate (`shuyi-physics`).
    pub modules: Vec<String>,
    /// The targets the parts ship on.
    pub targets: Vec<String>,
    /// The stages, in order.
    pub stages: Vec<String>,
    /// The engine's skills it holds in `.claude/skills/`, by name: every
    /// skill of its pack but the modules'.
    pub skills: Vec<String>,
    /// The answers it was made for, each question the answers left out at
    /// its default.
    pub answers: Answers,
    /// What the template doesn't do for these answers.
    pub limits: Vec<String>,
}

impl Plan {
    /// Its files' paths, `/`-separated, in order.
    pub fn paths(&self) -> Vec<String> {
        self.files.keys().map(|p| shown(p)).collect()
    }
}

/// A path as the plan names it, `/`-separated.
fn shown(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Why a template makes no game for a set of answers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ComposeError {
    /// An answer the template doesn't take.
    Answer {
        /// What is wrong.
        message: String,
    },
    /// A part's folder the template can't take files from.
    Part {
        /// The part.
        part: String,
        /// What is wrong.
        message: String,
    },
    /// A part picked needs another that isn't.
    Requires {
        /// The part.
        part: String,
        /// The part it needs.
        requires: String,
        /// Why the first is picked, in words.
        because: String,
    },
    /// Two parts picked can't be together.
    Conflict {
        /// The two parts.
        parts: [String; 2],
        /// Which answers picked each, in words.
        because: String,
    },
    /// Two parts write one file that isn't merged.
    SameFile {
        /// The file, from the game's folder.
        path: String,
        /// The parts that write it.
        parts: Vec<String>,
    },
    /// Two parts write one merged file, with values that differ.
    Merge {
        /// The file, from the game's folder.
        path: String,
        /// The parts that write it.
        parts: Vec<String>,
        /// Where in it (`workspace.dependencies.serde`), or `""`.
        at: String,
        /// What differs.
        message: String,
    },
    /// A placeholder no game fills.
    Placeholder {
        /// The file, from the template's folder or the game's.
        file: String,
        /// Its line, from 1.
        line: usize,
        /// The placeholder (`{{nope}}`).
        placeholder: String,
    },
    /// A module a part turns on that the engine doesn't build.
    Module {
        /// The part.
        part: String,
        /// The module, as the part names it.
        module: String,
        /// Why it is none.
        message: String,
    },
    /// A stage the flow can't take.
    Stage {
        /// The parts concerned.
        parts: Vec<String>,
        /// What is wrong.
        message: String,
    },
}

impl fmt::Display for ComposeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let names = |parts: &[String]| {
            let named: Vec<String> = parts.iter().map(|p| format!("`{p}`")).collect();
            named.join(" and ")
        };
        match self {
            ComposeError::Answer { message } => write!(f, "{message}"),
            ComposeError::Part { part, message } => write!(f, "part `{part}`: {message}"),
            ComposeError::Requires {
                part,
                requires,
                because,
            } => write!(
                f,
                "part `{part}` (picked because {because}) needs part `{requires}`, which no \
                 answer picks"
            ),
            ComposeError::Conflict { parts, because } => write!(
                f,
                "parts `{}` and `{}` can't be in one game: {because}",
                parts[0], parts[1]
            ),
            ComposeError::SameFile { path, parts } => write!(
                f,
                "{path} is written by the parts {}, and only a Cargo.toml or a .ron file is \
                 merged",
                names(parts)
            ),
            ComposeError::Merge {
                path,
                parts,
                at,
                message,
            } => {
                let at = if at.is_empty() {
                    String::new()
                } else {
                    format!(" at `{at}`")
                };
                write!(
                    f,
                    "{path}, written by the parts {}, doesn't merge{at}: {message}",
                    names(parts)
                )
            }
            ComposeError::Placeholder {
                file,
                line,
                placeholder,
            } => write!(
                f,
                "the template's {file}:{line} has an unknown placeholder {placeholder}"
            ),
            ComposeError::Module {
                part,
                module,
                message,
            } => write!(f, "part `{part}` turns on `{module}`: {message}"),
            ComposeError::Stage { parts, message } => {
                write!(f, "the stages of {}: {message}", names(parts))
            }
        }
    }
}

impl std::error::Error for ComposeError {}

/// `text`, the file `file`, with its placeholders filled: `{{name}}`,
/// `{{crate_name}}`, `"{{engine.<crate>}}"` (with its quotes) and
/// `{{answer.<question>}}` (the question's answers in `answers`, joined
/// by `,`). A GitHub Actions expression, `${{ … }}`, passes through as it
/// is. Any other `{{` is [`ComposeError::Placeholder`].
pub fn fill(
    file: &str,
    text: &str,
    with: &Fill,
    answers: &Answers,
) -> Result<String, ComposeError> {
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while let Some(found) = text[at..].find("{{") {
        let open = at + found;
        let before = &text[at..open];
        let unknown = |shown: &str| ComposeError::Placeholder {
            file: file.to_owned(),
            line: text[..open].matches('\n').count() + 1,
            placeholder: shown.to_owned(),
        };
        let Some(length) = text[open..].find("}}") else {
            return Err(unknown(&text[open..]));
        };
        let close = open + length + 2;
        let token = &text[open..close];
        let inner = &text[open + 2..close - 2];
        out.push_str(before);
        at = close;
        if before.ends_with('$') {
            out.push_str(token);
            continue;
        }
        if let Some(krate) = inner.strip_prefix("engine.")
            && let Some(dependency) = with.engine.get(krate)
            && out.ends_with('"')
            && text[close..].starts_with('"')
        {
            out.pop();
            out.push_str(dependency);
            at += 1;
            continue;
        }
        let value = match inner {
            "name" => with.name.clone(),
            "crate_name" => with.crate_name.clone(),
            _ => match inner
                .strip_prefix("answer.")
                .and_then(|question| answers.0.get(question))
            {
                Some(picked) => picked.join(","),
                None => return Err(unknown(token)),
            },
        };
        out.push_str(&value);
    }
    out.push_str(&text[at..]);
    Ok(out)
}

/// The module of stage `stage`: its id with `_` for `-`.
fn module_of(stage: &str) -> String {
    stage.replace('-', "_")
}

/// The flow's list of stages ([`GENERATED_STAGES`]) for `stages`, in
/// order, each a module `stages/<id with _>.rs` of the game crate that
/// gives its `Own` part of the game's data and its `own` reader, its
/// `first` screen, its `screens` and its `Tools`. With none, the title is
/// the game's one screen and its tools are `BasicTools`.
pub fn stages_rs(stages: &[String]) -> String {
    let head = "//! The game's stages, in the order its template's parts add them. `shuyi\n\
                //! new` wrote this file from the parts it was made with: the title\n\
                //! ([`crate::title`]) starts the first, and a stage's own part of the\n\
                //! game's data is [`Own`].\n\n";
    let mut out = head.to_owned();
    match stages {
        [] => out.push_str(
            "use shuyi::prelude::*;\n\n\
             use crate::Data;\n\n\
             /// The stages' ids, in order: none yet.\n\
             pub fn names() -> &'static [&'static str] {\n    &[]\n}\n\n\
             /// The stages' own part of the game's data: none.\n\
             pub type Own = ();\n\n\
             /// Reads the stages' own part from the game's files.\n\
             pub fn own(_: &Bundle) -> Result<Own, Vec<ContentError>> {\n    Ok(())\n}\n\n\
             /// The first stage, which the title starts: none.\n\
             pub fn first(_: &mut Ctx<Data>) -> Option<Box<dyn Screen<Data>>> {\n    None\n}\n\n\
             /// Every screen the stages can show, for the check that the game's\n\
             /// controls fit each target: none.\n\
             pub fn screens(_: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {\n    Vec::new()\n}\n\n\
             /// The game's tools: the engine's, for a game on the short path.\n\
             #[cfg(feature = \"tools\")]\n\
             pub fn tools() -> shuyi_cli::BasicTools<UiAction, Own> {\n    \
             shuyi_cli::BasicTools::new(crate::game())\n}\n",
        ),
        [stage, ..] => {
            let module = module_of(stage);
            // Writing to a `String` can't fail.
            for stage in stages {
                let _ = writeln!(out, "pub mod {};", module_of(stage));
            }
            let names: Vec<String> = stages.iter().map(|s| format!("\"{s}\"")).collect();
            let _ = write!(
                out,
                "\nuse shuyi::prelude::*;\n\n\
                 use crate::Data;\n\n\
                 /// The stages' ids, in order.\n\
                 pub fn names() -> &'static [&'static str] {{\n    &[{names}]\n}}\n\n\
                 /// The stages' own part of the game's data.\n\
                 pub type Own = {module}::Own;\n\n\
                 /// Reads the stages' own part from the game's files.\n\
                 pub fn own(bundle: &Bundle) -> Result<Own, Vec<ContentError>> {{\n    \
                 {module}::own(bundle)\n}}\n\n\
                 /// The first stage, which the title starts.\n\
                 pub fn first(ctx: &mut Ctx<Data>) -> Option<Box<dyn Screen<Data>>> {{\n    \
                 Some({module}::first(ctx))\n}}\n\n\
                 /// Every screen the stages can show, for the check that the game's\n\
                 /// controls fit each target.\n\
                 pub fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {{\n    \
                 {module}::screens(ctx)\n}}\n\n\
                 /// The game's tools: its stage's, which give the engine's for the rest.\n\
                 #[cfg(feature = \"tools\")]\n\
                 pub fn tools() -> {module}::Tools {{\n    \
                 {module}::Tools::new(crate::game())\n}}\n",
                names = names.join(", "),
            );
        }
    }
    out
}

/// What a file merges as, by its path; `None` if it doesn't.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Merged {
    Toml,
    Ron,
}

impl Merged {
    fn of(path: &str) -> Option<Self> {
        if path == "Cargo.toml" || path.ends_with("/Cargo.toml") {
            Some(Merged::Toml)
        } else if Path::new(path)
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("ron"))
        {
            Some(Merged::Ron)
        } else {
            None
        }
    }
}

/// The one file that `writers` (each a part and its bytes) write at
/// `path`: the only one's, or theirs merged.
fn merged(path: &str, writers: Vec<(String, Vec<u8>)>) -> Result<Vec<u8>, ComposeError> {
    let parts: Vec<String> = writers.iter().map(|(part, _)| part.clone()).collect();
    if writers.len() == 1 {
        return Ok(writers
            .into_iter()
            .next()
            .map(|(_, b)| b)
            .unwrap_or_default());
    }
    let Some(kind) = Merged::of(path) else {
        return Err(ComposeError::SameFile {
            path: path.to_owned(),
            parts,
        });
    };
    let failed = |at: String, message: String| ComposeError::Merge {
        path: path.to_owned(),
        parts: parts.clone(),
        at,
        message,
    };
    let mut texts = Vec::new();
    for (part, bytes) in &writers {
        let text = std::str::from_utf8(bytes)
            .map_err(|e| failed(String::new(), format!("part `{part}`'s is not UTF-8: {e}")))?;
        texts.push(text);
    }
    let mut out = texts[0].to_owned();
    for text in &texts[1..] {
        out = match kind {
            Merged::Toml => merge_toml(&out, text),
            Merged::Ron => ron_merge::merge(&out, text),
        }
        .map_err(|(at, message)| failed(at, message))?;
    }
    Ok(out.into_bytes())
}

/// The TOML `base` with `other`'s keys added, each with its comments;
/// tables merged key by key, arrays item by item. A key both have with
/// different values fails, with its path and both values.
pub(super) fn merge_toml(base: &str, other: &str) -> Result<String, (String, String)> {
    let mut doc: DocumentMut = base.parse().map_err(|e| (String::new(), format!("{e}")))?;
    let other: DocumentMut = other.parse().map_err(|e| (String::new(), format!("{e}")))?;
    merge_tables(doc.as_table_mut(), other.as_table(), "")?;
    Ok(doc.to_string())
}

/// `from`'s keys merged into `into` ([`merge_toml`]); `at` is their path.
fn merge_tables(into: &mut Table, from: &Table, at: &str) -> Result<(), (String, String)> {
    for (key, item) in from {
        let here = if at.is_empty() {
            key.to_owned()
        } else {
            format!("{at}.{key}")
        };
        if let Some(existing) = into.get_mut(key) {
            merge_items(existing, item, &here)?;
        } else {
            let formatted = from.key(key).cloned().unwrap_or_else(|| key.into());
            into.insert_formatted(&formatted, item.clone());
        }
    }
    Ok(())
}

/// A value with no comments or spaces round it, to compare.
fn bare(value: &Value) -> String {
    let mut value = value.clone();
    value.decor_mut().clear();
    value.to_string()
}

/// `from` merged into `into`, both at `at`.
fn merge_items(into: &mut Item, from: &Item, at: &str) -> Result<(), (String, String)> {
    match (into, from) {
        (Item::Table(into), Item::Table(from)) => merge_tables(into, from, at),
        (Item::Value(Value::Array(into)), Item::Value(Value::Array(from))) => {
            for value in from {
                if !into.iter().any(|v| bare(v) == bare(value)) {
                    into.push_formatted(value.clone());
                }
            }
            Ok(())
        }
        (Item::ArrayOfTables(into), Item::ArrayOfTables(from)) => {
            for table in from {
                if !into.iter().any(|t| t.to_string() == table.to_string()) {
                    into.push(table.clone());
                }
            }
            Ok(())
        }
        (into, from) if into.is_table_like() && from.is_table_like() => {
            let (Some(into), Some(from)) = (into.as_table_like_mut(), from.as_table_like()) else {
                return Ok(());
            };
            for (key, item) in from.iter() {
                let here = format!("{at}.{key}");
                match into.get_mut(key) {
                    Some(existing) => merge_items(existing, item, &here)?,
                    None => {
                        into.insert(key, item.clone());
                    }
                }
            }
            Ok(())
        }
        (into, from) => {
            let shown = |item: &Item| match item.as_value() {
                Some(value) => bare(value),
                None => item.to_string().trim().to_owned(),
            };
            let (a, b) = (shown(into), shown(from));
            if a == b {
                Ok(())
            } else {
                Err((at.to_owned(), format!("`{a}` in one, `{b}` in another")))
            }
        }
    }
}

/// The parts of `template` picked by `answers`, checked against each
/// other's `requires` and `conflicts`.
fn picked<'a>(template: &'a Template, answers: &Answers) -> Result<Vec<&'a Part>, ComposeError> {
    let parts: Vec<&Part> = template
        .parts
        .iter()
        .filter(|part| part.when.holds(answers))
        .collect();
    for part in &parts {
        if let Some(missing) = part
            .requires
            .iter()
            .find(|id| !parts.iter().any(|p| &p.id == *id))
        {
            return Err(ComposeError::Requires {
                part: part.id.clone(),
                requires: missing.clone(),
                because: part.when.because(answers),
            });
        }
        if let Some(other) = parts.iter().find(|p| part.conflicts.contains(&p.id)) {
            return Err(ComposeError::Conflict {
                parts: [part.id.clone(), other.id.clone()],
                because: format!(
                    "`{}` is picked because {}, and `{}` because {}",
                    part.id,
                    part.when.because(answers),
                    other.id,
                    other.when.because(answers)
                ),
            });
        }
    }
    Ok(parts)
}

/// Every file the parts write, by its path from the game's folder, with
/// each part that writes it and its bytes, the placeholders filled.
fn written(
    template: &Template,
    parts: &[&Part],
    with: &Fill,
    answers: &Answers,
) -> Result<Written, ComposeError> {
    let mut written = Written::new();
    for part in parts {
        let folder = folder_of(&part.files).map_err(|message| ComposeError::Part {
            part: part.id.clone(),
            message,
        })?;
        let files = template
            .files
            .range(folder.clone()..)
            .take_while(|(path, _)| path.starts_with(&folder));
        for (path, bytes) in files {
            let source = format!("{}{}", folder, &path[folder.len()..]);
            let to = fill(&source, &path[folder.len()..], with, answers)?;
            let bytes = match std::str::from_utf8(bytes) {
                Ok(text) => fill(&source, text, with, answers)?.into_bytes(),
                Err(_) => bytes.clone(),
            };
            written
                .entry(to)
                .or_default()
                .push((part.id.clone(), bytes));
        }
    }
    Ok(written)
}

/// The stages the parts add, with the part of each, checked for the flow;
/// and the list of them written, if the template has a part with a stage.
fn stages(
    template: &Template,
    parts: &[&Part],
    written: &mut Written,
) -> Result<Vec<String>, ComposeError> {
    let with_part: Vec<(&str, &str)> = parts
        .iter()
        .filter_map(|p| p.stage.as_deref().map(|stage| (p.id.as_str(), stage)))
        .collect();
    if !template.parts.iter().any(|p| p.stage.is_some()) {
        return Ok(Vec::new());
    }
    if let Some(writers) = written.get(GENERATED_STAGES) {
        return Err(ComposeError::Stage {
            parts: writers.iter().map(|(p, _)| p.clone()).collect(),
            message: format!(
                "{GENERATED_STAGES} is written by {COMPOSER} from the parts' stages, not by a part"
            ),
        });
    }
    if with_part.len() > 1 {
        return Err(ComposeError::Stage {
            parts: with_part.iter().map(|(p, _)| (*p).to_owned()).collect(),
            message: "each adds a stage, and a game has one stage for now: pick one".to_owned(),
        });
    }
    for (part, stage) in &with_part {
        let module = format!("crates/game/src/stages/{}.rs", module_of(stage));
        if !written.contains_key(&module) {
            return Err(ComposeError::Stage {
                parts: vec![(*part).to_owned()],
                message: format!("the part adds the stage `{stage}` but writes no {module}"),
            });
        }
    }
    let stages: Vec<String> = with_part.iter().map(|(_, s)| (*s).to_owned()).collect();
    written.insert(
        GENERATED_STAGES.to_owned(),
        vec![(COMPOSER.to_owned(), stages_rs(&stages).into_bytes())],
    );
    Ok(stages)
}

/// The game `template` makes for `answers` (each question they leave out
/// at its default), with the engine's modules in `catalog` and the
/// placeholders filled `with`: its files and their bytes, and what its
/// parts turn on. The same input always gives the same plan.
pub fn compose(
    template: &Template,
    answers: &Answers,
    catalog: &Catalog,
    with: &Fill,
) -> Result<Plan, ComposeError> {
    template
        .check_answers(answers)
        .map_err(|message| ComposeError::Answer { message })?;
    let answers = template.with_defaults(answers);
    let parts = picked(template, &answers)?;
    let mut written = written(template, &parts, with, &answers)?;
    let stages = stages(template, &parts, &mut written)?;

    let mut files = BTreeMap::new();
    for (path, writers) in written {
        let bytes = merged(&path, writers)?;
        files.insert(PathBuf::from(path), bytes);
    }
    let skills = with_skills(catalog, &mut files);

    let mut modules = Vec::new();
    let mut features = Vec::new();
    for part in &parts {
        for name in &part.modules {
            let module = catalog.module(name).ok_or_else(|| ComposeError::Module {
                part: part.id.clone(),
                module: name.clone(),
                message: crate::contract::add::unknown(catalog, name),
            })?;
            if !modules.contains(&module.name) {
                modules.push(module.name.clone());
                features.push((part.id.clone(), module.feature().to_owned()));
            }
        }
    }
    let root = PathBuf::from("Cargo.toml");
    for (part, feature) in &features {
        let failed = |message: String| ComposeError::Module {
            part: part.clone(),
            module: catalog::module_of(feature),
            message,
        };
        let manifest = files
            .get(&root)
            .ok_or_else(|| failed("the game has no Cargo.toml to turn it on in".to_owned()))?;
        let text = String::from_utf8_lossy(manifest).into_owned();
        if let Some(edited) = catalog::turn_on(&text, feature).map_err(failed)? {
            files.insert(root.clone(), edited.into_bytes());
        }
    }

    let mut targets: Vec<String> = Vec::new();
    for target in parts.iter().flat_map(|p| &p.targets) {
        if !targets.contains(target) {
            targets.push(target.clone());
        }
    }
    let mut limits = Vec::new();
    for (question, picked) in &answers.0 {
        for answer in picked {
            let served = template.parts.iter().any(|p| {
                p.when
                    .picks()
                    .contains(&(question.as_str(), answer.as_str()))
            });
            if !served {
                limits.push(format!(
                    "no part of the template for `{question}` = `{answer}`: nothing added"
                ));
            }
        }
    }
    Ok(Plan {
        files,
        parts: parts.iter().map(|p| p.id.clone()).collect(),
        modules,
        targets,
        stages,
        skills,
        answers,
        limits,
    })
}

/// The skills of `catalog`'s pack added to `files` under `.claude/skills/`,
/// each with every file of its folder, and their names: all but a module's
/// skill, and a skill whose folder a part already writes.
fn with_skills(catalog: &Catalog, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> Vec<String> {
    let mut added = Vec::new();
    for skill in &catalog.skills.skills {
        let folder = format!("{}/{}/", catalog::SKILLS, skill.name);
        let written = files.keys().any(|p| shown(p).starts_with(&folder));
        if catalog.is_module(&skill.name) || written {
            continue;
        }
        for (path, bytes) in &skill.files {
            files.insert(PathBuf::from(format!("{folder}{path}")), bytes.clone());
        }
        added.push(skill.name.clone());
    }
    added
}

/// What [`apply`] wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Applied {
    /// Every file written, from the folder, `/`-separated, in order.
    pub written: Vec<String>,
    /// Those that were there already, and were written over.
    pub overwritten: Vec<String>,
}

/// Writes `plan`'s files into `dir`, made if missing. A folder that isn't
/// empty is refused unless `force`, with the files it would write over;
/// with `force`, [`Applied::overwritten`] lists them.
pub fn apply(plan: &Plan, dir: &Path, force: bool) -> Result<Applied, String> {
    let overwritten: Vec<String> = plan
        .files
        .keys()
        .filter(|path| dir.join(path).exists())
        .map(|p| shown(p))
        .collect();
    let empty = fs::read_dir(dir).map_or(true, |mut entries| entries.next().is_none());
    if !empty && !force {
        let over = if overwritten.is_empty() {
            String::new()
        } else {
            format!(" (it would write over {})", overwritten.join(", "))
        };
        return Err(format!("is not empty{over}: pass --force to write into it"));
    }
    for (path, bytes) in &plan.files {
        let to = dir.join(path);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(&to, bytes).map_err(|e| format!("{}: {e}", to.display()))?;
    }
    Ok(Applied {
        written: plan.paths(),
        overwritten,
    })
}
