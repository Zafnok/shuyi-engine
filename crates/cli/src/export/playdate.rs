//! The Playdate exporter (ADR-0062): the game's library built as a static
//! library for the device (`thumbv7em-none-eabihf`) and, on Windows where
//! MinGW is at hand, for the Simulator (`x86_64-pc-windows-gnu`), linked with
//! the backend's C shim and the SDK's `setup.c`; its pack and its sounds and
//! music ([`audio`]) baked beside them, its `pdxinfo` and launcher images
//! written and `NOTICES.txt`; all packed by
//! the SDK's `pdc` into `<base>.pdx`, and zipped for sideloading.
//!
//! The export is a list of [`Step`]s ([`plan`]), so the tests check every
//! command without the SDK; `export` runs them.

pub mod audio;

use std::collections::BTreeMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use shuyi_assets::bitmap::Bitmap;
use shuyi_assets::bitmap::dither::{Dither, OneBitSettings, dither};
use shuyi_assets::image::Image;

use super::disclosure::{FileKind, Shipped};
use super::{ExportCx, ExportError, ExportReport, Exporter, RunCx, Signing, archive, fresh, paths};
use crate::app_info;
use crate::codec;
use crate::contract::bake::{BakeInput, bake};
use crate::contract::target::{Level, TargetCheck, TargetProblem, engine_profiles};
use crate::game::{GameTools, is_unsupported};
use crate::notices::{self, Build, Section, Text, Work};
use crate::project::{AppPart, Project};

/// The device's Rust target.
pub const DEVICE: &str = "thumbv7em-none-eabihf";

/// The Simulator's Rust target: MinGW, like the GNU host toolchain.
pub const SIMULATOR: &str = "x86_64-pc-windows-gnu";

/// The cargo feature of a game's package that makes its library the game on
/// the Playdate.
pub const FEATURE: &str = "playdate";

/// What the build tells `shuyi-playdate`, to link the SDK.
const CFG: &str = "--cfg=shuyi_playdate_sdk";

/// The device's Rust flags: the device's single-precision FPU, and
/// position-independent code, so no MOVW/MOVT address pairs, which `pdc`
/// can't relocate.
const DEVICE_RUSTFLAGS: [&str; 4] = [
    "-Ctarget-cpu=cortex-m7",
    "-Ctarget-feature=-fp64",
    "-Crelocation-model=pic",
    CFG,
];

/// The C compiler's CPU flags for the device (the SDK's `common.mk`'s
/// `MCFLAGS`).
const DEVICE_CPU_FLAGS: [&str; 5] = [
    "-mthumb",
    "-mcpu=cortex-m7",
    "-mfloat-abi=hard",
    "-mfpu=fpv5-sp-d16",
    "-D__FPU_USED=1",
];

/// The rest of `common.mk`'s compile flags for the device, without its
/// debug information, which would hold the build machine's paths, and its
/// listing and dependency files.
const DEVICE_COMPILE_FLAGS: [&str; 13] = [
    "-O2",
    "-falign-functions=16",
    "-fomit-frame-pointer",
    "-Wall",
    "-Wno-unused",
    "-Wstrict-prototypes",
    "-Wno-unknown-pragmas",
    "-mword-relocations",
    "-fno-common",
    "-ffunction-sections",
    "-fdata-sections",
    "-DTARGET_PLAYDATE=1",
    "-DTARGET_EXTENSION=1",
];

/// The backend's C side, written beside the build's objects: it is compiled
/// against the SDK's headers, which are never copied into the repository.
const SHIM: &str = include_str!("../../../playdate/shim/shim.c");

/// Where MSYS2 puts the device's compiler
/// (`mingw-w64-ucrt-x86_64-arm-none-eabi-toolchain`).
const MSYS2_ARM_GCC: &str = r"C:\msys64\ucrt64\bin\arm-none-eabi-gcc.exe";

/// Where MSYS2 puts MinGW's compiler (`mingw-w64-x86_64-gcc`), the one a
/// Windows build links with.
const MSYS2_GCC: &str = r"C:\msys64\mingw64\bin\gcc.exe";

/// The launcher's folder, in the `.pdx`.
const LAUNCHER: &str = "launcher";

/// The launcher's list icon, a side in pixels (the SDK's docs).
const ICON_SIDE: u32 = 32;

/// The launcher's card, in pixels (the SDK's docs).
const CARD_SIZE: (u32, u32) = (350, 155);

/// The side of the game's icon on its card.
const CARD_ICON_SIDE: u32 = 128;

