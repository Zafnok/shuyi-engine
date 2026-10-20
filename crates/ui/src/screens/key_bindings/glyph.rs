//! The glyph look of the Key bindings screen: paints a [`KeyBindingsView`]
//! as a panel of text rows, a column per slot, with the focused slot as a
//! bar. Everything about the look lives here (where things go, the box
//! characters, the colours); nothing here decides what the screen shows or
//! does. Another look is another `paint` of the same view.
//!
//! It is laid out for a 100 × 32 console (`PANEL`) and fits another
//! (`Layout`): the panel keeps its 2-cell margin and reaches down to the
//! row over the message. On a narrower panel the label column is only as
//! wide as the longest label, with its keys that can't change if the slots
//! keep `SLOT_MIN_W` cells for a name (else those keys are cut), and the
//! slot columns share the rest. Every slot is still shown: a key name too
//! long for its column is cut, its last cell a `…`, and the not-mapped
//! note, with no room after the slots, is written over them (they are all
//! empty when it shows).

use super::Side;
use super::view::{ChoicesView, KeyBindingsView, RowView, SlotView, SwitchView};
use crate::ctx::{Ctx, GameData};
use crate::frame::color::EngineColor;
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::prelude::*;
use crate::screens::{cut, print_centred};
use crate::widgets::ask;

/// The panel, in cells, on a 100 × 32 console: at most this wide, with a
/// margin of `PANEL.x` cells either side, and down to the row over the
/// message ([`Layout`]).
const PANEL: Rect = Rect::new(2, 1, 96, 29);
/// Column of the action labels, from the panel's left edge.
const LABEL_DX: i32 = 4;
/// Column of the title and the group headings, from the panel's left edge:
/// two cells left of the labels.
const HEADING_DX: i32 = 2;
/// Cells from the labels' column to the first slot's highlight bar, on a
/// panel as wide as [`PANEL`].
const LABEL_W: i32 = 23;
/// Rows under the panel: the message's and the help line's.
const BELOW_PANEL: i32 = 2;
/// Row of the keyboard / controller switch.
const SWITCH_Y: i32 = PANEL.y + 1;
/// Row of the slot columns' headings, right under the switch; the first
/// group's heading is on the next row.
const COLUMNS_Y: i32 = PANEL.y + 2;
/// Cells a slot's key name may take on a panel as wide as [`PANEL`] (the
/// longest chord name fits).
const SLOT_W: usize = 15;
/// Columns from one slot to the next on a panel as wide as [`PANEL`]: the
/// name, a cell before it and two after it.
const SLOT_PITCH: i32 = 18;
/// Cells a slot's name keeps on a narrower panel before the label column
/// gives up room for the keys that can't change ("Backspace" fits).
const SLOT_MIN_W: usize = 9;
/// What an empty slot shows.
const EMPTY_SLOT: &str = "·";
/// The box of the choices, in cells: a line each and the border.
const CHOICE_W: i32 = 12;
const CHOICE_H: i32 = 4;

/// Where things go across a console of a given size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Layout {
    /// The panel: [`PANEL`] on a 100 × 32 console.
    panel: Rect,
    /// Column of the action labels.
    label_x: i32,
    /// Column of the first slot's highlight bar; its text starts one cell
    /// in.
    slots_x: i32,
    /// Columns from one slot to the next.
    pitch: i32,
    /// Cells a slot's key name may take.
    name_w: usize,
}

