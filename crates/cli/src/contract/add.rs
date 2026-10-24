//! `add <module>`: a module of the game's engine turned on (ADR-0009
//! §6). It:
//!
//! 1. turns on the module's feature of the `shuyi` crate in the game's
//!    `Cargo.toml`, keeping its layout ([`catalog::turn_on`]);
//! 2. copies the module's skill into the game's skills (`.claude/skills`),
//!    if the game keeps them ([`catalog::copy_skill`]);
//! 3. gives the first paragraph of the module's README: what it is for;
//! 4. says so when the game already has one of its alternatives on, with
//!    the README's section on using both ([`BESIDE`]);
//! 5. gives the line to add when the game's code never installs the
//!    modules it turns on (`shuyi::enabled_modules`), and the line for the
//!    game's `formats()` when the module adds formats.
//!
//! Run twice, the second run changes nothing and says so (`changed:
//! false`). A name that is no module of the engine fails with
//! [`codes::UNKNOWN_MODULE`] and the names there are.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Answer, Op, codes};
use crate::catalog::{self, Catalog, SkillCopy};
use crate::project::Project;

/// The anchor of a module README's section on sitting beside its
/// alternatives, and moving between them (its fifth, ADR-0009 §4):
/// `## Beside its alternatives`.
pub const BESIDE: &str = "beside-its-alternatives";

/// What `add` turns on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `AddInput::new` (a module has no default) and its
// fields assigned, it may grow (ADR-0039).
#[non_exhaustive]
pub struct AddInput {
    /// The module, by its crate's name with or without `shuyi-`
    /// (`physics` or `shuyi-physics`).
    pub module: String,
}

impl AddInput {
    /// The module named `module`.
    pub fn new(module: impl Into<String>) -> Self {
        Self {
            module: module.into(),
        }
    }
}

/// What `add` did, and what is left to the game.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddOutput {
    /// The module's crate (`shuyi-physics`).
    pub module: String,
    /// Its feature of the `shuyi` crate (`physics`).
    pub feature: String,
    /// Whether the game's `Cargo.toml` changed; `false` when the module
    /// was on already.
    pub changed: bool,
    /// Where its skill was copied, from the game's folder; absent when
    /// none was (the limits say why).
    pub skill: Option<String>,
    /// What it is for: its README's first paragraph.
    pub about: String,
    /// Its alternatives the game already has on.
    pub alternatives_on: Vec<String>,
    /// Its README's section on sitting beside them, when one is on.
    pub beside: Option<String>,
    /// The line to add to the game's code, when it never installs the
    /// modules it turns on.
    pub install: Option<String>,
    /// The line for the game's `formats()`, when the module adds formats.
    pub formats: Option<String>,
}

/// `add` on the game of `project`, in the engine it builds with.
pub fn add(project: &Project, input: &AddInput) -> Answer<AddOutput> {
    match catalog::engine_root(&project.root).and_then(|root| Catalog::read(&root)) {
        Ok(catalog) => add_in(&project.root, &catalog, input),
        Err(e) => Answer::failed(Op::Add, codes::ENGINE_DEPENDENCY, e),
    }
}

/// `add` on the game at `game`, with the engine's `catalog`.
pub fn add_in(game: &Path, catalog: &Catalog, input: &AddInput) -> Answer<AddOutput> {
    match added(game, catalog, input) {
        Ok((output, limits)) => Answer::new(Op::Add, true, output, limits),
        Err((code, e)) => Answer::failed(Op::Add, code, e),
    }
}

/// Why `name` is no module of `catalog`: one it plans, or the names there
/// are.
pub(crate) fn unknown(catalog: &Catalog, name: &str) -> String {
    let module = if name.starts_with("shuyi-") {
        name.to_owned()
    } else {
        catalog::module_of(name)
    };
    let planned: Vec<String> = catalog
        .concerns
        .answered_by(&module)
        .map(|(_, answer)| {
            format!(
                "{} is {} ({})",
                answer.name,
                answer.status.name(),
                answer.tickets.join(", ")
            )
        })
        .collect();
    if !planned.is_empty() {
        return format!(
            "{module} is in the catalog but not built yet: {}",
            planned.join("; ")
        );
    }
    let known: Vec<&str> = catalog.modules.iter().map(|m| m.name.as_str()).collect();
    if known.is_empty() {
        format!("no module {name}: the game's engine builds no module yet")
    } else {
        format!(
            "no module {name}: the game's engine builds {}",
            known.join(", ")
        )
    }
}

