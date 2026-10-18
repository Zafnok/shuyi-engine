//! The Key bindings screen's glyph skin, painting views of the toy game's
//! rows.

use shuyi_assets::keymap::sample::KeyGroup;
use shuyi_assets::keymap::{Button, Chord, Key, SLOTS};

use super::*;
use crate::frame::console::ConsoleSpec;
use crate::input::help_keys::NOT_MAPPED;
use crate::input::pad::PadKind;
use crate::screens::key_bindings::view::RestoreView;
use crate::screens::key_bindings::{CAPTURE_BUTTON_PROMPT, CAPTURE_PROMPT, CHOICES};
use crate::toy::{Action, ToyCtx};
use crate::toy::{KEY_ROWS as ROWS, ctx};

type Ctx = ToyCtx;
type KeyBindingsView = super::KeyBindingsView<Action>;
type RowView = super::RowView<Action>;
type GroupView = crate::screens::key_bindings::view::GroupView<Action>;
type KeyRow = crate::screens::key_bindings::KeyRow<Action>;

/// The console the skin is laid out for.
const PC: ConsoleSpec = ConsoleSpec::text(100, 32);

/// The row of `view` for `action`, mutable.
fn row_mut(view: &mut KeyBindingsView, action: Action) -> &mut RowView {
    let mut rows = view.groups.iter_mut().flat_map(|g| g.rows.iter_mut());
    rows.find(|r| r.action == action).unwrap()
}

/// A view of every action in [`ROWS`], the first slot of each holding a
/// key and the others empty, nothing focused or open, on `side`.
fn view(side: Side) -> KeyBindingsView {
    let row = |&(action, label, _): &KeyRow| RowView {
        action,
        label: ctx().text(label).to_owned(),
        fixed: Vec::new(),
        slots: (0..SLOTS)
            .map(|j| {
                if j == 0 {
                    SlotView::Bound("f".to_owned())
                } else {
                    SlotView::Empty
                }
            })
            .collect(),
        focus: None,
        unmapped: false,
        blocks_leaving: false,
        lost_key: false,
    };
    let group = |heading: &str, required: bool, group: KeyGroup| GroupView {
        heading: heading.to_owned(),
        required,
        rows: ROWS
            .iter()
            .filter(|(a, ..)| a.is_required() == required && a.group() == group)
            .map(row)
            .collect(),
    };
    KeyBindingsView {
        title: "Key bindings".to_owned(),
        switch: SwitchView {
            keyboard: "Keyboard · Left-handed".to_owned(),
            controller: "Controller".to_owned(),
            shown: side,
            focused: false,
        },
        columns: (1..=SLOTS).map(|n| format!("Key {n}")).collect(),
        groups: vec![
            group("Must have a key", true, KeyGroup::General),
            group("Optional", false, KeyGroup::General),
            group("In the dialogue log", false, KeyGroup::DialogueLog),
        ],
        restore: RestoreView {
            label: "Restore defaults".to_owned(),
            focused: false,
        },
        not_mapped: NOT_MAPPED.to_owned(),
        message: None,
        question: None,
        choices: None,
        help: "help".to_owned(),
    }
}

fn painted(c: &Ctx, view: &KeyBindingsView) -> GlyphBuffer {
    let stale = Cell::new(
        'x',
        c.palette.engine(EngineColor::Warning),
        c.palette.engine(EngineColor::Warning),
    );
    let mut buf = GlyphBuffer::with_spec(PC, stale);
    paint(c, view, &mut buf);
    buf
}

fn text(buf: &GlyphBuffer, y: i32) -> String {
    (0..i32::from(buf.width()))
        .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
        .collect()
}

/// The console row of `action` in `view`.
fn y_of(view: &KeyBindingsView, action: Action) -> i32 {
    let placed = place(view);
    let at = view.rows().position(|r| r.action == action).unwrap();
    placed.actions[at]
}

/// The column `word` starts at in console row `y` (in cells).
fn x_of(buf: &GlyphBuffer, y: i32, word: &str) -> i32 {
    let row = text(buf, y);
    let cells = row[..row.find(word).unwrap()].chars().count();
    i32::try_from(cells).unwrap()
}

