//! The glyph look of the layout picker: paints a [`LayoutPickerView`] as
//! one panel per layout, each with a small keyboard (the keys' places are
//! this skin's) and a legend. Everything about the look lives here; nothing
//! here decides what the screen shows. Another look is another `paint` of
//! the same view.

use shuyi_assets::keymap::Key;

use super::view::{KeyRole, LayoutPickerView, LayoutView};
use crate::ctx::{Ctx, GameData};
use crate::frame::color::EngineColor;
use crate::frame::draw::PanelLook;
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::prelude::*;
use crate::screens::print_centred;

/// Row of the title.
const TITLE_ROW: i32 = 1;
/// Top row of the first panel.
const FIRST_PANEL_ROW: i32 = 3;
/// Panel size in cells, border included.
const PANEL_W: i32 = 76;
const PANEL_H: i32 = 13;
/// Blank rows between panels.
const PANEL_GAP: i32 = 1;

/// Offsets inside a panel: the keyboard's left edge, its three rows, and
/// the legend's column and first row.
const KEYS_X: i32 = 3;
const TOP_ROW_Y: i32 = 4;
const HOME_ROW_Y: i32 = 5;
const SPACE_ROW_Y: i32 = 7;
const LEGEND_X: i32 = 50;
const LEGEND_Y: i32 = 2;
/// Width of the key column in the legend.
const LEGEND_KEY_W: usize = 12;

/// Width of one key cap, `[w]`.
const CAP_W: i32 = 3;
/// The two letter rows drawn, left to right as on a QWERTY keyboard. What
/// each key does comes from the view ([`LayoutView::role`]), not from here.
// check-keys: keyboard picture
const TOP_ROW: [Key; 10] = [
    Key::Q,
    Key::W,
    Key::E,
    Key::R,
    Key::T,
    Key::Y,
    Key::U,
    Key::I,
    Key::O,
    Key::P,
];
// check-keys: keyboard picture
const HOME_ROW: [Key; 10] = [
    Key::A,
    Key::S,
    Key::D,
    Key::F,
    Key::G,
    Key::H,
    Key::J,
    Key::K,
    Key::L,
    Key::Semicolon,
];
/// The arrow-key cluster's caps: the top one, then the bottom row.
// check-keys: keyboard picture
const UP_CAP: (Key, &str) = (Key::Up, "↑");
// check-keys: keyboard picture
const LOWER_ARROW_CAPS: [(Key, &str); 3] = [(Key::Left, "←"), (Key::Down, "↓"), (Key::Right, "→")];
/// The space bar's key.
// check-keys: keyboard picture
const SPACE_BAR: Key = Key::Space;
/// Left edge of the arrow-key cluster, right of the letter rows.
const ARROWS_X: i32 = KEYS_X + 10 * CAP_W + 4;
/// The space bar: left edge (under `d`) and width.
const SPACE_X: i32 = KEYS_X + 1 + 2 * CAP_W;
const SPACE_W: usize = 19;

/// The colour a key cap's label gets for `role`.
fn role_colour(role: KeyRole) -> EngineColor {
    match role {
        KeyRole::Movement => EngineColor::Accent,
        KeyRole::Bound => EngineColor::Highlight,
        KeyRole::Unbound => EngineColor::TextDim,
    }
}

/// Paints `view` over the whole of `buf`.
pub fn paint<G: GameData>(ctx: &Ctx<G>, view: &LayoutPickerView, buf: &mut GlyphBuffer) {
    let c = |e| ctx.palette.engine(e);
    let black = c(EngineColor::Background);
    buf.fill_rect(buf.bounds(), Cell::new(' ', c(EngineColor::Text), black));
    print_centred(
        buf,
        TITLE_ROW,
        &view.title,
        c(EngineColor::Highlight),
        black,
    );
    let x = (i32::from(buf.width()) - PANEL_W) / 2;
    for (i, layout) in (0..).zip(&view.layouts) {
        let y = FIRST_PANEL_ROW + i * (PANEL_H + PANEL_GAP);
        paint_panel(ctx, buf, layout, x, y);
    }
    let bottom = i32::from(buf.height()) - 1;
    print_centred(buf, bottom, &view.help, c(EngineColor::TextDim), black);
}

