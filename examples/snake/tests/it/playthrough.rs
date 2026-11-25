//! A run of seed 1 through actions only: from the title to the first food,
//! the snake grown, a wall hit, and a new run from the end screen.

use shuyi::prelude::UiAction;
use snake::rules::{Event, Outcome};
use snake::screens::{DIE, EAT};

use crate::{playthrough, stage};

#[test]
fn a_run_eats_its_first_food_grows_dies_on_a_wall_and_starts_again() {
    let (mut h, record) = playthrough();
    let run = stage(&h);
    let snake = &run.snake;
    assert_eq!(run.seed, 1);
    // It ate once, grew by one at the move after, and hit the board's
    // wall: its head is on a wall's neighbour.
    assert_eq!(snake.score(), 1);
    assert_eq!(snake.length(), 4);
    let ate = run
        .events
        .iter()
        .position(|e| *e == Event::Ate { score: 1 });
    let ate = ate.unwrap_or_else(|| panic!("{:?}", run.events));
    assert!(
        matches!(
            run.events[ate + 1],
            Event::Moved { .. } | Event::Turned { .. }
        ),
        "{:?}",
        &run.events[ate..]
    );
    assert_eq!(run.events.last(), Some(&Event::Died));
    let Some(Outcome::Died { step, length: 4 }) = snake.outcome() else {
        panic!("{:?}", snake.outcome());
    };
    assert_eq!(step, snake.steps());
    // One step of the record for each step of the rules.
    assert_eq!(record.len(), step);
    assert_eq!(record.seed(), 1);
    // Each sound played once, on its event.
    let sounds = h.sounds();
    let count = |id: &str| sounds.iter().filter(|s| s.as_str() == id).count();
    assert_eq!((count(EAT), count(DIE)), (1, 1), "{sounds:?}");

    // Confirm on the end screen: a new run, the next seed's.
    h.act(UiAction::Confirm);
    assert_eq!(h.screens(), ["title", "stage"]);
    let again = stage(&h);
    assert_eq!(
        (again.seed, again.snake.score(), again.snake.length()),
        (2, 0, 3)
    );
}

/// Writes the golden test's record (`tests/golden.rs`) from the
/// playthrough.
#[test]
#[ignore = "writes tests/records/seed-1.ron"]
fn write_the_golden_record() {
    let (_, record) = playthrough();
    let config = ron::ser::PrettyConfig::default().new_line("\n");
    let text = ron::ser::to_string_pretty(&record, config).unwrap_or_else(|e| panic!("{e}"));
    let path = crate::root().join("tests/records/seed-1.ron");
    std::fs::write(&path, text + "\n").unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

#[test]
fn the_golden_record_is_the_playthroughs() {
    let (_, record) = playthrough();
    let text = include_str!("../records/seed-1.ron");
    let checked_in: shuyi::core::tick::TickRecord =
        ron::from_str(text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        checked_in.runs(),
        record.runs(),
        "run `cargo test -p snake --test it -- --ignored write_the_golden_record`"
    );
}

#[test]
fn cancel_pauses_the_run_and_the_menu_resumes_it_or_goes_back_to_the_title() {
    let mut h = crate::at_title(1);
    h.act(UiAction::Confirm);
    h.wait(0.5);
    h.act(UiAction::Cancel);
    assert_eq!(h.screens(), ["title", "stage", "pause"]);
    // Paused, the snake waits.
    let steps = stage(&h).snake.steps();
    h.wait(1.0);
    assert_eq!(stage(&h).snake.steps(), steps);
    // A turn pressed in the menu moves its focus, not the snake.
    h.act(UiAction::Down);
    h.act(UiAction::Up);
    h.act(UiAction::Confirm);
    assert_eq!(h.screens(), ["title", "stage"]);
    let waiting = stage(&h).snake.turns();
    assert!(waiting.is_empty(), "{waiting:?}");
    h.wait(0.1);
    assert!(stage(&h).snake.steps() > steps);
    // Down to "Back to the title".
    h.act(UiAction::Cancel);
    h.act(UiAction::Down);
    h.act(UiAction::Confirm);
    assert_eq!(h.screens(), ["title"]);
}

#[test]
fn a_run_back_from_a_suspend_waits_in_the_pause_menu() {
    use shuyi::platform::SystemEvent;

    let mut h = crate::at_title(1);
    h.act(UiAction::Confirm);
    h.wait(0.2);
    h.system(SystemEvent::Suspend);
    h.system(SystemEvent::Resume);
    h.wait(0.1);
    assert_eq!(h.screens(), ["title", "stage", "pause"]);
}
