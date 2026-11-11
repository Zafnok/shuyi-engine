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
//! remapped, and the built binary is searched for them ([`paths`]). It
//! ships no symbol table: on Windows and Linux it keeps its symbols and line
//! tables in `<package>.debug`, beside the package and never in it, and the
//! package's copy is stripped of both with `objcopy --strip-all` (which moves
//! no code, so a crash's offsets name the same lines); on macOS and the web
//! the release profile strips them (`strip = true`).
//!
//! `shuyi.ron`'s `export` part names the features a build turns on and the
//! files and folders copied into the package ([`ExportCx::copy_beside`]);
//! its `targets` give one target a package and features of its own, the
//! only part of it the Playdate reads. `--with` turns on one more feature
//! the package has for one build. A game with no such part gets a build with
//! no feature and a package with nothing else in it.
//!
//! No build turns on a debug feature ([`DEBUG_FEATURES`]), the package's
//! or the engine's ([`ENGINE_DEBUG_FEATURES`]), however Cargo would reach
//! it: by the arguments, the package's `default` list, a feature that lists
//! it, or a dependency asking for the engine's. Each build is resolved by
//! Cargo and refused before anything is prepared or built ([`live_link`]).
//!
//! `shuyi.ron`'s `editions` name what one build of the game is
//! (`export --edition public-demo`), on any target: its package and
//! program, whether it keeps the package's default features (always said),
//! its features, the folder of files its program embeds, what ships beside it
//! and what its packages may hold, each in place of what the target would
//! build ([`EditionBuild`], ADR-0100). Its content root reaches the build as
//! [`CONTENT`], and a build without one has that variable taken away. Its
//! `never_features` are refused in the build as Cargo resolves it, as the
//! debug features are.
//!
//! Every final package is then read back and written down, and held to the
//! game's `disclosure` part, or its edition's, if it has one ([`disclosure`]).

pub mod archive;
mod desktop;
pub mod disclosure;
pub mod live_link;
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
use crate::project::{
    AppPart, Beside, Config, Edition, ExportPart, GameProgram, Project, WasmOptRule, Web,
};

/// The variable an export of an edition with a content root sets for its
/// builds: the folder's absolute path, which the game's build script hands to
/// what embeds its files (`include_dir!`). A build of no such edition runs
/// without it, whatever the shell has.
pub const CONTENT: &str = "SHUYI_CONTENT";

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

    /// The Rust flags its build for `triple` adds to the machine's, which
    /// can choose a target's dependencies by a `cfg`: by default none.
    fn rustflags(&self, _triple: &str) -> &'static [&'static str] {
        &[]
    }

    /// Whether `shuyi.ron`'s `export` part applies to it.
    fn takes_export_part(&self) -> bool {
        true
    }

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

    /// Whether the program `export` shipped for `cx` has no symbol table:
    /// by default not said, so `false`.
    fn strips_symbols(&self, _cx: &ExportCx) -> bool {
        false
    }

    /// What a panic does in the program it ships: by default it unwinds.
    fn panic(&self) -> disclosure::Panic {
        disclosure::Panic::Unwind
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
    /// The game's package: the target's own in the `export` part, else the
    /// `export` part's, the `game` part's or the web build's.
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
    /// The features its build turns on: the target's list or the `export`
    /// part's, less `--without`, plus `--with`.
    pub turned_on: Vec<String>,
    /// What ships beside the program.
    pub beside: Vec<Beside>,
    /// Whether a release web build needs `wasm-opt`: the `export` part's.
    pub wasm_opt: WasmOptRule,
    /// The edition it builds, if `--edition` named one.
    pub edition: Option<EditionBuild>,
}

/// What an export takes of the edition it builds, beside the package, the
/// program, the features and what ships beside it, which are in
/// [`ExportCx`]'s own fields.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct EditionBuild {
    /// Its name in `shuyi.ron`'s `editions`.
    pub name: String,
    /// Whether its builds keep the package's default features.
    pub default_features: bool,
    /// The features its builds never have on: the package's own, or
    /// `package/feature`.
    pub never_features: Vec<String>,
    /// Its content root, from the project's root, written with `/`.
    pub content: Option<String>,
}

