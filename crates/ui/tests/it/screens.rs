//! The engine's screens and widgets in the toy game, through the Harness
//! (ticket 1112): its lobby is an engine menu; from it the options screen
//! with the toy's rows (a setting kept on leaving, a volume typed in, the
//! layout switched through the layout picker), the Key bindings screen
//! with the toy's rows, and a scene read in the engine's text box.

use shuyi_ui::harness::Harness;
use shuyi_ui::screens::options::{LeaveChoice, OptionsScreen, ValueView};
use shuyi_ui::settings::{SETTINGS_KEY, TextSpeed};
use shuyi_ui::storage::Storage;
use shuyi_ui::toy::{self, LEFT_HANDED, Lobby, Talk, Toy};

/// The toy game at its lobby, the right-handed layout chosen.
fn lobby() -> Harness<Toy> {
    Harness::new(toy::lobby())
}

/// The next launch, from `h`'s storage, at the lobby.
fn relaunch(h: Harness<Toy>) -> Harness<Toy> {
    let ctx = toy::launch(h.into_storage());
    Harness::new(shuyi_ui::Game::new(
        ctx,
        Box::new(Lobby::new()),
        toy::hooks(),
    ))
}

fn options(h: &Harness<Toy>) -> &OptionsScreen<Toy> {
    h.screen::<OptionsScreen<Toy>>()
        .unwrap_or_else(|| panic!("no options: {:?}", h.screens()))
}

#[test]
fn the_lobby_is_an_engine_menu_with_its_sounds() {
    let mut h = lobby();
    assert_eq!(h.screens(), ["lobby"]);
    h.keys("Down Down Up");
    assert_eq!(h.sounds(), ["ui_move"; 3]);
    let focus = |h: &Harness<Toy>| h.screen::<Lobby>().map(|l| l.menu.focus());
    assert_eq!(focus(&h), Some(1));
    // Up from the first wraps to the last; nothing cancels it.
    h.keys("Up Up d");
    assert_eq!(focus(&h), Some(3));
    assert_eq!(h.screens(), ["lobby"]);
    assert!(!h.quit_requested());
    h.keys("f");
    assert!(h.quit_requested());
    assert!(
        h.snapshot().contains("│ Key bindings │"),
        "{}",
        h.snapshot()
    );
}

#[test]
fn a_setting_changed_on_the_options_screen_is_kept_on_leaving_and_saved() {
    let mut h = lobby();
    h.keys("f");
    assert_eq!(h.screens(), ["lobby", "options"]);
    h.clear_audio();
    h.keys("Right");
    assert_eq!(h.ctx().settings().text_speed, TextSpeed::Fast);
    assert_eq!(h.sounds(), ["ui_move"]);
    // Not saved yet: leaving asks.
    assert_eq!(h.ctx().storage.read(SETTINGS_KEY), Ok(None));
    h.keys("d");
    assert_eq!(options(&h).leaving(), Some(LeaveChoice::Back));
    h.keys("Left Left f");
    assert_eq!(h.screens(), ["lobby"]);
    let h = relaunch(h);
    assert_eq!(h.ctx().settings().text_speed, TextSpeed::Fast);
}

#[test]
fn a_volume_is_typed_in_and_a_discarded_change_is_put_back() {
    let mut h = lobby();
    h.keys("f Down Down Down Down Down f");
    assert!(options(&h).typing().is_some());
    h.type_text("37").keys("Enter");
    assert!(options(&h).typing().is_none());
    assert_eq!(h.ctx().settings().sound_volume, 37);
    // Thrown away on leaving: as it was.
    h.keys("d Left f");
    assert_eq!(h.screens(), ["lobby"]);
    assert_eq!(h.ctx().settings().sound_volume, 80);
}

#[test]
fn the_options_screen_switches_the_layout_through_the_picker() {
    let mut h = lobby();
    h.keys("f Down Down Down Down Down Down");
    assert_eq!(options(&h).focus(h.ctx()), "options.row.layout");
    h.keys("f");
    assert_eq!(h.screens(), ["lobby", "options", "layout_picker"]);
    h.keys("Down f");
    assert_eq!(h.screens(), ["lobby", "options"]);
    assert_eq!(h.ctx().layout(), Some(LEFT_HANDED));
    let view = options(&h).view(h.ctx());
    let layout = view.row("options.row.layout").map(|r| r.value.clone());
    assert_eq!(
        layout.as_ref().and_then(ValueView::text),
        Some("Left-handed")
    );
    // The left hand's keys now: `k` backs out, with nothing to keep.
    h.keys("k");
    assert_eq!(h.screens(), ["lobby"]);
}

#[test]
fn a_key_rebound_on_the_key_bindings_screen_works_once_it_is_left() {
    let mut h = lobby();
    h.keys("Down f");
    assert_eq!(h.screens(), ["lobby", "key_bindings"]);
    // Confirm's first slot, the fifth row: capture, then `g`.
    h.keys("Down Down Down Down f g");
    assert_eq!(h.screens(), ["lobby", "key_bindings"]);
    h.keys("d");
    assert_eq!(h.screens(), ["lobby"]);
    // `f` no longer confirms; `g` does.
    h.keys("f");
    assert_eq!(h.screens(), ["lobby"]);
    h.keys("g");
    assert_eq!(h.screens(), ["lobby", "key_bindings"]);
}

#[test]
fn a_scene_is_read_in_the_engines_text_box() {
    let mut h = lobby();
    h.keys("Down Down f");
    assert_eq!(h.screens(), ["lobby", "talk"]);
    let talk = |h: &Harness<Toy>| {
        h.screen::<Talk>()
            .map(|t| (t.text.clone(), t.player.clone()))
    };
    // It types out at the player's text speed.
    let (text, _) = talk(&h).unwrap();
    assert!(!text.is_revealed());
    h.wait(5.0);
    let (text, player) = talk(&h).unwrap();
    assert!(text.is_revealed());
    assert_eq!(player.current().text.as_deref(), Some("The hall is quiet."));
    let snapshot = h.snapshot();
    assert!(snapshot.contains("The hall is quiet."), "{snapshot}");
    // It waits for the key that goes on (its marker blinks).
    let waiting = h.screen::<Talk>().and_then(|t| t.view(h.ctx()).waiting);
    assert_eq!(waiting.map(|w| w.key).as_deref(), Some("f"));
    // Confirm goes on; the next line types from its start.
    h.keys("f");
    let (text, player) = talk(&h).unwrap();
    assert_eq!(player.current().text.as_deref(), Some("Welcome in."));
    assert!(!text.is_revealed());
    // Confirm reveals, then ends the scene.
    h.keys("f f");
    assert_eq!(h.screens(), ["lobby"]);
}