#[test]
fn the_title_switch_columns_and_groups_have_their_rows() {
    let c = ctx();
    let v = view(Side::Keyboard);
    let buf = painted(&c, &v);
    assert!(text(&buf, 1).contains("─ Key bindings ─"));
    assert!(text(&buf, 2).contains("  Keyboard · Left-handed    Controller  "));
    // The slot columns' headings are right under the switch.
    let columns = text(&buf, 3);
    assert!(columns.contains("Key 1") && columns.contains("Key 3"));
    assert!(text(&buf, 4).contains("Must have a key"));
    // Seven required actions, then a blank row and the optional ones, then
    // the dialogue log's group.
    let placed = place(&v);
    assert_eq!(placed.headings, [4, 13, 25]);
    assert_eq!(placed.actions[0], 5);
    assert_eq!(placed.actions[6], 11);
    assert_eq!(placed.actions[7], 14);
    assert_eq!(placed.actions[16], 23);
    assert_eq!(placed.actions[17], 26);
    let blank = |y| {
        text(&buf, y)
            .trim_matches(|ch| ch == ' ' || ch == '│')
            .is_empty()
    };
    assert!(blank(12) && blank(24));
    assert!(text(&buf, 13).contains("Optional"));
    assert!(text(&buf, 25).contains("In the dialogue log"));
    // Restore defaults is one blank row under the last action, on the
    // panel's last row.
    assert_eq!(placed.restore, 28);
    assert!(text(&buf, 28).contains("Restore defaults"));
    assert!(text(&buf, 29).contains('└'));
    assert_eq!(text(&buf, i32::from(PC.rows) - 1).trim(), "help");
}

#[test]
fn it_covers_the_whole_buffer() {
    let c = ctx();
    let stale = c.palette.engine(EngineColor::Warning);
    let buf = painted(&c, &view(Side::Keyboard));
    let left = (0..i32::from(PC.rows))
        .flat_map(|y| (0..i32::from(PC.cols)).map(move |x| (x, y)))
        .filter(|&(x, y)| buf.get(x, y).is_some_and(|c| c.bg == stale))
        .count();
    assert_eq!(left, 0, "an opaque screen covers the whole buffer");
}

#[test]
fn a_row_shows_its_slots_fixed_keys_and_not_mapped_note() {
    let c = ctx();
    let mut v = view(Side::Keyboard);
    row_mut(&mut v, Action::Cancel).fixed = vec!["Escape".to_owned()];
    row_mut(&mut v, Action::EndTurn).fixed = vec!["Space".to_owned(), "x".to_owned()];
    let buf = painted(&c, &v);
    let cancel = text(&buf, y_of(&v, Action::Cancel));
    assert!(cancel.contains(" Cancel  + Escape "), "{cancel}");
    let end = text(&buf, y_of(&v, Action::EndTurn));
    assert!(end.contains(" End turn  + Space/x "), "{end}");
    // The first slot's key sits one cell into its column; the empty ones
    // show a dot in the same place.
    let y = y_of(&v, Action::Confirm);
    let at = |x| *buf.get(x, y).unwrap();
    assert_eq!(at(slot_x(0) + 1).glyph, 'f');
    assert_eq!(at(slot_x(1) + 1).glyph, '·');
    assert_eq!(at(slot_x(2) + 1).glyph, '·');
    assert_eq!(at(slot_x(0) + 1).fg, c.palette.engine(EngineColor::Text));
    assert_eq!(at(slot_x(1) + 1).fg, c.palette.engine(EngineColor::TextDim));
    // The fixed keys are dim.
    let x = x_of(&buf, y_of(&v, Action::Cancel), "+ Escape");
    assert_eq!(
        buf.get(x, y_of(&v, Action::Cancel)).unwrap().fg,
        c.palette.engine(EngineColor::TextDim)
    );
}