impl Layout {
    /// The layout of `view` on a console of `width` × `height` cells: the
    /// panel as wide as the console allows up to [`PANEL`]'s width,
    /// centred; on a narrower panel, the label column as wide as the
    /// longest label and the slots sharing the rest.
    fn new<A: Copy + PartialEq>(view: &KeyBindingsView<A>, width: u16, height: u16) -> Self {
        let (width, height) = (i32::from(width), i32::from(height));
        let w = PANEL.w.min(width - 2 * PANEL.x).max(0);
        let h = (height - PANEL.y - BELOW_PANEL).max(0);
        let panel = Rect::new((width - w) / 2, PANEL.y, w, h);
        let label_x = panel.x + LABEL_DX;
        if w >= PANEL.w {
            return Self {
                panel,
                label_x,
                slots_x: label_x + LABEL_W,
                pitch: SLOT_PITCH,
                name_w: SLOT_W,
            };
        }
        let narrow = |with_fixed| {
            let slots_x = label_x + LABEL_W.min(widest_label(view, with_fixed) + 2);
            let slots = to_i32(view.columns.len().max(1));
            let pitch = SLOT_PITCH.min((right(panel) - slots_x) / slots).max(0);
            Self {
                panel,
                label_x,
                slots_x,
                pitch,
                name_w: to_usize(pitch - SLOT_PITCH + to_i32(SLOT_W)),
            }
        };
        let with_fixed = narrow(true);
        if with_fixed.name_w >= SLOT_MIN_W {
            with_fixed
        } else {
            narrow(false)
        }
    }

    /// Left edge of slot `i`'s highlight bar.
    fn slot_x(&self, i: usize) -> i32 {
        self.slots_x + to_i32(i) * self.pitch
    }

    /// Column of the title and the group headings.
    fn heading_x(&self) -> i32 {
        self.panel.x + HEADING_DX
    }

    /// Row of the message, right under the panel and over the help line.
    fn message_row(&self) -> i32 {
        self.panel.y + self.panel.h
    }

    /// Column of the panel's right border.
    fn right(&self) -> i32 {
        right(self.panel)
    }
}

/// Column of `panel`'s right border.
fn right(panel: Rect) -> i32 {
    panel.x + panel.w - 1
}

/// `n` as an `i32`, the largest one if it is larger.
fn to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// `n` as a `usize`, 0 if it is below 0.
fn to_usize(n: i32) -> usize {
    usize::try_from(n).unwrap_or(0)
}

/// The text of a row's keys that can't change, after its label.
fn fixed_text<A>(row: &RowView<A>) -> String {
    format!("+ {}", row.fixed.join("/"))
}

/// Cells the widest action label takes; `with_fixed`, with two cells and
/// its keys that can't change, if it has any.
fn widest_label<A: Copy + PartialEq>(view: &KeyBindingsView<A>, with_fixed: bool) -> i32 {
    let width = |row: &RowView<A>| {
        let label = row.label.chars().count();
        if row.fixed.is_empty() || !with_fixed {
            label
        } else {
            label + 2 + fixed_text(row).chars().count()
        }
    };
    to_i32(view.rows().map(width).max().unwrap_or(0))
}

/// The console rows of what is listed under the columns' headings: each
/// group's heading, each action, and Restore defaults.
#[derive(Debug, PartialEq, Eq)]
struct Rows {
    /// Row of each group's heading.
    headings: Vec<i32>,
    /// Row of each action, in the order of [`KeyBindingsView::rows`].
    actions: Vec<i32>,
    /// Row of Restore defaults.
    restore: i32,
}

/// Places the groups one under the other: a heading, then its actions; a
/// blank row between groups and before Restore defaults.
fn place<A>(view: &KeyBindingsView<A>) -> Rows {
    let mut y = COLUMNS_Y + 1;
    let (mut headings, mut actions) = (Vec::new(), Vec::new());
    for (i, group) in view.groups.iter().enumerate() {
        if i > 0 {
            y += 1;
        }
        headings.push(y);
        y += 1;
        for _ in &group.rows {
            actions.push(y);
            y += 1;
        }
    }
    Rows {
        headings,
        actions,
        restore: y + 1,
    }
}

