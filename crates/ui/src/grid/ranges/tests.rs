use proptest::prelude::*;

use core::f32::consts::TAU;

use super::*;
use crate::grid::testing::any_scene;

const MOVE: RangeName = RangeName("move");
const ATTACK: RangeName = RangeName("attack");

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

const BLUE: Rgb = Rgb::new(0, 0, 200);
const RED: Rgb = Rgb::new(200, 0, 0);

fn color(kind: RangeName) -> Rgb {
    if kind == MOVE { BLUE } else { RED }
}

/// The game's pick (game ticket 0923): spreading, a glint, one outline,
/// a slab.
const SLAB: RangeShow = RangeShow::TINT
    .spreading()
    .with_wave(Wave::Glint)
    .outlined()
    .with_relief(Relief::Slab);

/// A 6 × 3 view from (10, 20): move on (11, 21) and (12, 21), attack on
/// (13, 21); opened from (11, 21) at 1000 ms; the clock at `clock_ms`.
fn row(clock_ms: u64) -> MapScene<()> {
    let mut scene = MapScene::new(p(10, 20), (6, 3));
    scene.tint([p(11, 21), p(12, 21)], MOVE);
    scene.tint([p(13, 21)], ATTACK);
    scene.ranges_shown = Some(RangesShown::new(p(11, 21), 1000));
    scene.clock_ms = clock_ms;
    scene
}

/// 16 × 16 pixel tiles, as the glyph skin's.
fn rect(at: (i32, i32)) -> PxRect {
    Rect::new(16 * at.0, 16 * at.1, 16, 16)
}

const ALL: PxRect = Rect::new(-100, -100, 1000, 1000);

fn overlays(show: RangeShow, scene: &MapScene<()>, at: (i32, i32), clip: PxRect) -> Vec<Overlay> {
    show.overlays(scene, &scene.tints(), at, rect(at), 1, 0.5, color, clip)
}

#[test]
fn the_tint_is_todays_look() {
    let show = RangeShow::TINT;
    assert!(show.is_tint());
    assert_eq!(RangeShow::default(), show);
    let scene = row(1000);
    // At full strength from the first moment, with nothing drawn over it.
    assert_eq!(show.strength(&scene, (1, 1)), OVERLAY_BLEND);
    assert_eq!(show.strength(&scene, (3, 1)), OVERLAY_BLEND);
    let none = overlays(show, &scene, (1, 1), ALL);
    assert!(none.is_empty(), "{none:?}");
    assert!(!SLAB.is_tint());
}

#[test]
fn ranges_spread_ring_by_ring_from_where_they_were_opened() {
    let show = RangeShow::TINT.spreading();
    let shown = Some(RangesShown::new(p(11, 21), 1000));
    let at = |pos, ms| show.spread_at(pos, shown, ms);
    // Its own tile: from nothing at 1000 ms to whole a fade later.
    assert_eq!(at(p(11, 21), 999), 0.0);
    assert_eq!(at(p(11, 21), 1000), 0.0);
    assert_eq!(at(p(11, 21), 1000 + SPREAD_FADE_MS / 2), 0.5);
    assert_eq!(at(p(11, 21), 1000 + SPREAD_FADE_MS), 1.0);
    assert_eq!(at(p(11, 21), 9000), 1.0);
    // Two tiles away (one across, one down): two steps later.
    let reached = 1000 + 2 * SPREAD_STEP_MS;
    assert_eq!(at(p(12, 20), reached - 1), 0.0);
    assert_eq!(at(p(10, 22), reached + SPREAD_FADE_MS / 2), 0.5);
    // A tile left of and above the start counts the same.
    assert_eq!(at(p(9, 21), reached + SPREAD_FADE_MS), 1.0);
}

#[test]
fn ranges_show_at_once_unless_spreading_and_opened() {
    let shown = Some(RangesShown::new(p(0, 0), 1000));
    assert_eq!(RangeShow::TINT.spread_at(p(5, 5), shown, 0), 1.0);
    assert_eq!(RangeShow::TINT.spreading().spread_at(p(5, 5), None, 0), 1.0);
    assert_eq!(
        RangeShow::TINT.spreading().spread_at(p(5, 5), shown, 0),
        0.0
    );
}

#[test]
fn the_strength_is_the_blend_times_the_spread() {
    let show = RangeShow::TINT.spreading().outlined();
    let half = 1000 + SPREAD_STEP_MS + SPREAD_FADE_MS / 2;
    // (12, 21) is one step from the start: half faded in.
    assert_eq!(show.strength(&row(half), (2, 1)), SHOWN_BLEND * 0.5);
    assert_eq!(show.strength(&row(5000), (2, 1)), SHOWN_BLEND);
    assert_eq!(show.strength(&row(1000), (2, 1)), 0.0);
}

