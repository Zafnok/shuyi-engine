//! A record's runs, its text, a replay and its stamp, and the toy dot's
//! rules; the property tests replay random records and check that the
//! runs hold every step.

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use proptest::prelude::*;

use super::*;
use crate::grid::Dir;
use crate::made::{ContentHash, Part};
use crate::toy::{DOWN, Dot, DotEvent, FASTER_EVERY, LEFT, RIGHT, TOP_SPEED, UP};

/// A stamp of no particular build.
fn made() -> Made {
    Made::new("dot 1.0", ContentHash(7))
}

/// Seat 0 holding `held` and pressing `pressed`.
fn seat(pressed: &[ActionId], held: &[ActionId]) -> TickInput {
    TickInput::one(SeatInput::new(pressed.to_vec(), held.to_vec(), vec![]))
}

/// A record of `inputs`, in order.
fn record_of(inputs: &[TickInput]) -> TickRecord {
    let mut record = TickRecord::new(made(), 1);
    for input in inputs {
        record.push(input.clone());
    }
    record
}

#[test]
fn a_seat_says_what_it_pressed_held_and_released() {
    let input = TickInput::one(SeatInput::new(vec![UP], vec![UP, LEFT], vec![DOWN]));
    assert!(input.is_pressed(0, UP) && !input.is_pressed(0, LEFT));
    assert!(input.is_held(0, LEFT) && !input.is_held(0, DOWN));
    assert!(input.is_released(0, DOWN) && !input.is_released(0, UP));
    // No seat 1: nothing of it.
    assert!(!input.is_held(1, UP));
    assert!(input.seat(1).is_none());
    assert!(SeatInput::default().is_empty());
    assert!(!input.seats[0].is_empty());
}

#[test]
fn an_input_holds_at_most_four_seats() {
    let seats = vec![SeatInput::default(); MAX_SEATS + 2];
    assert_eq!(TickInput::new(seats).seats.len(), MAX_SEATS);
}

#[test]
fn steps_in_a_row_with_one_input_are_one_run() {
    let idle = TickInput::one(SeatInput::default());
    let right = seat(&[], &[RIGHT]);
    let record = record_of(&[
        idle.clone(),
        right.clone(),
        right.clone(),
        right.clone(),
        idle,
    ]);
    let counts: Vec<u32> = record.runs().iter().map(|run| run.steps).collect();
    assert_eq!(counts, [1, 3, 1]);
    assert_eq!(record.len(), 5);
    assert_eq!(record.inputs().filter(|&i| *i == right).count(), 3);
    assert!(!record.is_empty());
    assert!(TickRecord::new(made(), 1).is_empty());
}

#[test]
fn a_truncated_record_keeps_its_first_steps() {
    let right = seat(&[], &[RIGHT]);
    let left = seat(&[], &[LEFT]);
    let whole = record_of(&[right.clone(), right.clone(), left.clone(), left.clone()]);
    let mut record = whole.clone();
    record.truncate(3);
    assert_eq!(
        record,
        record_of(&[right.clone(), right.clone(), left.clone()])
    );
    record.truncate(2);
    assert_eq!(record.runs().len(), 1);
    record.truncate(10);
    assert_eq!(record.len(), 2);
    record.truncate(0);
    assert!(record.is_empty());
    // A step pushed after it joins the run before it.
    let mut record = whole;
    record.truncate(1);
    record.push(right);
    assert_eq!(record.runs().len(), 1);
}

