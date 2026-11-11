//! An export never ships a debug feature ([`super::DEBUG_FEATURES`],
//! ADR-0094): the live link (ADR-0047), the debug tools, the capture mode
//! or the sound probe, neither the game package's own nor the engine's
//! ([`super::ENGINE_DEBUG_FEATURES`]). Naming one in `--features` is one way
//! to turn it on; a package's `default` list, a feature that lists it
//! (`devtools = ["link"]`), and a dependency that asks for `shuyi-app/link`
//! or `shuyi-ui/debug-tools`, under any name it gives the engine, are
//! others. So before anything is prepared, baked or built, each build an
//! exporter plans ([`Exporter::cargo_args`]) is resolved by Cargo itself
//! (`cargo tree`, with that build's package, target and feature flags and
//! the Rust flags its exporter adds), and refused if a debug feature is on
//! in it ([`refuse`], ADR-0096). The module keeps the name of the live
//! link, the first feature it refused.
//!
//! An edition's `never_features` ([`super::EditionBuild`]) are refused the
//! same way, in the same resolution: a public demo whose package's
//! `default` list, a feature or a dependency turns on what only the full
//! game has is refused before it is built ([`unwanted`]).
//!
//! Only what the built program is made of counts: the normal and build
//! dependencies for that target. A dev-dependency (`shuyi-app` turns its
//! own `link` and `capture` on in its tests), another member of the
//! workspace, or a
//! dependency only for another target doesn't. A resolution that fails, or
//! whose answer can't be read, refuses the export: it never means the
//! build is safe.

use std::fmt::Write as _;
use std::process::Command;

use super::{DEBUG_FEATURES, ENGINE_DEBUG_FEATURES, ExportCx, ExportError, Exporter, paths};

/// `cargo tree`'s line for one package: its depth, its features and the
/// package (`name version (source)`), split by `|`, which neither a depth
/// nor a feature's name holds.
const FORMAT: &str = "|{f}|{p}";

/// Refuses `cx`'s export by `exporter` if a build it plans turns on a debug
/// feature: by the literal arguments first ([`super::no_debug_features`]),
/// then by the build as Cargo resolves it; or, in an edition's export, one of
/// its `never_features` ([`unwanted`]).
pub fn refuse(cx: &ExportCx, exporter: &dyn Exporter) -> Result<(), ExportError> {
    for args in exporter.cargo_args(cx) {
        super::no_debug_features(&args)?;
        let flags = target(&args).map_or(&[][..], |triple| exporter.rustflags(triple));
        let nodes = resolve(cx, &args, flags)?;
        let found = violations(&nodes);
        if !found.is_empty() {
            return Err(refusal(&args, &nodes, &found));
        }
        if let Some(edition) = &cx.edition {
            let found = unwanted(&nodes, &edition.never_features);
            if !found.is_empty() {
                return Err(edition_refusal(&edition.name, &args, &nodes, &found));
            }
        }
    }
    Ok(())
}

/// The value of `args`' `--target`.
fn target(args: &[String]) -> Option<&str> {
    args.windows(2)
        .find(|pair| pair[0] == "--target")
        .map(|pair| pair[1].as_str())
}

/// The arguments of `cargo tree` resolving the build `cargo <build>` runs
/// ([`build_flags`]): normal and build edges, in the check's format.
pub fn tree_args(build: &[String]) -> Result<Vec<String>, ExportError> {
    let mut args = build_flags(build)?;
    args.extend(["-e", "normal,build", "--prefix", "depth", "-f", FORMAT].map(str::to_owned));
    Ok(args)
}

/// `tree` and the flags of the build `cargo <build>` that choose what
/// `cargo tree` resolves: its package, target and feature flags as they
/// are, and what changes no feature (the profile, the target kind) left
/// out. A flag not known here is refused rather than guessed at.
/// `NOTICES.txt` lists the crates of the same resolution
/// ([`crate::notices::built_crates`]).
pub fn build_flags(build: &[String]) -> Result<Vec<String>, ExportError> {
    let mut rest = build.iter();
    match rest.next().map(String::as_str) {
        Some("build" | "rustc") => {}
        other => {
            return Err(ExportError::Setup(format!(
                "the debug features' check reads `cargo build` or `cargo rustc`, not `cargo {}`",
                other.unwrap_or_default()
            )));
        }
    }
    let mut args = vec!["tree".to_owned()];
    while let Some(arg) = rest.next() {
        match arg.as_str() {
            "-p" | "--package" | "--target" | "--features" | "-F" => {
                let value = rest.next().ok_or_else(|| {
                    ExportError::Setup(format!("the build's `{arg}` has no value"))
                })?;
                args.extend([arg.clone(), value.clone()]);
            }
            "--no-default-features" | "--all-features" => args.push(arg.clone()),
            flag if flag.starts_with("--features=") => args.push(arg.clone()),
            "--bin" | "--crate-type" => {
                rest.next();
            }
            "--lib" | "--release" => {}
            flag => {
                return Err(ExportError::Setup(format!(
                    "the debug features' check doesn't know what the build's `{flag}` does to \
                     its features, so it can't tell whether the build turns one on"
                )));
            }
        }
    }
    Ok(args)
}