impl EditionBuild {
    /// The edition `name`, which keeps the package's default features or not.
    pub fn new(name: impl Into<String>, default_features: bool) -> Self {
        Self {
            name: name.into(),
            default_features,
            ..Self::default()
        }
    }
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
        let part = config
            .export
            .as_ref()
            .filter(|_| exporter.takes_export_part());
        let edition = edition_of(project, input.edition.as_deref())?;
        let mut choice = Choice::of(config.export.as_ref(), exporter)?;
        if let Some((name, edition)) = edition {
            choice = Choice::of_edition(name, edition, choice);
        }
        let named = part.and_then(|part| part.package.as_deref());
        let package = choice
            .package
            .clone()
            .or_else(|| package_of(exporter.id(), named, config.game.as_ref(), web))
            .ok_or_else(|| {
                ExportError::Setup(format!(
                    "{} names no package to build: its `game` part names the game's",
                    crate::project::FILE
                ))
            })?;
        let (package_version, bins, features) = metadata(root, &package)?;
        let features: Vec<String> = features.into_keys().collect();
        let version = version(config.app.as_ref(), package_version);
        let (bin, web) = program_and_web(exporter, web, edition, &package, &bins)?;
        let shown = input.out.clone().unwrap_or_else(|| match edition {
            Some((name, _)) => format!("dist/export/{name}/{}", exporter.id()),
            None => format!("dist/export/{}", exporter.id()),
        });
        let out = root.join(&shown);
        fs::create_dir_all(&out)
            .map_err(|e| ExportError::Build(format!("create {}: {e}", out.display())))?;
        let game = match &config.notices {
            Some(texts) => notices::game_texts(root, texts).map_err(ExportError::Setup)?,
            None => Vec::new(),
        };
        let turned_on = choice.turned_on(&input.without, &input.with, &package, &features)?;
        let beside = beside_of(root, exporter, part, edition, web.as_ref())?;
        let edition = edition
            .map(|(name, edition)| EditionBuild::of(root, name, edition, &turned_on))
            .transpose()?;
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
            title: web
                .as_ref()
                .map_or_else(|| bin.clone(), |web| web.title.clone()),
            bin,
            version,
            game,
            web,
            app: config.app.clone(),
            features,
            turned_on,
            beside,
            wasm_opt: config
                .export
                .as_ref()
                .map_or_else(WasmOptRule::default, |part| part.wasm_opt),
            edition,
        })
    }

    /// The package's name without its extension: `<program>-<version>-<target>`,
    /// or `<program>-<version>-<edition>-<target>` for an edition.
    pub fn base(&self) -> String {
        match &self.edition {
            Some(edition) => format!(
                "{}-{}-{}-{}",
                self.bin, self.version, edition.name, self.target
            ),
            None => format!("{}-{}-{}", self.bin, self.version, self.target),
        }
    }

    /// Whether its builds keep the package's default features: unless its
    /// edition says not.
    pub fn default_features(&self) -> bool {
        self.edition
            .as_ref()
            .is_none_or(|edition| edition.default_features)
    }

    /// The absolute path of its edition's content root, if it has one.
    pub fn content(&self) -> Option<PathBuf> {
        self.edition
            .as_ref()
            .and_then(|edition| edition.content.as_ref())
            .map(|content| self.root.join(content))
    }

    /// Hands `cargo` its edition's content root as [`CONTENT`], or takes the
    /// variable away if it has none.
    pub fn content_env(&self, cargo: &mut Command) {
        match self.content() {
            Some(content) => cargo.env(CONTENT, content),
            None => cargo.env_remove(CONTENT),
        };
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
        if !self.default_features() {
            args.push("--no-default-features".to_owned());
        }
        if !self.turned_on.is_empty() {
            args.extend(["--features".to_owned(), self.turned_on.join(",")]);
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
        no_debug_features(&args)?;
        let mut cargo = Command::new(crate::cargo());
        cargo.args(&args).current_dir(&self.root);
        self.content_env(&mut cargo);
        if self.release {
            let outside = paths::outside(&self.root, &self.workspace);
            let remaps = paths::remaps(&self.workspace, &self.target_dir, &self.package, &outside);
            cargo.env(paths::ENCODED, paths::remapped_flags_here(&remaps));
            if keep_debug {
                cargo.env("CARGO_PROFILE_RELEASE_STRIP", "false");
                cargo.env("CARGO_PROFILE_RELEASE_DEBUG", "line-tables-only");
            } else {
                // Whatever the game's own profile says, nothing ships its
                // symbol table.
                cargo.env("CARGO_PROFILE_RELEASE_STRIP", "true");
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
            beside: Vec::new(),
            edition: self.edition.as_ref().map(|edition| edition.name.clone()),
        }
    }

    /// Copies what ships beside the program, each into the folder `place`
    /// picks for it from whether it is a folder, under its name there, and
    /// returns what it copied. A name the package already holds is refused.
    pub fn copy_beside(
        &self,
        place: &dyn Fn(bool) -> PathBuf,
    ) -> Result<Vec<FileSize>, ExportError> {
        let mut sizes = Vec::new();
        for item in &self.beside {
            let from = self.root.join(&item.path);
            let is_dir = from.is_dir();
            let to = place(is_dir).join(beside_name(item)?);
            if to.exists() {
                return Err(ExportError::Setup(format!(
                    "`export.beside` ships `{}` as {}, which the package already holds",
                    item.path,
                    to.display()
                )));
            }
            if is_dir {
                crate::web::copy_tree(&from, &to, &|_| true).map_err(ExportError::Build)?;
            } else {
                if let Some(parent) = to.parent() {
                    fs::create_dir_all(parent).map_err(|e| {
                        ExportError::Build(format!("create {}: {e}", parent.display()))
                    })?;
                }
                fs::copy(&from, &to).map_err(|e| {
                    ExportError::Build(format!("copy {} to {}: {e}", item.path, to.display()))
                })?;
            }
            sizes.push(self.size(&to)?);
        }
        Ok(sizes)
    }

    /// Says in `shipped` that what `report` lists as shipped beside the
    /// program is the game's own, for the exposure report.
    pub fn mark_beside(&self, report: &ExportReport, shipped: &mut Shipped) {
        for size in &report.beside {
            let on_disk = self.out.join(&size.file);
            let prefix = format!("{}/", report.folder);
            let name = size.file.strip_prefix(&prefix).unwrap_or(&size.file);
            let source = format!("export.beside: {name}");
            if on_disk.is_dir() {
                let inner = format!("{name}/");
                for entry in archive::entries(&on_disk, &inner, &|_| false).unwrap_or_default() {
                    shipped.source(entry.name, disclosure::FileKind::LooseAsset, source.clone());
                }
            } else {
                shipped.source(name, disclosure::FileKind::LooseAsset, source);
            }
        }
    }

    /// Writes `NOTICES.txt` into `dir` for a native build for `triple`:
    /// the crates of its build as Cargo resolves it ([`Self::cargo_args`],
    /// [`notices::built_crates`]), the engine's files they compile in, and
    /// the game's own works.
    pub fn native_notices(&self, triple: &str, dir: &Path) -> Result<(), ExportError> {
        let encoded = live_link::encoded_flags(self.release, &[]);
        let mut works = notices::built_crates(
            &self.root,
            &self.package,
            triple,
            &self.cargo_args(triple),
            encoded.as_deref(),
        )
        .map_err(ExportError::Build)?;
        works.extend(notices::engine_files(Build::Native, &works));
        works.extend(self.game.iter().cloned());
        notices::write(dir, &self.title, &works).map_err(ExportError::Build)
    }
}

/// The package an export of target `id` builds: the web build's on `web`;
/// otherwise the `export` part's, the `game` part's or the web build's, the
/// first there is.
fn package_of(
    id: &str,
    named: Option<&str>,
    game: Option<&GameProgram>,
    web: Option<&Web>,
) -> Option<String> {
    let web_package = web.map(|web| web.package.clone());
    if id == "web" {
        return web_package;
    }
    named
        .map(str::to_owned)
        .or_else(|| game.map(|game| game.package.clone()))
        .or(web_package)
}

/// The package and program of the project at `root` that `config` says runs
/// the game on a desktop, as `export` builds it.
pub(crate) fn package_and_program(
    root: &Path,
    config: &Config,
) -> Result<(String, String), ExportError> {
    let named = config
        .export
        .as_ref()
        .and_then(|part| part.package.as_deref());
    let package = package_of("desktop", named, config.game.as_ref(), config.web.as_ref())
        .ok_or_else(|| {
            ExportError::Setup(format!(
                "{} names no package to run: its `game` part names the game's",
                crate::project::FILE
            ))
        })?;
    let (_, bins, _) = metadata(root, &package)?;
    let bin = program(&package, &bins)?;
    Ok((package, bin))
}

/// What `shuyi.ron`'s `export` part says one target builds.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Choice<'a> {
    /// The package its own entry names, in place of the one
    /// [`package_of`] picks.
    package: Option<String>,
    /// The features it turns on.
    features: &'a [String],
    /// Where they are listed, for a refusal: `export.features`,
    /// `export.targets.<id>.features` or `editions.<name>.features`.
    listed_in: String,
}

