//! `export <target> [--out <dir>] [--release] [--run] [--without <feature>]`:
//! the game checked
//! against the target (`check --target` with the target's profile: the
//! `desktop` one for `windows`, `linux` and `macos`), refused on any
//! error, then built and packaged by the target's exporter
//! ([`crate::export`]). The package goes in `--out` (from the game's
//! folder; `dist/export/<target>` without it) with `export-report.json`,
//! which holds this answer's result: the target, the version, the sizes,
//! the check's warnings, the signing and, with `--release`, the debug file
//! kept beside the package. The final package is read back into
//! `exposure-report.json`, and refused if it breaks the game's `disclosure`
//! part ([`crate::export::disclosure`]). With `--run`, the package is run
//! after that.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::check::{self, CheckInput};
use super::target::{Level, TargetProblem};
use super::{Answer, Op, codes};
use crate::export::{self as exporters, ExportCx, ExportError, RunCx, disclosure};
use crate::game::GameTools;
use crate::project::Project;

/// The report's file, beside the package.
pub const REPORT: &str = "export-report.json";

/// What `export` makes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// An input: built with `default()` and its fields assigned, it may grow
// (ADR-0039).
#[non_exhaustive]
pub struct ExportInput {
    /// The target: `windows`, `linux`, `macos`, `web` or `playdate`. Any other
    /// target's check runs, and the answer says it has no exporter yet.
    pub target: String,
    /// The folder the package goes in, from the game's folder;
    /// `dist/export/<target>` without it.
    #[serde(default)]
    pub out: Option<String>,
    /// Whether to build with `--release`: the build's paths remapped and
    /// searched for, and on Windows and Linux the debug file kept beside
    /// the package.
    #[serde(default)]
    pub release: bool,
    /// Whether to run the package after writing it.
    #[serde(default)]
    pub run: bool,
    /// Features of `shuyi.ron`'s `export.features` to leave off this build
    /// (a build without the files a feature needs, say).
    #[serde(default)]
    pub without: Vec<String>,
}

/// How a package is signed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Signing {
    /// Not signed.
    #[default]
    None,
    /// Signed by no one (`codesign --sign -`): Apple silicon runs it, and
    /// the player opens it once with Open Anyway.
    AdHoc,
}

/// A file of the export and its size.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FileSize {
    /// The file, from the export's folder (`game-0.1.0-windows.zip`,
    /// `game-0.1.0-windows/game.exe`).
    pub file: String,
    /// Its size in bytes.
    pub bytes: u64,
}

/// What `export` made: `export-report.json`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportOutput {
    /// The target.
    pub target: String,
    /// The target profile its check ran with.
    pub profile: String,
    /// The game's version: its package's.
    pub version: String,
    /// The export's folder, as the game shows it.
    pub out: String,
    /// The package, from the export's folder: a zip, a `tar.gz` or a
    /// folder.
    pub package: String,
    /// The folder the package was made from, from the export's folder:
    /// what `--run` runs.
    pub folder: String,
    /// The package's size first (for `web`, its zip's), then each program
    /// in it.
    pub sizes: Vec<FileSize>,
    /// The check's warnings.
    pub warnings: Vec<TargetProblem>,
    /// How the package is signed.
    pub signing: Signing,
    /// With `--release` on Windows and Linux, the program with its debug
    /// sections, kept beside the package and never in it
    /// (`game-0.1.0-linux.debug`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<String>,
    /// What `shuyi.ron`'s `export.beside` copied into the package, from the
    /// export's folder, a folder with the bytes of all its files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub beside: Vec<FileSize>,
}

