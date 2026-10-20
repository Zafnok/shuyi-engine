//! The glyph look of a dialogue's text box: a single-line panel with the
//! speaker's name on its border, the revealed text (narration dim and
//! centred), the replies of a choice with the focused one marked, the
//! question before skipping a scene, and, while a page waits, the key that
//! goes on and a blinking `▼`. While the replies are up the box grows
//! upward to list them under the line being answered.

use super::{TextView, WaitView};
use crate::ctx::{Ctx, GameData};
use crate::frame::color::{EngineColor, Palette};
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::prelude::*;
use crate::widgets::menu::MenuView;

/// Cells between the log's key hint and the key that goes on.
const LOG_HINT_GAP: i32 = 3;

/// Where a text box goes on the console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoxPlace {
    /// The box, border included, when it holds one page.
    pub frame: Rect,
    /// The column the text starts at.
    pub text_x: i32,
    /// The row of the text's first line.
    pub text_y: i32,
}

/// The box `text` is painted in at `place`, and its first text row. While
/// the replies are up it grows upward (its bottom stays put) to fit the
/// line being answered, a blank row and one row per reply.
pub fn box_rect(place: BoxPlace, text: &TextView) -> (Rect, i32) {
    let frame = place.frame;
    let Some(choices) = &text.choices else {
        return (frame, place.text_y);
    };
    let question = text.lines.len();
    let gap = usize::from(question > 0);
    let rows = i32::try_from(question + gap + choices.items.len()).unwrap_or(0);
    // The last content row is the one the ▼ uses, free during a choice.
    let bottom = frame.y + frame.h - 1;
    let text_y = (bottom - rows).min(place.text_y);
    let top = text_y - (place.text_y - frame.y);
    (Rect::new(frame.x, top, frame.w, bottom + 1 - top), text_y)
}

/// Paints `text` in a box at `place`, with `speaker`'s name on its border
/// (none for narration). The replies' labels are in the player's language
/// ([`Ctx::label`]).
pub fn paint<G: GameData>(
    ctx: &Ctx<G>,
    buf: &mut GlyphBuffer,
    place: BoxPlace,
    text: &TextView,
    speaker: Option<&str>,
) {
    let palette = &ctx.palette;
    let c = |e| palette.engine(e);
    let bg = c(EngineColor::Panel);
    let (frame, text_y) = box_rect(place, text);
    paint_frame(palette, buf, speaker, frame);

    if let Some(skip) = &text.skip {
        let (x, y) = (place.text_x, place.text_y);
        buf.print(x, y, &skip.question, c(EngineColor::Text), bg);
        buf.print(x, y + 1, &skip.answers, c(EngineColor::TextDim), bg);
        return;
    }

    // Narration isn't centred vertically above replies, which follow
    // right after.
    let alone = text.choices.is_none();
    paint_lines(palette, buf, place, text, text_y, alone);
    if let Some(choices) = &text.choices {
        let gap = i32::from(!text.lines.is_empty());
        let first = text_y + i32::try_from(text.lines.len()).unwrap_or(0) + gap;
        paint_choices(ctx, buf, place.text_x, choices, first);
    }
    if let Some(waiting) = &text.waiting {
        paint_waiting(palette, buf, place.frame, waiting);
    }
}

/// The box `frame`, empty, with `speaker`'s name on its border.
fn paint_frame(palette: &Palette, buf: &mut GlyphBuffer, speaker: Option<&str>, frame: Rect) {
    let c = |e| palette.engine(e);
    let bg = c(EngineColor::Panel);
    buf.fill_rect(frame, Cell::new(' ', c(EngineColor::Text), bg));
    buf.draw_box(frame, BoxStyle::Single, c(EngineColor::Border), bg);
    if let Some(speaker) = speaker {
        let name = format!(" {speaker} ");
        let fg = c(EngineColor::Highlight);
        buf.print(frame.x + 3, frame.y, &name, fg, bg);
    }
}

/// The revealed part of the page from row `text_y`. Narration is dim and
/// centred; vertically too when `alone` (nothing follows it in the box):
/// one line goes in the middle of the three rows (two or three fill them
/// from the top).
fn paint_lines(
    palette: &Palette,
    buf: &mut GlyphBuffer,
    place: BoxPlace,
    text: &TextView,
    text_y: i32,
    alone: bool,
) {
    let c = |e| palette.engine(e);
    let bg = c(EngineColor::Panel);
    let narration = text.narration;
    let fg = if narration {
        c(EngineColor::TextDim)
    } else {
        c(EngineColor::Text)
    };
    let top = text_y + i32::from(narration && text.lines.len() == 1 && alone);
    for ((y, line), visible) in (top..).zip(&text.lines).zip(text.revealed()) {
        let x = if narration {
            let len = i32::try_from(line.chars().count()).unwrap_or(0);
            place.frame.x + (place.frame.w - len) / 2
        } else {
            place.text_x
        };
        buf.print(x, y, &visible, fg, bg);
    }
}

/// The replies from row `first` at column `x`, the focused one marked and
/// bright.
fn paint_choices<G: GameData>(
    ctx: &Ctx<G>,
    buf: &mut GlyphBuffer,
    x: i32,
    choices: &MenuView,
    first: i32,
) {
    let c = |e| ctx.palette.engine(e);
    let bg = c(EngineColor::Panel);
    for (y, item) in (first..).zip(&choices.items) {
        let (marker, fg) = if item.focused {
            ("> ", c(EngineColor::Highlight))
        } else {
            ("  ", c(EngineColor::Text))
        };
        let label = ctx.label(&item.label);
        buf.print(x, y, &format!("{marker}{label}"), fg, bg);
    }
}

/// At the right of the box's last row: the log's hint, the key that goes
/// on, and the `▼` while it is lit.
fn paint_waiting(palette: &Palette, buf: &mut GlyphBuffer, frame: Rect, waiting: &WaitView) {
    let c = |e| palette.engine(e);
    let bg = c(EngineColor::Panel);
    let right = frame.x + frame.w - 4;
    let bottom = frame.y + frame.h - 2;
    let key = &waiting.key;
    let w = i32::try_from(key.chars().count()).unwrap_or(0);
    buf.print(right - 1 - w, bottom, key, c(EngineColor::TextDim), bg);
    // The log's key, if it has one, before it.
    if let Some(hint) = &waiting.log_hint {
        let hint_w = i32::try_from(hint.chars().count()).unwrap_or(0);
        let x = right - 1 - w - LOG_HINT_GAP - hint_w;
        buf.print(x, bottom, hint, c(EngineColor::TextDim), bg);
    }
    if waiting.marker {
        buf.print(right, bottom, "▼", c(EngineColor::Highlight), bg);
    }
}

#[cfg(test)]
mod tests;
