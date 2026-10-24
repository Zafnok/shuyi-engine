//! [`BasicTools`]: the [`GameTools`] of a game on the short path
//! (ADR-0036), from its `BasicGame`: `frame-png`, `lang-status`, the
//! contract's `check`, `run` and `scene` (a session with no scene), and
//! the controls `check --target` checks. A game that adds a scene, a
//! record, bots, importers or a bake writes its own `GameTools` and calls
//! these methods for the rest.

use std::fmt::Debug;

use shuyi_assets::ContentError;
use shuyi_assets::keymap::ActionSet;
use shuyi_platform::BasicGame;
use shuyi_platform::harness::Harness;
use shuyi_ui::GameData;
use shuyi_ui::basic::{BasicData, FONT_PNG};
use shuyi_ui::input::fit::{Controls, GameControls, ScreenActions};

use crate::contract::session::{Headless, Session};
use crate::frame_png::{Image, Painter, Script, Shot};
use crate::game::{GameArg, GameLang, GameOption, GameTools};

/// The tools of a game on the short path.
#[derive(Debug)]
pub struct BasicTools<A: ActionSet, T: Debug + 'static = ()> {
    game: BasicGame<A, T>,
}

/// No option of the game's own.
fn no_options(options: &[GameArg]) -> Result<(), String> {
    match options.first() {
        Some(option) => Err(format!("unknown option {}", option.flag)),
        None => Ok(()),
    }
}

impl<A: ActionSet, T: Debug + 'static> BasicTools<A, T> {
    /// The tools of `game`.
    pub fn new(game: BasicGame<A, T>) -> Self {
        Self { game }
    }

    /// The game in the Harness, at its first screen; or its files'
    /// problems, one per line, or the module that can't be installed.
    pub fn harness(&self) -> Result<Harness<BasicData<A, T>>, String> {
        let game = self.game.game(self.game.ctx()?);
        Ok(Harness::new(game.map_err(|e| e.to_string())?))
    }

    /// The frame `h` shows, each console pixel `scale` × `scale` pixels,
    /// painted with the game's font and pictures.
    pub fn picture(h: &Harness<BasicData<A, T>>, scale: u32) -> Result<Image, String> {
        let ctx = h.ctx();
        let data = &ctx.game;
        let png = data.bundle.bytes(FONT_PNG).ok_or("no font atlas picture")?;
        let clear = data.blank_cell(&ctx.palette).bg;
        let painter = Painter::new(
            data.font.clone(),
            png,
            &data.images,
            |path| data.bundle.bytes(path),
            clear,
        )?;
        Ok(painter.render(h.game().buffer(), scale))
    }
}

impl<A: ActionSet, T: Debug + 'static> GameTools for BasicTools<A, T> {
    fn frame_options(&self) -> &[GameOption] {
        &[]
    }

    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        no_options(options)?;
        let mut h = self.harness()?;
        script.play(&mut h);
        Ok(Shot {
            image: Self::picture(&h, scale.unwrap_or(1))?,
            shows: h.top_screen().to_owned(),
            warning: None,
        })
    }

    fn lang(&self) -> Result<GameLang, String> {
        let data = self.game.data().map_err(|errors| {
            let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
            lines.join("\n")
        })?;
        Ok(GameLang {
            lang: data.lang().clone(),
            variants: Vec::new(),
        })
    }

    fn check(&self) -> Result<Vec<ContentError>, String> {
        Ok(self.game.data().err().unwrap_or_default())
    }

    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        no_options(options)?;
        let session = Headless::new(self.harness()?)
            .with_picture(|h, scale| Self::picture(h, scale.unwrap_or(1)));
        Ok(Box::new(session))
    }

    /// Its keymap and every screen it can show (`BasicGame::screens`).
    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        let controls = self.game.ctx().map(|mut ctx| {
            let keymap = ctx.game.keymap_def().clone();
            let screens = self.game.every_screen(&mut ctx);
            let game = screens
                .iter()
                .fold(GameControls::new(keymap), |game, screen| {
                    game.with(ScreenActions::of(screen.as_ref()))
                });
            Box::new(game) as Box<dyn Controls>
        });
        Some(controls)
    }
}

#[cfg(test)]
mod tests;
