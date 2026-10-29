//! Plans: each step read, the checks `parse` makes and how its errors
//! name the step, and the frames each step gives.

use proptest::prelude::*;
use shuyi_ui::input::{Button, PadKind};

use super::*;

/// The plan of `steps`, written as a plan file writes them.
fn plan(steps: &str) -> Result<Plan, String> {
    Plan::parse(&format!("(steps: [{steps}])"))
}

fn chord(name: &str) -> Chord {
    Chord::parse(name).unwrap()
}

fn button(name: &str) -> Button {
    Button::parse(name).unwrap()
}

/// The frames of `steps`.
fn frames(steps: &str) -> Vec<PlannedFrame> {
    plan(steps).unwrap().frames()
}

fn idle() -> PlannedFrame {
    PlannedFrame::default()
}

fn with(events: Vec<RawInputEvent>) -> PlannedFrame {
    PlannedFrame {
        events,
        capture: None,
    }
}

#[test]
fn a_frame_is_the_harness_frame() {
    assert_eq!(FRAME_DT.to_bits(), crate::harness::FRAME_DT.to_bits());
}

#[test]
fn each_step_reads_and_the_tolerance_defaults() {
    let read = Plan::parse(
        r#"(
            steps: [
                wait(0.5),
                capture("title"),
                keys("Enter Up"),
                pad("South+East"),
                wait(1),
                capture("after_moving-2"),
            ],
        )"#,
    )
    .unwrap();
    assert_eq!(
        read.steps(),
        [
            Step::Wait(0.5),
            Step::Capture("title".to_owned()),
            Step::Keys("Enter Up".to_owned()),
            Step::Pad("South+East".to_owned()),
            Step::Wait(1.0),
            Step::Capture("after_moving-2".to_owned()),
        ]
    );
    assert_eq!(read.tolerance(), Tolerance::default());
    let strict = Plan::parse("(steps: [], tolerance: (channel: 0, pixels: 3))").unwrap();
    assert_eq!(strict.tolerance(), Tolerance::new(0, 3));
    assert_eq!(strict.frames(), []);
}

