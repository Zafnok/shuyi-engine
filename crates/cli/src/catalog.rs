//! The engine's modules as the tools read them (ADR-0009 §2, §4, §6),
//! for `modules` and `add` ([`crate::contract::modules`],
//! [`crate::contract::add`]):
//!
//! - [`Concerns`]: the concerns a game answers and their answers, the
//!   engine's `docs/modules.ron`;
//! - [`ModuleInfo`]: what a module's crate says it answers, its
//!   `[package.metadata.shuyi]`, read with its crate into a
//!   [`ModuleCrate`];
//! - [`Catalog`]: the two read from an engine checkout, with its skill
//!   pack, and [`engine_root`], which finds the checkout a game builds with;
//! - a game's `Cargo.toml`: the `shuyi` features it has on
//!   ([`features_on`]), and one more turned on ([`turn_on`]);
//! - a module's package folder: the first paragraph of its README
//!   ([`readme_paragraph`]) and its skill ([`skill_of`], [`copy_skill`]).
//!
//! A module's feature of the `shuyi` crate is its crate's name without
//! `shuyi-` ([`feature_of`]).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use toml_edit::{Array, DocumentMut, InlineTable, Item, TableLike, Value};

use crate::skills::{self, Pack};

/// The concerns, from the engine's root.
pub const CONCERNS: &str = "docs/modules.ron";

/// Where a game keeps its agent's skills, from its folder.
pub const SKILLS: &str = ".claude/skills";

/// What the name of every engine crate starts with but the front door's.
const PREFIX: &str = "shuyi-";

/// The crates of the default engine, which are no modules.
pub const DEFAULT_CRATES: [&str; 7] = [
    "shuyi-core",
    "shuyi-assets",
    "shuyi-ui",
    "shuyi-platform",
    "shuyi-app",
    "shuyi-cli",
    "shuyi-alloc-count",
];

/// The concerns a game answers, `docs/modules.ron`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Concerns {
    /// Each concern, in the catalog's order.
    pub concerns: Vec<Concern>,
}

/// A question every game answers one way or another.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Concern {
    /// What a module's metadata names (`bodies`).
    pub id: String,
    /// How the catalog says it (`Bodies and motion`).
    pub name: String,
    /// Whether its answers are alternatives, never driving the same
    /// thing in one stage, so that two modules answering it name each
    /// other; `false` when they can share a stage.
    pub alternatives: bool,
    /// Its answers.
    pub answers: Vec<Choice>,
}

/// One answer to a concern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    /// What a module's metadata names (`rigid-body`).
    pub id: String,
    /// How the catalog says it (`rigid-body physics`).
    pub name: String,
    /// Who answers it: `default` (a kit of the default engine) or a
    /// module's crate (`shuyi-physics`).
    pub by: String,
    /// Whether it is built.
    pub status: Status,
    /// The tickets that build it.
    pub tickets: Vec<String>,
}

/// Whether an answer or a module is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Built, with an example passing CI.
    Ready,
    /// Ticketed, not built yet.
    Planned,
    /// Not started until the plan says.
    Parked,
}

impl Status {
    /// Its name (`parked`).
    pub fn name(self) -> &'static str {
        match self {
            Status::Ready => "ready",
            Status::Planned => "planned",
            Status::Parked => "parked",
        }
    }
}

impl Concerns {
    /// `docs/modules.ron`'s text, read.
    pub fn parse(text: &str) -> Result<Self, String> {
        ron::from_str(text).map_err(|e| format!("{CONCERNS}: {e}"))
    }

    /// The concern `concern`'s answer `answer`, if both are there.
    pub fn choice(&self, concern: &str, answer: &str) -> Option<&Choice> {
        self.concerns
            .iter()
            .find(|c| c.id == concern)?
            .answers
            .iter()
            .find(|a| a.id == answer)
    }

    /// Every answer `by` gives, with its concern.
    pub fn answered_by<'a>(
        &'a self,
        by: &'a str,
    ) -> impl Iterator<Item = (&'a Concern, &'a Choice)> {
        self.concerns.iter().flat_map(move |concern| {
            concern
                .answers
                .iter()
                .filter(move |answer| answer.by == by)
                .map(move |answer| (concern, answer))
        })
    }
}

