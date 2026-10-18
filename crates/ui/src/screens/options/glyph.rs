//! The glyph look of the Options screen: paints an [`OptionsView`] as a
//! panel of text rows, in the style of the Key bindings screen. Everything
//! about the look lives here (where things go, the box characters, the
//! slider's blocks, the colours); nothing here decides what the screen
//! shows or does. Another look is another `paint` of the same view.

use super::view::{LeaveView, OptionsView, QuestionView, RowView, ValueView};
use crate::ctx::{Ctx, GameData};
use crate::frame::color::EngineColor;
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::prelude::*;
use crate::screens::print_centred;

/// The panel, in cells, at its smallest: it grows down to fit its rows
/// ([`panel`]).
const PANEL: Rect = Rect::new(20, 3, 60, 20);
/// Column of the row labels.
const LABEL_X: i32 = PANEL.x + 4;
/// Column of the row values.
const VALUE_X: i32 = PANEL.x + 30;
/// Panel rows that aren't the screen's rows: the top border, a blank row
/// under it and the bottom border.
const PANEL_FRAME_H: i32 = 3;
/// Height of a question's box: its two lines, a blank row above and below,
/// and the border.
const QUESTION_H: i32 = 6;
/// Cells between two answers of the question asked on leaving.
const CHOICE_GAP: usize = 3;
/// Cells in a volume's bar.
const VOLUME_BAR: u8 = 10;
/// Cells of the number box, inside its highlight.
const BOX_W: usize = 4;

/// A volume as a bar of [`VOLUME_BAR`] cells (a half-filled cell for half
/// a cell's worth or more) and its number.
fn volume_text(level: u8, max: u8) -> String {
    let level = level.min(max);
    let per_cell = (max / VOLUME_BAR).max(1);
    let full = usize::from(level / per_cell).min(usize::from(VOLUME_BAR));
    // (A cell worth 1 has no half: its remainder is always 0.)
    let half = usize::from(level % per_cell >= per_cell.div_ceil(2));
    let empty = usize::from(VOLUME_BAR).saturating_sub(full + half);
    let bar = ["█".repeat(full), "▒".repeat(half), "░".repeat(empty)].concat();
    format!("{bar} {level:>3}")
}

/// The panel for `view`: [`PANEL`], taller if its rows and the blank rows
/// between their groups need more.
fn panel(view: &OptionsView) -> Rect {
    let gaps = view.rows.iter().filter(|r| r.starts_group).count();
    let lines = i32::try_from(view.rows.len() + gaps).unwrap_or(i32::MAX);
    let h = PANEL.h.max(lines.saturating_add(PANEL_FRAME_H));
    Rect::new(PANEL.x, PANEL.y, PANEL.w, h)
}

/// Paints `view` over the whole of `buf`.
pub fn paint<G: GameData>(ctx: &Ctx<G>, view: &OptionsView, buf: &mut GlyphBuffer) {
    let c = |e| ctx.palette.engine(e);
    let (black, bg) = (c(EngineColor::Background), c(EngineColor::Panel));
    let (text, dim) = (c(EngineColor::Text), c(EngineColor::TextDim));
    buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
    let panel = panel(view);
    buf.fill_rect(panel, Cell::new(' ', text, bg));
    buf.draw_box(panel, BoxStyle::Single, c(EngineColor::Border), bg);
    let title = format!(" {} ", view.title);
    buf.print(PANEL.x + 2, PANEL.y, &title, c(EngineColor::Highlight), bg);

    let mut y = PANEL.y + 2;
    for row in &view.rows {
        if row.starts_group {
            y += 1;
        }
        paint_row(ctx, buf, row, y);
        y += 1;
    }

    if let Some(message) = &view.message {
        // The row under the panel's.
        let row = panel.y + panel.h + 1;
        print_centred(buf, row, message, c(EngineColor::Highlight), black);
    }
    if let Some(question) = &view.question {
        paint_question(ctx, buf, question);
    }
    if let Some(leaving) = &view.leaving {
        paint_leave(ctx, buf, leaving);
    }
    let bottom = i32::from(buf.height()) - 1;
    print_centred(buf, bottom, &view.help, dim, black);
}