/// What `add` did to the game at `game`, with its limits; or the problem
/// code and why it couldn't.
fn added(
    game: &Path,
    catalog: &Catalog,
    input: &AddInput,
) -> Result<(AddOutput, Vec<String>), (codes::Code, String)> {
    let module = catalog
        .module(&input.module)
        .ok_or_else(|| (codes::UNKNOWN_MODULE, unknown(catalog, &input.module)))?;
    let about = catalog::readme_paragraph(&module.dir).map_err(|e| (codes::FILE, e))?;
    let on = super::modules::on_in(game)?;
    let manifest = game.join("Cargo.toml");
    let text = fs::read_to_string(&manifest)
        .map_err(|e| (codes::FILE, format!("{}: {e}", manifest.display())))?;
    let edited =
        catalog::turn_on(&text, module.feature()).map_err(|e| (codes::ENGINE_DEPENDENCY, e))?;
    if let Some(edited) = &edited {
        fs::write(&manifest, edited)
            .map_err(|e| (codes::FILE, format!("{}: {e}", manifest.display())))?;
    }

    let mut limits =
        vec!["only the game's root Cargo.toml was read for the modules it has on".to_owned()];
    let skill = match catalog::skill_of(&module.dir, &module.name) {
        None => {
            limits.push(format!("{} has no skill to copy", module.name));
            None
        }
        Some(skill) => match catalog::copy_skill(&skill, game, &module.name) {
            Ok(SkillCopy::Copied(_)) => Some(format!("{}/{}", catalog::SKILLS, module.name)),
            Ok(SkillCopy::AlreadyThere(_)) => {
                limits.push(format!(
                    "the game already has a skill {} in {}: it was left as it is",
                    module.name,
                    catalog::SKILLS
                ));
                None
            }
            Ok(SkillCopy::NoSkills) => {
                limits.push(format!(
                    "the game keeps no skills ({} isn't there): {}'s skill wasn't copied",
                    catalog::SKILLS,
                    module.name
                ));
                None
            }
            Err(e) => return Err((codes::FILE, e)),
        },
    };

    let alternatives_on: Vec<String> = on
        .iter()
        .map(|feature| catalog::module_of(feature))
        .filter(|other| module.info.alternatives.contains(other))
        .collect();
    let beside = (!alternatives_on.is_empty()).then(|| {
        let readme = module.dir.join("README.md");
        format!("{}#{BESIDE}", readme.to_string_lossy().replace('\\', "/"))
    });
    let install = (!catalog::installs_modules(game)).then(|| {
        "add `.modules(shuyi::enabled_modules)` to the game's `BasicGame`, or \
         `.with_modules(shuyi::enabled_modules())` to its `Game`: every module it turns on then \
         joins it"
            .to_owned()
    });
    let formats = module.info.formats.as_ref().map(|function| {
        format!(
            "call `{function}` in the game's `formats()`: its formats are read with the game's \
             data, before its `Game` exists"
        )
    });
    let output = AddOutput {
        module: module.name.clone(),
        feature: module.feature().to_owned(),
        changed: edited.is_some(),
        skill,
        about,
        alternatives_on,
        beside,
        install,
        formats,
    };
    Ok((output, limits))
}

/// `add`'s usage.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} add <module> [--json]\n\n\
         Turns on a module of the game's engine (`physics` or `shuyi-physics`): its feature\n\
         of `shuyi` in Cargo.toml, its skill copied, what it is for. `{command} modules`\n\
         lists them."
    )
}

/// Reads `add`'s arguments (not `--json`).
pub fn parse_args(args: &[String]) -> Result<AddInput, String> {
    match args {
        [module] if !module.starts_with("--") => Ok(AddInput::new(module.clone())),
        _ => Err("one <module>, nothing else".to_owned()),
    }
}

/// What `add` did, for people.
pub fn text(output: &AddOutput) -> String {
    let mut out = if output.changed {
        format!(
            "add: {} on (the feature `{}` of shuyi in Cargo.toml)\n",
            output.module, output.feature
        )
    } else {
        format!("add: {} was already on: nothing changed\n", output.module)
    };
    for line in output.about.lines() {
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "  {line}");
    }
    if let Some(skill) = &output.skill {
        let _ = writeln!(out, "skill: copied to {skill}");
    }
    if !output.alternatives_on.is_empty() {
        let _ = writeln!(
            out,
            "warning: the game already has {} on, an alternative: two alternatives never drive \
             the same thing in one stage{}",
            output.alternatives_on.join(", "),
            output
                .beside
                .as_ref()
                .map(|beside| format!("; see {beside}"))
                .unwrap_or_default()
        );
    }
    for to_do in output.install.iter().chain(&output.formats) {
        let _ = writeln!(out, "to do: {to_do}");
    }
    out
}

#[cfg(test)]
mod tests;
