//! The game's tools for `shuyi`: `frame-png` (a picture of a scripted
//! frame, painted with the game's own font and tileset), `lang-status`,
//! and the tool contract's `check`, `run` and `scene` (the stage's map
//! scene and state), `replay` (a [`StageRecord`]), `bots` (a random bot
//! against the foe's mind), `playtest` (a random hero and a walker,
//! [`ENTRANTS`], judged by `playtest.ron`), `autobalance` (the knobs it
//! turns, [`Trip`] and [`HeadStart`], on the stage `balance.ron` tunes,
//! [`autobalance_plan`]), `import` (the stage's `.map` and the
//! keymap besides the engine's kinds), `bake` (its text baked into a
//! pack, [`crate::bake`]), the controls `check --target` checks
//! ([`controls`]), and its look on a one-bit target: the pattern map
//! `check --target` checks ([`one_bit_skin`]) and `frame-png --target
//! playdate` paints with ([`one_bit_picture`]). `examples/tools.rs` is the
//! tool program that runs them; the tests use them too.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use shuyi_assets::ContentError;
use shuyi_assets::bundle::Bundle;
use shuyi_assets::grid_map::{GridMap, MapText};
use shuyi_assets::keymap::{KeymapDef, KeymapRon};
use shuyi_assets::onebit::PatternMapDef;
use shuyi_assets::palette::PaletteDef;
use shuyi_cli::contract::autobalance::AutobalanceInput;
use shuyi_cli::contract::bots::{BotsInput, BotsOutput, Play, UNFINISHED, tally};
use shuyi_cli::contract::import::Importers;
use shuyi_cli::contract::playtest::PlaytestInput;
use shuyi_cli::contract::replay::{Record, ReplayRun, turns};
use shuyi_cli::contract::session::{Headless, Scene, Session};
use shuyi_cli::frame_png::{self, Painter, Script, Shot};
use shuyi_cli::serde_json::json;
use shuyi_cli::{
    AutobalancePlan, GameArg, GameBake, GameLang, GameOption, GameTools, OneBitSkin, PlaytestPlan,
    Tunable,
};
use shuyi_core::bots::playtest::{
    self, Check, Entrant, Measure, Measures, PlaytestError, PlaytestReport, Tries, play_tries,
};
use shuyi_core::{
    Bot, Determinize, Knob, Pos, RandomBot, Report, Sides, TerrainId, TurnRules, play_observed,
};
use shuyi_platform::harness::Harness;
use shuyi_ui::frame::onebit::{Font, PatternMap, to_one_bit};
use shuyi_ui::input::fit::{Controls, GameControls, ScreenActions};
use shuyi_ui::{Game, GameData, GameHooks};

use crate::map::Terrain;
use crate::rules::{self, Chase, Command, Event, Feature, Outcome, Side, Stage, Tuning, Walk};
use crate::screens::StageScreen;
use crate::{Action, FONT_PNG, Hello, MapLook};

