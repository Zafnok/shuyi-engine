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
fn a_script_presses_buttons_on_the_harnesss_pad() {
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
