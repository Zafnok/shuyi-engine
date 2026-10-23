//! The game's rules, pure and deterministic over the engine's
//! [`TurnRules`]: a small stage of floor, walls and a goal. Your piece
//! steps one tile a turn and wins on the goal; then the wanderer steps one
//! tile, to the open tile its luck picks, rolled from the seeded RNG kept
//! in the state. If the two meet, you are caught. The same seed and the
//! same commands always play out the same, so a record of a game is its
//! seed and its commands.
#![cfg_attr(not(test), deny(clippy::float_arithmetic))]

use std::fmt;

use serde::{Deserialize, Serialize};
use shuyi_assets::grid_map::{self, GridMap, MapFeature, MapFormat};
use shuyi_core::{Dir, Grid, Pos, Sides, SimRng, TerrainId, TurnRules};

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

/// What a `.map` tile may hold besides its terrain: where the pieces
/// start.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feature {
    /// Your piece starts here.
    Hero,
    /// The wanderer starts here.
    Wanderer,
}

impl MapFeature for Feature {}

/// The `.map` format of a stage: one tile look, at most 32 tiles a side.
pub const FORMAT: MapFormat<'static> = MapFormat {
    looks: &[grid_map::DEFAULT_TILES],
    max_size: 32,
};

/// Reads stage map `source` (errors name it `file`).
pub fn parse_map(file: &str, source: &str) -> Result<GridMap<TerrainId, Feature>, String> {
    grid_map::parse(file, source, &FORMAT, terrain_named).map_err(|errors| {
        let errors: Vec<String> = errors.iter().map(ToString::to_string).collect();
        errors.join("\n")
    })
}

/// What you ask for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Your piece steps one tile that way.
    Step(Dir),
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// Your piece stepped.
    Stepped {
        /// From here.
        from: Pos,
        /// To here.
        to: Pos,
    },
    /// Your piece is on the goal: you won.
    Reached,
    /// The wanderer stepped.
    Wandered {
        /// From here.
        from: Pos,
        /// To here.
        to: Pos,
    },
    /// The two met: you lost.
    Caught,
}

/// Why a command was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The stage is over.
    Over,
    /// A wall, or the map's edge, is that way.
    Blocked(Dir),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Over => f.write_str("the stage is over"),
            Self::Blocked(dir) => write!(f, "your piece can't step {dir:?}"),
        }
    }
}

/// How the stage ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    /// Your piece reached the goal on this turn.
    Won {
        /// The turn.
        turn: u32,
    },
    /// The wanderer and your piece met on this turn.
    Caught {
        /// The turn.
        turn: u32,
    },
}

/// The one side that moves: you. The wanderer's step is part of the
/// rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player;

/// A stage being played: the map, the two pieces, the turn, the luck to
/// come and, once it is over, how it ended. It serializes for its state
/// hash (`shuyi_core::state_hash`), which a replay compares step by step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stage {
    map: Grid<TerrainId>,
    goal: Pos,
    hero: Pos,
    wanderer: Pos,
    turn: u32,
    rng: SimRng,
    outcome: Option<Outcome>,
}

impl Stage {
    /// The stage of `map`, its luck seeded with `seed`: turn 1. Fails
    /// unless the map has exactly one goal tile, one `Hero` and one
    /// `Wanderer`, on floor tiles of their own.
    pub fn new(map: &GridMap<TerrainId, Feature>, seed: u64) -> Result<Self, String> {
        let goals: Vec<Pos> = map
            .tiles
            .positions()
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
        Ok(Self {
            map: map.tiles.clone(),
            goal,
            hero: start(Feature::Hero)?,
            wanderer: start(Feature::Wanderer)?,
            turn: 1,
            rng: SimRng::new(seed),
            outcome: None,
        })
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

    /// Where the wanderer is.
    pub fn wanderer(&self) -> Pos {
        self.wanderer
    }

    /// Whether anyone can stand on `pos`: it is on the map and no wall.
    pub fn open(&self, pos: Pos) -> bool {
        self.map.get(pos).is_some_and(|&t| t != WALL)
    }

    /// The wanderer steps to one of its open neighbours (never the goal),
    /// picked by the luck; with none, it stays.
    fn wander(&mut self, events: &mut Vec<Event>) {
        let steps: Vec<Pos> = Dir::ALL
            .into_iter()
            .map(|dir| self.wanderer.step(dir))
            .filter(|&to| self.open(to) && to != self.goal)
            .collect();
        let count = u32::try_from(steps.len()).unwrap_or(u32::MAX);
        if count == 0 {
            return;
        }
        let pick = usize::try_from(self.rng.roll_below(count)).unwrap_or(0);
        let (from, to) = (self.wanderer, steps[pick]);
        self.wanderer = to;
        events.push(Event::Wandered { from, to });
    }
}

impl TurnRules for Stage {
    type Command = Command;
    type Event = Event;
    type Error = Error;
    type Outcome = Outcome;

    fn check(&self, command: &Command) -> Result<(), Error> {
        let Command::Step(dir) = *command;
        if self.outcome.is_some() {
            return Err(Error::Over);
        }
        if self.open(self.hero.step(dir)) {
            Ok(())
        } else {
            Err(Error::Blocked(dir))
        }
    }