/// How to install the SDK.
fn sdk_help(looked: &str) -> String {
    format!(
        "the Playdate SDK wasn't found (looked in: {looked}). Read Panic's SDK license at \
         https://play.date/dev/ and, if you accept it, download the SDK and run its installer \
         with the defaults. Set PLAYDATE_SDK_PATH if it isn't in ~/Documents/PlaydateSDK"
    )
}

/// How to install the device's compiler.
fn arm_gcc_help() -> String {
    format!(
        "arm-none-eabi-gcc wasn't found on PATH or at {MSYS2_ARM_GCC}. Install it with MSYS2 \
         (C:\\msys64\\usr\\bin\\pacman.exe -S mingw-w64-ucrt-x86_64-arm-none-eabi-toolchain) or, \
         on Linux, apt-get install gcc-arm-none-eabi"
    )
}

/// Where to look for the SDK, in order: `PLAYDATE_SDK_PATH`, then
/// `~/Documents/PlaydateSDK` (the installer's default).
pub fn sdk_candidates(sdk_path: Option<OsString>, home: Option<PathBuf>) -> Vec<PathBuf> {
    let from_env = sdk_path.filter(|path| !path.is_empty()).map(PathBuf::from);
    let default = home.map(|home| home.join("Documents").join("PlaydateSDK"));
    from_env.into_iter().chain(default).collect()
}

/// The first of `candidates` that holds an SDK (its `C_API/pd_api.h`), or
/// how to install one.
pub fn find_sdk(candidates: &[PathBuf]) -> Result<PathBuf, ExportError> {
    candidates
        .iter()
        .find(|path| path.join("C_API").join("pd_api.h").is_file())
        .cloned()
        .ok_or_else(|| {
            let looked: Vec<String> = candidates.iter().map(|p| p.display().to_string()).collect();
            let looked = if looked.is_empty() {
                "nowhere: no PLAYDATE_SDK_PATH and no home folder".to_owned()
            } else {
                looked.join(", ")
            };
            ExportError::MissingTool(sdk_help(&looked))
        })
}

/// The SDK on this machine.
fn sdk_here() -> Result<PathBuf, ExportError> {
    find_sdk(&sdk_candidates(
        env::var_os("PLAYDATE_SDK_PATH"),
        paths::home(),
    ))
}

/// The program `file_name` (with `.exe` on Windows) among `first`, in a
/// folder of `path_var` (`PATH`), or among `then`, whichever is there first.
fn find_tool(
    file_name: &str,
    first: &[&Path],
    path_var: Option<&OsStr>,
    then: &[&Path],
) -> Option<PathBuf> {
    first
        .iter()
        .map(|path| path.to_path_buf())
        .chain(
            path_var
                .into_iter()
                .flat_map(env::split_paths)
                .map(|dir| dir.join(file_name)),
        )
        .chain(then.iter().map(|path| path.to_path_buf()))
        .find(|path| path.is_file())
}

/// What the build needs from the machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tools {
    /// The Playdate SDK.
    pub sdk: PathBuf,
    /// `arm-none-eabi-gcc`.
    pub arm_gcc: PathBuf,
    /// What the Simulator's part is linked with, or why there is none.
    pub simulator: Result<PathBuf, String>,
    /// `CARGO_ENCODED_RUSTFLAGS` of a device build and a Simulator's.
    pub rustflags: (String, String),
}

/// The Simulator's linker on this machine, or why there is no Simulator
/// part: it is a Windows DLL here, and a build needs MinGW's `gcc` and the
/// Rust target.
fn simulator_here(path_var: Option<&OsStr>) -> Result<PathBuf, String> {
    if !cfg!(windows) {
        return Err(
            "no Simulator part: the export builds one on Windows only, so this .pdx runs on the \
             device alone"
                .to_owned(),
        );
    }
    if super::installed(SIMULATOR).is_err() {
        return Err(format!(
            "no Simulator part: the Rust target {SIMULATOR} isn't installed (rustup target add \
             {SIMULATOR})"
        ));
    }
    find_tool("gcc.exe", &[Path::new(MSYS2_GCC)], path_var, &[]).ok_or_else(|| {
        format!(
            "no Simulator part: MinGW's gcc wasn't found at {MSYS2_GCC} or on PATH (C:\\msys64\\usr\\bin\\pacman.exe \
             -S mingw-w64-x86_64-gcc)"
        )
    })
}