/// The game's own `frame-png` options.
pub const OPTIONS: [GameOption; 3] = [
    GameOption::new(
        "--stage",
        "start on the stage, its scene seen, not at the title",
    ),
    GameOption::new(
        "--tiles",
        "draw the map with the tileset (the Tiles look), not glyphs",
    ),
    GameOption::new("--seed", "seed the stage's luck with N (0 by default)").with_value(Some("N")),
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

/// The skin's pattern map, beside its palette in `files/` (ADR-0008 §4).
pub const PATTERN_MAP: &str = "palette@onebit.ron";

/// The font atlas's one-bit rows (`cargo shuyi font-atlas` writes them
/// beside it), which a one-bit screen draws glyphs with.
pub const FONT_1BIT: &str = "font/atlas_1bit.bin";

/// The skin's palette and its pattern map: its look on a one-bit target.
pub fn one_bit_skin() -> Result<OneBitSkin, String> {
    let bundle = Bundle::new(vec![crate::layer()]);
    let formats = crate::formats();
    let palette = formats
        .load::<PaletteDef>(&bundle, "palette.ron")
        .map_err(|e| lines(&e))?;
    let map = formats
        .load::<PatternMapDef>(&bundle, PATTERN_MAP)
        .map_err(|e| lines(&e))?;
    Ok(OneBitSkin::new(
        bundle.display_path("palette.ron"),
        palette,
        bundle.display_path(PATTERN_MAP),
        map,
    ))
}

/// The frame `h` shows as a one-bit target's display shows it: in the
/// skin's patterns, its glyphs from the font's one-bit rows (a sprite,
/// whose picture isn't made one-bit yet, is a grey rectangle), each pixel
/// `scale` × `scale` pixels.
pub fn one_bit_picture(
    h: &Harness<Hello>,
    scale: u32,
) -> Result<shuyi_cli::frame_png::Image, String> {
    let ctx = h.ctx();
    let map = PatternMap::new(ctx.palette.iter(), &one_bit_skin()?.map);
    let bits = crate::file(FONT_1BIT).ok_or("no one-bit font atlas")?;
    let font = Font::new(&ctx.game.font, bits)?;
    let frame = to_one_bit(h.game().buffer(), &map, &font, &());
    Ok(frame_png::one_bit_picture(frame.bytes(), scale))
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
        Ok(Shot::new(picture(&h, scale.unwrap_or(1))?, h.top_screen()))
    }

    /// The frame as [`one_bit_picture`] paints it: the same on every
    /// one-bit target, the Playdate's 400 × 240 screen.
    fn frame_one_bit(
        &self,
        _target: &str,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        let mut h = Start::from_options(options)?.harness();
        script.play(&mut h);
        Ok(Shot::new(
            one_bit_picture(&h, scale.unwrap_or(1))?,
            h.top_screen(),
        ))
    }

    fn lang(&self) -> Result<GameLang, String> {
        let game = loaded()?;
        Ok(GameLang::new(game.lang().clone(), Vec::new()))
    }

    fn check(&self) -> Result<Vec<ContentError>, String> {
        Ok(Hello::load().err().unwrap_or_default())
    }

    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let harness = Start::from_options(options)?.harness();
        let session = Headless::new(harness)
            .with_scene(scene)
            .with_events(events)
            .with_picture(|h, scale| picture(h, scale.unwrap_or(1)));
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

    /// [`ENTRANTS`] on the stage, judged by `playtest.ron`
    /// ([`playtest_plan`]).
    fn playtest_plan(
        &self,
        _stage: Option<&str>,
        input: &PlaytestInput,
    ) -> Result<PlaytestPlan, String> {
        if let Some(option) = input.start.first() {
            return Err(format!(
                "playtest takes no start option, not {}",
                option.flag
            ));
        }
        playtest_plan()
    }

    /// The stage tuned as `balance.ron` says, searched as
    /// [`autobalance_plan`] says; `--apply` writes `balance.ron` beside
    /// `shuyi.ron`.
    fn autobalance_plan(
        &self,
        _stage: Option<&str>,
        input: &AutobalanceInput,
    ) -> Result<AutobalancePlan, String> {
        if let Some(option) = input.start.first() {
            return Err(format!(
                "autobalance takes no start option, not {}",
                option.flag
            ));
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(BALANCE_FILE);
        autobalance_plan(path)
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

    /// Its palette and pattern map, the same on every one-bit target.
    fn one_bit(&self, _target: &str) -> Option<Result<OneBitSkin, String>> {
        Some(one_bit_skin())
    }

    /// One pack for every target: the game's files, its text baked.
    fn bake(&self, _target: Option<&str>) -> Result<GameBake, String> {
        Ok(GameBake::new(
            crate::bake(),
            Bundle::new(vec![crate::layer()]),
            crate::formats(),
        ))
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
    Hello::load().map_err(|e| lines(&e))
}

/// `errors`, a line each.
fn lines(errors: &[ContentError]) -> String {
    let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
    errors.join("\n")
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
    let mut scene = Scene::new("stage", "grid map", map.to_text(&Terrain));
    scene.data = json!({
        "turn": state.turn(),
        "to_move": format!("{:?}", state.side_to_move()),
        "hero": at(state.hero()),
        "foe": at(state.foe()),
        "goal": at(state.goal()),
        "outcome": state.outcome().map(|o| format!("{o:?}")),
        "commands": screen.commands.len(),
    });
    Ok(scene)
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
    let commands = u32::try_from(report.commands.len()).unwrap_or(u32::MAX);
    let mut play = Play::new(seed, outcome, report.turns, commands);
    play.measures = BTreeMap::from([("foe trips".to_owned(), trips)]);
    play.record = ron::to_string(&record).ok();
    Ok(play)
}

/// How a playtest measures a try of the stage: `won` (1 if your piece
/// reached the goal, else 0), `turns` (the turn it stopped on) and `foe
/// trips` (how often the foe tripped).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StageMeasure {
    trips: i64,
}

impl Measure<Stage> for StageMeasure {
    fn observe(&mut self, _: &Stage, _: &Command, events: &[Event]) {
        self.trips += i64::from(events.contains(&Event::Tripped));
    }

    fn finish(self: Box<Self>, report: &Report<Stage>, _: &Stage) -> Measures {
        let won = matches!(report.outcome, Some(Outcome::Won { .. }));
        Measures::from([
            ("won".to_owned(), i64::from(won)),
            ("turns".to_owned(), i64::from(report.turns)),
            ("foe trips".to_owned(), self.trips),
        ])
    }
}

/// A playtest's hero: a named bot for your piece against the foe's mind
/// ([`Chase`]).
#[derive(Debug, Clone, Copy)]
pub struct Hero {
    /// Its name in `playtest.ron`.
    pub name: &'static str,
    /// Its bot for the try of a seed.
    pub bot: fn(u64) -> Box<dyn Bot<Stage>>,
}

impl Entrant<Stage, Stage> for Hero {
    fn name(&self) -> &str {
        self.name
    }

    /// The stage with its luck seeded `seed` (a copy whose luck is
    /// redrawn is the stage of that seed).
    fn start(&self, stage: &Stage, seed: u64) -> Stage {
        stage.determinize(Side::Hero, seed)
    }

    fn bots(&self, seed: u64) -> Vec<(Side, Box<dyn Bot<Stage>>)> {
        vec![(Side::Hero, (self.bot)(seed)), (Side::Foe, Box::new(Chase))]
    }

    fn measure(&self) -> Box<dyn Measure<Stage>> {
        Box::new(StageMeasure::default())
    }
}

/// The playtest's heroes: `random` (a random bot of the try's seed) and
/// `walk` ([`Walk`], to the tile in reach nearest the goal).
pub const ENTRANTS: [Hero; 2] = [
    Hero {
        name: "random",
        bot: |seed| Box::new(RandomBot::new(seed)),
    },
    Hero {
        name: "walk",
        bot: |_| Box::new(Walk),
    },
];

/// The playtest's checks, `playtest.ron` beside `shuyi.ron`: the tools'
/// data, not the game's, so not in `files/` and its pack.
pub fn playtest_checks() -> Result<Vec<Check>, String> {
    let text = include_str!("../playtest.ron");
    ron::from_str(text).map_err(|e| format!("playtest.ron: {e}"))
}

/// The playtest's plan: [`ENTRANTS`] on the one stage, each try played
/// by [`play_tries`] from its seed alone, and those of
/// [`playtest_checks`] whose entrants all play.
pub fn playtest_plan() -> Result<PlaytestPlan, String> {
    let stage = Stage::new(&loaded()?.stage_map, 0)?;
    let checks = playtest_checks()?;
    Ok(PlaytestPlan::new(
        "the stage (files/stage.map)",
        ENTRANTS.iter().map(|e| (e.name.to_owned(), None)).collect(),
        Box::new(move |names| {
            checks
                .iter()
                .filter(|c| c.entrants().iter().all(|e| names.contains(e)))
                .cloned()
                .collect()
        }),
        Box::new(move |entrant, seed, turns| {
            let mut tries = play_tries(&stage, &ENTRANTS[entrant], seed..seed + 1, turns)
                .map_err(|e| e.to_string())?;
            tries.pop().ok_or_else(|| "no try".to_owned())
        }),
    ))
}

/// [`ENTRANTS`] play the stage as `tries` says, judged by
/// [`playtest_checks`], on this thread.
pub fn playtest_run(tries: &Tries) -> Result<PlaytestReport, String> {
    let stage = Stage::new(&loaded()?.stage_map, 0)?;
    let entrants: Vec<&dyn Entrant<Stage, Stage>> = ENTRANTS
        .iter()
        .map(|e| e as &dyn Entrant<Stage, Stage>)
        .collect();
    playtest::run(&stage, &entrants, tries, &playtest_checks()?).map_err(|e| playtest_error(&e))
}

/// What a playtest's error says: a check naming an entrant who doesn't
/// play is `playtest.ron`'s.
pub fn playtest_error(e: &PlaytestError<Stage>) -> String {
    match e {
        PlaytestError::UnknownEntrant(e) => format!("playtest.ron: {e}"),
        _ => e.to_string(),
    }
}

/// The highest trip chance [`Trip`] goes to, in percent.
pub const MOST_TRIPS: u8 = 75;

/// The longest wait [`HeadStart`] gives the foe, in turns.
pub const MOST_WAITS: u8 = 3;

/// A knob of the foe's trip chance, from a stage tuned with `from`: five
/// points a step, from 0% to [`MOST_TRIPS`], two steps at first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Trip {
    /// The trip chance of the stage as authored, in percent.
    pub from: u8,
}

impl Trip {
    /// The chance `step` gives, if it is from 0% to [`MOST_TRIPS`].
    fn percent(self, step: i32) -> Option<u8> {
        let percent = step.checked_mul(5)?.checked_add(i32::from(self.from))?;
        u8::try_from(percent).ok().filter(|&p| p <= MOST_TRIPS)
    }
}

impl Knob<Stage> for Trip {
    fn name(&self) -> &'static str {
        "trip"
    }

    fn range(&self) -> RangeInclusive<i32> {
        let from = i32::from(self.from);
        -(from / 5)..=((i32::from(MOST_TRIPS) - from) / 5).max(0)
    }

    fn stride(&self) -> u32 {
        2
    }

    fn apply(&self, stage: &Stage, step: i32) -> Option<Stage> {
        let trip_percent = self.percent(step)?;
        Some(stage.retuned(Tuning {
            trip_percent,
            ..stage.tuning()
        }))
    }

    fn describe(&self, step: i32) -> String {
        let percent = self
            .percent(step)
            .map_or_else(|| "?".to_owned(), |p| p.to_string());
        format!("the foe trips {percent}% of the time, not {}%", self.from)
    }
}

