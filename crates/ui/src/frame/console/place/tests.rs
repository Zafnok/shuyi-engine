use super::*;

const WIDE: (u32, u32) = (800, 512);

/// A placement of `scale` (a whole number, or `num / den`) drawn at
/// `draw`, `w × h` at `(x, y)`.
fn at(scale: Factor, draw: u32, (w, h): (u32, u32), (x, y): (i32, i32)) -> Placement {
    Placement {
        scale,
        draw_scale: draw,
        x,
        y,
        w,
        h,
    }
}

fn whole(n: u32, size: (u32, u32), xy: (i32, i32)) -> Placement {
    at(Factor::whole(n), n, size, xy)
}

#[test]
fn an_800_by_512_frame_in_the_common_windows() {
    let rows = [
        // window, Integer, Fit
        (
            (1280, 720),
            whole(1, WIDE, (240, 104)),
            at(Factor { num: 720, den: 512 }, 2, (1125, 720), (77, 0)),
        ),
        (
            (1280, 800),
            whole(1, WIDE, (240, 144)),
            at(Factor { num: 800, den: 512 }, 2, (1250, 800), (15, 0)),
        ),
        (
            (1366, 768),
            whole(1, WIDE, (283, 128)),
            at(Factor { num: 768, den: 512 }, 2, (1200, 768), (83, 0)),
        ),
        // The whole scale fills 95 % of the height: Fit snaps to it.
        (
            (1920, 1080),
            whole(2, (1600, 1024), (160, 28)),
            whole(2, (1600, 1024), (160, 28)),
        ),
        (
            (2560, 1440),
            whole(2, (1600, 1024), (480, 208)),
            at(
                Factor {
                    num: 1440,
                    den: 512,
                },
                3,
                (2250, 1440),
                (155, 0),
            ),
        ),
        (
            (3840, 2160),
            whole(4, (3200, 2048), (320, 56)),
            whole(4, (3200, 2048), (320, 56)),
        ),
    ];
    for (window, integer, fit) in rows {
        assert_eq!(
            place(ScaleMode::Integer, WIDE, window),
            integer,
            "{window:?}"
        );
        assert_eq!(place(ScaleMode::Fit, WIDE, window), fit, "{window:?}");
    }
}

#[test]
fn other_frames_and_windows() {
    // A square frame on a Steam Deck: 2× fills 96 % of the height, so Fit
    // snaps; on 1080p 2× is 71 %, so it fills.
    let square = (384, 384);
    let deck = whole(2, (768, 768), (256, 16));
    assert_eq!(place(ScaleMode::Integer, square, (1280, 800)), deck);
    assert_eq!(place(ScaleMode::Fit, square, (1280, 800)), deck);
    assert_eq!(
        place(ScaleMode::Fit, square, (1920, 1080)),
        at(
            Factor {
                num: 1080,
                den: 384
            },
            3,
            (1080, 1080),
            (420, 0)
        )
    );
    // 2.4× rounds up to 3, but 3 × 1600 passes 4096: drawn at 2.
    assert_eq!(
        place(ScaleMode::Fit, (1600, 1000), (3840, 2600)),
        at(
            Factor {
                num: 3840,
                den: 1600
            },
            2,
            (3840, 2400),
            (0, 100)
        )
    );
    // Too small: Integer crops at 1×, Fit shrinks.
    assert_eq!(
        place(ScaleMode::Integer, WIDE, (640, 400)),
        whole(1, WIDE, (-80, -56))
    );
    assert_eq!(
        place(ScaleMode::Fit, WIDE, (640, 400)),
        at(Factor { num: 400, den: 512 }, 1, (625, 400), (7, 0))
    );
    // A leftover odd pixel rounds down; a window a pixel short is -1.
    assert_eq!(place(ScaleMode::Integer, WIDE, (799, 512)).x, -1);
    assert_eq!(place(ScaleMode::Integer, WIDE, (803, 515)).x, 1);
}

