//! `export <target> [--out <dir>] [--release] [--run] [--edition <name>]
//! [--without <feature>] [--with <feature>]`: the game, or one of its
//! editions ([`crate::export`]), checked
//! against the target (`check --target` with the target's profile: the
//! `desktop` one for `windows`, `linux` and `macos`), refused on any
//! error, then built and packaged by the target's exporter
//! ([`crate::export`]). The package goes in `--out` (from the game's
//! folder; `dist/export/<target>` without it) with `export-report.json`,
//! which holds this answer's result: the target, the version, the sizes,
//! the check's warnings, the signing and, with `--release`, the debug file
//! kept beside the package. An edition goes in `dist/export/<edition>/<target>`
//! without `--out`. The final package is read back into
//! `exposure-report.json`, and refused if it breaks the game's `disclosure`
//! part, or its edition's ([`crate::export::disclosure`]). With `--run`, the package is run
//! after that.
//!
//! Before the exporter prepares or builds anything, each build it plans is
//! resolved by Cargo, and the export refused if a debug feature is on in
//! one, or one of its edition's `never_features`, by its default features, a
//! feature that lists it or a dependency's ([`crate::export::live_link`]):
//! nothing is built or run.
//!
//! `export --plan [--tag <tag>]` builds nothing: it answers the game's
//! [`ExportPlan`], which a release workflow reads (ticket 3050). For each
//! target of `shuyi.ron`'s `targets` (sorted by id), each exporter that
//! checks it (in the order they are registered): the runner it builds on in
//! CI, the image it builds in, and whether CI builds it or only checks it.
//! A target no exporter builds is a limit, not an error. With `--tag`, the
//! answer fails unless the tag is `v` and the game's version.

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::check::{self, CheckInput};
use super::target::{Level, TargetProblem};
use super::{Answer, Op, codes};
use crate::export::{
    self as exporters, ExportCx, ExportError, Exporter, InCi, RunCx, disclosure, live_link,
};
use crate::game::GameTools;
use crate::project::{Config, Project};

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
    /// target's check runs, and the answer says it has no exporter yet. None
    /// with `plan`.
    #[serde(default)]
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
    /// Features of `shuyi.ron`'s `export.features` (or the target's own
    /// list, `export.targets.<target>.features`) to leave off this build (a
    /// build without the files a feature needs, say).
    #[serde(default)]
    pub without: Vec<String>,
    /// Features of the game's package to turn on for this build only (a
    /// measuring build, say): each one the package has.
    #[serde(default)]
    pub with: Vec<String>,
    /// The edition of `shuyi.ron`'s `editions` to build: its program,
    /// features and files in place of what the target would build. Without
    /// one, the game as `export` builds it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// Whether to answer the game's [`ExportPlan`] and build nothing: what
    /// each target's exporters build, and where, in CI. It takes no target
    /// and no other field but `tag`.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub plan: bool,
    /// With `plan`, the tag a release is made from: the answer fails unless
    /// it is `v` and the game's version (`v0.1.0`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// The runner a package builds on in CI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Host {
    /// Linux on x86-64 (`ubuntu-latest`).
    Linux,
    /// Linux on Arm (`ubuntu-24.04-arm`).
    LinuxArm64,
    /// Windows (`windows-latest`).
    Windows,
    /// macOS (`macos-latest`).
    Macos,
}

/// What CI does with a target's exporter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Ci {
    /// It builds the package.
    Build,
    /// It only checks the game against the target (`check --target`): the
    /// package is built elsewhere, and `why` says why.
    CheckOnly,
}

/// One exporter of the plan: what CI does with it, and where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PlannedExport {
    /// The exporter: `shuyi export <exporter>`.
    pub exporter: String,
    /// The target of `shuyi.ron`'s `targets` it checks (its profile).
    pub profile: String,
    /// The runner it builds on.
    pub host: Host,
    /// The image it builds in, if any (the Steam Linux Runtime's for
    /// `linux`).
    pub container: Option<String>,
    /// Whether CI builds it or only checks it.
    pub ci: Ci,
    /// Why CI only checks it, for `check-only`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why: Option<String>,
}