impl<'a> Choice<'a> {
    /// What the edition `name`, `edition`, builds in place of `target`'s
    /// choice: its package, else the target's, and its features.
    fn of_edition(name: &str, edition: &'a Edition, target: Self) -> Self {
        Self {
            package: edition.package.clone().or(target.package),
            features: &edition.features,
            listed_in: format!("`editions.{name}.features`"),
        }
    }

    /// What `part` says `exporter`'s target builds: its own entry's package
    /// and features, each when the entry names it, else the part's features
    /// if the target takes the part. An entry of a target with no exporter is
    /// refused.
    fn of(part: Option<&'a ExportPart>, exporter: &dyn Exporter) -> Result<Self, ExportError> {
        let id = exporter.id();
        let targets = part.map(|part| &part.targets);
        if let Some(unknown) = targets
            .into_iter()
            .flat_map(BTreeMap::keys)
            .find(|name| find(name).is_none())
        {
            return Err(ExportError::Setup(format!(
                "`export.targets` in {} names `{unknown}`, which has no exporter (the exporters: \
                 {})",
                crate::project::FILE,
                ids().join(", ")
            )));
        }
        let own = targets.and_then(|targets| targets.get(id));
        let (features, listed_in) = match (own.and_then(|own| own.features.as_deref()), part) {
            (Some(features), _) => (features, format!("`export.targets.{id}.features`")),
            (None, Some(part)) if exporter.takes_export_part() => {
                (&part.features[..], "`export.features`".to_owned())
            }
            _ => (&[][..], "`export.features`".to_owned()),
        };
        Ok(Self {
            package: own.and_then(|own| own.package.clone()),
            features,
            listed_in,
        })
    }

