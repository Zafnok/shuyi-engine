//! `web [--release] [--<feature>...]`: builds the game's binary
//! ([`crate::project::Web`]) for `wasm32-unknown-unknown` and packages it
//! with the web shell into the project's `web.dist` folder: the page
//! (`web/index.html`, with the game's title and binary written in), the
//! vendored JS loaders (`web/mq_js_bundle.js`, and `web/sapp_jsutils.js`
//! with `web/quad-storage.js` for `localStorage`) and the engine's controller
//! plugin (`web/gamepad.js`). The shell is built into this tool; see
//! `web/README.md`, which also says how a game's binary gets the link
//! argument these plugins need. Each `--<feature>` turns on a feature of
//! the game's package that `web.features` lists (a debug build's tools,
//! say). The game's music tracks (`.ogg` in `web.music`, not embedded) are
//! copied beside the page into a folder of the same name, which exists
//! even when there are none; its voice clips (`web.voice`, `.ogg` and
//! `.ron`) are copied the same way when there is such a folder.
//!
//! Every build also writes `NOTICES.txt` beside the page ([`notices`]):
//! the crates compiled in, the shell's files (`SHELL_NOTICES`) and the
//! game's own license texts.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::export::paths;
use crate::notices::{self, EngineFile, Section, Work};
use crate::project::Web;

/// The shell's files: name and contents. `index.html` has `%TITLE%` and
/// `%WASM%` in it, for the game's title and binary.
const SHELL: [(&str, &str); 5] = [
    ("index.html", include_str!("../web/index.html")),
    ("mq_js_bundle.js", include_str!("../web/mq_js_bundle.js")),
    ("sapp_jsutils.js", include_str!("../web/sapp_jsutils.js")),
    ("quad-storage.js", include_str!("../web/quad-storage.js")),
    ("gamepad.js", include_str!("../web/gamepad.js")),
];

/// Where a file of [`SHELL`] comes from, for `NOTICES.txt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Origin {
    /// The engine's own code, under the engine's licenses.
    Engine,
    /// Vendored from another work: its row of [`notices::FILES`].
    Vendored,
}

/// Each file of [`SHELL`], by name, and where it comes from.
const SHELL_NOTICES: [(&str, Origin); 5] = [
    ("index.html", Origin::Engine),
    ("mq_js_bundle.js", Origin::Vendored),
    ("sapp_jsutils.js", Origin::Vendored),
    ("quad-storage.js", Origin::Vendored),
    ("gamepad.js", Origin::Engine),
];

/// The row of [`notices::FILES`] of the shell's file `name`.
fn vendored(name: &str) -> Option<&'static EngineFile> {
    notices::FILES
        .iter()
        .find(|file| file.file.strip_prefix("crates/cli/web/") == Some(name))
}

/// The shell's files as works of `NOTICES.txt`, in the shell's order.
pub fn shell_works() -> Vec<Work> {
    SHELL_NOTICES
        .iter()
        .map(|(file, origin)| match (origin, vendored(file)) {
            (Origin::Vendored, Some(vendored)) => vendored.work(Section::Shell),
            _ => Work {
                section: Section::Shell,
                name: format!("{file}, from Shuyi Engine"),
                version: env!("CARGO_PKG_VERSION").to_owned(),
                license: env!("CARGO_PKG_LICENSE").to_owned(),
                source: env!("CARGO_PKG_REPOSITORY").to_owned(),
                texts: notices::engine_texts(),
            },
        })
        .collect()
}

/// A file of [`SHELL`] as `dist` gets it: a vendored script starts with
/// a `/*!` line naming its work and pointing at `NOTICES.txt` (a comment
/// before `"use strict";` keeps it strict, and `/*!` survives
/// minifiers); the engine's own files are as committed.
fn shipped_shell_file(name: &str, text: &str) -> String {
    match vendored(name) {
        Some(file) => format!(
            "/*! {name}: {} {}, {}. See {}. */\n{text}",
            file.work,
            file.version,
            file.license,
            notices::FILE
        ),
        None => text.to_owned(),
    }
}

/// The target the game is built for.
pub(crate) const TARGET: &str = "wasm32-unknown-unknown";

/// Parsed `web` arguments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// Whether to build with `--release` (and run `wasm-opt` afterwards).
    pub release: bool,
    /// The features to turn on, in the order given.
    pub features: Vec<String>,
}

/// The usage line, with `command` as the tool's name.
pub fn usage(rules: &Web, command: &str) -> String {
    let mut out = format!("usage: {command} web [--release]");
    for feature in &rules.features {
        // Writing to a `String` can't fail.
        let _ = write!(out, " [--{feature}]");
    }
    out
}

/// Parses the arguments after `web`: `--release` and a `--<feature>` for
/// each feature `rules` list, each at most once, in any order. The error
/// is the usage line.
pub fn parse_args(rules: &Web, command: &str, args: &[String]) -> Result<Options, String> {
    let mut options = Options::default();
    for arg in args {
        let bad = || usage(rules, command);
        if arg == "--release" {
            if options.release {
                return Err(bad());
            }
            options.release = true;
            continue;
        }
        let feature = arg
            .strip_prefix("--")
            .filter(|name| rules.features.iter().any(|f| f == name))
            .ok_or_else(bad)?;
        if options.features.iter().any(|f| f == feature) {
            return Err(bad());
        }
        options.features.push(feature.to_owned());
    }
    Ok(options)
}

