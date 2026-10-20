//! The glyph look of the Options screen: paints an [`OptionsView`] as a
//! panel of text rows, in the style of the Key bindings screen. Everything
//! about the look lives here (where things go, the box characters, the
//! slider's blocks, the colours); nothing here decides what the screen
//! shows or does. Another look is another `paint` of the same view.

use super::view::{OptionsView, RowView, ValueView};
use crate::ctx::{Ctx, GameData};
use crate::frame::color::EngineColor;
use crate::frame::draw::PanelLook;
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::prelude::*;
use crate::screens::print_centred;
use crate::widgets::ask;

/// The panel, in cells, at its widest and smallest, on a 100-cell
/// console: on another it is centred on the console's width, and as wide
/// as the console if that is narrower; it grows down to fit its rows
/// ([`panel`]).
const PANEL: Rect = Rect::new(20, 3, 60, 20);
/// Column of the row labels, from the panel's left edge.
const LABEL_DX: i32 = 4;
/// Column of the row values, from the panel's left edge.
const VALUE_DX: i32 = 30;
/// Rows under the panel at the least: a blank one, the message's and the
/// help line's.
const BELOW_PANEL: i32 = 3;
/// Panel rows that aren't the screen's rows: the top border, a blank row
/// under it and the bottom border.
const PANEL_FRAME_H: i32 = 3;
/// Cells in a volume's bar.
const VOLUME_BAR: u8 = 10;
/// Cells of the number box, inside its highlight.
const BOX_W: usize = 4;

/// A volume as a bar of ten cells (a half-filled cell for half
/// a cell's worth or more) and its number.
pub fn volume_text(level: u8, max: u8) -> String {
    let level = level.min(max);
    let per_cell = (max / VOLUME_BAR).max(1);
    let full = usize::from(level / per_cell).min(usize::from(VOLUME_BAR));
    // (A cell worth 1 has no half: its remainder is always 0.)
    let half = usize::from(level % per_cell >= per_cell.div_ceil(2));
    let empty = usize::from(VOLUME_BAR).saturating_sub(full + half);
    let bar = ["█".repeat(full), "▒".repeat(half), "░".repeat(empty)].concat();
    format!("{bar} {level:>3}")
}

/// The panel for `view` on `buf`: [`PANEL`] centred on the buffer's
/// width and no wider than it; no taller than leaves room for the message
/// under it, unless its rows and the blank rows between their groups need
/// more.
fn panel(view: &OptionsView, buf: &GlyphBuffer) -> Rect {
    let width = i32::from(buf.width());
    let w = PANEL.w.min(width);
    let room = i32::from(buf.height()) - PANEL.y - BELOW_PANEL - 1;
    let gaps = view.rows.iter().filter(|r| r.starts_group).count();
    let lines = i32::try_from(view.rows.len() + gaps).unwrap_or(i32::MAX);
    let h = PANEL.h.min(room).max(lines.saturating_add(PANEL_FRAME_H));
    Rect::new((width - w) / 2, PANEL.y, w, h)
}

/// Paints `view` over the whole of `buf`.
pub fn paint<G: GameData>(ctx: &Ctx<G>, view: &OptionsView, buf: &mut GlyphBuffer) {
    let c = |e| ctx.palette.engine(e);
    let (black, bg) = (c(EngineColor::Background), c(EngineColor::Panel));
    let (text, dim) = (c(EngineColor::Text), c(EngineColor::TextDim));
    buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
    let panel = panel(view, buf);
    let look = PanelLook::new(BoxStyle::Single, text, bg)
        .border(c(EngineColor::Border))
        .title(c(EngineColor::Highlight));
    buf.panel(panel, &look, Some(&view.title));

    let mut y = panel.y + 2;
    for row in &view.rows {
        if row.starts_group {
            y += 1;
        }
        paint_row(ctx, buf, row, panel, y);
        y += 1;
    }

    if let Some(message) = &view.message {
        // The row under the panel's.
        let row = panel.y + panel.h + 1;
        print_centred(buf, row, message, c(EngineColor::Highlight), black);
    }
    if let Some(question) = &view.question {
        ask::glyph::paint(&ctx.palette, question, buf);
    }
    if let Some(leaving) = &view.leaving {
        ask::glyph::paint(&ctx.palette, leaving, buf);
    }
    let bottom = i32::from(buf.height()) - 1;
    print_centred(buf, bottom, &view.help, dim, black);
}

