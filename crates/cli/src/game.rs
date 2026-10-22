//! What the commands that run the game need from it: [`GameTools`].
//!
//! The `shuyi` binary knows no game. A game gives its tools in a program
//! of its own, which runs [`crate::main_with`] with them; `shuyi.ron`
//! names that program ([`crate::project::GameProgram`]) and `shuyi` runs
//! it with `cargo run` for the commands that run the game: `frame-png`,
//! `lang-status`, and the contract's `check`, `run`, `scene`, `replay`,
//! `bots`, `playtest`, `import` and `bake`.

use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::format::Formats;
use shuyi_assets::lang::Lang;
use shuyi_assets::onebit::PatternMapDef;
use shuyi_assets::pack::Bake;
use shuyi_assets::palette::PaletteDef;
use shuyi_ui::input::fit::Controls;

use shuyi_core::bots::playtest::PlaytestReport;

use crate::contract::bots::{BotsInput, BotsOutput};
use crate::contract::import::Importers;
use crate::contract::playtest::PlaytestInput;
use crate::contract::replay::{Record, ReplayRun};
use crate::contract::session::Session;
use crate::frame_png::{Script, Shot};
use crate::project::Project;

/// An option a game adds to `frame-png`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameOption {
    /// The flag (`--light`).
    pub flag: &'static str,
    /// What its value is called in the usage text (`NAME`), if it takes
    /// one.
    pub value: Option<&'static str>,
    /// What it does, for the usage text (lines after the first are
    /// indented under it).
    pub help: &'static str,
}

/// A game option as given.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GameArg {
    /// Its flag.
    pub flag: String,
    /// Its value, if it takes one.
    #[serde(default)]
    pub value: Option<String>,
}

/// A game's languages, for `lang-status`.
#[derive(Debug, Clone)]
pub struct GameLang {
    /// The game's English and its packs, with the data's and dialogue's
    /// English.
    pub lang: Lang,
    /// The variants of a pack's line, in the pack files' order (`text_m`,
    /// `text_f`: `["m", "f"]`).
    pub variants: Vec<String>,
}

/// What a game bakes into its pack ([`GameTools::bake`]).
#[derive(Debug)]
pub struct GameBake {
    /// Which assets of which kinds, and the game's stamp.
    pub bake: Bake,
    /// The bundle of its sources: the layers it ships for the target.
    pub bundle: Bundle,
    /// The formats its loaders read the sources with (the pack's own are
    /// added to them).
    pub formats: Formats,
}

/// A skin's one-bit look ([`GameTools::one_bit`]): its palette of roles
/// and the pattern map beside it (ADR-0008 §4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneBitSkin {
    /// The palette's file, as messages show it.
    pub palette_file: String,
    /// The palette: each role's colour.
    pub palette: PaletteDef,
    /// The pattern map's file, as messages show it
    /// (`files/palette@onebit.ron`).
    pub map_file: String,
    /// The pattern map.
    pub map: PatternMapDef,
}

/// What a game gives the commands that run it. Only `frame-png`'s and
/// `lang-status`'s parts are required; a contract operation whose part a
/// game leaves out answers that the game's tools don't offer it.
pub trait GameTools {
    /// The options the game adds to `frame-png`, which also start its
    /// game for `run`, `scene` and `bots` (none: `&[]`).
    fn frame_options(&self) -> &[GameOption];

