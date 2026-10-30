//! The backend's whole loop on the fake SDK, with the toy game.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::sync::OnceLock;

use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageTable;
use shuyi_assets::keymap::Button;
use shuyi_assets::onebit::PatternMapDef;
use shuyi_platform::{Card, Look, OneBitLook, Platform, Setup, Start, SystemEvent};
use shuyi_ui::frame::onebit::{Font, PatternMap, ROW_BYTES};
use shuyi_ui::frame_cap::{CapStatus, FrameCap};
use shuyi_ui::input::help_keys::key_name;
use shuyi_ui::input::pad::PadKind;
use shuyi_ui::input::state::Device;
use shuyi_ui::log::{LogCategory, LogLevel};
use shuyi_ui::toy::{self, Action, Toy};
use shuyi_ui::{Game, Keyboard};

use super::*;
use crate::fake::FakeSdk;

/// What the toy's frames are drawn with: a font whose only glyph is its
/// fallback, solid, and no pictures, so a cell's glyph is a solid block.
struct Drawn {
    atlas: &'static FontAtlasDef,
    font: Font<'static>,
    map: PatternMap,
    images: ImageTable,
}

fn drawn() -> &'static Drawn {
    static DRAWN: OnceLock<Drawn> = OnceLock::new();
    DRAWN.get_or_init(|| {
        let atlas: &'static FontAtlasDef = Box::leak(Box::new(FontAtlasDef {
            cell_w: 8,
            cell_h: 16,
            columns: 16,
            glyphs: BTreeMap::from([('?', 0)]),
        }));
        Drawn {
            atlas,
            font: Font::new(atlas, &[0xFF; 16]).unwrap_or_else(|e| panic!("{e}")),
            map: PatternMap::new([], &PatternMapDef::default()),
            images: ImageTable::new(BTreeMap::new()),
        }
    })
}

fn look(_: &Toy) -> Look<'_> {
    let drawn = drawn();
    Look::new(drawn.atlas, &drawn.images).with_one_bit(OneBitLook::new(
        &drawn.map,
        &drawn.font,
        &(),
    ))
}

/// The toy at its home screen, as a game's load makes it on `setup`.
fn load(setup: Setup) -> Game<Toy> {
    toy::game_on(setup.ctx(toy::ctx()))
}

fn start() -> Start<Toy> {
    Start::new(Card::text("Toy"), look, |setup| Ok(load(setup)))
}

/// The fake and a driver on it, the game loaded: two frames.
fn running() -> (FakeSdk, Driver<Toy, FakeSdk>) {
    let sdk = FakeSdk::default();
    let mut driver = Driver::new(start(), sdk.clone());
    driver.update();
    driver.update();
    assert!(driver.app().game().is_some(), "{:?}", driver.app().errors());
    (sdk, driver)
}

#[test]
fn the_display_runs_at_the_profiles_rate() {
    let (sdk, _driver) = running();
    assert_eq!(sdk.refresh_rate(), Some(30));
}

#[test]
fn the_loading_card_is_the_sdks_text_and_then_the_game_draws() {
    let sdk = FakeSdk::default();
    let mut driver = Driver::new(start(), sdk.clone());
    assert!(driver.update(), "the card changes the screen");
    assert_eq!(sdk.take_texts(), [("Toy".to_string(), false)]);
    assert!(driver.update(), "the game's first frame changes it");
    assert_eq!(sdk.take_texts(), [], "no text once the game runs");
}

#[test]
fn a_failed_load_shows_why_once() {
    let sdk = FakeSdk::default();
    let start = Start::new(Card::text("Toy"), look, |_: Setup| {
        Err("no such file".to_string())
    });
    let mut driver = Driver::new(start, sdk.clone());
    driver.update();
    sdk.take_texts();
    assert!(driver.update());
    assert_eq!(sdk.take_texts(), [("no such file".to_string(), true)]);
    assert!(!driver.update(), "the same text is not drawn again");
    assert_eq!(sdk.take_texts(), []);
}

#[test]
fn a_game_with_no_one_bit_look_says_so() {
    fn plain(_: &Toy) -> Look<'_> {
        Look::new(drawn().atlas, &drawn().images)
    }
    let sdk = FakeSdk::default();
    let start = Start::new(Card::text("Toy"), plain, |setup| Ok(load(setup)));
    let mut driver = Driver::new(start, sdk.clone());
    driver.update();
    driver.update();
    let texts = sdk.take_texts();
    assert!(texts[1].0.contains("no one-bit look"), "{texts:?}");
}

