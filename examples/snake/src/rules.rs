//! The snake's rules, pure and deterministic over the engine's [`TickSim`]
//! (ADR-0148): a board of floor and walls, a snake that moves one tile each
//! time its [`Every`] fires, and food on a free tile drawn from the seeded
//! RNG kept in the state. A direction pressed in a step's [`TickInput`]
//! turns it at its next move; up to two turns wait, so two quick presses
//! both count, and a turn back into its neck is ignored. Eating grows it by
//! one at its next move and scores a point, and every few points it moves
//! a step sooner. A wall or its own body ends the run, and so does filling
//! the board. The same seed and the same inputs always play out the same,
//! at any frame rate: no clock, no I/O and no float.
#![deny(clippy::float_arithmetic)]

use serde::{Deserialize, Serialize};
use shuyi::assets::format::AssetKind;
use shuyi::assets::grid_map::{self, GridMap, MapFeature, MapFormat};
use shuyi::assets::keymap::{ActionSet, UiAction};
use shuyi::core::tick::{ActionId, TickInput, TickSim};
use shuyi::core::{Dir, Grid, Pos, RandomSource, SimRng, TerrainId, timer::Every};

/// Floor: the snake moves on it, and food lands on it.
pub const FLOOR: TerrainId = TerrainId(0);
/// A wall: the snake dies on it.
pub const WALL: TerrainId = TerrainId(1);

/// Each terrain's id in a `.map` legend, by [`TerrainId`].
pub const TERRAIN_NAMES: [&str; 2] = ["floor", "wall"];

/// The widest board: a glyph tile is two cells across, on a console of 50.
pub const MAX_WIDTH: u16 = 25;
/// The tallest board: the console's 15 rows, less the score line.
pub const MAX_HEIGHT: u16 = 14;

/// The terrain a legend's id names.
pub fn terrain_named(name: &str) -> Option<TerrainId> {
    let index = TERRAIN_NAMES.iter().position(|&n| n == name)?;
    u16::try_from(index).ok().map(TerrainId)
}

/// What a `.map` tile may hold besides its terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feature {
    /// The snake's head starts here, its body to the left.
    Start,
}

impl MapFeature for Feature {}

/// The `.map` format of a board: one tile look, at most 32 tiles a side
/// (the rules then hold it to [`MAX_WIDTH`] × [`MAX_HEIGHT`]).
pub const FORMAT: MapFormat<'static> = MapFormat::new(&[grid_map::DEFAULT_TILES], 32);

/// A board as its file reads.
pub type Board = GridMap<TerrainId, Feature>;

/// The numbers of a run, `files/tuning.ron`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "tools", derive(schemars::JsonSchema))]
pub struct Tuning {
    /// The snake's tiles at the start.
    pub start_length: u16,
    /// The steps between two moves at the start (60 steps a second).
    pub first_period: u32,
    /// The fewest steps between two moves.
    pub fastest_period: u32,
    /// The points between two speed-ups.
    pub points_per_speed_up: u32,
}

impl AssetKind for Tuning {
    const NAME: &'static str = "tuning";
}

/// The id a step's input gives `action` (its place in [`UiAction::ALL`]).
fn id(action: UiAction) -> ActionId {
    let place = UiAction::ALL.iter().position(|&a| a == action);
    ActionId(place.and_then(|p| u16::try_from(p).ok()).unwrap_or(u16::MAX))
}

/// The four direction actions and their directions.
fn turns_of() -> [(ActionId, Dir); 4] {
    [
        (id(UiAction::Up), Dir::Up),
        (id(UiAction::Down), Dir::Down),
        (id(UiAction::Left), Dir::Left),
        (id(UiAction::Right), Dir::Right),
    ]
}

/// The way back.
pub const fn opposite(dir: Dir) -> Dir {
    match dir {
        Dir::Right => Dir::Left,
        Dir::Left => Dir::Right,
        Dir::Up => Dir::Down,
        Dir::Down => Dir::Up,
    }
}