/// What the build needs, found on this machine.
fn tools_here(cx: &ExportCx) -> Result<Tools, ExportError> {
    let sdk = sdk_here()?;
    super::installed(DEVICE)?;
    let path_var = env::var_os("PATH");
    let arm_gcc = find_tool(
        &format!("arm-none-eabi-gcc{}", env::consts::EXE_SUFFIX),
        &[],
        path_var.as_deref(),
        &[Path::new(MSYS2_ARM_GCC)],
    )
    .ok_or_else(|| ExportError::MissingTool(arm_gcc_help()))?;
    let remaps = if cx.release {
        let outside = paths::outside(&cx.root, &cx.workspace);
        paths::remaps(&cx.workspace, &cx.target_dir, &cx.package, &outside)
    } else {
        Vec::new()
    };
    let flags = paths::remapped_flags_here(&remaps);
    Ok(Tools {
        sdk,
        arm_gcc,
        simulator: simulator_here(path_var.as_deref()),
        rustflags: (
            with_flags(&flags, &DEVICE_RUSTFLAGS),
            with_flags(&flags, &[CFG]),
        ),
    })
}

/// `encoded`, Cargo's flags variable, with `flags` after what it holds.
fn with_flags(encoded: &str, flags: &[&str]) -> String {
    let mut all: Vec<&str> = encoded.split('\x1f').filter(|f| !f.is_empty()).collect();
    all.extend(flags);
    all.join("\x1f")
}

/// One thing the export does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Runs `program` with `args` and `envs`.
    Run {
        /// What it does, for the error when it fails.
        what: &'static str,
        /// The program.
        program: PathBuf,
        /// Its arguments.
        args: Vec<OsString>,
        /// What it has in its environment beside this process's.
        envs: Vec<(String, String)>,
    },
    /// Fails if `0` holds a path of the build machine (a release build).
    Search(PathBuf),
}

/// `items` as arguments.
fn os_args<I, S>(items: I) -> Vec<OsString>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    items.into_iter().map(Into::into).collect()
}

/// `cargo rustc`'s arguments for the game's library as a static library.
fn cargo_args(cx: &ExportCx, triple: &str) -> Vec<String> {
    let mut args: Vec<String> = [
        "rustc",
        "-p",
        &cx.package,
        "--lib",
        "--crate-type",
        "staticlib",
        "--no-default-features",
        "--features",
        FEATURE,
        "--target",
        triple,
    ]
    .map(str::to_owned)
    .to_vec();
    if cx.release {
        args.push("--release".to_owned());
    }
    args
}

/// Cargo's profile settings for the build, by variable, over the game's own
/// profile: a `no_std` library has no unwinding, and the device's link wants
/// everything inlined. Line tables stay in `pdex.elf` for reading a crash's
/// addresses (`pdc` leaves them out of the game it packs), their paths
/// remapped.
fn profile_env(release: bool, rustflags: &str) -> Vec<(String, String)> {
    let settings: &[(&str, &str)] = if release {
        &[
            ("CARGO_PROFILE_RELEASE_PANIC", "abort"),
            ("CARGO_PROFILE_RELEASE_LTO", "fat"),
            ("CARGO_PROFILE_RELEASE_CODEGEN_UNITS", "1"),
            ("CARGO_PROFILE_RELEASE_STRIP", "false"),
            ("CARGO_PROFILE_RELEASE_DEBUG", "line-tables-only"),
        ]
    } else {
        &[("CARGO_PROFILE_DEV_PANIC", "abort")]
    };
    settings
        .iter()
        .map(|&(key, value)| (key.to_owned(), value.to_owned()))
        .chain([(paths::ENCODED.to_owned(), rustflags.to_owned())])
        .collect()
}

/// The folders of a build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dirs {
    /// What `pdc` packs.
    pub source: PathBuf,
    /// The C side's sources and objects.
    pub obj: PathBuf,
    /// The build's own folder, which outlives a build.
    pub build: PathBuf,
}

impl Dirs {
    /// The folders of `cx`'s build, in Cargo's target folder.
    fn of(cx: &ExportCx) -> Self {
        let build = cx.target_dir.join("playdate").join(&cx.package);
        Self {
            source: build.join("Source"),
            obj: build.join("obj"),
            build,
        }
    }
}

/// The static library Cargo builds of `cx`'s package for `triple`.
fn rust_lib(cx: &ExportCx, triple: &str) -> PathBuf {
    let profile = if cx.release { "release" } else { "debug" };
    cx.target_dir
        .join(triple)
        .join(profile)
        .join(format!("lib{}.a", cx.package.replace('-', "_")))
}

