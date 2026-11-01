//! The puzzle's rules, pure and deterministic: a state that changes only
//! through commands. They know nothing of screens, keys or the clock.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use shuyi::core::{Grid, Pos, TerrainId};

use crate::level::{Feature, Level, TARGET};

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
}
