//! The project file, `shuyi.ron`, at a game's root: which paths each check
//! scans, the game's tool program (for the commands that run the game),
//! the font atlas's rules, the web build's package and the private
//! repository's settings, and the targets the game ships on. A command
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

/// A project: its root and what its `shuyi.ron` says.
#[derive(Debug, Clone, PartialEq)]
pub struct Project {
    /// The folder holding `shuyi.ron`.
    pub root: PathBuf,
    /// What it says.
    pub config: Config,
}

impl Project {
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
pub struct Config {
    /// What the project calls this tool in what it prints (`usage: …`,
    /// generated files): `shuyi` unless it says otherwise.
    pub command: Option<String>,
    /// The game's tool program, for `frame-png` and `lang-status`.
    pub game: Option<GameProgram>,
    /// `check-keys`.
    pub check_keys: Option<CheckKeys>,
    /// `check-text`.
    pub check_text: Option<CheckText>,
    /// `font-atlas`.
    pub font_atlas: Option<FontAtlas>,
    /// `web`.
    pub web: Option<Web>,
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
        let mut out: Vec<String> = ["run", "--quiet", "-p", &self.package]
            .map(str::to_owned)
            .to_vec();
        if let Some(bin) = &self.bin {
            out.extend(["--bin".to_owned(), bin.clone()]);
        }
        if let Some(example) = &self.example {
            out.extend(["--example".to_owned(), example.clone()]);
        }
        if !self.features.is_empty() {
            out.extend(["--features".to_owned(), self.features.join(",")]);
        }
        out.push("--".to_owned());
        out.extend(self.args.iter().cloned());
        out.push(command.to_owned());
        out.extend(args.iter().cloned());
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
    /// Files whose `// check-keys: keyboard picture` comments are
    /// honoured.
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
}
