//! Chapter 3, actions, never keys: the help line that names the player's
//! own keys, and an action set of a game's own, for a game that needs
//! more than the six moves of `UiAction`.

use shuyi::assets::keymap::Chord;
use shuyi::prelude::*;
use shuyi::ui::input::help_keys::key_name;

use crate::Data;

// ANCHOR: key_name
/// "Enter plants", or whichever key the player put on Confirm: text built
/// in code names a key through the player's keymap, never by its name.
pub fn plant_hint(ctx: &Ctx<Data>) -> String {
    let key = key_name(ctx.help_keys(), UiAction::Confirm);
    ctx.text_with("hint.plant", &[("key", &key)])
}
// ANCHOR_END: key_name

// ANCHOR: action_set
/// The actions of a garden game that needs more than the six moves: a
/// watering can, and a key to leave with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GardenAction {
    /// Walk left.
    Left,
    /// Walk right.
    Right,
    /// Plant a seed.
    Plant,
    /// Water the plot: optional, so a player may leave it with no key.
    Water,
    /// Leave the garden.
    Leave,
}

impl ActionSet for GardenAction {
    /// No two actions share a key.
    type Group = ();

    const ALL: &'static [Self] = &[
        Self::Left,
        Self::Right,
        Self::Plant,
        Self::Water,
        Self::Leave,
    ];

    /// Its name in the keymap file and the player's saved keys.
    fn name(self) -> &'static str {
        match self {
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Plant => "Plant",
            Self::Water => "Water",
            Self::Leave => "Leave",
        }
    }

    /// Walking repeats while the key is held.
    fn is_repeatable(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    /// Every action but watering must keep a key and a button.
    fn is_required(self) -> bool {
        self != Self::Water
    }

    fn is_rebindable(self) -> bool {
        true
    }

    fn group(self) {}

    /// What the engine's menus and screens take each for.
    fn ui(self) -> Option<UiAction> {
        match self {
            Self::Left => Some(UiAction::Left),
            Self::Right => Some(UiAction::Right),
            Self::Plant => Some(UiAction::Confirm),
            Self::Leave => Some(UiAction::Cancel),
            Self::Water => None,
        }
    }

    /// No key that the player can't move.
    fn fixed_keys() -> &'static [(Chord, Self)] {
        &[]
    }
}
// ANCHOR_END: action_set

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use shuyi::assets::keymap::KeymapDef;
    use shuyi::assets::{Bundle, Layer};

    use super::*;
    use crate::assets::Plan;
    use crate::game;

    #[test]
    fn the_hint_and_the_help_name_the_players_keys() {
        let h = game().harness();
        assert_eq!(plant_hint(h.ctx()), "Enter plants");
        assert_eq!(
            h.ctx().text_with("title.help", &[]),
            "Enter plant · Backspace leave"
        );
    }

    #[test]
    fn the_action_sets_keymap_file_reads_with_every_action() {
        let text = include_bytes!("../actions/keymap.ron").to_vec();
        let files = BTreeMap::from([("keymap.ron".to_owned(), text)]);
        let bundle = Bundle::new(vec![Layer::owned("actions".to_owned(), files).unwrap()]);
        let formats = BasicData::<GardenAction, Plan>::formats();
        let keymap = formats.load::<KeymapDef<GardenAction>>(&bundle, "keymap");
        assert!(keymap.is_ok(), "{keymap:?}");
        assert_eq!(GardenAction::from_name("Water"), Some(GardenAction::Water));
        assert_eq!(
            GardenAction::for_ui(UiAction::Cancel),
            Some(GardenAction::Leave)
        );
    }
}
