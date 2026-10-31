//! `cargo xtask check-layers`: the engine's layers (ADR-0032), as
//! `tools/layers.ron` writes them, checked against `cargo metadata
//! --all-features`.
//!
//! It fails on:
//!
//! 1. an engine crate (a workspace member named `shuyi` or `shuyi-*`) with
//!    no entry, or
//!    an entry or an `engine` name that is no engine crate;
//! 2. a normal or build dependency on an engine crate not on the crate's
//!    `engine` list (dev-dependencies are free: `shuyi-ui`'s tests use
//!    `shuyi-platform`'s Harness);
//! 3. with `outside: Only([...])`, a normal or build dependency not on the
//!    list, optional and per-target ones too, and a listed crate no longer
//!    used, so the list stays exact;
//! 4. with `devices: false`, a device crate anywhere below the crate over
//!    the resolve's normal and build edges, on every target;
//! 5. with `floats: false`, a `src/lib.rs` with no line that starts with
//!    `#![` and holds `deny(clippy::float_arithmetic)`.
//!
//! [`check`] is a pure function over plain data; [`run`] feeds it the
//! repository.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

/// The layers, from the repository's root.
pub const LAYERS: &str = "tools/layers.ron";

/// The ADR the layers file is part of, named in what to do.
const ADR: &str = "ADR-0032";

/// What lets a simulation crate's `src/lib.rs` pass the float rule.
const FLOAT_BAN: &str = "deny(clippy::float_arithmetic)";

/// `tools/layers.ron`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layers {
    /// The crates only a platform backend may reach; a trailing `*` is a
    /// prefix.
    pub devices: Vec<String>,
    /// One rule per engine crate, by package name.
    pub crates: BTreeMap<String, CrateRule>,
}

/// What one engine crate may depend on.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrateRule {
    /// The engine crates it may depend on.
    pub engine: Vec<String>,
    /// The outside crates it may use.
    pub outside: Outside,
    /// Whether a device crate may be below it.
    pub devices: bool,
    /// The crates of the `devices` list it may reach all the same, each a
    /// file reader it uses at build time and no device.
    #[serde(default)]
    pub readers: Vec<String>,
    /// Whether it may use floats (`false`: its `src/lib.rs` denies
    /// `clippy::float_arithmetic`).
    pub floats: bool,
}

/// The outside crates an engine crate may use.
#[derive(Debug, Deserialize)]
pub enum Outside {
    /// These, and every one of them used.
    Only(Vec<String>),
    /// Any crate.
    Any,
}

/// A dependency's kind, as `cargo metadata` gives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `[dependencies]`.
    Normal,
    /// `[build-dependencies]`.
    Build,
    /// `[dev-dependencies]`.
    Dev,
}

/// A dependency a workspace member declares, by package name.
#[derive(Debug)]
pub struct Declared {
    /// The package's name (`shuyi-core`, not `shuyi_core`).
    pub name: String,
    /// Its kind.
    pub kind: Kind,
}

/// A workspace member.
#[derive(Debug, Default)]
pub struct Member {
    /// The folder of its `Cargo.toml`.
    pub dir: PathBuf,
    /// Its declared dependencies, on every target.
    pub deps: Vec<Declared>,
    /// Its cargo features, by name (`check-modules` reads the `shuyi`
    /// crate's).
    pub features: BTreeSet<String>,
    /// Its `[package.metadata.shuyi]`, if it has one (a module's metadata,
    /// ADR-0009 §4).
    pub shuyi: Option<serde_json::Value>,
}

/// What `cargo metadata` says, as the check reads it.
#[derive(Debug, Default)]
pub struct Graph {
    /// The workspace's members, by package name.
    pub members: BTreeMap<String, Member>,
    /// Each package's normal and build dependencies in the resolve, by
    /// package name, on every target.
    pub edges: BTreeMap<String, BTreeSet<String>>,
}

