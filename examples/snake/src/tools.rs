//! The game's tools, for the engine's `shuyi` (`cargo shuyi <command>`):
//! the engine's for a game on the short path (`BasicTools`: `frame-png`,
//! `lang-status`, `check` with the font atlas, the controls `check
//! --target` checks, and `bake`), with a `--seed` for the first run, and
//! the game's own: `run` and `scene` with the board's scene and the run's
//! state, `replay` (a [`TickRecord`] of the run's steps, through the
//! contract's `replay::ticks`), and its look on a one-bit screen.
//! `examples/tools.rs` is the tool program that runs them; the tests use
//! them too. A tick game has no bots yet, so no `playtest`.

use serde::Serialize;
use shuyi::assets::font::FontAtlasDef;
use shuyi::assets::onebit::PatternMapDef;
use shuyi::assets::palette::PaletteDef;
use shuyi::core::tick::TickRecord;
use shuyi::core::{Made, Pos};
use shuyi::platform::harness::Harness;
use shuyi::platform::session::{SessionRecord, Stamps};
use shuyi::prelude::*;
use shuyi::ui::input::fit::Controls;
use shuyi_cli::contract::replay::{Record, ReplayRun, ticks};
use shuyi_cli::contract::session::{Headless, Scene, Session};
use shuyi_cli::frame_png::{Script, Shot};
use shuyi_cli::{BasicTools, GameArg, GameBake, GameLang, GameOption, GameTools, OneBitSkin};

use crate::rules::{Outcome, Snake};
use crate::screens::{self, Stage, Terrain};
use crate::{Data, Own};

/// The look of the game on a one-bit screen, beside `palette.ron`.
// check-text: not player text, a file name
pub const PATTERN_MAP: &str = "palette@onebit.ron";

/// The game's own options, for `frame-png`, `run` and `scene`.
pub const OPTIONS: [GameOption; 1] =
    [
        GameOption::new("--seed", "seed the first run's luck with N (1 by default)")
            .with_value(Some("N")),
    ];

/// The first run's seed `options` ask for.
pub fn seed_of(options: &[GameArg]) -> Result<u64, String> {
    let mut seed = 1;
    for option in options {
        match (option.flag.as_str(), option.value.as_deref()) {
            ("--seed", Some(n)) => {
                seed = n.parse().map_err(|_| format!("--seed {n}: not a number"))?;
            }
            (flag, _) => return Err(format!("unknown option {flag}")),
        }
    }
    Ok(seed)
}

/// The data of the board's scene: what an agent asks of a run.
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct StageData {
    /// The snake's tiles.
    pub length: u32,
    /// The food eaten.
    pub score: u32,
    /// Whether the run has ended.
    pub over: bool,
    /// How it ended, once it has.
    pub outcome: Option<Outcome>,
    /// The steps the rules have run.
    pub step: u64,
    /// The snake's tiles, its head first.
    pub body: Vec<Pos>,
    /// The food, if the board has room for it.
    pub food: Option<Pos>,
    /// The steps between two moves now.
    pub period: u32,
    /// The seed the run's luck started from.
    pub seed: u64,
}

impl StageData {
    /// The data of `stage`'s run.
    pub fn of(stage: &Stage) -> Self {
        let snake = &stage.snake;
        Self {
            length: snake.length(),
            score: snake.score(),
            over: snake.outcome().is_some(),
            outcome: snake.outcome(),
            step: snake.steps(),
            body: snake.body().to_vec(),
            food: snake.food(),
            period: snake.period(),
            seed: stage.seed,
        }
    }
}

/// The board's scene in `h`: its map and the run's state ([`StageData`]).
pub fn scene(h: &Harness<Data>) -> Result<Scene, String> {
    let stage = h
        .screen::<Stage>()
        .ok_or_else(|| format!("no run is open (the top screen is `{}`)", h.top_screen()))?;
    let map = screens::scene(&stage.snake);
    Scene::new("stage", "grid map", map.to_text(&Terrain)).with_data(&StageData::of(stage))
}

/// What happened in the rules of the run in `h`.
pub fn events(h: &Harness<Data>) -> Vec<String> {
    h.screen::<Stage>()
        .map_or_default(|stage| stage.events.iter().map(|e| format!("{e:?}")).collect())
}

