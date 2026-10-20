//! The game's tools for the engine's `shuyi` (`cargo shuyi <command>`):
//! `frame-png` (a picture of a frame, painted with the game's own font),
//! `lang-status`, and the tool contract's `check`, `run` and `scene` (the
//! stage's map scene and state), `replay` (a [`StageRecord`]), `bots`
//! (the engine's random bot) and `import` (the stage's `.map` besides the
//! engine's kinds). `examples/tools.rs` is the tool program that runs
//! them; the tests use them too.

use std::collections::BTreeMap;

use rules::{Command, Feature, Outcome, Player, Stage};
use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::grid_map::{GridMap, MapText};
use shuyi_cli::contract::bots::{BotsInput, BotsOutput, Play, UNFINISHED, tally};
use shuyi_cli::contract::import::Importers;
use shuyi_cli::contract::replay::{Record, ReplayRun, turns};
use shuyi_cli::contract::session::{Headless, Scene, Session};
use shuyi_cli::frame_png::{Image, Painter, Script, Shot};
use shuyi_cli::serde_json::json;
use shuyi_cli::{GameArg, GameLang, GameOption, GameTools};
use shuyi_core::{Bot, Pos, RandomBot, Sides, TerrainId, TurnRules, play};
use shuyi_ui::GameData;
use shuyi_ui::harness::Harness;

use crate::stage::{StageScreen, Terrain};
use crate::{Data, FONT_PNG};

/// The game's own options, for `frame-png`, `run` and `scene`.
pub const OPTIONS: [GameOption; 1] = [GameOption {
    flag: "--seed",
    value: Some("N"),
    help: "seed the first stage's luck with N (0 by default)",
}];

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

/// The game in the Harness, its first stage seeded `seed`.
pub fn harness(seed: u64) -> Harness<Data> {
    Harness::new(crate::game(crate::ctx(seed)))
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
    Ok(Scene {
        screen: "stage".to_owned(),
        kind: "grid map".to_owned(),
        text: map.to_text(&Terrain),
        data: json!({
            "seed": screen.seed,
            "turn": stage.turn(),
            "hero": at(stage.hero()),
            "wanderer": at(stage.wanderer()),
            "goal": at(stage.goal()),
            "outcome": stage.outcome().map(|o| format!("{o:?}")),
            "commands": screen.commands.len(),
        }),
    })
}

/// What happened in the rules of the stage in `h`.
pub fn events(h: &Harness<Data>) -> Vec<String> {
    h.screen::<StageScreen>()
        .map(|screen| screen.events.iter().map(|e| format!("{e:?}")).collect())
        .unwrap_or_default()
}

/// The game's data, or its errors in a line each.
fn loaded() -> Result<Data, String> {
    Data::load().map_err(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        errors.join("\n")
    })
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
    Ok(Play {
        seed,
        outcome: outcome.to_owned(),
        turns: report.turns,
        commands: u32::try_from(report.commands.len()).unwrap_or(u32::MAX),
        measures: BTreeMap::from([("tiles from the goal".to_owned(), i64::from(distance))]),
        record: ron::to_string(&record).ok(),
    })
}

/// The game's tools.
#[derive(Debug, Clone, Copy, Default)]
pub struct Tools;

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
        let mut h = harness(seed_of(options)?);
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
        let session = Headless::new(harness(seed_of(options)?))
            .with_scene(scene)
            .with_events(events);
        Ok(Box::new(session))
    }

    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        let parsed: StageRecord =
            ron::from_str(&record.text).map_err(|e| format!("{}: {e}", record.file))?;
        let stage = Stage::new(&loaded()?.stage_map, parsed.seed)?;
        turns(&record.file, stage, &parsed.commands)
    }

    fn bots(&self, input: &BotsInput) -> Result<BotsOutput, String> {
        let map = loaded()?.stage_map;
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
}