    /// The features a build of `package`, whose features are `features`,
    /// turns on: what the choice lists but `without`, then `with`. Leaving
    /// out one it doesn't list is refused, as is turning on one `package`
    /// lacks.
    fn turned_on(
        &self,
        without: &[String],
        with: &[String],
        package: &str,
        features: &[String],
    ) -> Result<Vec<String>, ExportError> {
        if let Some(unknown) = without.iter().find(|name| !self.features.contains(name)) {
            return Err(ExportError::Setup(format!(
                "--without {unknown}: {} in {} doesn't list it",
                self.listed_in,
                crate::project::FILE
            )));
        }
        if let Some(unknown) = with.iter().find(|name| !features.contains(name)) {
            return Err(ExportError::Setup(format!(
                "--with {unknown}: the package `{package}` has no such feature"
            )));
        }
        let mut on: Vec<String> = self
            .features
            .iter()
            .filter(|name| !without.contains(name))
            .cloned()
            .collect();
        for name in with {
            if !on.contains(name) {
                on.push(name.clone());
            }
        }
        Ok(on)
    }
}

/// What of `listed` is there: one that isn't refuses the export, unless it is
/// optional.
fn present(root: &Path, listed: &[Beside]) -> Result<Vec<Beside>, ExportError> {
    let mut found = Vec::new();
    for item in listed {
        beside_name(item)?;
        if root.join(&item.path).exists() {
            found.push(item.clone());
        } else if !item.optional {
            return Err(ExportError::Setup(format!(
                "`export.beside` in {} names `{}`, which isn't there",
                crate::project::FILE,
                item.path
            )));
        }
    }
    Ok(found)
}

