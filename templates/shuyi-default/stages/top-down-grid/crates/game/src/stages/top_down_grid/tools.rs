//! The game's tools with its grid stage, for the engine's `shuyi` (`cargo
//! shuyi <command>`): the engine's for a game on the short path
//! (`BasicTools`: `frame-png`, `lang-status`, `check` and the controls
//! `check --target` checks), with a `--seed` for the first stage, and the
//! stage's own: `run` and `scene` with its map scene and state, `replay`
//! (a [`StageRecord`]), `bots` (the engine's random bot) and `import` (the
//! stage's `.map` besides the engine's kinds). `examples/tools.rs` is the
//! tool program that runs them; the tests use them too.

use std::collections::BTreeMap;

use rules::{Command, Feature, Outcome, Player, Stage};
use serde::{Deserialize, Serialize};
use shuyi::assets::grid_map::{GridMap, MapText};
use shuyi::core::{Bot, Pos, RandomBot, Sides, TerrainId, TurnRules, play};
use shuyi::platform::harness::Harness;
use shuyi::prelude::*;
use shuyi::ui::input::fit::Controls;
use shuyi_cli::contract::bots::{BotsInput, BotsOutput, Play, UNFINISHED, tally};
use shuyi_cli::contract::import::Importers;
use shuyi_cli::contract::replay::{Record, ReplayRun, turns};
use shuyi_cli::contract::session::{Headless, Scene, Session};
use shuyi_cli::frame_png::{Script, Shot};
use shuyi_cli::serde_json::json;
use shuyi_cli::{BasicTools, GameArg, GameLang, GameOption, GameTools};

use super::{Own, StageScreen, Terrain};
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

/// A record of a stage: its seed and the commands sent to its rules. The
/// game's record format for `replay`; `bots` writes one of each play.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRecord {
    /// The stage's seed.
    pub seed: u64,
    /// The commands, in order.
    pub commands: Vec<Command>,
}

/// The stage's scene in `h`: its map and the rules' state.
pub fn scene(h: &Harness<Data>) -> Result<Scene, String> {
    let screen = h
        .screen::<StageScreen>()
        .ok_or_else(|| format!("no stage is open (the top screen is `{}`)", h.top_screen()))?;
    let map = screen.scene(StageScreen::view_tiles());
    let stage = &screen.stage;
    let at = |p: Pos| json!({ "x": p.x, "y": p.y });
    let mut scene = Scene::new("stage", "grid map", map.to_text(&Terrain));
    scene.data = json!({
        "seed": screen.seed,
        "turn": stage.turn(),
        "hero": at(stage.hero()),
        "wanderer": at(stage.wanderer()),
        "goal": at(stage.goal()),
        "outcome": stage.outcome().map(|o| format!("{o:?}")),
        "commands": screen.commands.len(),
    });
    Ok(scene)
}

/// What happened in the rules of the stage in `h`.
pub fn events(h: &Harness<Data>) -> Vec<String> {
    h.screen::<StageScreen>()
        .map(|screen| screen.events.iter().map(|e| format!("{e:?}")).collect())
        .unwrap_or_default()
}

/// A play of the stage of `map` seeded `seed` by a random bot of the same
/// seed, to its end or turn `limit`.
fn bot_play(map: &GridMap<TerrainId, Feature>, seed: u64, limit: u32) -> Result<Play, String> {
    let mut stage = Stage::new(map, seed)?;
    let mut bot = RandomBot::new(seed);
    let mut bots: [(Player, &mut dyn Bot<Stage>); 1] = [(Player, &mut bot)];
    let report = play(&mut stage, &mut bots, limit).map_err(|e| format!("seed {seed}: {e}"))?;
    let outcome = match report.outcome {
        Some(Outcome::Won { .. }) => "won",
        Some(Outcome::Caught { .. }) => "caught",
        None => UNFINISHED,
    };
    let record = StageRecord {
        seed,
        commands: report.commands.clone(),
    };
    let distance = Pos::manhattan(stage.hero(), stage.goal());
    let commands = u32::try_from(report.commands.len()).unwrap_or(u32::MAX);
    let mut play = Play::new(seed, outcome, report.turns, commands);
    play.measures = BTreeMap::from([("tiles from the goal".to_owned(), i64::from(distance))]);
    play.record = ron::to_string(&record).ok();
    Ok(play)
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
        let session = Headless::new(self.harness(seed_of(options)?)?)
            .with_scene(scene)
            .with_events(events)
            .with_picture(|h, scale| BasicTools::<UiAction, Own>::picture(h, scale.unwrap_or(1)));
        Ok(Box::new(session))
    }

    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        let parsed: StageRecord =
            ron::from_str(&record.text).map_err(|e| format!("{}: {e}", record.file))?;
        let stage = Stage::new(&self.map()?, parsed.seed)?;
        turns(&record.file, stage, &parsed.commands)
    }

    fn bots(&self, input: &BotsInput) -> Result<BotsOutput, String> {
        let map = self.map()?;
        let plays = (input.seed..input.seed + u64::from(input.games))
            .map(|seed| bot_play(&map, seed, input.turns))
            .collect::<Result<_, _>>()?;
        Ok(tally(plays))
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
    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        self.basic().controls()
    }
}