/// The ticket: only the rows that changed are marked. The home screen draws
/// one cell in the first row of cells; drawing a second row of cells marks
/// those 16 pixel rows and no others.
#[test]
fn only_changed_rows_are_marked_and_copied() {
    let (sdk, mut driver) = running();
    let marked = sdk.take_marked();
    assert_eq!(marked.len(), 1, "one run, the screen's first draw: {marked:?}");
    assert!(!driver.update(), "nothing changed");
    assert_eq!(sdk.take_marked(), []);
    let before = sdk.frame();

    // The menu draws `M` in the top row of cells: 16 rows, no more.
    press(&mut driver, Action::Confirm);
    let top = driver.app().game().and_then(Game::top_screen);
    assert_eq!(top, Some("menu"));
    let marked = sdk.take_marked();
    assert_ne!(marked, [] as [(i32, i32); 0]);
    assert!(marked.iter().all(|&(_, last)| last < 16), "{marked:?}");
    let after = sdk.frame();
    for y in 0..240 {
        let start = y * ROW_BYTES;
        let differs = before[start..start + ROW_BYTES] != after[start..start + ROW_BYTES];
        let in_mark = marked
            .iter()
            .any(|&(first, last)| (first..=last).contains(&i32::try_from(y).unwrap_or(-1)));
        assert_eq!(differs, in_mark, "row {y}, marked {marked:?}");
    }
}

/// Runs a frame with `action` pressed by its default button.
fn press(driver: &mut Driver<Toy, FakeSdk>, action: Action) {
    driver
        .app
        .game_mut()
        .unwrap_or_else(|| panic!("the game has loaded"))
        .act(action);
    driver.update();
}

#[test]
fn a_frame_buffer_a_system_menu_drew_over_is_drawn_again_on_resume() {
    let (sdk, mut driver) = running();
    let screen = sdk.frame();
    driver.event(SdkEvent::Pause);
    sdk.scribble_on_frame();
    driver.event(SdkEvent::Resume);
    assert!(driver.update(), "the rows come back");
    assert_eq!(sdk.frame(), screen);
    let marked = sdk.take_marked();
    assert_eq!(marked.last(), Some(&(0, 239)), "{marked:?}");
}

/// The ticket: a game starts on the Playdate's pad, so its help names the
/// Playdate's buttons from the first frame, before any press.
#[test]
fn the_first_frames_help_names_the_playdates_buttons() {
    let (_sdk, driver) = running();
    let game = driver.app().game().unwrap_or_else(|| panic!("loaded"));
    let ctx = game.ctx();
    assert_eq!(ctx.device, Device::Pad(PadKind::Playdate));
    assert_eq!(game.input().device(), Device::Pad(PadKind::Playdate));
    let chord = game.input().keymap().buttons_for(Action::Confirm)[0];
    let name = key_name(ctx.help_keys(), Action::Confirm);
    assert_eq!(name, PadKind::Playdate.chord_name(chord, "+"));
    assert_ne!(
        name,
        key_name(ctx.help_keys().on_keyboard(), Action::Confirm),
        "not the keyboard's"
    );
}

#[test]
fn the_facts_are_the_profiles() {
    let platform = Playdate::new(FakeSdk::default());
    let facts = platform.facts();
    assert_eq!(facts.target, "playdate");
    assert!(!facts.can_quit() && !facts.windowed);
    assert!(!facts.pointer && !facts.touch);
    assert_eq!(facts.keyboard, Keyboard::Absent);
    assert_eq!(facts.device, Some(Device::Pad(PadKind::Playdate)));
    let display = platform.display();
    assert_eq!((display.width, display.height), (400, 240));
    assert_eq!(display.color, shuyi_platform::platform::Color::OneBit);
}

#[test]
fn the_game_hears_the_buttons_through_its_bindings() {
    let (sdk, mut driver) = running();
    // A is `East`, which the toy's own table has on Cancel.
    let on_east = driver
        .app()
        .game()
        .and_then(|game| game.input().keymap().pad_action(Button::East));
    assert_eq!(on_east, Some(Action::Cancel));
    sdk.queue_buttons(0x20, 0x20);
    driver.update();
    sdk.queue_buttons(0, 0);
    driver.update();
    assert!(driver.app().has_quit(), "Cancel on the home screen quits");
    // A quit game keeps its last frame, and the loop goes on.
    assert!(!driver.update());
}

/// Keeps what the platform gives, to see it without the game.
fn polled(platform: &mut Playdate<FakeSdk>) -> Vec<shuyi_ui::RawInputEvent> {
    let mut out = Vec::new();
    platform.poll_input(&mut out);
    out
}

