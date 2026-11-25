//! Pictures of the board as `cargo shuyi frame-png` paints them, with the
//! game's font, checked as snapshots: a run's start, and the snake after
//! its first food. Review a change with `cargo insta review`; the `.png`
//! beside each `.snap` is the picture.

use shuyi::prelude::UiAction;
use shuyi_cli::BasicTools;
use shuyi_cli::codec::encode_png;
use snake::{Data, Own};

use crate::{at_title, eat_first_food, stage};

/// The frame `h` shows, as a PNG.
fn png(h: &shuyi::platform::harness::Harness<Data>) -> Vec<u8> {
    let image = BasicTools::<UiAction, Own>::picture(h, 1).unwrap_or_else(|e| panic!("{e}"));
    encode_png(image.width, image.height, &image.rgba).unwrap_or_else(|e| panic!("{e}"))
}

#[test]
fn the_start_of_a_run() {
    let mut h = at_title(1);
    h.act(UiAction::Confirm);
    assert_eq!(stage(&h).snake.length(), 3);
    insta::assert_binary_snapshot!("start.png", png(&h));
}

#[test]
fn the_snake_after_its_first_food() {
    let mut h = at_title(1);
    h.act(UiAction::Confirm);
    eat_first_food(&mut h);
    // The move after it ate: four long.
    while stage(&h).snake.length() < 4 {
        h.wait(crate::FRAME);
    }
    assert_eq!(stage(&h).snake.score(), 1);
    insta::assert_binary_snapshot!("first_food.png", png(&h));
}
