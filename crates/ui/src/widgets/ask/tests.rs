//! A question on the toy game's actions and text.

use super::*;
use crate::frame::color::EngineColor;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer};
use crate::toy::{Action, Voice, ctx};
use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp, Info};

/// The answers of the toy's question asked on leaving the options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Leave {
    Keep,
    Discard,
    Back,
}

/// Keep or Discard, Keep in focus; Cancel does nothing.
fn two() -> Ask<Leave> {
    Ask::new(
        "options.question.keep",
        [
            (Leave::Keep, "options.leave.keep"),
            (Leave::Discard, "options.leave.discard"),
        ],
    )
}

/// Keep, Discard or Back, Back in focus and Cancel's answer.
fn three() -> Ask<Leave> {
    Ask::new(
        "options.question.keep",
        [
            (Leave::Keep, "options.leave.keep"),
            (Leave::Discard, "options.leave.discard"),
            (Leave::Back, "options.leave.back"),
        ],
    )
    .focused_on(Leave::Back)
    .cancelled_by(Leave::Back)
}

/// What `actions` give on `ask`, one by one, and the sounds they play.
fn press<T: Copy + PartialEq>(
    ask: &mut Ask<T>,
    actions: &[Action],
) -> (Vec<Option<T>>, Vec<String>) {
    let mut audio = AudioQueue::<Voice>::new(crate::audio::AudioConfig::default());
    let answers = actions.iter().map(|&a| ask.handle(a, &mut audio)).collect();
    let sounds = audio
        .take()
        .iter()
        .filter_map(|r| r.cue().map(str::to_owned))
        .collect();
    (answers, sounds)
}

#[test]
fn two_answers_step_both_ways_and_wrap_with_the_move_sound() {
    let mut ask = two();
    assert_eq!(ask.focus(), Some(Leave::Keep));
    let (answers, sounds) = press(&mut ask, &[CursorRight]);
    assert_eq!((answers, ask.focus()), (vec![None], Some(Leave::Discard)));
    assert_eq!(sounds, ["ui_move"]);
    // Past the last, round to the first; up and down step too.
    press(&mut ask, &[CursorDown]);
    assert_eq!(ask.focus(), Some(Leave::Keep));
    press(&mut ask, &[CursorLeft]);
    assert_eq!(ask.focus(), Some(Leave::Discard));
    press(&mut ask, &[CursorUp]);
    assert_eq!(ask.focus(), Some(Leave::Keep));
}

#[test]
fn three_answers_wrap_both_ways() {
    let mut ask = three();
    assert_eq!(ask.focus(), Some(Leave::Back));
    press(&mut ask, &[CursorRight]);
    assert_eq!(ask.focus(), Some(Leave::Keep));
    press(&mut ask, &[CursorLeft, CursorLeft]);
    assert_eq!(ask.focus(), Some(Leave::Discard));
    let (answers, sounds) = press(&mut ask, &[CursorDown, CursorDown, CursorDown]);
    assert_eq!(answers, [None; 3]);
    assert_eq!((ask.focus(), sounds.len()), (Some(Leave::Discard), 3));
}

#[test]
fn confirm_gives_the_focused_answer_and_plays_nothing() {
    let mut ask = three();
    press(&mut ask, &[CursorLeft]);
    let (answers, sounds) = press(&mut ask, &[Confirm]);
    assert_eq!(answers, [Some(Leave::Discard)]);
    assert!(sounds.is_empty(), "the screen plays the answer's sound");
}

#[test]
fn cancel_gives_its_answer_or_nothing() {
    let mut ask = three();
    assert_eq!(ask.cancel(), Some(Leave::Back));
    press(&mut ask, &[CursorRight]);
    assert_eq!(press(&mut ask, &[Cancel]).0, [Some(Leave::Back)]);
    // The focus doesn't matter to Cancel, nor Cancel to the focus.
    assert_eq!(ask.focus(), Some(Leave::Keep));
    let mut ask = two();
    assert_eq!(ask.cancel(), None);
    assert_eq!(press(&mut ask, &[Cancel]), (vec![None], vec![]));
}

#[test]
fn other_actions_give_nothing() {
    let mut ask = three();
    assert_eq!(press(&mut ask, &[Info]), (vec![None], vec![]));
    assert_eq!(ask.focus(), Some(Leave::Back));
}

#[test]
fn focusing_an_answer_not_offered_focuses_the_first() {
    let ask = two().focused_on(Leave::Back);
    assert_eq!(ask.focus(), Some(Leave::Keep));
    let none: Ask<Leave> = Ask::new("options.question.keep", []);
    assert_eq!(none.focus(), None);
}

#[test]
fn a_yes_no_is_confirm_or_cancel_and_moves_nowhere() {
    let mut ask = Ask::yes_no("options.question.restore", "options.question.answers");
    assert_eq!(ask.key(), "options.question.restore");
    let (answers, sounds) = press(&mut ask, &[CursorRight, CursorUp, Confirm, Cancel]);
    assert_eq!(answers, [None, None, Some(true), Some(false)]);
    assert!(sounds.is_empty());
}

#[test]
fn the_view_is_the_text_and_the_answers_in_words_or_the_keys_line() {
    let c = ctx();
    let view = three().view(&c);
    assert_eq!(view.text, "Keep these changes?");
    let labels: Vec<_> = view.answers.iter().map(|a| a.label.as_str()).collect();
    assert_eq!(labels, ["Keep", "Discard", "Back"]);
    assert_eq!((view.focused(), view.keys), (Some(Leave::Back), None));
    let yes_no = Ask::yes_no("options.question.restore", "options.question.answers").view(&c);
    assert_eq!(yes_no.text, "Restore every option to its default?");
    assert_eq!(yes_no.answers, []);
    assert_eq!(yes_no.keys.as_deref(), Some("f yes / d no"));
    assert_eq!(yes_no.focused(), None);
}

/// `view` painted alone on a console of `spec`, over a stale picture.
fn painted(view: &AskView<Leave>, spec: ConsoleSpec) -> GlyphBuffer {
    let c = ctx();
    let black = c.palette.engine(EngineColor::Background);
    let mut buf = GlyphBuffer::with_spec(spec, Cell::new('x', black, black));
    glyph::paint(&c.palette, view, &mut buf);
    buf
}

/// Row `y` of `buf`, as glyphs.
fn row(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(buf.width()))
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

#[test]
fn the_box_is_in_the_middle_over_whatever_is_under_it() {
    let view = three().view(&ctx());
    let buf = painted(&view, ConsoleSpec::text(40, 12));
    let rect = glyph::rect(&view, &buf);
    // The answers' line is wider than the question.
    assert_eq!((rect.x, rect.y, rect.w, rect.h), (4, 3, 31, glyph::HEIGHT));
    assert_eq!(row(&buf, 2), "x".repeat(40), "outside the box stays");
    let x = |n| "x".repeat(n);
    let question = format!("{}║ {:<27} ║{}", x(4), "Keep these changes?", x(5));
    assert_eq!(row(&buf, 5), question);
    let answers = format!("{}║  Keep     Discard     Back  ║{}", x(4), x(5));
    assert_eq!(row(&buf, 6), answers);
}

#[test]
fn the_widget_alone_snapshot() {
    let c = ctx();
    let mut ask = two();
    press(&mut ask, &[CursorRight]);
    let buf = painted(&ask.view(&c), ConsoleSpec::text(30, 8));
    insta::assert_snapshot!(buf.to_snapshot(&c.palette));
}
