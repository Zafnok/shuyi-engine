//! The scene camera over the toy's field (moved from the game's tests,
//! ticket 1154): the field in glyphs is 160 × 128 pixels, and the toy's
//! window ([`CAMERA`]) 192 × 160.

use super::*;
use crate::frame::color::{EngineColor, Rgb};
use crate::frame::glyph_buffer::Cell;
use crate::toy::grid::{CAMERA, Field};
use crate::toy::{Action, Toy, ToyCtx, ctx};

type Input = FrameInput<Action>;

fn camera(ctx: &ToyCtx) -> SceneCameraScreen {
    Field::glyphs().camera(ctx)
}

fn draw(ctx: &ToyCtx, screen: &SceneCameraScreen) -> GlyphBuffer {
    let c = |e| ctx.palette.engine(e);
    let stale = Cell::new('x', c(EngineColor::Text), c(EngineColor::Panel));
    let mut buf = GlyphBuffer::with_spec(ctx.console, stale);
    Screen::<Toy>::draw(screen, ctx, &mut buf);
    buf
}

fn update(screen: &mut SceneCameraScreen, ctx: &mut ToyCtx, input: &Input) -> Transition<Toy> {
    Screen::<Toy>::update(screen, ctx, input)
}

/// One frame of `dt` seconds with `pressed` pressed and `held` down.
fn frame(pressed: &[Action], dt: f32, held: &[Action]) -> Input {
    FrameInput::new(pressed.to_vec(), dt, held.to_vec())
}

fn near(a: (f32, f32), b: (f32, f32)) -> bool {
    (a.0 - b.0).abs() < 1e-3 && (a.1 - b.1).abs() < 1e-3
}

fn row(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(buf.width()))
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

#[test]
fn the_sizes_are_the_windows_and_the_pictures_pixels() {
    let ctx = ctx();
    let screen = camera(&ctx);
    assert_eq!(screen.window_px(), (192, 160));
    // The text box overlaps the window's corner and sticks out of it.
    let overlap = CAMERA.window.intersect(&CAMERA.text_box).unwrap();
    assert_eq!(overlap, Rect::new(18, 12, 8, 2));
    let blank = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
    let shown = SceneCameraScreen::new(
        ctx.console,
        Some(Rc::new(GlyphBuffer::new(3, 2, blank))),
        CAMERA,
        String::new(),
    );
    assert!(near(shown.centre(), (12.0, 16.0)));
}

#[test]
fn it_starts_at_one_x_over_the_middle_of_the_picture() {
    let ctx = ctx();
    let screen = camera(&ctx);
    assert_eq!(Screen::<Toy>::name(&screen), "scene_camera");
    // 10 × 8 tiles of two cells by one.
    let picture = screen.picture().unwrap();
    assert_eq!((picture.width(), picture.height()), (20, 8));
    assert_eq!(picture.get(4, 3).map(|c| c.glyph), Some('y'));
    assert!(near(screen.centre(), (80.0, 64.0)));
    assert_eq!(screen.zoom(), 1);
    // The toy's tiles are 24 px: 240 × 192, in whole cells.
    let screen = Field::pictures(&ctx).camera(&ctx);
    let picture = screen.picture().unwrap();
    assert_eq!((picture.width(), picture.height()), (30, 12));
    let found = picture.sprites();
    assert!(!found.is_empty(), "{found:?}");
    assert!(near(screen.centre(), (120.0, 96.0)));
}

#[test]
fn the_picture_shows_only_through_the_windows_see_through_cells() {
    let ctx = ctx();
    let screen = camera(&ctx);
    let buf = draw(&ctx, &screen);
    let backdrop = buf.backdrop().unwrap();
    assert_eq!(backdrop.clip(), CAMERA.window);
    assert_eq!(backdrop.zoom(), 1);
    // A picture smaller than the window is centred in it.
    assert!(near(backdrop.origin_px(), (-16.0, -16.0)));
    assert_eq!(backdrop.scene(), &**screen.picture().unwrap());
    for y in 0..i32::from(ctx.console.rows) {
        for x in 0..i32::from(ctx.console.cols) {
            let hole = CAMERA.window.contains(x, y) && !CAMERA.text_box.contains(x, y);
            assert_eq!(buf.get(x, y).unwrap().see_through, hole, "({x}, {y})");
        }
    }
    // The caption's glyphs are over the picture; the box's cells are
    // solid, with their own background.
    let c = |e| ctx.palette.engine(e);
    let hi = c(EngineColor::Highlight);
    assert_eq!(buf.get(3, 4), Some(&Cell::see_through('T', hi)));
    let (text, panel, black) = (
        c(EngineColor::Text),
        c(EngineColor::Panel),
        c(EngineColor::Background),
    );
    assert_eq!(buf.get(20, 13), Some(&Cell::new('A', text, panel)));
    assert_eq!(buf.get(0, 0), Some(&Cell::new(' ', text, black)));
    assert_eq!(buf.get(2, 0), Some(&Cell::new('S', hi, black)));
    // The window's frame and the box's border.
    assert_eq!(buf.get(1, 3).map(|c| c.fg), Some(c(EngineColor::Border)));
    assert_eq!(buf.get(18, 12).map(|c| c.fg), Some(c(EngineColor::Focus)));
    assert_eq!(row(&buf, 0).trim(), "Scene camera (debug)");
    assert_eq!(
        row(&buf, 2).trim(),
        "zoom 1x   window starts at -16.0,-16.0"
    );
    let help = row(&buf, 19);
    assert!(help.trim().starts_with("arrows pan (hold)"), "{help}");
    assert_eq!(buf.get(2, 19).map(|c| c.fg), Some(c(EngineColor::TextDim)));
    // The box's lines, from its second cell in.
    let line = |y| {
        row(&buf, y)[..]
            .chars()
            .skip(20)
            .take(16)
            .collect::<String>()
    };
    assert_eq!(line(13), "A text box drawn");
    assert_eq!(line(15).trim_end(), "hides it.");
}

