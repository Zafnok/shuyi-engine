//! An export never ships the debug build's live link (ADR-0047): the game
//! package's own `link` feature, nor the engine's ([`ENGINE`]). Naming
//! `link` in `--features` is one way to turn it on; a package's `default`
//! list, a feature that lists it (`devtools = ["link"]`), and a dependency
//! that asks for `shuyi-app/link`, under any name it gives the engine, are
//! others. So before anything is prepared, baked or built, each build an
//! exporter plans ([`Exporter::cargo_args`]) is resolved by Cargo itself
//! (`cargo tree`, with that build's package, target and feature flags and
//! the Rust flags its exporter adds), and refused if the live link is on
//! in it ([`refuse`], ADR-0092).
//!
//! Only what the built program is made of counts: the normal and build
//! dependencies for that target. A dev-dependency (`shuyi-app` turns its
//! own `link` on in its tests), another member of the workspace, or a
//! dependency only for another target doesn't. A resolution that fails, or
//! whose answer can't be read, refuses the export: it never means the
//! build is safe.

use std::fmt::Write as _;
use std::process::Command;

use super::{ExportCx, ExportError, Exporter, paths};

/// The feature the live link is, in the game's package and the engine's.
pub const FEATURE: &str = "link";

/// The engine's packages whose [`FEATURE`] is the live link.
pub const ENGINE: [&str; 2] = ["shuyi-app", "shuyi-platform"];

/// `cargo tree`'s line for one package: its depth, its features and the
/// package (`name version (source)`), split by `|`, which neither a depth
/// nor a feature's name holds.
const FORMAT: &str = "|{f}|{p}";

/// Refuses `cx`'s export by `exporter` if a build it plans turns on the live
/// link: the literal arguments first ([`super::no_live_link`]), then the
/// build as Cargo resolves it.
pub fn refuse(cx: &ExportCx, exporter: &dyn Exporter) -> Result<(), ExportError> {
    for args in exporter.cargo_args(cx) {
        super::no_live_link(&args)?;
        let flags = target(&args).map_or(&[][..], |triple| exporter.rustflags(triple));
        let nodes = resolve(cx, &args, flags)?;
        let found = violations(&nodes);
        if !found.is_empty() {
            return Err(refusal(&args, &nodes, &found));
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

/// The arguments of `cargo tree` resolving the build `cargo <build>` runs:
/// its package, target and feature flags as they are, and what changes no
/// feature (the profile, the target kind) left out. A flag not known here
/// is refused rather than guessed at.
pub fn tree_args(build: &[String]) -> Result<Vec<String>, ExportError> {
    let mut rest = build.iter();
    match rest.next().map(String::as_str) {
        Some("build" | "rustc") => {}
        other => {
            return Err(ExportError::Setup(format!(
                "the live link's check reads `cargo build` or `cargo rustc`, not `cargo {}`",
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
                    "the live link's check doesn't know what the build's `{flag}` does to its \
                     features, so it can't tell whether the build turns the live link on"
                )));
            }
        }
    }
    args.extend(["-e", "normal,build", "--prefix", "depth", "-f", FORMAT].map(str::to_owned));
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

/// The live link turned on in a build: the package and how it was reached.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// The package whose [`FEATURE`] is on.
    pub package: String,
    /// The packages from the one built to it, the first way `cargo tree`
    /// shows.
    pub path: Vec<String>,
}

/// Where the live link is on in the build of `nodes` (the first, at depth
/// 0, is the package built): its own [`FEATURE`], and the engine's, each
/// package once.
pub fn violations(nodes: &[Node]) -> Vec<Violation> {
    let mut found: Vec<Violation> = Vec::new();
    let mut path: Vec<&str> = Vec::new();
    for node in nodes {
        path.truncate(node.depth);
        path.push(&node.name);
        let watched = node.depth == 0 || ENGINE.contains(&node.name.as_str());
        if watched
            && node.features.iter().any(|feature| feature == FEATURE)
            && !found.iter().any(|v| v.package == node.name)
        {
            found.push(Violation {
                package: node.name.clone(),
                path: path.iter().map(|&name| name.to_owned()).collect(),
            });
        }
    }
    found
}

/// The refusal of the build `cargo <args>`, resolved as `nodes`, which turns
/// the live link on where `found` says.
fn refusal(args: &[String], nodes: &[Node], found: &[Violation]) -> ExportError {
    let root = &nodes[0];
    let on_target = target(args).map_or_else(String::new, |triple| format!(" for {triple}"));
    let mut message = format!(
        "an export never turns on the debug build's live link, and Cargo's build of `{}`{on_target} \
         turns it on:",
        root.name
    );
    for violation in found {
        let _ = if violation.path.len() == 1 {
            write!(
                message,
                "\n  `{}/{FEATURE}`, among the package's features as built: {}",
                violation.package,
                root.features.join(", ")
            )
        } else {
            write!(
                message,
                "\n  `{}/{FEATURE}`, the engine's live link, reached through {}",
                violation.package,
                violation.path.join(" -> ")
            )
        };
    }
    message.push_str(
        "\nTake it out of what turns it on: the package's `default`, a feature that lists it, or \
         a dependency's `features`. `--without` leaves out only a feature `export.features` lists.",
    );
    ExportError::Setup(message)
}

/// The packages of the build `cargo <build>` in `cx`'s project, with the Rust
/// flags `rustflags` after the machine's, as `cargo tree` resolves them. A
/// `cfg` in those flags can choose a target's dependencies, so they are
/// handed over as the build hands them: in [`paths::ENCODED`] when it is a
/// release build or its exporter adds flags, else as Cargo finds them.
fn resolve(cx: &ExportCx, build: &[String], rustflags: &[&str]) -> Result<Vec<Node>, ExportError> {
    let args = tree_args(build)?;
    let mut cargo = Command::new(crate::cargo());
    cargo.args(&args).current_dir(&cx.root);
    if cx.release || !rustflags.is_empty() {
        let mut flags: Vec<String> = paths::remapped_flags_here(&[])
            .split('\x1f')
            .filter(|flag| !flag.is_empty())
            .map(str::to_owned)
            .collect();
        flags.extend(rustflags.iter().map(|&flag| flag.to_owned()));
        cargo.env(paths::ENCODED, flags.join("\x1f"));
    }
    let output = cargo.output().map_err(|e| {
        ExportError::MissingTool(format!(
            "cargo doesn't start, so the export can't tell whether its build turns on the \
                 live link: {e}"
        ))
    })?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(ExportError::Setup(format!(
            "cargo {} failed, so the export can't tell whether its build turns on the live \
             link: {}",
            args.join(" "),
            error.trim()
        )));
    }
    parse(&String::from_utf8_lossy(&output.stdout)).map_err(|e| {
        ExportError::Build(format!(
            "{e}, so the export can't tell whether its build turns on the live link"
        ))
    })
}

#[cfg(test)]
mod tests;
