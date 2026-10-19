//! The scene camera over the toy's field through the Harness (ticket
//! 1154): the field's picture is a backdrop behind a window of the
//! console, and a snapshot pins both the picture and where the window is.

use insta::assert_snapshot;
use shuyi_ui::debug::SceneCameraScreen;
use shuyi_ui::harness::Harness;
use shuyi_ui::toy::grid::Field;
use shuyi_ui::toy::{self, Toy};
use shuyi_ui::{Game, GameHooks};

/// The toy game showing the scene camera over its field in `field`'s skin.
fn opened(field: fn(&toy::ToyCtx) -> Field) -> Harness<Toy> {
    let ctx = toy::ctx();
    let camera = field(&ctx).camera(&ctx);
    Harness::new(Game::new(ctx, Box::new(camera), GameHooks::default()))
}

fn glyphs(_: &toy::ToyCtx) -> Field {
    Field::glyphs()
}

/// The snapshot's backdrop header line.
fn header(snapshot: &str) -> &str {
    snapshot
        .lines()
        .find(|l| l.starts_with("--- backdrop: "))
        .unwrap_or("no backdrop")
}

fn camera(h: &Harness<Toy>) -> &SceneCameraScreen {
    h.screen::<SceneCameraScreen>()
        .unwrap_or_else(|| panic!("no camera: {:?}", h.screens()))
}

#[test]
fn each_whole_zoom_is_a_header_line_and_the_field() {
    let mut h = opened(glyphs);
    assert_eq!(h.screens(), ["scene_camera"]);
    let one = h.snapshot();
    assert_eq!(
        header(&one),
        "--- backdrop: clip 2,4 24x10  origin -16.0,-16.0  zoom 1 ---"
    );
    // After the header comes the picture's own snapshot: the whole field,
    // your piece on it.
    let scene = h.game().buffer().backdrop().unwrap().scene();
    assert_eq!((scene.width(), scene.height()), (20, 8));
    assert_eq!(scene.get(4, 3).map(|c| c.glyph), Some('y'));
    assert!(one.ends_with(&scene.to_snapshot(&h.ctx().palette)));
    assert_snapshot!("scene_camera_1x", one);
    let origins = ["32.0,24.0", "48.0,37.3", "56.0,44.0"];
    for (zoom, origin) in (2..).zip(origins) {
        h.keys("f");
        assert_eq!(
            header(&h.snapshot()),
            format!("--- backdrop: clip 2,4 24x10  origin {origin}  zoom {zoom} ---")
        );
    }
    assert_snapshot!("scene_camera_4x", h.snapshot());
    // Round again: no step past 4×, none in between.
    h.keys("f");
    assert_eq!(h.snapshot(), one);
    assert_eq!(h.sounds().iter().filter(|s| *s == "ui_select").count(), 4);
}

#[test]
fn holding_a_cursor_key_pans_by_the_pixel_to_the_edge() {
    let mut h = opened(glyphs);
    h.keys("f f f");
    // 60 picture pixels a second, at 60 frames a second: a pixel a frame.
    h.hold("Right", 0.5);
    let origin = h.game().buffer().backdrop().unwrap().origin_px();
    assert!((origin.0 - 86.0).abs() < 0.05, "{origin:?}");
    assert!((origin.1 - 44.0).abs() < 0.05, "{origin:?}");
    assert!((camera(&h).centre().0 - 110.0).abs() < 0.05);
    // The window stops at the picture's edge.
    h.hold("Left", 5.0).hold("Down", 5.0);
    assert_eq!(
        header(&h.snapshot()),
        "--- backdrop: clip 2,4 24x10  origin 0.0,88.0  zoom 4 ---"
    );
}

#[test]
fn the_window_shows_the_field_and_the_box_over_it_hides_it() {
    let h = opened(glyphs);
    let buf = h.game().buffer();
    let clip = buf.backdrop().unwrap().clip();
    assert!(buf.get(clip.x, clip.y).unwrap().see_through);
    let corner = buf.get(clip.x + clip.w - 1, clip.y + clip.h - 1).unwrap();
    assert!(!corner.see_through);
    assert!(!buf.get(clip.x - 1, clip.y).unwrap().see_through);
    let snapshot = h.snapshot();
    assert!(snapshot.contains("bg:see-through"), "{snapshot}");
    assert!(snapshot.contains("A text box drawn"), "{snapshot}");
    assert!(snapshot.contains("f zoom"), "{snapshot}");
}

#[test]
fn a_sprite_skinned_field_is_a_picture_of_sprites() {
    let h = opened(Field::pictures);
    let scene = h.game().buffer().backdrop().unwrap().scene();
    // 10 × 8 tiles of 24 px, and the two cards.
    assert_eq!((scene.width(), scene.height()), (30, 12));
    assert_eq!(scene.sprites().len(), 10 * 8 + 2);
    // The console's own items stay its own: the picture's aren't copied in.
    assert!(h.game().buffer().items().is_empty());
}

#[test]
fn cancel_closes_it() {
    let mut h = opened(glyphs);
    h.keys("d");
    assert!(h.screens().is_empty(), "{:?}", h.screens());
    assert_eq!(h.sounds(), ["ui_cancel"]);
}
