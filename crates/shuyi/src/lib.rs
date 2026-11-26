//! Shuyi Engine, by its front door (ADR-0009 §6). A game depends on this
//! crate alone and turns on the features of the modules it wants, each
//! off by default; `docs/modules.md` lists them by concern, and `shuyi
//! add <module>` turns one on.
//!
//! - [`core`], [`assets`], [`ui`], [`platform`] and [`app`]: the default
//!   engine's crates, `shuyi-core` to `shuyi-app`, under short names
//!   (`app`, the macroquad backend, on every target but WASI, which has no
//!   window, and with its feature, on by default; a game on a device without
//!   `std` turns the default features off);
//! - [`prelude`]: what a game on the short path names, with
//!   [`run_game`](prelude::run_game);
//! - [`files!`]: a folder of the game's files, embedded in the program
//!   (with `std`);
//! - [`enabled_modules`]: the [`Module`] of each module turned on, which
//!   the game installs once.
//! - `script` (feature `script`): `shuyi-script`, rules from scripts,
//!   which the game's rules call.
//! - `net` (feature `net`): `shuyi-net`, the transport a game's online
//!   play talks through, and an in-memory network for its tests.
//! - `system-languages` (feature, on by default): the macroquad backend
//!   reads the system's preferred languages on a desktop (`sys-locale`),
//!   so a new player starts in the system's language when the game has
//!   it. Off, a desktop game starts in its source language until the
//!   player picks one; the web reads the browser's either way.
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(all(feature = "app", not(target_os = "wasi")))]
pub use shuyi_app as app;
pub use shuyi_assets as assets;
/// Embeds a folder of the game's files in the program: `shuyi-assets`'
/// [`files!`](shuyi_assets::files), which [`Layer::dir`](shuyi_assets::Layer::dir)
/// makes a layer of.
pub use shuyi_assets::files;
pub use shuyi_core as core;
#[cfg(feature = "net")]
pub use shuyi_net as net;
pub use shuyi_platform as platform;
#[cfg(feature = "script")]
pub use shuyi_script as script;
pub use shuyi_ui as ui;

use alloc::boxed::Box;
use alloc::vec::Vec;

use shuyi_ui::GameData;
use shuyi_ui::module::Module;

/// What a game on the short path names (ADR-0036): `shuyi-platform`'s
/// prelude, and [`run_game`](shuyi_app::run_game), which runs it from a
/// plain `main`.
pub mod prelude {
    #[cfg(all(feature = "app", not(target_os = "wasi")))]
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

/// The top-down grid guide, `docs/guides/top-down-grid.md`, whose code
/// runs as this crate's doc test. Only `cargo test --doc`
/// builds it.
#[cfg(doctest)]
#[doc = include_str!("../../../docs/guides/top-down-grid.md")]
pub struct TopDownGridGuide;

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
    #[cfg(all(feature = "app", not(target_os = "wasi")))]
    fn the_prelude_names_the_short_path_and_run_game() {
        use crate::prelude::{BasicGame, UiAction, run_game};
        let _: fn(BasicGame<UiAction>) = run_game::<UiAction, ()>;
    }
}
