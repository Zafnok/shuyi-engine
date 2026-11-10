//! The preview's glyph skin over views built by hand: where each part goes,
//! its colours, what is cut, wrapped and scrolled, as snapshots.

use super::*;
use crate::dialogue::preview::view::{
    GoToView, LineRow, PlayingView, ProblemRow, SceneRow, StripView, SwitchRow,
};
use crate::frame::glyph_buffer::Cell;
use crate::toy::{self, ToyCtx};

/// A buffer `w` × `h` of `X`s, so what the skin leaves shows.
fn buffer(ctx: &ToyCtx, w: u16, h: u16) -> GlyphBuffer {
    let colour = |e| ctx.palette.engine(e);
    let cell = Cell::new('X', colour(EngineColor::Text), colour(EngineColor::Border));
    GlyphBuffer::new(w, h, cell)
}

/// `view` painted over the background on a `w` × `h` buffer.
fn painted(ctx: &ToyCtx, view: &PreviewView, w: u16, h: u16) -> String {
    let mut buf = buffer(ctx, w, h);
    paint_back(ctx, &mut buf);
    paint(ctx, view, &mut buf);
    buf.to_snapshot(&ctx.palette)
}

/// The tab `tab`, named as the toy names them, its help a short line.
fn view(tab: Tab) -> PreviewView {
    let names = ["Scenes", "Switches", "Lines", "Problems"];
    let mut view = PreviewView::new(tab, names.map(String::from).to_vec());
    view.help = "keys here".to_owned();
    view
}

#[test]
fn the_background_is_cleared() {
    let ctx = toy::ctx();
    let mut buf = buffer(&ctx, 4, 2);
    paint_back(&ctx, &mut buf);
    insta::assert_snapshot!(buf.to_snapshot(&ctx.palette));
}

#[test]
fn the_scenes_tab_marks_the_scene_picked_and_files_with_problems() {
    let ctx = toy::ctx();
    let mut v = view(Tab::Scenes);
    let mut gate = SceneRow::new("gate.dlg".into(), "gate".into());
    gate.picked = true;
    let mut lonely = SceneRow::new("lonely.dlg".into(), "lonely".into());
    lonely.problems = true;
    v.scenes = vec![
        gate,
        SceneRow::new("gate.dlg".into(), "second".into()),
        lonely,
    ];
    v.focus = 1;
    let mut playing = PlayingView::new(
        "a/folder/with/a/very/long/name/gate.dlg".into(),
        "gate".into(),
    );
    playing.text_box = 1;
    v.playing = Some(playing);
    insta::assert_snapshot!(painted(&ctx, &v, 40, 8));
}

#[test]
fn the_switches_tab_puts_each_value_in_its_column() {
    let ctx = toy::ctx();
    let mut v = view(Tab::Switches);
    v.switches = vec![
        SwitchRow::new("scout".into(), "Gone".into(), false),
        SwitchRow::new("hero named".into(), "On".into(), true),
    ];
    v.focus = 1;
    insta::assert_snapshot!(painted(&ctx, &v, 40, 6));
}

#[test]
fn the_lines_tab_shows_the_line_box_and_the_text_box_playing() {
    let ctx = toy::ctx();
    let mut v = view(Tab::Lines);
    let mut go_to = GoToView::new("Go to line".into());
    go_to.digits = "12".into();
    go_to.typing = true;
    v.go_to = go_to;
    let mut said = LineRow::new(4, "Is the road clear?".into());
    said.speaker = Some("mentor".into());
    let mut playing = LineRow::new(9, "Nobody keeps watch.".into());
    playing.current = true;
    v.lines = vec![said, playing];
    insta::assert_snapshot!(painted(&ctx, &v, 40, 6));
}

#[test]
fn a_note_goes_under_the_line_box_and_at_the_top_of_an_empty_list() {
    let ctx = toy::ctx();
    let mut lines = view(Tab::Lines);
    lines.note = Some("Pick a scene first".into());
    let mut scenes = view(Tab::Scenes);
    scenes.note = Some("No scenes".into());
    let both = [painted(&ctx, &lines, 30, 6), painted(&ctx, &scenes, 30, 6)];
    insta::assert_snapshot!(both.join("\n"));
}

