//! The game's tools with its grid stage, for the engine's `shuyi` (`cargo
//! shuyi <command>`): the engine's for a game on the short path
//! (`BasicTools`: `frame-png`, `lang-status`, `check` with the font atlas
//! its text is checked against, and the controls
//! `check --target` checks), with a `--seed` for the first stage, and the
//! stage's own: `run` and `scene` with its map scene and state, `replay`
//! (a [`StageRecord`], stamped with what made it: [`crate::made`]),
//! `playtest` (the engine's random bot, each try recorded:
//! [`playtest_plan`]) and `import` (the stage's `.map` besides the
//! engine's kinds). `examples/tools.rs` is the
//! tool program that runs them; the tests use them too.

use rules::{Feature, Player, Stage};
use serde::Serialize;
use shuyi::assets::font::FontAtlasDef;
use shuyi::assets::grid_map::{GridMap, MapText};
use shuyi::core::bots::playtest::{Measures, PlayedTry, Tried};
use shuyi::core::{Bot, Made, Pos, RandomBot, TerrainId, play};
use shuyi::platform::harness::Harness;
use shuyi::platform::session::{SessionRecord, Stamps};
use shuyi::prelude::*;
use shuyi::ui::input::fit::Controls;
use shuyi_cli::contract::import::Importers;
use shuyi_cli::contract::playtest::PlaytestInput;
use shuyi_cli::contract::replay::{Record, ReplayRun, turns};
use shuyi_cli::contract::session::{Headless, Scene, Session};
use shuyi_cli::frame_png::{Script, Shot};
use shuyi_cli::{BasicTools, GameArg, GameLang, GameOption, GameTools, PlaytestPlan};

use super::{Own, StageRecord, StageScreen, Terrain};
use crate::Data;

/// The game's own options, for `frame-png`, `run` and `scene`.
pub const OPTIONS: [GameOption; 1] = [GameOption::new(
    "--seed",
    "seed the first stage's luck with N (0 by default)",
)
.with_value(Some("N"))];

/// The first stage's seed `options` ask for.
pub fn seed_of(options: &[GameArg]) -> Result<u64, String> {
    let mut seed = 0;
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

/// The data of the stage's scene: the rules' whole state, and what lives
/// on its screen.
#[derive(Serialize, schemars::JsonSchema)]
pub struct StageData<'a> {
    /// The rules' state, its keys at the top (`turn`, `hero`, `outcome`).
    #[serde(flatten)]
    pub stage: &'a Stage,
    /// The seed the stage's luck started from.
    pub seed: u64,
    /// How many commands the stage has taken.
    pub commands: usize,
}

/// The stage's scene in `h`: its map and the rules' state ([`StageData`]).
pub fn scene(h: &Harness<Data>) -> Result<Scene, String> {
    let screen = h
        .screen::<StageScreen>()
        .ok_or_else(|| format!("no stage is open (the top screen is `{}`)", h.top_screen()))?;
    let map = screen.scene(StageScreen::view_tiles());
    let data = StageData {
        stage: &screen.stage,
        seed: screen.seed,
        commands: screen.commands.len(),
    };
    Scene::new("stage", "grid map", map.to_text(&Terrain)).with_data(&data)
}

/// What happened in the rules of the stage in `h`.
pub fn events(h: &Harness<Data>) -> Vec<String> {
    h.screen::<StageScreen>()
        .map(|screen| screen.events.iter().map(|e| format!("{e:?}")).collect())
        .unwrap_or_default()
}

/// The stage's bot run for `playtest` (`cargo shuyi playtest --tries 20`):
/// one entrant, `random`, on the stage of `map`, and no checks, so it
/// judges nothing and passes when every try played. Each try is
/// [`bot_try`]'s. Add checks (a `playtest.ron`) when the game knows how
/// often a bot should win.
pub fn playtest_plan(map: GridMap<TerrainId, Feature>) -> PlaytestPlan {
    PlaytestPlan::new(
        "the stage, played by a random bot",
        vec![("random".to_owned(), None)],
        Box::new(|_| Vec::new()),
        Box::new(move |_, seed, turns| bot_try(&map, seed, turns)),
    )
}