/// The steps that build `pdex.elf`, for the device.
fn device_steps(cx: &ExportCx, tools: &Tools, dirs: &Dirs) -> Vec<Step> {
    let c_api = tools.sdk.join("C_API");
    let compile = |input: PathBuf, output: PathBuf, strict: bool| {
        let mut args = os_args(
            DEVICE_CPU_FLAGS
                .iter()
                .chain(&DEVICE_COMPILE_FLAGS)
                .copied(),
        );
        if strict {
            args.push("-Werror".into());
        }
        args.extend(os_args(["-I".into(), c_api.clone().into_os_string()]));
        args.extend(os_args(["-c".into(), input.into_os_string()]));
        args.extend(os_args(["-o".into(), output.into_os_string()]));
        args
    };
    let mut link = os_args([
        dirs.obj.join("shim.o"),
        dirs.obj.join("setup.o"),
        rust_lib(cx, DEVICE),
    ]);
    link.push("-nostartfiles".into());
    link.extend(os_args(DEVICE_CPU_FLAGS.iter().copied()));
    let mut script = OsString::from("-T");
    script.push(c_api.join("buildsupport").join("link_map.ld"));
    link.push(script);
    let mut map = OsString::from("-Wl,-Map=");
    map.push(dirs.obj.join("pdex.map"));
    map.push(",--cref,--gc-sections,--no-warn-mismatch,--emit-relocs");
    link.push(map);
    let elf = dirs.source.join("pdex.elf");
    link.extend(os_args(["-o".into(), elf.clone().into_os_string()]));
    let mut steps = vec![
        Step::Run {
            what: "cargo rustc (device)",
            program: crate::cargo().into(),
            args: os_args(cargo_args(cx, DEVICE)),
            envs: profile_env(cx.release, &tools.rustflags.0),
        },
        Step::Run {
            what: "compile shim.c (device)",
            program: tools.arm_gcc.clone(),
            // Our own C: no warnings.
            args: compile(dirs.obj.join("shim.c"), dirs.obj.join("shim.o"), true),
            envs: Vec::new(),
        },
        Step::Run {
            what: "compile setup.c (device)",
            program: tools.arm_gcc.clone(),
            args: compile(
                c_api.join("buildsupport").join("setup.c"),
                dirs.obj.join("setup.o"),
                false,
            ),
            envs: Vec::new(),
        },
        Step::Run {
            what: "link pdex.elf (device)",
            program: tools.arm_gcc.clone(),
            args: link,
            envs: Vec::new(),
        },
    ];
    if !cx.release {
        // The SDK's link map discards `.ARM.exidx` but not each function's
        // `.ARM.exidx.*`: unoptimized, a game of some 57,000 functions keeps
        // one for each (and its relocations), past the ELF's 65,280 sections,
        // and `pdc` crashes on that (0xc0000005, no message). With `panic =
        // "abort"` nothing unwinds, so they go. A release build, inlined and
        // LTO'd, stays well under.
        steps.push(Step::Run {
            what: "drop unwind index sections (device)",
            program: tools
                .arm_gcc
                .with_file_name(format!("arm-none-eabi-objcopy{}", env::consts::EXE_SUFFIX)),
            args: os_args([
                OsString::from("--remove-section=.ARM.exidx*"),
                OsString::from("--remove-section=.rel.ARM.exidx*"),
                elf.clone().into_os_string(),
            ]),
            envs: Vec::new(),
        });
    }
    steps.push(Step::Search(elf));
    steps
}

/// The steps that build `pdex.dll`, for the Simulator, linked by `gcc`.
fn simulator_steps(cx: &ExportCx, tools: &Tools, gcc: &Path, dirs: &Dirs) -> Vec<Step> {
    let c_api = tools.sdk.join("C_API");
    let mut args = os_args([
        "-shared",
        "-O2",
        // Stripped: the `.pdx` carries the Simulator's file to the device too.
        "-s",
        "-Wall",
        "-D_WINDLL",
        "-DTARGET_SIMULATOR=1",
        "-DTARGET_EXTENSION=1",
        "-I",
    ]);
    args.push(c_api.clone().into_os_string());
    args.extend(os_args([
        dirs.obj.join("shim.c"),
        c_api.join("buildsupport").join("setup.c"),
        rust_lib(cx, SIMULATOR),
    ]));
    // No time of this build in the file: the same input gives the same file.
    args.extend(os_args([
        "-static-libgcc",
        "-Wl,--no-insert-timestamp",
        "-o",
    ]));
    let dll = dirs.source.join("pdex.dll");
    args.push(dll.clone().into_os_string());
    vec![
        Step::Run {
            what: "cargo rustc (Simulator)",
            program: crate::cargo().into(),
            args: os_args(cargo_args(cx, SIMULATOR)),
            envs: profile_env(cx.release, &tools.rustflags.1),
        },
        Step::Run {
            what: "link pdex.dll (Simulator)",
            program: gcc.to_path_buf(),
            args,
            envs: Vec::new(),
        },
        Step::Search(dll),
    ]
}

