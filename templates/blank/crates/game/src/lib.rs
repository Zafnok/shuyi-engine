//! The game: its files in `files/`, embedded, which the engine reads as
//! its data ([`Data`]): the English text, the keys, the palette and the
//! font. Its one screen is the title ([`title`]). [`game`] says all the
//! engine needs to know: `src/main.rs` runs it, and `examples/tools.rs`
//! hands it to the engine's tools.

pub mod title;

use include_dir::{Dir, include_dir};
use shuyi_platform::prelude::*;

/// The game's files, embedded.
static FILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/files");

/// The game's data and state, which its screens share: the engine's, read
/// from `files/`, with the six moves of `files/keymap.ron` as its actions.
pub type Data = BasicData<UiAction>;

/// The game: its name, where the player's settings and keys are kept
/// (never change it, or every player's are lost), its files and its first
/// screen.
pub fn game() -> BasicGame<UiAction> {
    BasicGame::new("{{name}}", "{{crate}}", Layer::dir("files", &FILES), |_| {
        Box::new(title::Title)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_load() {
        let data = game().data().unwrap_or_else(|e| panic!("{e:?}"));
        assert!(data.images.images.is_empty());
        assert!(game().ctx().is_ok());
    }
}