impl PlannedExport {
    /// What the plan says of `exporter`.
    pub fn of(exporter: &dyn Exporter) -> Self {
        let (ci, why) = match exporter.ci() {
            InCi::Build => (Ci::Build, None),
            InCi::CheckOnly(why) => (Ci::CheckOnly, Some(why.to_owned())),
        };
        Self {
            exporter: exporter.id().to_owned(),
            profile: exporter.profile().to_owned(),
            host: exporter.host(),
            container: exporter.container().map(str::to_owned),
            ci,
            why,
        }
    }
}

/// What `export --plan` answers: the packages a release of the game makes.
/// Each target of `shuyi.ron` no exporter builds is a limit of the answer.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportPlan {
    /// The version the packages carry, as `export-report.json` writes it:
    /// `shuyi.ron`'s `app.version` when it sets one, else the package's.
    pub version: String,
    /// Whether the version has a pre-release part (`1.2.0-beta.1`).
    pub prerelease: bool,
    /// Each exporter of each target, targets sorted by id and a target's
    /// exporters in the order they are registered.
    pub exports: Vec<PlannedExport>,
}

/// What `export` answers: a package's report, or, with `plan`, the plan.
/// Only its schema is read: [`export`] answers the one and [`plan`] the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ExportResult {
    /// What `export` made.
    Package(ExportOutput),
    /// What `export --plan` plans.
    Plan(ExportPlan),
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
    /// The edition of `shuyi.ron`'s `editions` it built, if any:
    /// `exposure-report.json` says what it was built from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edition: Option<String>,
    /// The pack of the game's files the program reads, from the export's
    /// folder with its size, if the build is packed (a release desktop or web
    /// export of a package with a `packed` feature): beside the program, or in
    /// a `.app`'s `Contents/Resources` (ADR-0110).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pack: Option<FileSize>,
}

