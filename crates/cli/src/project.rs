//! The project file, `shuyi.ron`, at a game's root: who the game is (its
//! `app` part), which paths each check scans, the game's tool program (for the commands that run the game),
//! the font atlas's rules, the web build's package, the game's own license
//! texts and the private repository's settings, and the targets the game
//! ships on. A command
//! whose part is missing says so.
//!
//! Every path is relative to the project's root (the folder holding
//! `shuyi.ron`) and written with `/`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::contract::codes::{self, Code};

/// The project file's name.
pub const FILE: &str = "shuyi.ron";

/// A project: its root and what its `shuyi.ron` says. Build it with
/// [`Project::new`] or [`Project::load`] (ADR-0039).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Project {
    /// The folder holding `shuyi.ron`.
    pub root: PathBuf,
    /// What it says.
    pub config: Config,
}

impl Project {
    /// The project at `root` whose `shuyi.ron` says `config`.
    pub fn new(root: impl Into<PathBuf>, config: Config) -> Self {
        Self {
            root: root.into(),
            config,
        }
    }

    /// The project whose `shuyi.ron` is in `root`.
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join(FILE);
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let config = Config::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(Self {
            root: root.to_path_buf(),
            config,
        })
    }

    /// The project of the first folder from `start` up that holds a
    /// `shuyi.ron`, or the problem code and the message why there is
    /// none: no file ([`codes::NO_PROJECT`]), or one that doesn't read
    /// ([`codes::PROJECT_FILE`]).
    pub fn find(start: &Path) -> Result<Self, (Code, String)> {
        let root = start
            .ancestors()
            .find(|dir| dir.join(FILE).is_file())
            .ok_or_else(|| {
                (
                    codes::NO_PROJECT,
                    format!("no {FILE} in {} or a folder above it", start.display()),
                )
            })?;
        Self::load(root).map_err(|e| (codes::PROJECT_FILE, e))
    }

    /// The part `part` of the config, named `name`, or the error that
    /// names it as missing (a [`codes::PROJECT_FILE`] problem).
    pub fn part<'a, T>(&self, part: &'a Option<T>, name: &str) -> Result<&'a T, String> {
        part.as_ref()
            .ok_or_else(|| format!("{FILE} has no `{name}`"))
    }
}

/// What `shuyi.ron` says.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
// Parsed, never built by a literal outside this crate: it may gain parts
// (ADR-0039).
#[non_exhaustive]
pub struct Config {
    /// What the project calls this tool in what it prints (`usage: …`,
    /// generated files): `shuyi` unless it says otherwise.
    pub command: Option<String>,
    /// Who the game is: its name, id, version and storage's name, which
    /// `check --target` checks and `app-info` writes as Rust
    /// ([`crate::app_info`]).
    pub app: Option<AppPart>,
    /// The game's tool program, for `frame-png` and `lang-status`.
    pub game: Option<GameProgram>,
    /// `check-keys`.
    pub check_keys: Option<CheckKeys>,
    /// `check-text`.
    pub check_text: Option<CheckText>,
    /// The game's rules package, which `lint` checks is fenced: clippy's
    /// bans and no screen, device or tool crate under it (ADR-0032 §4).
    pub rules: Option<String>,
    /// `font-atlas`.
    pub font_atlas: Option<FontAtlas>,
    /// `web`.
    pub web: Option<Web>,
    /// The game's own license texts, which its packages carry.
    pub notices: Option<Notices>,
    /// What `export` turns on and ships beside the program.
    pub export: Option<ExportPart>,
    /// What the game's packages may hold, which `export` holds each final
    /// package to ([`crate::export::disclosure`]).
    pub disclosure: Option<Disclosure>,
    /// `glyph-tiles`.
    pub glyph_tiles: Option<GlyphTiles>,
    /// `private-assets`.
    pub private_assets: Option<PrivateAssets>,
    /// `check --target`: the targets the game ships on, by the id of
    /// their profile (ADR-0008 §1), and how it shows its frame on each.
    /// It lists only those.
    pub targets: BTreeMap<String, TargetSetting>,
    /// The folder of the game's own target profiles, if it has any: each
    /// adds a target, or replaces the engine's of its id.
    pub target_profiles: Option<String>,
}

