//! The game's tools for `shuyi`: `frame-png` (a picture of a scripted
//! frame, painted with the game's own font and tileset), `lang-status`,
//! and the tool contract's `check`, `run` and `scene` (the stage's map
//! scene and state), `replay` (a [`StageRecord`]), `bots` (a random bot
//! against the foe's mind), `import` (the stage's `.map` and the
//! keymap besides the engine's kinds), `bake` (its text baked into a
//! pack, [`crate::bake`]) and the controls `check --target` checks
//! ([`controls`]). `examples/tools.rs` is the tool
//! program that runs them; the tests use them too.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::grid_map::{GridMap, MapText};
use shuyi_assets::keymap::{KeymapDef, KeymapRon};
use shuyi_cli::contract::bots::{BotsInput, BotsOutput, Play, UNFINISHED, tally};
use shuyi_cli::contract::import::Importers;
use shuyi_cli::contract::replay::{Record, ReplayRun, turns};
use shuyi_cli::contract::session::{Headless, Scene, Session};
use shuyi_cli::frame_png::{Painter, Script, Shot};
use shuyi_cli::serde_json::json;
use shuyi_cli::{GameArg, GameBake, GameLang, GameOption, GameTools};
use shuyi_core::{Bot, Pos, RandomBot, Sides, TerrainId, TurnRules, play_observed};
use shuyi_platform::harness::Harness;
use shuyi_ui::input::fit::{Controls, GameControls, ScreenActions};
use shuyi_ui::{Game, GameData, GameHooks};

use crate::map::Terrain;
use crate::rules::{self, Chase, Command, Event, Feature, Outcome, Side, Stage};
use crate::screens::StageScreen;
use crate::{Action, FONT_PNG, Hello, MapLook};

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
        let game = loaded()?;
        Ok(GameLang {
            lang: game.lang().clone(),
            variants: Vec::new(),
        })
    }

    fn check(&self) -> Result<Vec<ContentError>, String> {
        Ok(Hello::load().err().unwrap_or_default())
    }

    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let harness = Start::from_options(options)?.harness();
        let session = Headless::new(harness).with_scene(scene).with_events(events);
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
        // Neither kind has a format in the standard registry, so neither
        // registration is refused.
        let map = MapText::<TerrainId, Feature>::new(rules::FORMAT, rules::terrain_named);
        let _ = importers.formats.register(map);
        let _ = importers.formats.register(KeymapRon::<Action>::new());
        importers
            .kind::<GridMap<TerrainId, Feature>>(|map| {
                format!(
                    "{} tiles by {}, {} piece(s)",
                    map.tiles.width(),
                    map.tiles.height(),
                    map.features.len()
                )
            })
            .kind::<KeymapDef<Action>>(|keymap| format!("{} layout(s)", keymap.layouts.len()))
    }

    /// Its keymap and every screen it can show ([`crate::screens::all`]).
    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        Some(controls().map(|c| Box::new(c) as Box<dyn Controls>))
    }

    /// One pack for every target: the game's files, its text baked.
    fn bake(&self, _target: Option<&str>) -> Result<GameBake, String> {
        Ok(GameBake {
            bake: crate::bake(),
            bundle: Bundle::new(vec![crate::layer()]),
            formats: crate::formats(),
        })
    }
}

/// The game's keymap and every screen it can show, for `check --target`.
pub fn controls() -> Result<GameControls<Action>, String> {
    let keymap = loaded()?.keymap_def().clone();
    let mut ctx = crate::ctx(0);
    let screens = crate::screens::all(&mut ctx);
    Ok(screens
        .iter()
        .fold(GameControls::new(keymap), |game, screen| {
            game.with(ScreenActions::of(screen.as_ref()))
        }))
}

/// The game's data, or its errors in a line each.
fn loaded() -> Result<Hello, String> {
    Hello::load().map_err(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        errors.join("\n")
    })
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

/// The stage's scene in `h`: its map as the skin of the player's
/// settings fits it, and the rules' state.
pub fn scene(h: &Harness<Hello>) -> Result<Scene, String> {
    let screen = h
        .screen::<StageScreen>()
        .ok_or_else(|| format!("no stage is open (the top screen is `{}`)", h.top_screen()))?;
    let map = screen.scene(StageScreen::view_tiles(h.ctx()));
    let state = &screen.stage;
    let at = |p: Pos| json!({ "x": p.x, "y": p.y });
    Ok(Scene {
        screen: "stage".to_owned(),
        kind: "grid map".to_owned(),
        text: map.to_text(&Terrain),
        data: json!({
            "turn": state.turn(),
            "to_move": format!("{:?}", state.side_to_move()),
            "hero": at(state.hero()),
            "foe": at(state.foe()),
            "goal": at(state.goal()),
            "outcome": state.outcome().map(|o| format!("{o:?}")),
            "commands": screen.commands.len(),
        }),
    })
}

/// What happened in the rules of the stage in `h`, if one is open.
pub fn events(h: &Harness<Hello>) -> Vec<String> {
    h.screen::<StageScreen>()
        .map(|stage| stage.events.iter().map(|e| format!("{e:?}")).collect())
        .unwrap_or_default()
}

/// A play of the stage of `map` seeded `seed`, to its end or turn
/// `limit`: a random bot of the same seed moves your piece, the foe's
/// mind the foe. Its measure is how often the foe tripped.
fn bot_play(map: &GridMap<TerrainId, Feature>, seed: u64, limit: u32) -> Result<Play, String> {
    let mut stage = Stage::new(map, seed)?;
    let mut hero = RandomBot::new(seed);
    let mut foe = Chase;
    let mut bots: [(Side, &mut dyn Bot<Stage>); 2] =
        [(Side::Hero, &mut hero), (Side::Foe, &mut foe)];
    let mut trips: i64 = 0;
    let report = play_observed(&mut stage, &mut bots, limit, |_, _, events| {
        trips += i64::from(events.contains(&Event::Tripped));
    })
    .map_err(|e| format!("seed {seed}: {e}"))?;
    let outcome = match report.outcome {
        Some(Outcome::Won { .. }) => "won",
        Some(Outcome::Caught { .. }) => "caught",
        None => UNFINISHED,
    };
    let record = StageRecord {
        seed,
        commands: report.commands.clone(),
    };
    Ok(Play {
        seed,
        outcome: outcome.to_owned(),
        turns: report.turns,
        commands: u32::try_from(report.commands.len()).unwrap_or(u32::MAX),
        measures: BTreeMap::from([("foe trips".to_owned(), trips)]),
        record: ron::to_string(&record).ok(),
    })
}
