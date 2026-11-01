//! The exporters `export` runs ([`crate::contract::export`]): one per
//! target, each building the game with the target's Rust triple and
//! writing its package ([`Exporter`]). `windows` writes a zip, `linux` a
//! `tar.gz`, `macos` a folder holding a signed `.app`, `web` a folder and
//! its zip, and `playdate` a `.pdx` and its zip. Every package carries
//! `NOTICES.txt` ([`notices`]), and
//! the Windows and macOS ones a note for players on opening an unsigned
//! program (`HOW-TO-OPEN.txt`).
//!
//! A `--release` build has nothing of the build machine: its paths are
//! remapped, and the built binary is searched for them ([`paths`]). On
//! Windows and Linux it keeps its line tables in `<package>.debug`, beside
//! the package and never in it, and the package's copy is stripped of
//! them with `objcopy --strip-debug` (which moves no code, so a crash's
//! offsets name the same lines). No export turns on a feature: never the
//! debug build's live link (`link`), whatever the game lists.
//!
//! Every final package is then read back and written down, and held to the
//! game's `disclosure` part if it has one ([`disclosure`]).

pub mod archive;
mod desktop;
pub mod disclosure;
pub mod paths;
pub mod playdate;
mod web;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use self::disclosure::Shipped;
use crate::contract::codes::{self, Code};
use crate::contract::export::ExportInput;
pub use crate::contract::export::{ExportOutput as ExportReport, FileSize, Signing};
use crate::game::GameTools;
use crate::notices::{self, Build, Work};
use crate::project::{AppPart, Project, Web};

/// A target's exporter.
pub trait Exporter: Sync {
    /// The target's id: `shuyi export <id>`.
    fn id(&self) -> &'static str;

    /// The target profile its check runs with (`targets/<profile>.ron`).
    fn profile(&self) -> &'static str;

    /// The Rust triples it builds for.
    fn triples(&self) -> &'static [&'static str];

    /// Whether it packages a program of the game's package; otherwise it
    /// builds the package's library, and the package needs no `[[bin]]`.
    fn builds_a_program(&self) -> bool {
        true
    }

    /// The `cargo` arguments of each build it runs.
    fn cargo_args(&self, cx: &ExportCx) -> Vec<Vec<String>>;

    /// What it needs of the game's tools before it builds, run where they
    /// are at hand: by default nothing.
    fn prepare(
        &self,
        _cx: &ExportCx,
        _project: &Project,
        _game: Option<&dyn GameTools>,
    ) -> Result<(), ExportError> {
        Ok(())
    }

    /// Builds and writes the package into `cx.out`.
    fn export(&self, cx: &ExportCx) -> Result<ExportReport, ExportError>;

    /// What the export that answered `report` wrote and where, for the
    /// inspection after it. By default the folder and archive `report` names
    /// and no file known, so a target that doesn't say holds only undeclared
    /// files.
    fn shipped(&self, cx: &ExportCx, report: &ExportReport) -> Shipped {
        let archive = (report.package != report.folder).then(|| cx.out.join(&report.package));
        Shipped::new(
            cx.out.join(&report.folder),
            archive,
            format!("{}/", report.folder),
        )
    }

    /// Runs the package `export` wrote: by default there is no run step.
    fn run(&self, _cx: &RunCx) -> Result<(), ExportError> {
        Err(ExportError::NoRunStep(format!(
            "no run step for {} yet",
            self.id()
        )))
    }
}

/// The exporters, by target id.
static EXPORTERS: [&dyn Exporter; 5] = [
    &desktop::WINDOWS,
    &desktop::LINUX,
    &desktop::MACOS,
    &web::WEB,
    &playdate::PLAYDATE,
];

/// Every exporter.
pub fn exporters() -> &'static [&'static dyn Exporter] {
    &EXPORTERS
}

/// The exporter of target `id`.
pub fn find(id: &str) -> Option<&'static dyn Exporter> {
    EXPORTERS
        .iter()
        .copied()
        .find(|exporter| exporter.id() == id)
}

/// The targets with an exporter.
pub fn ids() -> Vec<&'static str> {
    EXPORTERS.iter().map(|exporter| exporter.id()).collect()
}

/// Why an export failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExportError {
    /// A tool is missing: the message says how to install it.
    MissingTool(String),
    /// `shuyi.ron` or the game's package lacks what the export needs.
    Setup(String),
    /// The build, or writing the package, failed.
    Build(String),
    /// The built binary holds a path of the build machine.
    BuildPath(String),
    /// The target has no run step yet.
    NoRunStep(String),
}

impl ExportError {
    /// Its problem code.
    pub fn code(&self) -> Code {
        match self {
            Self::MissingTool(_) => codes::TOOLCHAIN,
            Self::Setup(_) => codes::PROJECT_FILE,
            Self::Build(_) => codes::PACKAGE,
            Self::BuildPath(_) => codes::BUILD_PATH,
            Self::NoRunStep(_) => codes::NO_EXPORTER,
        }
    }

