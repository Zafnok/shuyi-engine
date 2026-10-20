//! The glyph look of an [`Ask`](super::Ask): paints an [`AskView`] in a
//! double-bordered box in the middle of the buffer, over whatever is under
//! it. The question is on the first line; under it, the answers in a line
//! with the one in focus as a highlight bar, or the dim line naming the
//! keys that answer. Everything about the look lives here; nothing here
//! decides what the question shows. Another look is another `paint` of the
//! same view.

use super::view::AskView;
use crate::frame::color::{EngineColor, Palette};
use crate::frame::glyph_buffer::{BoxStyle, Cell, GlyphBuffer, Rect};
use crate::prelude::*;

/// Height of the box: its two lines, a blank row above and below, and the
/// border.
pub const HEIGHT: i32 = 6;
/// Cells between two answers.
const ANSWER_GAP: usize = 3;

/// The answers of `view` on one line: each name with a space either side,
/// [`ANSWER_GAP`] cells apart. Returns the line and where each answer
/// starts in it.
fn answers_line<T>(view: &AskView<T>) -> (String, Vec<usize>) {
    let mut line = String::new();
    let mut starts = Vec::new();
    for (i, answer) in view.answers.iter().enumerate() {
        if i > 0 {
            line.push_str(&" ".repeat(ANSWER_GAP));
        }
        starts.push(line.chars().count());
        line.push(' ');
        line.push_str(&answer.label);
        line.push(' ');
    }
    (line, starts)
}

/// The box `view` is painted in on `buf`: as wide as its widest line and
/// a margin of two cells either side, [`HEIGHT`] tall, in the middle.
pub fn rect<T>(view: &AskView<T>, buf: &GlyphBuffer) -> Rect {
    let under = match &view.keys {
        Some(keys) => keys.chars().count(),
        None => answers_line(view).0.chars().count(),
    };
    let widest = view.text.chars().count().max(under);
    let w = i32::try_from(widest).unwrap_or(0) + 4;
    Rect::new(
        (i32::from(buf.width()) - w) / 2,
        (i32::from(buf.height()) - HEIGHT) / 2,
        w,
        HEIGHT,
    )
}

/// Paints `view` in its box ([`rect`]) in the middle of `buf`.
pub fn paint<T>(palette: &Palette, view: &AskView<T>, buf: &mut GlyphBuffer) {
    let c = |e| palette.engine(e);
    let (bg, text) = (c(EngineColor::Panel), c(EngineColor::Text));
    let rect = rect(view, buf);
    buf.fill_rect(rect, Cell::new(' ', text, bg));
    buf.draw_box(rect, BoxStyle::Double, c(EngineColor::Focus), bg);
    let (x, y) = (rect.x + 2, rect.y + 2);
    buf.print(x, y, &view.text, text, bg);
    if let Some(keys) = &view.keys {
        buf.print(x, y + 1, keys, c(EngineColor::TextDim), bg);
        return;
    }
    let bar = c(EngineColor::Focus);
    let (_, starts) = answers_line(view);
    for (answer, start) in view.answers.iter().zip(starts) {
        let x = x + i32::try_from(start).unwrap_or(0);
        let shown = format!(" {} ", answer.label);
        if answer.focused {
            buf.print(x, y + 1, &shown, bg, bar);
        } else {
            buf.print(x, y + 1, &shown, text, bg);
        }
    }
}
