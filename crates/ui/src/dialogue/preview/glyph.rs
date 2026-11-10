//! The glyph look of the dialogue preview: paints a [`PreviewView`] as text
//! rows on the whole console, or, while a scene plays, a strip on its top
//! row. Everything about the look lives here (where things go, the marks,
//! the colours); nothing here decides what the preview shows or does.

use super::view::{PreviewView, Tab};
use crate::ctx::{Ctx, GameData};
use crate::frame::color::EngineColor;
use crate::frame::draw::cut;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Rect};
use crate::prelude::*;
use shuyi_assets::lang::text_width;
use shuyi_assets::text::word_wrap;

/// The first row of a tab's list.
const LIST_Y: i32 = 2;
/// The column of a switch's value, from the right edge.
const VALUE_DX: i32 = 12;
/// How far a problem's fix sits in from its message.
const FIX_INDENT: usize = 4;

/// Clears `buf` to the background, under the scene playing and the tabs.
pub fn paint_back<G: GameData>(ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
    let c = |e| ctx.palette.engine(e);
    let cell = Cell::new(' ', c(EngineColor::Text), c(EngineColor::Background));
    buf.fill_rect(buf.bounds(), cell);
}

/// Paints `view` over `buf`: the tabs over the whole of it, or the strip on
/// its top row while the scene plays.
pub fn paint<G: GameData>(ctx: &Ctx<G>, view: &PreviewView, buf: &mut GlyphBuffer) {
    if view.open {
        paint_tabs(ctx, view, buf);
    } else if view.strip.is_some() {
        paint_strip(ctx, view, buf);
    }
}

/// The strip: the file and line, then the direction, and on the right
/// what brings the tabs back.
fn paint_strip<G: GameData>(ctx: &Ctx<G>, view: &PreviewView, buf: &mut GlyphBuffer) {
    let Some(strip) = &view.strip else { return };
    let c = |e| ctx.palette.engine(e);
    let bg = c(EngineColor::Panel);
    let width = i32::from(buf.width());
    buf.fill_rect(Rect::new(0, 0, width, 1), Cell::new(' ', bg, bg));
    let at = format!("{}:{}", strip.file, strip.line);
    let mut x = 1 + i32::from(buf.print(1, 0, &at, c(EngineColor::Highlight), bg));
    let help_w = i32::try_from(text_width(&strip.help, buf.widths())).unwrap_or(width);
    let help_x = width - help_w - 1;
    if let Some(direction) = &strip.direction {
        x += 2;
        let room = usize::try_from(help_x - x - 1).unwrap_or(0);
        let shown = cut(&format!("({direction})"), room, buf.widths());
        buf.print(x, 0, &shown, c(EngineColor::Text), bg);
    }
    if help_x > x {
        buf.print(help_x, 0, &strip.help, c(EngineColor::TextDim), bg);
    }
}

