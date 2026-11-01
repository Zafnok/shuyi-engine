//! Chapter 5, assets and formats: the garden's plan as a kind of asset of
//! the game's own, read from `files/garden.ron` through the format
//! registry, the bundle's layers, a player's mod laid over the game's
//! files, and the plan reloading as its file changes.

use std::sync::Arc;

use serde::Deserialize;
use shuyi::assets::format::AssetKind;
use shuyi::assets::hot::Hot;
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

// ANCHOR: hot
/// The garden's plan, reloading as its file changes. A screen reads it from
/// the context, as it reads any module's state.
pub struct HotPlan {
    /// The plan: reads like a `Plan`.
    pub plan: Hot<Plan>,
}

impl HotPlan {
    /// The plan of `bundle`, or every problem with it.
    pub fn new(bundle: &Bundle) -> Result<Self, Vec<ContentError>> {
        let plan = Hot::load(&Arc::new(formats()), bundle, "garden")?;
        Ok(Self { plan })
    }
}

impl<G: GameData> Module<G> for HotPlan {
    fn name(&self) -> &'static str {
        "hot_plan"
    }

    fn install(&mut self, install: &mut Install<'_, G>) {
        install.hook(Hook::Reloaded);
    }

    fn reloaded(&mut self, ctx: &mut Ctx<G>, changed: &[String]) {
        // A broken file keeps the old plan, and shows its error.
        let reload = self.plan.refresh(changed);
        ctx.note_reload(self.plan.id(), &reload);
    }
}
// ANCHOR_END: hot

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use shuyi::assets::Layer;
    use shuyi::assets::hot::Reload;

    use super::*;
    use crate::game;

    #[test]
    fn a_hot_plan_follows_its_file_and_keeps_the_last_good_one() {
        let plan =
            |plots: u32| format!("(plots: {plots}, seeds: 20, grow_percent: 50)").into_bytes();
        let files = BTreeMap::from([("garden.ron".to_owned(), plan(12))]);
        let layer = Layer::owned("assets".to_owned(), files).unwrap();
        let bundle = Bundle::new(vec![layer]);
        let mut hot = HotPlan::new(&bundle).unwrap();
        assert_eq!(*hot.plan, Plan::new(12, 20, 50));
        assert!(bundle.replace("garden.ron", plan(14)));
        let changed = ["garden.ron".to_owned()];
        assert_eq!(hot.plan.refresh(&changed), Reload::Reloaded);
        assert_eq!(*hot.plan, Plan::new(14, 20, 50));
        assert!(bundle.replace("garden.ron", b"(plots".to_vec()));
        assert!(matches!(hot.plan.refresh(&changed), Reload::Failed(_)));
        assert_eq!(*hot.plan, Plan::new(14, 20, 50));
    }

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
        // A seed that always grows is a plan.
        let files = BTreeMap::from([(
            "garden.ron".to_owned(),
            b"(plots: 6, seeds: 9, grow_percent: 100)".to_vec(),
        )]);
        let layer = Layer::owned("mods/sure".to_owned(), files).unwrap();
        assert_eq!(
            read_plan(&Bundle::new(vec![layer])),
            Ok(Plan::new(6, 9, 100))
        );
    }
}