/// `export` on `project`: the target's check, with the game's tools
/// `game` if they are at hand, then its exporter.
pub fn export(
    project: &Project,
    input: &ExportInput,
    game: Option<&dyn GameTools>,
) -> Answer<ExportOutput> {
    let op = Op::Export;
    if input.plan {
        let error = "`plan` answers with the plan, which `export::plan` gives";
        return Answer::failed(op, codes::WRONG_ARGUMENTS, error);
    }
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
    let edition = input
        .edition
        .as_ref()
        .and_then(|name| project.config.editions.get(name));
    let part = edition
        .and_then(|edition| edition.disclosure.as_ref())
        .or(project.config.disclosure.as_ref());
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
    let mut report = match build(&cx, exporter, project, game) {
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

/// The package `exporter` makes of `cx`: its builds refused first if one
/// turns on a debug feature ([`live_link::refuse`]), so a refused export
/// prepares, bakes and builds nothing; then what it prepares, then its
/// export.
fn build(
    cx: &ExportCx,
    exporter: &dyn Exporter,
    project: &Project,
    game: Option<&dyn GameTools>,
) -> Result<ExportOutput, ExportError> {
    live_link::refuse(cx, exporter)?;
    exporter.prepare(cx, project, game)?;
    exporter.export(cx)
}

/// An answer of `export` that failed with `error`.
fn failed<T>(error: &ExportError) -> Answer<T> {
    Answer::failed(Op::Export, error.code(), error.message())
}

/// `export --plan` on `project` ([`ExportPlan`]): the game's version, then
/// [`plan_of`] with every registered exporter. It builds nothing and runs
/// no check.
pub fn plan(project: &Project, input: &ExportInput) -> Answer<ExportPlan> {
    match exporters::version_of(project) {
        Ok(version) => plan_of(
            &project.config,
            &version,
            exporters::exporters(),
            input.tag.as_deref(),
        ),
        Err(e) => failed(&e),
    }
}

/// The plan of the game whose project file is `config` and whose version is
/// `version`, with `exporters`, checked against `tag` if one is given.
pub fn plan_of(
    config: &Config,
    version: &str,
    exporters: &[&dyn Exporter],
    tag: Option<&str>,
) -> Answer<ExportPlan> {
    let op = Op::Export;
    let prerelease = match semver::Version::parse(version) {
        Ok(parsed) => !parsed.pre.is_empty(),
        Err(e) => {
            let error = format!(
                "the game's version `{version}` isn't a semantic version (`1.2.0`, \
                 `1.2.0-beta.1`): {e}"
            );
            return Answer::failed(op, codes::PROJECT_FILE, error);
        }
    };
    if let Some(tag) = tag
        && tag.strip_prefix('v') != Some(version)
    {
        let error = format!(
            "the tag `{tag}` doesn't name the game's version `{version}`: its tag is `v{version}`"
        );
        let help = format!(
            "tag the version the game has (`git tag -a v{version} -m <notes>`), or set the \
             version to the tag's (`app.version` in {}, else the package's) and tag that commit",
            crate::project::FILE
        );
        return Answer::failed(op, codes::WRONG_ARGUMENTS, error).with_help(help);
    }
    let mut exports = Vec::new();
    let mut limits = Vec::new();
    for id in config.targets.keys() {
        let before = exports.len();
        exports.extend(
            exporters
                .iter()
                .filter(|exporter| exporter.profile() == id)
                .map(|exporter| PlannedExport::of(*exporter)),
        );
        if exports.len() == before {
            limits.push(format!(
                "`{id}` is checked (`check --target {id}`) but not packaged: no exporter builds \
                 it yet"
            ));
        }
    }
    let plan = ExportPlan {
        version: version.to_owned(),
        prerelease,
        exports,
    };
    Answer::new(op, true, plan, limits)
}

/// The plan, for people.
pub fn plan_text(plan: &ExportPlan) -> String {
    let pre = if plan.prerelease {
        " (a prerelease)"
    } else {
        ""
    };
    let mut text = format!("export --plan: version {}{pre}\n", plan.version);
    for planned in &plan.exports {
        let _ = write!(text, "  {} ({}): ", planned.exporter, planned.profile);
        match planned.ci {
            Ci::CheckOnly => {
                let why = planned.why.as_deref().unwrap_or_default();
                let _ = write!(text, "checked only: {why}");
            }
            Ci::Build => {
                let host = serde_json::to_value(planned.host).unwrap_or_default();
                let _ = write!(text, "built on {}", host.as_str().unwrap_or_default());
                if let Some(container) = &planned.container {
                    let _ = write!(text, " in {container}");
                }
            }
        }
        text.push('\n');
    }
    text
}

/// `export`'s arguments (`--json` taken out): the target, then
/// `--out <dir>`, `--release`, `--run` and `--edition <name>` in any order,
/// each once, and `--without <feature>` and `--with <feature>`, any number of
/// times.
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
            "--with" => {
                let feature = rest.next().ok_or("--with needs a feature")?;
                input.with.push(feature.clone());
            }
            "--edition" => {
                once(input.edition.is_some())?;
                let name = rest.next().ok_or("--edition needs an edition's name")?;
                input.edition = Some(name.clone());
            }
            "--plan" => input.plan = once(input.plan)?,
            "--tag" => {
                once(input.tag.is_some())?;
                let tag = rest.next().ok_or("--tag needs a tag (`v0.1.0`)")?;
                input.tag = Some(tag.clone());
            }
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            _ if target.is_none() => target = Some(arg.clone()),
            _ => return Err("one target".to_owned()),
        }
    }
    if input.plan {
        if target.is_some() {
            return Err("--plan takes no target: it plans every target of shuyi.ron".to_owned());
        }
        let alone = ExportInput {
            plan: true,
            tag: input.tag.clone(),
            ..ExportInput::default()
        };
        if input != alone {
            return Err("--plan builds nothing: it takes only --tag".to_owned());
        }
        return Ok(input);
    }
    if input.tag.is_some() {
        return Err("--tag goes with --plan".to_owned());
    }
    input.target = target.ok_or("a target: windows, linux, macos, web or playdate")?;
    Ok(input)
}

/// `export`'s usage, with `command` as the tool's name.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} export <windows | linux | macos | web | playdate> [--out <dir>] [--release] \
         [--run] [--edition <name>] [--without <feature>] [--with <feature>] [--json]\n       \
         {command} export --plan [--tag <tag>] [--json]"
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
    if let Some(pack) = &output.pack {
        let _ = writeln!(text, "  pack: {}: {} bytes", pack.file, pack.bytes);
    }
    if let Some(debug) = &output.debug {
        let _ = writeln!(text, "  debug file: {}", out.join(debug).display());
    }
    let signing = match output.signing {
        Signing::None => "none",
        Signing::AdHoc => "ad-hoc",
    };
    let _ = writeln!(text, "  signing: {signing}");
    if let Some(edition) = &output.edition {
        let _ = writeln!(text, "  edition: {edition}");
    }
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