/// The tabs: their names on the top row, the scene picked under them, the
/// tab's rows, and the help on the bottom row.
fn paint_tabs<G: GameData>(ctx: &Ctx<G>, view: &PreviewView, buf: &mut GlyphBuffer) {
    let c = |e| ctx.palette.engine(e);
    let (black, bg) = (c(EngineColor::Background), c(EngineColor::Panel));
    let (text, dim) = (c(EngineColor::Text), c(EngineColor::TextDim));
    let width = i32::from(buf.width());
    let bottom = i32::from(buf.height()) - 1;
    buf.fill_rect(Rect::new(0, 0, width, 1), Cell::new(' ', bg, bg));
    let mut x = 1;
    for (tab, name) in Tab::ALL.iter().zip(&view.tabs) {
        let (fg, under) = if *tab == view.tab {
            (bg, c(EngineColor::Focus))
        } else {
            (text, bg)
        };
        x += i32::from(buf.print(x, 0, &format!(" {name} "), fg, under)) + 1;
    }
    if let Some(playing) = &view.playing {
        let at = format!("{} · {}", playing.file, playing.scene);
        let shown = cut(&at, usize::from(buf.width()) - 2, buf.widths());
        buf.print(1, 1, &shown, dim, black);
    }
    let room = usize::try_from(bottom - LIST_Y).unwrap_or(0);
    let items = items(view, usize::from(buf.width()) - 2, buf);
    if let Some(note) = &view.note {
        buf.print(2, LIST_Y + i32::from(!items.is_empty()), note, dim, black);
    }
    let first = first_shown(&items, view.focus, room);
    let mut y = LIST_Y;
    for (i, item) in items.iter().enumerate().skip(first) {
        for (j, line) in item.lines.iter().enumerate() {
            if y >= bottom {
                break;
            }
            let focused = i == view.focus && j == 0;
            let (fg, under) = if focused {
                (bg, c(EngineColor::Focus))
            } else if item.bright || j > 0 {
                // The scene picked, the text box playing, a problem's fix.
                (c(EngineColor::Highlight), black)
            } else {
                (text, black)
            };
            let shown = cut(line, usize::from(buf.width()) - 2, buf.widths());
            buf.print(1, y, &shown, fg, under);
            if let Some(value) = &item.value {
                let (fg, under) = if focused { (fg, under) } else { (text, black) };
                buf.print(width - VALUE_DX, y, value, fg, under);
            }
            y += 1;
        }
    }
    let help = cut(&view.help, usize::from(buf.width()), buf.widths());
    let help_x = (width - i32::try_from(text_width(&help, buf.widths())).unwrap_or(0)) / 2;
    buf.print(help_x, bottom, &help, dim, black);
}

/// A row of a tab, as painted: its lines (the first the row itself), the
/// value on its right, and whether it stands out.
struct Item {
    lines: Vec<String>,
    value: Option<String>,
    bright: bool,
}

impl Item {
    fn line(line: String) -> Self {
        Self {
            lines: vec![line],
            value: None,
            bright: false,
        }
    }
}

/// The rows of the tab `view` shows, `width` cells wide at most.
fn items(view: &PreviewView, width: usize, buf: &GlyphBuffer) -> Vec<Item> {
    match view.tab {
        Tab::Scenes => view
            .scenes
            .iter()
            .map(|s| {
                let mark = if s.picked { '►' } else { ' ' };
                let warn = if s.problems { '!' } else { ' ' };
                let mut item = Item::line(format!("{mark}{warn} {}: {}", s.file, s.id));
                item.bright = s.picked;
                item
            })
            .collect(),
        Tab::Switches => view
            .switches
            .iter()
            .map(|s| {
                let mut item = Item::line(format!("  {}", s.name));
                item.value = Some(s.value.clone());
                item
            })
            .collect(),
        Tab::Lines => {
            let caret = if view.go_to.typing { "_" } else { "" };
            let go_to = format!("  {}: {}{caret}", view.go_to.label, view.go_to.digits);
            let mut items = vec![Item::line(go_to)];
            for l in &view.lines {
                let mark = if l.current { '►' } else { ' ' };
                let who = l
                    .speaker
                    .as_deref()
                    .map_or(String::from(">"), |s| format!("{s}:"));
                let mut item = Item::line(format!("{mark}{:>4} {who} {}", l.line, l.words));
                item.bright = l.current;
                items.push(item);
            }
            items
        }
        Tab::Problems => view
            .problems
            .iter()
            .map(|p| {
                let at = p.line.map_or(String::new(), |l| format!(":{l}"));
                let head = format!("{}{at}: {}", p.file, p.message);
                let mut lines = word_wrap(&head, width, buf.widths());
                if let Some(fix) = &p.fix {
                    let indent = " ".repeat(FIX_INDENT);
                    let wrapped = word_wrap(fix, width - FIX_INDENT, buf.widths());
                    lines.extend(wrapped.into_iter().map(|l| format!("{indent}{l}")));
                }
                Item {
                    lines,
                    value: None,
                    bright: false,
                }
            })
            .collect(),
    }
}

/// The first item shown, so that the focused one shows in `room` rows.
fn first_shown(items: &[Item], focus: usize, room: usize) -> usize {
    let height = |from: usize| -> usize {
        items
            .iter()
            .take(focus + 1)
            .skip(from)
            .map(|i| i.lines.len())
            .sum()
    };
    let mut first = 0;
    while first < focus && height(first) > room {
        first += 1;
    }
    first
}
