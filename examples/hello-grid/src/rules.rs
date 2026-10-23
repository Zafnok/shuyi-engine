//! The rules, pure and deterministic over the engine's [`TurnRules`]: a
//! 12 × 10 stage of floor, walls and a goal; your piece walks up to
//! [`MOVES`] tiles a turn and wins on the goal; then the foe steps one
//! tile, and catches you if it steps onto your tile. Each step it may trip
//! instead ([`TRIP_PERCENT`]), rolled from the seeded RNG kept in the
//! state, so the same seed and the same commands always play out the same.
//! A stage's [`Tuning`] can change the trip chance and give your piece a
//! head start: the knobs of `tools::knobs` turn it.
//!
//! [`Chase`] is the foe's mind: a [`Bot`] that steps towards your piece.
//! The stage screen asks it for the foe's commands, and the bot test lets
//! it play against a random bot and against a rollout bot
//! (`shuyi_core::Rollout`). That one plans on copies of the stage from
//! [`Determinize`], ordered by [`hero_order`], played on by [`playout`]
//! ([`Walk`] and [`Chase`]) and judged by [`score`].

use std::fmt;

use serde::{Deserialize, Serialize};
use shuyi_assets::grid_map::{self, GridMap, MapFeature, MapFormat};
use shuyi_core::{Bot, Determinize, Dir, Grid, Pos, Reach, Sides, SimRng, TerrainId, TurnRules};
use shuyi_core::{flow_field, reachable};

/// Floor: anyone walks it.
pub const FLOOR: TerrainId = TerrainId(0);
/// A wall: no one walks it.
pub const WALL: TerrainId = TerrainId(1);
/// The goal: floor that wins the stage for your piece.
pub const GOAL: TerrainId = TerrainId(2);

/// Each terrain's id in a `.map` legend, by [`TerrainId`].
pub const TERRAIN_NAMES: [&str; 3] = ["floor", "wall", "goal"];

/// The terrain a legend's id names.
pub fn terrain_named(name: &str) -> Option<TerrainId> {
    let index = TERRAIN_NAMES.iter().position(|&n| n == name)?;
    u16::try_from(index).ok().map(TerrainId)
}

/// How many tiles your piece walks in a turn.
pub const MOVES: u32 = 4;

/// The chance, in percent, that the foe trips and stays where it is.
pub const TRIP_PERCENT: u8 = 25;

/// How a stage is tuned: the knobs of an autobalance search turn these
/// (`tools::knobs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tuning {
    /// The chance, in percent, that the foe trips on a step.
    pub trip_percent: u8,
    /// How many of its first turns the foe skips.
    pub foe_waits: u8,
}

impl Default for Tuning {
    /// [`TRIP_PERCENT`], and no wait.
    fn default() -> Self {
        Self {
            trip_percent: TRIP_PERCENT,
            foe_waits: 0,
        }
    }
}

/// Whether `value` is its type's default: a field that is isn't written.
fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

/// What a `.map` tile may hold besides its terrain: where the pieces start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feature {
    /// Your piece starts here.
    Hero,
    /// The foe starts here.
    Foe,
}

impl MapFeature for Feature {}

/// The `.map` format of a stage: one tile look, at most 32 tiles a side.
pub const FORMAT: MapFormat<'static> = MapFormat::new(&[grid_map::DEFAULT_TILES], 32);

/// Reads stage map `source` (errors name it `file`).
pub fn parse_map(file: &str, source: &str) -> Result<GridMap<TerrainId, Feature>, String> {
    grid_map::parse(file, source, &FORMAT, terrain_named).map_err(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        errors.join("\n")
    })
}

/// Who moves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    /// You.
    Hero,
    /// The foe.
    Foe,
}

