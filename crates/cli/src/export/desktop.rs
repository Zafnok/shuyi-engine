//! The desktop exporters, which all check the `desktop` profile: `windows`
//! (`x86_64-pc-windows-gnu`, a zip), `linux` (`x86_64-unknown-linux-gnu`,
//! a `tar.gz`; CI builds it in the Steam Linux Runtime's sniper SDK, whose
//! glibc is the floor) and `macos` (one universal program, `lipo` of
//! `aarch64-apple-darwin` and `x86_64-apple-darwin`, in an ad-hoc signed
//! `.app`). Each builds on its own system: the linker, the system
//! libraries and the tools are that system's. What ships beside the program
//! goes beside it in the package; in a `.app`, a folder goes in
//! `Contents/Resources`, where the engine's backend looks for it and the
//! signature covers it, and a file beside the `.app`, where a player finds it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::disclosure::{FileKind, Shipped};
use super::identity::{self, Identity};
use super::{
    ExportCx, ExportError, ExportReport, Exporter, Host, InCi, RunCx, Signing, archive, fresh,
    tool, tool_in, write,
};
use crate::contract::target::{Level, TargetCheck, TargetProblem};
use crate::game::GameTools;
use crate::notices;
use crate::project::Project;

/// The Windows exporter.
pub static WINDOWS: Desktop = Desktop {
    id: "windows",
    system: System::Windows,
    triples: &["x86_64-pc-windows-gnu"],
};

/// The Linux exporter.
pub static LINUX: Desktop = Desktop {
    id: "linux",
    system: System::Linux,
    triples: &["x86_64-unknown-linux-gnu"],
};

/// The macOS exporter.
pub static MACOS: Desktop = Desktop {
    id: "macos",
    system: System::MacOs,
    triples: &["aarch64-apple-darwin", "x86_64-apple-darwin"],
};

/// The image CI builds the Linux package in: Valve's Steam Linux Runtime 3.0
/// ("sniper") SDK, whose glibc 2.31 (Debian 11's) is the package's floor
/// (ADR-0058). Its 3.0.20261007.263965, pinned by the digest the registry
/// gave on 2026-10-29 (`registry.gitlab.steamos.cloud/v2/steamrt/sniper/sdk/
/// manifests/<tag>`; amd64 only). The engine's CI and the templates'
/// workflows name the same.
pub const SNIPER: &str = "registry.gitlab.steamos.cloud/steamrt/sniper/sdk:3.0.20261007.263965@sha256:220d1dc739254155a91c010f959a4f8e7c183e69d18300d6be53a20aee12b02d";

/// The note for players a Windows package carries, `HOW-TO-OPEN.txt`.
pub const WINDOWS_NOTE: &str = "\
How to open this game on Windows
================================

This game isn't signed, so Windows SmartScreen may stop it the first
time with \"Windows protected your PC\". Choose More info, then Run
anyway. Windows asks only once.
";

/// The note for players a macOS package carries, `HOW-TO-OPEN.txt`.
pub const MACOS_NOTE: &str = "\
How to open this game on macOS
==============================

This game isn't signed by an Apple developer, so macOS stops it the
first time. On macOS 15 and later: open the app once and choose Done,
then open System Settings > Privacy & Security, scroll down, and choose
Open Anyway next to the game's name. macOS asks only once.
";

/// The note's file name.
pub const NOTE: &str = "HOW-TO-OPEN.txt";

/// The system a desktop package is built on and for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum System {
    /// Windows.
    Windows,
    /// Linux.
    Linux,
    /// macOS.
    MacOs,
}

impl System {
    /// The system this tool runs on, if a desktop one.
    fn here() -> Option<Self> {
        match std::env::consts::OS {
            "windows" => Some(Self::Windows),
            "linux" => Some(Self::Linux),
            "macos" => Some(Self::MacOs),
            _ => None,
        }
    }

    /// How to install binutils' `objcopy` there.
    fn objcopy_help(self) -> &'static str {
        match self {
            Self::Windows => {
                "it comes with MSYS2's mingw-w64 GCC: `C:\\msys64\\usr\\bin\\pacman.exe -S \
                 mingw-w64-x86_64-binutils`, with C:\\msys64\\mingw64\\bin on PATH"
            }
            _ => "install binutils with the system's package manager (`apt-get install binutils`)",
        }
    }
}

/// A desktop exporter.
pub struct Desktop {
    id: &'static str,
    system: System,
    triples: &'static [&'static str],
}