#[test]
fn a_crank_turn_of_90_degrees_gives_two_forward_presses() {
    let sdk = FakeSdk::default();
    let mut platform = Playdate::new(sdk.clone());
    sdk.queue_crank(90.0);
    let forward = Button::CrankForward;
    let (down, up) = (
        shuyi_ui::RawInputEvent::PadDown(forward, PadKind::Playdate),
        shuyi_ui::RawInputEvent::PadUp(forward),
    );
    assert_eq!(polled(&mut platform), [down, up, down, up]);
}

#[test]
fn a_menu_choice_is_a_press_of_its_button_and_only_of_a_slot_it_has() {
    let mut platform = Playdate::new(FakeSdk::default());
    platform.menu_chosen(1);
    platform.menu_chosen(3);
    let button = Button::SystemMenu1;
    assert_eq!(
        polled(&mut platform),
        [
            shuyi_ui::RawInputEvent::PadDown(button, PadKind::Playdate),
            shuyi_ui::RawInputEvent::PadUp(button),
        ]
    );
    assert_eq!(polled(&mut platform), []);
}

/// The game with `PrevUnit` on the first menu item and `Confirm` past the
/// three slots the profile gives.
fn on_the_menu(setup: Setup) -> Game<Toy> {
    let mut ctx = setup.ctx(toy::ctx());
    ctx.keymap = ctx.keymap.clone().with_pad([
        (Button::SystemMenu0, Action::PrevUnit),
        (Button::SystemMenu3, Action::Confirm),
    ]);
    toy::game_on(ctx)
}

#[test]
fn optional_actions_on_the_system_menu_are_its_items() {
    let sdk = FakeSdk::default();
    let start = Start::new(Card::text("Toy"), look, |setup| Ok(on_the_menu(setup)));
    let mut driver = Driver::new(start, sdk.clone());
    driver.update();
    driver.update();
    assert_eq!(sdk.menu(), [("PrevUnit".to_string(), 0)]);
}

/// Confirm and Cancel can only be pressed by winding the crank.
fn on_the_crank(setup: Setup) -> Game<Toy> {
    let mut ctx = setup.ctx(toy::ctx());
    ctx.keymap = ctx.keymap.clone().with_pad([
        (Button::CrankForward, Action::Confirm),
        (Button::CrankBack, Action::Cancel),
    ]);
    toy::game_on(ctx)
}

/// The text of the last row of cells.
fn last_row(driver: &Driver<Toy, FakeSdk>) -> String {
    let buffer = driver
        .app()
        .game()
        .unwrap_or_else(|| panic!("loaded"))
        .buffer();
    let y = i32::from(buffer.height()) - 1;
    (0..i32::from(buffer.width()))
        .filter_map(|x| buffer.get(x, y).map(|cell| cell.glyph))
        .collect()
}

#[test]
fn a_docked_crank_gets_a_hint_while_a_required_action_is_on_it() {
    let sdk = FakeSdk::default();
    sdk.set_docked(true);
    let start = Start::new(Card::text("Toy"), look, |setup| Ok(on_the_crank(setup)));
    let mut driver = Driver::new(start, sdk.clone());
    driver.update();
    driver.update();
    driver.update();
    assert!(
        last_row(&driver).contains("Unfold the crank"),
        "{:?}",
        last_row(&driver)
    );
    sdk.set_docked(false);
    driver.update();
    driver.update();
    assert!(
        !last_row(&driver).contains("Unfold the crank"),
        "{:?}",
        last_row(&driver)
    );
}

#[test]
fn no_hint_where_the_required_actions_have_a_button() {
    let (sdk, mut driver) = running();
    sdk.set_docked(true);
    driver.update();
    driver.update();
    assert!(!last_row(&driver).contains("crank"));
}

#[test]
fn the_crank_is_reported_when_it_changes() {
    let sdk = FakeSdk::default();
    let mut platform = Playdate::new(sdk.clone());
    let events = |platform: &mut Playdate<FakeSdk>| {
        let mut out = Vec::new();
        platform.system_events(&mut out);
        out
    };
    assert_eq!(events(&mut platform), [SystemEvent::CrankUndocked]);
    assert_eq!(events(&mut platform), []);
    sdk.set_docked(true);
    assert_eq!(events(&mut platform), [SystemEvent::CrankDocked]);
}