/// One package of a resolved build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Its depth: 0 for the package built.
    pub depth: usize,
    /// Its name.
    pub name: String,
    /// The features the build turns on in it.
    pub features: Vec<String>,
}

/// The packages of `cargo tree`'s answer `text`, in the check's format with depths,
/// in its order. A line that isn't one, or an answer with no package at
/// depth 0, is an error.
pub fn parse(text: &str) -> Result<Vec<Node>, String> {
    let mut nodes = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let mut parts = line.splitn(3, '|');
        let (Some(depth), Some(features), Some(package)) =
            (parts.next(), parts.next(), parts.next())
        else {
            return Err(format!(
                "`cargo tree` wrote `{line}`, which isn't a package"
            ));
        };
        let depth = depth
            .trim()
            .parse()
            .map_err(|_| format!("`cargo tree` wrote `{line}`, which has no depth"))?;
        let name = package
            .split_whitespace()
            .next()
            .ok_or_else(|| format!("`cargo tree` wrote `{line}`, which names no package"))?;
        nodes.push(Node {
            depth,
            name: name.to_owned(),
            features: features
                .split(',')
                .filter(|feature| !feature.is_empty())
                .map(str::to_owned)
                .collect(),
        });
    }
    if nodes.first().is_none_or(|node| node.depth != 0) {
        return Err("`cargo tree` named no package built".to_owned());
    }
    Ok(nodes)
}

/// A debug feature turned on in a build: the package, the feature and how
/// it was reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The package whose debug feature is on.
    pub package: String,
    /// The feature, as [`DEBUG_FEATURES`] names it.
    pub feature: &'static str,
    /// The packages from the one built to it, the first way `cargo tree`
    /// shows.
    pub path: Vec<String>,
}

/// Whether `package`, at `depth` in a build, is watched for the debug
/// feature `feature`: the package built (depth 0) for each, an engine
/// package for those [`ENGINE_DEBUG_FEATURES`] gives it.
fn watched(package: &str, depth: usize, feature: &str) -> bool {
    depth == 0
        || ENGINE_DEBUG_FEATURES
            .iter()
            .any(|(name, engine)| *name == feature && engine.contains(&package))
}

/// Where a debug feature is on in the build of `nodes` (the first, at depth
/// 0, is the package built): the package's own [`DEBUG_FEATURES`] and the
/// engine's ([`ENGINE_DEBUG_FEATURES`]), each feature of each package once,
/// in the order of `nodes`, then of [`DEBUG_FEATURES`].
pub fn violations(nodes: &[Node]) -> Vec<Violation> {
    let mut found: Vec<Violation> = Vec::new();
    let mut path: Vec<&str> = Vec::new();
    for node in nodes {
        path.truncate(node.depth);
        path.push(&node.name);
        for (feature, _) in DEBUG_FEATURES {
            if watched(&node.name, node.depth, feature)
                && node.features.iter().any(|on| on == feature)
                && !found
                    .iter()
                    .any(|v| v.package == node.name && v.feature == feature)
            {
                found.push(Violation {
                    package: node.name.clone(),
                    feature,
                    path: path.iter().map(|&name| name.to_owned()).collect(),
                });
            }
        }
    }
    found
}

/// A feature an edition never has, turned on in its build: the package, the
/// feature, and the packages from the one built to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unwanted {
    /// The package whose feature is on.
    pub package: String,
    /// The feature, as `never_features` names it.
    pub feature: String,
    /// The packages from the one built to it, the first way `cargo tree`
    /// shows.
    pub path: Vec<String>,
}

/// Where a feature of `never` is on in the build of `nodes` (the first, at
/// depth 0, is the package built): a bare name is the built package's
/// feature, `package/feature` that package's anywhere in the build. Each
/// once, in the order of `nodes`, then of `never`.
pub fn unwanted(nodes: &[Node], never: &[String]) -> Vec<Unwanted> {
    let Some(built) = nodes.first() else {
        return Vec::new();
    };
    let mut found: Vec<Unwanted> = Vec::new();
    let mut path: Vec<&str> = Vec::new();
    for node in nodes {
        path.truncate(node.depth);
        path.push(&node.name);
        for name in never {
            let (package, feature) = name
                .split_once('/')
                .unwrap_or((built.name.as_str(), name.as_str()));
            if node.name == package
                && node.features.iter().any(|on| on == feature)
                && !found
                    .iter()
                    .any(|u| u.package == package && u.feature == feature)
            {
                found.push(Unwanted {
                    package: package.to_owned(),
                    feature: feature.to_owned(),
                    path: path.iter().map(|&name| name.to_owned()).collect(),
                });
            }
        }
    }
    found
}

