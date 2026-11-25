//! The snake moves by the rules' steps, never by the frame's time: left
//! alone for a second, it moves as far at 30 frames a second as at 60.

use shuyi::prelude::UiAction;

use crate::{at_title, stage};

/// How far the snake moved in one second of frames of `1 / rate` s with no
/// input, from the start of a run, and the steps the rules ran.
fn moved_in_a_second(rate: u16) -> (i32, u64) {
    let mut h = at_title(1);
    h.act(UiAction::Confirm);
    let (from, steps) = (stage(&h).snake.head(), stage(&h).snake.steps());
    for _ in 0..rate {
        h.frame(&[], 1.0 / f32::from(rate));
    }
    let snake = &stage(&h).snake;
    (snake.head().x - from.x, snake.steps() - steps)
}

#[test]
fn left_alone_it_moves_as_far_in_a_second_at_30_and_at_60_frames() {
    let at_60 = moved_in_a_second(60);
    assert_eq!(moved_in_a_second(30), at_60);
    // 60 steps, a move every 8: 7 tiles.
    assert_eq!(at_60, (7, 60));
}
