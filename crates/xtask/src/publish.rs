//! `cargo xtask check-publish`: the engine crates are the publish list,
//! and their manifests can be packaged together (ticket 3510, ADR-0152).
//!
//! A workspace member is published when [`layers::is_engine`] names it,
//! and only then. The check reads the root `Cargo.toml` and each member's,
//! found through `cargo metadata`, and fails:
//!
//! 1. an engine crate whose `version` or `publish` isn't the workspace's;
//! 2. any other member without its own `publish = false`;
//! 3. an engine crate's normal or build dependency on an engine crate that
//!    doesn't come from `[workspace.dependencies]`, an engine crate other
//!    crates depend on with no entry there, or an entry whose `version`
//!    isn't `=` and the workspace's version;
//! 4. an engine crate's dev-dependency on an engine crate that carries a
//!    version (cargo keeps it in the package then, and the dev cycles with
//!    it);
//! 5. an engine crate that depends, normally or to build, on a member that
//!    isn't published;
//! 6. a cycle among the engine crates' normal and build dependencies.
//!
//! [`order`] gives the engine crates in the order they publish, each after
//! what it depends on. [`missing_from_index`] asks crates.io's sparse index
//! for each name, through a function the tests replace.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use crate::layers::{Kind, is_engine};

#[cfg(test)]
mod tests;

/// The ADR the rules are in, named in what to do.
const ADR: &str = "ADR-0152";

/// crates.io's sparse index.
pub const INDEX: &str = "https://index.crates.io";

/// A manifest field that a member may inherit from the workspace.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Field {
    /// `<field>.workspace = true`.
    Inherited,
    /// Its own value, as TOML writes it (`false`, `"0.2.0"`).
    Own(String),
    /// Not written.
    #[default]
    Missing,
}

/// A dependency as a member's manifest writes it.
#[derive(Debug, Clone)]
pub struct Dep {
    /// The package's name (its key, or its `package` when renamed).
    pub name: String,
    /// Its kind.
    pub kind: Kind,
    /// Whether it says `workspace = true`.
    pub workspace: bool,
    /// Its own `version`, if it writes one.
    pub version: Option<String>,
}

/// A member's manifest, as the check reads it.
#[derive(Debug, Clone, Default)]
pub struct Manifest {
    /// `package.version`.
    pub version: Field,
    /// `package.publish`.
    pub publish: Field,
    /// Every dependency, on every target.
    pub deps: Vec<Dep>,
}

/// The workspace, as the check reads it.
#[derive(Debug, Clone, Default)]
pub struct Workspace {
    /// `[workspace.package]`'s `version`.
    pub version: Option<String>,
    /// `[workspace.dependencies]`: each entry's `version`, by package name.
    pub entries: BTreeMap<String, Option<String>>,
    /// The members, by package name.
    pub members: BTreeMap<String, Manifest>,
}

/// The repository's workspace, read from its manifests.
pub fn read(root: &Path) -> Result<Workspace, String> {
    let members = members(root)?;
    let root_toml = read_toml(&root.join("Cargo.toml"))?;
    let mut workspace = parse_root(&root_toml);
    for (name, manifest) in members {
        let table = read_toml(&manifest)?;
        workspace.members.insert(name, parse_member(&table));
    }
    Ok(workspace)
}

fn read_toml(path: &Path) -> Result<toml::Table, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    text.parse::<toml::Table>()
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// The parts of `cargo metadata --no-deps` read here.
#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
}

/// The workspace's members and their manifests, from `cargo metadata`.
fn members(root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--no-deps", "--locked"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cargo metadata does not run: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_members(&String::from_utf8_lossy(&output.stdout))
}

/// `cargo metadata --no-deps`'s JSON, read into each member's name and
/// manifest.
pub fn parse_members(json: &str) -> Result<Vec<(String, PathBuf)>, String> {
    let metadata: Metadata =
        serde_json::from_str(json).map_err(|e| format!("cargo metadata's output: {e}"))?;
    Ok(metadata
        .packages
        .into_iter()
        .filter(|p| metadata.workspace_members.contains(&p.id))
        .map(|p| (p.name, p.manifest_path))
        .collect())
}

/// The root manifest's `[workspace.package]` version and its
/// `[workspace.dependencies]` entries.
pub fn parse_root(table: &toml::Table) -> Workspace {
    let workspace = table.get("workspace").and_then(toml::Value::as_table);
    let version = workspace
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let entries = workspace
        .and_then(|w| w.get("dependencies"))
        .and_then(toml::Value::as_table)
        .into_iter()
        .flatten()
        .map(|(key, value)| {
            let dep = dep(key, value, Kind::Normal);
            (dep.name, dep.version)
        })
        .collect();
    Workspace {
        version,
        entries,
        members: BTreeMap::new(),
    }
}