/// A session of `harness`, with the board's scene and events and the
/// game's picture.
fn session_of(harness: Harness<Data>) -> Box<dyn Session> {
    let session = Headless::new(harness)
        .with_scene(scene)
        .with_events(events)
        .with_picture(|h, scale| BasicTools::<UiAction, Own>::picture(h, scale.unwrap_or(1)));
    Box::new(session)
}

/// `errors`, one a line, each as `check` prints it.
pub fn lines(errors: &[ContentError]) -> String {
    let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
    lines.join("\n")
}

/// The game's tools: the engine's, and the game's own on top.
#[derive(Debug, Clone, Copy)]
pub struct Tools {
    game: BasicGame<UiAction, Own>,
}

impl Tools {
    /// The tools of `game`.
    pub fn new(game: BasicGame<UiAction, Own>) -> Self {
        Self { game }
    }

    /// The engine's tools for the game.
    fn basic(&self) -> BasicTools<UiAction, Own> {
        BasicTools::new(self.game)
    }

    /// The game in the Harness, at its title, its first run seeded `seed`,
    /// pausing without focus and opening its pause menu as on a device.
    pub fn harness(&self, seed: u64) -> Result<Harness<Data>, String> {
        let mut ctx = self.game.ctx()?;
        ctx.game.own.seed = seed;
        let game = self.game.game(ctx).map_err(|e| e.to_string())?;
        Ok(self.game.harness_on(game))
    }

    /// The skin's palette and its pattern map: its look on a one-bit
    /// target.
    pub fn one_bit_skin(&self) -> Result<OneBitSkin, String> {
        let bundle = self.game.bundle();
        let formats = Data::formats();
        let palette =
            (formats.load::<PaletteDef>(&bundle, "palette.ron")).map_err(|e| lines(&e))?;
        let map = (formats.load::<PatternMapDef>(&bundle, PATTERN_MAP)).map_err(|e| lines(&e))?;
        Ok(OneBitSkin::new(
            bundle.display_path("palette.ron"),
            palette,
            bundle.display_path(PATTERN_MAP),
            map,
        ))
    }
}

impl GameTools for Tools {
    fn frame_options(&self) -> &[GameOption] {
        &OPTIONS
    }

    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        let mut h = self.harness(seed_of(options)?)?;
        script.play(&mut h);
        let image = BasicTools::<UiAction, Own>::picture(&h, scale.unwrap_or(1))?;
        Ok(Shot::new(image, h.top_screen()))
    }

    fn lang(&self) -> Result<GameLang, String> {
        self.basic().lang()
    }

    fn check(&self) -> Result<Vec<ContentError>, String> {
        self.basic().check()
    }

    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        Ok(session_of(self.harness(seed_of(options)?)?))
    }

    /// The record played from the game's real start, as `src/main.rs` runs
    /// it (`BasicGame::start`).
    fn session_from(
        &self,
        record: &SessionRecord,
        stamps: Stamps,
    ) -> Result<Box<dyn Session>, String> {
        let harness =
            Harness::from_session(self.game.start(), record, stamps).map_err(|e| e.to_string())?;
        Ok(session_of(harness))
    }

    /// A [`TickRecord`] of a run in RON, its first state built from its
    /// seed, replayed step by step.
    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        let parsed: TickRecord =
            ron::from_str(&record.text).map_err(|e| format!("{}: {e}", record.file))?;
        let own = self.game.data().map_err(|e| lines(&e))?.own;
        let snake = Snake::new(&own.board, own.tuning, parsed.seed())?;
        let outcome = |snake: &Snake| snake.outcome().map(|o| format!("{o:?}"));
        Ok(ticks::<UiAction, _>(&record.file, snake, &parsed, outcome))
    }

    /// [`crate::made`]: `replay` refuses a record another build made.
    fn made(&self) -> Option<Made> {
        Some(crate::made())
    }

    fn font_atlas(&self) -> Option<Result<FontAtlasDef, String>> {
        self.basic().font_atlas()
    }

    /// Its keymap and every screen it can show.
    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        self.basic().controls()
    }

    /// Its palette and pattern map, the same on every one-bit target.
    fn one_bit(&self, _target: &str) -> Option<Result<OneBitSkin, String>> {
        Some(self.one_bit_skin())
    }

    fn bake(&self, target: Option<&str>) -> Result<GameBake, String> {
        self.basic().bake(target)
    }
}
