//! What the commands that run the game need from it: [`GameTools`].
//!
//! The `shuyi` binary knows no game. A game gives its tools in a program
//! of its own, which runs [`crate::main_with`] with them; `shuyi.ron`
//! names that program ([`crate::project::GameProgram`]) and `shuyi` runs
//! it with `cargo run` for `frame-png` and `lang-status`.

use std::process::Command;

use shuyi_assets::lang::Lang;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameArg {
    /// Its flag.
    pub flag: String,
    /// Its value, if it takes one.
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

/// What a game gives the commands that run it.
pub trait GameTools {
    /// The options the game adds to `frame-png` (none: `&[]`).
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
}