/// A member's manifest, read.
pub fn parse_member(table: &toml::Table) -> Manifest {
    let package = table.get("package");
    let field = |name: &str| match package.and_then(|p| p.get(name)) {
        None => Field::Missing,
        Some(toml::Value::Table(t)) if t.get("workspace") == Some(&toml::Value::Boolean(true)) => {
            Field::Inherited
        }
        Some(toml::Value::String(s)) => Field::Own(s.clone()),
        Some(toml::Value::Boolean(b)) => Field::Own(b.to_string()),
        Some(other) => Field::Own(format!("{other:?}")),
    };
    let mut deps = Vec::new();
    let mut tables = vec![table];
    if let Some(targets) = table.get("target").and_then(toml::Value::as_table) {
        tables.extend(targets.values().filter_map(toml::Value::as_table));
    }
    for t in tables {
        for (section, kind) in [
            ("dependencies", Kind::Normal),
            ("build-dependencies", Kind::Build),
            ("dev-dependencies", Kind::Dev),
        ] {
            for (key, value) in t
                .get(section)
                .and_then(toml::Value::as_table)
                .into_iter()
                .flatten()
            {
                deps.push(dep(key, value, kind));
            }
        }
    }
    Manifest {
        version: field("version"),
        publish: field("publish"),
        deps,
    }
}

fn dep(key: &str, value: &toml::Value, kind: Kind) -> Dep {
    let get = |field: &str| value.as_table().and_then(|t| t.get(field));
    Dep {
        name: get("package")
            .and_then(toml::Value::as_str)
            .unwrap_or(key)
            .to_owned(),
        kind,
        workspace: get("workspace").and_then(toml::Value::as_bool) == Some(true),
        version: value
            .as_str()
            .or_else(|| get("version").and_then(toml::Value::as_str))
            .map(str::to_owned),
    }
}

/// Every error the rules find in `workspace`.
pub fn check(workspace: &Workspace) -> Vec<String> {
    let engine: BTreeSet<&str> = workspace
        .members
        .keys()
        .map(String::as_str)
        .filter(|name| is_engine(name))
        .collect();
    let mut errors = Vec::new();
    let mut depended_on = BTreeSet::new();
    for (name, manifest) in &workspace.members {
        if !engine.contains(name.as_str()) {
            if manifest.publish != Field::Own("false".into()) {
                errors.push(format!(
                    "{name} is no engine crate and has no `publish = false` of its own: add it \
                     to its [package], so the workspace's `publish` never makes it publishable \
                     ({ADR})"
                ));
            }
            continue;
        }
        if manifest.version != Field::Inherited {
            errors.push(format!(
                "{name}'s `version` is not the workspace's: write `version.workspace = true` \
                 (one version for every engine crate, {ADR})"
            ));
        }
        if manifest.publish != Field::Inherited {
            errors.push(format!(
                "{name} sets its own `publish`: an engine crate is published with the rest, so \
                 write `publish.workspace = true` ({ADR})"
            ));
        }
        for dep in &manifest.deps {
            let is_member = workspace.members.contains_key(&dep.name);
            if dep.kind == Kind::Dev {
                if engine.contains(dep.name.as_str()) && (dep.workspace || dep.version.is_some()) {
                    errors.push(format!(
                        "{name}'s dev-dependency on {} carries a version: keep it path-only \
                         (`path = ...` alone), so cargo leaves it out of the package ({ADR})",
                        dep.name
                    ));
                }
            } else if engine.contains(dep.name.as_str()) {
                depended_on.insert(dep.name.as_str());
                if !dep.workspace {
                    errors.push(format!(
                        "{name}'s {} dependency on {} doesn't come from \
                         [workspace.dependencies]: write `{} = {{ workspace = true, ... }}` \
                         with its own `features` and `optional` (its exact pin, {ADR})",
                        kind(dep.kind),
                        dep.name,
                        dep.name
                    ));
                }
            } else if is_member {
                errors.push(format!(
                    "{name} depends on {}, a member that isn't published, so {name}'s package \
                     can't resolve it: make it a dev-dependency, or move what it needs into an \
                     engine crate ({ADR})",
                    dep.name
                ));
            }
        }
    }
    errors.extend(check_entries(workspace, &depended_on));
    if let Err(cycle) = order(workspace) {
        errors.push(format!(
            "the engine crates' normal and build dependencies make a cycle ({}): no order \
             publishes them, so break it (a dev-dependency may close a cycle, {ADR})",
            cycle.join(" → ")
        ));
    }
    errors
}