#[test]
fn the_shimmer_runs_along_the_diagonal() {
    let show = RangeShow::TINT.with_wave(Wave::Shimmer);
    let blend = |at, ms| show.blend(at, (6, 3), ms);
    // A sine about the shown blend: a quarter wave in, at its top.
    assert_eq!(blend((0, 0), 0), SHOWN_BLEND);
    let top = blend((0, 0), WAVE_MS * 3 / 4);
    assert!((top - (SHOWN_BLEND + SHIMMER_DEPTH)).abs() < 1e-5, "{top}");
    let bottom = blend((0, 0), WAVE_MS / 4);
    assert!(
        (bottom - (SHOWN_BLEND - SHIMMER_DEPTH)).abs() < 1e-5,
        "{bottom}"
    );
    // A whole wave later, the same; a tile further along, a step behind.
    assert_eq!(
        blend((0, 0), WAVE_MS * 3 / 4),
        blend((0, 0), WAVE_MS * 7 / 4)
    );
    assert_eq!(blend((1, 0), 500), blend((0, 1), 500));
    let behind = SHOWN_BLEND + SHIMMER_DEPTH * (TAU * SHIMMER_PER_TILE).sin();
    assert!(
        (blend((1, 0), 0) - behind).abs() < 1e-6,
        "{}",
        blend((1, 0), 0)
    );
}

#[test]
fn the_glint_band_sweeps_across_the_view() {
    let show = RangeShow::TINT.with_wave(Wave::Glint);
    let size = (6, 3);
    // The band's middle runs over 6 + 3 diagonals plus 2.5 each side in a
    // wave: 14 in 2400 ms. At 2.5 / 14 of the wave it is on the first
    // tile, at full strength; the far corner (diagonal 7) is out of it.
    let first = WAVE_MS * 25 / 140;
    let peak = show.blend((0, 0), size, first);
    assert!((peak - (GLINT_BASE + GLINT_DEPTH)).abs() < 1e-3, "{peak}");
    assert_eq!(show.blend((5, 2), size, first), GLINT_BASE);
    // One diagonal on, weaker but still in the band.
    let near = show.blend((1, 0), size, first);
    assert!(near > GLINT_BASE && near < peak, "{near}");
    // Later it has moved on to the far corner.
    let last = WAVE_MS * 95 / 140;
    let far = show.blend((5, 2), size, last);
    assert!((far - (GLINT_BASE + GLINT_DEPTH)).abs() < 1e-3, "{far}");
    assert_eq!(show.blend((0, 0), size, last), GLINT_BASE);
}

#[test]
fn one_outline_goes_round_all_the_ranges_together() {
    let show = RangeShow::TINT.outlined();
    let scene = row(0);
    let light = |c: Rgb| c.lerp(Rgb::new(255, 255, 255), 0.5);
    // The first move tile: top, bottom and left face no range.
    let first = overlays(show, &scene, (1, 1), ALL);
    let r = rect((1, 1));
    let expect = [
        Overlay::new(Rect::new(r.x, r.y, 16, 1), light(BLUE), Layer::Under),
        Overlay::new(Rect::new(r.x, r.y + 15, 16, 1), light(BLUE), Layer::Under),
        Overlay::new(Rect::new(r.x, r.y, 1, 16), light(BLUE), Layer::Under),
    ];
    assert_eq!(first, expect);
    // The middle one: top and bottom only, none toward either neighbour.
    assert_eq!(overlays(show, &scene, (2, 1), ALL).len(), 2);
    // The attack tile, in its own color: none toward the move tile.
    let last = overlays(show, &scene, (3, 1), ALL);
    let r = rect((3, 1));
    assert_eq!(last.len(), 3);
    assert!(last.iter().all(|o| o.color == light(RED)));
    assert!(last.iter().all(|o| o.rect.x != r.x || o.rect.w != 1));
    // A tile in no range: nothing.
    let none = overlays(show, &scene, (0, 0), ALL);
    assert!(none.is_empty(), "{none:?}");
}

#[test]
fn the_outline_waits_for_the_spread() {
    let show = RangeShow::TINT.spreading().outlined();
    // (13, 21) is two steps from the start.
    let reached = 1000 + 2 * SPREAD_STEP_MS;
    let none = overlays(show, &row(reached), (3, 1), ALL);
    assert!(none.is_empty(), "{none:?}");
    assert_eq!(overlays(show, &row(reached + 1), (3, 1), ALL).len(), 3);
}