/// A module's `[package.metadata.shuyi]` (ADR-0009 §4): what it answers,
/// what it is built for, and what it sits beside.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModuleInfo {
    /// The concerns it answers, and how: one module may answer several
    /// (`[{ concern = "bodies", answer = "rigid-body" }]`).
    pub answers: Vec<Answered>,
    /// The directions it is built for (`side-view`).
    pub serves: Vec<String>,
    /// The modules that answer one of its concerns another way, by crate.
    #[serde(default)]
    pub alternatives: Vec<String>,
    /// The modules it has been tested beside in one stage, by crate.
    #[serde(default)]
    pub works_with: Vec<String>,
    /// The function a game's `formats()` calls to add the module's
    /// formats, if it has any (`shuyi_crawl::formats`).
    #[serde(default)]
    pub formats: Option<String>,
}

/// One concern a module answers, and its answer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Answered {
    /// The concern's id, in `docs/modules.ron`.
    pub concern: String,
    /// The answer's id under it.
    pub answer: String,
}

/// A module's crate: its name, its package folder and its metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleCrate {
    /// Its package's name (`shuyi-physics`).
    pub name: String,
    /// The folder of its `Cargo.toml`.
    pub dir: PathBuf,
    /// Its metadata.
    pub info: ModuleInfo,
}

/// The parts of a `Cargo.toml` a module is read from.
#[derive(Deserialize)]
struct Manifest {
    package: Option<ManifestPackage>,
}

#[derive(Deserialize)]
struct ManifestPackage {
    name: String,
    #[serde(default)]
    metadata: Option<ManifestMetadata>,
}

#[derive(Deserialize)]
struct ManifestMetadata {
    #[serde(default)]
    shuyi: Option<ModuleInfo>,
}

impl ModuleCrate {
    /// The module whose package folder is `dir`, or `None` for a crate
    /// with no `[package.metadata.shuyi]`.
    pub fn read(dir: &Path) -> Result<Option<Self>, String> {
        let path = dir.join("Cargo.toml");
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let manifest: Manifest =
            toml_edit::de::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        let Some(package) = manifest.package else {
            return Ok(None);
        };
        Ok(package
            .metadata
            .and_then(|metadata| metadata.shuyi)
            .map(|info| Self {
                name: package.name,
                dir: dir.to_path_buf(),
                info,
            }))
    }

    /// Its feature of the `shuyi` crate.
    pub fn feature(&self) -> &str {
        feature_of(&self.name)
    }
}

/// The feature of the `shuyi` crate that turns on module `name`: its
/// name without `shuyi-` (`physics`).
pub fn feature_of(name: &str) -> &str {
    name.strip_prefix(PREFIX).unwrap_or(name)
}

/// The module that feature `feature` of the `shuyi` crate turns on.
pub fn module_of(feature: &str) -> String {
    format!("{PREFIX}{feature}")
}

/// What the tools know of an engine checkout: its concerns, the modules
/// it builds and its skill pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    /// The checkout's folder (the one holding `crates/`).
    pub root: PathBuf,
    /// Its `docs/modules.ron`.
    pub concerns: Concerns,
    /// Its modules: every crate under `crates/` with
    /// `[package.metadata.shuyi]`, by name.
    pub modules: Vec<ModuleCrate>,
    /// Its skill pack, `skills/` (empty in a checkout without one).
    pub skills: Pack,
}

impl Catalog {
    /// The catalog of the engine checkout at `root`.
    pub fn read(root: &Path) -> Result<Self, String> {
        let path = root.join(CONCERNS);
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let concerns = Concerns::parse(&text)?;
        let crates = root.join("crates");
        let entries = fs::read_dir(&crates).map_err(|e| format!("{}: {e}", crates.display()))?;
        let mut modules = Vec::new();
        for entry in entries.flatten() {
            let dir = entry.path();
            if dir.join("Cargo.toml").is_file()
                && let Some(module) = ModuleCrate::read(&dir)?
                && !DEFAULT_CRATES.contains(&module.name.as_str())
            {
                modules.push(module);
            }
        }
        modules.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self {
            root: root.to_path_buf(),
            concerns,
            modules,
            skills: Pack::read(&root.join(skills::PACK))?,
        })
    }

    /// The catalog of the engine this `shuyi` was built from, as it embeds
    /// it, for a game made with no checkout: its concerns, its modules by
    /// the `shuyi` crate's features, one a module, with no metadata, and its
    /// skill pack.
    pub fn embedded() -> Result<Self, String> {
        let concerns = Concerns::parse(include_str!("../../../docs/modules.ron"))?;
        Ok(Self {
            root: PathBuf::new(),
            concerns,
            modules: modules_of_features(include_str!("../../shuyi/Cargo.toml"))?,
            skills: Pack::embedded(),
        })
    }

    /// The module named `name`, with or without `shuyi-`.
    pub fn module(&self, name: &str) -> Option<&ModuleCrate> {
        self.modules
            .iter()
            .find(|m| m.name == name || feature_of(&m.name) == name)
    }

    /// Whether `name` is a module's crate: one the engine builds, or one a
    /// concern names as an answer's (`shuyi-net`), built or not. A
    /// module's skill is named after its crate, and a game gets it with
    /// `shuyi add`, not `shuyi new`.
    pub fn is_module(&self, name: &str) -> bool {
        name != "default"
            && (self.modules.iter().any(|m| m.name == name)
                || self.concerns.answered_by(name).next().is_some())
    }
}