/// Optional and unmapped: dim. Required and unmapped: the warning colour.
#[test]
fn the_not_mapped_note_is_dim_unless_it_blocks_leaving() {
    let c = ctx();
    let mut v = view(Side::Keyboard);
    let optional = row_mut(&mut v, Action::Select);
    optional.unmapped = true;
    let required = row_mut(&mut v, Action::Confirm);
    required.unmapped = true;
    required.blocks_leaving = true;
    let buf = painted(&c, &v);
    let note_fg = |action| {
        let y = y_of(&v, action);
        assert!(text(&buf, y).contains(NOT_MAPPED), "{action:?}");
        let x = x_of(&buf, y, NOT_MAPPED);
        assert_eq!(x, slot_x(SLOTS) + 1);
        buf.get(x, y).unwrap().fg
    };
    assert_eq!(
        note_fg(Action::Select),
        c.palette.engine(EngineColor::TextDim)
    );
    assert_eq!(
        note_fg(Action::Confirm),
        c.palette.engine(EngineColor::Warning)
    );
    // No note on a row that has a key.
    assert!(!text(&buf, y_of(&v, Action::Cancel)).contains(NOT_MAPPED));
}

#[test]
fn the_focused_slot_is_a_bar_and_while_capturing_it_shows_the_prompt() {
    let c = ctx();
    let bar = c.palette.engine(EngineColor::Focus);
    let mut v = view(Side::Keyboard);
    let confirm = row_mut(&mut v, Action::Confirm);
    confirm.focus = Some(0);
    let y = y_of(&v, Action::Confirm);
    let buf = painted(&c, &v);
    let bg = |x| buf.get(x, y).unwrap().bg;
    assert_eq!(bg(slot_x(0)), bar);
    assert_eq!(bg(slot_x(0) + 15), bar);
    assert_ne!(bg(slot_x(0) + 16), bar);
    assert_ne!(bg(slot_x(1)), bar);
    assert!(text(&buf, y).contains(" f    "));
    // The focused row's label takes the highlight colour.
    assert_eq!(
        buf.get(LABEL_X, y).unwrap().fg,
        c.palette.engine(EngineColor::Highlight)
    );
    // Capturing: the same bar, with the prompt in it. An empty focused slot
    // is an empty bar.
    let confirm = row_mut(&mut v, Action::Confirm);
    confirm.focus = Some(1);
    confirm.slots[1] = SlotView::Capturing("Press a key…".to_owned());
    let buf = painted(&c, &v);
    assert!(text(&buf, y).contains("Press a key…"));
    assert_eq!(buf.get(slot_x(1), y).unwrap().bg, bar);
    assert_ne!(buf.get(slot_x(0), y).unwrap().bg, bar);
    let confirm = row_mut(&mut v, Action::Confirm);
    confirm.slots[1] = SlotView::Empty;
    let buf = painted(&c, &v);
    assert_eq!(buf.get(slot_x(1), y).unwrap().bg, bar);
    assert_eq!(buf.get(slot_x(1) + 1, y).unwrap().glyph, ' ');
}

#[test]
fn a_row_that_lost_its_key_is_white_unless_the_cursor_is_on_it() {
    let c = ctx();
    let mut v = view(Side::Keyboard);
    row_mut(&mut v, Action::Info).lost_key = true;
    let y = y_of(&v, Action::Info);
    let buf = painted(&c, &v);
    assert_eq!(
        buf.get(LABEL_X, y).unwrap().fg,
        c.palette.engine(EngineColor::Bright)
    );
    let other = y_of(&v, Action::Rewind);
    assert_eq!(
        buf.get(LABEL_X, other).unwrap().fg,
        c.palette.engine(EngineColor::Text)
    );
    // Its own row, when focused, keeps the focus colour.
    row_mut(&mut v, Action::Info).focus = Some(0);
    let buf = painted(&c, &v);
    assert_eq!(
        buf.get(LABEL_X, y).unwrap().fg,
        c.palette.engine(EngineColor::Highlight)
    );
}

#[test]
fn the_switch_highlights_the_shown_side_and_is_a_bar_when_focused() {
    let c = ctx();
    let mut v = view(Side::Controller);
    let buf = painted(&c, &v);
    let fg = |buf: &GlyphBuffer, word: &str| buf.get(x_of(buf, 2, word), 2).unwrap().fg;
    assert_eq!(
        fg(&buf, "Controller"),
        c.palette.engine(EngineColor::Highlight)
    );
    assert_eq!(fg(&buf, "Keyboard"), c.palette.engine(EngineColor::TextDim));
    // On the switch, the shown side is a bar.
    v.switch.focused = true;
    let buf = painted(&c, &v);
    let bar = c.palette.engine(EngineColor::Focus);
    let x = x_of(&buf, 2, "Controller");
    let bg = |x| buf.get(x, 2).unwrap().bg;
    assert_eq!(bg(x - 1), bar);
    assert_eq!(bg(x + 10), bar);
    assert_ne!(bg(x + 11), bar);
    assert_ne!(bg(x - 2), bar);
    v.switch.shown = Side::Keyboard;
    let buf = painted(&c, &v);
    assert_eq!(buf.get(LABEL_X - 1, 2).unwrap().bg, bar);
    assert_ne!(buf.get(x, 2).unwrap().bg, bar);
}