/// Paints `view` over the whole of `buf`.
pub fn paint<G: GameData>(ctx: &Ctx<G>, view: &KeyBindingsView<G::Action>, buf: &mut GlyphBuffer) {
    let c = |e| ctx.palette.engine(e);
    let (black, bg) = (c(EngineColor::Background), c(EngineColor::Panel));
    let (text, dim) = (c(EngineColor::Text), c(EngineColor::TextDim));
    let bar = c(EngineColor::Focus);
    let at = Layout::new(view, buf.width(), buf.height());
    buf.fill_rect(buf.bounds(), Cell::new(' ', text, black));
    buf.fill_rect(at.panel, Cell::new(' ', text, bg));
    buf.draw_box(at.panel, BoxStyle::Single, c(EngineColor::Border), bg);
    let title = format!(" {} ", view.title);
    let highlight = c(EngineColor::Highlight);
    buf.print(at.heading_x(), at.panel.y, &title, highlight, bg);
    if let Some(switch) = &view.switch {
        paint_switch(ctx, buf, switch, at.label_x);
    }

    for (j, heading) in view.columns.iter().enumerate() {
        // Over the gap after the slot too, a cell short of the next one.
        let heading = cut(heading, to_usize(at.pitch - 2));
        buf.print(at.slot_x(j) + 1, COLUMNS_Y, &heading, dim, bg);
    }
    let placed = place(view);
    let mut rows = view.rows().zip(&placed.actions);
    for (group, &heading_y) in view.groups.iter().zip(&placed.headings) {
        buf.print(at.heading_x(), heading_y, &group.heading, bar, bg);
        for _ in &group.rows {
            if let Some((row, &y)) = rows.next() {
                paint_row(ctx, buf, view, row, &at, y);
            }
        }
    }
    if view.restore.focused {
        let label = format!(" {} ", view.restore.label);
        buf.print(at.label_x - 1, placed.restore, &label, bg, bar);
    } else {
        buf.print(at.label_x, placed.restore, &view.restore.label, text, bg);
    }

    if let Some(message) = &view.message {
        let row = at.message_row();
        print_centred(buf, row, message, c(EngineColor::Warning), black);
    }
    if let Some(question) = &view.question {
        ask::glyph::paint(&ctx.palette, question, buf);
    }
    if let Some(choices) = &view.choices {
        let focused_y = view
            .rows()
            .zip(&placed.actions)
            .find_map(|(row, &y)| row.focused().then_some(y));
        if let Some(y) = focused_y {
            paint_choices(ctx, buf, choices, &at, y);
        }
    }
    let bottom = i32::from(buf.height()) - 1;
    print_centred(buf, bottom, &view.help, dim, black);
}

/// Paints the keyboard / controller switch from column `x`: the shown side
/// highlighted (as a bar while the focus is on the switch), the other dim.
fn paint_switch<G: GameData>(ctx: &Ctx<G>, buf: &mut GlyphBuffer, switch: &SwitchView, x: i32) {
    let c = |e| ctx.palette.engine(e);
    let (bg, bar) = (c(EngineColor::Panel), c(EngineColor::Focus));
    let mut x = x;
    for (side, name) in [
        (Side::Keyboard, switch.keyboard.as_str()),
        (Side::Controller, switch.controller.as_str()),
    ] {
        if side != switch.shown {
            buf.print(x, SWITCH_Y, name, c(EngineColor::TextDim), bg);
        } else if switch.focused {
            buf.print(x - 1, SWITCH_Y, &format!(" {name} "), bg, bar);
        } else {
            buf.print(x, SWITCH_Y, name, c(EngineColor::Highlight), bg);
        }
        x += i32::try_from(name.chars().count()).unwrap_or(0) + 4;
    }
}