/// Paints `layout`'s panel with its top-left corner at `(x, y)`: a double
/// border and an arrow for the focused one.
fn paint_panel<G: GameData>(
    ctx: &Ctx<G>,
    buf: &mut GlyphBuffer,
    layout: &LayoutView,
    x: i32,
    y: i32,
) {
    let c = |e| ctx.palette.engine(e);
    let bg = c(EngineColor::Panel);
    let rect = Rect::new(x, y, PANEL_W, PANEL_H);
    let (style, border, title, title_fg) = if layout.focused {
        (
            BoxStyle::Double,
            c(EngineColor::Focus),
            // check-text: not a data name (the view's own)
            format!("► {}", layout.name),
            c(EngineColor::Highlight),
        )
    } else {
        (
            BoxStyle::Single,
            c(EngineColor::Border),
            layout.name.clone(),
            c(EngineColor::Text),
        )
    };
    let look = PanelLook::new(style, c(EngineColor::Text), bg)
        .border(border)
        .title(title_fg);
    buf.panel(rect, &look, Some(&title));

    let cap_fg = |key| c(role_colour(layout.role(key)));
    let dim = c(EngineColor::TextDim);
    let mut cap = |cx: i32, cy: i32, key: Key, glyph: &str| {
        buf.print(cx, cy, "[", dim, bg);
        buf.print(cx + 1, cy, glyph, cap_fg(key), bg);
        buf.print(cx + 2, cy, "]", dim, bg);
    };
    for (col, key) in (0..).zip(TOP_ROW) {
        cap(x + KEYS_X + col * CAP_W, y + TOP_ROW_Y, key, key.name());
    }
    for (col, key) in (0..).zip(HOME_ROW) {
        cap(
            x + KEYS_X + 1 + col * CAP_W,
            y + HOME_ROW_Y,
            key,
            key.name(),
        );
    }
    let (up, up_glyph) = UP_CAP;
    cap(x + ARROWS_X + CAP_W, y + TOP_ROW_Y, up, up_glyph);
    for (col, (key, glyph)) in (0..).zip(LOWER_ARROW_CAPS) {
        cap(x + ARROWS_X + col * CAP_W, y + HOME_ROW_Y, key, glyph);
    }
    let space = format!("[{:^w$}]", SPACE_BAR.name(), w = SPACE_W - 2);
    buf.print(x + SPACE_X, y + SPACE_ROW_Y, &space, dim, bg);
    let name_x = x + SPACE_X + (i32::try_from(SPACE_W).unwrap_or(0) - 5) / 2;
    buf.print(
        name_x,
        y + SPACE_ROW_Y,
        SPACE_BAR.name(),
        cap_fg(SPACE_BAR),
        bg,
    );

    for (row, line) in (y + LEGEND_Y..).zip(&layout.legend) {
        let fg = if line.movement {
            c(EngineColor::Accent)
        } else {
            c(EngineColor::Highlight)
        };
        buf.print(x + LEGEND_X, row, &line.keys, fg, bg);
        let what_x = x + LEGEND_X + i32::try_from(LEGEND_KEY_W).unwrap_or(0);
        buf.print(what_x, row, &line.what, c(EngineColor::Text), bg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::console::ConsoleSpec;
    use crate::screens::layout_picker::LayoutPickerScreen;
    use crate::screens::layout_picker::view::{KeyCapView, LegendRow};
    use crate::toy::{self, ToyCtx, ctx};

    /// The console the skin is laid out for.
    const PC: ConsoleSpec = ConsoleSpec::text(100, 32);

    fn painted(c: &ToyCtx, view: &LayoutPickerView) -> GlyphBuffer {
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

    /// The column `word` starts at in `row` (in cells, not bytes).
    fn col(row: &str, word: &str) -> usize {
        row[..row.find(word).unwrap()].chars().count()
    }

    /// A one-key legend and two layouts, the second focused.
    fn view(role_of_q: KeyRole, role_of_f: KeyRole) -> LayoutPickerView {
        let layout = |name: &str, focused| LayoutView {
            layout: toy::RIGHT_HANDED,
            name: name.to_owned(),
            focused,
            legend: vec![
                LegendRow {
                    keys: "arrows".to_owned(),
                    what: "move".to_owned(),
                    movement: true,
                },
                LegendRow {
                    keys: "f".to_owned(),
                    what: "select".to_owned(),
                    movement: false,
                },
            ],
            keys: vec![
                KeyCapView {
                    key: Key::Q,
                    role: role_of_q,
                },
                KeyCapView {
                    key: Key::F,
                    role: role_of_f,
                },
            ],
        };
        LayoutPickerView {
            title: "Pick".to_owned(),
            layouts: vec![layout("First", false), layout("Second", true)],
            help: "help".to_owned(),
        }
    }

    const LEFT: i32 = (PC.cols as i32 - PANEL_W) / 2;
    const SECOND: i32 = FIRST_PANEL_ROW + PANEL_H + PANEL_GAP;

    #[test]
    fn two_panels_stack_under_the_centred_title_with_help_at_the_bottom() {
        let c = ctx();
        let buf = painted(&c, &view(KeyRole::Bound, KeyRole::Bound));
        assert_eq!(text(&buf, TITLE_ROW).trim(), "Pick");
        let first = text(&buf, FIRST_PANEL_ROW);
        let second = text(&buf, SECOND);
        assert_eq!(first.find('┌'), Some(usize::try_from(LEFT).unwrap()));
        assert!(first.contains("┌─ First ─"), "{first}");
        // The focused panel: double border and an arrow before its name.
        assert!(second.contains("╔═ ► Second ═"), "{second}");
        assert!(text(&buf, FIRST_PANEL_ROW + PANEL_H - 1).contains('└'));
        assert!(text(&buf, SECOND + PANEL_H - 1).contains('╚'));
        assert_eq!(text(&buf, i32::from(PC.rows) - 1).trim(), "help");
        // The gap between the panels is blank.
        assert!(text(&buf, FIRST_PANEL_ROW + PANEL_H).trim().is_empty());
    }

    #[test]
    fn the_caps_sit_where_the_keyboard_has_them_and_take_their_role_colour() {
        let c = ctx();
        let buf = painted(&c, &view(KeyRole::Movement, KeyRole::Unbound));
        let at = |x, y| *buf.get(x, y).unwrap();
        // `q` is the first cap of the top row, `f` the fourth of the home
        // row (indented half a cap).
        let (qx, qy) = (LEFT + KEYS_X, FIRST_PANEL_ROW + TOP_ROW_Y);
        let q_row = text(&buf, qy);
        assert_eq!(col(&q_row, "[q]"), usize::try_from(qx).unwrap());
        assert_eq!(at(qx + 1, qy).fg, c.palette.engine(EngineColor::Accent));
        let (fx, fy) = (LEFT + KEYS_X + 1 + 3 * CAP_W, FIRST_PANEL_ROW + HOME_ROW_Y);
        assert_eq!(at(fx + 1, fy).glyph, 'f');
        assert_eq!(at(fx + 1, fy).fg, c.palette.engine(EngineColor::TextDim));
        // The brackets are dim whatever the key does.
        assert_eq!(at(qx, qy).fg, c.palette.engine(EngineColor::TextDim));
        // A key the view doesn't list is unbound.
        let (wx, wy) = (LEFT + KEYS_X + CAP_W, FIRST_PANEL_ROW + TOP_ROW_Y);
        assert_eq!(at(wx + 1, wy).glyph, 'w');
        assert_eq!(at(wx + 1, wy).fg, c.palette.engine(EngineColor::TextDim));
        // The arrow cluster's caps and the space bar.
        let arrows = text(&buf, FIRST_PANEL_ROW + HOME_ROW_Y);
        assert!(arrows.contains("[←][↓][→]"), "{arrows}");
        assert!(text(&buf, FIRST_PANEL_ROW + TOP_ROW_Y).contains("[↑]"));
        assert!(text(&buf, FIRST_PANEL_ROW + SPACE_ROW_Y).contains("[      Space      ]"));
    }

    #[test]
    fn the_legend_is_two_columns_and_the_movement_keys_are_set_apart() {
        let c = ctx();
        let buf = painted(&c, &view(KeyRole::Bound, KeyRole::Bound));
        let (x, y) = (LEFT + LEGEND_X, FIRST_PANEL_ROW + LEGEND_Y);
        let first = text(&buf, y);
        assert_eq!(col(&first, "arrows"), usize::try_from(x).unwrap());
        assert_eq!(
            col(&first, "move"),
            usize::try_from(x).unwrap() + LEGEND_KEY_W
        );
        let at = |x, y| buf.get(x, y).unwrap().fg;
        assert_eq!(at(x, y), c.palette.engine(EngineColor::Accent));
        assert_eq!(at(x, y + 1), c.palette.engine(EngineColor::Highlight));
        let what_x = x + i32::try_from(LEGEND_KEY_W).unwrap();
        assert_eq!(at(what_x, y), c.palette.engine(EngineColor::Text));
    }

    #[test]
    fn it_paints_the_real_screen_over_the_whole_buffer() {
        let c = ctx();
        let screen = LayoutPickerScreen::new(&c, &toy::LEGEND);
        let stale = Cell::new(
            'x',
            c.palette.engine(EngineColor::Warning),
            c.palette.engine(EngineColor::Warning),
        );
        let buf = painted(&c, &screen.view(&c));
        let left = (0..i32::from(PC.rows))
            .flat_map(|y| (0..i32::from(PC.cols)).map(move |x| (x, y)))
            .filter(|&(x, y)| buf.get(x, y) == Some(&stale))
            .count();
        assert_eq!(left, 0);
    }

    #[test]
    fn roles_have_their_colours() {
        assert_eq!(role_colour(KeyRole::Movement), EngineColor::Accent);
        assert_eq!(role_colour(KeyRole::Bound), EngineColor::Highlight);
        assert_eq!(role_colour(KeyRole::Unbound), EngineColor::TextDim);
    }
}