#[test]
fn a_problem_wraps_and_its_fix_wraps_further_in() {
    let ctx = toy::ctx();
    let mut v = view(Tab::Problems);
    let mut long = ProblemRow::new(
        "lonely.dlg".into(),
        "\"mentor\" speaks but is not on screen, nor anywhere near it".into(),
    );
    long.line = Some(2);
    long.fix = Some("place them with @left or @right first, before the line".into());
    v.problems = vec![
        long,
        ProblemRow::new("all".into(), "too many scenes".into()),
    ];
    insta::assert_snapshot!(painted(&ctx, &v, 40, 10));
}

#[test]
fn a_long_list_scrolls_to_show_the_focused_row() {
    let ctx = toy::ctx();
    let mut v = view(Tab::Scenes);
    v.scenes = (0..30)
        .map(|i| SceneRow::new("a.dlg".into(), format!("scene{i}")))
        .collect();
    v.focus = 25;
    insta::assert_snapshot!(painted(&ctx, &v, 24, 12));
}

#[test]
fn the_rows_above_a_tall_focused_problem_scroll_away() {
    let ctx = toy::ctx();
    let mut v = view(Tab::Problems);
    let row = |i: u32| {
        let mut p = ProblemRow::new("a.dlg".into(), format!("problem {i}"));
        p.line = Some(i);
        p.fix = Some(format!("fix {i}"));
        p
    };
    v.problems = (1..=5).map(row).collect();
    v.focus = 3;
    insta::assert_snapshot!(painted(&ctx, &v, 24, 8));
}

#[test]
fn the_focused_row_stays_on_screen_in_any_room() {
    let item = |n: usize| Item {
        lines: vec![String::new(); n],
        value: None,
        bright: false,
    };
    let items: Vec<Item> = [1, 2, 1, 3, 1].into_iter().map(item).collect();
    // Rows of items 0..=focus: 1, 3, 4, 7, 8.
    assert_eq!(first_shown(&items, 0, 0), 0);
    assert_eq!(first_shown(&items, 3, 7), 0);
    assert_eq!(first_shown(&items, 3, 6), 1);
    assert_eq!(first_shown(&items, 3, 4), 2);
    assert_eq!(first_shown(&items, 3, 3), 3);
    assert_eq!(first_shown(&items, 3, 1), 3, "never past the focus");
    assert_eq!(first_shown(&items, 4, 2), 4);
}

#[test]
fn the_strip_cuts_the_direction_before_the_help() {
    let ctx = toy::ctx();
    let mut v = view(Tab::Scenes);
    v.open = false;
    let mut strip = StripView::new("gate.dlg".into(), 14);
    strip.direction = Some("quietly, out of breath, looking back at the hill".into());
    strip.help = "Esc tabs".into();
    v.strip = Some(strip);
    insta::assert_snapshot!(painted(&ctx, &v, 40, 2));
}

#[test]
fn the_strip_leaves_out_a_help_that_doesnt_fit() {
    let ctx = toy::ctx();
    let mut v = view(Tab::Scenes);
    v.open = false;
    let mut strip = StripView::new("f".into(), 1);
    strip.help = "ab".into();
    v.strip = Some(strip.clone());
    // "f:1" ends at column 4; a help of two cells would start there.
    let tight = painted(&ctx, &v, 7, 1);
    strip.help = "a".into();
    v.strip = Some(strip);
    let room = painted(&ctx, &v, 7, 1);
    insta::assert_snapshot!([tight, room].join("\n"));
}

#[test]
fn closed_with_no_strip_paints_nothing() {
    let ctx = toy::ctx();
    let mut v = view(Tab::Scenes);
    v.open = false;
    let mut buf = buffer(&ctx, 6, 2);
    paint(&ctx, &v, &mut buf);
    assert_eq!(
        buf.to_snapshot(&ctx.palette),
        buffer(&ctx, 6, 2).to_snapshot(&ctx.palette)
    );
}
