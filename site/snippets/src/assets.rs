//! Chapter 5, assets and formats: the garden's plan as a kind of asset of
//! the game's own, read from `files/garden.ron` through the format
//! registry, and the bundle's layers, a player's mod laid over the
//! game's files.

use serde::Deserialize;
use shuyi::assets::format::AssetKind;
use shuyi::assets::ron::Ron;
use shuyi::assets::{Bundle, ContentError, Formats};
use shuyi::prelude::*;

// ANCHOR: kind
/// The garden's plan: a kind of asset of the game's own, plain data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    /// How many plots the garden has.
    pub plots: u8,
    /// How many seeds the player starts with.
    pub seeds: u32,
    /// How often a seed grows, in percent.
    pub grow_percent: u8,
}

impl AssetKind for Plan {
    const NAME: &'static str = "garden plan";
}
// ANCHOR_END: kind

impl Plan {
    /// A plan of `plots` plots, `seeds` seeds and a `grow_percent` chance.
    pub fn new(plots: u8, seeds: u32, grow_percent: u8) -> Self {
        Self {
            plots,
            seeds,
            grow_percent,
        }
    }
}

// ANCHOR: formats
/// The formats the game reads its files with: the short path's, and a
/// RON file for its plan.
pub fn formats() -> Formats {
    let mut formats = BasicData::<UiAction, Plan>::formats();
    // The first format of its kind: never refused.
    let _ = formats.register(Ron::<Plan>::new());
    formats
}

/// Reads the game's own part, its plan, from `garden.ron` (the id needs
/// no extension: the registry finds the file a format of the kind reads),
/// then checks what the format can't.
pub fn read_plan(bundle: &Bundle) -> Result<Plan, Vec<ContentError>> {
    let plan = formats().load::<Plan>(bundle, "garden")?;
    if plan.grow_percent > 100 {
        let file = bundle.display_path("garden.ron");
        return Err(vec![ContentError::new(file, "grow_percent is over 100")]);
    }
    Ok(plan)
}
// ANCHOR_END: formats

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use shuyi::assets::Layer;

    use super::*;
    use crate::game;

    // ANCHOR: layers
    #[test]
    fn a_mod_laid_over_the_games_files_replaces_its_plan() {
        let files = BTreeMap::from([(
            "garden.ron".to_owned(),
            b"(plots: 12, seeds: 20, grow_percent: 50)".to_vec(),
        )]);
        let layer = Layer::owned("mods/big-garden".to_owned(), files).unwrap();
        // The first layer that has a file gives it.
        let bundle = Bundle::new(vec![layer, game().bundle().layers()[0]]);
        assert_eq!(read_plan(&bundle), Ok(Plan::new(12, 20, 50)));
        // Every other file still comes from the game's own layer.
        assert!(bundle.has("en.ron"));
    }
    // ANCHOR_END: layers

    #[test]
    fn a_plan_that_cant_be_is_refused_naming_its_file() {
        let files = BTreeMap::from([(
            "garden.ron".to_owned(),
            b"(plots: 6, seeds: 9, grow_percent: 140)".to_vec(),
        )]);
        let layer = Layer::owned("mods/broken".to_owned(), files).unwrap();
        let errors = read_plan(&Bundle::new(vec![layer])).unwrap_err();
        assert_eq!(errors.len(), 1, "{errors:?}");
        let shown = errors[0].to_string();
        assert!(shown.contains("mods/broken/garden.ron"), "{shown}");
    }
}