/// `export` on `project`: the target's check, with the game's tools
/// `game` if they are at hand, then its exporter.
pub fn export(
    project: &Project,
    input: &ExportInput,
    game: Option<&dyn GameTools>,
) -> Answer<ExportOutput> {
    let op = Op::Export;
    let exporter = exporters::find(&input.target);
    let profile = exporter.map_or(input.target.as_str(), |exporter| exporter.profile());
    let checked = check::targets(
        &project.config,
        &project.root,
        &CheckInput {
            target: Some(profile.to_owned()),
            ..CheckInput::default()
        },
        game,
    );
    let Some(found) = &checked.result else {
        let code = checked.code.as_deref().and_then(codes::entry);
        let error = checked.error.clone().unwrap_or_default();
        return Answer::failed(op, code.map_or(codes::TARGET, |entry| entry.code), error);
    };
    let problems: Vec<&TargetProblem> = found.targets.iter().flat_map(|t| &t.problems).collect();
    if !checked.ok {
        let mut error = format!("check --target {profile} found errors; nothing was built:");
        for problem in problems.iter().filter(|p| p.level == Level::Error) {
            let _ = write!(error, "\n  {}: {}", problem.setting, problem.message);
        }
        return Answer::failed(op, codes::TARGET_CHECK, error);
    }
    let Some(exporter) = exporter else {
        let error = format!(
            "{} passes check --target {profile}, but there is no exporter for it yet (the \
             exporters: {})",
            input.target,
            exporters::ids().join(", ")
        );
        return Answer::failed(op, codes::NO_EXPORTER, error);
    };
    let part = project.config.disclosure.as_ref();
    if let Some(part) = part {
        let problems = disclosure::problems(part);
        if !problems.is_empty() {
            let error = format!(
                "{} can't be used; nothing was built:\n  {}",
                crate::project::FILE,
                problems.join("\n  ")
            );
            return Answer::failed(op, codes::PROJECT_FILE, error);
        }
    }
    let cx = match ExportCx::of(project, exporter, input) {
        Ok(cx) => cx,
        Err(e) => return failed(&e),
    };
    if let Err(e) = exporter.prepare(&cx, project, game) {
        return failed(&e);
    }
    let mut report = match exporter.export(&cx) {
        Ok(report) => report,
        Err(e) => return failed(&e),
    };
    let shipped = exporter.shipped(&cx, &report);
    if let Err((code, error)) = disclosure::gate(&cx, exporter, part, &shipped) {
        return Answer::failed(op, code, error);
    }
    report.warnings.extend(
        problems
            .into_iter()
            .filter(|p| p.level == Level::Warning)
            .cloned(),
    );
    let mut answer = Answer::new(op, true, report, checked.limits);
    let json = serde_json::to_string_pretty(&answer.result).unwrap_or_default();
    if let Err(e) = fs::write(cx.out.join(REPORT), format!("{json}\n")) {
        let error = format!("write {}: {e}", cx.out.join(REPORT).display());
        return Answer::failed(op, codes::FILE, error);
    }
    if input.run
        && let Some(report) = &answer.result
        && let Err(e) = exporter.run(&RunCx {
            out: &cx.out,
            report,
        })
    {
        answer.ok = false;
        answer.error = Some(e.message());
        answer = answer.with_code(e.code());
    }
    answer
}

/// An answer of `export` that failed with `error`.
fn failed(error: &ExportError) -> Answer<ExportOutput> {
    Answer::failed(Op::Export, error.code(), error.message())
}

/// `export`'s arguments (`--json` taken out): the target, then
/// `--out <dir>`, `--release` and `--run` in any order, each once, and
/// `--without <feature>`, any number of times.
pub fn parse_args(args: &[String]) -> Result<ExportInput, String> {
    let mut input = ExportInput::default();
    let mut rest = args.iter();
    let mut target = None;
    while let Some(arg) = rest.next() {
        let once = |seen: bool| {
            if seen {
                Err(format!("{arg} twice"))
            } else {
                Ok(true)
            }
        };
        match arg.as_str() {
            "--release" => input.release = once(input.release)?,
            "--run" => input.run = once(input.run)?,
            "--out" => {
                once(input.out.is_some())?;
                let dir = rest.next().ok_or("--out needs a folder")?;
                input.out = Some(dir.clone());
            }
            "--without" => {
                let feature = rest.next().ok_or("--without needs a feature")?;
                input.without.push(feature.clone());
            }
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            _ if target.is_none() => target = Some(arg.clone()),
            _ => return Err("one target".to_owned()),
        }
    }
    input.target = target.ok_or("a target: windows, linux, macos, web or playdate")?;
    Ok(input)
}

/// `export`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} export <windows | linux | macos | web | playdate> [--out <dir>] [--release] \
         [--run] [--without <feature>] [--json]"
    )
}

/// What `export` made, for people.
pub fn text(output: &ExportOutput) -> String {
    let out = Path::new(&output.out);
    let mut text = format!(
        "export {}: {} ({}, check --target {})\n",
        output.target,
        out.join(&output.package).display(),
        output.version,
        output.profile
    );
    for size in &output.sizes {
        let _ = writeln!(text, "  {}: {} bytes", size.file, size.bytes);
    }
    for size in &output.beside {
        let _ = writeln!(text, "  beside: {}: {} bytes", size.file, size.bytes);
    }
    if let Some(debug) = &output.debug {
        let _ = writeln!(text, "  debug file: {}", out.join(debug).display());
    }
    let signing = match output.signing {
        Signing::None => "none",
        Signing::AdHoc => "ad-hoc",
    };
    let _ = writeln!(text, "  signing: {signing}");
    let _ = writeln!(text, "  report: {}", out.join(REPORT).display());
    let exposure = out.join(disclosure::REPORT);
    let _ = writeln!(text, "  exposure: {}", exposure.display());
    for warning in &output.warnings {
        let _ = writeln!(text, "  warning: {}: {}", warning.setting, warning.message);
    }
    text
}

#[cfg(test)]
mod tests;
