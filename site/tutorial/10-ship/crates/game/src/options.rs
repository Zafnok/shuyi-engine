//! The options: the engine's screen, with the rows the game wants, and the
//! engine's Key bindings screen, where the player rebinds every action.

use shuyi::prelude::*;
use shuyi::ui::screens::key_bindings::{KeyBindingsScreen, KeyRow, RowGroup};
use shuyi::ui::screens::options::{self, OptionRow, RowAction, RowValue};

use crate::Data;
use crate::action::Action;

// ANCHOR: rows
/// The Key bindings screen's rows: every action must keep a key.
pub const KEY_ROWS: [KeyRow<Action>; 7] = [
    (Action::Up, "key_bindings.action.up", RowGroup::Required),
    (Action::Down, "key_bindings.action.down", RowGroup::Required),
    (Action::Left, "key_bindings.action.left", RowGroup::Required),
    (
        Action::Right,
        "key_bindings.action.right",
        RowGroup::Required,
    ),
    (
        Action::Confirm,
        "key_bindings.action.confirm",
        RowGroup::Required,
    ),
    (
        Action::Cancel,
        "key_bindings.action.cancel",
        RowGroup::Required,
    ),
    (Action::Undo, "key_bindings.action.undo", RowGroup::Required),
];

/// The options' rows: the frame rate, the keys, and Restore defaults.
pub fn option_rows(_: &Ctx<Data>) -> Vec<OptionRow<Data>> {
    vec![
        OptionRow::frame_cap(),
        OptionRow::new(
            "options.row.key_bindings",
            RowValue::action(
                |_| None,
                |ctx| RowAction::Open(Box::new(KeyBindingsScreen::new(ctx, &KEY_ROWS))),
                |_| options::HELP_OPEN,
            ),
        )
        .starting_group(),
        OptionRow::restore_defaults().starting_group(),
    ]
}
// ANCHOR_END: rows