/// Paints `row` on console row `y` of `panel`: the focused row's label is a
/// highlight bar; a value the cursor changes sits between two arrows; the
/// number box is a highlight bar where the value was.
fn paint_row<G: GameData>(ctx: &Ctx<G>, buf: &mut GlyphBuffer, row: &RowView, panel: Rect, y: i32) {
    let (label_x, value_x) = (panel.x + LABEL_DX, panel.x + VALUE_DX);
    let c = |e| ctx.palette.engine(e);
    let (bg, text, dim) = (
        c(EngineColor::Panel),
        c(EngineColor::Text),
        c(EngineColor::TextDim),
    );
    let bar = c(EngineColor::Focus);
    if row.focused {
        buf.print(label_x - 1, y, &format!(" {} ", row.label), bg, bar);
    } else {
        buf.print(label_x, y, &row.label, text, bg);
    }
    let (value, adjustable) = match &row.value {
        ValueView::None => return,
        ValueView::NumberBox { digits, typing } => {
            let caret = if *typing { "_" } else { "" };
            let shown = format!(" {:<BOX_W$} ", format!("{digits}{caret}"));
            buf.print(value_x - 1, y, &shown, bg, bar);
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
    buf.print(value_x, y, &value, value_fg, bg);
    if row.focused && adjustable {
        buf.print(value_x - 2, y, "◄", dim, bg);
        let after = value_x + i32::try_from(value.chars().count()).unwrap_or(0) + 1;
        buf.print(after, y, "►", dim, bg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::console::ConsoleSpec;
    use crate::toy::{ToyCtx, ctx};
    use crate::widgets::AskView;

    /// The console the skin is laid out for.
    const PC: ConsoleSpec = ConsoleSpec::text(100, 32);
    /// A small console, which the skin fits.
    const SMALL: ConsoleSpec = ConsoleSpec::text(48, 24);

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
            (PANEL.x + VALUE_DX - 1..i32::from(PC.cols))
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
        v.question = Some(AskView {
            text: "Show every tip again?".to_owned(),
            answers: Vec::new(),
            keys: Some("f yes / d no".to_owned()),
        });
        let top = (i32::from(PC.rows) - ask::glyph::HEIGHT) / 2;
        assert!(painted_row(&c, &v, top).contains("╔═══════════════════════╗"));
        assert!(painted_row(&c, &v, top + 2).contains("║ Show every tip again? ║"));
        assert!(painted_row(&c, &v, top + 3).contains("║ f yes / d no          ║"));
        assert!(painted_row(&c, &v, top + 5).contains("╚═══════════════════════╝"));
    }

    #[test]
    fn the_question_on_leaving_is_a_box_with_its_answers_in_a_line_the_focused_one_lit() {
        use crate::screens::options::LeaveChoice;
        use crate::widgets::AnswerView;
        let c = ctx();
        let mut v = view(vec![row("Text speed", words("Fast", true), true)]);
        let choice = |answer, label: &str| AnswerView {
            answer,
            label: label.to_owned(),
            focused: answer == LeaveChoice::Back,
        };
        v.leaving = Some(AskView {
            text: "Keep these changes?".to_owned(),
            keys: None,
            answers: vec![
                choice(LeaveChoice::Keep, "Keep"),
                choice(LeaveChoice::Discard, "Discard"),
                choice(LeaveChoice::Back, "Back"),
            ],
        });
        let top = (i32::from(PC.rows) - ask::glyph::HEIGHT) / 2;
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

    /// The rows of a game's options, the music volume focused, and a
    /// message.
    fn small_view() -> OptionsView {
        let volume = |level| ValueView::Volume { level, max: 100 };
        let mut look = row("Map look", words("Glyphs", true), false);
        look.starts_group = true;
        let mut keys = row("Key bindings", ValueView::None, false);
        keys.starts_group = true;
        let mut v = view(vec![
            row("Text speed", words("Normal", true), false),
            row("Fullscreen", words("Off", true), false),
            row("Music volume", volume(100), true),
            row("Sound volume", volume(65), false),
            look,
            keys,
            row("Restore defaults", ValueView::None, false),
        ]);
        v.message = Some("Couldn't save: the change lasts until you quit".to_owned());
        v
    }

    fn painted_small(c: &ToyCtx, view: &OptionsView) -> GlyphBuffer {
        let black = c.palette.engine(EngineColor::Background);
        let mut buf = GlyphBuffer::with_spec(SMALL, Cell::new(' ', black, black));
        paint(c, view, &mut buf);
        buf
    }

    /// On a 48-cell console the panel is the console's width and every
    /// row's label and value is inside it; the message still shows, under
    /// it.
    #[test]
    fn on_a_small_console_every_row_is_inside_the_panel() {
        let c = ctx();
        let v = small_view();
        let buf = painted_small(&c, &v);
        let row = |y| -> String {
            (0..i32::from(SMALL.cols))
                .map(|x| buf.get(x, y).map_or(' ', |c| c.glyph))
                .collect()
        };
        assert_eq!(panel(&v, &buf), Rect::new(0, 3, 48, 17));
        assert_eq!(
            row(7),
            format!("│   {:<24}◄ ██████████ 100 ►{:>2}", "Music volume", "│")
        );
        for y in 4..19 {
            let line = row(y);
            assert!(line.starts_with('│') && line.ends_with('│'), "{line}");
        }
        assert!(row(19).starts_with('└'));
        assert_eq!(
            row(21).trim(),
            "Couldn't save: the change lasts until you quit"
        );
        assert_eq!(row(23).trim(), "help");
        // On the game's console, the panel is where it always was.
        let mut buf = GlyphBuffer::with_spec(
            PC,
            Cell::new(
                ' ',
                c.palette.engine(EngineColor::Background),
                c.palette.engine(EngineColor::Background),
            ),
        );
        paint(&c, &v, &mut buf);
        assert_eq!(panel(&v, &buf), PANEL);
    }

    #[test]
    fn the_small_console_snapshot() {
        let c = ctx();
        let buf = painted_small(&c, &small_view());
        insta::assert_snapshot!(buf.to_snapshot(&c.palette));
    }
}