/// The steps that pack `dirs.source` with the SDK's `pdc` into `pdx`.
fn pdc_step(tools: &Tools, dirs: &Dirs, pdx: &Path) -> Step {
    let pdc = tools
        .sdk
        .join("bin")
        .join(format!("pdc{}", env::consts::EXE_SUFFIX));
    Step::Run {
        what: "pdc",
        program: pdc,
        args: os_args([
            OsString::from("-sdkpath"),
            tools.sdk.clone().into_os_string(),
            dirs.source.clone().into_os_string(),
            pdx.as_os_str().to_owned(),
        ]),
        envs: Vec::new(),
    }
}

/// The steps of `cx`'s export, with `tools`, into `pdx`.
pub fn plan(cx: &ExportCx, tools: &Tools, dirs: &Dirs, pdx: &Path) -> Vec<Step> {
    let mut steps = device_steps(cx, tools, dirs);
    if let Ok(gcc) = &tools.simulator {
        steps.extend(simulator_steps(cx, tools, gcc, dirs));
    }
    steps.push(pdc_step(tools, dirs, pdx));
    steps
}

/// Runs `steps` in order, stopping at the first that fails.
fn execute(cx: &ExportCx, steps: &[Step]) -> Result<(), ExportError> {
    for step in steps {
        match step {
            Step::Run {
                what,
                program,
                args,
                envs,
            } => {
                let status = Command::new(program)
                    .args(args)
                    .envs(envs.iter().map(|(k, v)| (k, v)))
                    .current_dir(&cx.root)
                    .status()
                    .map_err(|e| {
                        ExportError::MissingTool(format!(
                            "{what}: {} doesn't start: {e}",
                            program.display()
                        ))
                    })?;
                if !status.success() {
                    return Err(ExportError::Build(format!("{what} exited with {status}")));
                }
            }
            Step::Search(file) => cx.search(file)?,
        }
    }
    Ok(())
}

/// The `pdxinfo` of the game `part` says, at `version`, with the launcher's
/// images in its launcher folder if it has any.
pub fn pdxinfo(part: &AppPart, version: &str, images: bool) -> String {
    let image_path = if images {
        format!("imagePath={LAUNCHER}\n")
    } else {
        String::new()
    };
    format!(
        "name={}\nauthor={}\nbundleID={}\nversion={version}\nbuildNumber={}\n{image_path}",
        part.name, part.publisher, part.id, part.build
    )
}

/// One-bit picture file `file`'s first style as an RGBA PNG: what is
/// see-through is white.
fn one_bit_png(file: &[u8]) -> Result<Vec<u8>, String> {
    let bitmap = Bitmap::parse(file)?;
    let (width, height) = (bitmap.width(), bitmap.height());
    let style = bitmap.style(0).ok_or("no style")?;
    let stride = usize::from(width).div_ceil(8);
    let mut rgba = Vec::with_capacity(usize::from(width) * usize::from(height) * 4);
    for y in 0..usize::from(height) {
        for x in 0..usize::from(width) {
            let at = y * stride + x / 8;
            let bit = |plane: &[u8]| plane[at] >> (7 - (x % 8)) & 1 == 1;
            let white = bit(style) || bitmap.mask().is_some_and(|mask| !bit(mask));
            let shade = if white { 255 } else { 0 };
            rgba.extend_from_slice(&[shade, shade, shade, 255]);
        }
    }
    codec::encode_png(u32::from(width), u32::from(height), &rgba)
}

/// A black and white PNG of the `side` × `side` RGBA pixels, by `settings`.
fn dithered(side: u32, rgba: Vec<u8>, settings: OneBitSettings) -> Result<Vec<u8>, String> {
    let image = Image {
        width: side,
        height: side,
        rgba,
    };
    one_bit_png(&dither(&image, settings)?)
}

/// `icon`'s card: its [`CARD_ICON_SIDE`] middle on white, [`CARD_SIZE`], in
/// black and white by a threshold, since a card's art is line art.
fn card_from_icon(side: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let small = app_info::downscale(side, side, rgba, CARD_ICON_SIDE);
    let (width, height) = CARD_SIZE;
    let mut out = vec![255; (width * height * 4) as usize];
    let (left, top) = ((width - CARD_ICON_SIDE) / 2, (height - CARD_ICON_SIDE) / 2);
    for y in 0..CARD_ICON_SIDE {
        for x in 0..CARD_ICON_SIDE {
            let from = ((y * CARD_ICON_SIDE + x) * 4) as usize;
            let to = (((y + top) * width + x + left) * 4) as usize;
            // On white, by its alpha.
            let alpha = u32::from(small[from + 3]);
            for channel in 0..3 {
                let shade = u32::from(small[from + channel]) * alpha + 255 * (255 - alpha);
                out[to + channel] = u8::try_from(shade / 255).unwrap_or(u8::MAX);
            }
        }
    }
    let image = Image {
        width,
        height,
        rgba: out,
    };
    one_bit_png(&dither(
        &image,
        OneBitSettings::new(Dither::Threshold(128)),
    )?)
}

