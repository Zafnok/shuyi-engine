//! The puzzle's rules, pure and deterministic: a state that changes only
//! through commands. They know nothing of screens, keys or the clock.

use core::fmt;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use shuyi::core::{Dir, Grid, Pos, TerrainId, TurnRules};

use crate::level::{Feature, Level, TARGET, WALL};

// ANCHOR: state
/// The puzzle: its tiles, where the player stands and where the boxes are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Puzzle {
    /// The tiles, which never change.
    tiles: Grid<TerrainId>,
    /// Where the player stands.
    player: Pos,
    /// Where the boxes are.
    boxes: BTreeSet<Pos>,
}
// ANCHOR_END: state

// ANCHOR: command
/// What the player asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    /// Take one step that way.
    Step(Dir),
}

/// What happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Event {
    /// The player walked to this tile.
    Walked(Pos),
    /// A box was pushed from one tile to another.
    Pushed {
        /// Where it was.
        from: Pos,
        /// Where it is now.
        to: Pos,
    },
}

/// Why a command was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A wall is that way.
    Wall,
    /// The box that way has a wall or another box behind it.
    Stuck,
    /// Every box is already on a target.
    Over,
}

impl fmt::Display for Refusal {
    // check-text: not player text, what a test or a tool prints
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Refusal::Wall => "a wall is that way",
            Refusal::Stuck => "the box can't move that way",
            Refusal::Over => "every box is on a target",
        })
    }
}
// ANCHOR_END: command

/// How a level ends: only one way, every box on a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Won;

impl Puzzle {
    /// The puzzle `level` lays out: it needs one start, a box, and as many
    /// targets as boxes.
    pub fn new(level: &Level) -> Result<Self, String> {
        let mut player = None;
        let mut boxes = BTreeSet::new();
        for (&pos, feature) in &level.features {
            match feature {
                Feature::Start if player.is_some() => return Err("two starts".to_owned()),
                Feature::Start => player = Some(pos),
                Feature::Box => {
                    boxes.insert(pos);
                }
            }
        }
        let player = player.ok_or("no start")?;
        let puzzle = Self {
            tiles: level.tiles.clone(),
            player,
            boxes,
        };
        let targets = puzzle.targets().len();
        if puzzle.boxes.is_empty() || puzzle.boxes.len() != targets {
            let boxes = puzzle.boxes.len();
            return Err(format!("{boxes} boxes and {targets} targets"));
        }
        Ok(puzzle)
    }

    /// The tiles.
    pub fn tiles(&self) -> &Grid<TerrainId> {
        &self.tiles
    }

    /// Where the player stands.
    pub fn player(&self) -> Pos {
        self.player
    }

    /// Where the boxes are.
    pub fn boxes(&self) -> &BTreeSet<Pos> {
        &self.boxes
    }

    /// Every target tile.
    pub fn targets(&self) -> BTreeSet<Pos> {
        let (width, height) = (self.tiles.width(), self.tiles.height());
        (0..i32::from(height))
            .flat_map(|y| (0..i32::from(width)).map(move |x| Pos::new(x, y)))
            .filter(|&pos| self.tiles.get(pos) == Some(&TARGET))
            .collect()
    }

    /// Whether `pos` is a wall, or off the map.
    fn is_wall(&self, pos: Pos) -> bool {
        self.tiles.get(pos).is_none_or(|&tile| tile == WALL)
    }
}

// ANCHOR: rules
impl TurnRules for Puzzle {
    type Command = Command;
    type Event = Event;
    type Error = Refusal;
    type Outcome = Won;

    fn check(&self, command: &Command) -> Result<(), Refusal> {
        if self.outcome().is_some() {
            return Err(Refusal::Over);
        }
        let Command::Step(dir) = *command;
        let to = self.player.step(dir);
        if self.is_wall(to) {
            return Err(Refusal::Wall);
        }
        // ANCHOR: push_check
        let past = to.step(dir);
        if self.boxes.contains(&to) && (self.is_wall(past) || self.boxes.contains(&past)) {
            return Err(Refusal::Stuck);
        }
        // ANCHOR_END: push_check
        Ok(())
    }

    fn apply(&mut self, command: &Command) -> Result<Vec<Event>, Refusal> {
        // All or nothing: a refused command changes nothing.
        self.check(command)?;
        let Command::Step(dir) = *command;
        let to = self.player.step(dir);
        // ANCHOR: push_apply
        let mut events = Vec::new();
        if self.boxes.remove(&to) {
            let past = to.step(dir);
            self.boxes.insert(past);
            events.push(Event::Pushed { from: to, to: past });
        }
        // ANCHOR_END: push_apply
        self.player = to;
        events.push(Event::Walked(to));
        Ok(events)
    }

    fn legal_commands(&self) -> Vec<Command> {
        Dir::ALL
            .into_iter()
            .map(Command::Step)
            .filter(|command| self.check(command).is_ok())
            .collect()
    }

    // ANCHOR: outcome
    /// Won once every box is on a target.
    fn outcome(&self) -> Option<Won> {
        (self.boxes == self.targets()).then_some(Won)
    }
    // ANCHOR_END: outcome
}
// ANCHOR_END: rules