/// The edition `name` of `project`'s `editions`, with its name, if `name` is
/// given: one it doesn't have refuses the export, naming those it has.
fn edition_of<'a>(
    project: &'a Project,
    name: Option<&str>,
) -> Result<Option<(&'a str, &'a Edition)>, ExportError> {
    let Some(name) = name else {
        return Ok(None);
    };
    let editions = &project.config.editions;
    editions
        .get_key_value(name)
        .map(|(name, edition)| Some((name.as_str(), edition)))
        .ok_or_else(|| {
            let has = if editions.is_empty() {
                "it has none".to_owned()
            } else {
                let names: Vec<&str> = editions.keys().map(String::as_str).collect();
                format!("it has {}", names.join(", "))
            };
            ExportError::Setup(format!(
                "--edition {name}: {}'s `editions` has no such edition ({has})",
                crate::project::FILE
            ))
        })
}

/// The program `exporter` ships of `package`, whose programs are `bins`, and
/// the web build's part `web` it builds with: the package itself for a
/// library, else the program `edition` names, else the web build's on the web
/// (when no edition changes its package), else the package's own. An
/// edition's web build builds its package and program.
fn program_and_web(
    exporter: &dyn Exporter,
    web: Option<&Web>,
    edition: Option<(&str, &Edition)>,
    package: &str,
    bins: &[String],
) -> Result<(String, Option<Web>), ExportError> {
    let on_web = exporter.id() == "web";
    let named = edition.and_then(|(_, edition)| edition.bin.as_deref());
    let bin = match (web, named) {
        _ if !exporter.builds_a_program() => package.to_owned(),
        (_, Some(bin)) => chosen(edition, package, bins, bin)?,
        (Some(web), None) if on_web && (edition.is_none() || web.package == package) => {
            web.bin.clone()
        }
        _ => program(package, bins)?,
    };
    let web = web.map(|web| {
        let mut web = web.clone();
        if on_web && edition.is_some() {
            package.clone_into(&mut web.package);
            web.bin.clone_from(&bin);
        }
        web
    });
    Ok((bin, web))
}

/// What ships beside the program `exporter` packages: `edition`'s list, else
/// `part`'s, those there, less what the web build `web` already ships; none
/// for a target that takes no `export` part.
fn beside_of(
    root: &Path,
    exporter: &dyn Exporter,
    part: Option<&ExportPart>,
    edition: Option<(&str, &Edition)>,
    web: Option<&Web>,
) -> Result<Vec<Beside>, ExportError> {
    if !exporter.takes_export_part() {
        return Ok(Vec::new());
    }
    let listed = match edition.and_then(|(_, edition)| edition.beside.as_deref()) {
        Some(beside) => beside,
        None => part.map_or(&[][..], |part| &part.beside),
    };
    let mut beside = present(root, listed)?;
    if let Some(web) = web.filter(|_| exporter.id() == "web") {
        beside.retain(|item| !in_web_build(item, web));
    }
    Ok(beside)
}

/// The program `bin` that `edition` names, which `package`, whose programs
/// are `bins`, must have.
fn chosen(
    edition: Option<(&str, &Edition)>,
    package: &str,
    bins: &[String],
    bin: &str,
) -> Result<String, ExportError> {
    if bins.iter().any(|name| name == bin) {
        return Ok(bin.to_owned());
    }
    let name = edition.map_or("", |(name, _)| name);
    Err(ExportError::Setup(format!(
        "`editions.{name}.bin` in {} names `{bin}`, but the package `{package}` has no such \
         program (`[[bin]]`)",
        crate::project::FILE
    )))
}