/// Rule 3's half on the root: an entry for each engine crate depended on,
/// pinned exactly to the workspace's version.
fn check_entries(workspace: &Workspace, depended_on: &BTreeSet<&str>) -> Vec<String> {
    let mut errors = Vec::new();
    let Some(version) = &workspace.version else {
        return vec![
            "the root Cargo.toml's [workspace.package] has no `version`: the engine crates \
             share it"
                .into(),
        ];
    };
    let pin = format!("={version}");
    for name in depended_on {
        if !workspace.entries.contains_key(*name) {
            errors.push(format!(
                "an engine crate depends on {name}, and [workspace.dependencies] has no entry \
                 for it: add `{name} = {{ path = ..., version = \"{pin}\", default-features = \
                 false }}` ({ADR})"
            ));
        }
    }
    for (name, entry) in &workspace.entries {
        if !is_engine(name) || !workspace.members.contains_key(name) {
            continue;
        }
        match entry {
            Some(v) if *v == pin => {}
            Some(v) if !v.starts_with('=') => errors.push(format!(
                "[workspace.dependencies] pins {name} to \"{v}\", which lets cargo pick a later \
                 version: write \"{pin}\", exactly the workspace's version ({ADR})"
            )),
            Some(v) => errors.push(format!(
                "[workspace.dependencies] pins {name} to \"{v}\", and the workspace's version \
                 is {version}: write \"{pin}\" ({ADR})"
            )),
            None => errors.push(format!(
                "[workspace.dependencies]'s entry for {name} has no `version`, and cargo \
                 refuses to package a crate that depends on it: write `version = \"{pin}\"` \
                 ({ADR})"
            )),
        }
    }
    errors
}

fn kind(kind: Kind) -> &'static str {
    match kind {
        Kind::Normal => "normal",
        Kind::Build => "build",
        Kind::Dev => "dev",
    }
}

/// The engine crates in the order they publish: each after every engine
/// crate it depends on normally or to build, ties by name. Dev edges don't
/// count. On a cycle, the crates of one, the first repeated at the end.
pub fn order(workspace: &Workspace) -> Result<Vec<String>, Vec<String>> {
    let deps: BTreeMap<&str, BTreeSet<&str>> = workspace
        .members
        .iter()
        .filter(|(name, _)| is_engine(name))
        .map(|(name, manifest)| {
            let on = manifest
                .deps
                .iter()
                .filter(|d| d.kind != Kind::Dev && d.name != *name)
                .map(|d| d.name.as_str())
                .filter(|d| is_engine(d) && workspace.members.contains_key(*d))
                .collect();
            (name.as_str(), on)
        })
        .collect();
    let mut done: Vec<String> = Vec::new();
    let mut left: BTreeSet<&str> = deps.keys().copied().collect();
    while let Some(next) = left
        .iter()
        .copied()
        .find(|name| deps[name].iter().all(|d| !left.contains(d)))
    {
        left.remove(next);
        done.push(next.to_owned());
    }
    match left.first() {
        None => Ok(done),
        Some(&start) => Err(cycle(start, &deps, &left)),
    }
}

/// A cycle among `left`, each of which depends on one of `left`: walked
/// from `start` until a crate repeats.
fn cycle(start: &str, deps: &BTreeMap<&str, BTreeSet<&str>>, left: &BTreeSet<&str>) -> Vec<String> {
    let mut path = vec![start];
    loop {
        let at = path[path.len() - 1];
        let Some(next) = deps[at].iter().copied().find(|d| left.contains(d)) else {
            return path.into_iter().map(str::to_owned).collect();
        };
        if let Some(i) = path.iter().position(|p| *p == next) {
            let mut cycle: Vec<String> = path[i..].iter().map(|p| (*p).to_owned()).collect();
            cycle.push(next.to_owned());
            return cycle;
        }
        path.push(next);
    }
}

/// A crate's path in the sparse index (`https://index.crates.io/<path>`).
pub fn index_path(name: &str) -> String {
    let name = name.to_ascii_lowercase();
    match name.len() {
        1 => format!("1/{name}"),
        2 => format!("2/{name}"),
        3 => format!("3/{}/{name}", &name[..1]),
        _ => format!("{}/{}/{name}", &name[..2], &name[2..4]),
    }
}

/// An error for each of `names` the index doesn't hold. `has` answers
/// whether one is there, or why it can't tell.
pub fn missing_from_index(
    names: &[String],
    has: impl Fn(&str) -> Result<bool, String>,
) -> Vec<String> {
    names
        .iter()
        .filter_map(|name| match has(name) {
            Ok(true) => None,
            Ok(false) => Some(format!(
                "{name} is not on crates.io ({INDEX}/{}): its name must be held before it can \
                 be published, which is Nick's to do (a 0.0.0 placeholder, ask with the \
                 `ask-nick` skill)",
                index_path(name)
            )),
            Err(error) => Some(format!("{name}: crates.io's index can't be read: {error}")),
        })
        .collect()
}

/// Whether crates.io's sparse index has `name`, asked with `curl`.
pub fn on_index(name: &str) -> Result<bool, String> {
    let url = format!("{INDEX}/{}", index_path(name));
    let null = if cfg!(windows) { "NUL" } else { "/dev/null" };
    let output = Command::new("curl")
        .args(["-sS", "-o", null, "-w", "%{http_code}", &url])
        .output()
        .map_err(|e| format!("curl does not run: {e}"))?;
    let code = String::from_utf8_lossy(&output.stdout);
    match code.trim() {
        "200" => Ok(true),
        "404" => Ok(false),
        other => Err(format!(
            "{url} answered {other:?}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
    }
}