impl Exporter for Desktop {
    fn id(&self) -> &'static str {
        self.id
    }

    fn profile(&self) -> &'static str {
        "desktop"
    }

    /// Each builds on its own system.
    fn host(&self) -> Host {
        match self.system {
            System::Windows => Host::Windows,
            System::Linux => Host::Linux,
            System::MacOs => Host::Macos,
        }
    }

    /// Linux builds in the Steam Linux Runtime's SDK ([`SNIPER`]).
    fn container(&self) -> Option<&'static str> {
        (self.system == System::Linux).then_some(SNIPER)
    }

    fn ci(&self) -> InCi {
        InCi::Build
    }

    fn triples(&self) -> &'static [&'static str] {
        self.triples
    }

    fn cargo_args(&self, cx: &ExportCx) -> Vec<Vec<String>> {
        self.triples
            .iter()
            .map(|triple| cx.cargo_args(triple))
            .collect()
    }

    /// Bakes the pack a packed build reads, and proves it equal to its
    /// sources ([`ExportCx::bake_pack`]).
    fn prepare(
        &self,
        cx: &ExportCx,
        project: &Project,
        game: Option<&dyn GameTools>,
    ) -> Result<(), ExportError> {
        cx.bake_pack(project, game)
    }

    fn export(&self, cx: &ExportCx) -> Result<ExportReport, ExportError> {
        if System::here() != Some(self.system) {
            return Err(ExportError::MissingTool(format!(
                "the {} package is built on {} (its linker, libraries and tools): run this \
                 there, or let CI build it",
                self.id,
                self.name()
            )));
        }
        let mut report = match self.system {
            System::MacOs => {
                let slices = self
                    .triples
                    .iter()
                    .map(|triple| cx.build(triple, false, None))
                    .collect::<Result<Vec<_>, _>>()?;
                pack_macos(cx, &slices)
            }
            system => {
                let resource = if system == System::Windows {
                    windows_resource(cx)?
                } else {
                    None
                };
                let program = cx.build(self.triples[0], true, resource.as_deref())?;
                pack(cx, system, self.triples[0], &program)
            }
        }?;
        report.warnings.extend(no_identity(cx));
        Ok(report)
    }

    /// The program, its notices and the players' note, all written here, and
    /// on macOS the bundle's files; the debug file beside them.
    fn shipped(&self, cx: &ExportCx, report: &ExportReport) -> Shipped {
        let tree = cx.out.join(&report.folder);
        let archive = (report.package != report.folder).then(|| cx.out.join(&report.package));
        let mut shipped = Shipped::new(tree, archive, format!("{}/", report.folder));
        shipped.debug = report.debug.as_ref().map(|debug| cx.out.join(debug));
        shipped.source(notices::FILE, FileKind::Written, "written by the export");
        let program = match self.system {
            System::Windows => format!("{}.exe", cx.bin),
            System::Linux => {
                let share = cx.out.join(&report.folder).join(SHARE);
                let entries = archive::entries(&share, &format!("{SHARE}/"), &|_| false);
                for entry in entries.unwrap_or_default() {
                    shipped.source(entry.name, FileKind::Written, "written by the export");
                }
                cx.bin.clone()
            }
            System::MacOs => {
                let contents = format!("{}/Contents", app_folder(cx));
                let mut files = vec!["Info.plist", "_CodeSignature/CodeResources"];
                let icns = cx.out.join(&report.folder).join(&contents).join(ICNS);
                if icns.is_file() {
                    files.push(ICNS);
                }
                for file in files {
                    let path = format!("{contents}/{file}");
                    shipped.source(path, FileKind::Written, "written by the export");
                }
                format!("{contents}/MacOS/{}", cx.bin)
            }
        };
        shipped.source(program, FileKind::Program, "the built program");
        if self.system != System::Linux {
            shipped.source(NOTE, FileKind::Written, "written by the export");
        }
        cx.mark_beside(report, &mut shipped);
        cx.mark_pack(report, &mut shipped);
        shipped
    }

    fn overlays(&self) -> bool {
        true
    }

    /// The zip or `tar.gz` made again from the folder; on macOS the `.app`
    /// signed again, there.
    fn repackage(&self, cx: &ExportCx, report: &ExportReport) -> Result<(), ExportError> {
        let folder = cx.out.join(&report.folder);
        if self.system == System::MacOs {
            if System::here() != Some(System::MacOs) {
                return Err(ExportError::MissingTool(
                    "a recipient's macOS copy is signed again on macOS (`codesign`)".to_owned(),
                ));
            }
            let app = folder.join(app_folder(cx));
            let program = app.join("Contents").join("MacOS").join(&cx.bin);
            return sign_app(&program, &app);
        }
        let name = if self.system == System::Windows {
            format!("{}.exe", cx.bin)
        } else {
            cx.bin.clone()
        };
        archive_folder(
            self.system,
            &folder,
            &report.folder,
            std::ffi::OsStr::new(&name),
            &cx.out.join(&report.package),
        )
    }

    /// A release build's: Windows' and Linux' copy is stripped by
    /// `objcopy`, and macOS' is built stripped.
    fn strips_symbols(&self, cx: &ExportCx) -> bool {
        cx.release
    }

    fn run(&self, cx: &RunCx) -> Result<(), ExportError> {
        let program = cx.report.sizes.get(1).map(|size| cx.out.join(&size.file));
        let (Some(program), Some(_)) = (program, System::here().filter(|s| *s == self.system))
        else {
            return Err(ExportError::NoRunStep(format!(
                "no run step for {} on this system yet",
                self.id
            )));
        };
        // Its output goes to stderr, so `--json`'s answer stays alone on
        // stdout.
        let status = Command::new(&program)
            .current_dir(cx.out.join(&cx.report.folder))
            .stdout(std::io::stderr())
            .status()
            .map_err(|e| ExportError::Build(format!("start {}: {e}", program.display())))?;
        if status.success() {
            Ok(())
        } else {
            Err(ExportError::Build(format!(
                "{} exited with {status}",
                program.display()
            )))
        }
    }
}