impl Config {
    /// Parses the text of a `shuyi.ron`. A part is written as it is, not
    /// in `Some(…)` (RON's `implicit_some` is on).
    pub fn parse(text: &str) -> Result<Self, String> {
        ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME)
            .from_str(text)
            .map_err(|e| e.to_string())
    }

    /// The tool's name in what it prints.
    pub fn command(&self) -> &str {
        self.command.as_deref().unwrap_or("shuyi")
    }
}

/// Who the game is, the same on every target ([`crate::app_info`],
/// ADR-0059).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct AppPart {
    /// The name players see.
    pub name: String,
    /// Its reverse-DNS id, unique to the game: `com.example.moon-garden`.
    pub id: String,
    /// Its version, semver; without it, its crate's
    /// (`CARGO_PKG_VERSION`).
    #[serde(default)]
    pub version: Option<String>,
    /// A number that only grows from one build handed out to the next.
    #[serde(default = "first_build")]
    pub build: u32,
    /// Who publishes it.
    #[serde(default)]
    pub publisher: String,
    /// The name its saves are kept under; without it, the id.
    #[serde(default)]
    pub storage: Option<String>,
    /// Where `app-info` writes the Rust file (`crates/game/src/app_info.rs`);
    /// without it, no Rust is written.
    #[serde(default)]
    pub code: Option<String>,
    /// The game's icon: one square PNG, 512 pixels a side or more, which
    /// the tools downscale to every size a target asks (ADR-0061).
    #[serde(default)]
    pub icon: Option<String>,
    /// The folder where `app-info` writes the window's icons, `16.png`,
    /// `32.png` and `64.png` (`crates/game/icon`); `code` reads them.
    #[serde(default)]
    pub window_icons: Option<String>,
    /// The window's icons drawn by hand: a side in pixels and the square
    /// PNG of that side to use for it, in place of downscaling `icon`
    /// (ADR-0069). `icon` still makes the sides this doesn't name.
    #[serde(default)]
    pub icon_sizes: BTreeMap<u32, String>,
    /// The Playdate launcher's list icon: a 32 × 32 black and white PNG, in
    /// place of a one-bit bake of `icon`.
    #[serde(default)]
    pub playdate_icon: Option<String>,
    /// The Playdate launcher's card: a 350 × 155 black and white PNG, in
    /// place of a one-bit bake of `icon`.
    #[serde(default)]
    pub playdate_card: Option<String>,
}

/// A game's first build.
const fn first_build() -> u32 {
    1
}

impl AppPart {
    /// The game called `name` with the id `id`, and every other field at
    /// its default.
    pub fn new(name: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
            version: None,
            build: first_build(),
            publisher: String::new(),
            storage: None,
            code: None,
            icon: None,
            window_icons: None,
            icon_sizes: BTreeMap::new(),
            playdate_icon: None,
            playdate_card: None,
        }
    }

    /// The name its saves are kept under: its `storage`, else its id.
    pub fn storage(&self) -> &str {
        self.storage.as_deref().unwrap_or(&self.id)
    }
}

/// How a game shows its frame on one target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetSetting {
    /// Its frame's width and height, in logical pixels.
    pub frame: (u16, u16),
    /// How the frame is scaled to the display.
    pub scale: Scale,
    /// The height of the smallest text on screen, in logical pixels: the
    /// font's cell height.
    pub text_px: u16,
}

/// How a frame is scaled to a display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scale {
    /// By the largest whole number that fits: pixels stay square and
    /// sharp.
    Integer,
    /// By the largest factor that fits, whole or not, smaller than 1 too.
    Fit,
}

/// The game's tool program: a program of the game that runs
/// [`crate::main_with`] with the game's [`crate::GameTools`]. `shuyi`
/// runs it with `cargo run` for the commands that need the game.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GameProgram {
    /// Its package.
    pub package: String,
    /// Its binary, if the package has more than one.
    pub bin: Option<String>,
    /// Or its example.
    pub example: Option<String>,
    /// Features to build it with.
    pub features: Vec<String>,
    /// Arguments before the command's.
    pub args: Vec<String>,
}