/// Paints `row` on console row `y`: the focused row's label is a highlight
/// bar; a value the cursor changes sits between two arrows; the number box
/// is a highlight bar where the value was.
fn paint_row<G: GameData>(ctx: &Ctx<G>, buf: &mut GlyphBuffer, row: &RowView, y: i32) {
    let c = |e| ctx.palette.engine(e);
    let (bg, text, dim) = (
        c(EngineColor::Panel),
        c(EngineColor::Text),
        c(EngineColor::TextDim),
    );
    let bar = c(EngineColor::Focus);
    if row.focused {
        buf.print(LABEL_X - 1, y, &format!(" {} ", row.label), bg, bar);
    } else {
        buf.print(LABEL_X, y, &row.label, text, bg);
    }
    let (value, adjustable) = match &row.value {
        ValueView::None => return,
        ValueView::NumberBox { digits, typing } => {
            let caret = if *typing { "_" } else { "" };
            let shown = format!(" {:<BOX_W$} ", format!("{digits}{caret}"));
            buf.print(VALUE_X - 1, y, &shown, bg, bar);
            return;
        }
        ValueView::Text { text, adjustable } => (text.clone(), *adjustable),
        ValueView::Volume { level, max } => (volume_text(*level, *max), true),
    };
    let value_fg = if row.focused {
        c(EngineColor::Highlight)
    } else {
        text
    };
    buf.print(VALUE_X, y, &value, value_fg, bg);
    if row.focused && adjustable {
        buf.print(VALUE_X - 2, y, "◄", dim, bg);
        let after = VALUE_X + i32::try_from(value.chars().count()).unwrap_or(0) + 1;
        buf.print(after, y, "►", dim, bg);
    }
}

/// Paints `question` in a double-bordered box in the middle of the screen,
/// with its answers under it (the look of the Key bindings screen's
/// question).
fn paint_question<G: GameData>(ctx: &Ctx<G>, buf: &mut GlyphBuffer, question: &QuestionView) {
    let c = |e| ctx.palette.engine(e);
    let bg = c(EngineColor::Panel);
    let widest = question
        .text
        .chars()
        .count()
        .max(question.answers.chars().count());
    let w = i32::try_from(widest).unwrap_or(0) + 4;
    let rect = Rect::new(
        (i32::from(buf.width()) - w) / 2,
        (i32::from(buf.height()) - QUESTION_H) / 2,
        w,
        QUESTION_H,
    );
    buf.fill_rect(rect, Cell::new(' ', c(EngineColor::Text), bg));
    buf.draw_box(rect, BoxStyle::Double, c(EngineColor::Focus), bg);
    buf.print(
        rect.x + 2,
        rect.y + 2,
        &question.text,
        c(EngineColor::Text),
        bg,
    );
    let dim = c(EngineColor::TextDim);
    buf.print(rect.x + 2, rect.y + 3, &question.answers, dim, bg);
}

/// The answers of `leaving` on one line: each name with a space either
/// side, [`CHOICE_GAP`] cells apart. Returns the line and where each
/// answer starts in it.
fn choices_line(leaving: &LeaveView) -> (String, Vec<usize>) {
    let mut line = String::new();
    let mut starts = Vec::new();
    for (i, choice) in leaving.choices.iter().enumerate() {
        if i > 0 {
            line.push_str(&" ".repeat(CHOICE_GAP));
        }
        starts.push(line.chars().count());
        line.push(' ');
        line.push_str(&choice.label);
        line.push(' ');
    }
    (line, starts)
}