/// A try of the stage of `map` seeded `seed` by a random bot of the same
/// seed, to its end or turn `limit`: how far your piece ended from the
/// goal, and its record, stamped with this build.
fn bot_try(map: &GridMap<TerrainId, Feature>, seed: u64, limit: u32) -> Result<PlayedTry, String> {
    let mut stage = Stage::new(map, seed)?;
    let mut bot = RandomBot::new(seed);
    let mut bots: [(Player, &mut dyn Bot<Stage>); 1] = [(Player, &mut bot)];
    let report = play(&mut stage, &mut bots, limit).map_err(|e| format!("seed {seed}: {e}"))?;
    let distance = Pos::manhattan(stage.hero(), stage.goal());
    let tried = Tried {
        seed,
        outcome: report.outcome.map(|o| format!("{o:?}")),
        turns: report.turns,
        measures: Measures::from([("tiles from the goal".to_owned(), i64::from(distance))]),
    };
    let commands = u32::try_from(report.commands.len()).unwrap_or(u32::MAX);
    let record = StageRecord {
        seed,
        commands: report.commands,
        made: Some(crate::made()),
    };
    let mut one = PlayedTry::new(tried, commands);
    one.record = ron::to_string(&record).ok();
    Ok(one)
}

/// A session of `harness`, with the stage's scene and events and the game's
/// picture.
fn session_of(harness: Harness<Data>) -> Box<dyn Session> {
    let session = Headless::new(harness)
        .with_scene(scene)
        .with_events(events)
        .with_picture(|h, scale| BasicTools::<UiAction, Own>::picture(h, scale.unwrap_or(1)));
    Box::new(session)
}

/// The game's tools: the engine's, and the stage's on top.
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

    /// The game in the Harness, at its title, its first stage seeded
    /// `seed`.
    pub fn harness(&self, seed: u64) -> Result<Harness<Data>, String> {
        let mut ctx = self.game.ctx()?;
        ctx.game.own.seed = seed;
        let game = self.game.game(ctx).map_err(|e| e.to_string())?;
        Ok(Harness::new(game))
    }

    /// The stage's map, from the game's files.
    fn map(&self) -> Result<GridMap<TerrainId, Feature>, String> {
        let data = self.game.data().map_err(|errors| {
            let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
            lines.join("\n")
        })?;
        Ok(data.own.map)
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

    /// The record played from the game's real start, as `src/main.rs`
    /// runs it (`BasicGame::start`).
    fn session_from(
        &self,
        record: &SessionRecord,
        stamps: Stamps,
    ) -> Result<Box<dyn Session>, String> {
        let harness =
            Harness::from_session(self.game.start(), record, stamps).map_err(|e| e.to_string())?;
        Ok(session_of(harness))
    }

    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        let parsed: StageRecord =
            ron::from_str(&record.text).map_err(|e| format!("{}: {e}", record.file))?;
        let stage = Stage::new(&self.map()?, parsed.seed)?;
        let mut run = turns(&record.file, stage, &parsed.commands)?;
        run.made = parsed.made;
        Ok(run)
    }

    /// [`crate::made`]: `replay` refuses a record another build made.
    fn made(&self) -> Option<Made> {
        Some(crate::made())
    }

    /// The stage's bot run ([`playtest_plan`]); the game's options don't
    /// change it.
    fn playtest_plan(
        &self,
        _stage: Option<&str>,
        _input: &PlaytestInput,
    ) -> Result<PlaytestPlan, String> {
        Ok(playtest_plan(self.map()?))
    }

    fn importers(&self) -> Importers {
        let mut importers = Importers::standard();
        // The standard registry has no map format, so this is never
        // refused.
        let map = MapText::<TerrainId, Feature>::new(rules::FORMAT, rules::terrain_named);
        let _ = importers.formats.register(map);
        importers.kind::<GridMap<TerrainId, Feature>>(|map| {
            format!(
                "{} tiles by {}, {} piece(s)",
                map.tiles.width(),
                map.tiles.height(),
                map.features.len()
            )
        })
    }

    /// Its keymap and every screen it can show (`game::game`'s screens).
    fn font_atlas(&self) -> Option<Result<FontAtlasDef, String>> {
        self.basic().font_atlas()
    }

    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        self.basic().controls()
    }
}
