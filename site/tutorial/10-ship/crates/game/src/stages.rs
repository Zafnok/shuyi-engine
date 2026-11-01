//! The game's one stage: the puzzle. The title ([`crate::title`]) starts
//! it, and its part of the game's data is [`Own`].

use shuyi::assets::lighting::Lighting;
use shuyi::prelude::*;
use shuyi::ui::screens::key_bindings::KeyBindingsScreen;
use shuyi::ui::screens::options::OptionsScreen;

use crate::Data;
use crate::board::Board;
use crate::level::{self, Level};
use crate::levels::LevelMenu;
use crate::options::{KEY_ROWS, option_rows};

/// The stages' ids, in order.
pub fn names() -> &'static [&'static str] {
    &["board"]
}

// ANCHOR: own
/// The game's own part of its data.
#[derive(Debug)]
pub struct Own {
    /// Every level, in the order they are played.
    pub levels: Vec<Level>,
    /// The lights a level may be in: none, so the default, which changes
    /// nothing.
    pub lights: Lighting,
}

/// Reads the game's own part from its files.
pub fn own(bundle: &Bundle) -> Result<Own, Vec<ContentError>> {
    Ok(Own {
        levels: level::read_all(bundle)?,
        lights: Lighting::default(),
    })
}
// ANCHOR_END: own

/// What the title starts: the level menu.
pub fn first(ctx: &mut Ctx<Data>) -> Option<Box<dyn Screen<Data>>> {
    Some(Box::new(LevelMenu::new(ctx)))
}

/// Every screen the game can show past the title, for the check that its
/// controls fit each target.
pub fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    vec![
        Box::new(LevelMenu::new(ctx)),
        Box::new(Board::new(ctx, 0)),
        Box::new(OptionsScreen::new(option_rows)),
        Box::new(KeyBindingsScreen::new(ctx, &KEY_ROWS)),
    ]
}

/// The game's tools: the engine's, and its scene, bot and replay on top.
#[cfg(feature = "tools")]
pub fn tools() -> crate::tools::Tools {
    crate::tools::Tools::new(crate::game())
}