/// A knob of your head start, from a stage tuned with `from`: the foe
/// skips one more of its first turns a step, up to [`MOST_WAITS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeadStart {
    /// The foe's wait in the stage as authored, in turns.
    pub from: u8,
}

impl HeadStart {
    /// The wait `step` gives, if it is from 0 to [`MOST_WAITS`] turns.
    fn waits(self, step: i32) -> Option<u8> {
        let waits = step.checked_add(i32::from(self.from))?;
        u8::try_from(waits).ok().filter(|&w| w <= MOST_WAITS)
    }
}

impl Knob<Stage> for HeadStart {
    fn name(&self) -> &'static str {
        "head start"
    }

    fn range(&self) -> RangeInclusive<i32> {
        let from = i32::from(self.from);
        -from..=(i32::from(MOST_WAITS) - from).max(0)
    }

    fn apply(&self, stage: &Stage, step: i32) -> Option<Stage> {
        let foe_waits = self.waits(step)?;
        Some(stage.retuned(Tuning {
            foe_waits,
            ..stage.tuning()
        }))
    }

    fn describe(&self, step: i32) -> String {
        let turns = |n: u8| match n {
            1 => "1 turn".to_owned(),
            n => format!("{n} turns"),
        };
        let waits = self.waits(step).map_or_else(|| "?".to_owned(), turns);
        format!(
            "the foe waits {waits} before it moves, not {}",
            turns(self.from)
        )
    }
}

