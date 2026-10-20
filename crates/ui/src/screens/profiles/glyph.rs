//! The glyph look of the profiles screen: a panel with the profiles, the
//! one in use marked, then the row that adds one (in the style of the
//! options screen); the menu of what can be done to a profile, the name
//! boxes and the questions in double-bordered boxes over it. Everything
//! about the look lives here; nothing here decides what the screen shows.
//! Another look is another `paint` of the same view
//! ([`ProfilesScreen::painted_by`](super::ProfilesScreen::painted_by)).

use super::RowKind;
use super::view::{ActionsView, ProfilesView, RowView};
use crate::ctx::{Ctx, GameData};
use crate::frame::color::EngineColor;
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::prelude::*;
use crate::screens::print_centred;
use crate::widgets::ask;
use crate::widgets::name_entry::glyph::{TYPING_HEIGHT, grid_height, paint_grid, paint_typing};

/// The panel's top and width, in cells; it is centred on the buffer's
/// width (and no wider than it) and as tall as its rows ([`panel`]).
const PANEL_Y: i32 = 5;
/// Width of the panel.
const PANEL_W: i32 = 40;
/// Column of the row labels, from the panel's left edge.
const LABEL_DX: i32 = 4;
/// Column of the mark of the profile in use, from the panel's left edge.
const MARK_DX: i32 = 24;
/// Width of the name boxes (no wider than the buffer).
const NAME_BOX_W: i32 = 46;

/// Cells `text` takes, as a coordinate.
fn cells(text: &str) -> i32 {
    i32::try_from(text.chars().count()).unwrap_or(0)
}

/// The panel for `rows` rows on `buf`: the border, a blank row above and
/// below them, and one between the profiles and the row that adds one.
fn panel(rows: usize, buf: &GlyphBuffer) -> Rect {
    let rows = i32::try_from(rows).unwrap_or(0);
    let width = i32::from(buf.width());
    let w = PANEL_W.min(width);
    Rect::new((width - w) / 2, PANEL_Y, w, rows + 5)
}

/// A rect `w` wide (no wider than `buf`) and `h` tall in the middle of
/// `buf`.
fn middle(buf: &GlyphBuffer, w: i32, h: i32) -> Rect {
    let w = w.min(i32::from(buf.width()));
    Rect::new(
        (i32::from(buf.width()) - w) / 2,
        ((i32::from(buf.height()) - h) / 2).max(0),
        w,
        h,
    )
}

/// Paints `view` over the whole of `buf`.
pub fn paint<G: GameData>(ctx: &Ctx<G>, view: &ProfilesView, buf: &mut GlyphBuffer) {
    let c = |e| ctx.palette.engine(e);
    let (black, bg) = (c(EngineColor::Background), c(EngineColor::Panel));
    let (text, dim) = (c(EngineColor::Text), c(EngineColor::TextDim));
    let panel = panel(view.rows.len(), buf);
    buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
    buf.fill_rect(panel, Cell::new(' ', text, bg));
    buf.draw_box(panel, BoxStyle::Single, c(EngineColor::Border), bg);
    let title = format!(" {} ", view.title);
    buf.print(panel.x + 2, panel.y, &title, c(EngineColor::Highlight), bg);

    let mut y = panel.y + 2;
    for row in &view.rows {
        if row.kind == RowKind::Add {
            // A blank row between the profiles and the row that adds one.
            y += 1;
        }
        paint_row(ctx, buf, row, &view.current_mark, panel.x, y);
        y += 1;
    }

    if let Some(message) = &view.message {
        // One blank row under the panel.
        let y = panel.y + panel.h + 1;
        print_centred(buf, y, message, c(EngineColor::Highlight), black);
    }
    if let Some(actions) = &view.actions {
        paint_actions(ctx, buf, actions);
    }
    if let Some(grid) = &view.grid {
        let rect = middle(buf, NAME_BOX_W, grid_height(grid));
        paint_grid(&ctx.palette, buf, rect, grid);
    }
    if let Some(typing) = &view.typing {
        let rect = middle(buf, NAME_BOX_W, TYPING_HEIGHT);
        paint_typing(&ctx.palette, buf, rect, typing);
    }
    if let Some(question) = &view.leaving {
        ask::glyph::paint(&ctx.palette, question, buf);
    }
    if let Some(question) = &view.deleting {
        ask::glyph::paint(&ctx.palette, question, buf);
    }
    let bottom = i32::from(buf.height()) - 1;
    print_centred(buf, bottom, &view.help, dim, black);
}

