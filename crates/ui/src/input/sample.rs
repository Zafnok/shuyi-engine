//! The input mechanism over the sample game's actions and keymap file
//! (`shuyi_assets::keymap::sample`), for the tests.
use crate::prelude::*;
use shuyi_assets::ContentError;
use shuyi_assets::keymap::sample;

pub use sample::{Action, KeyGroup, LEFT_HANDED, RIGHT_HANDED};

/// The sample game's validated keymap.
pub type KeymapDef = shuyi_assets::keymap::KeymapDef<Action>;

/// The sample game's chord and button lookup.
pub type Keymap = super::keymap::Keymap<Action>;

/// The sample game's held keys and repeat.
pub type InputState = super::state::InputState<Action>;

/// One layout's player-edited keys.
pub type LayoutBindings = super::bindings::LayoutBindings<Action>;

/// The player-edited controller buttons.
pub type PadBindings = super::bindings::PadBindings<Action>;

/// Every layout's player keys and the buttons, as saved.
pub type PlayerKeys = super::bindings::PlayerKeys<Action>;

/// The sample keymap file, loaded like a game loads its own.
pub fn load_keymap() -> Result<KeymapDef, Vec<ContentError>> {
    KeymapDef::from_source("sample.ron", sample::SOURCE)
}
