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
use std::path::Path;
use std::process::Command;

use super::{
    ExportCx, ExportError, ExportReport, Exporter, RunCx, Signing, archive, fresh, tool, write,
};

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

    fn triples(&self) -> &'static [&'static str] {
        self.triples
    }

    fn cargo_args(&self, cx: &ExportCx) -> Vec<Vec<String>> {
        self.triples
            .iter()
            .map(|triple| cx.cargo_args(triple))
            .collect()
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
        match self.system {
            System::MacOs => {
                let slices = self
                    .triples
                    .iter()
                    .map(|triple| cx.build(triple, false))
                    .collect::<Result<Vec<_>, _>>()?;
                pack_macos(cx, &slices)
            }
            system => {
                let program = cx.build(self.triples[0], true)?;
                pack(cx, system, self.triples[0], &program)
            }
        }
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
/// the folder `<base>/` with the program, `NOTICES.txt` and on Windows the
/// players' note, then its zip or `tar.gz`. In a release build the
/// program keeps its line tables in `<base>.debug`, and the package's copy
/// is stripped of them.
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
            &[Path::new("--strip-debug"), program, &shipped],
            system.objcopy_help(),
        )?;
        Some(debug)
    } else {
        copy(program, &shipped)?;
        None
    };
    cx.search(&shipped)?;
    cx.native_notices(triple, &folder)?;
    if system == System::Windows {
        write(&folder.join(NOTE), WINDOWS_NOTE)?;
    }
    let beside = cx.copy_beside(&|_| folder.clone())?;
    let entries = archive::entries(&folder, &format!("{base}/"), &|path| {
        path.file_name() == Some(name)
    })
    .map_err(ExportError::Build)?;
    let package = if system == System::Windows {
        format!("{base}.zip")
    } else {
        format!("{base}.tar.gz")
    };
    let written = cx.out.join(&package);
    if system == System::Windows {
        archive::zip(&written, &entries)
    } else {
        archive::tar_gz(&written, &entries)
    }
    .map_err(ExportError::Build)?;
    let sizes = vec![cx.size(&written)?, cx.size(&shipped)?];
    let mut report = cx.report(&package, sizes, Signing::None, debug);
    report.beside = beside;
    Ok(report)
}

/// Writes the macOS package of the program's `slices`: the folder
/// `<base>/` with `<program>.app` (the slices joined by `lipo`, a minimal
/// `Info.plist`, signed ad hoc), `NOTICES.txt` and the players' note.
fn pack_macos(cx: &ExportCx, slices: &[std::path::PathBuf]) -> Result<ExportReport, ExportError> {
    let base = cx.base();
    let folder = cx.out.join(&base);
    fresh(&folder)?;
    let app = folder.join(format!("{}.app", cx.bin));
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
    write(&contents.join("Info.plist"), &info_plist(&cx.bin))?;
    cx.native_notices(MACOS.triples[0], &folder)?;
    write(&folder.join(NOTE), MACOS_NOTE)?;
    let resources = contents.join("Resources");
    let beside = cx.copy_beside(&|is_dir| {
        if is_dir {
            resources.clone()
        } else {
            folder.clone()
        }
    })?;
    // Every file of the bundle is written: sign the nested code first,
    // then the bundle (not `--deep`).
    for code in [program.as_path(), app.as_path()] {
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
    let sizes = vec![cx.size(&folder)?, cx.size(&program)?];
    let mut report = cx.report(&base, sizes, Signing::AdHoc, None);
    report.beside = beside;
    Ok(report)
}

/// The minimal `Info.plist` of a bundle whose program is `bin`.
pub fn info_plist(bin: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>CFBundleExecutable</key>\n\
         \t<string>{bin}</string>\n\
         \t<key>CFBundlePackageType</key>\n\
         \t<string>APPL</string>\n\
         </dict>\n\
         </plist>\n"
    )
}

/// Copies `from` to `to`.
fn copy(from: &Path, to: &Path) -> Result<(), ExportError> {
    fs::copy(from, to).map(|_| ()).map_err(|e| {
        ExportError::Build(format!("copy {} to {}: {e}", from.display(), to.display()))
    })
}