/// What a side asks for. A record of a stage is its seed and these
/// (`tools::StageRecord`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Your piece walks to this tile, within its reach; its own tile waits.
    Move(Pos),
    /// The foe steps one tile that way, or stays (`None`).
    Step(Option<Dir>),
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Your piece walked.
    Moved {
        /// From here.
        from: Pos,
        /// To here.
        to: Pos,
    },
    /// Your piece is on the goal: you won.
    Reached,
    /// The foe stepped.
    Stepped {
        /// From here.
        from: Pos,
        /// To here.
        to: Pos,
    },
    /// The foe tripped and stayed where it was.
    Tripped,
    /// The foe skipped its turn: one of its first [`Tuning::foe_waits`].
    Waited,
    /// The foe stayed where it was, on purpose.
    Stayed,
    /// The foe is on your tile: you lost.
    Caught,
}

/// Why a command was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The stage is over.
    Over,
    /// It is the other side's move.
    NotYourMove(Side),
    /// Your piece can't walk there this turn.
    OutOfReach(Pos),
    /// The foe can't step that way.
    Blocked(Dir),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Over => f.write_str("the stage is over"),
            Self::NotYourMove(side) => write!(f, "it is {side:?}'s move"),
            Self::OutOfReach(pos) => write!(f, "({}, {}) is out of reach", pos.x, pos.y),
            Self::Blocked(dir) => write!(f, "the foe can't step {dir:?}"),
        }
    }
}

/// How the stage ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Your piece reached the goal on this turn.
    Won {
        /// The turn it got there.
        turn: u32,
    },
    /// The foe caught your piece on this turn.
    Caught {
        /// The turn it did.
        turn: u32,
    },
}

/// A stage being played: the map, the two pieces, whose move it is, the
/// turn, the luck to come, its [`Tuning`] and, once it is over, how it
/// ended. It serializes for its state hash (`shuyi_core::state_hash`),
/// which a replay compares step by step; the tuning only when it isn't the
/// default, so a stage of [`Stage::new`] hashes as it always has.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stage {
    map: Grid<TerrainId>,
    goal: Pos,
    hero: Pos,
    foe: Pos,
    side: Side,
    turn: u32,
    rng: SimRng,
    outcome: Option<Outcome>,
    #[serde(skip_serializing_if = "is_default")]
    tuning: Tuning,
}

impl Stage {
    /// The stage of `map`, its luck seeded with `seed`: your move, turn 1,
    /// tuned the default way. Fails unless the map has exactly one goal
    /// tile, one `Hero` and one `Foe`, on floor tiles of their own.
    pub fn new(map: &GridMap<TerrainId, Feature>, seed: u64) -> Result<Self, String> {
        Self::with_tuning(map, seed, Tuning::default())
    }

    /// [`Stage::new`], tuned as `tuning` says.
    pub fn with_tuning(
        map: &GridMap<TerrainId, Feature>,
        seed: u64,
        tuning: Tuning,
    ) -> Result<Self, String> {
        let tiles: Vec<Pos> = map.tiles.positions().collect();
        let goals: Vec<Pos> = tiles
            .iter()
            .copied()
            .filter(|&p| map.tiles.get(p) == Some(&GOAL))
            .collect();
        let [goal] = goals[..] else {
            return Err(format!("the map has {} goal tiles, not one", goals.len()));
        };
        let start = |feature: Feature| {
            let at: Vec<Pos> = map
                .features
                .iter()
                .filter(|&(_, &f)| f == feature)
                .map(|(&p, _)| p)
                .collect();
            match at[..] {
                [p] if map.tiles.get(p) == Some(&FLOOR) => Ok(p),
                [p] => Err(format!("{feature:?} at ({}, {}) is not on floor", p.x, p.y)),
                _ => Err(format!(
                    "the map has {} {feature:?} tiles, not one",
                    at.len()
                )),
            }
        };
        let (hero, foe) = (start(Feature::Hero)?, start(Feature::Foe)?);
        Ok(Self {
            map: map.tiles.clone(),
            goal,
            hero,
            foe,
            side: Side::Hero,
            turn: 1,
            rng: SimRng::new(seed),
            outcome: None,
            tuning,
        })
    }

    /// How it is tuned.
    pub fn tuning(&self) -> Tuning {
        self.tuning
    }

    /// This stage tuned as `tuning` says instead: how a knob changes a
    /// stage before it is played.
    #[must_use]
    pub fn retuned(&self, tuning: Tuning) -> Self {
        Self {
            tuning,
            ..self.clone()
        }
    }

