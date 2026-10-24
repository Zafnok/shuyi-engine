//! `cargo xtask check-modules`: the engine's modules (ADR-0009 §4, §6)
//! against the catalog, `docs/modules.md`, and the concerns,
//! `docs/modules.ron`, over the graph `check-layers` reads from `cargo
//! metadata --all-features` ([`crate::layers`]).
//!
//! A **module** is a workspace member under `crates/` other than the
//! default engine's six crates, the front door (`shuyi`) and this tool
//! (`xtask`). It fails on:
//!
//! 1. a default crate that depends on a module, by any kind of
//!    dependency;
//! 2. a module with no `README.md`, no `[package.metadata.shuyi]` (or one
//!    that doesn't read, or serves no direction), or no row in the
//!    catalog;
//! 3. a module that answers a concern, or an answer, not in
//!    `docs/modules.ron`;
//! 4. two modules answering one concern where one's `alternatives` doesn't
//!    name the other, unless the concern is `alternatives: false`;
//! 5. a feature of the `shuyi` crate that names no catalog row (feature
//!    `physics` is the row of `shuyi-physics`).
//!
//! [`check`] is a pure function over plain data; [`run`] feeds it the
//! repository.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::layers::{self, Graph, Kind, Member};

/// The concerns, from the repository's root.
pub const CONCERNS: &str = "docs/modules.ron";

/// The catalog, from the repository's root.
pub const CATALOG: &str = "docs/modules.md";

/// The ADR a module's duties are in, named in what to do.
const ADR: &str = "ADR-0009";

/// The default engine's crates: no module, and none depends on one.
pub const DEFAULT_CRATES: [&str; 6] = [
    "shuyi-core",
    "shuyi-assets",
    "shuyi-ui",
    "shuyi-platform",
    "shuyi-app",
    "shuyi-cli",
];

/// The members under `crates/` that are neither default crates nor
/// modules: the front door and this tool.
const NOT_MODULES: [&str; 2] = ["shuyi", "xtask"];

/// `docs/modules.ron`, as the check reads it (`shuyi_cli::catalog` reads
/// all of it).
#[derive(Debug, Deserialize)]
pub struct Concerns {
    /// Every concern.
    pub concerns: Vec<Concern>,
}

/// A concern, as the check reads it.
#[derive(Debug, Deserialize)]
pub struct Concern {
    /// Its id.
    pub id: String,
    /// Whether two modules answering it must name each other.
    pub alternatives: bool,
    /// Its answers.
    pub answers: Vec<Choice>,
}

/// An answer, as the check reads it.
#[derive(Debug, Deserialize)]
pub struct Choice {
    /// Its id.
    pub id: String,
}

/// The keys of a module's `[package.metadata.shuyi]`, as `shuyi_cli`'s
/// `ModuleInfo` takes them.
const KEYS: [&str; 5] = ["answers", "serves", "alternatives", "works_with", "formats"];

/// The parts of a module's `[package.metadata.shuyi]` the check reads.
#[derive(Debug, Deserialize)]
pub struct Info {
    /// The concerns it answers, and how.
    pub answers: Vec<Answered>,
    /// The directions it serves.
    pub serves: Vec<String>,
    /// The modules that answer one of its concerns another way.
    #[serde(default)]
    pub alternatives: Vec<String>,
}

impl Info {
    /// A module's metadata, read; or why it doesn't read, a key not in
    /// [`KEYS`] among the reasons.
    pub fn read(metadata: &serde_json::Value) -> Result<Self, String> {
        if let Some(table) = metadata.as_object()
            && let Some(key) = table.keys().find(|key| !KEYS.contains(&key.as_str()))
        {
            return Err(format!(
                "unknown key `{key}`, expected one of {}",
                KEYS.join(", ")
            ));
        }
        serde_json::from_value(metadata.clone()).map_err(|e| e.to_string())
    }
}

/// One concern a module answers.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Answered {
    /// The concern's id.
    pub concern: String,
    /// The answer's id.
    pub answer: String,
}

/// Checks the repository at `repo_root` and returns every error (empty
/// means clean).
pub fn run(repo_root: &Path) -> Vec<String> {
    let read =
        |file: &str| fs::read_to_string(repo_root.join(file)).map_err(|e| format!("{file}: {e}"));
    let concerns = read(CONCERNS).and_then(|text| parse_concerns(&text));
    let catalog = read(CATALOG);
    let graph = layers::metadata(repo_root).and_then(|json| layers::parse_graph(&json));
    match (concerns, catalog, graph) {
        (Ok(concerns), Ok(catalog), Ok(graph)) => check(&graph, &concerns, &catalog, |member| {
            member.dir.join("README.md").is_file()
        }),
        (concerns, catalog, graph) => concerns
            .err()
            .into_iter()
            .chain(catalog.err())
            .chain(graph.err())
            .collect(),
    }
}

/// `docs/modules.ron`'s text, read.
pub fn parse_concerns(text: &str) -> Result<Concerns, String> {
    ron::from_str(text).map_err(|e| format!("{CONCERNS}: {e}"))
}