/// Checks the repository at `repo_root` and returns every error (empty
/// means clean).
pub fn run(repo_root: &Path) -> Vec<String> {
    let layers = match fs::read_to_string(repo_root.join(LAYERS)) {
        Ok(text) => parse_layers(&text),
        Err(e) => Err(format!("{LAYERS}: {e}")),
    };
    let graph = metadata(repo_root).and_then(|json| parse_graph(&json));
    match (layers, graph) {
        (Ok(layers), Ok(graph)) => check(&layers, &graph, |name| {
            let member = graph.members.get(name)?;
            fs::read_to_string(member.dir.join("src").join("lib.rs")).ok()
        }),
        (layers, graph) => layers.err().into_iter().chain(graph.err()).collect(),
    }
}

/// `tools/layers.ron`'s text, read.
pub fn parse_layers(text: &str) -> Result<Layers, String> {
    ron::from_str(text).map_err(|e| format!("{LAYERS}: {e}"))
}

/// `cargo metadata`'s JSON for the repository, with every feature on, run
/// by the `cargo` this runs under.
pub fn metadata(repo_root: &Path) -> Result<String, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--format-version",
            "1",
            "--all-features",
            "--locked",
        ])
        .current_dir(repo_root)
        .output()
        .map_err(|e| format!("cargo metadata does not run: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| format!("cargo metadata's output: {e}"))
}

/// The parts of `cargo metadata --format-version 1` the check reads.
#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
    workspace_members: Vec<String>,
    resolve: Option<Resolve>,
}

#[derive(Deserialize)]
struct Package {
    id: String,
    name: String,
    manifest_path: PathBuf,
    dependencies: Vec<Dependency>,
    #[serde(default)]
    features: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    kind: Option<String>,
}

#[derive(Deserialize)]
struct Resolve {
    nodes: Vec<Node>,
}

#[derive(Deserialize)]
struct Node {
    id: String,
    deps: Vec<NodeDep>,
}

/// A resolved dependency. Its `name` is the library's name (`shuyi_core`),
/// so the package is found by `pkg`, its id.
#[derive(Deserialize)]
struct NodeDep {
    pkg: String,
    dep_kinds: Vec<DepKind>,
}

#[derive(Deserialize)]
struct DepKind {
    kind: Option<String>,
}

/// `cargo metadata`'s JSON, read into a [`Graph`].
pub fn parse_graph(json: &str) -> Result<Graph, String> {
    let metadata: Metadata =
        serde_json::from_str(json).map_err(|e| format!("cargo metadata's output: {e}"))?;
    let names: BTreeMap<&str, &str> = metadata
        .packages
        .iter()
        .map(|p| (p.id.as_str(), p.name.as_str()))
        .collect();
    let name_of = |id: &str| {
        names
            .get(id)
            .map(|name| (*name).to_string())
            .ok_or_else(|| format!("cargo metadata names no package {id}"))
    };

    let mut graph = Graph::default();
    for id in &metadata.workspace_members {
        let Some(package) = metadata.packages.iter().find(|p| &p.id == id) else {
            return Err(format!("cargo metadata names no package {id}"));
        };
        let deps = package
            .dependencies
            .iter()
            .map(|d| {
                Ok(Declared {
                    name: d.name.clone(),
                    kind: kind(d.kind.as_deref())?,
                })
            })
            .collect::<Result<_, String>>()?;
        let dir = package
            .manifest_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let member = Member {
            dir,
            deps,
            features: package.features.keys().cloned().collect(),
            shuyi: package
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.get("shuyi"))
                .cloned(),
        };
        graph.members.insert(package.name.clone(), member);
    }

    let resolve = metadata
        .resolve
        .ok_or("cargo metadata gave no resolve of the dependencies")?;
    for node in &resolve.nodes {
        let mut to = BTreeSet::new();
        for dep in &node.deps {
            let kinds = dep
                .dep_kinds
                .iter()
                .map(|k| kind(k.kind.as_deref()))
                .collect::<Result<Vec<_>, _>>()?;
            if kinds.iter().any(|&k| k != Kind::Dev) {
                to.insert(name_of(&dep.pkg)?);
            }
        }
        graph
            .edges
            .entry(name_of(&node.id)?)
            .or_default()
            .extend(to);
    }
    Ok(graph)
}