/// The refusal of the edition `edition`'s build `cargo <args>`, resolved as
/// `nodes`, which turns on what its `never_features` lists, as `found` says.
fn edition_refusal(
    edition: &str,
    args: &[String],
    nodes: &[Node],
    found: &[Unwanted],
) -> ExportError {
    let root = &nodes[0];
    let on_target = target(args).map_or_else(String::new, |triple| format!(" for {triple}"));
    let mut message = format!(
        "the edition `{edition}` never turns on what its `never_features` lists, and Cargo's \
         build of `{}`{on_target} turns on:",
        root.name
    );
    for unwanted in found {
        let named = format!("`{}/{}`", unwanted.package, unwanted.feature);
        let _ = if unwanted.path.len() == 1 {
            write!(
                message,
                "\n  {named}, among the package's features as built: {}",
                root.features.join(", ")
            )
        } else {
            write!(
                message,
                "\n  {named}, reached through {}",
                unwanted.path.join(" -> ")
            )
        };
    }
    message.push_str(
        "\nTake each out of what turns it on: the package's `default` (`default_features: false` \
         leaves it off), a feature the edition lists, or a dependency's `features`.",
    );
    ExportError::Setup(message)
}

/// What the debug feature `feature` is, in [`DEBUG_FEATURES`]' words.
fn what(feature: &str) -> &'static str {
    DEBUG_FEATURES
        .iter()
        .find(|(name, _)| *name == feature)
        .map_or("a debug feature", |(_, what)| what)
}

/// The refusal of the build `cargo <args>`, resolved as `nodes`, which turns
/// debug features on where `found` says.
fn refusal(args: &[String], nodes: &[Node], found: &[Violation]) -> ExportError {
    let root = &nodes[0];
    let on_target = target(args).map_or_else(String::new, |triple| format!(" for {triple}"));
    let mut message = format!(
        "an export never turns on a debug feature, and Cargo's build of `{}`{on_target} turns \
         on:",
        root.name
    );
    for violation in found {
        let named = format!(
            "`{}/{}`, {}",
            violation.package,
            violation.feature,
            what(violation.feature)
        );
        let _ = if violation.path.len() == 1 {
            write!(
                message,
                "\n  {named}, among the package's features as built: {}",
                root.features.join(", ")
            )
        } else {
            write!(
                message,
                "\n  {named}, reached through {}",
                violation.path.join(" -> ")
            )
        };
    }
    message.push_str(
        "\nTake each out of what turns it on: the package's `default`, a feature that lists it, or \
         a dependency's `features`. `--without` leaves out only a feature `export.features` lists.",
    );
    ExportError::Setup(message)
}

/// The Rust flags a resolution of a build hands Cargo in [`paths::ENCODED`]:
/// the machine's, then `rustflags`, when it is a release build (`release`)
/// or its exporter adds flags; else none, and Cargo finds them as it does.
/// A `cfg` in them can choose a target's dependencies, so they are handed
/// over as the build hands them.
pub fn encoded_flags(release: bool, rustflags: &[&str]) -> Option<String> {
    if !release && rustflags.is_empty() {
        return None;
    }
    let mut flags: Vec<String> = paths::remapped_flags_here(&[])
        .split('\x1f')
        .filter(|flag| !flag.is_empty())
        .map(str::to_owned)
        .collect();
    flags.extend(rustflags.iter().map(|&flag| flag.to_owned()));
    Some(flags.join("\x1f"))
}

/// The packages of the build `cargo <build>` in `cx`'s project, with the Rust
/// flags `rustflags` after the machine's ([`encoded_flags`]), as `cargo tree`
/// resolves them.
fn resolve(cx: &ExportCx, build: &[String], rustflags: &[&str]) -> Result<Vec<Node>, ExportError> {
    let args = tree_args(build)?;
    let mut cargo = Command::new(crate::cargo());
    cargo.args(&args).current_dir(&cx.root);
    if let Some(flags) = encoded_flags(cx.release, rustflags) {
        cargo.env(paths::ENCODED, flags);
    }
    let output = cargo.output().map_err(|e| {
        ExportError::MissingTool(format!(
            "cargo doesn't start, so the export can't tell whether its build turns on a \
                 debug feature: {e}"
        ))
    })?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(ExportError::Setup(format!(
            "cargo {} failed, so the export can't tell whether its build turns on a debug \
             feature: {}",
            args.join(" "),
            error.trim()
        )));
    }
    parse(&String::from_utf8_lossy(&output.stdout)).map_err(|e| {
        ExportError::Build(format!(
            "{e}, so the export can't tell whether its build turns on a debug feature"
        ))
    })
}

#[cfg(test)]
mod tests;