#[test]
fn a_record_is_written_as_its_stamp_seed_and_runs_and_read_back() {
    let made =
        ron::from_str::<Made>(r#"(engine:"0.1.0",game:"dot 1.0",content:"0000000000000007")"#)
            .unwrap_or_else(|e| panic!("{e}"));
    let mut record = TickRecord::new(made, 42);
    record.push(seat(&[RIGHT], &[RIGHT]));
    record.push(seat(&[], &[RIGHT]));
    record.push(seat(&[], &[RIGHT]));
    record.push(TickInput::one(SeatInput::new(vec![], vec![], vec![RIGHT])));
    record.push(TickInput::one(SeatInput::default()));
    let text = ron::to_string(&record).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        text,
        "(made:(engine:\"0.1.0\",game:\"dot 1.0\",content:\"0000000000000007\"),seed:42,\
         runs:[(steps:1,input:(seats:[(pressed:[3],held:[3])])),\
         (steps:2,input:(seats:[(held:[3])])),\
         (steps:1,input:(seats:[(released:[3])])),\
         (steps:1,input:(seats:[()]))])"
    );
    let back: TickRecord = ron::from_str(&text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(back, record);
    // A record of no step leaves its runs out.
    let empty = TickRecord::new(back.made().clone(), 0);
    let text = ron::to_string(&empty).unwrap_or_else(|e| panic!("{e}"));
    assert!(!text.contains("runs"), "{text}");
    assert_eq!(ron::from_str::<TickRecord>(&text).ok(), Some(empty));
    // A field it doesn't know is refused.
    assert!(ron::from_str::<SeatInput>("(pointer:[1])").is_err());
}

#[test]
fn a_replay_gives_a_hash_after_each_step() {
    let right = seat(&[RIGHT], &[RIGHT]);
    let record = record_of(&[right.clone(), right, TickInput::default()]);
    let mut sim = Dot::default();
    let hashes = replay(&mut sim, &record, &made()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(hashes.len(), 3);
    // The state is left at the record's end: two pixels right, stopped.
    assert_eq!((sim.x, sim.heading()), (2, None));
    assert_eq!(hashes[2], sim.hash());
    assert_ne!(hashes[0], hashes[1]);
}

#[test]
fn a_replay_refuses_a_record_of_another_build_naming_both() {
    let record = record_of(&[seat(&[RIGHT], &[RIGHT])]);
    let mut now = made();
    now.content = ContentHash(8);
    let mut sim = Dot::default();
    let refused = replay(&mut sim, &record, &now).unwrap_err();
    assert_eq!(refused.parts.len(), 1);
    assert_eq!(refused.parts[0].part, Part::Content);
    assert_eq!(
        alloc::format!("{refused}"),
        "made with content 0000000000000007; this build: content 0000000000000008"
    );
    // Nothing stepped.
    assert_eq!(sim, Dot::default());
}

/// Each event of a run, with its step.
type Events = Vec<(usize, DotEvent)>;

/// The dot's place after each step of `inputs`, and its events.
fn run(inputs: &[TickInput]) -> (Vec<(i32, i32)>, Events) {
    let mut dot = Dot::default();
    let mut places = Vec::new();
    let mut events = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        events.extend(dot.step(input).into_iter().map(|e| (i + 1, e)));
        places.push((dot.x, dot.y));
    }
    (places, events)
}

#[test]
fn the_dot_moves_a_pixel_a_step_while_held_and_stops_when_let_go() {
    let mut inputs = vec![seat(&[DOWN], &[DOWN])];
    inputs.extend(vec![seat(&[], &[DOWN]); 2]);
    inputs.push(TickInput::default());
    inputs.push(TickInput::default());
    let (places, events) = run(&inputs);
    assert_eq!(places, [(0, 1), (0, 2), (0, 3), (0, 3), (0, 3)]);
    assert_eq!(events, [(4, DotEvent::Stopped { x: 0, y: 3 })]);
}

#[test]
fn the_dot_speeds_up_every_eight_steps_held_up_to_four() {
    let mut dot = Dot::default();
    let mut speeds = Vec::new();
    dot.step(&seat(&[RIGHT], &[RIGHT]));
    speeds.push(dot.speed());
    for _ in 0..40 {
        dot.step(&seat(&[], &[RIGHT]));
        speeds.push(dot.speed());
    }
    let every = usize::try_from(FASTER_EVERY).unwrap_or(usize::MAX);
    assert!(speeds[..every].iter().all(|&s| s == 1), "{speeds:?}");
    assert_eq!(speeds[every], 2);
    assert_eq!(speeds[2 * every], 3);
    assert_eq!(speeds[3 * every], TOP_SPEED);
    assert_eq!(speeds.last(), Some(&TOP_SPEED));
    // A turn starts again at a pixel a step.
    dot.step(&seat(&[UP], &[RIGHT, UP]));
    assert_eq!((dot.heading(), dot.speed()), (Some(Dir::Up), 1));
}

#[test]
fn a_tap_moves_the_dot_one_pixel_and_it_stops_the_next_step() {
    let tap = seat(&[LEFT], &[]);
    let (places, events) = run(&[tap, TickInput::default()]);
    assert_eq!(places, [(-1, 0), (-1, 0)]);
    assert_eq!(events, [(2, DotEvent::Stopped { x: -1, y: 0 })]);
}

#[test]
fn the_dot_keeps_its_way_while_it_is_held_and_takes_the_first_held_after() {
    // Right, then Up held as well: it keeps going right.
    let (places, _) = run(&[seat(&[RIGHT], &[RIGHT]), seat(&[], &[UP, RIGHT])]);
    assert_eq!(places, [(1, 0), (2, 0)]);
    // Right let go with Up and Left held: the first held in the dot's
    // order, Left.
    let (places, _) = run(&[seat(&[RIGHT], &[RIGHT]), seat(&[], &[UP, LEFT])]);
    assert_eq!(places, [(1, 0), (0, 0)]);
    assert_eq!(alloc::format!("{}", Dot::default()), "at 0, 0, still");
    let mut dot = Dot::new(14, 0);
    dot.step(&seat(&[RIGHT], &[RIGHT]));
    assert_eq!(
        alloc::format!("{dot}"),
        "at 15, 0, heading Right at speed 1"
    );
}

/// A random step's input of seat 0: some of the dot's directions pressed,
/// held and released.
fn any_input() -> impl Strategy<Value = TickInput> {
    let ids = || proptest::collection::vec(0..6_u16, 0..3);
    (ids(), ids(), ids()).prop_map(|(pressed, held, released)| {
        let ids = |v: Vec<u16>| v.into_iter().map(ActionId).collect();
        TickInput::one(SeatInput::new(ids(pressed), ids(held), ids(released)))
    })
}

proptest! {
    #[test]
    fn replaying_a_record_twice_gives_the_same_hashes(
        inputs in proptest::collection::vec(any_input(), 0..200),
    ) {
        let record = record_of(&inputs);
        let first = replay(&mut Dot::default(), &record, &made());
        let second = replay(&mut Dot::default(), &record, &made());
        prop_assert_eq!(first.as_ref().map(Vec::len), Ok(inputs.len()));
        prop_assert_eq!(first, second);
    }

    #[test]
    fn a_record_gives_back_every_step_it_was_given(
        inputs in proptest::collection::vec(any_input(), 0..100),
        keep in 0..120_u64,
    ) {
        let mut record = record_of(&inputs);
        let back: Vec<TickInput> = record.inputs().cloned().collect();
        prop_assert_eq!(&back, &inputs);
        prop_assert!(record.runs().iter().all(|run| run.steps > 0));
        prop_assert!(record.runs().windows(2).all(|w| w[0].input != w[1].input));
        record.truncate(keep);
        let kept = usize::try_from(keep).unwrap_or(usize::MAX).min(inputs.len());
        let back: Vec<TickInput> = record.inputs().cloned().collect();
        prop_assert_eq!(back.as_slice(), &inputs[..kept]);
        let text = ron::to_string(&record).unwrap_or_else(|e| panic!("{e}"));
        let read: Result<TickRecord, String> = ron::from_str(&text).map_err(|e| alloc::format!("{e}"));
        prop_assert_eq!(read, Ok(record));
    }
}
