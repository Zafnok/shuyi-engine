//! The engine's hints name their keys in key spans: each switched hint,
//! built with the toy's `Ctx`, has exactly the keys' names in them, in
//! order, and reads as its plain text did.

use super::*;
use crate::dialogue::{SkipView, TextView, text_box};
use crate::frame::color::{EngineColor, Rgb};
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Rect};
use crate::input::fixed::{capture_abort_key_name, clear_slot_key_name, text_key_names};
use crate::input::help_keys::{all_key_names, cursor_keys_name, key_name};
use crate::screen::Screen;
use crate::screens::key_bindings::KeyBindingsScreen;
use crate::screens::layout_picker::LayoutPickerScreen;
use crate::screens::options::OptionsScreen;
use crate::screens::profiles::ProfilesScreen;
use crate::screens::text_keys_help;
use crate::toy::{self, Action, KEY_ROWS, Talk, Toy, ToyCtx, ctx};
use crate::widgets::Ask;

/// The texts of `text`'s key spans, a no-break space read as a space.
fn keys(text: &StyledText) -> Vec<String> {
    let spans = text
        .spans()
        .iter()
        .filter(|s| matches!(s.role, Some(TextRole::Key | TextRole::Arrows)));
    spans.map(|s| s.text.replace(NO_BREAK_SPACE, " ")).collect()
}

/// The name of `action`'s key.
fn name(c: &ToyCtx, action: Action) -> String {
    key_name(c.help_keys(), action)
}

#[test]
fn every_text_reads_the_same_styled_or_plain() {
    let c = ctx();
    let mut texts = toy::english_keys();
    texts.sort_unstable();
    assert!(texts.len() > 100, "{}", texts.len());
    for key in texts {
        let styled = c.text_styled(&key, &[]);
        assert_eq!(styled.plain(), c.text_with(&key, &[]), "{key}");
        let label = Label::key(key.as_str()).with_arg("name", "Ada");
        assert_eq!(c.label_styled(&label).plain(), c.label(&label), "{key}");
    }
    let text = Label::Text("as it is".to_owned());
    assert_eq!(c.label_styled(&text), StyledText::from("as it is"));
}

#[test]
fn the_text_box_and_a_question_name_their_keys_in_key_spans() {
    let c = ctx();
    let mut talk = Talk::new(&c, "hall");
    talk.text.reveal();
    let waiting = talk.view(&c).waiting.unwrap();
    assert_eq!(keys(&waiting.key), [name(&c, Action::Confirm)]);
    let ask = Ask::yes_no("options.question.restore", "options.question.answers");
    let answers = ask.view(&c).keys.unwrap();
    let yes_no = [name(&c, Action::Confirm), name(&c, Action::Cancel)];
    assert_eq!(keys(&answers), yes_no);
}

#[test]
fn the_screens_help_lines_name_their_keys_in_key_spans() {
    let c = ctx();
    let km = c.help_keys();
    let (cursor, confirm, cancel) = (
        cursor_keys_name(km),
        name(&c, Action::Confirm),
        name(&c, Action::Cancel),
    );
    let options = OptionsScreen::<Toy>::new(toy::option_rows).help(&c);
    assert_eq!(
        keys(&options),
        [cursor.clone(), confirm.clone(), cancel.clone()]
    );
    let profiles = ProfilesScreen::<Toy>::new(&c).help(&c);
    assert_eq!(
        keys(&profiles),
        [cursor.clone(), confirm.clone(), cancel.clone()]
    );
    // The key bindings screen starts on a slot, whose help names the
    // fixed key that clears it.
    let bindings = KeyBindingsScreen::new(&c, &KEY_ROWS).help(&c);
    let clear = clear_slot_key_name();
    assert_eq!(keys(&bindings), [cursor, confirm, clear, cancel]);
    // The layout picker's first line hands in every key of its actions.
    let all = |action| all_key_names(km, action);
    let picker = LayoutPickerScreen::help(&c);
    let first = [Action::CursorUp, Action::CursorDown, Action::Confirm].map(all);
    assert_eq!(keys(&picker), first);
    assert_eq!(keys(&text_keys_help(&c)), text_key_names());
}

#[test]
fn the_key_bindings_capture_help_names_its_back_out_key() {
    let mut c = ctx();
    let mut s = KeyBindingsScreen::new(&c, &KEY_ROWS);
    let input = crate::screen::FrameInput::new(vec![Action::Confirm], 0.0, vec![]);
    s.update(&mut c, &input);
    let help = s.help(&c);
    assert_eq!(keys(&help), [capture_abort_key_name()]);
    assert_eq!(help.plain(), "Press the key to put here · Escape back");
}

#[test]
fn a_skip_question_paints_its_keys_in_the_key_colours() {
    let mut c = ctx();
    c.palette = toy::key_palette(&c);
    let (gold, purple) = (Rgb::new(255, 200, 0), Rgb::new(40, 0, 60));
    let mut view = TextView::new(Vec::new(), 0);
    view.skip = Some(SkipView::new(
        "Skip?",
        c.text_styled("dialogue.skip_answers", &[]),
    ));
    let spec = ConsoleSpec::text(40, 10);
    let mut buf =
        GlyphBuffer::with_spec(spec, Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0)));
    let place = text_box::glyph::BoxPlace::new(Rect::new(0, 3, 40, 7), 2, 5);
    text_box::glyph::paint(&c, &mut buf, place, &view, None);
    // The answers, from the text's column 2 of row 6.
    let (yes, no) = (name(&c, Action::Confirm), name(&c, Action::Cancel));
    let expected = format!("{yes} yes / {no} no");
    let cells: Vec<&Cell> = (2..)
        .zip(expected.chars())
        .filter_map(|(x, _)| buf.get(x, 6))
        .collect();
    let line: String = cells.iter().map(|c| c.glyph).collect();
    assert_eq!(line, expected);
    let dim = c.palette.engine(EngineColor::TextDim);
    let panel = c.palette.engine(EngineColor::Panel);
    let no_at = yes.len() + " yes / ".len();
    for (i, cell) in cells.iter().enumerate() {
        let in_key = i < yes.len() || (no_at..no_at + no.len()).contains(&i);
        let colours = if in_key { (gold, purple) } else { (dim, panel) };
        assert_eq!((cell.fg, cell.bg), colours, "{i}: {:?}", cell.glyph);
    }
}