#[test]
fn the_view_says_what_the_camera_shows() {
    let ctx = ctx();
    let screen = camera(&ctx);
    let view = screen.view(&ctx);
    assert_eq!(view.title, "Scene camera (debug)");
    assert_eq!(view.info, "zoom 1x   window starts at -16.0,-16.0");
    assert_eq!(view.layout, CAMERA);
    assert_eq!(view.caption, "Text over the scene");
    assert_eq!(view.box_text, BOX_TEXT);
    let (picture, frame) = view.shown.unwrap();
    assert_eq!(&picture, screen.picture().unwrap());
    assert_eq!(frame.zoom, 1);
    assert!(near(frame.origin_px, (-16.0, -16.0)));
    assert_eq!(view.help, "arrows pan (hold) · f zoom · d back");
}

#[test]
fn confirm_steps_the_zoom_through_the_whole_steps_and_round() {
    let mut ctx = ctx();
    let mut screen = camera(&ctx);
    let mut zooms = Vec::new();
    for _ in 0..5 {
        let stay = update(&mut screen, &mut ctx, &frame(&[Action::Confirm], 0.0, &[]));
        assert!(matches!(stay, Transition::None));
        zooms.push(screen.zoom());
        assert_eq!(
            draw(&ctx, &screen).backdrop().unwrap().zoom(),
            screen.zoom()
        );
    }
    assert_eq!(zooms, [2, 3, 4, 1, 2]);
    let sounds = ctx.audio.take();
    let cues: Vec<_> = sounds.iter().map(|s| s.cue()).collect();
    assert_eq!(cues, [Some("ui_select"); 5]);
    // Two presses in one frame are two steps; other actions do nothing.
    let both = [Action::Confirm, Action::Info, Action::Confirm];
    update(&mut screen, &mut ctx, &frame(&both, 0.0, &[]));
    assert_eq!(screen.zoom(), 4);
}

#[test]
fn held_cursor_keys_pan_by_the_pixel_and_stop_at_the_edges() {
    use Action::{CursorDown, CursorLeft, CursorRight, CursorUp};
    let mut ctx = ctx();
    let mut screen = camera(&ctx);
    let mut hold = |screen: &mut SceneCameraScreen, dt: f32, held: &[Action]| {
        update(screen, &mut ctx, &frame(&[], dt, held));
        screen.origin_px()
    };
    // At 1× the picture is smaller than the window: nothing to pan.
    assert!(near(
        hold(&mut screen, 1.0, &[CursorRight, CursorDown]),
        (-16.0, -16.0)
    ));
    // At 4× the window shows 48 × 40 of 160 × 128, from the middle.
    screen.zoom = 4;
    screen.pan(0.0, 0.0);
    assert!(near(screen.origin_px(), (56.0, 44.0)));
    // 60 px a second: a tenth of a second is 6 px.
    assert!(near(hold(&mut screen, 0.1, &[CursorRight]), (62.0, 44.0)));
    assert!(near(
        hold(&mut screen, 0.0125, &[CursorDown]),
        (62.0, 44.75)
    ));
    let back = [CursorLeft, CursorUp];
    assert!(near(hold(&mut screen, 0.1, &back), (56.0, 38.75)));
    // Opposite keys cancel; a pressed key that isn't held does nothing.
    update(
        &mut screen,
        &mut ctx,
        &frame(&[CursorRight], 0.1, &[CursorLeft, CursorRight]),
    );
    assert!(near(screen.origin_px(), (56.0, 38.75)));
    let mut hold = |screen: &mut SceneCameraScreen, dt: f32, held: &[Action]| {
        update(screen, &mut ctx, &frame(&[], dt, held));
        screen.origin_px()
    };
    // Not held: no pan, however long the frame.
    assert!(near(hold(&mut screen, 5.0, &[]), (56.0, 38.75)));
    // Each edge stops the window, and the centre with it: one step back
    // moves at once.
    let ahead = [CursorRight, CursorDown];
    assert!(near(hold(&mut screen, 60.0, &ahead), (112.0, 88.0)));
    assert!(near(screen.centre(), (136.0, 108.0)));
    assert!(near(hold(&mut screen, 0.1, &back), (106.0, 82.0)));
    assert!(near(hold(&mut screen, 60.0, &back), (0.0, 0.0)));
    assert!(near(screen.centre(), (24.0, 20.0)));
    // A bad frame time moves nothing.
    for dt in [f32::NAN, f32::INFINITY, -1.0] {
        assert!(near(hold(&mut screen, dt, &ahead), (0.0, 0.0)), "{dt}");
    }
    // The frame shows where the window is.
    let buf = draw(&ctx, &screen);
    assert!(near(buf.backdrop().unwrap().origin_px(), (0.0, 0.0)));
}