    fn apply(&mut self, command: &Command) -> Result<Vec<Event>, Error> {
        self.check(command)?;
        let Command::Step(dir) = *command;
        let (from, to) = (self.hero, self.hero.step(dir));
        self.hero = to;
        let mut events = vec![Event::Stepped { from, to }];
        if to == self.goal {
            events.push(Event::Reached);
            self.outcome = Some(Outcome::Won { turn: self.turn });
            return Ok(events);
        }
        if to != self.wanderer {
            self.wander(&mut events);
        }
        if self.wanderer == self.hero {
            events.push(Event::Caught);
            self.outcome = Some(Outcome::Caught { turn: self.turn });
        } else {
            self.turn += 1;
        }
        Ok(events)
    }

    /// Each open step, in [`Dir::ALL`]'s order.
    fn legal_commands(&self) -> Vec<Command> {
        Dir::ALL
            .into_iter()
            .map(Command::Step)
            .filter(|c| self.check(c).is_ok())
            .collect()
    }

    fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }
}

impl Sides for Stage {
    type Side = Player;

    fn side_to_move(&self) -> Player {
        Player
    }

    fn turn(&self) -> u32 {
        self.turn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A test map's header, up to its features.
    const LEGEND: &str =
        "(name: \"t\", legend: { '.': \"floor\", '#': \"wall\", '>': \"goal\" }, features: { ";

    /// The stage of `rows` (`@` your piece, `&` the wanderer, `>` the
    /// goal), seeded `seed`.
    fn stage(rows: &str, seed: u64) -> Result<Stage, String> {
        let mut features = Vec::new();
        let mut tiles = String::new();
        for (y, row) in rows.lines().enumerate() {
            for (x, c) in row.chars().enumerate() {
                let feature = match c {
                    '@' => Some("Hero"),
                    '&' => Some("Wanderer"),
                    _ => None,
                };
                if let Some(feature) = feature {
                    features.push(format!("({x}, {y}): {feature}"));
                }
                tiles.push(if feature.is_some() { '.' } else { c });
            }
            tiles.push('\n');
        }
        let mut source = String::from(LEGEND);
        source.push_str(&features.join(", "));
        source.push_str(" })\n---\n");
        source.push_str(&tiles);
        Stage::new(&parse_map("t.map", &source)?, seed)
    }

    const OPEN: &str = "#######\n#@...>#\n#.....#\n#...&.#\n#######";

    #[test]
    fn a_stage_needs_one_goal_one_hero_and_one_wanderer() {
        assert!(stage(OPEN, 0).is_ok());
        let no_goal = stage("####\n#@&#\n####", 0).unwrap_err();
        assert!(no_goal.contains("0 goal tiles"), "{no_goal}");
        let no_wanderer = stage("####\n#@>#\n####", 0).unwrap_err();
        assert!(no_wanderer.contains("0 Wanderer tiles"), "{no_wanderer}");
        assert_eq!(terrain_named("wall"), Some(WALL));
        assert_eq!(terrain_named("lava"), None);
    }

    #[test]
    fn a_step_moves_your_piece_then_the_wanderer() {
        let mut s = stage(OPEN, 3).unwrap();
        let events = s.apply(&Command::Step(Dir::Right)).unwrap();
        assert_eq!(
            events[0],
            Event::Stepped {
                from: Pos::new(1, 1),
                to: Pos::new(2, 1)
            }
        );
        assert!(matches!(events[1], Event::Wandered { from, .. } if from == Pos::new(4, 3)));
        assert_eq!(Pos::manhattan(s.wanderer(), Pos::new(4, 3)), 1);
        assert_eq!(s.turn(), 2);
        assert_eq!(s.side_to_move(), Player);
        // The same seed and commands, the same game.
        let mut again = stage(OPEN, 3).unwrap();
        again.apply(&Command::Step(Dir::Right)).unwrap();
        assert_eq!(again, s);
    }

    #[test]
    fn walls_block_and_only_open_steps_are_legal() {
        let s = stage(OPEN, 0).unwrap();
        assert_eq!(
            s.check(&Command::Step(Dir::Up)),
            Err(Error::Blocked(Dir::Up))
        );
        assert_eq!(
            s.legal_commands(),
            [Command::Step(Dir::Right), Command::Step(Dir::Down)]
        );
        assert_eq!(
            Error::Blocked(Dir::Up).to_string(),
            "your piece can't step Up"
        );
    }

    #[test]
    fn the_goal_wins_and_ends_the_stage() {
        // The wanderer is walled in: it never moves.
        let mut s = stage("######\n#@>#&#\n######", 0).unwrap();
        let events = s.apply(&Command::Step(Dir::Right)).unwrap();
        assert_eq!(events.last(), Some(&Event::Reached));
        assert_eq!(s.outcome(), Some(Outcome::Won { turn: 1 }));
        let found = s.legal_commands();
        assert!(found.is_empty(), "{found:?}");
        assert_eq!(s.apply(&Command::Step(Dir::Left)), Err(Error::Over));
        assert_eq!(Error::Over.to_string(), "the stage is over");
    }

    #[test]
    fn meeting_the_wanderer_loses() {
        // Its only step is onto your piece's.
        let mut s = stage("#####\n#@.&#\n#>###\n#####", 0).unwrap();
        let events = s.apply(&Command::Step(Dir::Right)).unwrap();
        assert_eq!(events.last(), Some(&Event::Caught));
        assert_eq!(s.outcome(), Some(Outcome::Caught { turn: 1 }));
        // Stepping onto it loses too.
        let mut s = stage("#####\n#@&.#\n#>###\n#####", 0).unwrap();
        s.apply(&Command::Step(Dir::Right)).unwrap();
        assert_eq!(s.outcome(), Some(Outcome::Caught { turn: 1 }));
    }
}