/// The launcher's images of the game `part` says, by file name: from its
/// `playdate_icon` and `playdate_card` where it names them, else a one-bit
/// bake of its `icon`. None if it has neither.
fn launcher_images(part: &AppPart, root: &Path) -> Result<BTreeMap<&'static str, Vec<u8>>, String> {
    let mut images = BTreeMap::new();
    let read = |path: &String| fs::read(root.join(path)).map_err(|e| format!("{path}: {e}"));
    if let Some(path) = &part.playdate_icon {
        images.insert("icon.png", read(path)?);
    }
    if let Some(path) = &part.playdate_card {
        images.insert("card.png", read(path)?);
    }
    if let Some(icon) = app_info::icon(part, root)? {
        if !images.contains_key("icon.png") {
            let small = app_info::downscale(icon.side, icon.side, &icon.rgba, ICON_SIDE);
            images.insert(
                "icon.png",
                dithered(ICON_SIDE, small, OneBitSettings::default())?,
            );
        }
        if !images.contains_key("card.png") {
            images.insert("card.png", card_from_icon(icon.side, &icon.rgba)?);
        }
    }
    Ok(images)
}

/// The SDK's `setup.c`, linked into every game, in `NOTICES.txt`: its
/// license is the SDK's `buildsupport/License.md`.
fn setup_c_notice(sdk: &Path) -> Result<Work, ExportError> {
    let file = sdk.join("C_API").join("buildsupport").join("License.md");
    let license = fs::read_to_string(&file)
        .map_err(|e| ExportError::Build(format!("read {}: {e}", file.display())))?;
    let mut work = Work::new(Section::Crates, "setup.c, of the Playdate SDK");
    "0BSD".clone_into(&mut work.license);
    "Panic Inc., https://play.date/dev/".clone_into(&mut work.source);
    work.texts = vec![Text::new("the SDK's buildsupport/License.md", license)];
    Ok(work)
}

/// The pack's file in the export's folder, between the bake and the build.
const BAKED: &str = "game.pack";

/// The baked audio's folder in the export's folder, between the bake and the
/// build. It isn't there when the game's tools offer no audio.
const BAKED_AUDIO: &str = "baked-audio";

/// Bakes the sounds and music `game` ships on `cx`'s target, for the build.
fn bake_audio(cx: &ExportCx, game: &dyn GameTools) -> Result<(), ExportError> {
    let into = cx.out.join(BAKED_AUDIO);
    if into.exists() {
        fs::remove_dir_all(&into)
            .map_err(|e| ExportError::Build(format!("clear {}: {e}", into.display())))?;
    }
    let shipped = match game.audio(&cx.profile) {
        Ok(shipped) => shipped,
        Err(e) if is_unsupported(&e) => return Ok(()),
        Err(e) => return Err(ExportError::Setup(format!("the game's audio: {e}"))),
    };
    let jobs = audio::jobs(&shipped, &cx.root)
        .map_err(|e| ExportError::Setup(format!("the game's audio: {e}")))?;
    fs::create_dir_all(&into)
        .map_err(|e| ExportError::Build(format!("create {}: {e}", into.display())))?;
    audio::bake(&jobs, &audio::cache_dir(&Dirs::of(cx).build), &into)
        .map_err(|e| ExportError::Build(format!("the game's audio: {e}")))
}

/// Moves what [`bake_audio`] made into what `pdc` packs: true if there was any.
fn place_audio(cx: &ExportCx, dirs: &Dirs) -> Result<bool, ExportError> {
    let baked = cx.out.join(BAKED_AUDIO);
    if !baked.is_dir() {
        return Ok(false);
    }
    let names = fs::read_dir(&baked)
        .map_err(|e| ExportError::Build(format!("read {}: {e}", baked.display())))?;
    for entry in names.flatten() {
        let to = dirs.source.join(entry.file_name());
        fs::rename(entry.path(), &to)
            .map_err(|e| ExportError::Build(format!("move to {}: {e}", to.display())))?;
    }
    fs::remove_dir_all(&baked)
        .map_err(|e| ExportError::Build(format!("clear {}: {e}", baked.display())))?;
    Ok(true)
}

/// The Playdate exporter.
pub static PLAYDATE: PlaydateExporter = PlaydateExporter;

/// The Playdate exporter's type.
pub struct PlaydateExporter;

