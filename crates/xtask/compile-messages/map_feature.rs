//~ `Feature` is not a map's tile feature
//~ write `impl MapFeature for Feature`, or use the engine's `NoFeature`
//~ https://zafnok.github.io/shuyi-engine/learn/book/frame-and-skins/
//! A game's code with its map's feature, its `impl MapFeature` missing (ticket 2510).

use shuyi::assets::grid_map::{self, GridMap, MapFormat};
use shuyi::assets::ContentError;

/// A thing on a tile.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
enum Feature {
    Chest,
}

fn read(source: &str) -> Result<GridMap<u8, Feature>, Vec<ContentError>> {
    grid_map::parse("stage.map", source, &MapFormat::new(&[], 32), |_| Some(0))
}

fn main() {
    let _ = (read, Feature::Chest);
}