    /// The terrain.
    pub fn map(&self) -> &Grid<TerrainId> {
        &self.map
    }

    /// The goal tile.
    pub fn goal(&self) -> Pos {
        self.goal
    }

    /// Where your piece is.
    pub fn hero(&self) -> Pos {
        self.hero
    }

    /// Where the foe is.
    pub fn foe(&self) -> Pos {
        self.foe
    }

    /// Whether anyone can stand on `pos`: it is on the map and no wall.
    pub fn walkable(&self, pos: Pos) -> bool {
        self.map.get(pos).is_some_and(|&t| t != WALL)
    }

    /// Where your piece can walk this turn: up to [`MOVES`] tiles of floor,
    /// round the foe.
    pub fn reach(&self) -> Reach {
        let cost = |pos: Pos| (self.walkable(pos) && pos != self.foe).then_some(1);
        reachable(self.map.width(), self.map.height(), self.hero, MOVES, cost)
    }

    /// How many steps each tile is from your piece, walls in the way.
    pub fn distances_to_hero(&self) -> Grid<Option<u32>> {
        let cost = |pos: Pos| self.walkable(pos).then_some(1);
        flow_field(self.map.width(), self.map.height(), [self.hero], cost)
    }

    /// How many steps each tile is from the goal, walls in the way.
    pub fn distances_to_goal(&self) -> Grid<Option<u32>> {
        let cost = |pos: Pos| self.walkable(pos).then_some(1);
        flow_field(self.map.width(), self.map.height(), [self.goal], cost)
    }

    fn check_side(&self, side: Side) -> Result<(), Error> {
        if self.outcome.is_some() {
            return Err(Error::Over);
        }
        if self.side == side {
            Ok(())
        } else {
            Err(Error::NotYourMove(self.side))
        }
    }

    /// Where the foe ends up stepping `dir`.
    fn step_to(&self, dir: Dir) -> Result<Pos, Error> {
        let to = self.foe.step(dir);
        if self.walkable(to) {
            Ok(to)
        } else {
            Err(Error::Blocked(dir))
        }
    }

    /// The foe's move is over: your move, on the next turn.
    fn next_turn(&mut self) {
        self.side = Side::Hero;
        self.turn += 1;
    }
}

impl TurnRules for Stage {
    type Command = Command;
    type Event = Event;
    type Error = Error;
    type Outcome = Outcome;

    fn check(&self, command: &Command) -> Result<(), Error> {
        match *command {
            Command::Move(to) => {
                self.check_side(Side::Hero)?;
                if self.reach().contains(to) {
                    Ok(())
                } else {
                    Err(Error::OutOfReach(to))
                }
            }
            Command::Step(dir) => {
                self.check_side(Side::Foe)?;
                dir.map_or(Ok(()), |dir| self.step_to(dir).map(|_| ()))
            }
        }
    }

    fn apply(&mut self, command: &Command) -> Result<Vec<Event>, Error> {
        self.check(command)?;
        let mut events = Vec::new();
        match *command {
            Command::Move(to) => {
                events.push(Event::Moved {
                    from: self.hero,
                    to,
                });
                self.hero = to;
                if to == self.goal {
                    events.push(Event::Reached);
                    self.outcome = Some(Outcome::Won { turn: self.turn });
                } else if self.turn <= u32::from(self.tuning.foe_waits) {
                    events.push(Event::Waited);
                    self.next_turn();
                } else {
                    self.side = Side::Foe;
                }
            }
            Command::Step(None) => {
                events.push(Event::Stayed);
                self.next_turn();
            }
            Command::Step(Some(dir)) => {
                let to = self.step_to(dir)?;
                if self.rng.roll_percent() < self.tuning.trip_percent {
                    events.push(Event::Tripped);
                } else {
                    events.push(Event::Stepped { from: self.foe, to });
                    self.foe = to;
                }
                if self.foe == self.hero {
                    events.push(Event::Caught);
                    self.outcome = Some(Outcome::Caught { turn: self.turn });
                } else {
                    self.next_turn();
                }
            }
        }
        Ok(events)
    }

