//! The game's one stage: the puzzle. The title ([`crate::title`]) starts
//! it, and its part of the game's data is [`Own`].

use shuyi::assets::lighting::Lighting;
use shuyi::prelude::*;

use crate::Data;
use crate::board::Board;
use crate::level::{self, Level};

/// The stages' ids, in order.
pub fn names() -> &'static [&'static str] {
    &["board"]
}

// ANCHOR: own
/// The game's own part of its data.
#[derive(Debug)]
pub struct Own {
    /// The level.
    pub level: Level,
    /// The lights a level may be in: none, so the default, which changes
    /// nothing.
    pub lights: Lighting,
}

/// Reads the game's own part from its files.
pub fn own(bundle: &Bundle) -> Result<Own, Vec<ContentError>> {
    Ok(Own {
        // check-text: not player text, a file name
        level: level::read(bundle, "levels/1.map")?,
        lights: Lighting::default(),
    })
}
// ANCHOR_END: own

/// What the title starts: the board.
pub fn first(ctx: &mut Ctx<Data>) -> Option<Box<dyn Screen<Data>>> {
    Some(Box::new(Board::new(ctx)))
}

/// Every screen the game can show past the title, for the check that its
/// controls fit each target.
pub fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    vec![Box::new(Board::new(ctx))]
}

/// The game's tools: the engine's, for a game on the short path.
#[cfg(feature = "tools")]
pub fn tools() -> shuyi_cli::BasicTools<crate::action::Action, Own> {
    shuyi_cli::BasicTools::new(crate::game())
}
