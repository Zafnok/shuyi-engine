//! The name box and the name grid on the toy game's actions and text.

use shuyi_assets::keymap::{Chord, Key};

use super::*;
use crate::frame::color::EngineColor;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Rect};
use crate::toy::{Action, ToyCtx, ctx};
use Action::{Cancel, Confirm, CursorDown, CursorLeft, CursorRight, CursorUp};

type FrameInput = crate::screen::FrameInput<Action>;

const PROFILE: NameRules = NameRules {
    max_chars: 12,
    digits: true,
};
const LETTERS: NameRules = NameRules {
    max_chars: 4,
    digits: false,
};

/// A frame where `text` is typed and the keys `keys` are pressed.
fn typed(text: &str, keys: &[Key]) -> FrameInput {
    let chords = keys.iter().map(|&k| Chord::plain(k)).collect();
    FrameInput::new(vec![], 0.0, vec![]).with_typing(chords, text.chars().collect())
}

/// The sounds asked for since the last call.
fn sounds(c: &mut ToyCtx) -> Vec<String> {
    let cue = |r: crate::audio::AudioRequest<crate::toy::Voice>| r.cue().map(str::to_owned);
    c.audio.take().into_iter().filter_map(cue).collect()
}

#[test]
fn digits_are_allowed_only_where_the_rules_say() {
    let mut name = "Player ".to_owned();
    assert!(add_char(&mut name, '3', PROFILE));
    assert_eq!(name, "Player 3");
    let mut name = "Ab".to_owned();
    assert!(!add_char(&mut name, '3', LETTERS));
    assert!(!add_char(&mut name, '!', PROFILE));
    assert_eq!(name, "Ab");
    assert!(add_char(&mut name, '-', LETTERS));
    assert!(add_char(&mut name, '\'', LETTERS));
    assert_eq!(name, "Ab-'");
}

#[test]
fn the_rules_cap_the_length_and_refuse_a_leading_or_double_space() {
    let mut name = String::new();
    assert!(!add_char(&mut name, ' ', LETTERS));
    for c in "Ab c".chars() {
        assert!(add_char(&mut name, c, LETTERS), "{c}");
    }
    assert!(!add_char(&mut name, 'd', LETTERS));
    let mut name = "A ".to_owned();
    assert!(!add_char(&mut name, ' ', PROFILE));
}

#[test]
fn a_finished_name_is_trimmed_and_never_blank() {
    assert_eq!(finished(" Mei ").as_deref(), Some("Mei"));
    assert_eq!(finished("   "), None);
}

#[test]
fn the_box_types_deletes_and_is_done_on_the_text_keys() {
    let mut c = ctx();
    let mut b = NameBox::new("", PROFILE);
    assert_eq!(b.handle(&typed("Mei!", &[]), &mut c), NameEvent::Open);
    assert_eq!(b.name(), "Mei");
    assert_eq!(sounds(&mut c), ["ui_cancel"], "the ! is refused");
    assert_eq!(
        b.handle(&typed("", &[Key::Backspace]), &mut c),
        NameEvent::Open
    );
    assert_eq!(b.name(), "Me");
    assert_eq!(
        b.handle(&typed("i ", &[Key::Enter]), &mut c),
        NameEvent::Done
    );
    assert_eq!(b.name(), "Mei", "trimmed");
    assert_eq!(b.view(&c, "Name").typed, "Mei");
}

#[test]
fn the_box_refuses_done_on_a_blank_name_and_reports_cancel() {
    let mut c = ctx();
    let mut b = NameBox::new("", PROFILE);
    assert_eq!(b.handle(&typed("", &[Key::Enter]), &mut c), NameEvent::Open);
    assert_eq!(sounds(&mut c), ["ui_cancel"]);
    assert_eq!(
        b.handle(&typed("", &[Key::Escape]), &mut c),
        NameEvent::Cancel
    );
    assert!(sounds(&mut c).is_empty(), "Cancel's sound is the screen's");
}

#[test]
fn the_box_ignores_the_games_actions() {
    let mut c = ctx();
    let mut b = NameBox::new("A", PROFILE);
    let input = FrameInput::new(vec![Confirm, Cancel], 0.0, vec![]);
    assert_eq!(b.handle(&input, &mut c), NameEvent::Open);
    assert_eq!(b.name(), "A");
}

#[test]
fn the_grid_has_a_row_of_digits_only_where_the_rules_allow_them() {
    assert_eq!(grid(LETTERS).len(), 5);
    let rows = grid(PROFILE);
    assert_eq!(rows.len(), 6);
    assert_eq!(rows[4].first(), Some(&GridCell::Char('0')));
    assert_eq!(rows[4].last(), Some(&GridCell::Char('9')));
    assert_eq!(rows[5].last(), Some(&GridCell::Done));
}

