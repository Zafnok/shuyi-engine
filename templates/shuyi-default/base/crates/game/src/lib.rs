//! The game: its files in `files/`, embedded, which the engine reads as
//! its data ([`Data`]): the English text, the keys, the palette, the font
//! and its stages' own part. It opens on the title ([`title`]), which
//! starts its first stage ([`stages`]) if it has one. [`game`] says all
//! the engine needs to know: `src/main.rs` runs it, and
//! `examples/tools.rs` hands it to the engine's tools.

pub mod stages;
pub mod title;

use include_dir::{Dir, include_dir};
use shuyi::prelude::*;

/// The game's files, embedded.
// check-text: not player text, a folder
static FILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/files");

/// The game's data and state, which its screens share: the engine's, read
/// from `files/`, with the six moves of `files/keymap.ron` as its actions,
/// and its stages' own part.
pub type Data = BasicData<UiAction, stages::Own>;

/// The game's name.
// check-text: not player text, the window's title (`files/en.ron` has it too)
const NAME: &str = "{{name}}";

/// Where the player's settings and keys are kept: never change it, or
/// every player's are lost.
// check-text: not player text, a storage name
const STORAGE_NAME: &str = "{{crate_name}}";

/// The game: its name, where the player's settings and keys are kept, its
/// files and its stages' part of them, its first screen, every screen it
/// can show, and the engine's modules its features turn on (`Cargo.toml`).
pub fn game() -> BasicGame<UiAction, stages::Own> {
    // check-text: not player text, a folder
    let files = Layer::dir("files", &FILES);
    BasicGame::with_own(NAME, STORAGE_NAME, files, stages::own, |_| {
        Box::new(title::Title)
    })
    .screens(screens)
    .modules(shuyi::enabled_modules)
}

/// Every screen the game can show: the title and its stages'.
fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    let mut screens: Vec<Box<dyn Screen<Data>>> = vec![Box::new(title::Title)];
    screens.extend(stages::screens(ctx));
    screens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_load() {
        let data = game().data().unwrap_or_else(|e| panic!("{e:?}"));
        assert!(data.images.images.is_empty(), "{:?}", data.images.images);
        assert!(game().ctx().is_ok());
    }
}