#[test]
fn the_scale_of_each_mode() {
    let deck = (1280, 800);
    assert_eq!(
        scale(ScaleMode::Integer, (640, 400), deck),
        Factor::whole(2)
    );
    assert_eq!(
        scale(ScaleMode::Integer, WIDE, (400, 240)),
        Factor::whole(0)
    );
    assert_eq!(
        scale(ScaleMode::Fit, WIDE, deck),
        Factor { num: 800, den: 512 }
    );
    assert_eq!(
        scale(ScaleMode::Fit, WIDE, (400, 240)),
        Factor { num: 240, den: 512 }
    );
    // Snapped: 2 is 95 % of 1080 / 512.
    assert_eq!(scale(ScaleMode::Fit, WIDE, (1920, 1080)), Factor::whole(2));
    // A side of 0 counts as 1.
    assert_eq!(scale(ScaleMode::Fit, (0, 0), deck), Factor::whole(800));
    // Just at the snap: 9 / 10 of 10 / 9 is 1; just under it doesn't.
    assert_eq!(scale(ScaleMode::Fit, (9, 9), (10, 10)), Factor::whole(1));
    assert_eq!(
        scale(ScaleMode::Fit, (90, 90), (101, 101)),
        Factor { num: 101, den: 90 }
    );
    assert_eq!(
        scale(ScaleMode::Fit, (900, 900), (1000, 1000)),
        Factor::whole(1)
    );
    assert_eq!(
        scale(ScaleMode::Fit, (900, 900), (1001, 1001)),
        Factor {
            num: 1001,
            den: 900
        }
    );
}

#[test]
fn factors() {
    assert_eq!(Factor::whole(3).shown(), "3");
    assert_eq!(Factor { num: 4, den: 2 }.shown(), "2");
    assert_eq!(Factor { num: 800, den: 512 }.shown(), "1.56");
    assert_eq!(Factor { num: 800, den: 512 }.of(512), 800);
    assert_eq!(Factor { num: 720, den: 512 }.floor(), 1);
    assert_eq!(Factor { num: 720, den: 512 }.ceil(), 2);
    assert_eq!(Factor::whole(2).ceil(), 2);
    assert!(Factor { num: 4, den: 2 }.is_whole());
    assert!(!Factor { num: 3, den: 2 }.is_whole());
    assert_eq!(ScaleMode::default(), ScaleMode::Integer);
    assert_eq!(ScaleMode::ALL, [ScaleMode::Integer, ScaleMode::Fit]);
}

#[test]
fn the_modes_are_named_as_shuyi_ron_names_them() {
    assert_eq!(ron::to_string(&ScaleMode::Fit).unwrap(), "fit");
    assert_eq!(
        ron::from_str::<ScaleMode>("integer").unwrap(),
        ScaleMode::Integer
    );
}

#[test]
fn a_fit_placement_maps_the_window_back_to_frame_pixels() {
    let placed = place(ScaleMode::Fit, WIDE, (1280, 720));
    let px = |x, y| placed.frame_px(WIDE, (x, y));
    assert_eq!(px(77.0, 0.0), Some((0, 0)));
    // 1.40625 window pixels a frame pixel.
    assert_eq!(px(77.0 + 1.40, 1.40), Some((0, 0)));
    assert_eq!(px(77.0 + 1.41, 1.41), Some((1, 1)));
    assert_eq!(px(77.0 + 1124.9, 719.9), Some((799, 511)));
    assert_eq!(px(76.9, 10.0), None);
    assert_eq!(px(77.0 + 1125.0, 10.0), None);
    assert_eq!(px(f32::NAN, 10.0), None);
    assert!(!placed.is_whole(WIDE));
    assert!(place(ScaleMode::Fit, WIDE, (1920, 1080)).is_whole(WIDE));
}

proptest::proptest! {
    #[test]
    fn fit_fills_the_window_and_never_loses_to_integer(
        frame in (1u32..=2000, 1u32..=2000),
        window in (0u32..=8000, 0u32..=8000),
    ) {
        let fit = place(ScaleMode::Fit, frame, window);
        let integer = place(ScaleMode::Integer, frame, window);
        proptest::prop_assert_eq!(fit, place(ScaleMode::Fit, frame, window));
        proptest::prop_assert!(fit.draw_scale >= 1);
        let side = u64::from(frame.0.max(frame.1));
        if fit.is_whole(frame) {
            // Snapped, or the fraction came out whole: Integer's placement.
            proptest::prop_assert_eq!(fit, integer);
        } else {
            // Its picture, drawn before the stretch, stays within MAX_DRAW_PX.
            proptest::prop_assert!(
                fit.draw_scale == 1 || side * u64::from(fit.draw_scale) <= u64::from(MAX_DRAW_PX)
            );
            // Inside the window, and equal to it on one side.
            proptest::prop_assert!(fit.x >= 0 && fit.y >= 0);
            proptest::prop_assert!(fit.w <= window.0 && fit.h <= window.1);
            proptest::prop_assert!(fit.w == window.0 || fit.h == window.1);
            if scale(ScaleMode::Integer, frame, window).num >= 1 {
                proptest::prop_assert!(fit.w >= integer.w && fit.h >= integer.h);
                // It filled because the whole scale was under SNAP of it.
                proptest::prop_assert!(u64::from(integer.h) * 10 < u64::from(fit.h) * 9 + 10);
            }
        }
    }
}