/// What happened in a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The snake turned, as it moved.
    Turned {
        /// Its heading now.
        to: Dir,
    },
    /// Its head moved onto this tile.
    Moved {
        /// The tile.
        head: Pos,
    },
    /// It ate the food, and has this score now.
    Ate {
        /// The score.
        score: u32,
    },
    /// It hit a wall or itself.
    Died,
    /// It fills the board.
    Filled,
}

/// How a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "tools", derive(schemars::JsonSchema))]
pub enum Outcome {
    /// It hit a wall or itself on this step, this long.
    Died {
        /// The step, from 1.
        step: u64,
        /// Its tiles.
        length: u32,
    },
    /// It fills the board.
    Filled,
}

/// The most turns that wait for the snake's next moves.
pub const MAX_TURNS: usize = 2;

/// A run: the board, the snake (its head first), its heading and the turns
/// waiting, the food, the score, the timer of its moves, the steps run, the
/// luck to come and, once it is over, how it ended. It serializes for its
/// state hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Snake {
    board: Grid<TerrainId>,
    tuning: Tuning,
    body: Vec<Pos>,
    heading: Dir,
    turns: Vec<Dir>,
    grow: u32,
    food: Option<Pos>,
    score: u32,
    mover: Every,
    step: u64,
    rng: SimRng,
    outcome: Option<Outcome>,
}

impl Snake {
    /// The run of `board` with `tuning`, its luck seeded with `seed`: the
    /// snake on the `Start` tile heading right, its body to the left, and
    /// the first food. Fails unless the board fits [`MAX_WIDTH`] ×
    /// [`MAX_HEIGHT`] and has one `Start`, with floor under the whole
    /// snake.
    pub fn new(board: &Board, tuning: Tuning, seed: u64) -> Result<Self, String> {
        let (width, height) = (board.tiles.width(), board.tiles.height());
        if width > MAX_WIDTH || height > MAX_HEIGHT {
            return Err(format!(
                "the board is {width} × {height} tiles: at most {MAX_WIDTH} × {MAX_HEIGHT} fit the screen"
            ));
        }
        let starts: Vec<Pos> = board.features.keys().copied().collect();
        let [start] = starts[..] else {
            return Err(format!("the board has {} Start tiles, not one", starts.len()));
        };
        if tuning.start_length == 0 || tuning.fastest_period == 0 {
            return Err("the start length and the fastest period must be at least 1".to_owned());
        }
        let body: Vec<Pos> = (0..i32::from(tuning.start_length))
            .map(|i| Pos::new(start.x - i, start.y))
            .collect();
        if let Some(p) = body.iter().find(|&&p| board.tiles.get(p) != Some(&FLOOR)) {
            return Err(format!(
                "the snake starts on ({}, {}), which is not floor",
                p.x, p.y
            ));
        }
        let mut snake = Self {
            board: board.tiles.clone(),
            tuning,
            body,
            heading: Dir::Right,
            turns: Vec::new(),
            grow: 0,
            food: None,
            score: 0,
            mover: Every::new(tuning.first_period),
            step: 0,
            rng: SimRng::new(seed),
            outcome: None,
        };
        snake.food = place_food(&snake.board, &snake.body, &mut snake.rng);
        Ok(snake)
    }

    /// The board.
    pub fn board(&self) -> &Grid<TerrainId> {
        &self.board
    }

    /// The snake's tiles, its head first.
    pub fn body(&self) -> &[Pos] {
        &self.body
    }

    /// Its head.
    pub fn head(&self) -> Pos {
        self.body[0]
    }

    /// Its tiles.
    pub fn length(&self) -> u32 {
        u32::try_from(self.body.len()).unwrap_or(u32::MAX)
    }

    /// Where it is heading.
    pub fn heading(&self) -> Dir {
        self.heading
    }

    /// The turns waiting for its next moves, oldest first.
    pub fn turns(&self) -> &[Dir] {
        &self.turns
    }

    /// The food, if the board has a free tile for it.
    pub fn food(&self) -> Option<Pos> {
        self.food
    }

    /// The food eaten.
    pub fn score(&self) -> u32 {
        self.score
    }

    /// The steps between two moves now.
    pub fn period(&self) -> u32 {
        self.mover.period()
    }

