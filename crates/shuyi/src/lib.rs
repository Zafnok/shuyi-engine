//! Shuyi Engine, by its front door (ADR-0009 §6). A game depends on this
//! crate alone and turns on the features of the modules it wants, each
//! off by default; `docs/modules.md` lists them by concern, and `shuyi
//! add <module>` turns one on.
//!
//! - [`core`], [`assets`], [`ui`], [`platform`] and [`app`]: the default
//!   engine's crates, `shuyi-core` to `shuyi-app`, under short names;
//! - [`prelude`]: what a game on the short path names, with
//!   [`run_game`](prelude::run_game);
//! - [`enabled_modules`]: the [`Module`] of each module turned on, which
//!   the game installs once.

pub use shuyi_app as app;
pub use shuyi_assets as assets;
pub use shuyi_core as core;
pub use shuyi_platform as platform;
pub use shuyi_ui as ui;

use shuyi_ui::GameData;
use shuyi_ui::module::Module;

/// What a game on the short path names (ADR-0036): `shuyi-platform`'s
/// prelude, and [`run_game`](shuyi_app::run_game), which runs it from a
/// plain `main`.
pub mod prelude {
    pub use shuyi_app::run_game;
    pub use shuyi_platform::prelude::*;
}

/// The [`Module`] of each module whose feature is on, made with its
/// defaults, in the catalog's order: empty with no feature on. A game
/// installs them once, with [`Game::with_modules`](shuyi_ui::Game::with_modules)
/// or [`BasicGame::modules`](shuyi_platform::BasicGame::modules), so
/// turning a module on needs no change to its code. A module that needs
/// arguments is not here: the game installs it, as its README says.
pub fn enabled_modules<G: GameData>() -> Vec<Box<dyn Module<G>>> {
    // A module's ticket adds its line here, behind its feature:
    // `#[cfg(feature = "<name>")] modules.push(Box::new(...));`.
    Vec::new()
}

#[cfg(test)]
mod tests {
    use shuyi_ui::toy::{Toy, game};

    use super::enabled_modules;

    #[test]
    fn every_module_turned_on_installs_beside_the_others() {
        // With every feature on (CI's modules job), the engine's modules
        // have names and states of their own: none is a `ModuleError`.
        let modules = enabled_modules::<Toy>();
        let names: Vec<&str> = modules.iter().map(|m| m.name()).collect();
        let game = game()
            .with_modules(modules)
            .unwrap_or_else(|e| panic!("{e}"));
        let installed: Vec<&str> = game.modules().iter().map(|m| m.name).collect();
        assert_eq!(installed, names);
    }

    #[test]
    fn the_prelude_names_the_short_path_and_run_game() {
        use crate::prelude::{BasicGame, UiAction, run_game};
        let _: fn(BasicGame<UiAction>) = run_game::<UiAction, ()>;
    }
}
