//! A tick stage through the Harness (ticket 2137): the toy's dot, stepped
//! by a `TickDriver`. One script of presses and releases at five frame
//! rates and with jittered frames gives the same steps' presses and
//! releases, each in one step; a tap on a frame with no step, a late
//! frame, key repeats and an action from `Game::act` reach the steps as
//! they should; a press kept for a step never fires after the stage is
//! covered or the game suspended; and a rewind plays no stop twice.
//! What happened is read from the stage's record and its sounds.

use std::time::Duration;

use insta::assert_snapshot;
use proptest::prelude::*;
use shuyi_core::tick::TickRecord;
use shuyi_platform::SystemEvent;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_ui::input::{Chord, Key};
use shuyi_ui::tick::ActionIds;
use shuyi_ui::toy::tick::{DotStage, STOP};
use shuyi_ui::toy::{self, Action, Toy};
use shuyi_ui::{Game, GameHooks, RawInputEvent};

/// The toy game showing the dot's stage alone.
fn stage() -> Harness<Toy> {
    Harness::new(Game::new(
        toy::ctx(),
        Box::new(DotStage::new()),
        GameHooks::default(),
    ))
}

fn dot(h: &Harness<Toy>) -> &DotStage {
    h.screen::<DotStage>()
        .unwrap_or_else(|| panic!("no dot stage: {:?}", h.screens()))
}

/// The keys of the dot's directions, in the toy's right-handed layout, by
/// the action they press.
const KEYS: [(Key, Action); 4] = [
    (Key::Left, Action::CursorLeft),
    (Key::Down, Action::CursorDown),
    (Key::Up, Action::CursorUp),
    (Key::Right, Action::CursorRight),
];

/// The key `key` going down.
fn down(key: Key) -> RawInputEvent {
    RawInputEvent::Down(Chord::plain(key))
}

/// The key `key` coming up.
fn up(key: Key) -> RawInputEvent {
    RawInputEvent::Up(key)
}

/// Whether an edge is a press or a release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Edge {
    Press,
    Release,
}

/// Every press and release in `record`, with its step (from 1): in each
/// step its releases, then its presses.
fn edges(record: &TickRecord) -> Vec<(u64, Edge, Action)> {
    let action = |id| Action::from_id(id).unwrap_or_else(|| panic!("no action {id:?}"));
    let mut edges = Vec::new();
    for (step, input) in (1..).zip(record.inputs()) {
        let Some(seat) = input.seat(0) else { continue };
        edges.extend(
            seat.released
                .iter()
                .map(|&id| (step, Edge::Release, action(id))),
        );
        edges.extend(
            seat.pressed
                .iter()
                .map(|&id| (step, Edge::Press, action(id))),
        );
    }
    edges
}

/// The edges alone, without their steps.
fn order(edges: &[(u64, Edge, Action)]) -> Vec<(Edge, Action)> {
    edges
        .iter()
        .map(|&(_, edge, action)| (edge, action))
        .collect()
}

/// A key going down (`true`) or up at a time, in milliseconds.
type Script = [(u32, Key, bool)];

/// What a run of a script at some frame rate gave: each press and release
/// with its step, and for each of the script's events the first step run
/// in or after the frame that read it.
struct Played {
    edges: Vec<(u64, Edge, Action)>,
    first_steps: Vec<u64>,
}

/// Plays `script` on the dot's stage in frames of `frames` milliseconds
/// (cycled): each event with the first frame at or after its time, then a
/// quarter of a second of steps.
fn play(script: &Script, frames: &[f64]) -> Played {
    let mut h = stage();
    let mut now = 0.0;
    let mut next = 0;
    let mut waiting: Vec<usize> = Vec::new();
    let mut first_steps = vec![0; script.len()];
    for &dt in frames.iter().cycle() {
        if next == script.len() {
            break;
        }
        now += dt;
        let mut events = Vec::new();
        while next < script.len() && f64::from(script[next].0) <= now {
            let (_, key, is_down) = script[next];
            events.push(if is_down { down(key) } else { up(key) });
            waiting.push(next);
            next += 1;
        }
        let before = dot(&h).driver.steps();
        h.frame(&events, Duration::from_secs_f64(dt / 1000.0).as_secs_f32());
        if dot(&h).driver.steps() > before {
            for event in waiting.drain(..) {
                first_steps[event] = before + 1;
            }
        }
    }
    // Enough steps for every edge left waiting: two a step at the least.
    let before = dot(&h).driver.steps();
    h.steps(u32::try_from(script.len()).unwrap_or(u32::MAX) + 4);
    for event in waiting.drain(..) {
        first_steps[event] = before + 1;
    }
    Played {
        edges: edges(dot(&h).record()),
        first_steps,
    }
}