    /// Your piece: every tile of its reach, row by row. The foe: staying,
    /// then each open step in [`Dir::ALL`]'s order.
    fn legal_commands(&self) -> Vec<Command> {
        if self.outcome.is_some() {
            return Vec::new();
        }
        match self.side {
            Side::Hero => self.reach().tiles().iter().map(Command::Move).collect(),
            Side::Foe => {
                let steps = Dir::ALL
                    .into_iter()
                    .filter(|&dir| self.step_to(dir).is_ok())
                    .map(Some);
                core::iter::once(None)
                    .chain(steps)
                    .map(Command::Step)
                    .collect()
            }
        }
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }
}

impl Sides for Stage {
    type Side = Side;

    fn side_to_move(&self) -> Side {
        self.side
    }

    fn turn(&self) -> u32 {
        self.turn
    }
}

/// The foe's mind: the step that brings it closest to your piece (the
/// first of [`Dir::ALL`] among equals), or staying if none does.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Chase;

impl Bot<Stage> for Chase {
    fn choose(&mut self, stage: &Stage) -> Command {
        let distances = stage.distances_to_hero();
        let far = |pos: Pos| distances.get(pos).copied().flatten().unwrap_or(u32::MAX);
        let mut best = (far(stage.foe), None);
        for dir in Dir::ALL {
            if let Ok(to) = stage.step_to(dir)
                && far(to) < best.0
            {
                best = (far(to), Some(dir));
            }
        }
        Command::Step(best.1)
    }
}

impl Determinize for Stage {
    /// The luck to come (the foe's trips) is redrawn; nothing else is
    /// hidden, so your side sees the whole board.
    fn determinize(&self, _: Side, seed: u64) -> Self {
        Self {
            rng: SimRng::new(seed),
            ..self.clone()
        }
    }
}

/// The steps a distance grid gives for `pos`: 99 for a tile no path
/// reaches.
fn steps(distances: &Grid<Option<u32>>, pos: Pos) -> i32 {
    let steps = distances.get(pos).copied().flatten().unwrap_or(99);
    i32::try_from(steps).unwrap_or(99).min(99)
}

/// How good `end` is for `side`, in points. For your piece: 1 000 for a
/// win, -1 000 for being caught, and between them the foe's steps to
/// your piece minus your piece's steps to the goal. The foe's score is
/// its negation. Only the end counts: `start` is unused.
pub fn score(_start: &Stage, end: &Stage, side: Side) -> i32 {
    let hero = match end.outcome {
        Some(Outcome::Won { .. }) => 1000,
        Some(Outcome::Caught { .. }) => -1000,
        None => {
            steps(&end.distances_to_hero(), end.foe) - steps(&end.distances_to_goal(), end.hero)
        }
    };
    match side {
        Side::Hero => hero,
        Side::Foe => -hero,
    }
}

/// Your piece's candidates, nearest the goal first: the tiles in reach,
/// sorted by their steps to the goal (in reach order among equals).
pub fn hero_order(stage: &Stage, mut commands: Vec<Command>) -> Vec<Command> {
    let distances = stage.distances_to_goal();
    commands.sort_by_key(|command| match command {
        Command::Move(to) => steps(&distances, *to),
        Command::Step(_) => i32::MAX,
    });
    commands
}

/// A plain walker for your piece: the tile in reach closest to the goal
/// (the first in reach order among equals).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Walk;

impl Bot<Stage> for Walk {
    fn choose(&mut self, stage: &Stage) -> Command {
        let distances = stage.distances_to_goal();
        let to = stage
            .reach()
            .tiles()
            .iter()
            .min_by_key(|&to| steps(&distances, to))
            .unwrap_or(stage.hero);
        Command::Move(to)
    }
}

/// How a rollout bot plays a copy of the stage on: [`Walk`] for your
/// piece, [`Chase`] for the foe.
pub fn playout(stage: &Stage) -> Command {
    match stage.side {
        Side::Hero => Walk.choose(stage),
        Side::Foe => Chase.choose(stage),
    }
}

#[cfg(test)]
mod tests;
