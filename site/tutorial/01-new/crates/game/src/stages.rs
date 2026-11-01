//! The game's stages, in the order its template's parts add them. `shuyi
//! new` wrote this file from the parts it was made with: the title
//! ([`crate::title`]) starts the first, and a stage's own part of the
//! game's data is [`Own`].

use shuyi::prelude::*;

use crate::Data;

/// The stages' ids, in order: none yet.
pub fn names() -> &'static [&'static str] {
    &[]
}

/// The stages' own part of the game's data: none.
pub type Own = ();

/// Reads the stages' own part from the game's files.
pub fn own(_: &Bundle) -> Result<Own, Vec<ContentError>> {
    Ok(())
}

/// The first stage, which the title starts: none.
pub fn first(_: &mut Ctx<Data>) -> Option<Box<dyn Screen<Data>>> {
    None
}

/// Every screen the stages can show, for the check that the game's
/// controls fit each target: none.
pub fn screens(_: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    Vec::new()
}

/// The game's tools: the engine's, for a game on the short path.
#[cfg(feature = "tools")]
pub fn tools() -> shuyi_cli::BasicTools<UiAction, Own> {
    shuyi_cli::BasicTools::new(crate::game())
}