fn kind(kind: Option<&str>) -> Result<Kind, String> {
    match kind {
        None => Ok(Kind::Normal),
        Some("build") => Ok(Kind::Build),
        Some("dev") => Ok(Kind::Dev),
        Some(other) => Err(format!(
            "cargo metadata gave an unknown dependency kind {other:?}"
        )),
    }
}

/// Whether `name` is an engine crate's name: the front door, `shuyi`, or
/// `shuyi-*`.
pub fn is_engine(name: &str) -> bool {
    name == "shuyi" || name.starts_with("shuyi-")
}

/// Whether `name` is one of `devices` (a trailing `*` is a prefix).
fn is_device(name: &str, devices: &[String]) -> bool {
    devices.iter().any(|device| match device.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == device,
    })
}

/// Every error the layers find in the graph. `lib_rs` gives an engine
/// crate's `src/lib.rs`, or `None` when it can't be read.
pub fn check(
    layers: &Layers,
    graph: &Graph,
    lib_rs: impl Fn(&str) -> Option<String>,
) -> Vec<String> {
    let engine: BTreeSet<&str> = graph
        .members
        .keys()
        .map(String::as_str)
        .filter(|name| is_engine(name))
        .collect();
    let mut errors = Vec::new();
    for name in &engine {
        if !layers.crates.contains_key(*name) {
            errors.push(format!(
                "{name} has no entry in {LAYERS}: add one (its `engine`, `outside`, `devices` \
                 and `floats`) in this PR ({ADR})"
            ));
        }
    }
    for (name, rule) in &layers.crates {
        let Some(member) = graph.members.get(name).filter(|_| is_engine(name)) else {
            errors.push(format!(
                "{LAYERS} has an entry for {name}, which is no engine crate of the workspace: \
                 remove it, or fix its name"
            ));
            continue;
        };
        for listed in &rule.engine {
            if !engine.contains(listed.as_str()) {
                errors.push(format!(
                    "{name}'s `engine` list in {LAYERS} names {listed}, which is no engine crate \
                     of the workspace: remove it, or fix its name"
                ));
            }
        }
        check_dependencies(name, rule, member, &engine, &mut errors);
        if !rule.devices {
            let devices: Vec<String> = layers
                .devices
                .iter()
                .filter(|device| !rule.readers.contains(device))
                .cloned()
                .collect();
            for chain in device_chains(name, &devices, &graph.edges) {
                let device = chain.last().map_or("", String::as_str);
                errors.push(format!(
                    "{name} reaches the device crate {device} ({}), and only a platform backend \
                     may (its `devices` is false in {LAYERS}, ADR-0008 §2): remove the \
                     dependency that brings it in",
                    chain.join(" → ")
                ));
            }
        }
        if !rule.floats {
            match lib_rs(name) {
                Some(text) if denies_floats(&text) => {}
                Some(_) => errors.push(format!(
                    "{name}'s src/lib.rs does not deny `clippy::float_arithmetic`, and its \
                     `floats` is false in {LAYERS} (a simulation, ADR-0002 §2): add \
                     `#![cfg_attr(not(test), {FLOAT_BAN})]` beside its other `#![...]` lines"
                )),
                None => errors.push(format!(
                    "{name}'s src/lib.rs can't be read, and its `floats` is false in {LAYERS}: \
                     the float rule needs it"
                )),
            }
        }
    }
    errors
}

/// Rules 2 and 3: `member`'s normal and build dependencies against its
/// `engine` and `outside` lists.
fn check_dependencies(
    name: &str,
    rule: &CrateRule,
    member: &Member,
    engine: &BTreeSet<&str>,
    errors: &mut Vec<String>,
) {
    let used: BTreeSet<&str> = member
        .deps
        .iter()
        .filter(|dep| dep.kind != Kind::Dev)
        .map(|dep| dep.name.as_str())
        .collect();
    for dep in &used {
        if engine.contains(dep) {
            if !rule.engine.iter().any(|allowed| allowed == dep) {
                errors.push(format!(
                    "{name} depends on {dep}, which is not on its `engine` list in {LAYERS}: \
                     engine crates depend only down the layers (ADR-0002 §1); remove the \
                     dependency, or make it a dev-dependency if only tests need it"
                ));
            }
        } else if let Outside::Only(allowed) = &rule.outside
            && !allowed.iter().any(|allowed| allowed == dep)
        {
            errors.push(format!(
                "{name} uses {dep}, which is not on its `outside` list in {LAYERS}: add it there \
                 with a comment saying why, in this PR ({ADR})"
            ));
        }
    }
    if let Outside::Only(allowed) = &rule.outside {
        for listed in allowed {
            if !used.contains(listed.as_str()) {
                errors.push(format!(
                    "{name}'s `outside` list in {LAYERS} names {listed}, which {name} no longer \
                     uses: remove it from the list"
                ));
            }
        }
    }
}

