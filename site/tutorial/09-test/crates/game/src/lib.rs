//! The game: its files in `files/`, embedded, which the engine reads as
//! its data ([`Data`]): the English text, the keys, the palette, the font
//! and its stages' own part. It opens on the title ([`title`]), which
//! starts its first stage ([`stages`]) if it has one. [`game`] says all
//! the engine needs to know: `src/main.rs` runs it, and
//! `examples/tools.rs` hands it to the engine's tools. Who the game is,
//! its name, id and version, is `shuyi.ron`'s `app`, which
//! `cargo shuyi app-info` writes into [`app_info`].

pub mod action;
pub mod app_info;
pub mod board;
pub mod level;
pub mod levels;
pub mod options;
pub mod progress;
pub mod rules;
pub mod stages;
pub mod title;
#[cfg(feature = "tools")]
pub mod tools;

use std::sync::OnceLock;

use include_dir::{Dir, include_dir};
use shuyi::core::Made;
use shuyi::prelude::*;

use crate::action::Action;

/// The game's files, embedded.
// check-text: not player text, a folder
static FILES: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/files");

/// The game's data and state, which its screens share: the engine's, read
/// from `files/`, with the six moves of `files/keymap.ron` as its actions,
/// and its stages' own part.
pub type Data = BasicData<Action, stages::Own>;

/// The game: who it is ([`app_info::APP`]: its name, and where the
/// player's settings and keys are kept), its files and its stages' part of
/// them, its first screen, every screen it can show, and the engine's
/// modules its features turn on (`Cargo.toml`).
pub fn game() -> BasicGame<Action, stages::Own> {
    // check-text: not player text, a folder
    let files = Layer::dir("files", &FILES);
    BasicGame::with_own(app_info::APP, files, stages::own, |_| {
        Box::new(title::Title)
    })
    .screens(screens)
    .modules(shuyi::enabled_modules)
}

/// What made this build, for its records and saves: the game's id and
/// version, and the fingerprint of the files it ships, computed once. A
/// record or a save keeps it in a `made` field, checked on reading.
pub fn made() -> Made {
    static MADE: OnceLock<Made> = OnceLock::new();
    MADE.get_or_init(|| {
        // check-text: not player text, the build's name in its records
        let build = format!("{} {}", app_info::APP.id, app_info::APP.version);
        Made::new(build, game().bundle().fingerprint())
    })
    .clone()
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

    #[test]
    fn the_build_is_stamped_with_its_files() {
        let made = made();
        assert_eq!(made.content, game().bundle().fingerprint());
        assert_eq!(made, Made::new(made.game.clone(), made.content));
        assert!(made.game.starts_with(app_info::APP.id), "{made:?}");
        assert!(made.game.ends_with(app_info::APP.version), "{made:?}");
    }
}