/// The knobs of a stage tuned as `tuning` says: [`Trip`], then
/// [`HeadStart`].
pub fn knobs(tuning: Tuning) -> (Trip, HeadStart) {
    (
        Trip {
            from: tuning.trip_percent,
        },
        HeadStart {
            from: tuning.foe_waits,
        },
    )
}

/// The autobalance's file, beside `shuyi.ron`: the tools' data, like
/// `playtest.ron`, so not in `files/` and its pack.
pub const BALANCE_FILE: &str = "balance.ron";

/// What `balance.ron` holds: the stage the autobalance search starts from,
/// and the bands it should land the heroes in besides `playtest.ron`'s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Balance {
    /// How `files/stage.map` is tuned as authored: where the search
    /// starts, and what `--apply` writes the tuning found to.
    pub tuning: Tuning,
    /// The checks besides `playtest.ron`'s.
    pub checks: Vec<Check>,
}

/// What comes before the RON in `balance.ron`.
const BALANCE_HEADER: &str = "\
// The autobalance's stage (`cargo shuyi autobalance`): files/stage.map
// tuned as `tuning` says, which the search changes with the knobs of
// `src/tools.rs`, until the heroes land in the bands of playtest.ron and
// of `checks`. `cargo shuyi autobalance --apply` writes the tuning it
// finds here; the game itself plays the tuning of `rules::Tuning::default`.
";

