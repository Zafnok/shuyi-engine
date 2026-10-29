//! How fast the fast renderer draws (ticket 3056): timed by the clock,
//! so only with `std`.
#![cfg(feature = "std")]

use std::time::Instant;

use super::*;

/// How long the fight's frame takes to draw, fast and plain: the least
/// of a few tries of `rounds` renders each.
fn time_fight(rounds: u32) -> (core::time::Duration, core::time::Duration) {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let fight = Fight::new();
    let buf = Fight::frame(37);
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    let (mut fast, mut slow) = (core::time::Duration::MAX, core::time::Duration::MAX);
    for _ in 0..9 {
        let start = Instant::now();
        for _ in 0..rounds {
            render(&buf, &font, &fight, &mut frame);
        }
        fast = fast.min(start.elapsed() / rounds);
        let start = Instant::now();
        for _ in 0..rounds {
            reference::render(&buf, &font, &fight, &mut frame);
        }
        slow = slow.min(start.elapsed() / rounds);
    }
    (fast, slow)
}

/// The fight's frame draws much faster than the plain renderer draws it
/// (ticket 3056).
#[test]
fn a_fight_draws_fast() {
    let (fast, slow) = time_fight(4);
    assert!(fast * 4 <= slow, "{fast:?} fast against {slow:?} plain");
}