#[test]
fn a_step_is_shown_as_a_plan_writes_it() {
    let shown: Vec<String> = [
        Step::Keys("Enter".to_owned()),
        Step::Pad("South".to_owned()),
        Step::Wait(0.25),
        Step::Capture("title".to_owned()),
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    assert_eq!(
        shown,
        [
            r#"keys("Enter")"#,
            r#"pad("South")"#,
            "wait(0.25)",
            r#"capture("title")"#
        ]
    );
}

/// `parse`'s error for `steps`, which must name `wanted`.
fn refused(steps: &str, wanted: &str) {
    let error = plan(steps).unwrap_err();
    assert!(error.contains(wanted), "{error}");
}

#[test]
fn an_unknown_key_or_button_is_refused_naming_the_step() {
    refused(
        r#"wait(1), keys("Enter Entr")"#,
        r#"step 2 (keys("Enter Entr"))"#,
    );
    refused(r#"pad("South+Sout")"#, r#"step 1 (pad("South+Sout"))"#);
}

#[test]
fn a_capture_needs_a_frame_before_it() {
    refused(
        r#"capture("title")"#,
        r#"step 1 (capture("title")): no frame"#,
    );
    refused(r#"wait(0), keys(""), capture("title")"#, "step 3");
    refused(
        r#"wait(1), capture("a"), capture("b")"#,
        "step 3 (capture(\"b\")): no frame since the last capture",
    );
    assert!(plan(r#"wait(0.01), capture("title")"#).is_ok());
    // A key's or a button's press is a frame to capture.
    assert!(plan(r#"keys("Enter"), capture("title")"#).is_ok());
    assert!(plan(r#"pad("South"), capture("title")"#).is_ok());
}

#[test]
fn a_capture_name_is_used_once_and_names_a_file() {
    refused(
        r#"wait(1), capture("title"), wait(1), capture("title")"#,
        "step 4 (capture(\"title\")): step 2 already uses that name",
    );
    for bad in ["", "Title", "a b", "../up", "a.png", "é"] {
        refused(&format!("wait(1), capture({bad:?})"), "step 2");
    }
}

#[test]
fn a_wait_is_a_number_of_seconds_in_range() {
    for bad in ["-1", "601", "inf", "NaN"] {
        refused(&format!("wait({bad})"), "step 1 (wait(");
    }
}

#[test]
fn a_file_that_is_not_a_plan_is_refused() {
    for text in ["", "(steps: [jump(1)])", "(steps: [], colour: 1)", "[]"] {
        let error = Plan::parse(text).unwrap_err();
        assert!(error.starts_with("not a plan"), "{error}");
    }
}

#[test]
fn waits_round_up_to_whole_frames() {
    assert_eq!(wait_frames(0.0), 0);
    assert_eq!(wait_frames(1.0 / 60.0), 1);
    assert_eq!(wait_frames(0.5), 30);
    assert_eq!(wait_frames(0.51), 31);
    // A thousandth of a frame over rounds down, two round up.
    assert_eq!(wait_frames(0.016_684), 1);
    assert_eq!(wait_frames(0.016_7), 2);
    assert_eq!(wait_frames(0.000_001), 1);
    assert_eq!(wait_frames(-1.0), 0);
    assert_eq!(wait_frames(f32::NAN), 0);
    assert_eq!(wait_frames(f32::INFINITY), 0);
    assert_eq!(wait_frames(1_000.0), 36_000);
    assert_eq!(frames("wait(0)"), []);
    assert_eq!(frames("wait(0.5)").len(), 30);
    assert_eq!(frames("wait(0.51)").len(), 31);
    assert!(frames("wait(0.5)").iter().all(|f| *f == idle()));
}

#[test]
fn keys_and_buttons_go_down_in_one_frame_and_up_in_the_next() {
    let shift_space = chord("Shift+Space");
    assert_eq!(
        frames(r#"keys("Enter Shift+Space")"#),
        [
            with(vec![RawInputEvent::Down(chord("Enter"))]),
            with(vec![RawInputEvent::Up(chord("Enter").key)]),
            with(vec![RawInputEvent::Down(shift_space)]),
            with(vec![RawInputEvent::Up(shift_space.key)]),
        ]
    );
    let kind = PadKind::default();
    let (south, east) = (button("South"), button("East"));
    assert_eq!(
        frames(r#"pad("South+East DpadUp")"#),
        [
            with(vec![
                RawInputEvent::PadDown(south, kind),
                RawInputEvent::PadDown(east, kind),
            ]),
            with(vec![
                RawInputEvent::PadUp(south),
                RawInputEvent::PadUp(east)
            ]),
            with(vec![RawInputEvent::PadDown(button("DpadUp"), kind)]),
            with(vec![RawInputEvent::PadUp(button("DpadUp"))]),
        ]
    );
}

#[test]
fn a_capture_names_the_frame_before_it() {
    let run = frames(r#"wait(0.05), capture("title"), keys("Enter"), capture("next")"#);
    assert_eq!(run.len(), 5);
    let named: Vec<(usize, &str)> = run
        .iter()
        .enumerate()
        .filter_map(|(i, f)| Some((i, f.capture.as_deref()?)))
        .collect();
    assert_eq!(named, [(2, "title"), (4, "next")]);
    assert_eq!(run[4].events, [RawInputEvent::Up(chord("Enter").key)]);
}

/// A step that parses, and the frames it adds.
fn step() -> impl Strategy<Value = (String, usize)> {
    let keys = prop::sample::subsequence(vec!["Enter", "Up", "Shift+Space", "x"], 0..4)
        .prop_map(|keys| (format!("keys({:?})", keys.join(" ")), keys.len() * 2));
    let pad = prop::sample::subsequence(vec!["South", "East+DpadUp", "Start"], 0..3)
        .prop_map(|pads| (format!("pad({:?})", pads.join(" ")), pads.len() * 2));
    let wait = (0u32..200).prop_map(|hundredths| {
        #[allow(clippy::cast_precision_loss)] // below 200
        let seconds = hundredths as f32 / 100.0;
        (format!("wait({seconds})"), wait_frames(seconds) as usize)
    });
    prop_oneof![keys, pad, wait]
}

proptest! {
    /// A plan has as many frames as its steps add up to, whatever they are.
    #[test]
    fn a_plan_has_the_frames_its_steps_add_up_to(steps in prop::collection::vec(step(), 0..12)) {
        let text: Vec<&str> = steps.iter().map(|(s, _)| s.as_str()).collect();
        let total: usize = steps.iter().map(|(_, n)| n).sum();
        let read = plan(&text.join(", ")).unwrap();
        prop_assert_eq!(read.frames().len(), total);
    }
}
