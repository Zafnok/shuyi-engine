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

/// TEMPORARY (ticket in progress): CI's numbers for each measurement.
#[test]
fn zz_measure() {
    let (atlas, bits) = test_font();
    let font = Font::new(&atlas, &bits).unwrap();
    let fight = Fight::new();
    let buf = Fight::frame(37);
    let mut frame = Box::new([0u8; FRAME_BYTES]);
    let mut plain = Box::new([0u8; FRAME_BYTES]);
    let mut fs = Vec::new();
    let mut ss = Vec::new();
    for i in 0..200 {
        let mut fast = || {
            let t = Instant::now();
            render(&buf, &font, &fight, &mut frame);
            t.elapsed().as_secs_f64() * 1e3
        };
        let f;
        let s;
        if i % 2 == 0 {
            f = fast();
            let t = Instant::now();
            reference::render(&buf, &font, &fight, &mut plain);
            s = t.elapsed().as_secs_f64() * 1e3;
        } else {
            let t = Instant::now();
            reference::render(&buf, &font, &fight, &mut plain);
            s = t.elapsed().as_secs_f64() * 1e3;
            f = fast();
        }
        fs.push(f);
        ss.push(s);
    }
    let mut cur = Vec::new();
    for _ in 0..10 {
        let (f, s) = time_fight(4);
        cur.push(s.as_secs_f64() / f.as_secs_f64());
    }
    cur.sort_by(f64::total_cmp);
    let mut r: Vec<f64> = fs.iter().zip(&ss).map(|(f, s)| s / f).collect();
    r.sort_by(f64::total_cmp);
    fs.sort_by(f64::total_cmp);
    ss.sort_by(f64::total_cmp);
    let q = |v: &Vec<f64>| {
        let n = v.len();
        format!("{:.3}/{:.3}/{:.3}/{:.3}/{:.3}", v[0], v[n / 10], v[n / 2], v[n * 9 / 10], v[n - 1])
    };
    let r2 = |x: f64| (x * 100.0).round() / 100.0;
    panic!(
        "ZZMEASURE least-of-9x4 x10: {:?} | fast ms {} | plain ms {} | pair ratio {} | min/min {:.2} med/med {:.2} sum/sum {:.2}",
        cur.iter().map(|x| r2(*x)).collect::<Vec<_>>(),
        q(&fs),
        q(&ss),
        q(&r),
        ss[0] / fs[0],
        ss[100] / fs[100],
        ss.iter().sum::<f64>() / fs.iter().sum::<f64>()
    );
}