/// What `script` presses and releases, in order.
fn script_edges(script: &Script) -> Vec<(Edge, Action)> {
    let action = |key| KEYS.iter().find(|(k, _)| *k == key).map(|&(_, a)| a);
    script
        .iter()
        .filter_map(|&(_, key, is_down)| {
            let edge = if is_down { Edge::Press } else { Edge::Release };
            action(key).map(|a| (edge, a))
        })
        .collect()
}

/// The sweep's script: holds, a chord, taps and a release of two at once,
/// events of different times at least 100 ms apart.
const SWEEP: &Script = &[
    (0, Key::Right, true),
    (250, Key::Right, false),
    (400, Key::Up, true),
    (400, Key::Left, true),
    (500, Key::Up, false),
    (650, Key::Left, false),
    (800, Key::Down, true),
    (800, Key::Down, false),
    (1000, Key::Right, true),
    (1100, Key::Right, false),
    (1250, Key::Right, true),
    (1250, Key::Right, false),
    (1500, Key::Down, true),
    (1600, Key::Up, true),
    (1750, Key::Down, false),
    (1750, Key::Up, false),
];

/// Frame times between 4 and 40 milliseconds, the same each run.
fn jittered() -> Vec<f64> {
    let mut seed: u32 = 2137;
    (0..97)
        .map(|_| {
            seed = seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            4.0 + f64::from((seed >> 16) % 37)
        })
        .collect()
}

#[test]
fn one_script_gives_the_same_presses_and_releases_at_every_frame_rate() {
    let mut rates: Vec<(String, Vec<f64>)> = [30.0, 60.0, 61.0, 144.0, 240.0]
        .into_iter()
        .map(|hz: f64| (format!("{hz} Hz"), vec![1000.0 / hz]))
        .collect();
    rates.push(("jittered".to_owned(), jittered()));
    let expected = script_edges(SWEEP);
    for (name, frames) in rates {
        let played = play(SWEEP, &frames);
        assert_eq!(order(&played.edges), expected, "at {name}");
        // Each press in the first step run in or after the frame that read
        // it, and each release after its press.
        let mut presses = played.edges.iter().filter(|e| e.1 == Edge::Press);
        for (i, &(_, _, is_down)) in SWEEP.iter().enumerate() {
            if is_down {
                let step = presses.next().map(|e| e.0);
                assert_eq!(step, Some(played.first_steps[i]), "event {i} at {name}");
            }
        }
        for (i, &(step, edge, action)) in played.edges.iter().enumerate() {
            if edge == Edge::Release {
                let press = played.edges[..i]
                    .iter()
                    .rev()
                    .find(|e| e.2 == action && e.1 == Edge::Press);
                assert!(press.is_some_and(|p| p.0 < step), "{action:?} at {name}");
            }
        }
    }
}