/// Paints the question asked on leaving in a double-bordered box in the
/// middle of the screen (the look of [`paint_question`]), its answers on
/// the line under it, the one in focus as a highlight bar.
fn paint_leave<G: GameData>(ctx: &Ctx<G>, buf: &mut GlyphBuffer, leaving: &LeaveView) {
    let c = |e| ctx.palette.engine(e);
    let (bg, text) = (c(EngineColor::Panel), c(EngineColor::Text));
    let (line, starts) = choices_line(leaving);
    let widest = leaving.text.chars().count().max(line.chars().count());
    let w = i32::try_from(widest).unwrap_or(0) + 4;
    let rect = Rect::new(
        (i32::from(buf.width()) - w) / 2,
        (i32::from(buf.height()) - QUESTION_H) / 2,
        w,
        QUESTION_H,
    );
    buf.fill_rect(rect, Cell::new(' ', text, bg));
    buf.draw_box(rect, BoxStyle::Double, c(EngineColor::Focus), bg);
    buf.print(rect.x + 2, rect.y + 2, &leaving.text, text, bg);
    let bar = c(EngineColor::Focus);
    for (choice, start) in leaving.choices.iter().zip(starts) {
        let x = rect.x + 2 + i32::try_from(start).unwrap_or(0);
        let shown = format!(" {} ", choice.label);
        if choice.focused {
            buf.print(x, rect.y + 3, &shown, bg, bar);
        } else {
            buf.print(x, rect.y + 3, &shown, text, bg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::console::ConsoleSpec;
    use crate::toy::{ToyCtx, ctx};

    /// The console the skin is laid out for.
    const PC: ConsoleSpec = ConsoleSpec::text(100, 32);

    /// Row `y` of `view` as painted, trimmed.
    fn painted_row(c: &ToyCtx, view: &OptionsView, y: i32) -> String {
        let black = c.palette.engine(EngineColor::Background);
        let mut buf = GlyphBuffer::with_spec(PC, Cell::new(' ', black, black));
        paint(c, view, &mut buf);
        let glyphs = (0..i32::from(PC.cols)).map(|x| buf.get(x, y).map_or(' ', |c| c.glyph));
        glyphs.collect::<String>().trim().to_owned()
    }

    fn row(label: &str, value: ValueView, focused: bool) -> RowView {
        RowView {
            key: "options.row.text_speed",
            label: label.to_owned(),
            value,
            focused,
            starts_group: false,
        }
    }

    fn view(rows: Vec<RowView>) -> OptionsView {
        OptionsView {
            title: "Options".to_owned(),
            rows,
            message: None,
            question: None,
            leaving: None,
            help: "help".to_owned(),
        }
    }

    fn words(text: &str, adjustable: bool) -> ValueView {
        ValueView::Text {
            text: text.to_owned(),
            adjustable,
        }
    }

    #[test]
    fn a_volume_is_a_bar_of_ten_cells_and_its_number() {
        assert_eq!(volume_text(80, 100), "████████░░  80");
        assert_eq!(volume_text(0, 100), "░░░░░░░░░░   0");
        assert_eq!(volume_text(100, 100), "██████████ 100");
        // A half cell for the odd five; under it rounds down.
        assert_eq!(volume_text(85, 100), "████████▒░  85");
        assert_eq!(volume_text(84, 100), "████████░░  84");
        assert_eq!(volume_text(5, 100), "▒░░░░░░░░░   5");
        assert_eq!(volume_text(99, 100), "█████████▒  99");
        // Past the loudest is the loudest; a small scale has no halves.
        assert_eq!(volume_text(200, 100), "██████████ 100");
        assert_eq!(volume_text(7, 10), "███████░░░   7");
        assert_eq!(volume_text(3, 5), "███░░░░░░░   3");
    }

    #[test]
    fn rows_are_painted_from_the_second_row_of_the_panel_with_gaps_before_groups() {
        let c = ctx();
        let mut grouped = row("Layout", words("Right-handed", false), false);
        grouped.starts_group = true;
        let rows = vec![
            row("Text speed", words("Normal", true), true),
            row(
                "Music volume",
                ValueView::Volume {
                    level: 65,
                    max: 100,
                },
                false,
            ),
            grouped,
            row("Key bindings", ValueView::None, false),
        ];
        let v = view(rows);
        assert!(painted_row(&c, &v, PANEL.y).starts_with("┌─ Options ─"));
        assert_eq!(
            painted_row(&c, &v, PANEL.y + 1),
            "│                                                          │"
        );
        // The focused row: arrows round a value the cursor changes.
        let first = painted_row(&c, &v, PANEL.y + 2);
        assert_eq!(
            first,
            format!("│   {:<24}◄ Normal ►{:>22}", "Text speed", "│")
        );
        let second = painted_row(&c, &v, PANEL.y + 3);
        assert!(
            second.contains("Music volume              ██████▒░░░  65"),
            "{second}"
        );
        assert!(!second.contains('◄'));
        // A blank row before the group.
        assert_eq!(
            painted_row(&c, &v, PANEL.y + 4),
            painted_row(&c, &v, PANEL.y + 1)
        );
        assert!(
            painted_row(&c, &v, PANEL.y + 5).contains("Layout                    Right-handed")
        );
        assert!(painted_row(&c, &v, PANEL.y + 6).contains("Key bindings   "));
        assert_eq!(painted_row(&c, &v, 31), "help");
    }

    #[test]
    fn a_focused_value_the_cursor_cant_change_has_no_arrows() {
        let c = ctx();
        let v = view(vec![row("Layout", words("Right-handed", false), true)]);
        let painted = painted_row(&c, &v, PANEL.y + 2);
        assert!(
            painted.contains("Right-handed") && !painted.contains('◄') && !painted.contains('►')
        );
        let v = view(vec![row(
            "Music volume",
            ValueView::Volume {
                level: 80,
                max: 100,
            },
            true,
        )]);
        assert!(painted_row(&c, &v, PANEL.y + 2).contains("◄ ████████░░  80 ►"));
    }

    #[test]
    fn the_number_box_shows_a_caret_only_while_typing() {
        let c = ctx();
        let boxed = |digits: &str, typing| {
            let value = ValueView::NumberBox {
                digits: digits.to_owned(),
                typing,
            };
            painted_row(
                &c,
                &view(vec![row("Sound volume", value, true)]),
                PANEL.y + 2,
            )
        };
        assert!(boxed("37", true).contains(" 37_ "), "{}", boxed("37", true));
        assert!(boxed("", true).contains("  _ "));
        assert!(boxed("80", false).contains(" 80 ") && !boxed("80", false).contains('_'));
        // The box's highlight is as wide whatever is in it.
        let bar = c.palette.engine(EngineColor::Focus);
        let width = |digits: &str| {
            let value = ValueView::NumberBox {
                digits: digits.to_owned(),
                typing: true,
            };
            let black = c.palette.engine(EngineColor::Background);
            let mut buf = GlyphBuffer::with_spec(PC, Cell::new(' ', black, black));
            paint(&c, &view(vec![row("Sound volume", value, true)]), &mut buf);
            (VALUE_X - 1..i32::from(PC.cols))
                .take_while(|&x| buf.get(x, PANEL.y + 2).is_some_and(|cell| cell.bg == bar))
                .count()
        };
        assert_eq!((width(""), width("100")), (BOX_W + 2, BOX_W + 2));
    }

    /// The message sits one blank row under the panel's bottom border.
    #[test]
    fn the_message_is_painted_under_the_panel() {
        let c = ctx();
        let mut v = view(vec![]);
        v.message = Some("Tips will show again".to_owned());
        let bottom = PANEL.y + PANEL.h - 1;
        assert!(painted_row(&c, &v, bottom).starts_with('└'));
        assert_eq!(painted_row(&c, &v, bottom + 1), "");
        assert_eq!(painted_row(&c, &v, bottom + 2), "Tips will show again");
        assert_eq!(painted_row(&c, &v, bottom + 3), "");
    }

    /// Every row a campaign with pictures shows (20 lines with the two
    /// group gaps) fits inside the panel: it grows, and the message
    /// moves down with it.
    #[test]
    fn the_panel_grows_to_fit_its_rows() {
        let c = ctx();
        let mut rows: Vec<RowView> = (0..18)
            .map(|i| row(&format!("Row {i}"), ValueView::None, false))
            .collect();
        rows[12].starts_group = true;
        rows[16].starts_group = true;
        let mut v = view(rows);
        v.message = Some("Defaults restored".to_owned());
        let last = PANEL.y + 2 + 20 - 1;
        assert!(painted_row(&c, &v, last).starts_with("│   Row 17"));
        assert!(painted_row(&c, &v, last + 1).starts_with('└'));
        assert_eq!(painted_row(&c, &v, last + 3), "Defaults restored");
        assert_eq!(painted_row(&c, &v, 31), "help");
    }

    #[test]
    fn a_question_is_a_box_in_the_middle_with_its_answers_under_it() {
        let c = ctx();
        let mut v = view(vec![row("Reset tips", ValueView::None, true)]);
        v.question = Some(QuestionView {
            text: "Show every tip again?".to_owned(),
            answers: "f yes / d no".to_owned(),
        });
        let top = (i32::from(PC.rows) - QUESTION_H) / 2;
        assert!(painted_row(&c, &v, top).contains("╔═══════════════════════╗"));
        assert!(painted_row(&c, &v, top + 2).contains("║ Show every tip again? ║"));
        assert!(painted_row(&c, &v, top + 3).contains("║ f yes / d no          ║"));
        assert!(painted_row(&c, &v, top + 5).contains("╚═══════════════════════╝"));
    }

    #[test]
    fn the_question_on_leaving_is_a_box_with_its_answers_in_a_line_the_focused_one_lit() {
        use crate::screens::options::{ChoiceView, LeaveChoice};
        let c = ctx();
        let mut v = view(vec![row("Text speed", words("Fast", true), true)]);
        let choice = |choice, label: &str| ChoiceView {
            choice,
            label: label.to_owned(),
            focused: choice == LeaveChoice::Back,
        };
        v.leaving = Some(LeaveView {
            text: "Keep these changes?".to_owned(),
            choices: vec![
                choice(LeaveChoice::Keep, "Keep"),
                choice(LeaveChoice::Discard, "Discard"),
                choice(LeaveChoice::Back, "Back"),
            ],
        });
        let top = (i32::from(PC.rows) - QUESTION_H) / 2;
        // As wide as the answers' line, the wider.
        let question = format!("║ {:<27} ║", "Keep these changes?");
        assert!(painted_row(&c, &v, top + 2).contains(&question));
        assert!(painted_row(&c, &v, top + 3).contains("║  Keep     Discard     Back  ║"));
        // Only the focused answer is lit.
        let black = c.palette.engine(EngineColor::Background);
        let mut buf = GlyphBuffer::with_spec(PC, Cell::new(' ', black, black));
        paint(&c, &v, &mut buf);
        let bar = c.palette.engine(EngineColor::Focus);
        let lit: String = (0..i32::from(PC.cols))
            .filter_map(|x| buf.get(x, top + 3))
            .filter(|cell| cell.bg == bar)
            .map(|cell| cell.glyph)
            .collect();
        assert_eq!(lit, " Back ");
    }
}
