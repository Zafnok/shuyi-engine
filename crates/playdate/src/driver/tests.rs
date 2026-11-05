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
use shuyi_ui::perf::{HeapCounts, MemoryReading};
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
            wide: BTreeMap::new(),
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
fn a_light_card_is_black_on_white_and_a_picture_shows_its_ground() {
    let white = shuyi_ui::frame::Rgb::new(255, 255, 255);
    let black = shuyi_ui::frame::Rgb::new(0, 0, 0);
    for (card, text, on_white) in [
        (Card::text("Toy").colored(black, white), "Toy", true),
        (Card::picture("card.png"), "", false),
    ] {
        let sdk = FakeSdk::default();
        let start = Start::new(card, look, |setup| Ok(load(setup)));
        let mut driver = Driver::new(start, sdk.clone());
        assert!(driver.update());
        assert_eq!(sdk.take_texts(), [(text.to_string(), on_white)]);
    }
}

#[test]
fn a_frame_like_the_last_doesnt_touch_the_frame_buffer() {
    let (sdk, mut driver) = running();
    let before = sdk.frame_uses();
    assert!(!driver.update());
    assert_eq!(sdk.frame_uses(), before);
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
    assert_eq!(
        marked.len(),
        1,
        "one run, the screen's first draw: {marked:?}"
    );
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
fn a_menu_the_sdk_refuses_is_logged_and_one_it_takes_isnt() {
    let refused = |full: bool| {
        let sdk = FakeSdk::default();
        if full {
            sdk.fill_menu();
        }
        let start = Start::new(Card::text("Toy"), look, |setup| Ok(on_the_menu(setup)));
        let mut driver = Driver::new(start, sdk.clone());
        driver.update();
        driver.update();
        sdk.logs().iter().any(|line| line.contains("refused"))
    };
    assert!(refused(true));
    assert!(!refused(false));
}

/// The game with `Cancel` on the first menu item alone.
fn cancel_on_the_menu(setup: Setup) -> Game<Toy> {
    let mut ctx = setup.ctx(toy::ctx());
    ctx.keymap = ctx
        .keymap
        .clone()
        .with_pad([(Button::SystemMenu0, Action::Cancel)]);
    toy::game_on(ctx)
}

#[test]
fn a_menu_item_chosen_reaches_the_game_as_its_action() {
    let sdk = FakeSdk::default();
    let start = Start::new(Card::text("Toy"), look, |setup| {
        Ok(cancel_on_the_menu(setup))
    });
    let mut driver = Driver::new(start, sdk.clone());
    driver.update();
    driver.update();
    driver.menu_chosen(0);
    driver.update();
    driver.update();
    assert!(driver.app().has_quit(), "Cancel on the home screen quits");
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
        out.retain(|event| !matches!(event, SystemEvent::CrankDocked | SystemEvent::CrankUndocked));
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
    assert_eq!(
        clock.at(5),
        Millis((1 << 32) + 5),
        "the same reading isn't a wrap"
    );
}

#[test]
fn the_log_goes_to_the_console_one_line_each_with_its_frame() {
    let (sdk, mut driver) = running();
    let game = driver.app.game_mut().unwrap_or_else(|| panic!("loaded"));
    game.ctx_mut().log(
        LogLevel::Warn,
        LogCategory::Assets,
        "a file is big".to_string(),
    );
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
    assert_eq!(
        platform.memory(),
        Some(MemoryReading::new(HeapCounts::new(1024, 2048, 3)))
    );
}

#[test]
fn the_pack_is_read_from_the_pdx() {
    let sdk = FakeSdk::default();
    assert!(crate::backend::read_pack(&sdk).is_err());
    sdk.put_bundled(crate::sdk::PACK, b"pack");
    assert_eq!(crate::backend::read_pack(&sdk), Ok(&b"pack"[..]));
}

/// The game's sounds load with it, from the `.pdx`, and the console says
/// which it lacks.
#[test]
fn the_games_sounds_load_with_it_and_the_console_names_a_missing_one() {
    let frames = |sdk: &FakeSdk| {
        let mut driver = Driver::new(start(), sdk.clone());
        for _ in 0..3 {
            driver.update();
        }
    };
    let sdk = FakeSdk::default();
    sdk.put_sample("audio/click");
    frames(&sdk);
    assert_eq!(sdk.samples_loaded(), ["audio/click"]);
    assert_eq!(sdk.logs(), [] as [String; 0]);
    let lacking = FakeSdk::default();
    frames(&lacking);
    let logs = lacking.logs();
    let line = "audio: sound \"click\": audio/click: not in the .pdx";
    assert!(logs.iter().any(|l| l == line), "{logs:?}");
}

/// The game files the reading toy's load reads.
static FILES: &[(&str, &[u8])] = &[("data/a.ron", b"a"), ("data/b.ron", b"b")];

/// The toy, whose load reads two files, names a step, and asks for a file
/// the bundle lacks.
fn reading_start() -> Start<Toy> {
    Start::new(Card::text("Toy"), look, |setup| {
        let bundle = shuyi_assets::Bundle::new(alloc::vec![shuyi_assets::bundle::Layer::table(
            "game", FILES
        )]);
        let _ = bundle.bytes("data/a.ron");
        crate::report::mark("the rules");
        let _ = bundle.bytes("data/b.ron");
        let _ = bundle.bytes("data/missing.ron");
        let _ = bundle.bytes("data/a.ron");
        Ok(load(setup))
    })
}

/// The run report's lines in the console, without their frame.
fn report_lines(sdk: &FakeSdk) -> Vec<String> {
    sdk.logs()
        .iter()
        .filter_map(|line| line.split_once(" info platform: "))
        .map(|(_, text)| text.to_string())
        .collect()
}

/// Runs `driver` for `frames` updates of 33 milliseconds each, the counter
/// going on from `from`; the counter's next reading.
fn run_for(sdk: &FakeSdk, driver: &mut Driver<Toy, FakeSdk>, from: u32, frames: u32) -> u32 {
    let mut now = from;
    for _ in 0..frames {
        sdk.set_millis(now);
        driver.update();
        now = now.wrapping_add(33);
    }
    now
}

/// The ticket: the start's lines, then a minute's, on the minute, through
/// the public API.
#[test]
fn report_lines_on_the_fake() {
    let sdk = FakeSdk::default();
    FakeSdk::set_clock_us(0);
    FakeSdk::set_clock_step_us(1_000);
    sdk.set_stack_used(Some(4_096));
    let mut driver = Driver::with_report(reading_start(), sdk.clone(), RunReport::new());
    let now = run_for(&sdk, &mut driver, 0, 3);
    let lines = report_lines(&sdk);
    let starts = |prefix: &str| lines.iter().any(|l| l.starts_with(prefix));
    assert!(starts("Loading card shown "), "{lines:?}");
    assert!(starts("Game data loaded in "), "{lines:?}");
    assert!(lines.contains(&"Game stack used: 4,096 bytes".to_string()));
    assert!(
        lines.contains(&"Bundle: 4 reads of 3 files, 1 not in the bundle".to_string()),
        "{lines:?}"
    );
    assert!(lines.contains(&"Not in the bundle: data/missing.ron".to_string()));
    let slowest = lines
        .iter()
        .find(|l| l.starts_with("Slowest reads, ms to the next read: "))
        .unwrap_or_else(|| panic!("{lines:?}"));
    assert!(slowest.contains("(the rules) "), "{slowest}");
    assert!(starts("First frame of the game shown "), "{lines:?}");
    let card = lines.iter().position(|l| l.starts_with("Loading card"));
    let first = lines.iter().position(|l| l.starts_with("First frame"));
    assert!(card < first, "{lines:?}");

    // Nothing more until the minute is up, then the running lines.
    let before = report_lines(&sdk).len();
    let now = run_for(&sdk, &mut driver, now, 1_800);
    assert_eq!(report_lines(&sdk).len(), before, "{:?}", report_lines(&sdk));
    run_for(&sdk, &mut driver, now, 20);
    let minute = report_lines(&sdk).split_off(before);
    let top = driver.app().game().and_then(Game::top_screen);
    let top = top.unwrap_or_else(|| panic!("a screen is open"));
    assert_eq!(minute.len(), 5, "{minute:?}");
    assert!(
        minute[0].starts_with(&format!("On {top} for 60 seconds: 30.")),
        "{minute:?}"
    );
    assert!(minute[1].starts_with("App::frame: min "), "{minute:?}");
    assert!(minute[2].starts_with("Draw and flush: "), "{minute:?}");
    assert_eq!(minute[3], "Game stack used: 4,096 bytes");
    assert_eq!(minute[4], "Music: none");
}

/// The ticket: off, five minutes say nothing and read the timer no more
/// than the app itself does.
#[test]
fn with_the_report_off_nothing_is_measured_or_said() {
    const FRAMES: u32 = 5 * 60 * 30;
    let sdk = FakeSdk::default();
    let reads = FakeSdk::clock_reads();
    let mut driver = Driver::new(reading_start(), sdk.clone());
    run_for(&sdk, &mut driver, 0, FRAMES);
    let by_driver = FakeSdk::clock_reads() - reads;
    assert_eq!(report_lines(&sdk), [] as [String; 0]);

    // The same frames on the app and the backend alone.
    let bare = FakeSdk::default();
    let reads = FakeSdk::clock_reads();
    let mut app = App::new(reading_start());
    let mut platform = Playdate::new(bare.clone());
    let mut started = false;
    for frame in 0..FRAMES {
        let now = u64::from(frame) * 33;
        app.set_pacing(platform.pacing(0));
        let _ = app.frame(&mut platform, Millis(now));
        if let (false, Some(game)) = (started, app.game()) {
            platform.start(game);
            started = true;
        }
    }
    assert_eq!(by_driver, FakeSdk::clock_reads() - reads);
}

/// With `phases`, the running lines give every phase of the draw.
#[test]
fn phases_on_name_every_phase_and_off_none() {
    for phases in [true, false] {
        let sdk = FakeSdk::default();
        let report = RunReport::new().every_seconds(1).phases(phases);
        let mut driver = Driver::with_report(start(), sdk.clone(), report);
        let now = run_for(&sdk, &mut driver, 0, 3);
        press(&mut driver, Action::Confirm);
        run_for(&sdk, &mut driver, now, 40);
        let lines = report_lines(&sdk);
        let line = lines.iter().find(|l| l.starts_with("Draw by phase"));
        assert_eq!(line.is_some(), phases, "{lines:?}");
        if let Some(line) = line {
            for phase in shuyi_ui::frame::onebit::Phase::ALL {
                assert!(line.contains(&format!(" {} ", phase.name())), "{line}");
            }
        }
        let draws = lines.iter().find(|l| l.starts_with("Draw and flush"));
        let draws = draws.unwrap_or_else(|| panic!("{lines:?}"));
        assert!(draws.contains(" 1 times"), "the menu's draw: {draws}");
    }
}

/// A watcher set before the report's keeps its place: the report says so
/// and goes on without reads.
#[test]
fn a_second_watcher_is_refused_and_the_report_goes_on() {
    fn other(_: &str, _: bool) {}
    let sdk = FakeSdk::default();
    let mut driver = Driver::reporting(reading_start(), sdk.clone(), RunReport::new(), |_| {
        Err(other)
    });
    run_for(&sdk, &mut driver, 0, 3);
    let lines = report_lines(&sdk);
    assert_eq!(lines[0], crate::report::REFUSED);
    assert!(!lines.iter().any(|l| l.starts_with("Bundle:")), "{lines:?}");
    assert!(
        lines.iter().any(|l| l.starts_with("First frame")),
        "{lines:?}"
    );
}

/// A named file holds the start's lines and every read, once the first
/// frame is shown.
#[test]
fn the_report_file_holds_the_start_and_every_read() {
    let sdk = FakeSdk::default();
    let report = RunReport::new().file("run-report.txt");
    let mut driver = Driver::with_report(reading_start(), sdk.clone(), report);
    driver.update();
    assert_eq!(
        sdk.file("run-report.txt"),
        None,
        "not before the game shows"
    );
    run_for(&sdk, &mut driver, 0, 3);
    let text = sdk
        .file("run-report.txt")
        .unwrap_or_else(|| panic!("written"));
    let text = String::from_utf8(text).unwrap_or_else(|e| panic!("{e}"));
    let lines: Vec<&str> = text.lines().collect();
    for line in report_lines(&sdk) {
        assert!(lines.contains(&line.as_str()), "{line} in {lines:?}");
    }
    for read in [
        "data/a.ron",
        "(the rules)",
        "data/b.ron",
        "data/missing.ron",
    ] {
        assert!(
            lines.iter().any(|l| l.starts_with(&format!("  {read}: "))),
            "{read} in {lines:?}"
        );
    }
    assert!(lines.iter().any(|l| l.ends_with(", 2 reads")), "{lines:?}");
    assert!(
        lines.iter().any(|l| l.ends_with("ms, not in the bundle")),
        "{lines:?}"
    );
}

/// The running lines name the music heard, and say when its file player ran
/// out of data.
#[test]
fn the_music_line_says_when_a_file_player_underran() {
    let sdk = FakeSdk::default();
    sdk.put_track("music/theme", 100.0);
    let report = RunReport::new().every_seconds(1);
    let mut driver = Driver::with_report(start(), sdk.clone(), report);
    let now = run_for(&sdk, &mut driver, 0, 3);
    let game = driver.app.game_mut().unwrap_or_else(|| panic!("loaded"));
    game.ctx_mut().audio.play_music("theme");
    let now = run_for(&sdk, &mut driver, now, 40);
    let lines = report_lines(&sdk);
    assert!(
        lines.contains(&"Music: theme, no file player ran out of data".to_string()),
        "{lines:?}"
    );
    sdk.underrun("music/theme");
    run_for(&sdk, &mut driver, now, 40);
    let lines = report_lines(&sdk);
    assert!(
        lines.contains(&"Music: theme, and a file player ran out of data".to_string()),
        "{lines:?}"
    );
}

/// The running lines' stretch goes on across the counter's wrap.
#[test]
fn a_minute_runs_across_the_counters_wrap() {
    let sdk = FakeSdk::default();
    let mut driver = Driver::with_report(start(), sdk.clone(), RunReport::new());
    let from = u32::MAX - 30_000;
    let now = run_for(&sdk, &mut driver, from, 3);
    let before = report_lines(&sdk).len();
    run_for(&sdk, &mut driver, now, 1_830);
    let minute = report_lines(&sdk).split_off(before);
    assert!(
        minute
            .first()
            .is_some_and(|l| l.contains(" for 60 seconds: 30.")),
        "{minute:?}"
    );
}
