//! What the commands that run the game need from it: [`GameTools`].
//!
//! The `shuyi` binary knows no game. A game gives its tools in a program
//! of its own, which runs [`crate::main_with`] with them; `shuyi.ron`
//! names that program ([`crate::project::GameProgram`]) and `shuyi` runs
//! it with `cargo run` for the commands that run the game: `frame-png`,
//! `lang-status`, and the contract's `check`, `run`, `scene`, `replay`,
//! `bots`, `import` and `bake`.

use std::process::Command;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::format::Formats;
use shuyi_assets::lang::Lang;
use shuyi_assets::pack::Bake;

use crate::contract::bots::{BotsInput, BotsOutput};
use crate::contract::import::Importers;
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

    /// `import`: the kinds and formats to read a file with. The engine's
    /// standard ones unless the game adds its own.
    fn importers(&self) -> Importers {
        Importers::standard()
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
        assert_eq!(Plain.importers().kinds(), Importers::standard().kinds());
        assert_eq!(
            Plain.bake(Some("desktop")).unwrap_err(),
            "the game's tools don't offer bake and check --baked (GameTools::bake)"
        );
    }
}