impl Desktop {
    /// Its system's name.
    fn name(&self) -> &'static str {
        match self.system {
            System::Windows => "Windows",
            System::Linux => "Linux",
            System::MacOs => "macOS",
        }
    }
}

/// Writes the Windows or Linux package of `program` (built for `triple`):
/// the folder `<base>/` with the program, its pack if it is packed,
/// `NOTICES.txt` and on Windows the players' note, then its zip or
/// `tar.gz`. In a release build the program keeps its symbols and line
/// tables in `<base>.debug`, and the package's copy is stripped of both
/// (`objcopy --strip-all`).
pub fn pack(
    cx: &ExportCx,
    system: System,
    triple: &str,
    program: &Path,
) -> Result<ExportReport, ExportError> {
    let base = cx.base();
    let folder = cx.out.join(&base);
    fresh(&folder)?;
    let name = program.file_name().unwrap_or_default();
    let shipped = folder.join(name);
    let debug = if cx.release {
        let debug = format!("{base}.debug");
        copy(program, &cx.out.join(&debug))?;
        tool(
            "objcopy",
            &[Path::new("--strip-all"), program, &shipped],
            system.objcopy_help(),
        )?;
        Some(debug)
    } else {
        copy(program, &shipped)?;
        None
    };
    cx.search(&shipped)?;
    let pack = cx.place_pack(&folder)?;
    cx.native_notices(triple, &folder)?;
    if system == System::Windows {
        write(&folder.join(NOTE), WINDOWS_NOTE)?;
    }
    if system == System::Linux
        && let Some(part) = &cx.app
    {
        let identity = Identity::of(part, &cx.root, &cx.version);
        let files = identity::linux_files(&identity, &cx.bin)
            .map_err(|e| ExportError::Setup(identity::icon_error(&e)))?;
        for (path, bytes) in files {
            write_bytes(&folder.join(path), &bytes)?;
        }
    }
    let beside = cx.copy_beside(&|_| folder.clone())?;
    let package = if system == System::Windows {
        format!("{base}.zip")
    } else {
        format!("{base}.tar.gz")
    };
    let written = cx.out.join(&package);
    archive_folder(system, &folder, &base, name, &written)?;
    let sizes = vec![cx.size(&written)?, cx.size(&shipped)?];
    let mut report = cx.report(&package, sizes, Signing::None, debug);
    report.beside = beside;
    report.pack = pack;
    Ok(report)
}