impl GameProgram {
    /// The `cargo` arguments that run `command` with `args` in it.
    pub fn cargo_args(&self, command: &str, args: &[String]) -> Vec<String> {
        let mut out = self.selected(&["run", "--quiet"]);
        out.push("--".to_owned());
        out.extend(self.args.iter().cloned());
        out.push(command.to_owned());
        out.extend(args.iter().cloned());
        out
    }

    /// The `cargo` arguments that build it as [`Self::cargo_args`] runs
    /// it, so that run has nothing left to compile.
    pub fn cargo_build_args(&self) -> Vec<String> {
        self.selected(&["build"])
    }

    /// `first`, then the arguments that pick the program and its
    /// features.
    fn selected(&self, first: &[&str]) -> Vec<String> {
        let mut out: Vec<String> = first.iter().map(|s| (*s).to_owned()).collect();
        out.extend(["-p".to_owned(), self.package.clone()]);
        if let Some(bin) = &self.bin {
            out.extend(["--bin".to_owned(), bin.clone()]);
        }
        if let Some(example) = &self.example {
            out.extend(["--example".to_owned(), example.clone()]);
        }
        if !self.features.is_empty() {
            out.extend(["--features".to_owned(), self.features.join(",")]);
        }
        out
    }
}

/// What `check-keys` scans and lets through.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CheckKeys {
    /// Rust source folders scanned (tests left out).
    pub rust: Vec<String>,
    /// Asset folders whose `.ron` and `.dlg` text is scanned.
    pub assets: Vec<String>,
    /// Asset files that are no player text: the keymap file, which names
    /// keys by design.
    pub not_text: Vec<String>,
    /// The key pipeline: files that may name key and button types and key
    /// names.
    pub key_files: Vec<String>,
    /// The only file that may read the keyboard (`KeyCode`, `is_key_down`,
    /// …).
    pub platform_file: String,
    /// Files only compiled for tests (besides `tests.rs`, `*_tests.rs` and
    /// `tests/`).
    pub test_support: Vec<String>,
    /// Files where a comment marking a keyboard picture (the text of
    /// `check_keys::MARKER`) is honoured.
    pub picture_files: Vec<String>,
    /// What every error says to do about it.
    pub hint: String,
}

/// What `check-text` scans and lets through.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CheckText {
    /// The screens' Rust source folder.
    pub dir: String,
    /// The most player-facing literals allowed.
    pub max_literals: usize,
    /// Where the data's names are read on purpose: the helpers every
    /// screen uses.
    pub words_file: String,
    /// Files left out besides tests (a path ending in `/` leaves out a
    /// folder): the debug screens, a frame's text form for snapshots.
    pub skip: Vec<String>,
    /// Files only compiled for tests (besides `tests.rs`, `*_tests.rs` and
    /// `tests/`).
    pub test_support: Vec<String>,
    /// What a literal is told to do about it.
    pub hint: String,
    /// What a `.name` read is told to do about it.
    pub name_hint: String,
}

/// `font-atlas`'s rules.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct FontAtlas {
    /// The glyphs an atlas must have, in any order.
    pub required_glyphs: String,
    /// The README the generated `atlas.ron` points at, if any.
    pub readme: Option<String>,
}

/// `glyph-tiles`: a tileset of glyphs on colored squares
/// ([`crate::glyph_tiles`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GlyphTiles {
    /// The folder holding the font atlas (`atlas.png` and `atlas.ron`).
    pub font: String,
    /// The palette file whose colors the tiles name.
    pub palette: String,
    /// Where the PNG goes.
    pub out: String,
    /// A tile's side, in pixels.
    pub tile: u32,
    /// Tiles per row of the picture.
    pub columns: u32,
    /// The tiles, in order.
    pub tiles: Vec<GlyphTile>,
}

/// One tile of a [`GlyphTiles`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlyphTile {
    /// Its glyphs, side by side and centred.
    pub glyphs: String,
    /// The glyphs' color, by its palette name.
    pub fg: String,
    /// The square's color, by its palette name; `None` leaves it clear.
    #[serde(default)]
    pub bg: Option<String>,
}