/// The modules the `shuyi` crate's `Cargo.toml` (`manifest`) has a
/// feature for, one a feature but `default`, by name, with no metadata.
pub fn modules_of_features(manifest: &str) -> Result<Vec<ModuleCrate>, String> {
    let doc: DocumentMut = manifest
        .parse()
        .map_err(|e| format!("crates/shuyi/Cargo.toml: {e}"))?;
    let mut modules: Vec<ModuleCrate> = doc
        .get("features")
        .and_then(Item::as_table_like)
        .map(|features| {
            features
                .iter()
                .map(|(feature, _)| feature)
                .filter(|feature| *feature != "default")
                .map(|feature| ModuleCrate {
                    name: module_of(feature),
                    dir: PathBuf::from("crates").join(feature),
                    info: ModuleInfo::default(),
                })
                .collect()
        })
        .unwrap_or_default();
    modules.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(modules)
}

/// The entry of the `shuyi` crate in a `Cargo.toml`: under
/// `[workspace.dependencies]` (a game made by `shuyi new`), or else under
/// `[dependencies]`.
fn shuyi_entry(doc: &mut DocumentMut) -> Option<&mut Item> {
    // Looked at first: `Item::get_mut` adds a key that is missing.
    let in_workspace = doc
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.get("shuyi"))
        .is_some();
    let in_dependencies = doc
        .get("dependencies")
        .and_then(|d| d.get("shuyi"))
        .is_some();
    if in_workspace {
        doc.get_mut("workspace")?
            .get_mut("dependencies")?
            .get_mut("shuyi")
    } else if in_dependencies {
        doc.get_mut("dependencies")?.get_mut("shuyi")
    } else {
        None
    }
}

/// The text of a `Cargo.toml` that has no `shuyi` entry, as an error.
fn no_entry() -> String {
    "the game's Cargo.toml has no `shuyi` dependency (under [workspace.dependencies] or \
     [dependencies]): a game turns modules on through the engine's `shuyi` crate"
        .to_owned()
}