/// The `cargo` arguments that build the wasm binary for `options`.
pub(crate) fn cargo_args(rules: &Web, options: &Options) -> Vec<String> {
    let mut args: Vec<String> = ["build", "-p", &rules.package, "--target", TARGET]
        .map(str::to_owned)
        .to_vec();
    if options.release {
        args.push("--release".to_owned());
    }
    for feature in &options.features {
        args.extend(["--features".to_owned(), feature.clone()]);
    }
    args
}

/// Cargo's build profile directory name for a given release flag.
fn profile_dir(release: bool) -> &'static str {
    if release { "release" } else { "debug" }
}

/// Where cargo places the wasm binary for the given options, in its build
/// folder `target`.
fn wasm_artifact_path(rules: &Web, target: &Path, options: &Options) -> PathBuf {
    target
        .join(TARGET)
        .join(profile_dir(options.release))
        .join(format!("{}.wasm", rules.bin))
}

/// The folder cargo builds the project at `root` into: `CARGO_TARGET_DIR`
/// when it is set (from `root`, if relative), else `target` beside the
/// workspace's manifest, since a project may be one folder of a bigger
/// workspace (`cargo locate-project --workspace`), else `root/target`.
pub fn target_dir(root: &Path) -> PathBuf {
    let cargo = crate::cargo();
    let manifest = Command::new(cargo)
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|text| PathBuf::from(text.trim()));
    target_dir_of(
        root,
        std::env::var_os("CARGO_TARGET_DIR"),
        manifest.as_deref(),
    )
}

/// [`target_dir`] from what it found: the variable's value and the
/// workspace's manifest, if any.
fn target_dir_of(
    root: &Path,
    variable: Option<std::ffi::OsString>,
    manifest: Option<&Path>,
) -> PathBuf {
    if let Some(dir) = variable.filter(|dir| !dir.is_empty()) {
        return root.join(dir);
    }
    manifest
        .and_then(Path::parent)
        .map_or_else(|| root.join("target"), |dir| dir.join("target"))
}

/// Builds and packages the web build, returning a one-line summary on
/// success. `game` is what the project adds to `NOTICES.txt` (its own
/// license texts, [`notices::game_texts`]); the crates compiled in and the
/// shell's files are found here. A crate with no license text found is
/// named in a warning.
///
/// A release build has its paths remapped, and its binary is refused if it
/// still holds one of the build machine's ([`paths`]).
pub fn run(rules: &Web, root: &Path, options: &Options, game: &[Work]) -> Result<String, String> {
    let cargo = crate::cargo();
    let target = target_dir(root);
    let workspace = workspace_root(root);
    let mut build = Command::new(cargo);
    build.args(cargo_args(rules, options)).current_dir(root);
    if options.release {
        let remaps = paths::remaps(&workspace, &target, &rules.package);
        build.env(paths::ENCODED, paths::remapped_flags_here(&remaps));
    }
    let status = build
        .status()
        .map_err(|e| format!("spawn cargo build: {e}"))?;
    if !status.success() {
        return Err(format!("cargo build exited with {status}"));
    }
    if options.release {
        let needles = paths::needles(&[root, &workspace]);
        paths::check_file(&wasm_artifact_path(rules, &target, options), &needles)?;
    }
    let mut works = notices::crates(root, &rules.package, TARGET, &options.features)?;
    let bare: Vec<&str> = works
        .iter()
        .filter(|work| work.texts.is_empty())
        .map(|work| work.name.as_str())
        .collect();
    if !bare.is_empty() {
        eprintln!(
            "web: warning: no license text found for {}; {} lists them without one",
            bare.join(", "),
            notices::FILE
        );
    }
    works.extend(shell_works());
    works.extend(game.iter().cloned());
    package(&RealWasmOpt, rules, root, &target, options, &works)
}

/// The root of the workspace the project at `root` builds in: the folder
/// of `cargo locate-project --workspace`'s manifest, else `root`.
pub fn workspace_root(root: &Path) -> PathBuf {
    Command::new(crate::cargo())
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .and_then(|text| PathBuf::from(text.trim()).parent().map(Path::to_path_buf))
        .unwrap_or_else(|| root.to_path_buf())
}