/// Paints `row` on console row `y` of the panel at column `x`: the focused
/// row's label is a highlight bar, the profile in use has `mark` beside
/// it, a row that does nothing is dim.
fn paint_row<G: GameData>(
    ctx: &Ctx<G>,
    buf: &mut GlyphBuffer,
    row: &RowView,
    mark: &str,
    x: i32,
    y: i32,
) {
    let c = |e| ctx.palette.engine(e);
    let bg = c(EngineColor::Panel);
    let fg = if row.enabled {
        c(EngineColor::Text)
    } else {
        c(EngineColor::TextDim)
    };
    let label_x = x + LABEL_DX;
    if row.focused {
        let bar = c(EngineColor::Focus);
        buf.print(label_x - 1, y, &format!(" {} ", row.label), bg, bar);
    } else {
        buf.print(label_x, y, &row.label, fg, bg);
    }
    if row.current {
        buf.print(x + MARK_DX, y, mark, c(EngineColor::Highlight), bg);
    }
}

/// The menu of what can be done to a profile: a double-bordered box in
/// the middle with the profile's name in its top border and the items
/// under each other in the player's language, the focused one as a
/// highlight bar.
fn paint_actions<G: GameData>(ctx: &Ctx<G>, buf: &mut GlyphBuffer, actions: &ActionsView) {
    let c = |e| ctx.palette.engine(e);
    let (bg, text) = (c(EngineColor::Panel), c(EngineColor::Text));
    let items = &actions.menu.items;
    let labels: Vec<String> = items.iter().map(|i| ctx.label(&i.label)).collect();
    let widest = labels.iter().map(|l| cells(l)).max();
    let w = widest.unwrap_or(0).max(cells(&actions.title) + 2) + 6;
    let h = i32::try_from(items.len()).unwrap_or(0) + 4;
    let rect = middle(buf, w, h);
    buf.fill_rect(rect, Cell::new(' ', text, bg));
    buf.draw_box(rect, BoxStyle::Double, c(EngineColor::Focus), bg);
    let title = format!(" {} ", actions.title);
    buf.print(rect.x + 2, rect.y, &title, c(EngineColor::Highlight), bg);
    for ((item, label), y) in items.iter().zip(&labels).zip(rect.y + 2..) {
        if item.focused {
            let bar = c(EngineColor::Focus);
            buf.print(rect.x + 2, y, &format!(" {label} "), bg, bar);
        } else {
            buf.print(rect.x + 3, y, label, text, bg);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::console::ConsoleSpec;
    use crate::profiles::ProfileId;
    use crate::screens::profiles::DeleteChoice;
    use crate::toy::{ToyCtx, ctx};
    use crate::widgets::{AnswerView, AskView, Menu, MenuItem};

    const PC: ConsoleSpec = ConsoleSpec::text(100, 32);
    /// The panel's left edge on [`PC`].
    const PANEL_X: i32 = 30;

    /// `view` as painted on [`PC`].
    fn painted(c: &ToyCtx, view: &ProfilesView) -> GlyphBuffer {
        let black = c.palette.engine(EngineColor::Background);
        let mut buf = GlyphBuffer::with_spec(PC, Cell::new(' ', black, black));
        paint(c, view, &mut buf);
        buf
    }

    /// Row `y` of `buf`, trimmed.
    fn row(buf: &GlyphBuffer, y: i32) -> String {
        let glyphs = (0..i32::from(buf.width())).map(|x| buf.get(x, y).map_or(' ', |c| c.glyph));
        glyphs.collect::<String>().trim().to_owned()
    }

    fn profile(label: &str, current: bool, focused: bool) -> RowView {
        RowView {
            kind: RowKind::Profile(ProfileId::FIRST),
            label: label.to_owned(),
            current,
            focused,
            enabled: true,
        }
    }

    fn view(rows: Vec<RowView>) -> ProfilesView {
        ProfilesView {
            title: "Profile".to_owned(),
            rows,
            current_mark: "in use".to_owned(),
            actions: None,
            typing: None,
            grid: None,
            leaving: None,
            deleting: None,
            message: None,
            help: "help".to_owned(),
        }
    }

    #[test]
    fn the_profiles_then_a_blank_row_then_the_add_row_the_current_one_marked() {
        let c = ctx();
        let add = RowView {
            kind: RowKind::Add,
            label: "Add a profile".to_owned(),
            current: false,
            focused: false,
            enabled: false,
        };
        let v = view(vec![
            profile("Player 1", true, false),
            profile("Mei", false, true),
            add,
        ]);
        let buf = painted(&c, &v);
        assert!(row(&buf, PANEL_Y).starts_with("┌─ Profile ─"));
        let first = row(&buf, PANEL_Y + 2);
        assert_eq!(first, format!("│   {:<20}in use{:>10}", "Player 1", "│"));
        assert!(row(&buf, PANEL_Y + 3).starts_with("│   Mei "));
        assert_eq!(row(&buf, PANEL_Y + 4), row(&buf, PANEL_Y + 1));
        assert!(row(&buf, PANEL_Y + 5).starts_with("│   Add a profile"));
        // The add row that does nothing is dim; the focused row is lit.
        let label_x = PANEL_X + LABEL_DX;
        let dim = c.palette.engine(EngineColor::TextDim);
        assert_eq!(buf.get(label_x, PANEL_Y + 5).map(|c| c.fg), Some(dim));
        let bar = c.palette.engine(EngineColor::Focus);
        assert_eq!(buf.get(label_x, PANEL_Y + 3).map(|c| c.bg), Some(bar));
        assert_eq!(row(&buf, PANEL_Y + 6), row(&buf, PANEL_Y + 1));
        assert!(row(&buf, PANEL_Y + 7).starts_with('└'));
        assert_eq!(row(&buf, 31), "help");
    }

    #[test]
    fn the_message_is_painted_one_blank_row_under_the_panel() {
        let c = ctx();
        let mut v = view(vec![profile("Player 1", true, true)]);
        v.message = Some("There can be at most 8 profiles.".to_owned());
        let buf = painted(&c, &v);
        let bottom = PANEL_Y + 5;
        assert!(row(&buf, bottom).starts_with('└'));
        assert_eq!(row(&buf, bottom + 1), "");
        assert_eq!(row(&buf, bottom + 2), "There can be at most 8 profiles.");
    }

    #[test]
    fn the_delete_question_is_the_question_widgets_box() {
        let c = ctx();
        let mut v = view(vec![profile("Player 1", true, true)]);
        let answer = |answer, label: &str, focused| AnswerView {
            answer,
            label: label.to_owned(),
            focused,
        };
        v.deleting = Some(AskView {
            text: "Delete Mei?".to_owned(),
            answers: vec![
                answer(DeleteChoice::Yes, "Yes", false),
                answer(DeleteChoice::No, "No", true),
            ],
            keys: None,
        });
        let buf = painted(&c, &v);
        let top = (i32::from(PC.rows) - ask::glyph::HEIGHT) / 2;
        assert!(row(&buf, top + 2).contains("║ Delete Mei?  ║"));
        assert!(row(&buf, top + 3).contains("║  Yes     No  ║"));
    }

    #[test]
    fn the_actions_are_a_box_titled_with_the_profile_the_focused_item_lit() {
        let c = ctx();
        let mut v = view(vec![profile("Player 1", true, true)]);
        let menu = Menu::new(vec![
            MenuItem::keyed("profile.action.rename"),
            MenuItem::keyed("profile.action.delete"),
        ]);
        v.actions = Some(ActionsView {
            title: "Mei".to_owned(),
            menu: menu.view(),
        });
        let buf = painted(&c, &v);
        let top = (i32::from(PC.rows) - 6) / 2;
        assert!(row(&buf, top).contains("╔═ Mei ═"), "{}", row(&buf, top));
        assert!(row(&buf, top + 2).contains("║  Rename  ║"));
        assert!(row(&buf, top + 3).contains("║  Delete  ║"));
        let bar = c.palette.engine(EngineColor::Focus);
        let lit: String = (0..i32::from(PC.cols))
            .filter_map(|x| buf.get(x, top + 2))
            .filter(|cell| cell.bg == bar)
            .map(|cell| cell.glyph)
            .collect();
        assert_eq!(lit, " Rename ");
        // A name wider than the items widens the box.
        if let Some(actions) = &mut v.actions {
            actions.title = "Abcdefghijkl".to_owned();
        }
        let buf = painted(&c, &v);
        assert!(
            row(&buf, top).contains("╔═ Abcdefghijkl ═══╗"),
            "{}",
            row(&buf, top)
        );
    }
}