    /// What went wrong.
    pub fn message(&self) -> String {
        match self {
            Self::MissingTool(m)
            | Self::Setup(m)
            | Self::Build(m)
            | Self::BuildPath(m)
            | Self::NoRunStep(m) => m.clone(),
        }
    }
}

/// What an exporter works from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportCx {
    /// The target.
    pub target: String,
    /// Its profile.
    pub profile: String,
    /// The project's root.
    pub root: PathBuf,
    /// The root of the workspace it builds in.
    pub workspace: PathBuf,
    /// Cargo's target folder.
    pub target_dir: PathBuf,
    /// The export's folder, made.
    pub out: PathBuf,
    /// The export's folder as the game shows it.
    pub shown: String,
    /// Whether it is a release build.
    pub release: bool,
    /// The game's package.
    pub package: String,
    /// The program it builds.
    pub bin: String,
    /// The game's version: `shuyi.ron`'s `app.version` when it sets one,
    /// else the package's.
    pub version: String,
    /// The game's title, for `NOTICES.txt`.
    pub title: String,
    /// The game's own works for `NOTICES.txt`.
    pub game: Vec<Work>,
    /// The game's web build, if it has one.
    pub web: Option<Web>,
    /// Who the game is, if `shuyi.ron` says.
    pub app: Option<AppPart>,
    /// The cargo features of the game's package.
    pub features: Vec<String>,
}

/// What a run step works from.
pub struct RunCx<'a> {
    /// The export's folder.
    pub out: &'a Path,
    /// What the export made.
    pub report: &'a ExportReport,
}

/// A package's `[features]`: each feature and what it turns on.
type Features = BTreeMap<String, Vec<String>>;

/// `cargo metadata --no-deps`, the fields read here.
#[derive(Deserialize)]
struct Metadata {
    packages: Vec<MetaPackage>,
}

#[derive(Deserialize)]
struct MetaPackage {
    name: String,
    version: String,
    targets: Vec<MetaTarget>,
    features: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct MetaTarget {
    name: String,
    kind: Vec<String>,
}

impl ExportCx {
    /// The context of exporting `project` with `exporter` as `input`
    /// asks: the folder made, the game's package found with its version
    /// and program (`cargo metadata --no-deps`).
    pub fn of(
        project: &Project,
        exporter: &dyn Exporter,
        input: &ExportInput,
    ) -> Result<Self, ExportError> {
        let root = &project.root;
        let config = &project.config;
        let web = if exporter.id() == "web" {
            Some(
                project
                    .part(&config.web, "web")
                    .map_err(ExportError::Setup)?,
            )
        } else {
            config.web.as_ref()
        };
        let package = match (&config.game, web) {
            _ if exporter.id() == "web" => web.map(|web| web.package.clone()),
            (Some(game), _) => Some(game.package.clone()),
            (None, web) => web.map(|web| web.package.clone()),
        }
        .ok_or_else(|| {
            ExportError::Setup(format!(
                "{} names no package to build: its `game` part names the game's",
                crate::project::FILE
            ))
        })?;
        let (package_version, bins, features) = metadata(root, &package)?;
        let features = features.into_keys().collect();
        let version = version(config.app.as_ref(), package_version);
        let bin = match web {
            Some(web) if exporter.id() == "web" => web.bin.clone(),
            _ if !exporter.builds_a_program() => package.clone(),
            _ => program(&package, &bins)?,
        };
        let shown = input
            .out
            .clone()
            .unwrap_or_else(|| format!("dist/export/{}", exporter.id()));
        let out = root.join(&shown);
        fs::create_dir_all(&out)
            .map_err(|e| ExportError::Build(format!("create {}: {e}", out.display())))?;
        let game = match &config.notices {
            Some(texts) => notices::game_texts(root, texts).map_err(ExportError::Setup)?,
            None => Vec::new(),
        };
        Ok(Self {
            target: exporter.id().to_owned(),
            profile: exporter.profile().to_owned(),
            workspace: crate::web::workspace_root(root),
            target_dir: crate::web::target_dir(root),
            root: root.clone(),
            out,
            shown,
            release: input.release,
            package,
            title: web.map_or_else(|| bin.clone(), |web| web.title.clone()),
            bin,
            version,
            game,
            web: web.cloned(),
            app: config.app.clone(),
            features,
        })
    }

    /// The package's name without its extension:
    /// `<program>-<version>-<target>`.
    pub fn base(&self) -> String {
        format!("{}-{}-{}", self.bin, self.version, self.target)
    }