    /// The steps run.
    pub fn steps(&self) -> u64 {
        self.step
    }

    /// How the run ended, once it has.
    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    /// The floor tiles: the most the snake can fill.
    pub fn floor_tiles(&self) -> u32 {
        let floor = self.board.cells().iter().filter(|&&t| t == FLOOR).count();
        u32::try_from(floor).unwrap_or(u32::MAX)
    }

    /// Whether the snake may move onto `pos`: floor, and none of its body
    /// but the tail that leaves this move (when it isn't growing).
    fn free_to_move(&self, pos: Pos) -> bool {
        let leaving = usize::from(self.grow == 0);
        let body = &self.body[..self.body.len() - leaving];
        self.board.get(pos) == Some(&FLOOR) && !body.contains(&pos)
    }

    /// Queues the turns `input` presses, in order: up to [`MAX_TURNS`], each
    /// a change of the way it will be heading, never straight back.
    fn queue_turns(&mut self, input: &TickInput) {
        let Some(seat) = input.seat(0) else {
            return;
        };
        let turns = turns_of();
        for pressed in &seat.pressed {
            let Some(&(_, dir)) = turns.iter().find(|(id, _)| id == pressed) else {
                continue;
            };
            let last = self.turns.last().copied().unwrap_or(self.heading);
            if self.turns.len() < MAX_TURNS && dir != last && dir != opposite(last) {
                self.turns.push(dir);
            }
        }
    }

    /// One move: the next turn taken, the head onto the next tile, the tail
    /// following unless it grows, and the food eaten if it is there.
    fn advance(&mut self, events: &mut Vec<Event>) {
        if !self.turns.is_empty() {
            self.heading = self.turns.remove(0);
            events.push(Event::Turned { to: self.heading });
        }
        let next = self.head().step(self.heading);
        if !self.free_to_move(next) {
            self.outcome = Some(Outcome::Died {
                step: self.step,
                length: self.length(),
            });
            events.push(Event::Died);
            return;
        }
        self.body.insert(0, next);
        if self.grow > 0 {
            self.grow -= 1;
        } else {
            self.body.pop();
        }
        events.push(Event::Moved { head: next });
        if self.food == Some(next) {
            self.eat(events);
        }
        if self.length() >= self.floor_tiles() {
            self.outcome = Some(Outcome::Filled);
            events.push(Event::Filled);
        }
    }

    /// The food eaten: a point, a tile more at the next move, a step
    /// sooner every few points down to the fastest, and the next food.
    fn eat(&mut self, events: &mut Vec<Event>) {
        self.score += 1;
        self.grow += 1;
        events.push(Event::Ate { score: self.score });
        let every = self.tuning.points_per_speed_up.max(1);
        let period = self.mover.period();
        if self.score.is_multiple_of(every) && period > self.tuning.fastest_period {
            self.mover = Every::new(period - 1);
        }
        self.food = place_food(&self.board, &self.body, &mut self.rng);
    }
}

/// A tile for the food, drawn by `rng` from the floor tiles of `board` that
/// `body` isn't on, in reading order (one draw, [`RandomSource::pick`]);
/// `None` if there is none.
pub fn place_food(
    board: &Grid<TerrainId>,
    body: &[Pos],
    rng: &mut impl RandomSource,
) -> Option<Pos> {
    let free: Vec<Pos> = board
        .positions()
        .filter(|&p| board.get(p) == Some(&FLOOR) && !body.contains(&p))
        .collect();
    rng.pick(&free).copied()
}

impl TickSim for Snake {
    type Event = Event;

    fn step(&mut self, input: &TickInput) -> Vec<Event> {
        let mut events = Vec::new();
        if self.outcome.is_some() {
            return events;
        }
        self.step += 1;
        self.queue_turns(input);
        for _ in 0..self.mover.advance(1) {
            if self.outcome.is_none() {
                self.advance(&mut events);
            }
        }
        events
    }

    fn hash(&self) -> u64 {
        shuyi::core::state_hash(self).map_or(0, |hash| hash.0)
    }
}

#[cfg(test)]
mod tests;