#[test]
fn cancel_on_the_grid_deletes_then_asks_to_close() {
    let mut c = ctx();
    let mut e = NameEntry::new("A", PROFILE);
    assert_eq!(e.handle(Cancel, &mut c), NameEvent::Open);
    assert_eq!(e.name(), "");
    assert_eq!(e.handle(Cancel, &mut c), NameEvent::Cancel);
}

#[test]
fn the_grid_spells_a_digit_and_is_done() {
    let mut c = ctx();
    let mut e = NameEntry::new("P", PROFILE);
    // Down four rows to the digits; right to 2.
    for a in [CursorDown; 4] {
        e.handle(a, &mut c);
    }
    e.handle(CursorRight, &mut c);
    e.handle(CursorRight, &mut c);
    assert_eq!(e.handle(Confirm, &mut c), NameEvent::Open);
    assert_eq!(e.name(), "P2");
    // Down to the last row (Blank, under the 2), right to Done.
    e.handle(CursorDown, &mut c);
    e.handle(CursorRight, &mut c);
    e.handle(CursorRight, &mut c);
    assert_eq!(e.focus(), (5, 4));
    assert_eq!(e.handle(Confirm, &mut c), NameEvent::Done);
    assert_eq!(e.view(&c, "Name").spelled, "P2");
}

#[test]
fn the_grid_wraps_and_keeps_to_a_shorter_rows_last_cell() {
    let mut c = ctx();
    let mut e = NameEntry::new("", LETTERS);
    e.handle(CursorLeft, &mut c);
    assert_eq!(e.focus(), (0, 12), "round to the row's end");
    e.handle(CursorUp, &mut c);
    assert_eq!(e.focus(), (4, 4), "round to the last row, its last cell");
    e.handle(CursorRight, &mut c);
    assert_eq!(e.focus(), (4, 0));
    // Down from a long row's end to the short last row: its last cell.
    let mut down = NameEntry::new("", LETTERS);
    down.handle(CursorLeft, &mut c);
    for _ in 0..4 {
        down.handle(CursorDown, &mut c);
    }
    assert_eq!(down.focus(), (4, 4));
    // Done on a blank name is refused.
    e.handle(CursorLeft, &mut c);
    sounds(&mut c);
    assert_eq!(e.handle(Confirm, &mut c), NameEvent::Open);
    assert_eq!(sounds(&mut c), ["ui_cancel"]);
}

#[test]
fn the_grid_labels_its_cells_in_the_players_language() {
    let c = ctx();
    let view = NameEntry::new("", LETTERS).view(&c, "Name");
    let last: Vec<&str> = view.rows[4].iter().map(String::as_str).collect();
    assert_eq!(last, ["-", "'", "Blank", "Delete", "Done"]);
}

/// A buffer of `spec`, all background.
fn buffer(c: &ToyCtx, spec: ConsoleSpec) -> GlyphBuffer {
    let black = c.palette.engine(EngineColor::Background);
    GlyphBuffer::with_spec(spec, Cell::new(' ', black, black))
}

/// Row `y` of `buf`, trimmed.
fn row(buf: &GlyphBuffer, y: i32) -> String {
    let glyphs = (0..i32::from(buf.width())).map(|x| buf.get(x, y).map_or(' ', |c| c.glyph));
    glyphs.collect::<String>().trim().to_owned()
}

#[test]
fn the_typing_box_shows_the_name_with_a_cursor_and_the_hint() {
    let c = ctx();
    let mut buf = buffer(&c, ConsoleSpec::text(40, 9));
    let view = NameBox::new("Mei", PROFILE).view(&c, "Name");
    glyph::paint_typing(
        &c.palette,
        &mut buf,
        Rect::new(1, 1, 38, glyph::TYPING_HEIGHT),
        &view,
    );
    assert!(row(&buf, 1).starts_with("╔═ Name ═"), "{}", row(&buf, 1));
    assert!(row(&buf, 3).contains("Mei_"));
    assert!(row(&buf, 5).contains("Type a name on your keyboard."));
}

#[test]
fn the_grid_alone_snapshot() {
    let mut c = ctx();
    let mut e = NameEntry::new("Mo", LETTERS);
    e.handle(CursorDown, &mut c);
    let view = e.view(&c, "Name");
    let rows = u16::try_from(glyph::grid_height(&view) + 2).unwrap();
    let mut buf = buffer(&c, ConsoleSpec::text(46, rows));
    glyph::paint_grid(&c.palette, &mut buf, Rect::new(1, 1, 44, 0), &view);
    insta::assert_snapshot!(buf.to_snapshot(&c.palette));
}