/// Copies the wasm artifact and writes the web shell and `NOTICES.txt`
/// (with `works`) into the dist folder, optimizing the wasm binary with
/// `opt` first if `options.release`. Split out from [`run`] so tests can
/// exercise it (with a fake [`WasmOpt`]) without needing a real `cargo
/// build`.
fn package(
    opt: &impl WasmOpt,
    rules: &Web,
    root: &Path,
    target: &Path,
    options: &Options,
    works: &[Work],
) -> Result<String, String> {
    let dist = root.join(&rules.dist);
    fs::create_dir_all(&dist).map_err(|e| format!("create {}: {e}", dist.display()))?;

    let wasm_name = format!("{}.wasm", rules.bin);
    let wasm_src = wasm_artifact_path(rules, target, options);
    let wasm_dst = dist.join(&wasm_name);
    fs::copy(&wasm_src, &wasm_dst)
        .map_err(|e| format!("copy {} to {}: {e}", wasm_src.display(), wasm_dst.display()))?;

    for (name, text) in SHELL {
        let text = if name == "index.html" {
            text.replace("%TITLE%", &rules.title)
                .replace("%WASM%", &wasm_name)
        } else {
            shipped_shell_file(name, text)
        };
        let shell_dst = dist.join(name);
        fs::write(&shell_dst, text).map_err(|e| format!("write {}: {e}", shell_dst.display()))?;
    }
    notices::write(&dist, &rules.title, works)?;

    if let Some(music) = &rules.music {
        copy_music(&root.join(music), &dist.join(music))?;
    }
    if let Some(voice) = &rules.voice {
        copy_voice(&root.join(voice), &dist.join(voice))?;
    }

    if options.release {
        run_wasm_opt(opt, &wasm_dst);
    }

    let size = fs::metadata(&wasm_dst)
        .map_err(|e| format!("stat {}: {e}", wasm_dst.display()))?
        .len();
    Ok(format!("{} ({size} bytes)", dist.display()))
}