/// Writes the macOS package of the program's `slices`: the folder
/// `<base>/` with the `.app` ([`app_folder`]: the slices joined by `lipo`,
/// its `Info.plist` and its icon from the game's identity, its pack in
/// `Contents/Resources` if it is packed, signed ad hoc), `NOTICES.txt` and
/// the players' note.
fn pack_macos(cx: &ExportCx, slices: &[std::path::PathBuf]) -> Result<ExportReport, ExportError> {
    let base = cx.base();
    let folder = cx.out.join(&base);
    fresh(&folder)?;
    let app = folder.join(app_folder(cx));
    let contents = app.join("Contents");
    let macos = contents.join("MacOS");
    fs::create_dir_all(&macos)
        .map_err(|e| ExportError::Build(format!("create {}: {e}", macos.display())))?;
    let program = macos.join(&cx.bin);
    let mut lipo: Vec<&Path> = vec![Path::new("-create"), Path::new("-output"), &program];
    lipo.extend(slices.iter().map(std::path::PathBuf::as_path));
    let xcode = "install Xcode's command line tools: `xcode-select --install`";
    tool("lipo", &lipo, xcode)?;
    cx.search(&program)?;
    let warnings = bundle_identity(cx, &contents)?;
    cx.native_notices(MACOS.triples[0], &folder)?;
    write(&folder.join(NOTE), MACOS_NOTE)?;
    let resources = contents.join("Resources");
    let pack = cx.place_pack(&resources)?;
    let beside = cx.copy_beside(&|is_dir| {
        if is_dir {
            resources.clone()
        } else {
            folder.clone()
        }
    })?;
    sign_app(&program, &app)?;
    let sizes = vec![cx.size(&folder)?, cx.size(&program)?];
    let mut report = cx.report(&base, sizes, Signing::AdHoc, None);
    report.beside = beside;
    report.pack = pack;
    report.warnings = warnings;
    Ok(report)
}

/// Writes the `.app`'s `Info.plist` into its `contents` folder, and with an
/// icon its `Resources/AppIcon.icns`, before the signature covers them; and
/// returns the warning of an icon under 1024 pixels, whose `.icns` lacks its
/// biggest entry.
pub(super) fn bundle_identity(
    cx: &ExportCx,
    contents: &Path,
) -> Result<Vec<TargetProblem>, ExportError> {
    let Some(part) = &cx.app else {
        write(
            &contents.join("Info.plist"),
            &identity::info_plist(&cx.bin, None, false),
        )?;
        return Ok(Vec::new());
    };
    let identity = Identity::of(part, &cx.root, &cx.version);
    let short = identity.short_version().map_err(ExportError::Setup)?;
    let icons = identity::icons(&identity, &identity::icns_sides())
        .map_err(|e| ExportError::Setup(identity::icon_error(&e)))?;
    let mut warnings = Vec::new();
    if !icons.is_empty() {
        write_bytes(&contents.join(ICNS), &identity::icns(&icons))?;
        if !icons.iter().any(|(side, _)| *side == 1024) {
            warnings.push(app_warning(
                "the icon is under 1024 pixels a side, so the .app's AppIcon.icns has no \
                 1024-pixel entry (ic10), which a Retina screen's biggest icon uses"
                    .to_owned(),
            ));
        }
    }
    let plist = identity::info_plist(&cx.bin, Some((&identity, &short)), !icons.is_empty());
    write(&contents.join("Info.plist"), &plist)?;
    Ok(warnings)
}

/// The folder of the `.app`, in the macOS package's: named after the game's
/// name ([`Identity::app_folder`]), else after its program.
pub fn app_folder(cx: &ExportCx) -> String {
    cx.app.as_ref().map_or_else(
        || format!("{}.app", cx.bin),
        |part| Identity::of(part, &cx.root, &cx.version).app_folder(),
    )
}

/// The `.icns` in a `.app`'s `Contents`.
const ICNS: &str = "Resources/AppIcon.icns";

/// The folder of the Linux package's desktop entry and icons, for
/// `~/.local/share`.
const SHARE: &str = "share";

/// The warning of an export of a project without an `app` part, whose
/// packages carry no identity.
pub(super) fn no_identity(cx: &ExportCx) -> Option<TargetProblem> {
    cx.app
        .is_none()
        .then(|| app_warning(identity::NONE.to_owned()))
}

/// A warning of the export about the game's `app` part.
fn app_warning(message: String) -> TargetProblem {
    TargetProblem::display(Level::Warning, TargetCheck::App, "app".to_owned(), message)
}

/// binutils' `windres` for the Windows GNU target, by its prefixed name.
const WINDRES_PREFIXED: &str = "x86_64-w64-mingw32-windres";