/// The names the catalog has rows for: the first `` `name` `` of the first
/// cell of each table row.
pub fn rows(catalog: &str) -> BTreeSet<String> {
    catalog
        .lines()
        .filter_map(|line| line.trim().strip_prefix('|'))
        .filter_map(|row| row.split('|').next())
        .filter_map(|cell| cell.trim().strip_prefix('`'))
        .filter_map(|cell| cell.split('`').next())
        .map(str::to_owned)
        .collect()
}

/// Whether `member` sits in a folder of `crates/`.
fn under_crates(member: &Member) -> bool {
    member
        .dir
        .parent()
        .and_then(Path::file_name)
        .is_some_and(|name| name == "crates")
}

/// Every error the modules of `graph` have against `concerns` and the
/// catalog's text. `has_readme` says whether a member has its README.
pub fn check(
    graph: &Graph,
    concerns: &Concerns,
    catalog: &str,
    has_readme: impl Fn(&Member) -> bool,
) -> Vec<String> {
    let rows = rows(catalog);
    let modules: BTreeMap<&str, &Member> = graph
        .members
        .iter()
        .filter(|(name, member)| {
            under_crates(member)
                && !DEFAULT_CRATES.contains(&name.as_str())
                && !NOT_MODULES.contains(&name.as_str())
        })
        .map(|(name, member)| (name.as_str(), member))
        .collect();
    let mut errors = Vec::new();

    // Rule 1.
    for name in DEFAULT_CRATES {
        let Some(member) = graph.members.get(name) else {
            continue;
        };
        for dep in member
            .deps
            .iter()
            .filter(|d| modules.contains_key(d.name.as_str()))
        {
            let kind = match dep.kind {
                Kind::Normal => "a dependency",
                Kind::Build => "a build dependency",
                Kind::Dev => "a dev-dependency",
            };
            errors.push(format!(
                "{name} depends on the module {} ({kind}): no default crate depends on a module \
                 ({ADR} §4); move what needs it into the module, or into the game",
                dep.name
            ));
        }
    }

    // Rules 2 and 3.
    let mut infos: BTreeMap<&str, Info> = BTreeMap::new();
    for (&name, member) in &modules {
        if !has_readme(member) {
            errors.push(format!(
                "{name} is a module (a crate under crates/ outside the default engine) with no \
                 README.md: write one in {ADR} §4's order"
            ));
        }
        if !rows.contains(name) {
            errors.push(format!(
                "{name} is a module with no row in {CATALOG}: add one under its concern (what it \
                 serves, its status, its tickets)"
            ));
        }
        match member.shuyi.as_ref().map(Info::read) {
            None => errors.push(format!(
                "{name} is a module with no [package.metadata.shuyi] in its Cargo.toml: say what \
                 it answers (`answers`, `serves`, `alternatives`, `works_with`, {ADR} §4)"
            )),
            Some(Err(e)) => errors.push(format!(
                "{name}'s [package.metadata.shuyi] doesn't read: {e} ({ADR} §4)"
            )),
            Some(Ok(info)) => {
                if info.serves.is_empty() {
                    errors.push(format!(
                        "{name}'s `serves` in [package.metadata.shuyi] is empty: name the \
                         directions it is built for ({ADR} §7)"
                    ));
                }
                check_answers(name, &info, concerns, &mut errors);
                infos.insert(name, info);
            }
        }
    }

    // Rule 4.
    for concern in concerns.concerns.iter().filter(|c| c.alternatives) {
        let answering: Vec<(&str, &Info)> = infos
            .iter()
            .filter(|(_, info)| info.answers.iter().any(|a| a.concern == concern.id))
            .map(|(name, info)| (*name, info))
            .collect();
        for (name, info) in &answering {
            for (other, _) in answering.iter().filter(|(other, _)| other != name) {
                if !info.alternatives.iter().any(|a| a == other) {
                    errors.push(format!(
                        "{name} and {other} both answer `{}` and {name}'s `alternatives` doesn't \
                         name {other}: alternatives name each other ({ADR} §2), unless the \
                         concern is `alternatives: false` in {CONCERNS}",
                        concern.id
                    ));
                }
            }
        }
    }

    // Rule 5.
    if let Some(front) = graph.members.get("shuyi") {
        for feature in front.features.iter().filter(|f| *f != "default") {
            let row = format!("shuyi-{feature}");
            if !rows.contains(&row) {
                errors.push(format!(
                    "the shuyi crate's feature `{feature}` names no row of {CATALOG}: a feature \
                     turns on a module (`{feature}` is {row}), and every module has a row"
                ));
            }
        }
    }
    errors
}

/// Rule 3: every answer of module `name` is in `concerns`.
fn check_answers(name: &str, info: &Info, concerns: &Concerns, errors: &mut Vec<String>) {
    for answered in &info.answers {
        let Some(concern) = concerns.concerns.iter().find(|c| c.id == answered.concern) else {
            errors.push(format!(
                "{name} answers the concern `{}`, which {CONCERNS} doesn't have: add it there \
                 ({ADR} §2), or fix the name",
                answered.concern
            ));
            continue;
        };
        if !concern.answers.iter().any(|a| a.id == answered.answer) {
            errors.push(format!(
                "{name} answers `{}` with `{}`, which {CONCERNS} doesn't list under it: add the \
                 answer there ({ADR} §2), or fix the name",
                answered.concern, answered.answer
            ));
        }
    }
}

#[cfg(test)]
mod tests;