#[test]
fn the_controller_side_headings_and_buttons_are_painted_as_given() {
    let c = ctx();
    let mut v = view(Side::Controller);
    v.columns = (1..=SLOTS).map(|n| format!("Button {n}")).collect();
    v.groups[0].heading = "Must have a button".to_owned();
    row_mut(&mut v, Action::EndTurn).slots[0] = SlotView::Bound("Start".to_owned());
    let buf = painted(&c, &v);
    assert!(text(&buf, 3).contains("Button 1") && text(&buf, 3).contains("Button 3"));
    assert!(!text(&buf, 3).contains("Key 1"));
    assert!(text(&buf, 4).contains("Must have a button"));
    assert!(text(&buf, y_of(&v, Action::EndTurn)).contains(" Start "));
}

#[test]
fn the_restore_row_is_a_bar_when_focused() {
    let c = ctx();
    let mut v = view(Side::Keyboard);
    let y = place(&v).restore;
    let buf = painted(&c, &v);
    let bar = c.palette.engine(EngineColor::Focus);
    assert_eq!(buf.get(LABEL_X, y).unwrap().glyph, 'R');
    assert_ne!(buf.get(LABEL_X, y).unwrap().bg, bar);
    v.restore.focused = true;
    let buf = painted(&c, &v);
    assert_eq!(buf.get(LABEL_X - 1, y).unwrap().bg, bar);
    assert_eq!(buf.get(LABEL_X, y).unwrap().glyph, 'R');
    assert!(text(&buf, y).contains(" Restore defaults "));
}

/// The message sits one blank row under the panel's bottom border, in the
/// warning colour.
#[test]
fn the_message_is_painted_under_the_panel_in_the_warning_colour() {
    let c = ctx();
    let mut v = view(Side::Keyboard);
    v.message = Some("Give Cancel a key first".to_owned());
    let buf = painted(&c, &v);
    let bottom = PANEL.y + PANEL.h - 1;
    assert!(text(&buf, bottom).contains('└'));
    assert_eq!(MESSAGE_ROW, bottom + 1);
    assert_eq!(MESSAGE_ROW, i32::from(PC.rows) - 2, "over the help line");
    let message = text(&buf, MESSAGE_ROW);
    assert_eq!(message.trim(), "Give Cancel a key first");
    let x = x_of(&buf, MESSAGE_ROW, "Give");
    assert_eq!(
        buf.get(x, MESSAGE_ROW).unwrap().fg,
        c.palette.engine(EngineColor::Warning)
    );
}

#[test]
fn the_question_is_a_box_in_the_middle_with_its_answers_under_it() {
    let c = ctx();
    let mut v = view(Side::Keyboard);
    v.question = Some(QuestionView {
        text: "Restore the default keys for Right-handed?".to_owned(),
        answers: "f yes / d no".to_owned(),
    });
    let buf = painted(&c, &v);
    let top = (i32::from(PC.rows) - QUESTION_H) / 2;
    let border = format!("╔{}╗", "═".repeat(44));
    assert!(text(&buf, top).contains(&border), "{}", text(&buf, top));
    assert!(text(&buf, top + 2).contains("║ Restore the default keys for Right-handed? ║"));
    assert!(text(&buf, top + 3).contains("║ f yes / d no "));
    assert!(text(&buf, top + 5).contains('╚'));
    // No question, no box.
    v.question = None;
    let buf = painted(&c, &v);
    assert!(!(0..i32::from(PC.rows)).any(|y| text(&buf, y).contains('║')));
}