/// The web build.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Web {
    /// The game's binary's package.
    pub package: String,
    /// Its binary, whose `.wasm` the page loads.
    pub bin: String,
    /// The page's title.
    pub title: String,
    /// The folder of music tracks copied beside the page (`.ogg` only),
    /// if the game has one.
    pub music: Option<String>,
    /// The folder of voice clips copied beside the page (`.ogg` and
    /// `.ron`), if the game has one.
    pub voice: Option<String>,
    /// The features of the package a `--<feature>` flag turns on.
    pub features: Vec<String>,
    /// Where the page goes.
    pub dist: String,
}

/// The game's own license texts (its fonts', say): each file goes whole
/// into the `NOTICES.txt` of every package `shuyi` makes
/// ([`crate::notices`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
#[non_exhaustive]
pub struct Notices {
    /// The files, from the project's root.
    pub texts: Vec<String>,
}

/// What `export` builds and ships for the desktop and web targets, on top of
/// the program and its notes. The Playdate's package, a `.pdx`, takes none of
/// it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
#[non_exhaustive]
pub struct ExportPart {
    /// The package whose program ships, when it isn't the `game` part's (a
    /// game whose tool program is a package of its own).
    pub package: Option<String>,
    /// The features of the game's package a build turns on. `link`, the
    /// debug build's live link, is refused.
    pub features: Vec<String>,
    /// The files and folders copied into the package.
    pub beside: Vec<Beside>,
}

/// A file or folder of the game that ships in its package.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Beside {
    /// Where it is, from the project's root.
    pub path: String,
    /// Its name in the package; without it, the one it has.
    #[serde(default)]
    pub name: Option<String>,
    /// Whether the package goes without it when it isn't there; otherwise
    /// that refuses the export.
    #[serde(default)]
    pub optional: bool,
}

/// What the game's packages may hold, the same for every target: a path is
/// matched whole, with no pattern, from the package's root or the pack's.
/// What the exporter itself writes (the program, `NOTICES.txt`, the web
/// shell, the pack's file) needs no line here.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
#[non_exhaustive]
pub struct Disclosure {
    /// The files of the game a package may hold beside what the exporter
    /// writes (`music/title.ogg`, `audio/step.pda`).
    pub allowed_files: Vec<String>,
    /// The files a pack may hold (`maps/first.ron`).
    pub allowed_pack_entries: Vec<String>,
    /// The paths nothing may hold: no file of the package or beside it, no
    /// file of a pack, and no program as text (a file a game embeds keeps
    /// its path in the program).
    pub forbidden_paths: Vec<String>,
    /// The texts nothing may hold, each written in a file of what isn't out
    /// yet so that a copy of it can be found: UTF-8 or UTF-16, whole, in any
    /// file of the package, of a pack or beside it.
    pub canaries: Vec<String>,
}

/// The private repository of files the game may not hand out (bought art):
/// checked out into a git-ignored folder at a pinned commit.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct PrivateAssets {
    /// The repository.
    pub url: String,
    /// Where it is checked out, in the project's root.
    pub checkout: String,
    /// The file in the project's root that names the commit to build with.
    pub pin_file: String,
    /// The folders of the repository a checkout made here holds (the first
    /// is the one a new clone starts with).
    pub folders: Vec<String>,
    /// The folder `--library` brings too, if any.
    pub library: Option<String>,
    /// Folders copied out of the checkout into the project's root (each
    /// replaced whole), after a fetch.
    pub copies: Vec<CopyOut>,
    /// What `--pin` checks has a credit, if anything.
    pub credits: Option<Credited>,
    /// What a refused clone or fetch says about who may read the
    /// repository.
    pub no_access: String,
}

/// The files of the private checkout that need a credit before `--pin`
/// writes the pin.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Credited {
    /// The credits file, in the project's root.
    pub file: String,
    /// The folders of the checkout whose files need a credit.
    pub folders: Vec<String>,
    /// The audio manifest, as the credits file's errors name it.
    pub audio_manifest: String,
    /// The folder a credit's `private` paths are in, as its errors name
    /// it.
    pub private_root: String,
    /// What else an uncredited file's message says to do, if anything.
    pub hint: Option<String>,
}