/// Paints `row` on console row `y`: its label (a highlight colour if the
/// cursor is on it, white if it just lost its key), the keys it has that
/// can't change, its slots (the one the cursor is on a bar) and, if
/// nothing is in any, the note.
fn paint_row<G: GameData>(
    ctx: &Ctx<G>,
    buf: &mut GlyphBuffer,
    view: &KeyBindingsView<G::Action>,
    row: &RowView<G::Action>,
    at: &Layout,
    y: i32,
) {
    let c = |e| ctx.palette.engine(e);
    let (bg, text, dim) = (
        c(EngineColor::Panel),
        c(EngineColor::Text),
        c(EngineColor::TextDim),
    );
    let label_fg = if row.focused() {
        c(EngineColor::Highlight)
    } else if row.lost_key {
        c(EngineColor::Bright)
    } else {
        text
    };
    // The label and the fixed keys end a cell before the first slot.
    let room = to_usize(at.slots_x - at.label_x - 1);
    let label_w = buf.print(at.label_x, y, &cut(&row.label, room), label_fg, bg);
    if !row.fixed.is_empty() {
        let used = usize::from(label_w) + 2;
        let fixed = cut(&fixed_text(row), room.saturating_sub(used));
        buf.print(at.label_x + i32::from(label_w) + 2, y, &fixed, dim, bg);
    }
    let note_x = row.unmapped.then(|| note_x(view, row, at));
    for (j, slot) in row.slots.iter().enumerate() {
        let x = at.slot_x(j);
        if row.focus == Some(j) {
            let shown = match slot {
                SlotView::Capturing(prompt) => prompt.as_str(),
                SlotView::Bound(name) => name.as_str(),
                SlotView::Empty => "",
            };
            let w = at.name_w;
            let bar = format!(" {:<w$}", cut(shown, w));
            buf.print(x, y, &bar, bg, c(EngineColor::Focus));
        } else if let SlotView::Bound(name) = slot {
            buf.print(x + 1, y, &cut(name, at.name_w), text, bg);
        } else if note_x.is_none_or(|n| n == after_slots(row, at)) {
            buf.print(x + 1, y, EMPTY_SLOT, dim, bg);
        }
    }
    if let Some(x) = note_x {
        let fg = if row.blocks_leaving {
            c(EngineColor::Warning)
        } else {
            dim
        };
        let note = cut(&view.not_mapped, to_usize(at.right() - x));
        buf.print(x, y, &note, fg, bg);
    }
}

/// The column of `row`'s not-mapped note: a cell after its slots if it fits
/// there before the panel's right border; else over its slots (all empty),
/// a cell into the first, or after the bar of the slot the cursor is on if
/// it would cover the note.
fn note_x<A>(view: &KeyBindingsView<A>, row: &RowView<A>, at: &Layout) -> i32 {
    let len = to_i32(view.not_mapped.chars().count());
    let after = after_slots(row, at);
    if after + len <= at.right() {
        return after;
    }
    let first = at.slots_x + 1;
    match row.focus {
        Some(j) if at.slot_x(j) < first + len => at.slot_x(j) + at.pitch - 1,
        _ => first,
    }
}

/// The column a cell after `row`'s last slot.
fn after_slots<A>(row: &RowView<A>, at: &Layout) -> i32 {
    at.slot_x(row.slots.len()) + 1
}

/// Paints `choices` in a double-bordered box under the slot on console row
/// `slot_y` (over it on the last rows, where it wouldn't fit; left of it at
/// the panel's right edge), the focused line as a bar.
fn paint_choices<G: GameData>(
    ctx: &Ctx<G>,
    buf: &mut GlyphBuffer,
    choices: &ChoicesView,
    at: &Layout,
    slot_y: i32,
) {
    let c = |e| ctx.palette.engine(e);
    let (bg, bar) = (c(EngineColor::Panel), c(EngineColor::Focus));
    let below = slot_y + 1;
    let y = if below + CHOICE_H < at.panel.y + at.panel.h {
        below
    } else {
        slot_y - CHOICE_H
    };
    let x = at.slot_x(choices.slot).min(at.right() - CHOICE_W);
    let rect = Rect::new(x, y, CHOICE_W, CHOICE_H);
    buf.fill_rect(rect, Cell::new(' ', c(EngineColor::Text), bg));
    buf.draw_box(rect, BoxStyle::Double, bar, bg);
    let inner = usize::try_from(CHOICE_W - 3).unwrap_or(0);
    for (i, choice) in (0..).zip(&choices.lines) {
        let (x, y) = (rect.x + 1, rect.y + 1 + i);
        if usize::try_from(i).is_ok_and(|i| i == choices.focused) {
            buf.print(x, y, &format!(" {choice:<inner$}"), bg, bar);
        } else {
            buf.print(x + 1, y, choice, c(EngineColor::Text), bg);
        }
    }
}

#[cfg(test)]
mod tests;