#[test]
fn zooming_keeps_the_point_where_the_window_can_show_it() {
    let mut ctx = ctx();
    let mut screen = camera(&ctx);
    let confirm = frame(&[Action::Confirm], 0.0, &[]);
    screen.zoom = 4;
    screen.pan(-20.0, 10.0);
    assert!(near(screen.centre(), (60.0, 74.0)));
    // 4× → 1×: the whole picture fits, so the centre is its middle.
    update(&mut screen, &mut ctx, &confirm);
    assert!(near(screen.origin_px(), (-16.0, -16.0)));
    assert!(near(screen.centre(), (80.0, 64.0)));
    // → 2× → 3×: 64 × 53.33 shown, still from the middle.
    update(&mut screen, &mut ctx, &confirm);
    update(&mut screen, &mut ctx, &confirm);
    assert!(near(screen.centre(), (80.0, 64.0)));
    // 3× → 4× near the top-left corner: the point is kept.
    screen.pan(-30.0, -20.0);
    assert!(near(screen.centre(), (50.0, 44.0)));
    update(&mut screen, &mut ctx, &confirm);
    assert!(near(screen.centre(), (50.0, 44.0)));
    assert!(near(screen.origin_px(), (26.0, 24.0)));
    // At 3× the window's middle can't be nearer the edge than half of
    // what it shows: 32 and 26.67.
    screen.zoom = 3;
    screen.pan(-500.0, -500.0);
    assert!(near(screen.centre(), (32.0, 80.0 / 3.0)));
    assert!(near(screen.origin_px(), (0.0, 0.0)));
}

#[test]
fn cancel_closes_with_the_cancel_sound() {
    let mut ctx = ctx();
    let mut screen = camera(&ctx);
    let both = frame(&[Action::Cancel, Action::Confirm], 0.0, &[]);
    assert!(matches!(
        update(&mut screen, &mut ctx, &both),
        Transition::Pop
    ));
    assert_eq!(screen.zoom(), 1);
    let sounds = ctx.audio.take();
    let cues: Vec<_> = sounds.iter().map(|s| s.cue()).collect();
    assert_eq!(cues, [Some("ui_cancel")]);
}

#[test]
fn says_so_when_there_is_no_picture() {
    let mut ctx = ctx();
    let mut screen = SceneCameraScreen::new(ctx.console, None, CAMERA, "No field".to_owned());
    assert!(screen.picture().is_none());
    let input = frame(&[Action::Confirm], 0.5, &[Action::CursorRight]);
    update(&mut screen, &mut ctx, &input);
    assert!(near(screen.centre(), (0.0, 0.0)));
    let buf = draw(&ctx, &screen);
    assert!(buf.backdrop().is_none());
    assert_eq!(row(&buf, 2).trim(), "No field");
    assert_eq!(row(&buf, 0).trim(), "Scene camera (debug)");
    assert!(
        !buf.get(CAMERA.window.x, CAMERA.window.y)
            .unwrap()
            .see_through
    );
    // No window, no box.
    let blank = buf.get(CAMERA.text_box.x + 2, CAMERA.text_box.y + 1);
    assert_eq!(blank.map(|c| c.glyph), Some(' '));
    assert!(screen.view(&ctx).shown.is_none());
}

/// The camera declares the cursor, Confirm and Cancel (ADR-0025).
#[test]
fn the_camera_declares_its_actions() {
    use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp};
    let c = ctx();
    assert_eq!(
        Screen::<Toy>::actions(&camera(&c)),
        Some(vec![
            CursorLeft,
            CursorDown,
            CursorUp,
            CursorRight,
            Confirm,
            Cancel
        ])
    );
}