/// The features listed in an entry's table, if it is a table.
fn features_in(entry: &Item) -> Vec<String> {
    entry
        .as_table_like()
        .and_then(|table| table.get("features"))
        .and_then(Item::as_array)
        .map(|features| {
            features
                .iter()
                .filter_map(|f| f.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

/// The features of the `shuyi` crate a `Cargo.toml`'s text turns on.
pub fn features_on(manifest: &str) -> Result<Vec<String>, String> {
    let mut doc: DocumentMut = manifest.parse().map_err(|e| format!("Cargo.toml: {e}"))?;
    let entry = shuyi_entry(&mut doc).ok_or_else(no_entry)?;
    Ok(features_in(entry))
}

/// A `Cargo.toml`'s text with feature `feature` of the `shuyi` crate
/// turned on, its layout kept; `None` if it is already on.
pub fn turn_on(manifest: &str, feature: &str) -> Result<Option<String>, String> {
    let mut doc: DocumentMut = manifest.parse().map_err(|e| format!("Cargo.toml: {e}"))?;
    let entry = shuyi_entry(&mut doc).ok_or_else(no_entry)?;
    if features_in(entry).iter().any(|f| f == feature) {
        return Ok(None);
    }
    // A bare version becomes a table that keeps it.
    if let Some(version) = entry.as_str() {
        let mut table = InlineTable::new();
        table.insert("version", Value::from(version));
        *entry = Item::Value(Value::InlineTable(table));
    }
    let table: &mut dyn TableLike = entry
        .as_table_like_mut()
        .ok_or("the game's `shuyi` dependency is neither a version nor a table")?;
    if let Some(features) = table.get_mut("features").and_then(Item::as_array_mut) {
        features.push(feature);
    } else {
        let mut features = Array::new();
        features.push(feature);
        table.insert("features", Item::Value(Value::Array(features)));
    }
    // An inline table laid out again, so the new key sits as the others.
    if let Some(table) = entry.as_inline_table_mut() {
        table.fmt();
    }
    Ok(Some(doc.to_string()))
}

/// The engine checkout the game at `game` builds with: the folder above
/// the `shuyi` crate its `Cargo.toml` names by path, or else the one
/// cargo resolved it to (`cargo metadata`, run through the `CARGO`
/// variable).
pub fn engine_root(game: &Path) -> Result<PathBuf, String> {
    let path = game.join("Cargo.toml");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut doc: DocumentMut = text.parse().map_err(|e| format!("Cargo.toml: {e}"))?;
    let entry = shuyi_entry(&mut doc).ok_or_else(no_entry)?;
    let by_path = entry
        .as_table_like()
        .and_then(|table| table.get("path"))
        .and_then(Item::as_str)
        .map(|dir| game.join(dir));
    let shuyi = match by_path {
        Some(dir) => dir,
        None => resolved_shuyi(game)?,
    };
    shuyi
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| format!("{}: no engine checkout above it", shuyi.display()))
}

/// The folder cargo resolved the game's `shuyi` crate to.
fn resolved_shuyi(game: &Path) -> Result<PathBuf, String> {
    #[derive(Deserialize)]
    struct Metadata {
        packages: Vec<Package>,
    }
    #[derive(Deserialize)]
    struct Package {
        name: String,
        manifest_path: PathBuf,
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1"])
        .current_dir(game)
        .output()
        .map_err(|e| format!("cargo metadata does not run: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: Metadata = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("cargo metadata's output: {e}"))?;
    metadata
        .packages
        .into_iter()
        .find(|p| p.name == "shuyi")
        .and_then(|p| p.manifest_path.parent().map(Path::to_path_buf))
        .ok_or_else(|| "cargo metadata names no `shuyi` package for the game".to_owned())
}

/// The first paragraph of the README in package folder `dir`, after its
/// title: what the module is for (ADR-0009 §4).
pub fn readme_paragraph(dir: &Path) -> Result<String, String> {
    let path = dir.join("README.md");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim_end)
        .skip_while(|line| line.is_empty() || line.starts_with('#'))
        .take_while(|line| !line.is_empty())
        .collect();
    if lines.is_empty() {
        return Err(format!("{}: no paragraph after its title", path.display()));
    }
    Ok(lines.join("\n"))
}

/// The skill of the module `name` in package folder `dir`: its own
/// `skill/` folder, or the one named for it in the skill pack of the
/// repository it is in (`skills/<name>`, two folders up), whichever holds
/// a `SKILL.md`.
pub fn skill_of(dir: &Path, name: &str) -> Option<PathBuf> {
    let own = dir.join("skill");
    let pack = dir.join("../../skills").join(name);
    [own, pack]
        .into_iter()
        .find(|skill| skill.join("SKILL.md").is_file())
}

/// What [`copy_skill`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillCopy {
    /// It copied the skill to this folder.
    Copied(PathBuf),
    /// The game already has a skill of that name, in this folder: it is
    /// left as it is.
    AlreadyThere(PathBuf),
    /// The game keeps no skills ([`SKILLS`] isn't there).
    NoSkills,
}

/// Copies the skill folder `skill` into the skills of the game at `game`
/// as `name`: its regular files, at any depth (a link is skipped).
pub fn copy_skill(skill: &Path, game: &Path, name: &str) -> Result<SkillCopy, String> {
    let skills = game.join(SKILLS);
    if !skills.is_dir() {
        return Ok(SkillCopy::NoSkills);
    }
    let to = skills.join(name);
    if to.exists() {
        return Ok(SkillCopy::AlreadyThere(to));
    }
    copy_files(skill, &to)?;
    Ok(SkillCopy::Copied(to))
}

/// Copies the regular files under `from` to `to`, folders made as needed.
fn copy_files(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let entries = fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", from.display()))?;
        let kind = entry
            .file_type()
            .map_err(|e| format!("{}: {e}", entry.path().display()))?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_files(&entry.path(), &target)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &target).map_err(|e| format!("{}: {e}", target.display()))?;
        }
    }
    Ok(())
}

/// Whether a Rust file of the game at `game` (outside `target/` and
/// hidden folders) names `shuyi::enabled_modules`, which installs the
/// modules its features turn on.
pub fn installs_modules(game: &Path) -> bool {
    let mut pending = vec![game.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if path.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    pending.push(path);
                }
            } else if name.ends_with(".rs")
                && fs::read_to_string(&path).is_ok_and(|text| text.contains("enabled_modules"))
            {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests;
