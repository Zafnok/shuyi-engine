//! Scripts: each step's check, `wait_frames`, `press_for`, each step's
//! frames in the plan, and how its errors name the step.

use shuyi_ui::input::state::Device;
use shuyi_ui::input::{Key, RepeatDef};
use shuyi_ui::toy::{self, Toy};

use super::*;
use crate::harness::Harness;

fn keys(script: &str) -> Step {
    Step::Keys(script.to_owned())
}

fn pad(script: &str) -> Step {
    Step::Pad(script.to_owned())
}

fn act(name: &str) -> Step {
    Step::Act(name.to_owned())
}

/// A keymap with Confirm on a key, Cancel on a button, Info on two buttons
/// together, and nothing for Menu.
fn keymap() -> Keymap<toy::Action> {
    use toy::Action;
    let pair = ButtonChord::pair(Button::South, Button::East).unwrap();
    Keymap::new(
        [(Chord::plain(Key::F), Action::Confirm)],
        RepeatDef::default(),
    )
    .with_pad([
        (ButtonChord::single(Button::East), Action::Cancel),
        (ButtonChord::single(Button::South), Action::Confirm),
        (pair, Action::Info),
    ])
}

#[test]
fn a_frame_is_a_60th_of_a_second_as_the_harness_runs_it() {
    assert_eq!(FRAME_DT.to_bits(), (1.0f32 / 60.0).to_bits());
    assert_eq!(FRAME_DT.to_bits(), crate::harness::FRAME_DT.to_bits());
    assert_eq!(
        wait_frames(crate::harness::MAX_WAIT),
        crate::harness::MAX_FRAMES
    );
}

#[test]
fn check_refuses_every_kind_of_bad_step() {
    for (step, said) in [
        (keys("f Nope"), "keys: unknown key"),
        (pad("South+Nope"), "pad: "),
        (pad("Nope"), "pad: "),
        (act(""), "act: no action named"),
        (act("  "), "act: no action named"),
        (Step::Wait(-0.5), "wait: -0.5 is not a number of seconds"),
        (Step::Wait(f32::NAN), "wait: NaN is not"),
        (Step::Wait(f32::INFINITY), "wait: inf is not"),
        (
            Step::Crank(f32::NAN),
            "crank: NaN is not a number of degrees",
        ),
    ] {
        let error = step.check().unwrap_err();
        assert!(error.starts_with(said), "{step}: {error}");
    }
    for good in [
        keys("f Shift+Space"),
        keys(""),
        pad("South East+DpadDown"),
        act("Jump"),
        Step::Wait(0.0),
        Step::Wait(1_000_000.0),
        Step::Crank(-30.0),
    ] {
        assert_eq!(good.check(), Ok(()), "{good}");
    }
    let script = Script::new(vec![keys("f"), Step::Wait(-1.0)]);
    assert!(script.check().unwrap_err().starts_with("step 2: wait: "));
}

#[test]
fn the_longest_wait_is_the_players_to_check() {
    let steps = [keys("f"), Step::Wait(2.0), Step::Wait(2.5)];
    assert_eq!(check_waits(&steps[..2], 2.0), Ok(()));
    let error = check_waits(&steps, 2.0).unwrap_err();
    assert_eq!(
        error,
        "step 3: wait: at most 2.0 seconds in one step, not 2.5"
    );
}

#[test]
fn a_wait_is_its_seconds_at_60_frames_a_second_rounded_up() {
    for (seconds, frames) in [
        (0.0, 0),
        (1.0 / 60.0, 1),
        (0.5, 30),
        (0.51, 31),
        (0.001, 1),
        (1.0, 60),
        (0.25, 15),
        (1_800.0, 108_000),
        (-1.0, 0),
        (f32::NAN, 0),
    ] {
        assert_eq!(wait_frames(seconds), frames, "{seconds}");
    }
    // Every whole number of frames, as seconds, is that number again.
    for n in 0..=6_000u16 {
        assert_eq!(wait_frames(f32::from(n) / 60.0), u32::from(n), "{n}");
    }
}

