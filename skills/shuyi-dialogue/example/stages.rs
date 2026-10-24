//! The game's stages, in order: the title's Confirm starts the first. The
//! talk ([`talk`]) is the game's one stage, a scene of `files/talk.dlg`
//! read in the engine's text box, and its part of the game's data is
//! [`Own`].

pub mod talk;

use shuyi::prelude::*;

use crate::Data;

/// The stages' ids, in order.
pub fn names() -> &'static [&'static str] {
    &["talk"]
}

/// The stages' own part of the game's data.
pub type Own = talk::Own;

/// Reads the stages' own part from the game's files.
pub fn own(bundle: &Bundle) -> Result<Own, Vec<ContentError>> {
    talk::own(bundle)
}

/// The first stage, which the title starts.
pub fn first(ctx: &mut Ctx<Data>) -> Option<Box<dyn Screen<Data>>> {
    Some(talk::first(ctx))
}

/// Every screen the stages can show, for the check that the game's
/// controls fit each target.
pub fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    talk::screens(ctx)
}

/// The game's tools: the engine's, for a game on the short path.
#[cfg(feature = "tools")]
pub fn tools() -> shuyi_cli::BasicTools<UiAction, Own> {
    shuyi_cli::BasicTools::new(crate::game())
}
