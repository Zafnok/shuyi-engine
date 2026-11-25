//! The engine's screens and widgets in the toy game, through the Harness
//! (ticket 1112): its lobby is an engine menu; from it the options screen
//! with the toy's rows (a setting kept on leaving, a volume typed in, the
//! layout switched through the layout picker, the language switched and
//! kept or put back), the Key bindings screen
//! with the toy's rows, and a scene read in the engine's text box.

use insta::assert_snapshot;
use shuyi_assets::target::TargetProfile;
use shuyi_platform::harness::Harness;
use shuyi_ui::audio::groups::SFX;
use shuyi_ui::screens::options::{
    FULLSCREEN_ROW, LANGUAGE_ROW, LeaveChoice, OptionsScreen, ValueView,
};
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

/// The engine's profile `id`, from `targets/`.
fn engine_profile(id: &str) -> TargetProfile {
    let source = match id {
        "desktop" => include_str!("../../../../targets/desktop.ron"),
        "web" => include_str!("../../../../targets/web.ron"),
        "steam-deck" => include_str!("../../../../targets/steam-deck.ron"),
        "playdate" => include_str!("../../../../targets/playdate.ron"),
        _ => panic!("no profile {id}"),
    };
    TargetProfile::from_source(id, id, source).unwrap_or_else(|e| panic!("{e:?}"))
}

/// A device that is always full screen shows no Fullscreen row; a window
/// and a browser's canvas do.
#[test]
fn the_options_screen_offers_fullscreen_only_in_a_window() {
    for (id, windowed) in [
        ("desktop", true),
        ("web", true),
        ("steam-deck", false),
        ("playdate", false),
    ] {
        let mut h = lobby().as_target(&engine_profile(id));
        h.keys("f");
        assert_eq!(h.screens(), ["lobby", "options"], "{id}");
        let view = options(&h).view(h.ctx());
        let shown = view.row(FULLSCREEN_ROW).is_some();
        assert_eq!(shown, windowed, "{id}");
    }
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

/// The options screen focused on the Language row, nine rows down.
fn on_the_language_row() -> Harness<Toy> {
    let mut h = lobby();
    h.keys("f Down Down Down Down Down Down Down Down Down");
    assert_eq!(options(&h).focus(h.ctx()), LANGUAGE_ROW);
    h
}

/// The language row's value, as the options screen shows it.
fn language_shown(h: &Harness<Toy>) -> Option<String> {
    let view = options(h).view(h.ctx());
    let row = view.row(LANGUAGE_ROW)?;
    row.value.text().map(str::to_owned)
}

#[test]
fn the_language_row_switches_every_screens_text_at_once_and_keeps_it() {
    let mut h = on_the_language_row();
    assert_eq!(language_shown(&h).as_deref(), Some("English"));
    assert_eq!(h.ctx().text("home.greeting"), "Hello, {name}");
    h.keys("Right");
    assert_eq!(language_shown(&h).as_deref(), Some("Debug"));
    assert_eq!(h.ctx().text("home.greeting"), "HELLO, {name}");
    assert_snapshot!("options_with_the_language_row", h.snapshot());
    // Kept: the next launch from the same storage starts in it.
    h.keys("d Left Left f");
    assert_eq!(h.screens(), ["lobby"]);
    let h = relaunch(h);
    assert_eq!(h.ctx().text("home.greeting"), "HELLO, {name}");
    let picked = h.ctx().settings().language.clone();
    assert_eq!(
        picked.as_ref().map(shuyi_assets::lang::LangCode::as_str),
        Some("dbg")
    );
}

#[test]
fn a_discarded_language_is_put_back() {
    let mut h = on_the_language_row();
    h.keys("Right");
    assert_eq!(h.ctx().text("home.greeting"), "HELLO, {name}");
    h.keys("d Left f");
    assert_eq!(h.screens(), ["lobby"]);
    assert_eq!(h.ctx().text("home.greeting"), "Hello, {name}");
    assert_eq!(h.ctx().settings().language, None);
}

#[test]
fn a_volume_is_typed_in_and_a_discarded_change_is_put_back() {
    let mut h = lobby();
    h.keys("f Down Down Down Down Down Down f");
    assert!(options(&h).typing().is_some());
    h.type_text("37").keys("Enter");
    assert!(options(&h).typing().is_none());
    assert_eq!(h.ctx().volume(SFX), 37);
    // Thrown away on leaving: as it was.
    h.keys("d Left f");
    assert_eq!(h.screens(), ["lobby"]);
    assert_eq!(h.ctx().volume(SFX), 97);
    assert!(h.ctx().settings().volumes.is_empty());
}

#[test]
fn the_options_screen_switches_the_layout_through_the_picker() {
    let mut h = lobby();
    h.keys("f Down Down Down Down Down Down Down");
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
    assert_eq!(waiting.map(|w| w.key.plain()).as_deref(), Some("f"));
    // Confirm goes on; the next line types from its start.
    h.keys("f");
    let (text, player) = talk(&h).unwrap();
    assert_eq!(player.current().text.as_deref(), Some("Welcome in."));
    assert!(!text.is_revealed());
    // Confirm reveals, then ends the scene.
    h.keys("f f");
    assert_eq!(h.screens(), ["lobby"]);
}