#[test]
fn the_lock_and_the_menu_suspend_and_resume_once() {
    let mut platform = Playdate::new(FakeSdk::default());
    let events = |platform: &mut Playdate<FakeSdk>| {
        let mut out = Vec::new();
        platform.system_events(&mut out);
        out.retain(|event| {
            !matches!(event, SystemEvent::CrankDocked | SystemEvent::CrankUndocked)
        });
        out
    };
    assert!(platform.event(SdkEvent::Pause), "heard at once");
    assert_eq!(events(&mut platform), [SystemEvent::Suspend]);
    assert!(!platform.event(SdkEvent::Lock), "suspended already");
    assert!(!platform.event(SdkEvent::Unlock));
    assert_eq!(events(&mut platform), []);
    assert!(!platform.event(SdkEvent::Resume));
    assert_eq!(events(&mut platform), [SystemEvent::Resume]);
    assert!(platform.event(SdkEvent::LowPower));
    assert_eq!(events(&mut platform), [SystemEvent::LowBattery]);
    assert!(platform.event(SdkEvent::Terminate));
    assert_eq!(events(&mut platform), [SystemEvent::QuitRequested]);
}

#[test]
fn the_sdks_events_are_read_by_their_numbers() {
    let read = [2, 3, 4, 5, 6, 9].map(SdkEvent::from_raw);
    assert_eq!(
        read,
        [
            Some(SdkEvent::Lock),
            Some(SdkEvent::Unlock),
            Some(SdkEvent::Pause),
            Some(SdkEvent::Resume),
            Some(SdkEvent::Terminate),
            Some(SdkEvent::LowPower),
        ]
    );
    assert_eq!(SdkEvent::from_raw(7), None, "a key press on the Simulator");
}

#[test]
fn a_suspend_is_heard_at_once_and_the_game_stops() {
    let (_sdk, mut driver) = running();
    driver.event(SdkEvent::Lock);
    assert!(driver.app().is_suspended());
    driver.event(SdkEvent::Unlock);
    driver.update();
    assert!(!driver.app().is_suspended());
}

#[test]
fn the_system_closing_the_game_quits_it_at_once() {
    let (_sdk, mut driver) = running();
    driver.event(SdkEvent::Terminate);
    assert!(driver.app().has_quit());
}

/// The ticket: a request for the frame cap is confirmed, with the rate the
/// device runs at.
#[test]
fn the_rate_is_reported_as_the_cap_applied_with_the_last_serial() {
    let (sdk, mut driver) = running();
    let game = driver.app.game_mut().unwrap_or_else(|| panic!("loaded"));
    let request = game.ctx_mut().request_frame_cap(FrameCap::Fps(60));
    driver.update();
    let pacing = driver.platform().pacing(0);
    assert_eq!(pacing.cap, FrameCap::Fps(30));
    let serial = driver
        .app()
        .game()
        .map(|game| game.ctx().frame_cap_serial());
    assert_eq!(Some(pacing.serial), serial);
    driver.update();
    let _ = sdk;
    let status = driver
        .app()
        .game()
        .map(|game| game.ctx().frame_cap_status(request));
    assert!(
        matches!(status, Some(CapStatus::Applied(_))),
        "the request is answered: {status:?}"
    );
}

#[test]
fn the_frames_a_second_are_measured() {
    let (sdk, mut driver) = running();
    for frame in 0..100 {
        sdk.set_millis(frame * 33);
        driver.update();
    }
    assert_eq!(driver.fps.last, 30);
}

#[test]
fn the_clock_goes_on_past_the_counters_wrap() {
    let mut clock = Clock::default();
    assert_eq!(clock.at(u32::MAX), Millis(u64::from(u32::MAX)));
    assert_eq!(clock.at(5), Millis((1 << 32) + 5));
}

#[test]
fn the_log_goes_to_the_console_one_line_each_with_its_frame() {
    let (sdk, mut driver) = running();
    let game = driver.app.game_mut().unwrap_or_else(|| panic!("loaded"));
    game.ctx_mut()
        .log(LogLevel::Warn, LogCategory::Assets, "a file is big".to_string());
    driver.update();
    let frame = driver.app().frame_number();
    assert!(
        sdk.logs()
            .contains(&alloc::format!("f{frame} warn assets: a file is big")),
        "{:?}",
        sdk.logs()
    );
}

#[test]
fn the_debug_build_has_a_clock_and_counts_the_heap() {
    let mut platform = Playdate::new(FakeSdk::default());
    assert_eq!(platform.clock().is_some(), cfg!(debug_assertions));
    assert!(platform.memory().is_some());
}

#[test]
fn the_pack_is_read_from_the_pdx() {
    let sdk = FakeSdk::default();
    assert!(crate::backend::read_pack(&sdk).is_err());
    sdk.put_bundled(crate::sdk::PACK, b"pack");
    assert_eq!(crate::backend::read_pack(&sdk), Ok(&b"pack"[..]));
}