/// Replaces `dst` with a folder holding every `.ogg` file in `src` (none
/// if `src` is missing), so the web build fetches tracks from it.
fn copy_music(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|e| format!("clear {}: {e}", dst.display()))?;
    }
    fs::create_dir_all(dst).map_err(|e| format!("create {}: {e}", dst.display()))?;
    let Ok(entries) = fs::read_dir(src) else {
        return Ok(());
    };
    for entry in entries {
        let path = entry
            .map_err(|e| format!("read {}: {e}", src.display()))?
            .path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "ogg") {
            let name = path.file_name().unwrap_or_default();
            fs::copy(&path, dst.join(name)).map_err(|e| format!("copy {}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// Replaces `dst` with a copy of the voice folder `src`: its manifests
/// (`.ron`) and clips (`.ogg`), in the same folders, so the web build
/// fetches them from it. With no `src` there is no `dst` either: a build
/// without voices.
fn copy_voice(src: &Path, dst: &Path) -> Result<(), String> {
    if dst.exists() {
        fs::remove_dir_all(dst).map_err(|e| format!("clear {}: {e}", dst.display()))?;
    }
    if !src.is_dir() {
        return Ok(());
    }
    let shipped = |path: &Path| {
        path.extension()
            .is_some_and(|ext| ext == "ogg" || ext == "ron")
    };
    copy_tree(src, dst, &shipped).map(|_| ())
}

/// Copies the files under `src` that `keep` accepts to the same places
/// under `dst` (made if missing), folders included. Returns how many files
/// were copied.
pub fn copy_tree(src: &Path, dst: &Path, keep: &dyn Fn(&Path) -> bool) -> Result<usize, String> {
    fs::create_dir_all(dst).map_err(|e| format!("create {}: {e}", dst.display()))?;
    let entries = fs::read_dir(src).map_err(|e| format!("read {}: {e}", src.display()))?;
    let mut copied = 0;
    for entry in entries {
        let path = entry
            .map_err(|e| format!("read {}: {e}", src.display()))?
            .path();
        let to = dst.join(path.file_name().unwrap_or_default());
        if path.is_dir() {
            copied += copy_tree(&path, &to, keep)?;
        } else if keep(&path) {
            fs::copy(&path, &to).map_err(|e| format!("copy {}: {e}", path.display()))?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// Runs `wasm-opt -Oz` on a wasm binary, abstracted so tests can substitute a
/// fake instead of depending on a real `wasm-opt` install (which is optional
/// and often absent — see the "skip silently" rule on [`WasmOpt::optimize`]).
trait WasmOpt {
    /// Optimizes `input` into `output` (mirroring `wasm-opt -Oz -o <output>
    /// <input>`). `Ok(true)`/`Ok(false)` is a completed process's exit
    /// status; `Err` means it couldn't be spawned at all (e.g. not on
    /// `PATH`), which the caller treats the same as "skip silently".
    fn optimize(&self, input: &Path, output: &Path) -> Result<bool, String>;
}

/// The real `wasm-opt` binary on `PATH`.
struct RealWasmOpt;

impl WasmOpt for RealWasmOpt {
    /// Not covered by mutation testing (`.cargo/mutants.toml` leaves it
    /// out): its only job is spawning an optional external tool that isn't
    /// guaranteed to be installed anywhere tests run, so there's no way to
    /// exercise it without one; [`package`]'s tests cover every branch of
    /// the logic that consumes its result via a fake [`WasmOpt`] instead.
    fn optimize(&self, input: &Path, output: &Path) -> Result<bool, String> {
        Command::new("wasm-opt")
            .args(["-Oz", "-o"])
            .arg(output)
            .arg(input)
            .status()
            .map(|status| status.success())
            .map_err(|e| e.to_string())
    }
}

/// Runs `opt` on `wasm_path` in place, replacing it with the optimized
/// output on success. Does nothing (silently) if `opt` reports the tool
/// isn't available.
fn run_wasm_opt(opt: &impl WasmOpt, wasm_path: &Path) {
    let optimized = wasm_path.with_extension("wasm.opt");
    match opt.optimize(wasm_path, &optimized) {
        Ok(true) => {
            if let Err(e) = fs::rename(&optimized, wasm_path) {
                eprintln!("web: wasm-opt ran but replacing the binary failed: {e}");
            }
        }
        Ok(false) => eprintln!("web: wasm-opt exited with a failure, skipping optimization"),
        Err(_) => {} // wasm-opt not on PATH: skip silently.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Rust side of `web/gamepad.js`: the engine's web pads.
    const PADS_WEB: &str = include_str!("../../app/src/pads/web.rs");

    struct FakeWasmOpt<F>(F);

    impl<F: Fn(&Path, &Path) -> Result<bool, String>> WasmOpt for FakeWasmOpt<F> {
        fn optimize(&self, input: &Path, output: &Path) -> Result<bool, String> {
            (self.0)(input, output)
        }
    }

    /// A game's web build: two features, a music and a voice folder.
    fn rules() -> Web {
        Web {
            package: "game-app".to_owned(),
            bin: "the-game".to_owned(),
            title: "The Game".to_owned(),
            features: vec!["debug-tools".to_owned(), "bought-art".to_owned()],
            music: Some("music".to_owned()),
            voice: Some("voice".to_owned()),
            dist: "dist/web".to_owned(),
        }
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    fn parse(items: &[&str]) -> Result<Options, String> {
        parse_args(&rules(), "shuyi", &args(items))
    }

    fn debug() -> Options {
        Options::default()
    }

    fn release() -> Options {
        Options {
            release: true,
            features: Vec::new(),
        }
    }

    fn shell(name: &str) -> &'static str {
        SHELL.iter().find(|(n, _)| *n == name).unwrap().1
    }

    #[test]
    fn index_html_keeps_resuming_audio_until_it_runs() {
        // The shim must wrap `AudioContext` before the bundle creates one,
        // and retry on every key press.
        let html = shell("index.html");
        let shim = html
            .find("class TrackedAudioContext extends Base")
            .expect("index.html wraps AudioContext");
        let bundle = html
            .find(r#"<script src="mq_js_bundle.js">"#)
            .expect("index.html loads mq_js_bundle.js");
        assert!(shim < bundle, "wrap AudioContext before the bundle loads");
        assert!(html.contains(r#"for (const type of ["keydown", "mousedown", "touchend"])"#));
    }

    /// A fresh scratch project root under `env::temp_dir()`.
    fn fixture(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("shuyi-web-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_wasm(root: &Path, options: &Options, content: &[u8]) {
        let path = wasm_artifact_path(&rules(), &root.join("target"), options);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn parse_args_with_no_args_is_debug() {
        assert_eq!(parse(&[]), Ok(debug()));
        assert_eq!(parse(&["--release"]), Ok(release()));
    }

    #[test]
    fn parse_args_takes_the_listed_features_in_any_order_once() {
        let usage = "usage: shuyi web [--release] [--debug-tools] [--bought-art]";
        assert_eq!(usage, super::usage(&rules(), "shuyi"));
        let both = |features: &[&str]| Options {
            release: true,
            features: args(features),
        };
        assert_eq!(
            parse(&["--release", "--debug-tools"]),
            Ok(both(&["debug-tools"]))
        );
        assert_eq!(
            parse(&["--bought-art", "--release", "--debug-tools"]),
            Ok(both(&["bought-art", "debug-tools"]))
        );
        for bad in [
            &["--release", "--release"][..],
            &["--debug-tools", "--debug-tools"],
            &["--bogus"],
            &["--release", "extra"],
            &["--features"],
        ] {
            assert_eq!(parse(bad), Err(usage.to_owned()), "{bad:?}");
        }
    }

    #[test]
    fn cargo_args_add_release_and_the_features() {
        let base = [
            "build",
            "-p",
            "game-app",
            "--target",
            "wasm32-unknown-unknown",
        ];
        assert_eq!(cargo_args(&rules(), &debug()), base);
        assert_eq!(cargo_args(&rules(), &release())[5..], ["--release"]);
        let all = Options {
            release: true,
            features: args(&["debug-tools", "bought-art"]),
        };
        assert_eq!(
            cargo_args(&rules(), &all)[5..],
            [
                "--release",
                "--features",
                "debug-tools",
                "--features",
                "bought-art"
            ]
        );
    }

    #[test]
    fn wasm_artifact_path_uses_the_profile() {
        let root = Path::new("/repo/target");
        assert_eq!(
            wasm_artifact_path(&rules(), root, &debug()),
            Path::new("/repo/target/wasm32-unknown-unknown/debug/the-game.wasm")
        );
        assert_eq!(
            wasm_artifact_path(&rules(), root, &release()),
            Path::new("/repo/target/wasm32-unknown-unknown/release/the-game.wasm")
        );
    }

    #[test]
    fn the_target_folder_is_the_variables_else_the_workspaces() {
        let root = Path::new("/game");
        let manifest = Some(Path::new("/repo/Cargo.toml"));
        assert_eq!(
            target_dir_of(root, Some("/out".into()), manifest),
            Path::new("/out")
        );
        assert_eq!(
            target_dir_of(root, Some("out".into()), manifest),
            Path::new("/game/out")
        );
        assert_eq!(
            target_dir_of(root, Some("".into()), manifest),
            Path::new("/repo/target")
        );
        assert_eq!(
            target_dir_of(root, None, manifest),
            Path::new("/repo/target")
        );
        assert_eq!(target_dir_of(root, None, None), Path::new("/game/target"));
    }

    #[test]
    fn the_target_folder_of_a_crate_in_a_workspace_is_the_workspaces() {
        // This crate's folder is one of the workspace's: its build folder
        // is the workspace's (unless the variable moves it).
        let here = Path::new(env!("CARGO_MANIFEST_DIR"));
        let target = target_dir(here);
        if std::env::var_os("CARGO_TARGET_DIR").is_none() {
            assert_eq!(target.file_name().unwrap(), "target");
            assert!(here.starts_with(target.parent().unwrap()));
            assert_ne!(target, here.join("target"));
        }
        // Outside any workspace: the project's own.
        let alone = fixture("target-alone");
        if std::env::var_os("CARGO_TARGET_DIR").is_none() {
            assert_eq!(target_dir(&alone), alone.join("target"));
        }
        fs::remove_dir_all(&alone).unwrap();
    }

    #[test]
    fn package_copies_wasm_and_writes_the_shell() {
        let root = fixture("package-basic");
        write_wasm(&root, &debug(), b"wasm-bytes");
        let opt = FakeWasmOpt(|_: &Path, _: &Path| -> Result<bool, String> {
            panic!("wasm-opt should not run for a debug build")
        });
        let summary = package(&opt, &rules(), &root, &root.join("target"), &debug(), &[]).unwrap();
        assert!(summary.contains("10 bytes"), "{summary}");
        let dist = root.join("dist/web");
        assert_eq!(fs::read(dist.join("the-game.wasm")).unwrap(), b"wasm-bytes");
        let html = fs::read_to_string(dist.join("index.html")).unwrap();
        assert!(html.contains("<title>The Game</title>"), "{html}");
        assert!(html.contains(r#"load("the-game.wasm");"#), "{html}");
        assert!(!html.contains("%TITLE%") && !html.contains("%WASM%"));
        assert!(
            html.contains(r#"<link rel="license" href="NOTICES.txt">"#),
            "{html}"
        );
        for (name, header) in [
            (
                "mq_js_bundle.js",
                "/*! mq_js_bundle.js: macroquad 0.4.16, MIT. See NOTICES.txt. */\n",
            ),
            (
                "sapp_jsutils.js",
                "/*! sapp_jsutils.js: sapp-jsutils 0.1.7, MIT. See NOTICES.txt. */\n",
            ),
            (
                "quad-storage.js",
                "/*! quad-storage.js: quad-storage 0.1.3, MIT. See NOTICES.txt. */\n",
            ),
            ("gamepad.js", ""),
        ] {
            assert_eq!(
                fs::read_to_string(dist.join(name)).unwrap(),
                format!("{header}{}", shell(name)),
                "{name}"
            );
        }
        // No works given: the heading alone, and no text.
        let notices = fs::read_to_string(dist.join("NOTICES.txt")).unwrap();
        assert!(notices.starts_with("Notices for The Game\n"), "{notices}");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn package_writes_the_works_into_notices_txt() {
        let root = fixture("package-notices");
        write_wasm(&root, &debug(), b"w");
        let opt = FakeWasmOpt(|_: &Path, _: &Path| Ok(false));
        let mut font = Work::new(Section::Game, "files/font/OFL.txt");
        font.texts = vec![notices::Text::new(
            "files/font/OFL.txt",
            "Open Font License\n",
        )];
        let mut works = shell_works();
        works.push(font);
        package(
            &opt,
            &rules(),
            &root,
            &root.join("target"),
            &debug(),
            &works,
        )
        .unwrap();
        let notices = fs::read_to_string(root.join("dist/web/NOTICES.txt")).unwrap();
        for needle in [
            "- mq_js_bundle.js, from macroquad 0.4.16, MIT, https://github.com/not-fl3/macroquad\n",
            "- index.html, from Shuyi Engine ",
            "- files/font/OFL.txt\n",
            "  mq_js_bundle.js, from macroquad 0.4.16 (mq_js_bundle-LICENSE-MIT.txt)\n",
            "Open Font License\n",
            "Copyright (c) 2026 Nick Wentz\n",
            "Apache License",
        ] {
            assert!(notices.contains(needle), "{needle} in {notices}");
        }
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn every_shell_file_has_its_notice_and_the_vendored_ones_are_the_web_files() {
        let table: Vec<&str> = SHELL_NOTICES.iter().map(|(file, _)| *file).collect();
        let shell: Vec<&str> = SHELL.iter().map(|(file, _)| *file).collect();
        assert_eq!(
            table, shell,
            "a file of SHELL needs its row in SHELL_NOTICES"
        );
        // The vendored scripts are the web files of `notices::FILES`, which
        // a test keeps equal to THIRD_PARTY_ASSETS.md.
        let mut vendored: Vec<&str> = SHELL_NOTICES
            .iter()
            .filter(|(_, origin)| *origin == Origin::Vendored)
            .map(|(file, _)| *file)
            .collect();
        vendored.sort_unstable();
        let mut web: Vec<&str> = notices::FILES
            .iter()
            .filter(|file| file.builds.contains(&notices::Build::Web))
            .filter_map(|file| file.file.strip_prefix("crates/cli/web/"))
            .collect();
        web.sort_unstable();
        assert_eq!(vendored, web);
        for (file, origin) in &SHELL_NOTICES {
            assert_eq!(
                super::vendored(file).is_some(),
                *origin == Origin::Vendored,
                "{file}"
            );
        }
    }

    #[test]
    fn a_vendored_script_starts_with_a_one_line_header_and_the_engines_files_get_none() {
        for (file, _) in &SHELL_NOTICES {
            let shipped = shipped_shell_file(file, shell(file));
            if let Some(EngineFile { work, version, .. }) = vendored(file) {
                let (first, rest) = shipped.split_once('\n').unwrap();
                assert!(first.starts_with("/*! "), "{file}: {first}");
                assert!(first.ends_with(" */"), "{file}: {first}");
                assert_eq!(first.matches("*/").count(), 1, "{file}: {first}");
                assert!(first.contains(&format!("{work} {version}, MIT")), "{first}");
                assert!(first.contains(notices::FILE), "{first}");
                assert_eq!(rest, shell(file), "{file}");
            } else {
                assert_eq!(shipped, shell(file), "{file}");
            }
        }
    }

    #[test]
    fn a_game_without_music_or_voices_gets_no_folders() {
        let root = fixture("package-bare");
        write_wasm(&root, &debug(), b"w");
        let bare = Web {
            music: None,
            voice: None,
            ..rules()
        };
        fs::create_dir_all(root.join("music")).unwrap();
        let opt = FakeWasmOpt(|_: &Path, _: &Path| Ok(false));
        package(&opt, &bare, &root, &root.join("target"), &debug(), &[]).unwrap();
        assert!(!root.join("dist/web/music").exists());
        assert!(!root.join("dist/web/voice").exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn package_copies_the_music_tracks_only() {
        let root = fixture("package-music");
        write_wasm(&root, &debug(), b"w");
        let opt = FakeWasmOpt(|_: &Path, _: &Path| Ok(false));
        // No music folder: an empty one is still made.
        package(&opt, &rules(), &root, &root.join("target"), &debug(), &[]).unwrap();
        let music = root.join("dist/web/music");
        assert!(music.is_dir());
        assert_eq!(fs::read_dir(&music).unwrap().count(), 0);
        fs::create_dir_all(root.join("music/sub")).unwrap();
        fs::write(root.join("music/title.ogg"), "ogg").unwrap();
        fs::write(root.join("music/README.md"), "doc").unwrap();
        fs::write(root.join("music/sub/x.ogg"), "nested").unwrap();
        fs::write(music.join("stale.ogg"), "old").unwrap();
        package(&opt, &rules(), &root, &root.join("target"), &debug(), &[]).unwrap();
        let mut names: Vec<_> = fs::read_dir(&music)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        assert_eq!(names, ["title.ogg"]);
        assert_eq!(fs::read_to_string(music.join("title.ogg")).unwrap(), "ogg");
        fs::remove_dir_all(&root).unwrap();
    }

    /// The voice folder goes along whole, manifests and clips only;
    /// without one the package has none.
    #[test]
    fn package_copies_the_voice_folder_if_there_is_one() {
        let root = fixture("package-voice");
        write_wasm(&root, &debug(), b"w");
        let opt = FakeWasmOpt(|_: &Path, _: &Path| Ok(false));
        let voice = root.join("dist/web/voice");
        // A stale copy from an earlier build goes when the source is gone.
        fs::create_dir_all(voice.join("en")).unwrap();
        fs::write(voice.join("en/stale.ogg"), "old").unwrap();
        package(&opt, &rules(), &root, &root.join("target"), &debug(), &[]).unwrap();
        assert!(!voice.exists());
        fs::create_dir_all(root.join("voice/en/scene")).unwrap();
        fs::write(root.join("voice/en/voice.ron"), "manifest").unwrap();
        fs::write(root.join("voice/en/cast.ron"), "cast").unwrap();
        fs::write(root.join("voice/en/scene/line.ogg"), "clip").unwrap();
        fs::write(root.join("voice/en/scene/line.m.ogg"), "clip m").unwrap();
        fs::write(root.join("voice/en/scene/take.wav"), "source").unwrap();
        fs::write(root.join("voice/notes.md"), "doc").unwrap();
        fs::write(root.join("voice/ogg"), "no extension").unwrap();
        package(&opt, &rules(), &root, &root.join("target"), &debug(), &[]).unwrap();
        let read = |path: &str| fs::read_to_string(voice.join(path)).ok();
        assert_eq!(read("en/voice.ron").as_deref(), Some("manifest"));
        assert_eq!(read("en/cast.ron").as_deref(), Some("cast"));
        assert_eq!(read("en/scene/line.ogg").as_deref(), Some("clip"));
        assert_eq!(read("en/scene/line.m.ogg").as_deref(), Some("clip m"));
        assert_eq!(read("en/scene/take.wav"), None);
        assert_eq!(read("notes.md"), None);
        assert_eq!(read("ogg"), None);
        // A file named `voice` is not a voice folder.
        fs::remove_dir_all(root.join("voice")).unwrap();
        fs::write(root.join("voice"), "a file").unwrap();
        package(&opt, &rules(), &root, &root.join("target"), &debug(), &[]).unwrap();
        assert!(!voice.exists());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn copy_tree_counts_the_files_it_copies_and_reports_a_missing_source() {
        let root = fixture("copy-tree");
        fs::create_dir_all(root.join("src/a/b")).unwrap();
        fs::write(root.join("src/one.ogg"), "1").unwrap();
        fs::write(root.join("src/a/two.ogg"), "2").unwrap();
        fs::write(root.join("src/a/b/three.txt"), "3").unwrap();
        let ogg = |path: &Path| path.extension().is_some_and(|ext| ext == "ogg");
        assert_eq!(copy_tree(&root.join("src"), &root.join("dst"), &ogg), Ok(2));
        assert!(root.join("dst/a/two.ogg").is_file());
        assert!(root.join("dst/a/b").is_dir());
        assert!(!root.join("dst/a/b/three.txt").exists());
        let all = copy_tree(&root.join("src"), &root.join("all"), &|_| true);
        assert_eq!(all, Ok(3));
        let error = copy_tree(&root.join("nowhere"), &root.join("dst2"), &ogg).unwrap_err();
        assert!(error.starts_with("read "), "{error}");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn package_runs_wasm_opt_on_release_and_uses_its_output() {
        let root = fixture("package-release-ok");
        write_wasm(&root, &release(), b"unoptimized-bytes");
        let opt = FakeWasmOpt(|_input: &Path, output: &Path| {
            fs::write(output, b"opt").unwrap();
            Ok(true)
        });
        let summary =
            package(&opt, &rules(), &root, &root.join("target"), &release(), &[]).unwrap();
        assert!(summary.contains("3 bytes"), "{summary}");
        assert_eq!(
            fs::read(root.join("dist/web/the-game.wasm")).unwrap(),
            b"opt"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn package_keeps_original_when_wasm_opt_is_not_installed_or_fails() {
        for (name, outcome) in [
            ("missing", Err("not found".to_owned())),
            ("fails", Ok(false)),
        ] {
            let root = fixture(&format!("package-release-{name}"));
            write_wasm(&root, &release(), b"unoptimized");
            let opt = FakeWasmOpt(|_: &Path, _: &Path| outcome.clone());
            let summary =
                package(&opt, &rules(), &root, &root.join("target"), &release(), &[]).unwrap();
            assert!(summary.contains("11 bytes"), "{summary}");
            assert_eq!(
                fs::read(root.join("dist/web/the-game.wasm")).unwrap(),
                b"unoptimized"
            );
            fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn package_reports_a_missing_wasm_artifact() {
        let root = fixture("package-missing-wasm");
        let opt = FakeWasmOpt(|_: &Path, _: &Path| panic!("not reached"));
        let err = package(&opt, &rules(), &root, &root.join("target"), &debug(), &[]).unwrap_err();
        assert!(err.contains("copy"), "{err}");
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn run_reports_cargo_build_failure() {
        // An empty directory has no Cargo.toml, so `cargo build` fails fast
        // (no compilation), without needing a real broken build to test the
        // failure path.
        let root = fixture("run-no-manifest");
        let err = run(&rules(), &root, &debug(), &[]).unwrap_err();
        assert!(err.contains("cargo build"), "{err}");
        fs::remove_dir_all(&root).unwrap();
    }

    /// The version `quad_storage_crate_version()` reports for the
    /// `quad-storage-sys` version pinned in `Cargo.lock`:
    /// `(major << 24) + (minor << 16) + patch`.
    fn locked_quad_storage_sys_version(lock: &str) -> u32 {
        let version = lock
            .split("[[package]]")
            .find(|pkg| pkg.contains("name = \"quad-storage-sys\""))
            .and_then(|pkg| pkg.lines().find_map(|l| l.strip_prefix("version = \"")))
            .and_then(|v| v.strip_suffix('"'))
            .expect("quad-storage-sys is in Cargo.lock");
        let parts: Vec<u32> = version.split('.').map(|p| p.parse().unwrap()).collect();
        (parts[0] << 24) + (parts[1] << 16) + parts[2]
    }

    #[test]
    fn locked_quad_storage_sys_version_encodes_major_minor_patch() {
        let lock = r#"
[[package]]
name = "quad-storage"
version = "9.9.9"

[[package]]
name = "quad-storage-sys"
version = "1.2.3"
"#;
        assert_eq!(
            locked_quad_storage_sys_version(lock),
            (1 << 24) + (2 << 16) + 3
        );
    }

    #[test]
    fn index_html_declares_register_plugin_before_the_bundle() {
        // The strict-mode bundle assigns to `register_plugin`, which throws
        // unless the global already exists.
        let html = shell("index.html");
        let declared = html
            .find("var register_plugin;")
            .expect("index.html declares `var register_plugin;`");
        let bundle = html
            .find(r#"<script src="mq_js_bundle.js">"#)
            .expect("index.html loads mq_js_bundle.js");
        assert!(
            declared < bundle,
            "declare register_plugin before the bundle"
        );
    }

    /// Every name in `text` that starts with `prefix` (which isn't the
    /// tail of a longer identifier), sorted, once each.
    fn identifiers_starting(text: &str, prefix: &str) -> Vec<String> {
        let is_ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let mut found: Vec<String> = text
            .match_indices(prefix)
            .filter(|&(at, _)| !text[..at].chars().next_back().is_some_and(is_ident))
            .map(|(at, _)| {
                let rest = text[at + prefix.len()..].chars();
                let name: String = rest.take_while(|&c| is_ident(c)).collect();
                format!("{prefix}{name}")
            })
            .collect();
        found.sort();
        found.dedup();
        found
    }

    #[test]
    fn identifiers_starting_finds_whole_names_once() {
        let text = "a.shuyi_pad_poll = f; x_shuyi_pad_no(); shuyi_pad_axis(shuyi_pad_poll())";
        assert_eq!(
            identifiers_starting(text, "shuyi_pad_"),
            ["shuyi_pad_axis", "shuyi_pad_poll"]
        );
        let got = identifiers_starting(text, "nope_");
        assert!(got.is_empty(), "{got:?}");
    }

    /// The number after `before` on the line of `text` that starts with it.
    fn number_after(text: &str, before: &str) -> u32 {
        let line = text
            .lines()
            .find_map(|l| l.trim().strip_prefix(before))
            .unwrap_or_else(|| panic!("no line starting `{before}`"));
        line.trim_end_matches(';').parse().unwrap()
    }

    #[test]
    fn gamepad_js_provides_exactly_the_functions_the_app_imports() {
        // A function missing on either side only shows up in the browser,
        // as a warning and a pad that does nothing.
        let js = shell("gamepad.js");
        let provided: Vec<String> = identifiers_starting(js, "importObject.env.shuyi_pad_")
            .iter()
            .map(|name| name.replace("importObject.env.", ""))
            .collect();
        let imported: Vec<String> = identifiers_starting(PADS_WEB, "safe fn shuyi_pad_")
            .iter()
            .map(|name| name.replace("safe fn ", ""))
            .collect();
        assert!(
            imported.len() >= 5,
            "found only {imported:?} in pads/web.rs"
        );
        assert_eq!(provided, imported);
    }

    #[test]
    fn gamepad_js_version_matches_the_app() {
        // The loader logs a version-mismatch error unless the JS plugin's
        // `version` equals what `shuyi_gamepad_crate_version()` returns.
        let js = shell("gamepad.js");
        assert_eq!(
            number_after(js, "var VERSION = "),
            number_after(PADS_WEB, "const PLUGIN_VERSION: u32 = ")
        );
        assert!(js.contains("version: VERSION"));
        assert!(js.contains(r#"name: "shuyi_gamepad""#));
        assert!(PADS_WEB.contains("fn shuyi_gamepad_crate_version() -> u32"));
    }

    #[test]
    fn index_html_loads_gamepad_js_between_the_bundle_and_the_game() {
        // The plugin registers itself with the bundle's
        // `miniquad_add_plugin`, and must have done so before `load(...)`.
        let html = shell("index.html");
        let at = |needle: &str| {
            html.find(needle)
                .unwrap_or_else(|| panic!("index.html has `{needle}`"))
        };
        let bundle = at(r#"<script src="mq_js_bundle.js">"#);
        let gamepad = at(r#"<script src="gamepad.js">"#);
        let load = at(r#"load("%WASM%")"#);
        assert!(bundle < gamepad && gamepad < load);
    }

    #[test]
    fn quad_storage_js_version_matches_the_locked_crate() {
        // The loader logs a version-mismatch error unless the JS plugin's
        // `version` equals `quad_storage_crate_version()`.
        let lock = Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .map(|dir| dir.join("Cargo.lock"))
            .find(|lock| lock.is_file())
            .expect("the workspace has a Cargo.lock");
        let lock = fs::read_to_string(lock).unwrap();
        let expected = format!("version: {}", locked_quad_storage_sys_version(&lock));
        assert!(
            shell("quad-storage.js")
                .lines()
                .any(|line| line.trim() == expected),
            "web/quad-storage.js must declare `{}`",
            expected.trim_end()
        );
    }
}
