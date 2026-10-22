//! The game's tools for the engine's `shuyi` (`cargo shuyi <command>`):
//! `frame-png` (a picture of a frame, painted with the game's own font),
//! `lang-status`, the tool contract's `check`, `run` and `import`,
//! and the controls `check --target` checks ([`controls`]).
//! `scene`, `replay` and `bots` come with a stage and rules: the
//! `GameTools` methods of those names (`shuyi new grid` shows them).
//! `examples/tools.rs` is the tool program that runs them; the tests use
//! them too.

use shuyi_assets::ContentError;
use shuyi_cli::contract::session::{Headless, Session};
use shuyi_cli::frame_png::{Image, Painter, Script, Shot};
use shuyi_cli::{GameArg, GameLang, GameOption, GameTools};
use shuyi_platform::harness::Harness;
use shuyi_ui::GameData;
use shuyi_ui::input::fit::{Controls, GameControls, ScreenActions};

use crate::{Action, Data, FONT_PNG};

/// The game in the Harness, at its title.
pub fn harness() -> Harness<Data> {
    Harness::new(crate::game(crate::ctx()))
}

/// The frame `h` shows, each console pixel `scale` × `scale` pixels.
pub fn picture(h: &Harness<Data>, scale: u32) -> Result<Image, String> {
    let ctx = h.ctx();
    let png = crate::file(FONT_PNG).ok_or("no font atlas picture")?;
    let clear = ctx.game.blank_cell(&ctx.palette).bg;
    let painter = Painter::new(
        ctx.game.font.clone(),
        png,
        &ctx.game.images,
        crate::file,
        clear,
    )?;
    Ok(painter.render(h.game().buffer(), scale))
}

/// The game's keymap and every screen it can show ([`crate::screens`]),
/// for `cargo shuyi check --target`.
pub fn controls() -> Result<GameControls<Action>, String> {
    let keymap = loaded()?.keymap_def().clone();
    let mut ctx = crate::ctx();
    let screens = crate::screens(&mut ctx);
    Ok(screens
        .iter()
        .fold(GameControls::new(keymap), |game, screen| {
            game.with(ScreenActions::of(screen.as_ref()))
        }))
}

/// The game's data, or its errors in a line each.
fn loaded() -> Result<Data, String> {
    Data::load().map_err(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        errors.join("\n")
    })
}

/// No option of the game's own yet.
fn no_options(options: &[GameArg]) -> Result<(), String> {
    match options.first() {
        Some(option) => Err(format!("unknown option {}", option.flag)),
        None => Ok(()),
    }
}

/// The game's tools.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tools;

impl GameTools for Tools {
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
        let mut h = harness();
        script.play(&mut h);
        Ok(Shot {
            image: picture(&h, scale.unwrap_or(1))?,
            shows: h.top_screen().to_owned(),
            warning: None,
        })
    }

    fn lang(&self) -> Result<GameLang, String> {
        Ok(GameLang {
            lang: loaded()?.lang().clone(),
            variants: Vec::new(),
        })
    }

    fn check(&self) -> Result<Vec<ContentError>, String> {
        Ok(Data::load().err().unwrap_or_default())
    }

    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        no_options(options)?;
        let session =
            Headless::new(harness()).with_picture(|h, scale| picture(h, scale.unwrap_or(1)));
        Ok(Box::new(session))
    }

    /// Its keymap and every screen it can show ([`controls`]).
    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        Some(controls().map(|c| Box::new(c) as Box<dyn Controls>))
    }
}