    /// `cargo build`'s arguments for the program and `triple`.
    pub fn cargo_args(&self, triple: &str) -> Vec<String> {
        let mut args: Vec<String> = ["build", "-p", &self.package, "--bin", &self.bin]
            .map(str::to_owned)
            .to_vec();
        args.extend(["--target".to_owned(), triple.to_owned()]);
        if self.release {
            args.push("--release".to_owned());
        }
        args
    }

    /// The path of the program `cargo` builds for `triple`.
    pub fn artifact(&self, triple: &str) -> PathBuf {
        let profile = if self.release { "release" } else { "debug" };
        let exe = if triple.contains("-windows-") {
            format!("{}.exe", self.bin)
        } else {
            self.bin.clone()
        };
        self.target_dir.join(triple).join(profile).join(exe)
    }

    /// The paths a release package must not hold.
    pub fn needles(&self) -> Vec<PathBuf> {
        paths::needles(&[&self.root, &self.workspace])
    }

    /// Refuses `file` if it holds a path of the build machine (a release
    /// build only).
    pub fn search(&self, file: &Path) -> Result<(), ExportError> {
        if !self.release {
            return Ok(());
        }
        paths::check_file(file, &self.needles()).map_err(ExportError::BuildPath)
    }

    /// Builds the program for `triple`, with its line tables kept and not
    /// stripped if `keep_debug` and it is a release build, and returns the
    /// program's path.
    pub fn build(&self, triple: &str, keep_debug: bool) -> Result<PathBuf, ExportError> {
        installed(triple)?;
        let args = self.cargo_args(triple);
        no_live_link(&args)?;
        let mut cargo = Command::new(crate::cargo());
        cargo.args(&args).current_dir(&self.root);
        if self.release {
            let outside = paths::outside(&self.root, &self.workspace);
            let remaps = paths::remaps(&self.workspace, &self.target_dir, &self.package, &outside);
            cargo.env(paths::ENCODED, paths::remapped_flags_here(&remaps));
            if keep_debug {
                cargo.env("CARGO_PROFILE_RELEASE_STRIP", "false");
                cargo.env("CARGO_PROFILE_RELEASE_DEBUG", "line-tables-only");
            }
        }
        let status = cargo
            .status()
            .map_err(|e| ExportError::MissingTool(format!("cargo doesn't start: {e}")))?;
        if !status.success() {
            return Err(ExportError::Build(format!(
                "cargo {} exited with {status}",
                args.join(" ")
            )));
        }
        Ok(self.artifact(triple))
    }

    /// `file`'s size, named from the export's folder.
    pub fn size(&self, file: &Path) -> Result<FileSize, ExportError> {
        let bytes = if file.is_dir() {
            folder_size(file)?
        } else {
            fs::metadata(file)
                .map_err(|e| ExportError::Build(format!("stat {}: {e}", file.display())))?
                .len()
        };
        let name = file.strip_prefix(&self.out).unwrap_or(file);
        Ok(FileSize {
            file: name.to_string_lossy().replace('\\', "/"),
            bytes,
        })
    }

    /// The report of a package `package` made from `folder` (both in the
    /// export's folder), with `sizes` and `signing`.
    pub fn report(
        &self,
        package: &str,
        sizes: Vec<FileSize>,
        signing: Signing,
        debug: Option<String>,
    ) -> ExportReport {
        ExportReport {
            target: self.target.clone(),
            profile: self.profile.clone(),
            version: self.version.clone(),
            out: self.shown.replace('\\', "/"),
            package: package.to_owned(),
            folder: self.base(),
            sizes,
            warnings: Vec::new(),
            signing,
            debug,
        }
    }

    /// Writes `NOTICES.txt` into `dir` for a native build for `triple`:
    /// the crates compiled in, the engine's files they compile in, and the
    /// game's own works.
    pub fn native_notices(&self, triple: &str, dir: &Path) -> Result<(), ExportError> {
        let mut works =
            notices::crates(&self.root, &self.package, triple, &[]).map_err(ExportError::Build)?;
        works.extend(notices::engine_files(Build::Native, &works));
        works.extend(self.game.iter().cloned());
        notices::write(dir, &self.title, &works).map_err(ExportError::Build)
    }
}

/// The game's version: `app`'s, when it sets one, else its package's,
/// `package`.
fn version(app: Option<&crate::project::AppPart>, package: String) -> String {
    app.and_then(|app| app.version.clone()).unwrap_or(package)
}

/// The version, the programs and the features of `package`, by `cargo
/// metadata --no-deps` in `root`.
fn metadata(root: &Path, package: &str) -> Result<(String, Vec<String>, Features), ExportError> {
    let output = Command::new(crate::cargo())
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|e| ExportError::MissingTool(format!("cargo doesn't start: {e}")))?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(ExportError::Setup(format!(
            "cargo metadata failed: {}",
            error.trim()
        )));
    }
    let meta: Metadata = serde_json::from_slice(&output.stdout)
        .map_err(|e| ExportError::Build(format!("read cargo metadata: {e}")))?;
    let found = meta
        .packages
        .into_iter()
        .find(|p| p.name == package)
        .ok_or_else(|| ExportError::Setup(format!("no package `{package}` in the workspace")))?;
    let bins = found
        .targets
        .into_iter()
        .filter(|t| t.kind.iter().any(|kind| kind == "bin"))
        .map(|t| t.name)
        .collect();
    Ok((found.version, bins, found.features))
}

