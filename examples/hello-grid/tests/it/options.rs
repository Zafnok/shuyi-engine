//! The engine's options screen with the game's rows, opened from the
//! title: the map's look kept on leaving (and saved), and a key rebound on
//! the engine's Key bindings screen working once it is left.

use hello_grid::screens::StageScreen;
use hello_grid::tools::Start;
use hello_grid::{Hello, MapLook};
use shuyi_platform::harness::Harness;
use shuyi_ui::screens::options::{LeaveChoice, OptionsScreen};
use shuyi_ui::settings::{SETTINGS_KEY, TextSpeed};
use shuyi_ui::storage::Storage;

use crate::title;

fn options(h: &Harness<Hello>) -> &OptionsScreen<Hello> {
    h.screen::<OptionsScreen<Hello>>()
        .unwrap_or_else(|| panic!("no options: {:?}", h.screens()))
}

#[test]
fn the_options_have_the_two_settings_key_bindings_and_restore() {
    let mut h = title(0);
    h.keys("Down Enter");
    assert_eq!(h.screens(), ["title", "options"]);
    let keys: Vec<&str> = options(&h)
        .rows(h.ctx())
        .iter()
        .map(|row| row.key)
        .collect();
    assert_eq!(
        keys,
        [
            "options.row.text_speed",
            "options.row.look",
            "options.row.key_bindings",
            "options.row.restore_defaults"
        ]
    );
    let snapshot = h.snapshot();
    for label in ["Text speed", "Map look", "Glyphs", "Key bindings"] {
        assert!(snapshot.contains(label), "{label}: {snapshot}");
    }
}

#[test]
fn the_map_look_is_kept_on_leaving_saved_and_drawn_with() {
    let mut h = title(0);
    h.keys("Down Enter Down Right");
    assert_eq!(h.ctx().settings().game.look, MapLook::Tiles);
    assert!(h.snapshot().contains("Tiles"));
    // Not saved yet: leaving asks; Keep keeps it.
    assert_eq!(h.ctx().storage.read(SETTINGS_KEY), Ok(None));
    h.keys("Backspace");
    assert_eq!(options(&h).leaving(), Some(LeaveChoice::Back));
    h.keys("Left Left Enter");
    assert_eq!(h.screens(), ["title"]);
    let saved = h.ctx().storage.read(SETTINGS_KEY).unwrap().unwrap();
    assert!(saved.contains("look:Tiles"), "{saved}");
    // The stage is drawn with the sprite skin's 24 px tiles at their own
    // size: 16 × 14 of them fit, where the glyph skin fits 24 × 22.
    assert_eq!(StageScreen::view_tiles(h.ctx()), (16, 14));
    assert_eq!(StageScreen::view_tiles(title(0).ctx()), (24, 22));
    // With the tiles, the pieces are sprites of the tileset.
    h.keys("Up Enter Backspace");
    assert_eq!(h.screens(), ["title", "stage"]);
    assert!(h.snapshot().contains("tiles.png"), "{}", h.snapshot());
}

#[test]
fn the_text_speed_is_the_engines_own_row() {
    let mut h = title(0);
    h.keys("Down Enter Right");
    assert_eq!(h.ctx().settings().text_speed, TextSpeed::Fast);
    // Discarded on leaving: as it was.
    h.keys("Backspace Left Enter");
    assert_eq!(h.screens(), ["title"]);
    assert_eq!(h.ctx().settings().text_speed, TextSpeed::Normal);
}

#[test]
fn a_key_rebound_on_the_key_bindings_screen_works_once_it_is_left() {
    let mut h = title(0);
    h.keys("Down Enter Down Down Enter");
    assert_eq!(h.screens(), ["title", "options", "key_bindings"]);
    // Confirm's first key, the fifth row: capture, then `c`.
    h.keys("Down Down Down Down Enter c");
    assert_eq!(h.screens(), ["title", "options", "key_bindings"]);
    h.keys("Backspace Backspace");
    assert_eq!(h.screens(), ["title"]);
    // Enter no longer confirms; `c` does, and the help says so.
    assert!(
        h.snapshot().contains("arrows choose · c pick"),
        "{}",
        h.snapshot()
    );
    h.keys("Up Enter");
    assert_eq!(h.screens(), ["title"]);
    h.keys("c");
    assert_eq!(h.screens(), ["title", "stage", "talk"]);
}

#[test]
fn the_tools_start_on_the_stage_in_either_look() {
    let h = Start {
        stage: true,
        look: MapLook::Tiles,
        seed: 4,
    }
    .harness();
    assert_eq!(h.screens(), ["stage"]);
    assert_eq!(h.ctx().settings().game.look, MapLook::Tiles);
    assert_eq!(h.ctx().game.seed, 5);
}