#[test]
fn a_step_is_shown_as_a_plan_writes_it() {
    let shown: Vec<String> = [
        keys("Enter"),
        pad("South"),
        act("Confirm"),
        Step::Wait(0.25),
        Step::Crank(-30.0),
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(
        shown,
        [
            r#"keys("Enter")"#,
            r#"pad("South")"#,
            r#"act("Confirm")"#,
            "wait(0.25)",
            "crank(-30)"
        ]
    );
}

#[test]
fn a_steps_json_is_the_contracts() {
    let script = Script::new(vec![
        keys("Enter Up"),
        pad("South+East"),
        act("Confirm"),
        Step::Wait(0.5),
        Step::Crank(30.0),
    ]);
    let json = serde_json::to_string(&script).unwrap();
    assert_eq!(
        json,
        r#"{"steps":[{"keys":"Enter Up"},{"pad":"South+East"},{"act":"Confirm"},{"wait":0.5},{"crank":30.0}]}"#
    );
    assert_eq!(serde_json::from_str::<Script>(&json).unwrap(), script);
    assert!(serde_json::from_str::<Script>(r#"{"steps":[{"jump":1}]}"#).is_err());
    assert!(serde_json::from_str::<Script>(r#"{"steps":[],"more":1}"#).is_err());
}

#[test]
fn press_for_takes_the_primary_key_then_the_primary_button() {
    let keymap = keymap();
    let pair = ButtonChord::pair(Button::South, Button::East).unwrap();
    assert_eq!(
        press_for(&keymap, "Confirm"),
        Ok(Press::Keys(Chord::plain(Key::F)))
    );
    assert_eq!(
        press_for(&keymap, "Cancel"),
        Ok(Press::Pad(ButtonChord::single(Button::East)))
    );
    assert_eq!(press_for(&keymap, "Info"), Ok(Press::Pad(pair)));
    let neither = press_for(&keymap, "Menu").unwrap_err();
    assert!(
        neither.contains("Menu") && neither.contains("neither"),
        "{neither}"
    );
    let unknown = press_for(&keymap, "Jump").unwrap_err();
    assert!(
        unknown.contains("\"Jump\"") && unknown.contains("Confirm, Cancel"),
        "{unknown}"
    );
    // The actions an act can press: those with a key or a button.
    assert_eq!(pressable(&keymap), ["Confirm", "Cancel", "Info"]);
}

#[test]
fn each_step_becomes_frames_of_events() {
    let shift_space = Chord::shifted(Key::Space);
    let kind = PadKind::default();
    let frames = frames(
        &[
            keys("f Shift+Space"),
            pad("South East+DpadDown"),
            Step::Wait(0.0),
            Step::Wait(0.05),
            act("Info"),
            act("Confirm"),
            act("Cancel"),
        ],
        &keymap(),
        kind,
    )
    .unwrap();
    let events = |e: &[RawInputEvent]| Planned::Events(e.to_vec());
    let expected = [
        events(&[RawInputEvent::Down(Chord::plain(Key::F))]),
        events(&[RawInputEvent::Up(Key::F)]),
        events(&[RawInputEvent::Down(shift_space)]),
        events(&[RawInputEvent::Up(Key::Space)]),
        events(&[RawInputEvent::PadDown(Button::South, kind)]),
        events(&[RawInputEvent::PadUp(Button::South)]),
        events(&[
            RawInputEvent::PadDown(Button::East, kind),
            RawInputEvent::PadDown(Button::DpadDown, kind),
        ]),
        events(&[
            RawInputEvent::PadUp(Button::East),
            RawInputEvent::PadUp(Button::DpadDown),
        ]),
        // The wait of 0 is no frame; 0.05 s is 3.
        Planned::Idle(3),
        // An action by two buttons together, by a key, by a button.
        events(&[
            RawInputEvent::PadDown(Button::South, kind),
            RawInputEvent::PadDown(Button::East, kind),
        ]),
        events(&[
            RawInputEvent::PadUp(Button::South),
            RawInputEvent::PadUp(Button::East),
        ]),
        events(&[RawInputEvent::Down(Chord::plain(Key::F))]),
        events(&[RawInputEvent::Up(Key::F)]),
        events(&[RawInputEvent::PadDown(Button::East, kind)]),
        events(&[RawInputEvent::PadUp(Button::East)]),
    ];
    assert_eq!(frames, expected);
    assert_eq!(super::frames(&[], &keymap(), kind), Ok(Vec::new()));
}

#[test]
fn buttons_are_pressed_on_the_pad_given() {
    let kind = PadKind::default();
    let other = PadKind::ALL.iter().copied().find(|k| *k != kind).unwrap();
    let frames = frames(&[pad("South")], &keymap(), other).unwrap();
    assert_eq!(
        frames[0],
        Planned::Events(vec![RawInputEvent::PadDown(Button::South, other)])
    );
}

#[test]
fn an_error_names_its_step() {
    let kind = PadKind::default();
    let error = frames(&[keys("f"), act("Menu")], &keymap(), kind).unwrap_err();
    assert!(error.starts_with("step 2: act: Menu"), "{error}");
    let error = frames(&[act("Jump")], &keymap(), kind).unwrap_err();
    assert!(
        error.starts_with("step 1: act: the game has no action \"Jump\""),
        "{error}"
    );
    let error = frames(&[keys("f"), keys("f"), keys("Nope")], &keymap(), kind).unwrap_err();
    assert!(error.starts_with("step 3: keys: "), "{error}");
    let error = frames(&[Step::Crank(30.0)], &keymap(), kind).unwrap_err();
    assert_eq!(error, "step 1: crank: not over the live link yet");
}

#[test]
fn the_harness_plays_a_script_as_its_own_presses() {
    let script = Script::new(vec![
        keys("f"),
        Step::Wait(0.05),
        pad("South"),
        Step::Crank(90.0),
    ]);
    let mut by_script = Harness::new(toy::game());
    script.play(&mut by_script);
    let mut by_hand = Harness::new(toy::game());
    by_hand.keys("f");
    for _ in 0..3 {
        by_hand.frame(&[], FRAME_DT);
    }
    by_hand.pad("South").crank(90.0);
    assert_eq!(by_script.frame_count(), by_hand.frame_count());
    assert_eq!(by_script.frame_count(), 2 + 3 + 2 + 1);
    assert_eq!(by_script.screens(), by_hand.screens());
    assert_eq!(by_script.snapshot(), by_hand.snapshot());
}

#[test]
fn a_script_presses_buttons_on_the_pad_the_harness_uses() {
    let kind = PadKind::ALL
        .iter()
        .copied()
        .find(|k| *k != PadKind::default())
        .unwrap();
    let mut h = Harness::new(toy::game());
    h.use_pad(kind);
    Script::new(vec![pad("DpadDown")]).play(&mut h);
    // A bound button's press switches the player's device to its pad.
    assert_eq!(h.game().input().device(), Device::Pad(kind));
}

#[test]
fn the_harness_presses_an_action_by_its_key() {
    let confirm = press_for(toy::game().input().keymap(), "Confirm").unwrap();
    let Press::Keys(chord) = confirm else {
        panic!("Confirm has no key");
    };
    let mut by_name = Harness::new(toy::game());
    Script::new(vec![act("Confirm")]).play(&mut by_name);
    let mut by_key = Harness::new(toy::game());
    by_key.keys(chord.key.name());
    assert_eq!(by_name.screens(), by_key.screens());
    assert_eq!(by_name.snapshot(), by_key.snapshot());
}

#[test]
#[should_panic(expected = "Harness: step 1: act: the game has no action \"Jump\"")]
fn the_harness_panics_on_an_action_the_game_lacks() {
    let mut h: Harness<Toy> = Harness::new(toy::game());
    Script::new(vec![act("Jump")]).play(&mut h);
}

#[test]
#[should_panic(expected = "more than 2000 frames in one wait")]
fn the_harness_panics_on_a_wait_longer_than_its_longest() {
    let mut h: Harness<Toy> = Harness::new(toy::game());
    Script::new(vec![Step::Wait(crate::harness::MAX_WAIT + 1.0)]).play(&mut h);
}

#[test]
fn a_debug_step_turns_channels_on_in_the_harness_and_plays_no_frame() {
    let debug = |ids: &str| Step::Debug(ids.to_owned());
    for ok in ["grid", "grid,collision", "all", " grid , net_2 "] {
        assert_eq!(debug(ok).check(), Ok(()), "{ok}");
    }
    for bad in ["", "Grid", "grid,,net", "grid net", "é"] {
        let error = debug(bad).check().unwrap_err();
        assert!(error.starts_with("debug: "), "{bad}: {error}");
    }
    assert_eq!(debug("grid,net").to_string(), r#"debug("grid,net")"#);
    let json = serde_json::to_string(&debug("grid")).unwrap();
    assert_eq!(json, r#"{"debug":"grid"}"#);
    // Not over the live link, as a plan of frames.
    let error = frames(&[debug("grid")], &keymap(), PadKind::default()).unwrap_err();
    assert_eq!(error, "step 1: debug: not over the live link yet");
    let mut h: Harness<Toy> = Harness::new(toy::game());
    h.ctx_mut().debug_tools = false;
    Script::new(vec![debug("grid,net")]).play(&mut h);
    assert_eq!(h.frame_count(), 0);
    let view = &h.ctx().debug_view;
    assert!(h.ctx().debug_tools && view.is_on("grid") && view.is_on("net"));
    assert!(!view.is_on("anything") && !view.is_on("all"));
    Script::new(vec![debug("all")]).play(&mut h);
    let view = &h.ctx().debug_view;
    assert!(view.is_on("anything") && !view.on().any(|id| id == "all"));
}

/// A system hook that logs each dropped file's name and size.
fn log_drops(game: &mut shuyi_ui::Game<Toy>, event: crate::SystemEvent) {
    if let crate::SystemEvent::FilesDropped(files) = event {
        for file in files {
            let line = format!("dropped {} ({} bytes)", file.name(), file.bytes().len());
            let (level, category) = (
                shuyi_ui::log::LogLevel::Warn,
                shuyi_ui::log::LogCategory::Game,
            );
            game.ctx_mut().log(level, category, line);
        }
    }
}

/// The text of the lines `h`'s game has logged.
fn logged(h: &Harness<Toy>) -> Vec<String> {
    h.log().lines().map(|l| l.text.clone()).collect()
}

#[test]
#[cfg(feature = "std")]
fn a_drop_step_drops_the_file_by_its_name_in_one_frame() {
    let drop = |path: &str| Step::Drop(path.to_owned());
    assert_eq!(drop("a.txt").check(), Ok(()));
    assert_eq!(drop(" ").check().unwrap_err(), "drop: no file named");
    assert_eq!(drop("levels/a.txt").to_string(), r#"drop("levels/a.txt")"#);
    let json = serde_json::to_string(&drop("a.txt")).unwrap();
    assert_eq!(json, r#"{"drop":"a.txt"}"#);
    let error = frames(&[drop("a.txt")], &keymap(), PadKind::default()).unwrap_err();
    assert_eq!(error, "step 1: drop: not over the live link yet");

    // This file, from the crate's folder, where its tests run.
    let path = "src/script/tests.rs";
    let size = super::files::read(path).unwrap().len();
    let mut h: Harness<Toy> = Harness::new(toy::game()).with_system_hook(log_drops);
    Script::new(vec![drop(path)]).play(&mut h);
    assert_eq!(h.frame_count(), 1);
    assert!(
        logged(&h).contains(&format!("dropped tests.rs ({size} bytes)")),
        "{:?}",
        logged(&h)
    );
}

#[test]
#[cfg(feature = "std")]
#[should_panic(expected = "Harness: step 1: drop: no/such.file")]
fn the_harness_panics_on_a_file_it_cant_read() {
    let mut h: Harness<Toy> = Harness::new(toy::game());
    Script::new(vec![Step::Drop("no/such.file".to_owned())]).play(&mut h);
}

#[test]
fn a_drop_over_the_limits_is_refused_and_logged() {
    use crate::dropped::{DroppedFile, MAX_DROPPED_FILES};
    let mut h: Harness<Toy> = Harness::new(toy::game()).with_system_hook(log_drops);
    let many = (0..=MAX_DROPPED_FILES)
        .map(|i| DroppedFile::new(&format!("{i}.txt"), Vec::new()))
        .collect();
    h.drop_files(many);
    let big = DroppedFile::new("big.wad", vec![0; crate::dropped::MAX_DROPPED_BYTES + 1]);
    h.drop_files(vec![big, DroppedFile::new("small.wad", vec![1, 2])]);
    assert_eq!(
        logged(&h),
        [
            "a drop of 17 files was refused: at most 16 at once".to_owned(),
            format!(
                "the dropped file big.wad was refused: {} bytes, more than {}",
                crate::dropped::MAX_DROPPED_BYTES + 1,
                crate::dropped::MAX_DROPPED_BYTES
            ),
            "dropped small.wad (2 bytes)".to_owned(),
        ]
    );
}