/// The choices go under the slot, or over it on the last rows.
#[test]
fn the_choices_are_drawn_under_the_slot_or_over_it_on_the_last_rows() {
    let c = ctx();
    let bar = c.palette.engine(EngineColor::Focus);
    let choices = |focused| ChoicesView {
        lines: CHOICES.map(|key| c.text(key).to_owned()).to_vec(),
        focused,
        slot: 1,
    };
    let mut v = view(Side::Controller);
    row_mut(&mut v, Action::Info).focus = Some(1);
    v.choices = Some(choices(0));
    let (x, y) = (slot_x(1), y_of(&v, Action::Info));
    let buf = painted(&c, &v);
    assert_eq!(buf.get(x, y + 1).unwrap().glyph, '╔');
    assert_eq!(buf.get(x + CHOICE_W - 1, y + CHOICE_H).unwrap().glyph, '╝');
    assert!(text(&buf, y + 2).contains("║ Change   ║"));
    assert!(text(&buf, y + 3).contains("║ Clear    ║"));
    assert_eq!(buf.get(x + 1, y + 2).unwrap().bg, bar);
    assert_eq!(buf.get(x + CHOICE_W - 2, y + 2).unwrap().bg, bar);
    assert_ne!(buf.get(x + 1, y + 3).unwrap().bg, bar);
    v.choices = Some(choices(1));
    let buf = painted(&c, &v);
    assert_eq!(buf.get(x + 1, y + 3).unwrap().bg, bar);
    assert_ne!(buf.get(x + 1, y + 2).unwrap().bg, bar);
    assert!(text(&buf, y + 2).contains("║ Change   ║"));
    // Only under the last row it doesn't fit.
    for (row, over) in [(15, false), (16, false), (17, true)] {
        let mut v = view(Side::Controller);
        let action = ROWS[row].0;
        row_mut(&mut v, action).focus = Some(0);
        v.choices = Some(ChoicesView {
            slot: 0,
            ..choices(0)
        });
        let buf = painted(&c, &v);
        let y = y_of(&v, action);
        let top = if over { y - CHOICE_H } else { y + 1 };
        assert_eq!(buf.get(slot_x(0), top).unwrap().glyph, '╔', "row {row}");
        let bottom = top + CHOICE_H - 1;
        assert_eq!(buf.get(slot_x(0), bottom).unwrap().glyph, '╚', "row {row}");
        assert!(bottom < PANEL.y + PANEL.h - 1, "inside the panel");
        // The slot itself stays visible.
        assert_eq!(buf.get(slot_x(0), y).unwrap().bg, bar);
    }
}

#[test]
fn choices_with_no_focused_row_are_not_painted() {
    let c = ctx();
    let mut v = view(Side::Controller);
    v.choices = Some(ChoicesView {
        lines: vec!["Change".to_owned(), "Clear".to_owned()],
        focused: 0,
        slot: 0,
    });
    let buf = painted(&c, &v);
    assert!(!(0..i32::from(PC.rows)).any(|y| text(&buf, y).contains('╔')));
}

#[test]
fn every_chord_name_fits_a_slot() {
    for &key in Key::ALL {
        let name = Chord::shifted(key).to_string();
        assert!(name.chars().count() <= SLOT_W, "{name}");
    }
    assert!(slot_x(SLOTS) + 1 + i32::try_from(NOT_MAPPED.len()).unwrap() < PANEL.x + PANEL.w);
    assert_eq!(slot_x(0), SLOTS_X);
    assert_eq!(slot_x(2), SLOTS_X + 2 * SLOT_PITCH);
}

#[test]
fn every_button_name_fits_a_slot() {
    let kinds = [
        PadKind::Xbox,
        PadKind::PlayStation,
        PadKind::PlayStation4,
        PadKind::Nintendo,
        PadKind::Generic,
    ];
    for kind in kinds {
        for &button in Button::ALL {
            let name = kind.button_name(button);
            assert!(name.chars().count() <= SLOT_W, "{name}");
        }
    }
    assert!(ctx().text(CAPTURE_BUTTON_PROMPT).chars().count() <= SLOT_W);
    assert!(ctx().text(CAPTURE_PROMPT).chars().count() <= SLOT_W);
    for choice in CHOICES.map(|key| ctx().text(key).to_owned()) {
        assert!(i32::try_from(choice.chars().count()).unwrap() <= CHOICE_W - 4);
    }
}
