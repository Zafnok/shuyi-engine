//! A level: a text file in the engine's `.map` format, read through the
//! format registry, with the game's own terrains and features.

use serde::{Deserialize, Serialize};
use shuyi::assets::Formats;
use shuyi::assets::format::AssetKind;
use shuyi::assets::grid_map::{self, GridMap, MapFeature, MapFormat, MapText};
use shuyi::assets::ron::Ron;
use shuyi::core::TerrainId;
use shuyi::prelude::*;

use crate::rules::Puzzle;

// ANCHOR: terrains
/// Floor: the player and the boxes go over it.
pub const FLOOR: TerrainId = TerrainId(0);
/// A wall: nothing goes through it.
pub const WALL: TerrainId = TerrainId(1);
/// A target: floor a box should end on.
pub const TARGET: TerrainId = TerrainId(2);

/// Each terrain's id in a level's legend, by [`TerrainId`].
pub const TERRAIN_NAMES: [&str; 3] = ["floor", "wall", "target"];

/// The terrain a legend's id names.
pub fn terrain_named(name: &str) -> Option<TerrainId> {
    let index = TERRAIN_NAMES.iter().position(|&n| n == name)?;
    u16::try_from(index).ok().map(TerrainId)
}

/// What a tile may hold besides its terrain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Feature {
    /// The player starts here.
    Start,
    /// A box starts here.
    Box,
}

impl MapFeature for Feature {}
// ANCHOR_END: terrains

// ANCHOR: read
/// A level, as its file says.
pub type Level = GridMap<TerrainId, Feature>;

/// The format of a level: one tile look, at most 20 tiles a side, which
/// the screen shows whole.
pub const FORMAT: MapFormat<'static> = MapFormat::new(&[grid_map::DEFAULT_TILES], 20);

/// The formats the game reads its own files with.
pub fn formats() -> Formats {
    let mut formats = Formats::standard();
    // The first format of its kind: never refused.
    let _ = formats.register(MapText::<TerrainId, Feature>::new(FORMAT, terrain_named));
    // ANCHOR: register
    let _ = formats.register(Ron::<LevelList>::new());
    // ANCHOR_END: register
    formats
}

/// Reads the level in file `name`, which must make a puzzle.
pub fn read(bundle: &Bundle, name: &str) -> Result<Level, Vec<ContentError>> {
    let level = formats().load::<Level>(bundle, name)?;
    Puzzle::new(&level).map_err(|e| vec![ContentError::new(bundle.display_path(name), e)])?;
    Ok(level)
}
// ANCHOR_END: read

// ANCHOR: list
/// The level list: each level's file, in the order they are played.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct LevelList(pub Vec<String>);

impl AssetKind for LevelList {
    const NAME: &'static str = "level list";
}

/// Reads every level the list in `levels.ron` names.
pub fn read_all(bundle: &Bundle) -> Result<Vec<Level>, Vec<ContentError>> {
    // check-text: not player text, a file name
    let list = formats().load::<LevelList>(bundle, "levels.ron")?;
    let mut levels = Vec::new();
    let mut errors = Vec::new();
    for name in &list.0 {
        match read(bundle, name) {
            Ok(level) => levels.push(level),
            Err(problems) => errors.extend(problems),
        }
    }
    if errors.is_empty() {
        Ok(levels)
    } else {
        Err(errors)
    }
}
// ANCHOR_END: list
