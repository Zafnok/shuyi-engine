//! How fast the fast renderer draws (ticket 3056): timed by the clock,
//! so only with `std`.
#![cfg(feature = "std")]

use std::time::{Duration, Instant};

use super::*;

/// How many pairs of renders [`time_fight`] times.
const PAIRS: usize = 51;

/// The least speed-up that passes, in hundredths. A debug build, which
/// every CI job runs, measures 4.0 to 5.6 times on CI's runners and 4.4
/// to 5.6 on a PC, and a fast renderer that lost its byte path 1 or less
/// (ticket 2396's notes); an optimised build measures 13, against 3056's
/// 4.
const LEAST: u128 = if cfg!(debug_assertions) { 300 } else { 400 };

/// How long the fight's frame takes to draw, fast and plain, and how many
/// times faster the fast renderer is, in hundredths: the medians of
/// [`PAIRS`] pairs of renders (ticket 2396). A pair is one render of
/// each, back to back, the fast one first in every other pair, so a
/// runner slowed for a while (other tests on its cores, its clock's
/// frequency) slows both, and the median drops the pairs a pause hit.
fn time_fight() -> (Duration, Duration, u128) {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let fight = Fight::new();
    let buf = Fight::frame(37);
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    let mut time = |plain: bool| {
        let start = Instant::now();
        if plain {
            reference::render(&buf, &font, &fight, &mut frame);
        } else {
            render(&buf, &font, &fight, &mut frame);
        }
        start.elapsed()
    };
    let (mut fast, mut slow, mut speed_ups) = (Vec::new(), Vec::new(), Vec::new());
    for pair in 0..PAIRS {
        let (f, s) = if pair % 2 == 0 {
            let f = time(false);
            (f, time(true))
        } else {
            let s = time(true);
            (time(false), s)
        };
        speed_ups.push(s.as_nanos() * 100 / f.as_nanos().max(1));
        fast.push(f);
        slow.push(s);
    }
    for times in [&mut fast, &mut slow] {
        times.sort_unstable();
    }
    speed_ups.sort_unstable();
    (fast[PAIRS / 2], slow[PAIRS / 2], speed_ups[PAIRS / 2])
}

/// The fight's frame draws much faster than the plain renderer draws it
/// (ticket 3056).
#[test]
fn a_fight_draws_fast() {
    let (fast, slow, speed_up) = time_fight();
    assert!(
        speed_up >= LEAST,
        "{}.{:02} times: {fast:?} fast against {slow:?} plain, the medians of {PAIRS} pairs",
        speed_up / 100,
        speed_up % 100,
    );
}
