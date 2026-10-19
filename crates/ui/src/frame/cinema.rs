//! Framing a shot over a [`Backdrop`](crate::frame::glyph_buffer::Backdrop)
//! (game ADR-0048): where the window has to start in the scene to look at a
//! point at a zoom. Pure arithmetic; the cinematic's shots (0817) and the
//! debug menu's "Scene camera" use it.

pub use crate::frame::glyph_buffer::MAX_ZOOM;

/// The `origin_px` (the scene pixel at the window's top-left) that puts
/// the scene pixel `centre` in the middle of the window.
///
/// `scene_px` is the scene's size and `clip_px` the window's, both in
/// their own pixels; at `zoom` (clamped to 1..=[`MAX_ZOOM`]) the window
/// shows `clip_px / zoom` scene pixels each way. The window never leaves
/// the scene: near an edge it stops there. On an axis where the zoomed
/// scene is smaller than the window, the scene is centred instead (the
/// origin is then negative). A `centre` that isn't a number counts as 0.
pub fn view(scene_px: (u32, u32), clip_px: (u32, u32), centre: (f32, f32), zoom: u8) -> (f32, f32) {
    let zoom = f32::from(zoom.clamp(1, MAX_ZOOM));
    #[allow(clippy::cast_precision_loss)] // pixel sizes are far below 2^24
    let axis = |scene: u32, clip: u32, centre: f32| {
        let shown = clip as f32 / zoom;
        // How far the window can move. With room, from 0 to `room`;
        // without (a negative `room`), only to the centred place, `room / 2`.
        let room = scene as f32 - shown;
        let (first, last) = (room.min(0.0) / 2.0, room.max(room / 2.0));
        let centre = if centre.is_finite() { centre } else { 0.0 };
        (centre - shown / 2.0).clamp(first, last)
    };
    (
        axis(scene_px.0, clip_px.0, centre.0),
        axis(scene_px.1, clip_px.1, centre.1),
    )
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    /// A 224×128 scene (the test map in glyphs) in a 384×224 window.
    const SCENE: (u32, u32) = (224, 128);
    const CLIP: (u32, u32) = (384, 224);

    #[test]
    fn centres_the_window_on_the_point() {
        // At 4× the window shows 96×56 scene pixels.
        assert_eq!(view(SCENE, CLIP, (112.0, 64.0), 4), (64.0, 36.0));
        assert_eq!(view(SCENE, CLIP, (100.5, 40.25), 4), (52.5, 12.25));
        // At 2× it shows 192×112.
        assert_eq!(view(SCENE, CLIP, (112.0, 64.0), 2), (16.0, 8.0));
        // Each axis on its own.
        assert_eq!(view(SCENE, CLIP, (60.0, 90.0), 4), (12.0, 62.0));
    }

    #[test]
    fn stops_at_every_edge() {
        assert_eq!(view(SCENE, CLIP, (0.0, 0.0), 4), (0.0, 0.0));
        assert_eq!(view(SCENE, CLIP, (47.0, 27.0), 4), (0.0, 0.0));
        assert_eq!(view(SCENE, CLIP, (49.0, 29.0), 4), (1.0, 1.0));
        assert_eq!(view(SCENE, CLIP, (-500.0, 64.0), 4), (0.0, 36.0));
        assert_eq!(view(SCENE, CLIP, (112.0, -500.0), 4), (64.0, 0.0));
        // Right and bottom: 224 − 96 and 128 − 56.
        assert_eq!(view(SCENE, CLIP, (224.0, 128.0), 4), (128.0, 72.0));
        assert_eq!(view(SCENE, CLIP, (9e9, 64.0), 4), (128.0, 36.0));
        assert_eq!(view(SCENE, CLIP, (112.0, 9e9), 4), (64.0, 72.0));
        assert_eq!(view(SCENE, CLIP, (175.0, 99.0), 4), (127.0, 71.0));
        assert_eq!(
            view(SCENE, CLIP, (f32::INFINITY, f32::NEG_INFINITY), 4),
            (0.0, 0.0)
        );
        assert_eq!(view(SCENE, CLIP, (f32::NAN, 64.0), 4), (0.0, 36.0));
    }

    #[test]
    fn centres_a_scene_smaller_than_the_window() {
        // At 1× the window shows 384×224: more than the scene both ways.
        assert_eq!(view(SCENE, CLIP, (0.0, 0.0), 1), (-80.0, -48.0));
        assert_eq!(view(SCENE, CLIP, (999.0, 999.0), 1), (-80.0, -48.0));
        // Smaller one way only: 3× shows 128 × 74.67 of a 100×128 scene.
        let (x, y) = view((100, 128), CLIP, (50.0, 128.0), 3);
        assert!((x + 14.0).abs() < 1e-4, "{x}");
        assert!((y - (128.0 - 224.0 / 3.0)).abs() < 1e-4, "{y}");
        // Exactly the window's size: no room to pan.
        assert_eq!(view((192, 112), CLIP, (150.0, 100.0), 2), (0.0, 0.0));
        // Nothing to show.
        assert_eq!(view((0, 0), (0, 0), (5.0, 5.0), 2), (0.0, 0.0));
        assert_eq!(view((0, 0), CLIP, (5.0, 5.0), 1), (-192.0, -112.0));
    }

    #[test]
    fn a_zoom_outside_the_steps_is_clamped() {
        let at = |zoom| view(SCENE, CLIP, (112.0, 64.0), zoom);
        assert_eq!(at(0), at(1));
        assert_eq!(at(5), at(4));
        assert_eq!(at(u8::MAX), at(4));
        assert_ne!(at(3), at(4));
    }

    proptest! {
        #[test]
        fn the_window_stays_inside_a_scene_at_least_its_size(
            scene_w in 1u32..4000, scene_h in 1u32..2000,
            clip_w in 1u32..801, clip_h in 1u32..513,
            cx in prop_oneof![-5000f32..5000.0, any::<f32>()],
            cy in prop_oneof![-5000f32..5000.0, any::<f32>()],
            zoom in 1u8..=MAX_ZOOM,
        ) {
            let (x, y) = view((scene_w, scene_h), (clip_w, clip_h), (cx, cy), zoom);
            let check = |origin: f32, scene: u32, clip: u32| {
                #[allow(clippy::cast_precision_loss)]
                let (scene, shown) = (scene as f32, clip as f32 / f32::from(zoom));
                if scene >= shown {
                    assert!(origin >= 0.0, "{origin}");
                    assert!(origin + shown <= scene + 1e-3, "{origin} + {shown} > {scene}");
                } else {
                    // Centred: the same gap on both sides.
                    assert!((origin * 2.0 - (scene - shown)).abs() < 1e-3);
                }
            };
            check(x, scene_w, clip_w);
            check(y, scene_h, clip_h);
        }

        #[test]
        fn the_point_is_in_the_middle_when_there_is_room(
            cx in 48f32..176.0, cy in 28f32..100.0,
        ) {
            let (x, y) = view(SCENE, CLIP, (cx, cy), 4);
            prop_assert!((x + 48.0 - cx).abs() < 1e-3);
            prop_assert!((y + 28.0 - cy).abs() < 1e-3);
        }
    }
}