#[test]
fn a_raised_tile_is_lit_above_and_left_and_shaded_below_and_right() {
    let show = RangeShow::TINT.with_relief(Relief::Tiles);
    let scene = row(0);
    let got = overlays(show, &scene, (2, 1), ALL);
    let r = rect((2, 1));
    let lit = BLUE.lerp(Rgb::new(255, 255, 255), 0.5);
    let shade = BLUE.lerp(Rgb::new(0, 0, 0), SHADE);
    let expect = [
        Overlay::new(Rect::new(r.x, r.y, 16, 1), lit, Layer::Under),
        Overlay::new(Rect::new(r.x + 15, r.y, 1, 16), shade, Layer::Under),
        Overlay::new(Rect::new(r.x, r.y + 15, 16, 1), shade, Layer::Under),
        Overlay::new(Rect::new(r.x, r.y, 1, 16), lit, Layer::Under),
    ];
    assert_eq!(got, expect);
}

#[test]
fn a_slab_casts_its_shadow_below_and_right_of_the_range_inside_the_clip() {
    let show = RangeShow::TINT.with_relief(Relief::Slab);
    let scene = row(0);
    let shadow = Rgb::new(0, 0, 0).lerp(RED, SHADOW);
    // The attack tile ends the row: a shadow below it and right of it,
    // 2 pixels deep (twice the line) and pushed 2 pixels down and right.
    let r = rect((3, 1));
    let got = overlays(show, &scene, (3, 1), ALL);
    let expect = [
        Overlay::new(Rect::new(r.x + 2, r.y + 16, 16, 2), shadow, Layer::Under),
        Overlay::new(Rect::new(r.x + 16, r.y + 2, 2, 16), shadow, Layer::Under),
    ];
    assert_eq!(got, expect);
    // The middle tile has a range right of it: a shadow below only.
    assert_eq!(overlays(show, &scene, (2, 1), ALL).len(), 1);
    // Cut at the clip.
    let clip = Rect::new(0, 0, r.x + 16, 1000);
    let cut = overlays(show, &scene, (3, 1), clip);
    assert_eq!(
        cut,
        [Overlay::new(
            Rect::new(r.x + 2, r.y + 16, 14, 2),
            shadow,
            Layer::Under
        )]
    );
}

#[test]
fn the_outline_and_the_shadow_look_up_and_down_and_scale_with_the_line() {
    // A move tile above (12, 21): the outline's top there is closed, its
    // bottom open; the shadow is twice a 2-pixel line deep.
    let mut scene = row(5000);
    scene.tint([p(12, 20)], MOVE);
    let show = RangeShow::TINT.outlined().with_relief(Relief::Slab);
    let at = (2, 1);
    let r = rect(at);
    let got = show.overlays(&scene, &scene.tints(), at, r, 2, 0.5, color, ALL);
    let shadow = Rgb::new(0, 0, 0).lerp(BLUE, SHADOW);
    let light = BLUE.lerp(Rgb::new(255, 255, 255), 0.5);
    let expect = [
        Overlay::new(Rect::new(r.x + 4, r.y + 16, 16, 4), shadow, Layer::Under),
        Overlay::new(Rect::new(r.x, r.y + 14, 16, 2), light, Layer::Under),
    ];
    assert_eq!(got, expect);
    // The tile above: open on top, left and right; closed below.
    let above = show.overlays(
        &scene,
        &scene.tints(),
        (2, 0),
        rect((2, 0)),
        2,
        0.5,
        color,
        ALL,
    );
    let top = rect((2, 0));
    assert!(above.contains(&Overlay::new(
        Rect::new(top.x, top.y, 16, 2),
        light,
        Layer::Under
    )));
    assert!(!above.iter().any(|o| o.rect.y == top.y + 14));
}

#[test]
fn the_slab_shadow_goes_under_the_outline() {
    let got = overlays(SLAB, &row(5000), (3, 1), ALL);
    let shadow = Rgb::new(0, 0, 0).lerp(RED, SHADOW);
    assert_eq!(got.len(), 5);
    assert!(got[..2].iter().all(|o| o.color == shadow));
    assert!(got[2..].iter().all(|o| o.color != shadow));
}

proptest! {
    #[test]
    fn a_strength_is_a_fraction_and_the_overlays_lie_inside_the_clip(
        scene in any_scene(),
        wave in prop::sample::select(vec![Wave::None, Wave::Shimmer, Wave::Glint]),
        relief in prop::sample::select(vec![Relief::Flat, Relief::Tiles, Relief::Slab]),
        spread: bool,
        outline: bool,
    ) {
        let mut show = RangeShow::TINT.with_wave(wave).with_relief(relief);
        show.spread = spread;
        show.outline = outline;
        let tints = scene.tints();
        let clip = Rect::new(0, 0, 16 * scene.size.0, 16 * scene.size.1);
        for dy in 0..scene.size.1 {
            for dx in 0..scene.size.0 {
                let s = show.strength(&scene, (dx, dy));
                prop_assert!((0.0..=1.0).contains(&s), "{s}");
                let at = (dx, dy);
                for o in show.overlays(&scene, &tints, at, rect(at), 1, 0.5, color, clip) {
                    prop_assert_eq!(o.rect.intersect(&clip), Some(o.rect));
                }
            }
        }
    }
}