impl EditionBuild {
    /// What an export takes of the edition `name`, `edition`, of the project
    /// at `root`, whose build turns on `turned_on`: its content root must be a
    /// folder of the project, and none of its `never_features` may be turned
    /// on by name.
    fn of(
        root: &Path,
        name: &str,
        edition: &Edition,
        turned_on: &[String],
    ) -> Result<Self, ExportError> {
        if let Some(content) = &edition.content {
            crate::contract::bake::content_folder(root, content).map_err(|why| {
                ExportError::Setup(format!(
                    "`editions.{name}.content` in {}: {why}",
                    crate::project::FILE
                ))
            })?;
        }
        if let Some(both) = edition
            .never_features
            .iter()
            .find(|never| turned_on.contains(never))
        {
            return Err(ExportError::Setup(format!(
                "the edition `{name}` turns on `{both}`, which its `never_features` refuses"
            )));
        }
        Ok(Self {
            name: name.to_owned(),
            default_features: edition.default_features,
            never_features: edition.never_features.clone(),
            content: edition.content.clone(),
        })
    }
}

/// Whether the web build already ships `item`: its `music` or `voice` folder.
fn in_web_build(item: &Beside, web: &Web) -> bool {
    [&web.music, &web.voice]
        .into_iter()
        .any(|folder| folder.as_deref() == Some(item.path.as_str()))
}

/// The name `item` has in the package: one plain name, never a path.
fn beside_name(item: &Beside) -> Result<String, ExportError> {
    let name = item.name.as_deref().map_or_else(
        || Path::new(&item.path).file_name().and_then(|n| n.to_str()),
        Some,
    );
    name.filter(|name| {
        !name.is_empty() && !name.contains(['/', '\\']) && !matches!(*name, "." | "..")
    })
    .map(str::to_owned)
    .ok_or_else(|| {
        ExportError::Setup(format!(
            "`export.beside`'s `{}` has no plain name to ship under",
            item.path
        ))
    })
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

/// The features an export never turns on, each with what it is: the debug
/// build's surfaces, which a shipped game doesn't carry.
pub const DEBUG_FEATURES: [(&str, &str); 4] = [
    ("link", "the debug build's live link"),
    (
        "debug-tools",
        "the debug tools: the debug menu, `SHUYI_PERF` traces and session recording",
    ),
    ("capture", "the capture mode, for CI's pictures"),
    ("sound-probe", "the sound parity gates' probe"),
];

/// The engine's packages that have each of the [`DEBUG_FEATURES`], by the
/// same name: where, beside the package built, an export's build as Cargo
/// resolves it is checked for that feature ([`live_link::violations`]). A
/// test keeps it equal to the engine crates' `[features]`.
pub const ENGINE_DEBUG_FEATURES: [(&str, &[&str]); 4] = [
    ("link", &["shuyi-app", "shuyi-platform"]),
    ("debug-tools", &["shuyi-ui"]),
    ("capture", &["shuyi-app", "shuyi-platform"]),
    ("sound-probe", &["shuyi-app"]),
];

/// Refuses a build whose `args` turn on one of the [`DEBUG_FEATURES`], of
/// the game's package or a dependency's (`game/link`), naming it, by its
/// arguments alone: what the build turns on as Cargo resolves it is
/// [`live_link::refuse`]'s.
pub fn no_debug_features(args: &[String]) -> Result<(), ExportError> {
    let features = args
        .windows(2)
        .filter(|pair| pair[0] == "--features" || pair[0] == "-F")
        .flat_map(|pair| pair[1].split([',', ' ']))
        .chain(
            args.iter()
                .filter_map(|arg| arg.strip_prefix("--features=")),
        );
    for feature in features {
        let name = feature.rsplit('/').next();
        if let Some((_, what)) = DEBUG_FEATURES
            .iter()
            .find(|(debug, _)| Some(*debug) == name)
        {
            return Err(ExportError::Setup(format!(
                "an export never turns on `{feature}`, {what}"
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