/// The `windres` to run: `x86_64-w64-mingw32-windres` if `found` says it
/// runs, else `windres`.
pub fn windres(found: &dyn Fn(&str) -> bool) -> &'static str {
    if found(WINDRES_PREFIXED) {
        WINDRES_PREFIXED
    } else {
        "windres"
    }
}

/// Whether `program --version` runs here and succeeds.
fn runs(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// `windres`' arguments, run in the export's work folder: `app.rc`, UTF-8,
/// to the COFF object `app.o`.
pub const WINDRES_ARGS: [&str; 7] = [
    "--codepage=65001",
    "-O",
    "coff",
    "-i",
    "app.rc",
    "-o",
    "app.o",
];

/// How to install `windres` on `system`.
pub fn windres_help(system: Option<System>) -> &'static str {
    if system == Some(System::Windows) {
        "it comes with MSYS2's mingw-w64 GCC: `C:\\msys64\\usr\\bin\\pacman.exe -S \
         mingw-w64-x86_64-gcc`, with C:\\msys64\\mingw64\\bin on PATH"
    } else {
        "install mingw-w64's binutils with the system's package manager (`apt-get install \
         binutils-mingw-w64-x86-64`)"
    }
}

/// The game's icon and version resource for its Windows program, compiled
/// into the COFF object the build links in: `app.ico` and `app.rc` written in
/// the export's work folder ([`ExportCx::work`]), never in the package, and
/// `windres` run there. `None` without an `app` part. A version number or
/// build over 65535 refuses the export.
pub(super) fn windows_resource(cx: &ExportCx) -> Result<Option<PathBuf>, ExportError> {
    let Some(part) = &cx.app else {
        return Ok(None);
    };
    let identity = Identity::of(part, &cx.root, &cx.version);
    let numbers = identity.numbers().map_err(ExportError::Setup)?;
    let icons = identity::icons(&identity, &identity::ICO_SIDES)
        .map_err(|e| ExportError::Setup(identity::icon_error(&e)))?;
    let work = cx.work();
    fresh(&work)?;
    if !icons.is_empty() {
        write_bytes(&work.join("app.ico"), &identity::ico(&icons))?;
    }
    let rc = identity::rc(&identity, &cx.bin, numbers, !icons.is_empty());
    write(&work.join("app.rc"), &rc)?;
    let args: Vec<&Path> = WINDRES_ARGS.iter().map(Path::new).collect();
    tool_in(
        Some(&work),
        windres(&runs),
        &args,
        windres_help(System::here()),
    )?;
    Ok(Some(work.join("app.o")))
}

/// Writes `bytes` to `path`, making its folder.
fn write_bytes(path: &Path, bytes: &[u8]) -> Result<(), ExportError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| ExportError::Build(format!("create {}: {e}", parent.display())))?;
    }
    fs::write(path, bytes).map_err(|e| ExportError::Build(format!("write {}: {e}", path.display())))
}

/// Writes the zip (Windows) or `tar.gz` of `folder`, named `<base>/` in it,
/// to `written`, its program `name` executable.
fn archive_folder(
    system: System,
    folder: &Path,
    base: &str,
    name: &std::ffi::OsStr,
    written: &Path,
) -> Result<(), ExportError> {
    let entries = archive::entries(folder, &format!("{base}/"), &|path| {
        path.file_name() == Some(name)
    })
    .map_err(ExportError::Build)?;
    if system == System::Windows {
        archive::zip(written, &entries)
    } else {
        archive::tar_gz(written, &entries)
    }
    .map_err(ExportError::Build)
}

/// Signs the `.app` `app` whose program is `program`, ad hoc: every file of
/// the bundle is written, so the nested code first, then the bundle (not
/// `--deep`).
fn sign_app(program: &Path, app: &Path) -> Result<(), ExportError> {
    let xcode = "install Xcode's command line tools: `xcode-select --install`";
    for code in [program, app] {
        tool(
            "codesign",
            &[
                Path::new("--force"),
                Path::new("--sign"),
                Path::new("-"),
                code,
            ],
            xcode,
        )?;
    }
    Ok(())
}

/// Copies `from` to `to`.
fn copy(from: &Path, to: &Path) -> Result<(), ExportError> {
    fs::copy(from, to).map(|_| ()).map_err(|e| {
        ExportError::Build(format!("copy {} to {}: {e}", from.display(), to.display()))
    })
}