impl PlaydateExporter {
    /// `report` of `pdx`, with a warning if it is bigger than the profile's
    /// budget, and why it has no Simulator part if it hasn't.
    fn warnings(
        cx: &ExportCx,
        pdx_bytes: u64,
        simulator: &Result<PathBuf, String>,
        has_audio: bool,
    ) -> Vec<TargetProblem> {
        let budget = engine_profiles().ok().and_then(|profiles| {
            profiles
                .get(&cx.profile)
                .and_then(|profile| profile.profile.budgets.package_bytes)
        });
        let mut warnings = Vec::new();
        if let Some(budget) = budget.filter(|&budget| pdx_bytes > budget) {
            warnings.push(TargetProblem::display(
                Level::Warning,
                TargetCheck::Fits,
                "budgets.package_bytes".to_owned(),
                format!("the .pdx is {pdx_bytes} bytes, over the profile's {budget}"),
            ));
        }
        if let Err(why) = simulator {
            warnings.push(TargetProblem::display(
                Level::Warning,
                TargetCheck::Fits,
                "simulator".to_owned(),
                why.clone(),
            ));
        }
        if !has_audio {
            warnings.push(TargetProblem::display(
                Level::Warning,
                TargetCheck::Fits,
                "audio".to_owned(),
                "the game's tools offer no audio (GameTools::audio), so this .pdx is silent"
                    .to_owned(),
            ));
        }
        warnings
    }
}

