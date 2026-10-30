//! Chapter 1, the quick start: Moon Garden on the short path, as a game
//! made with `shuyi new` has it. In a new game `game` and `Data` are in
//! `crates/game/src/lib.rs`, `APP` in `crates/game/src/app_info.rs`,
//! `main` is `src/main.rs` and `tool_program` is `examples/tools.rs`.

use std::process::ExitCode;

use include_dir::{Dir, include_dir};
use shuyi::prelude::*;
use shuyi_cli::BasicTools;

use crate::assets::{Plan, read_plan};
use crate::screens::{GardenScreen, Leave, Title};

// ANCHOR: files
/// The game's files, embedded: its English, its keys, its palette, its
/// font and its own part.
// check-text: not player text, a folder
static FILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/files");
// ANCHOR_END: files

// ANCHOR: data
/// The game's data, which its screens share: the engine's part read from
/// `files/`, with the six moves of `UiAction` as its actions, and the
/// game's own part, its garden's [`Plan`].
pub type Data = BasicData<UiAction, Plan>;
// ANCHOR_END: data

// ANCHOR: app
/// Who the game is: its name, its id (the player's settings are kept under
/// it) and its version. `cargo shuyi app-info` writes it from `shuyi.ron`'s
/// `app`.
// check-text: not player text, who the game is
#[rustfmt::skip]
pub const APP: AppInfo = AppInfo::new("Moon Garden", "com.example.moon-garden", "0.1.0");
// ANCHOR_END: app

// ANCHOR: game
/// The game: who it is, its files, how its own part is read from them,
/// its first screen, every screen it can show, and the engine's modules
/// its features turn on.
pub fn game() -> BasicGame<UiAction, Plan> {
    // check-text: not player text, a folder
    let files = Layer::dir("files", &FILES);
    BasicGame::with_own(APP, files, read_plan, |_| Box::new(Title))
        .screens(every_screen)
        .modules(shuyi::enabled_modules)
}

/// Every screen the game can show, for the check that its controls fit
/// each target.
fn every_screen(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    vec![
        Box::new(Title),
        Box::new(GardenScreen::new(&ctx.game.own)),
        Box::new(Leave),
    ]
}
// ANCHOR_END: game

// ANCHOR: main
/// The game's binary: the engine's macroquad backend runs it in a window.
pub fn main() {
    run_game(game());
}
// ANCHOR_END: main

// ANCHOR: tools
/// The game's tool program: `cargo shuyi <command>` runs the engine's
/// tools in it, with the game's own, here the engine's for a game on the
/// short path.
pub fn tool_program() -> ExitCode {
    let tools = BasicTools::new(game());
    ExitCode::from(shuyi_cli::main_with(std::env::args().skip(1), Some(&tools)))
}
// ANCHOR_END: tools

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_load_and_the_game_starts_on_its_title() {
        let data = game().data().unwrap_or_else(|e| panic!("{e:?}"));
        assert_eq!(data.own, Plan::new(6, 9, 75));
        let h = game().harness();
        assert_eq!(h.screens(), ["title"]);
        let mut ctx = game().ctx().unwrap();
        let names: Vec<&str> = every_screen(&mut ctx).iter().map(|s| s.name()).collect();
        assert_eq!(names, ["title", "garden", "leave"]);
    }
}