/// A random script of key presses and releases: each event a gap after
/// the last, on one of the dot's keys, which goes down if it was up and up
/// if it was down.
fn any_script() -> impl Strategy<Value = Vec<(u32, Key, bool)>> {
    proptest::collection::vec((0..120_u32, 0..KEYS.len()), 0..40).prop_map(|moves| {
        let mut held = [false; 4];
        let mut now = 0;
        moves
            .into_iter()
            .map(|(gap, key)| {
                now += gap;
                held[key] = !held[key];
                (now, KEYS[key].0, held[key])
            })
            .collect()
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn no_press_or_release_is_lost_or_doubled_at_any_frame_times(
        script in any_script(),
        frames in proptest::collection::vec(0..=100_u32, 1..30),
    ) {
        // Frames that all take no time never reach a later event.
        prop_assume!(frames.iter().any(|&ms| ms > 0));
        let frames: Vec<f64> = frames.into_iter().map(f64::from).collect();
        let played = play(&script, &frames);
        // Each action's presses and releases in the order the keys gave
        // them, none lost and none twice.
        for &(_, action) in &KEYS {
            let of = |list: &[(Edge, Action)]| -> Vec<Edge> {
                list.iter().filter(|e| e.1 == action).map(|e| e.0).collect()
            };
            prop_assert_eq!(of(&order(&played.edges)), of(&script_edges(&script)));
        }
        // At most one press of an action a step, and a release always in
        // a step after its press's.
        for (i, &(step, edge, action)) in played.edges.iter().enumerate() {
            let earlier = played.edges[..i].iter().rev().find(|e| e.2 == action);
            match edge {
                Edge::Press => prop_assert!(earlier.is_none_or(|e| e.1 == Edge::Release)),
                Edge::Release => prop_assert!(earlier.is_some_and(|e| e.1 == Edge::Press && e.0 < step)),
            }
        }
    }
}

/// Runs one frame of `dt` seconds with `events` and says how many steps
/// were due in it.
fn frame(h: &mut Harness<Toy>, events: &[RawInputEvent], dt: f32) -> u32 {
    h.frame(events, dt);
    h.app().ticks()
}

#[test]
fn a_tap_on_a_frame_with_no_step_is_pressed_in_the_next_step_and_released_after() {
    let mut h = stage();
    h.steps(3);
    assert_eq!(frame(&mut h, &[down(Key::Up), up(Key::Up)], 0.004), 0);
    h.steps(2);
    let edges = edges(dot(&h).record());
    assert_eq!(
        edges,
        [
            (4, Edge::Press, Action::CursorUp),
            (5, Edge::Release, Action::CursorUp)
        ]
    );
    // The tap moved the dot a pixel up, and it stopped.
    assert_eq!((dot(&h).dot.x, dot(&h).dot.y), (0, -1));
    assert_eq!(h.sounds(), [STOP]);
}

#[test]
fn a_press_on_a_late_frame_of_four_steps_is_pressed_in_the_first_only() {
    let mut h = stage();
    assert_eq!(frame(&mut h, &[down(Key::Right)], 0.1), 4);
    let record = dot(&h).record();
    let inputs: Vec<_> = record.inputs().collect();
    assert_eq!(inputs.len(), 4);
    let right = Action::CursorRight.id();
    assert!(inputs[0].is_pressed(0, right));
    assert!(
        inputs[1..]
            .iter()
            .all(|i| !i.is_pressed(0, right) && i.is_held(0, right))
    );
    assert_eq!(dot(&h).dot.x, 4);
}

#[test]
fn a_held_keys_repeats_are_never_pressed_in_a_step() {
    let mut h = stage();
    h.hold("Right", 1.0);
    h.steps(2);
    let edges = edges(dot(&h).record());
    assert_eq!(
        order(&edges),
        [
            (Edge::Press, Action::CursorRight),
            (Edge::Release, Action::CursorRight)
        ]
    );
    // It sped up as it was held, then stopped once.
    assert_eq!(h.sounds(), [STOP]);
}

#[test]
fn an_action_handed_in_by_act_reaches_the_next_step() {
    let mut h = stage();
    h.steps(2);
    // `act` hands the action in, then runs a frame of one step.
    h.act(Action::CursorDown);
    h.steps(1);
    let edges = edges(dot(&h).record());
    assert_eq!(
        edges,
        [
            (3, Edge::Press, Action::CursorDown),
            (4, Edge::Release, Action::CursorDown)
        ]
    );
}

#[test]
fn a_press_kept_for_a_step_never_reaches_one_after_the_stage_is_covered() {
    let mut h = stage();
    h.steps(3);
    // Right goes down on a frame with no step, and Cancel opens the menu.
    assert_eq!(frame(&mut h, &[down(Key::Right)], 0.002), 0);
    assert_eq!(frame(&mut h, &[down(Key::D)], 0.002), 0);
    assert_eq!(h.screens(), ["dot", "menu"]);
    // Let go under the menu, which then closes.
    h.frame(&[up(Key::Right), up(Key::D)], FRAME_DT);
    h.keys("d");
    assert_eq!(h.screens(), ["dot"]);
    h.steps(10);
    let edges = edges(dot(&h).record());
    assert!(edges.is_empty(), "{edges:?}");
    assert_eq!((dot(&h).dot.x, dot(&h).dot.y), (0, 0));
}

#[test]
fn a_press_kept_for_a_step_never_reaches_one_after_a_suspend() {
    let mut h = stage();
    h.steps(3);
    assert_eq!(frame(&mut h, &[down(Key::Left), up(Key::Left)], 0.002), 0);
    h.system(SystemEvent::Suspend);
    h.system(SystemEvent::Resume);
    h.steps(5);
    let edges = edges(dot(&h).record());
    assert!(edges.is_empty(), "{edges:?}");
    assert_eq!(dot(&h).dot.x, 0);
}

/// Runs a frame of one step with `events`.
fn one_step(h: &mut Harness<Toy>, events: &[RawInputEvent]) {
    assert_eq!(frame(h, events, FRAME_DT), 1);
}

/// The frames of steps 6 to 10 of the rewind test: Up tapped, a step,
/// Left tapped, two steps. The dot stops at steps 7 and 9.
fn steps_six_to_ten(h: &mut Harness<Toy>) {
    one_step(h, &[down(Key::Up), up(Key::Up)]);
    one_step(h, &[]);
    one_step(h, &[down(Key::Left), up(Key::Left)]);
    one_step(h, &[]);
    one_step(h, &[]);
}

#[test]
fn a_rewind_runs_the_steps_again_and_hands_their_stops_on_once() {
    let mut h = stage();
    // Right held for steps 1 to 3: it stops at step 4.
    one_step(&mut h, &[down(Key::Right)]);
    one_step(&mut h, &[]);
    one_step(&mut h, &[]);
    one_step(&mut h, &[up(Key::Right)]);
    one_step(&mut h, &[]);
    steps_six_to_ten(&mut h);
    let stops: Vec<u32> = h.audio_log().iter().map(|&(frame, _)| frame).collect();
    assert_eq!(stops, [4, 7, 9]);
    let at_ten = dot(&h).dot.clone();
    // Back to step 5, and steps 6 to 10 again: their stops aren't played
    // again, and the dot ends where it did.
    h.clear_audio();
    h.screen_mut::<DotStage>()
        .unwrap_or_else(|| panic!("no dot stage"))
        .rewind_to(5);
    assert_eq!((dot(&h).dot.x, dot(&h).driver.steps()), (3, 5));
    steps_six_to_ten(&mut h);
    assert!(h.audio_log().is_empty(), "{:?}", h.audio_log());
    assert_eq!(dot(&h).dot, at_ten);
    assert_eq!(dot(&h).record().len(), 10);
    // Step 11 is new: its stop, at step 12, plays.
    one_step(&mut h, &[down(Key::Down), up(Key::Down)]);
    one_step(&mut h, &[]);
    assert_eq!(h.sounds(), [STOP]);
}

#[test]
fn the_harness_steps_the_dot_headlessly() {
    let mut h = stage();
    let mut lines = Vec::new();
    let mut note = |h: &Harness<Toy>| {
        let stage = dot(h);
        lines.push(format!("step {:02}: {}", stage.driver.steps(), stage.dot));
    };
    h.frame(&[down(Key::Right)], FRAME_DT);
    h.steps(19);
    note(&h);
    h.frame(&[down(Key::Down)], FRAME_DT);
    h.steps(4);
    note(&h);
    h.frame(&[up(Key::Right)], FRAME_DT);
    h.steps(2);
    note(&h);
    h.frame(&[up(Key::Down)], FRAME_DT);
    h.steps(3);
    note(&h);
    assert_eq!(dot(&h).driver.steps(), 32);
    lines.push(format!("runs: {}", dot(&h).record().runs().len()));
    assert_snapshot!("the_dot_after_32_steps", lines.join("\n"));
}