impl Exporter for PlaydateExporter {
    fn id(&self) -> &'static str {
        "playdate"
    }

    fn profile(&self) -> &'static str {
        "playdate"
    }

    fn triples(&self) -> &'static [&'static str] {
        &[DEVICE, SIMULATOR]
    }

    fn builds_a_program(&self) -> bool {
        false
    }

    fn takes_export_part(&self) -> bool {
        false
    }

    fn cargo_args(&self, cx: &ExportCx) -> Vec<Vec<String>> {
        self.triples()
            .iter()
            .map(|triple| cargo_args(cx, triple))
            .collect()
    }

    /// Bakes the game's pack for the target with its tools.
    fn prepare(
        &self,
        cx: &ExportCx,
        project: &Project,
        game: Option<&dyn GameTools>,
    ) -> Result<(), ExportError> {
        if !cx.features.iter().any(|feature| feature == FEATURE) {
            return Err(ExportError::Setup(format!(
                "the package `{}` has no `{FEATURE}` feature, which makes its library the game \
                 on the Playdate (docs/targets/playdate.md)",
                cx.package
            )));
        }
        let game = game.ok_or_else(|| {
            ExportError::Setup(format!(
                "{} has no `game` part: the Playdate's pack is baked by the game's tools",
                crate::project::FILE
            ))
        })?;
        bake_audio(cx, game)?;
        let input = BakeInput {
            target: Some(cx.profile.clone()),
            out: Some(format!("{}/{BAKED}", cx.shown.replace('\\', "/"))),
            ..BakeInput::default()
        };
        let answer = bake(game, project, &input);
        if answer.ok {
            Ok(())
        } else {
            let problems: Vec<String> = answer
                .result
                .iter()
                .flat_map(|output| &output.errors)
                .map(|problem| problem.message.clone())
                .chain(answer.error.clone())
                .collect();
            Err(ExportError::Build(format!(
                "the pack: {}",
                problems.join("; ")
            )))
        }
    }

    fn export(&self, cx: &ExportCx) -> Result<ExportReport, ExportError> {
        for args in self.cargo_args(cx) {
            super::no_live_link(&args)?;
        }
        let part = cx.app.as_ref().ok_or_else(|| {
            ExportError::Setup(format!(
                "{} has no `app` part: a .pdx's pdxinfo is written from it",
                crate::project::FILE
            ))
        })?;
        let tools = tools_here(cx)?;
        let dirs = Dirs::of(cx);
        fresh(&dirs.source)?;
        fresh(&dirs.obj)?;
        super::write(&dirs.obj.join("shim.c"), SHIM)?;
        let images = launcher_images(part, &cx.root).map_err(ExportError::Setup)?;
        let info = pdxinfo(part, &cx.version, !images.is_empty());
        super::write(&dirs.source.join("pdxinfo"), &info)?;
        if !images.is_empty() {
            let folder = dirs.source.join(LAUNCHER);
            fresh(&folder)?;
            for (name, bytes) in &images {
                let file = folder.join(name);
                fs::write(&file, bytes)
                    .map_err(|e| ExportError::Build(format!("write {}: {e}", file.display())))?;
            }
        }
        let baked = cx.out.join(BAKED);
        fs::rename(&baked, dirs.source.join(playdate_pack()))
            .map_err(|e| ExportError::Build(format!("move the pack {}: {e}", baked.display())))?;
        let has_audio = place_audio(cx, &dirs)?;
        let mut works = notices::crates(&cx.root, &cx.package, DEVICE, &[FEATURE.to_owned()])
            .map_err(ExportError::Build)?;
        works.extend(notices::engine_files(Build::Native, &works));
        works.push(setup_c_notice(&tools.sdk)?);
        works.extend(cx.game.iter().cloned());
        notices::write(&dirs.source, &cx.title, &works).map_err(ExportError::Build)?;
        let name = format!("{}.pdx", cx.base());
        let pdx = cx.out.join(&name);
        if pdx.exists() {
            fs::remove_dir_all(&pdx)
                .map_err(|e| ExportError::Build(format!("clear {}: {e}", pdx.display())))?;
        }
        execute(cx, &plan(cx, &tools, &dirs, &pdx))?;
        for part in ["pdex.bin", "pdex.dll"] {
            let file = pdx.join(part);
            if file.exists() {
                cx.search(&file)?;
            }
        }
        // Panic's sideloading page takes a zip of the folder.
        let zip = format!("{name}.zip");
        let entries =
            archive::entries(&pdx, &format!("{name}/"), &|_| false).map_err(ExportError::Build)?;
        archive::zip(&cx.out.join(&zip), &entries).map_err(ExportError::Build)?;
        let size = cx.size(&pdx)?;
        let warnings = Self::warnings(cx, size.bytes, &tools.simulator, has_audio);
        let sizes = vec![size, cx.size(&cx.out.join(&zip))?];
        let mut report = cx.report(&name, sizes, Signing::None, None);
        report.warnings = warnings;
        Ok(report)
    }

    /// What `pdc` makes of what is written here: the programs, `pdxinfo`, the
    /// launcher's images, the notices and the pack; the sounds and music it
    /// bakes from the game's tools; and `pdex.elf`, read in place of the
    /// `pdex.bin` that may be compressed.
    fn shipped(&self, cx: &ExportCx, report: &ExportReport) -> Shipped {
        let tree = cx.out.join(&report.package);
        let archive = cx.out.join(format!("{}.zip", report.package));
        let mut shipped = Shipped::new(tree, Some(archive), format!("{}/", report.package));
        for file in ["pdex.bin", "pdex.dll"] {
            shipped.source(file, FileKind::Program, "the built library");
        }
        for file in [
            "pdxinfo",
            notices::FILE,
            "launcher/icon.pdi",
            "launcher/card.pdi",
        ] {
            shipped.source(file, FileKind::Written, "written by the export");
        }
        shipped.source(BAKED, FileKind::Pack, "the game's bundle, baked");
        // `pdc` writes the time it ran into the `pdxinfo` it copies.
        shipped
            .varying
            .insert("pdxinfo".to_owned(), "buildtime=".to_owned());
        let found = archive::entries(&shipped.tree, "", &|_| false).unwrap_or_default();
        for entry in found {
            if entry.name.starts_with("audio/") || entry.name.starts_with("music/") {
                shipped.source(entry.name, FileKind::LooseAsset, "GameTools::audio, baked");
            }
        }
        shipped.inputs.push((
            "pdex.elf".to_owned(),
            "what pdc makes pdex.bin from".to_owned(),
            Dirs::of(cx).source.join("pdex.elf"),
        ));
        shipped
    }

    /// Opens the `.pdx` in the SDK's Simulator, and waits for it to close.
    /// What the game logs shows in the Simulator's own console: on Windows
    /// the program gives its caller nothing.
    fn run(&self, cx: &RunCx) -> Result<(), ExportError> {
        let pdx = cx.out.join(&cx.report.package);
        if !pdx.join("pdex.dll").is_file() {
            let why = cx
                .report
                .warnings
                .iter()
                .find(|warning| warning.setting == "simulator")
                .map_or("no Simulator part was built", |warning| {
                    warning.message.as_str()
                });
            return Err(ExportError::NoRunStep(format!(
                "{} can't open in the Simulator: {why}",
                cx.report.package
            )));
        }
        let simulator = sdk_here()?
            .join("bin")
            .join(format!("PlaydateSimulator{}", env::consts::EXE_SUFFIX));
        let status = Command::new(&simulator).arg(&pdx).status().map_err(|e| {
            ExportError::MissingTool(format!("{} doesn't start: {e}", simulator.display()))
        })?;
        if status.success() {
            Ok(())
        } else {
            Err(ExportError::Build(format!(
                "the Simulator exited with {status}"
            )))
        }
    }
}

/// The pack's file in the `.pdx`, as `shuyi-playdate` reads it: a test keeps
/// the two equal.
pub const fn playdate_pack() -> &'static str {
    BAKED
}

#[cfg(test)]
mod tests;
