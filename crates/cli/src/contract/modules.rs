//! `modules`: the catalog by concern (ADR-0009 §6), as the engine the
//! game builds with has it, and which modules the game has on.
//!
//! It reads the engine checkout the game's `shuyi` dependency names
//! ([`catalog::engine_root`]): its concerns (`docs/modules.ron`), every
//! answer with who gives it and its status, and the module crates it
//! builds, each with its metadata. A module is on when the game's
//! `Cargo.toml` turns on its feature of the `shuyi` crate.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Answer, Op, codes};
use crate::catalog::{self, Catalog, Concern, ModuleInfo};
use crate::project::Project;

/// What `modules` lists: everything (nothing to choose yet).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModulesInput {}

/// The catalog, and the game's modules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModulesOutput {
    /// The engine checkout read, `/`-separated.
    pub engine: String,
    /// The concerns, each with its answers, who gives each and its
    /// status.
    pub concerns: Vec<Concern>,
    /// The modules the engine builds, by name.
    pub modules: Vec<ModuleEntry>,
    /// The modules the game has on, by name, in its `Cargo.toml`'s order.
    pub on: Vec<String>,
}

/// A module the engine builds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ModuleEntry {
    /// Its crate (`shuyi-physics`).
    pub name: String,
    /// Its feature of the `shuyi` crate (`physics`).
    pub feature: String,
    /// Whether the game has it on.
    pub on: bool,
    /// Its metadata: what it answers, serves and sits beside.
    pub info: ModuleInfo,
}

/// `modules` on the game of `project`, in the engine it builds with.
pub fn modules(project: &Project, input: &ModulesInput) -> Answer<ModulesOutput> {
    match catalog::engine_root(&project.root).and_then(|root| Catalog::read(&root)) {
        Ok(catalog) => modules_in(&project.root, &catalog, input),
        Err(e) => Answer::failed(Op::Modules, codes::ENGINE_DEPENDENCY, e),
    }
}

/// `modules` on the game at `game`, with the engine's `catalog`.
pub fn modules_in(game: &Path, catalog: &Catalog, _input: &ModulesInput) -> Answer<ModulesOutput> {
    let features = match on_in(game) {
        Ok(features) => features,
        Err((code, e)) => return Answer::failed(Op::Modules, code, e),
    };
    let mut limits =
        vec!["only the game's root Cargo.toml was read for the modules it has on".to_owned()];
    for feature in &features {
        if catalog.module(feature).is_none() {
            limits.push(format!(
                "the game turns on the feature `{feature}` of `shuyi`, which no module of its \
                 engine has"
            ));
        }
    }
    let on: Vec<String> = features.iter().map(|f| catalog::module_of(f)).collect();
    let output = ModulesOutput {
        engine: catalog.root.to_string_lossy().replace('\\', "/"),
        concerns: catalog.concerns.concerns.clone(),
        modules: catalog
            .modules
            .iter()
            .map(|module| ModuleEntry {
                name: module.name.clone(),
                feature: module.feature().to_owned(),
                on: on.contains(&module.name),
                info: module.info.clone(),
            })
            .collect(),
        on,
    };
    Answer::new(Op::Modules, true, output, limits)
}

/// The features of `shuyi` the game at `game` turns on, or the problem
/// code and why they can't be read.
pub(crate) fn on_in(game: &Path) -> Result<Vec<String>, (codes::Code, String)> {
    let path = game.join("Cargo.toml");
    let text =
        fs::read_to_string(&path).map_err(|e| (codes::FILE, format!("{}: {e}", path.display())))?;
    catalog::features_on(&text).map_err(|e| (codes::ENGINE_DEPENDENCY, e))
}

/// `modules`' usage.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} modules [--json]\n\n\
         The engine's catalog by concern: each answer, who gives it and its status, the\n\
         modules the engine builds, and the ones this game has on."
    )
}

/// What `modules` lists, for people.
pub fn text(output: &ModulesOutput) -> String {
    let mut out = format!("modules: the catalog of {}\n", output.engine);
    for concern in &output.concerns {
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "{}", concern.name);
        for answer in &concern.answers {
            let mark = if output.on.contains(&answer.by) {
                " (on)"
            } else {
                ""
            };
            let _ = writeln!(
                out,
                "  {}: {}, {}{mark}",
                answer.name,
                answer.by,
                answer.status.name()
            );
        }
    }
    if output.modules.is_empty() {
        out.push_str("the engine builds no module yet\n");
    }
    for module in &output.modules {
        let on = if module.on { "on" } else { "off" };
        let _ = writeln!(out, "{} ({on}): feature `{}`", module.name, module.feature);
    }
    out
}

#[cfg(test)]
mod tests;