    /// What else `frame-png --help` says about the game's pictures.
    fn frame_notes(&self) -> &'static str {
        ""
    }

    /// Starts the game in its Harness as `options` say, plays `script`
    /// and paints the frame, each console pixel `scale` × `scale` (the
    /// game's choice if `None`).
    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String>;

    /// `frame-png --target <id>` on a one-bit target: as
    /// [`frame`](Self::frame), but painted as the target's display shows
    /// it, most simply with [`to_one_bit`](shuyi_ui::frame::onebit::to_one_bit)
    /// and [`one_bit_picture`](crate::frame_png::one_bit_picture), each of
    /// its pixels `scale` × `scale` (1 if `None`).
    fn frame_one_bit(
        &self,
        target: &str,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        let _ = (target, options, script, scale);
        Err(unsupported(
            "frame-png --target on a one-bit target",
            "frame_one_bit",
        ))
    }

    /// The game's languages.
    fn lang(&self) -> Result<GameLang, String>;

    /// `check`: every error of every validator of the game's content,
    /// each with its file and, when known, its line.
    fn check(&self) -> Result<Vec<ContentError>, String> {
        Err(unsupported("check", "check"))
    }

    /// `run` and `scene`: the game started headless as `options` say
    /// (its [`frame_options`](Self::frame_options)), most simply a
    /// [`Headless`](crate::contract::session::Headless) over its Harness.
    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let _ = options;
        Err(unsupported("run and scene", "session"))
    }

    /// `replay`: `record`, in the game's record format, re-run by its
    /// rules (for a turn game, [`turns`](crate::contract::replay::turns)).
    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        let _ = record;
        Err(unsupported("replay", "replay"))
    }

    /// `bots`: the games `input` asks for, played by the game's bots and
    /// summed up ([`tally`](crate::contract::bots::tally)).
    fn bots(&self, input: &BotsInput) -> Result<BotsOutput, String> {
        let _ = input;
        Err(unsupported("bots", "bots"))
    }

    /// `playtest`: the tries `input` asks for, of each of the game's named
    /// bots, measured and judged by the game's checks
    /// ([`run`](shuyi_core::bots::playtest::run)).
    fn playtest(&self, input: &PlaytestInput) -> Result<PlaytestReport, String> {
        let _ = input;
        Err(unsupported("playtest", "playtest"))
    }

    /// `import`: the kinds and formats to read a file with. The engine's
    /// standard ones unless the game adds its own.
    fn importers(&self) -> Importers {
        Importers::standard()
    }

    /// `check --target`: the game's keymap and every screen it can show,
    /// for the check that its controls fit each target (ADR-0008 §3), most
    /// simply a [`GameControls`](shuyi_ui::input::fit::GameControls).
    /// `None`, the default, doesn't offer them: `check --target` then
    /// checks the display alone and says so. An error (the game's files
    /// don't load) fails the check.
    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        None
    }

    /// `check --target` on a one-bit target: the skin's palette and its
    /// pattern map for `target`, for the check that every role has
    /// patterns and the roles that tell things apart stay distinct
    /// (ADR-0008 §4; `shuyi_assets::onebit::check`). `None`, the default,
    /// doesn't offer them, and `check --target` says they weren't checked.
    /// An error (the files don't load) fails the check.
    fn one_bit(&self, _target: &str) -> Option<Result<OneBitSkin, String>> {
        None
    }

    /// `bake` and `check --baked`: what the game bakes for `target`
    /// (`None` for its default bundle), from which bundle, read with which
    /// formats. The target is the game's to pick layers and kinds by.
    fn bake(&self, target: Option<&str>) -> Result<GameBake, String> {
        let _ = target;
        Err(unsupported("bake and check --baked", "bake"))
    }
}

/// What a game whose tools don't offer `op` answers: they lack the
/// method `method`.
pub fn unsupported(op: &str, method: &str) -> String {
    format!("the game's tools don't offer {op} (GameTools::{method})")
}

/// Runs `command` with `args` in the game's tool program of `project`
/// and returns its exit code.
pub fn run_in_game_program(project: &Project, command: &str, args: &[String]) -> u8 {
    let Some(program) = &project.config.game else {
        eprintln!(
            "{command}: {} names no game tool program (`game`), and {command} runs the game",
            crate::project::FILE
        );
        return 2;
    };
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    match Command::new(cargo)
        .args(program.cargo_args(command, args))
        .current_dir(&project.root)
        .status()
    {
        Ok(status) => status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(1),
        Err(e) => {
            eprintln!("{command}: cannot run cargo: {e}");
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A game that adds nothing to `frame-png`.
    struct Plain;

    impl GameTools for Plain {
        fn frame_options(&self) -> &[GameOption] {
            &[]
        }

        fn frame(&self, _: &[GameArg], _: &Script, _: Option<u32>) -> Result<Shot, String> {
            Err("no frame".to_owned())
        }

        fn lang(&self) -> Result<GameLang, String> {
            Ok(GameLang {
                lang: Lang::default(),
                variants: Vec::new(),
            })
        }
    }

    #[test]
    fn a_game_adds_no_options_or_notes_unless_it_says_so() {
        assert_eq!(Plain.frame_options(), &[]);
        assert_eq!(Plain.frame_notes(), "");
        assert!(Plain.lang().unwrap().variants.is_empty());
        assert!(Plain.frame(&[], &Script::default(), None).is_err());
    }

    #[test]
    fn a_game_offers_no_contract_operation_unless_it_says_so() {
        assert_eq!(
            Plain.check().unwrap_err(),
            "the game's tools don't offer check (GameTools::check)"
        );
        let session = Plain.session(&[]).err().unwrap();
        assert_eq!(
            session,
            "the game's tools don't offer run and scene (GameTools::session)"
        );
        let record = Record {
            file: "a.ron".to_owned(),
            text: String::new(),
        };
        assert!(
            Plain
                .replay(&record)
                .unwrap_err()
                .contains("GameTools::replay")
        );
        let bots = Plain.bots(&BotsInput::default()).unwrap_err();
        assert!(bots.contains("GameTools::bots"));
        let playtest = Plain.playtest(&PlaytestInput::default()).unwrap_err();
        assert!(playtest.contains("GameTools::playtest"));
        assert_eq!(Plain.importers().kinds(), Importers::standard().kinds());
        assert!(Plain.controls().is_none());
        assert!(Plain.one_bit("playdate").is_none());
        assert_eq!(
            Plain
                .frame_one_bit("playdate", &[], &Script::default(), None)
                .unwrap_err(),
            "the game's tools don't offer frame-png --target on a one-bit target \
             (GameTools::frame_one_bit)"
        );
        assert_eq!(
            Plain.bake(Some("desktop")).unwrap_err(),
            "the game's tools don't offer bake and check --baked (GameTools::bake)"
        );
    }
}
