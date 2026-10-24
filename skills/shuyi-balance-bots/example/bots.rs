//! The stage's playtest (`cargo shuyi playtest`): two named bots for your
//! piece, its entrants, play the stage from the same seeds, and
//! `playtest.ron` says how often each should reach the goal. Tool code,
//! not a screen: `shuyi.ron`'s `check_text` skips this file.

use rules::{Command, Event, Feature, Outcome, Player, Stage};
use shuyi::assets::grid_map::GridMap;
use shuyi::core::bots::playtest::{Check, Entrant, Measure, Measures, play_tries};
use shuyi::core::{
    Bot, Determinize, Horizon, Pos, RandomBot, Report, Rollout, RolloutSettings, TerrainId,
    TurnRules, Unordered,
};
use shuyi_cli::PlaytestPlan;

/// How a try is measured: `won` (1 if your piece reached the goal, else
/// 0) and `turns` (the turn it stopped on).
#[derive(Debug, Clone, Copy, Default)]
pub struct Won;

impl Measure<Stage> for Won {
    fn observe(&mut self, _: &Stage, _: &Command, _: &[Event]) {}

    fn finish(self: Box<Self>, report: &Report<Stage>, _: &Stage) -> Measures {
        let won = matches!(report.outcome, Some(Outcome::Won { .. }));
        Measures::from([
            ("won".to_owned(), i64::from(won)),
            ("turns".to_owned(), i64::from(report.turns)),
        ])
    }
}

/// How good `end` is for your piece, in whole points: the goal is best,
/// being caught worst, and a tile nearer the goal better.
fn score(_: &Stage, end: &Stage, _: Player) -> i32 {
    match end.outcome() {
        Some(Outcome::Won { .. }) => 1000,
        Some(Outcome::Caught { .. }) => -1000,
        None => -i32::try_from(Pos::manhattan(end.hero(), end.goal())).unwrap_or(i32::MAX),
    }
}

/// How the rollout bot thinks: its first 4 commands, each played out on
/// 8 copies for 6 turns, at most 30 commands a copy, always the best.
pub const ROLLOUT: RolloutSettings = RolloutSettings::new(4, 8, Horizon::Turns(6), 30, 0);

/// A named bot for your piece: an entrant of the playtest.
#[derive(Debug, Clone, Copy)]
pub struct Hero {
    /// Its name, as `playtest.ron` names it.
    pub name: &'static str,
    /// Its bot for the try of a seed.
    pub bot: fn(u64) -> Box<dyn Bot<Stage>>,
}

impl Entrant<Stage, Stage> for Hero {
    fn name(&self) -> &str {
        self.name
    }

    /// The stage with its luck drawn from `seed`.
    fn start(&self, stage: &Stage, seed: u64) -> Stage {
        stage.determinize(Player, seed)
    }

    fn bots(&self, seed: u64) -> Vec<(Player, Box<dyn Bot<Stage>>)> {
        vec![(Player, (self.bot)(seed))]
    }

    fn measure(&self) -> Box<dyn Measure<Stage>> {
        Box::new(Won)
    }
}

/// The random bot of the try of `seed`: any step it may take.
fn random(seed: u64) -> Box<dyn Bot<Stage>> {
    Box::new(RandomBot::new(seed))
}

/// The rollout bot of the try of `seed`: it plans its next step on copies
/// of the stage, played on by random steps.
fn rollout(seed: u64) -> Box<dyn Bot<Stage>> {
    Box::new(Rollout::new(
        Unordered,
        RandomBot::new(seed),
        score,
        ROLLOUT,
        seed,
    ))
}

/// The entrants, in the report's order.
pub const HEROES: [Hero; 2] = [
    Hero {
        name: "random",
        bot: random,
    },
    Hero {
        name: "rollout",
        bot: rollout,
    },
];

/// The checks, `crates/game/playtest.ron`: the tools' data, not the
/// game's, so not in `files/`.
pub fn checks() -> Result<Vec<Check>, String> {
    let text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/playtest.ron"));
    ron::from_str(text).map_err(|e| format!("playtest.ron: {e}"))
}

/// The playtest of the stage of `map`: [`HEROES`], each try played by
/// `play_tries` from its seed alone, judged by those of [`checks`] whose
/// entrants all play.
pub fn plan(map: &GridMap<TerrainId, Feature>) -> Result<PlaytestPlan, String> {
    let stage = Stage::new(map, 0)?;
    let checks = checks()?;
    Ok(PlaytestPlan::new(
        "the stage (files/stage.map)",
        HEROES.iter().map(|h| (h.name.to_owned(), None)).collect(),
        Box::new(move |names| {
            checks
                .iter()
                .filter(|c| c.entrants().iter().all(|e| names.contains(e)))
                .cloned()
                .collect()
        }),
        Box::new(move |entrant, seed, turns| {
            let mut tries = play_tries(&stage, &HEROES[entrant], seed..seed + 1, turns)
                .map_err(|e| e.to_string())?;
            tries.pop().ok_or_else(|| "no try".to_owned())
        }),
    ))
}
