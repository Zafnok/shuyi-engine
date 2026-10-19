//! The game's tools for `shuyi`: `frame-png` (a picture of a scripted
//! frame, painted with the game's own font and tileset) and `lang-status`.
//! `examples/tools.rs` is the tool program that runs them; the tests use
//! them too.

use shuyi_cli::frame_png::{Painter, Script, Shot};
use shuyi_cli::{GameArg, GameLang, GameOption, GameTools};
use shuyi_ui::harness::Harness;
use shuyi_ui::{Game, GameData, GameHooks};

use crate::screens::StageScreen;
use crate::{FONT_PNG, Hello, MapLook};

/// The game's own `frame-png` options.
pub const OPTIONS: [GameOption; 3] = [
    GameOption {
        flag: "--stage",
        value: None,
        help: "start on the stage, its scene seen, not at the title",
    },
    GameOption {
        flag: "--tiles",
        value: None,
        help: "draw the map with the tileset (the Tiles look), not glyphs",
    },
    GameOption {
        flag: "--seed",
        value: Some("N"),
        help: "seed the stage's luck with N (0 by default)",
    },
];

/// How a game in the Harness starts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Start {
    /// On the stage, its scene seen; otherwise at the title.
    pub stage: bool,
    /// The map's look.
    pub look: MapLook,
    /// The first stage's seed.
    pub seed: u64,
}

impl Start {
    /// The start `options` ask for.
    pub fn from_options(options: &[GameArg]) -> Result<Self, String> {
        let mut start = Self::default();
        for option in options {
            match (option.flag.as_str(), option.value.as_deref()) {
                ("--stage", _) => start.stage = true,
                ("--tiles", _) => start.look = MapLook::Tiles,
                ("--seed", Some(n)) => {
                    start.seed = n.parse().map_err(|_| format!("--seed {n}: not a number"))?;
                }
                (flag, _) => return Err(format!("unknown option {flag}")),
            }
        }
        Ok(start)
    }

    /// The game in the Harness, started so.
    pub fn harness(self) -> Harness<Hello> {
        let mut ctx = crate::ctx(self.seed);
        ctx.preview_settings(|s| s.game.look = self.look);
        let game = if self.stage {
            let stage = StageScreen::new(&mut ctx).briefed();
            Game::new(ctx, Box::new(stage), GameHooks::default())
        } else {
            crate::game(ctx)
        };
        Harness::new(game)
    }
}

/// The frame `h` shows, each console pixel `scale` × `scale` pixels.
pub fn picture(h: &Harness<Hello>, scale: u32) -> Result<shuyi_cli::frame_png::Image, String> {
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

/// The game's tools.
#[derive(Debug, Clone, Copy, Default)]
pub struct HelloTools;

impl GameTools for HelloTools {
    fn frame_options(&self) -> &[GameOption] {
        &OPTIONS
    }

    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        let mut h = Start::from_options(options)?.harness();
        script.play(&mut h);
        Ok(Shot {
            image: picture(&h, scale.unwrap_or(1))?,
            shows: h.top_screen().to_owned(),
            warning: None,
        })
    }

    fn lang(&self) -> Result<GameLang, String> {
        let game = Hello::load().map_err(|errors| {
            let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
            errors.join("\n")
        })?;
        Ok(GameLang {
            lang: game.lang().clone(),
            variants: Vec::new(),
        })
    }
}