/// The way from `start` to each device crate it reaches first, over
/// `edges`: the shortest, from `start` to the device. A device's own
/// dependencies aren't followed.
fn device_chains(
    start: &str,
    devices: &[String],
    edges: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<Vec<String>> {
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    let mut queue = VecDeque::from([start]);
    let mut chains = Vec::new();
    while let Some(at) = queue.pop_front() {
        for next in edges.get(at).into_iter().flatten() {
            if next == start || parent.contains_key(next.as_str()) {
                continue;
            }
            parent.insert(next, at);
            if is_device(next, devices) {
                let mut chain = vec![next.clone()];
                let mut back = at;
                while back != start {
                    chain.push(back.to_string());
                    back = parent[back];
                }
                chain.push(start.to_string());
                chain.reverse();
                chains.push(chain);
            } else {
                queue.push_back(next);
            }
        }
    }
    chains
}

/// Whether `lib_rs` has a line that starts with `#![` and denies
/// `clippy::float_arithmetic`.
fn denies_floats(lib_rs: &str) -> bool {
    lib_rs
        .lines()
        .any(|line| line.starts_with("#![") && line.contains(FLOAT_BAN))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(engine: &[&str], outside: Option<&[&str]>, devices: bool, floats: bool) -> CrateRule {
        CrateRule {
            engine: engine.iter().map(ToString::to_string).collect(),
            outside: outside.map_or(Outside::Any, |only| {
                Outside::Only(only.iter().map(ToString::to_string).collect())
            }),
            devices,
            readers: Vec::new(),
            floats,
        }
    }

    fn layers(crates: Vec<(&str, CrateRule)>) -> Layers {
        Layers {
            devices: vec!["macroquad".into(), "quad-*".into()],
            crates: crates
                .into_iter()
                .map(|(name, rule)| (name.to_string(), rule))
                .collect(),
        }
    }

    fn member(deps: &[(&str, Kind)]) -> Member {
        Member {
            deps: deps
                .iter()
                .map(|&(name, kind)| Declared {
                    name: name.into(),
                    kind,
                })
                .collect(),
            ..Member::default()
        }
    }

    fn graph(members: Vec<(&str, Member)>, edges: &[(&str, &str)]) -> Graph {
        let mut graph = Graph {
            members: members
                .into_iter()
                .map(|(name, member)| (name.to_string(), member))
                .collect(),
            ..Graph::default()
        };
        for (from, to) in edges {
            graph
                .edges
                .entry((*from).to_string())
                .or_default()
                .insert((*to).to_string());
        }
        graph
    }

    /// Every `src/lib.rs` denies floats.
    fn banned() -> impl Fn(&str) -> Option<String> {
        |_| Some("#![cfg_attr(not(test), deny(clippy::float_arithmetic))]\n".into())
    }

    /// A core, a ui over it and an outside crate, all as the layers say.
    fn clean() -> (Layers, Graph) {
        let layers = layers(vec![
            ("shuyi-core", rule(&[], Some(&["serde"]), false, false)),
            ("shuyi-ui", rule(&["shuyi-core"], None, false, true)),
        ]);
        let graph = graph(
            vec![
                ("shuyi-core", member(&[("serde", Kind::Normal)])),
                (
                    "shuyi-ui",
                    member(&[("shuyi-core", Kind::Normal), ("foo", Kind::Build)]),
                ),
                ("xtask", member(&[("anything", Kind::Normal)])),
            ],
            &[
                ("shuyi-core", "serde"),
                ("shuyi-ui", "shuyi-core"),
                ("shuyi-ui", "foo"),
            ],
        );
        (layers, graph)
    }

    #[test]
    fn a_graph_as_the_layers_say_passes() {
        let (layers, graph) = clean();
        assert_eq!(check(&layers, &graph, banned()), Vec::<String>::new());
    }

    #[test]
    fn an_engine_crate_with_no_entry_fails() {
        let (layers, mut graph) = clean();
        graph.members.insert("shuyi-net".into(), member(&[]));
        assert_eq!(
            check(&layers, &graph, banned()),
            [
                "shuyi-net has no entry in tools/layers.ron: add one (its `engine`, `outside`, \
              `devices` and `floats`) in this PR (ADR-0032)"
            ]
        );
    }

    #[test]
    fn an_entry_or_an_engine_name_that_is_no_engine_crate_fails() {
        let (mut layers, graph) = clean();
        layers
            .crates
            .insert("shuyi-gone".into(), rule(&[], None, true, true));
        layers
            .crates
            .insert("xtask".into(), rule(&[], None, true, true));
        layers
            .crates
            .get_mut("shuyi-ui")
            .unwrap()
            .engine
            .push("shuyi-old".into());
        assert_eq!(
            check(&layers, &graph, banned()),
            [
                "tools/layers.ron has an entry for shuyi-gone, which is no engine crate of the \
                 workspace: remove it, or fix its name",
                "shuyi-ui's `engine` list in tools/layers.ron names shuyi-old, which is no \
                 engine crate of the workspace: remove it, or fix its name",
                "tools/layers.ron has an entry for xtask, which is no engine crate of the \
                 workspace: remove it, or fix its name",
            ]
        );
    }

    #[test]
    fn a_normal_or_build_dependency_up_the_layers_fails() {
        for kind in [Kind::Normal, Kind::Build] {
            let (layers, mut graph) = clean();
            graph
                .members
                .get_mut("shuyi-core")
                .unwrap()
                .deps
                .push(Declared {
                    name: "shuyi-ui".into(),
                    kind,
                });
            assert_eq!(
                check(&layers, &graph, banned()),
                [
                    "shuyi-core depends on shuyi-ui, which is not on its `engine` list in \
                  tools/layers.ron: engine crates depend only down the layers (ADR-0002 §1); \
                  remove the dependency, or make it a dev-dependency if only tests need it"
                ],
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_dev_dependency_on_any_engine_crate_is_free() {
        let (layers, mut graph) = clean();
        let core = graph.members.get_mut("shuyi-core").unwrap();
        core.deps.push(Declared {
            name: "shuyi-ui".into(),
            kind: Kind::Dev,
        });
        core.deps.push(Declared {
            name: "proptest".into(),
            kind: Kind::Dev,
        });
        assert_eq!(check(&layers, &graph, banned()), Vec::<String>::new());
    }

    #[test]
    fn an_outside_crate_not_on_an_only_list_fails() {
        for kind in [Kind::Normal, Kind::Build] {
            let (layers, mut graph) = clean();
            graph
                .members
                .get_mut("shuyi-core")
                .unwrap()
                .deps
                .push(Declared {
                    name: "hashbrown".into(),
                    kind,
                });
            assert_eq!(
                check(&layers, &graph, banned()),
                [
                    "shuyi-core uses hashbrown, which is not on its `outside` list in \
                  tools/layers.ron: add it there with a comment saying why, in this PR \
                  (ADR-0032)"
                ],
                "{kind:?}"
            );
        }
    }

    #[test]
    fn a_crate_on_an_only_list_no_longer_used_fails() {
        let (mut layers, graph) = clean();
        layers.crates.get_mut("shuyi-core").unwrap().outside =
            Outside::Only(vec!["serde".into(), "libm".into()]);
        assert_eq!(
            check(&layers, &graph, banned()),
            [
                "shuyi-core's `outside` list in tools/layers.ron names libm, which shuyi-core no \
              longer uses: remove it from the list"
            ]
        );
    }

    #[test]
    fn a_crate_used_only_by_tests_is_not_a_use() {
        let (layers, mut graph) = clean();
        let core = graph.members.get_mut("shuyi-core").unwrap();
        core.deps[0].kind = Kind::Dev;
        assert_eq!(
            check(&layers, &graph, banned()),
            [
                "shuyi-core's `outside` list in tools/layers.ron names serde, which shuyi-core no \
              longer uses: remove it from the list"
            ]
        );
    }

    #[test]
    fn a_device_below_a_crate_without_devices_fails_with_its_chain() {
        let (layers, mut graph) = clean();
        for (from, to) in [
            ("foo", "bar"),
            ("bar", "macroquad"),
            ("macroquad", "quad-snd"),
        ] {
            graph
                .edges
                .entry(from.into())
                .or_default()
                .insert(to.into());
        }
        assert_eq!(
            check(&layers, &graph, banned()),
            [
                "shuyi-ui reaches the device crate macroquad (shuyi-ui → foo → bar → macroquad), \
              and only a platform backend may (its `devices` is false in tools/layers.ron, \
              ADR-0008 §2): remove the dependency that brings it in"
            ]
        );
    }

    #[test]
    fn a_crate_with_devices_may_reach_them() {
        let (mut layers, mut graph) = clean();
        layers.crates.get_mut("shuyi-ui").unwrap().devices = true;
        graph
            .edges
            .entry("foo".into())
            .or_default()
            .insert("macroquad".into());
        assert_eq!(check(&layers, &graph, banned()), Vec::<String>::new());
    }

    /// A file reader its entry names is let through, and only that one.
    #[test]
    fn a_crate_may_reach_the_readers_its_entry_names() {
        let (mut layers, mut graph) = clean();
        layers.devices.push("hound".into());
        let edges = graph.edges.entry("foo".into()).or_default();
        edges.insert("hound".into());
        layers.crates.get_mut("shuyi-ui").unwrap().readers = vec!["hound".into()];
        assert_eq!(check(&layers, &graph, banned()), Vec::<String>::new());
        layers.crates.get_mut("shuyi-ui").unwrap().readers = vec!["lewton".into()];
        assert_eq!(check(&layers, &graph, banned()).len(), 1);
    }

    #[test]
    fn the_chain_is_the_shortest_and_each_device_is_named_once() {
        let edges = graph(
            vec![],
            &[
                ("a", "b"),
                ("a", "c"),
                ("b", "c"),
                ("c", "a"),
                ("c", "quad-snd"),
                ("b", "quad-snd"),
                ("b", "macroquad"),
            ],
        )
        .edges;
        let devices = layers(vec![]).devices;
        assert_eq!(
            device_chains("a", &devices, &edges),
            [vec!["a", "b", "macroquad"], vec!["a", "b", "quad-snd"]]
        );
        assert_eq!(
            device_chains("z", &devices, &edges),
            Vec::<Vec<String>>::new()
        );
    }

    #[test]
    fn a_trailing_star_is_a_prefix() {
        let devices = layers(vec![]).devices;
        assert!(is_device("quad-snd", &devices));
        assert!(is_device("macroquad", &devices));
        assert!(!is_device("quadtree", &devices));
        assert!(!is_device("macroquad-x", &devices));
        assert!(!is_device("my-quad-snd", &devices));
    }

    #[test]
    fn a_simulation_crate_must_deny_floats_in_its_lib_rs() {
        let (layers, graph) = clean();
        let lib_rs =
            |text: &'static str| move |name: &str| (name == "shuyi-core").then(|| text.to_string());
        let error = "shuyi-core's src/lib.rs does not deny `clippy::float_arithmetic`, and its \
                     `floats` is false in tools/layers.ron (a simulation, ADR-0002 §2): add \
                     `#![cfg_attr(not(test), deny(clippy::float_arithmetic))]` beside its other \
                     `#![...]` lines";
        for denied in [
            "//! Docs.\n#![forbid(unsafe_code)]\n#![deny(clippy::float_arithmetic)]\n",
            "#![cfg_attr(not(test), deny(clippy::float_arithmetic))]",
        ] {
            assert_eq!(check(&layers, &graph, lib_rs(denied)), Vec::<String>::new());
        }
        for not_denied in [
            "#![forbid(unsafe_code)]\n",
            "// #![deny(clippy::float_arithmetic)]\n",
            " #![deny(clippy::float_arithmetic)]\n",
            "#[deny(clippy::float_arithmetic)]\nfn f() {}\n",
            "#![warn(clippy::float_arithmetic)]\n",
        ] {
            assert_eq!(
                check(&layers, &graph, lib_rs(not_denied)),
                [error],
                "{not_denied:?}"
            );
        }
    }

    #[test]
    fn an_unreadable_lib_rs_of_a_simulation_crate_fails() {
        let (layers, graph) = clean();
        assert_eq!(
            check(&layers, &graph, |_| None),
            [
                "shuyi-core's src/lib.rs can't be read, and its `floats` is false in \
              tools/layers.ron: the float rule needs it"
            ]
        );
    }

    /// A small `cargo metadata` answer: two members, a renamed library, the
    /// three kinds and a per-target dependency.
    const METADATA: &str = r#"{
        "packages": [
            {"id": "path+file:///r/crates/core#shuyi-core@0.1.0", "name": "shuyi-core",
             "manifest_path": "/r/crates/core/Cargo.toml",
             "dependencies": [
                {"name": "serde", "kind": null, "target": null, "optional": false},
                {"name": "proptest", "kind": "dev", "target": null, "optional": false},
                {"name": "cc", "kind": "build", "target": null, "optional": false},
                {"name": "winapi", "kind": null, "target": "cfg(windows)", "optional": true}
             ]},
            {"id": "path+file:///r/crates/ui#shuyi-ui@0.1.0", "name": "shuyi-ui",
             "manifest_path": "/r/crates/ui/Cargo.toml",
             "dependencies": [
                {"name": "shuyi-core", "kind": null, "target": null, "optional": false}
             ]},
            {"id": "registry+x#serde@1.0.0", "name": "serde", "manifest_path": "/s/Cargo.toml", "dependencies": []},
            {"id": "registry+x#proptest@1.0.0", "name": "proptest", "manifest_path": "/p/Cargo.toml", "dependencies": []},
            {"id": "registry+x#cc@1.0.0", "name": "cc", "manifest_path": "/c/Cargo.toml", "dependencies": []},
            {"id": "registry+x#winapi@0.3.0", "name": "winapi", "manifest_path": "/w/Cargo.toml", "dependencies": []}
        ],
        "workspace_members": [
            "path+file:///r/crates/core#shuyi-core@0.1.0",
            "path+file:///r/crates/ui#shuyi-ui@0.1.0"
        ],
        "resolve": {"nodes": [
            {"id": "path+file:///r/crates/core#shuyi-core@0.1.0", "deps": [
                {"name": "serde", "pkg": "registry+x#serde@1.0.0", "dep_kinds": [{"kind": null, "target": null}]},
                {"name": "proptest", "pkg": "registry+x#proptest@1.0.0", "dep_kinds": [{"kind": "dev", "target": null}]},
                {"name": "cc", "pkg": "registry+x#cc@1.0.0", "dep_kinds": [{"kind": "build", "target": null}]},
                {"name": "winapi", "pkg": "registry+x#winapi@0.3.0", "dep_kinds": [{"kind": "dev", "target": null}, {"kind": null, "target": "cfg(windows)"}]}
            ]},
            {"id": "path+file:///r/crates/ui#shuyi-ui@0.1.0", "deps": [
                {"name": "shuyi_core", "pkg": "path+file:///r/crates/core#shuyi-core@0.1.0", "dep_kinds": [{"kind": null, "target": null}]}
            ]},
            {"id": "registry+x#serde@1.0.0", "deps": []}
        ]}
    }"#;

    #[test]
    fn cargo_metadata_is_read_by_package_name_and_kind() {
        let graph = parse_graph(METADATA).unwrap();
        assert_eq!(
            graph.members.keys().collect::<Vec<_>>(),
            ["shuyi-core", "shuyi-ui"]
        );
        let core = &graph.members["shuyi-core"];
        assert_eq!(core.dir, Path::new("/r/crates/core"));
        let deps: Vec<(&str, Kind)> = core
            .deps
            .iter()
            .map(|d| (d.name.as_str(), d.kind))
            .collect();
        assert_eq!(
            deps,
            [
                ("serde", Kind::Normal),
                ("proptest", Kind::Dev),
                ("cc", Kind::Build),
                ("winapi", Kind::Normal),
            ]
        );
        let edges: Vec<(&str, Vec<&str>)> = graph
            .edges
            .iter()
            .map(|(from, to)| (from.as_str(), to.iter().map(String::as_str).collect()))
            .collect();
        assert_eq!(
            edges,
            [
                ("serde", vec![]),
                ("shuyi-core", vec!["cc", "serde", "winapi"]),
                ("shuyi-ui", vec!["shuyi-core"]),
            ]
        );
    }

    #[test]
    fn broken_cargo_metadata_is_an_error() {
        assert!(
            parse_graph("not json")
                .unwrap_err()
                .starts_with("cargo metadata's output: ")
        );
        let unknown = METADATA.replace(r#""kind": "build""#, r#""kind": "other""#);
        assert_eq!(
            parse_graph(&unknown).unwrap_err(),
            "cargo metadata gave an unknown dependency kind \"other\""
        );
        let no_resolve = r#"{"packages": [], "workspace_members": [], "resolve": null}"#;
        assert_eq!(
            parse_graph(no_resolve).unwrap_err(),
            "cargo metadata gave no resolve of the dependencies"
        );
        let no_member = r#"{"packages": [], "workspace_members": ["x"], "resolve": null}"#;
        assert_eq!(
            parse_graph(no_member).unwrap_err(),
            "cargo metadata names no package x"
        );
        let no_package = METADATA.replace(
            r#""pkg": "registry+x#cc@1.0.0""#,
            r#""pkg": "registry+x#gone@1.0.0""#,
        );
        assert_eq!(
            parse_graph(&no_package).unwrap_err(),
            "cargo metadata names no package registry+x#gone@1.0.0"
        );
    }

    #[test]
    fn the_layers_file_parses() {
        let root = crate::repo_root();
        let layers = parse_layers(&fs::read_to_string(root.join(LAYERS)).unwrap()).unwrap();
        assert!(layers.devices.iter().any(|d| d == "quad-*"));
        let core = &layers.crates["shuyi-core"];
        assert!(core.engine.is_empty() && !core.devices && !core.floats);
        assert!(matches!(core.outside, Outside::Only(_)));
        assert!(matches!(layers.crates["shuyi-app"].outside, Outside::Any));
        let error = parse_layers("(devices: [], crates: {}, extra: 1)").unwrap_err();
        assert!(error.starts_with("tools/layers.ron: "), "{error}");
    }

    #[test]
    fn the_repository_passes() {
        assert_eq!(run(&crate::repo_root()), Vec::<String>::new());
    }

    #[test]
    fn a_missing_layers_file_and_a_folder_with_no_workspace_are_errors() {
        let root = std::env::temp_dir().join(format!("xtask-layers-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let errors = run(&root);
        let _ = fs::remove_dir_all(&root);
        assert_eq!(errors.len(), 2, "{errors:?}");
        assert!(errors[0].starts_with("tools/layers.ron: "), "{errors:?}");
        assert!(
            errors[1].starts_with("cargo metadata failed: "),
            "{errors:?}"
        );
    }

    #[test]
    fn a_new_games_rules_test_keeps_a_copy_of_the_devices() {
        let root = crate::repo_root();
        let layers = parse_layers(&fs::read_to_string(root.join(LAYERS)).unwrap()).unwrap();
        let copy = "templates/shuyi-default/stages/top-down-grid/crates/rules/tests/layers.rs";
        let text = fs::read_to_string(root.join(copy)).unwrap();
        for device in &layers.devices {
            assert!(
                text.contains(&format!("\"{device}\"")),
                "{copy} doesn't name the device crate {device} of {LAYERS}: copy it there"
            );
        }
    }
}