/// The program of `package` to export among its `bins`: the one named as
/// the package, else its only one.
fn program(package: &str, bins: &[String]) -> Result<String, ExportError> {
    if bins.iter().any(|bin| bin == package) {
        return Ok(package.to_owned());
    }
    match bins {
        [one] => Ok(one.clone()),
        [] => Err(ExportError::Setup(format!(
            "the package `{package}` has no program (`[[bin]]`) to export"
        ))),
        _ => Err(ExportError::Setup(format!(
            "the package `{package}` has several programs ({}), and none is named `{package}`",
            bins.join(", ")
        ))),
    }
}

/// Refuses a build whose `args` turn on a feature named `link`, the debug
/// build's live link (ADR-0035).
pub fn no_live_link(args: &[String]) -> Result<(), ExportError> {
    let features = args
        .windows(2)
        .filter(|pair| pair[0] == "--features" || pair[0] == "-F")
        .flat_map(|pair| pair[1].split([',', ' ']))
        .chain(
            args.iter()
                .filter_map(|arg| arg.strip_prefix("--features=")),
        );
    for feature in features {
        if feature.rsplit('/').next() == Some("link") {
            return Err(ExportError::Setup(format!(
                "an export never turns on `{feature}`, the debug build's live link"
            )));
        }
    }
    Ok(())
}

/// Fails with how to add `triple` if rustup is here and doesn't have it.
fn installed(triple: &str) -> Result<(), ExportError> {
    let listed = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output();
    match listed {
        Ok(output) if output.status.success() => {
            let text = String::from_utf8_lossy(&output.stdout);
            if text.lines().any(|line| line.trim() == triple) {
                Ok(())
            } else {
                Err(missing_target(triple))
            }
        }
        // No rustup: cargo says what it lacks.
        _ => Ok(()),
    }
}

/// The error of a Rust target that isn't installed.
fn missing_target(triple: &str) -> ExportError {
    ExportError::MissingTool(format!(
        "the Rust target {triple} isn't installed: run `rustup target add {triple}`"
    ))
}

/// The time a packaging tool may write into a file, the archives' day
/// (1980-01-01): `objcopy` would stamp a Windows program with the time it ran,
/// and the same input is to give the same package.
const SOURCE_DATE_EPOCH: &str = "315532800";

/// Runs `program` with `args`, failing with `install` if it isn't there.
fn tool(program: &str, args: &[&Path], install: &str) -> Result<(), ExportError> {
    let mut command = Command::new(program);
    command
        .args(args)
        .env("SOURCE_DATE_EPOCH", SOURCE_DATE_EPOCH);
    let status = command.status().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ExportError::MissingTool(format!("{program} isn't installed: {install}"))
        } else {
            ExportError::Build(format!("{program} doesn't start: {e}"))
        }
    })?;
    if status.success() {
        Ok(())
    } else {
        let args: Vec<String> = args.iter().map(|a| a.display().to_string()).collect();
        Err(ExportError::Build(format!(
            "{program} {} exited with {status}",
            args.join(" ")
        )))
    }
}

/// `dir`, emptied: removed if it is there, then made.
fn fresh(dir: &Path) -> Result<(), ExportError> {
    if dir.exists() {
        fs::remove_dir_all(dir)
            .map_err(|e| ExportError::Build(format!("clear {}: {e}", dir.display())))?;
    }
    fs::create_dir_all(dir)
        .map_err(|e| ExportError::Build(format!("create {}: {e}", dir.display())))
}

/// Writes `text` to `path`.
fn write(path: &Path, text: &str) -> Result<(), ExportError> {
    fs::write(path, text).map_err(|e| ExportError::Build(format!("write {}: {e}", path.display())))
}

/// The bytes of the files under `dir`.
fn folder_size(dir: &Path) -> Result<u64, ExportError> {
    let entries = archive::entries(dir, "", &|_| false).map_err(ExportError::Build)?;
    entries
        .iter()
        .map(|entry| {
            fs::metadata(&entry.path)
                .map(|meta| meta.len())
                .map_err(|e| ExportError::Build(format!("stat {}: {e}", entry.path.display())))
        })
        .sum()
}

#[cfg(test)]
mod tests;
