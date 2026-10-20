//! The text box's glyph look, on the engine's own palette.

use super::*;
use crate::dialogue::{SkipView, TextView, WaitView};
use crate::frame::color::tests::engine_palette;
use crate::widgets::menu::MenuItemView;

/// A 40-cell box on rows 10 to 16, its text from (3, 12).
const PLACE: BoxPlace = BoxPlace {
    frame: Rect::new(0, 10, 40, 7),
    text_x: 3,
    text_y: 12,
};

fn text(lines: &[&str]) -> TextView {
    let lines: Vec<String> = lines.iter().map(|&l| l.to_owned()).collect();
    TextView {
        narration: false,
        shown: lines.iter().map(|l| l.chars().count()).sum(),
        lines,
        choices: None,
        skip: None,
        waiting: None,
    }
}

fn replies(labels: &[&str], focus: usize) -> MenuView {
    let item = |(i, &label): (usize, &&str)| MenuItemView {
        label: label.into(),
        suffix: None,
        note: None,
        enabled: true,
        focused: i == focus,
    };
    MenuView::new(labels.iter().enumerate().map(item).collect())
}

fn painted(view: &TextView, speaker: Option<&str>) -> (GlyphBuffer, Palette) {
    let mut c = crate::toy::ctx();
    c.palette = engine_palette();
    let black = c.palette.engine(EngineColor::Background);
    let mut buf = GlyphBuffer::new(40, 20, Cell::new('x', black, black));
    paint(&c, &mut buf, PLACE, view, speaker);
    (buf, c.palette)
}

/// `len` cells of row `y` from column `x`.
fn cells(buf: &GlyphBuffer, y: i32, x: i32, len: i32) -> String {
    (x..x + len)
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

fn row(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(buf.width()))
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

#[test]
fn the_box_has_the_speakers_name_on_its_border_and_the_text_inside() {
    let (buf, p) = painted(&text(&["Hello there."]), Some("Mei"));
    assert_eq!(row(&buf, 9), "x".repeat(40), "above the box");
    assert!(row(&buf, 10).starts_with("┌── Mei ──"), "{}", row(&buf, 10));
    assert!(row(&buf, 16).starts_with('└'));
    assert!(row(&buf, 12).starts_with("│  Hello there."));
    let at = |x, y| *buf.get(x, y).unwrap();
    assert_eq!(at(4, 10).fg, p.engine(EngineColor::Highlight));
    assert_eq!(at(0, 10).fg, p.engine(EngineColor::Border));
    assert_eq!(at(3, 12).fg, p.engine(EngineColor::Text));
    assert_eq!(at(3, 12).bg, p.engine(EngineColor::Panel));
    // No speaker: no name.
    let (buf, _) = painted(&text(&["Hi."]), None);
    assert!(row(&buf, 10).starts_with("┌───────"));
}

#[test]
fn only_the_revealed_text_shows() {
    let mut view = text(&["abc", "defg"]);
    view.shown = 5;
    let (buf, _) = painted(&view, None);
    assert!(row(&buf, 12).starts_with("│  abc "));
    assert!(row(&buf, 13).starts_with("│  de "));
}

#[test]
fn narration_is_dim_and_centred_and_one_line_sits_in_the_middle() {
    let mut view = text(&["It rains."]);
    view.narration = true;
    let (buf, p) = painted(&view, None);
    // 9 characters in 40 cells: from column 15, on the middle row.
    assert_eq!(row(&buf, 12).trim_matches(['│', ' ']), "");
    assert_eq!(cells(&buf, 13, 15, 9), "It rains.");
    assert_eq!(buf.get(15, 13).unwrap().fg, p.engine(EngineColor::TextDim));
    // Two lines fill from the top.
    let mut view = text(&["It rains.", "Hard."]);
    view.narration = true;
    let (buf, _) = painted(&view, None);
    assert_eq!(cells(&buf, 12, 15, 9), "It rains.");
    assert_eq!(cells(&buf, 13, 17, 5), "Hard.");
}

#[test]
fn the_replies_grow_the_box_upward_under_the_line_they_answer() {
    let mut view = text(&["Ready?"]);
    view.choices = Some(replies(&["Yes.", "No.", "Later."], 1));
    // The line, a blank row and three replies end on the ▼'s row (15).
    assert_eq!(box_rect(PLACE, &view), (Rect::new(0, 9, 40, 8), 11));
    let (buf, p) = painted(&view, Some("Guide"));
    assert!(row(&buf, 9).starts_with("┌── Guide"));
    assert!(row(&buf, 11).starts_with("│  Ready?"));
    assert!(row(&buf, 12).trim_matches(['│', ' ']).is_empty());
    assert!(row(&buf, 13).starts_with("│    Yes."));
    assert!(row(&buf, 14).starts_with("│  > No."));
    assert!(row(&buf, 15).starts_with("│    Later."));
    assert_eq!(buf.get(5, 14).unwrap().fg, p.engine(EngineColor::Highlight));
    assert_eq!(buf.get(5, 13).unwrap().fg, p.engine(EngineColor::Text));
    // Narration above replies isn't centred down a row.
    view.narration = true;
    let (buf, _) = painted(&view, None);
    assert!(row(&buf, 11).contains("Ready?"));
    // Replies with no line: no blank row; few enough fit the box as it is.
    let mut view = text(&[]);
    view.choices = Some(replies(&["Yes.", "No."], 0));
    assert_eq!(box_rect(PLACE, &view), (PLACE.frame, 12));
    view.choices = Some(replies(&["a", "b", "c", "d", "e"], 0));
    assert_eq!(box_rect(PLACE, &view), (Rect::new(0, 9, 40, 8), 11));
    // Without replies the box is as placed.
    assert_eq!(box_rect(PLACE, &text(&["x"])), (PLACE.frame, 12));
}

#[test]
fn the_skip_question_shows_instead_of_the_text() {
    let mut view = text(&["Hello."]);
    view.skip = Some(SkipView {
        question: "Skip scene?".to_owned(),
        answers: "f yes / d no".to_owned(),
    });
    view.choices = Some(replies(&["a"], 0));
    let (buf, p) = painted(&view, None);
    assert!(row(&buf, 12).starts_with("│  Skip scene?"));
    assert!(row(&buf, 13).starts_with("│  f yes / d no"));
    assert_eq!(buf.get(3, 13).unwrap().fg, p.engine(EngineColor::TextDim));
    assert!(!row(&buf, 12).contains("Hello"));
}

#[test]
fn a_waiting_page_names_the_key_the_log_and_blinks() {
    let mut view = text(&["Hi."]);
    view.waiting = Some(WaitView {
        key: "f".to_owned(),
        log_hint: Some("l log".to_owned()),
        marker: true,
    });
    let (buf, p) = painted(&view, None);
    // On the box's last row inside: the hint, three cells, the key, the ▼.
    let last = row(&buf, 15);
    assert!(last.ends_with("l log   f ▼  │"), "{last}");
    assert_eq!(
        buf.get(36, 15).unwrap().fg,
        p.engine(EngineColor::Highlight)
    );
    assert_eq!(buf.get(34, 15).unwrap().fg, p.engine(EngineColor::TextDim));
    view.waiting = Some(WaitView {
        key: "f".to_owned(),
        log_hint: None,
        marker: false,
    });
    let (buf, _) = painted(&view, None);
    assert!(row(&buf, 15).ends_with("  f    │"), "{}", row(&buf, 15));
}