/// A folder copied out of the private checkout.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyOut {
    /// The folder in the checkout, and what its files are called.
    pub from: String,
    /// Where it goes in the project's root.
    pub to: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_file_is_a_project_with_no_parts() {
        let config = Config::parse("()").unwrap();
        assert_eq!(config, Config::default());
        assert_eq!(config.command(), "shuyi");
    }

    #[test]
    fn parts_are_read_and_unknown_fields_refused() {
        let text = r#"(
    command: "cargo shuyi",
    game: (package: "tools", features: ["a", "b"]),
    font_atlas: (required_glyphs: "?ab"),
)"#;
        let config = Config::parse(text).unwrap();
        assert_eq!(config.command(), "cargo shuyi");
        assert_eq!(config.game.as_ref().unwrap().package, "tools");
        assert_eq!(config.font_atlas.unwrap().required_glyphs, "?ab");
        assert!(Config::parse("(colour: 1)").is_err());
        assert!(Config::parse("(web: (name: \"x\"))").is_err());
    }

    #[test]
    fn the_app_part_is_read_with_its_defaults_and_unknown_fields_refused() {
        let config =
            Config::parse(r#"(app: (name: "Moon Garden", id: "com.example.moon-garden"))"#)
                .unwrap();
        let app = config.app.unwrap();
        assert_eq!(app, AppPart::new("Moon Garden", "com.example.moon-garden"));
        assert_eq!((app.build, app.publisher.as_str()), (1, ""));
        assert_eq!((&app.version, &app.code), (&None, &None));
        assert_eq!(app.storage(), "com.example.moon-garden");
        assert!(app.icon_sizes.is_empty());
        let text = r#"(app: (
    name: "Moon Garden",
    id: "com.example.moon-garden",
    version: "1.2.0-beta.1",
    build: 12,
    publisher: "Someone",
    storage: "moon_garden",
    code: "crates/game/src/app_info.rs",
    icon: "art/icon.png",
    window_icons: "crates/game/icon",
    icon_sizes: { 16: "art/icon-16.png", 32: "art/icon-32.png" },
))"#;
        let app = Config::parse(text).unwrap().app.unwrap();
        assert_eq!(app.icon.as_deref(), Some("art/icon.png"));
        assert_eq!(
            app.icon_sizes,
            BTreeMap::from([
                (16, "art/icon-16.png".to_owned()),
                (32, "art/icon-32.png".to_owned()),
            ])
        );
        assert_eq!(app.window_icons.as_deref(), Some("crates/game/icon"));
        assert_eq!(app.version.as_deref(), Some("1.2.0-beta.1"));
        assert_eq!((app.build, app.publisher.as_str()), (12, "Someone"));
        assert_eq!(app.storage(), "moon_garden");
        assert_eq!(app.code.as_deref(), Some("crates/game/src/app_info.rs"));
        assert!(Config::parse(r#"(app: (name: "A", id: "a.b", colour: 1))"#).is_err());
        assert!(
            Config::parse(r#"(app: (name: "A"))"#).is_err(),
            "the id is needed"
        );
    }

    #[test]
    fn the_notices_part_lists_the_games_texts() {
        let config =
            Config::parse(r#"(notices: (texts: ["files/font/Terminus-LICENSE.txt"]))"#).unwrap();
        assert_eq!(
            config.notices.unwrap().texts,
            ["files/font/Terminus-LICENSE.txt"]
        );
        assert_eq!(
            Config::parse("(notices: ())").unwrap().notices,
            Some(Notices::default())
        );
        assert!(Config::parse(r#"(notices: (files: ["a"]))"#).is_err());
    }

    #[test]
    fn the_export_part_names_features_and_what_ships_beside() {
        let text = r#"(export: (
    features: ["art"],
    beside: [
        (path: "music"),
        (path: "voice", optional: true),
        (path: "files/licenses", name: "licenses"),
    ],
))"#;
        let part = Config::parse(text).unwrap().export.unwrap();
        assert_eq!(part.features, ["art"]);
        let row = |path: &str, name: Option<&str>, optional| Beside {
            path: path.to_owned(),
            name: name.map(str::to_owned),
            optional,
        };
        assert_eq!(
            part.beside,
            [
                row("music", None, false),
                row("voice", None, true),
                row("files/licenses", Some("licenses"), false),
            ]
        );
        assert!(Config::parse("(export: (beside: [(optional: true)]))").is_err());
        assert!(Config::parse("(export: (files: []))").is_err());
    }

    #[test]
    fn targets_are_a_table_of_settings_by_id() {
        let text = r#"(
    targets: {
        "desktop": (frame: (800, 512), scale: integer, text_px: 16),
        "web": (frame: (400, 240), scale: fit, text_px: 8),
    },
    target_profiles: "files/targets",
)"#;
        let config = Config::parse(text).unwrap();
        assert_eq!(
            config.targets["desktop"],
            TargetSetting {
                frame: (800, 512),
                scale: Scale::Integer,
                text_px: 16,
            }
        );
        assert_eq!(config.targets["web"].scale, Scale::Fit);
        assert_eq!(config.target_profiles.as_deref(), Some("files/targets"));
        assert!(
            Config::parse("(targets: {\"a\": (frame: (1, 1), scale: none, text_px: 1)})").is_err()
        );
    }

    #[test]
    fn the_game_program_runs_through_cargo() {
        let args =
            |items: &[&str]| -> Vec<String> { items.iter().map(|s| (*s).to_owned()).collect() };
        let plain = GameProgram {
            package: "tools".to_owned(),
            ..GameProgram::default()
        };
        assert_eq!(
            plain.cargo_args("frame-png", &args(&["a.png"])),
            args(&["run", "--quiet", "-p", "tools", "--", "frame-png", "a.png"])
        );
        let example = GameProgram {
            package: "engine".to_owned(),
            bin: Some("b".to_owned()),
            example: Some("toy".to_owned()),
            features: args(&["toy", "x"]),
            args: args(&["--first"]),
        };
        assert_eq!(
            example.cargo_args("lang-status", &[]),
            args(&[
                "run",
                "--quiet",
                "-p",
                "engine",
                "--bin",
                "b",
                "--example",
                "toy",
                "--features",
                "toy,x",
                "--",
                "--first",
                "lang-status"
            ])
        );
        // Built as it is run: the same package, program and features.
        assert_eq!(plain.cargo_build_args(), args(&["build", "-p", "tools"]));
        assert_eq!(
            example.cargo_build_args(),
            args(&[
                "build",
                "-p",
                "engine",
                "--bin",
                "b",
                "--example",
                "toy",
                "--features",
                "toy,x"
            ])
        );
    }

    #[test]
    fn a_project_is_found_above_where_it_starts() {
        let dir = std::env::temp_dir().join(format!("shuyi-project-{}", std::process::id()));
        let deep = dir.join("a/b");
        fs::create_dir_all(&deep).unwrap();
        let _ = fs::remove_file(dir.join(FILE));
        fs::write(dir.join(FILE), "(command: \"t\")").unwrap();
        let project = Project::find(&deep).unwrap();
        assert_eq!(project.root, dir);
        assert_eq!(project.config.command(), "t");
        fs::write(dir.join(FILE), "(nope)").unwrap();
        let (code, error) = Project::find(&deep).unwrap_err();
        assert_eq!(code, codes::PROJECT_FILE);
        assert!(error.contains(FILE), "{error}");
        let missing = project.part(&project.config.web, "web").unwrap_err();
        assert_eq!(missing, "shuyi.ron has no `web`");
        fs::remove_file(dir.join(FILE)).unwrap();
        let (code, error) = Project::find(&deep).unwrap_err();
        assert_eq!(code, codes::NO_PROJECT);
        assert!(error.starts_with("no shuyi.ron in "), "{error}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_new_project_is_its_root_and_config() {
        let config = Config {
            command: Some("cargo shuyi".to_owned()),
            ..Config::default()
        };
        let literal = Project {
            root: PathBuf::from("game"),
            config: config.clone(),
        };
        assert_eq!(Project::new("game", config), literal);
    }
}