/// `balance.ron`, read.
pub fn balance() -> Result<Balance, String> {
    let text = include_str!("../balance.ron");
    ron::from_str(text).map_err(|e| format!("{BALANCE_FILE}: {e}"))
}

/// `balance` as `balance.ron` holds it: its comment, then its RON.
pub fn balance_text(balance: &Balance) -> Result<String, String> {
    // The same file on every machine: RON's own new line is the system's.
    let config = ron::ser::PrettyConfig::new()
        .indentor("    ")
        .new_line("\n");
    let ron = ron::ser::to_string_pretty(balance, config).map_err(|e| e.to_string())?;
    Ok(format!("{BALANCE_HEADER}{ron}\n"))
}

/// The autobalance's plan: [`ENTRANTS`] on `files/stage.map` tuned as
/// `balance.ron` says, judged by those of [`playtest_checks`] and of its
/// `checks` whose entrants all play, changed by [`knobs`], each try played
/// by [`play_tries`] from its seed alone. The stage found is written to
/// `path` with its new tuning.
pub fn autobalance_plan(path: PathBuf) -> Result<AutobalancePlan, String> {
    let file = balance()?;
    let stage = Stage::with_tuning(&loaded()?.stage_map, 0, file.tuning)?;
    let mut checks = playtest_checks()?;
    checks.extend(file.checks.iter().cloned());
    let (trip, head_start) = knobs(file.tuning);
    Ok(AutobalancePlan::new(
        format!("the stage (files/stage.map), tuned as {BALANCE_FILE} says"),
        ENTRANTS.iter().map(|e| (e.name.to_owned(), None)).collect(),
        Box::new(move |names| {
            checks
                .iter()
                .filter(|c| c.entrants().iter().all(|e| names.contains(e)))
                .cloned()
                .collect()
        }),
        Box::new(Tunable::new(
            stage,
            vec![Box::new(trip), Box::new(head_start)],
            Box::new(|stage, entrant, seed, turns| {
                let mut tries = play_tries(stage, &ENTRANTS[entrant], seed..seed + 1, turns)
                    .map_err(|e| e.to_string())?;
                tries.pop().ok_or_else(|| "no try".to_owned())
            }),
            Box::new(move |stage: &Stage| {
                let text = balance_text(&Balance {
                    tuning: stage.tuning(),
                    checks: file.checks.clone(),
                })?;
                std::fs::write(&path, text).map_err(|e| format!("{}: {e}", path.display()))?;
                Ok(vec![BALANCE_FILE.to_owned()])
            }),
        )),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_shown_a_line_each() {
        let errors = [
            ContentError::new("files/a.ron", "bad"),
            ContentError::new("files/b.ron", "worse"),
        ];
        assert_eq!(lines(&errors), "files/a.ron: bad\nfiles/b.ron: worse");
        assert_eq!(lines(&[]), "");
    }

    #[test]
    fn balance_ron_is_as_apply_writes_it() {
        let file = balance().unwrap();
        assert_eq!(
            balance_text(&file).unwrap(),
            include_str!("../balance.ron").replace("\r\n", "\n")
        );
    }
}
