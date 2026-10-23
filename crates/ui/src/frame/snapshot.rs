//! Text dump of a [`GlyphBuffer`] for snapshot tests (game ADR-0007):
//!
//! ```text
//! <glyph rows, exactly width chars each, trailing spaces kept>
//! --- colours ---
//! <rows of single-char keys, one per cell>
//! --- legend ---
//! a = fg:text bg:panel
//! b = fg:highlight bg:#1c3a79
//! --- overlays ---
//! over  18,46 11x2  hp_mid
//! over  sprite 8,8 80x80  images/test_card.png 0,0 16x16
//! ```
//!
//! Each distinct (fg, bg) pair gets a key in first-seen order (row-major):
//! `a-z`, `A-Z`, `0-9`, then further Unicode letters should a screen ever
//! use more than 62 pairs. Colours print as their palette name when one
//! matches exactly, else as `#rrggbb`. Lines end in `\n` on every OS.
//!
//! The overlays section lists the buffer's items in drawing order, in
//! console pixels; it is left out when there are none. A rectangle
//! (game ADR-0018) is `layer  x,y wxh  colour`. A sprite (game ADR-0038) is
//! `layer  sprite x,y wxh  image x,y wxh`: where it is drawn (`dest`), the
//! image's path in the asset bundle, and the part of the image shown
//! (`src`). After that come, only when they aren't the default, ` flip`,
//! ` opacity=N` (below 255), ` bright=N` (below 255: darkened, game ADR-0075),
//! ` solid=colour`, ` dimmed` or
//! ` shade=grey:red,green,blue` (how its pixels are coloured, game ADR-0049; a
//! shade is a light's numbers in thousandths, game ADR-0058) and
//! ` clip=x,y wxh` (not all of `dest` is drawn).
//!
//! A see-through cell (game ADR-0048) gets a key of its own, whose legend line
//! says `bg:see-through`: in the colour rows a hole can't be taken for a
//! blank cell. When the buffer has a backdrop, a line
//! `--- backdrop: clip x,y wxh  origin x,y  zoom N ---` follows (the window
//! in console cells, the scene pixel at its top-left to a tenth of a pixel,
//! the zoom) and then the scene's own snapshot, in this same format.

use crate::prelude::*;
use alloc::collections::BTreeMap;
use core::fmt::Write as _;

use crate::frame::color::{Palette, Rgb};
use crate::frame::glyph_buffer::Cell;
use crate::frame::glyph_buffer::{Backdrop, GlyphBuffer, Item, Paint, PxRect, Sprite};

/// What a see-through cell's background is called in the legend.
const SEE_THROUGH: &str = "see-through";

/// Shown instead of control characters, which would break the row layout.
const CONTROL_GLYPH: char = '\u{fffd}';

impl GlyphBuffer {
    /// Renders the buffer in the snapshot format described in [`crate::frame::snapshot`].
    pub fn to_snapshot(&self, palette: &Palette) -> String {
        let (w, h) = (i32::from(self.width()), i32::from(self.height()));
        let cells = || (0..h).flat_map(move |y| (0..w).filter_map(move |x| self.get(x, y)));
        // A see-through cell's `bg` isn't shown, so it isn't part of its key.
        let pair = |c: &Cell| (c.fg, (!c.see_through).then_some(c.bg));
        let mut keys: BTreeMap<(Rgb, Option<Rgb>), char> = BTreeMap::new();
        let mut legend: Vec<(char, Rgb, Option<Rgb>)> = Vec::new();
        for c in cells() {
            let (fg, bg) = pair(c);
            keys.entry((fg, bg)).or_insert_with(|| {
                let k = key(legend.len());
                legend.push((k, fg, bg));
                k
            });
        }
        let per_row = usize::from(self.width()).max(1);
        let mut out = String::new();
        for (i, c) in cells().enumerate() {
            out.push(if c.glyph.is_control() {
                CONTROL_GLYPH
            } else {
                c.glyph
            });
            if (i + 1) % per_row == 0 {
                out.push('\n');
            }
        }
        out.push_str("--- colours ---\n");
        for (i, c) in cells().enumerate() {
            out.push(keys[&pair(c)]);
            if (i + 1) % per_row == 0 {
                out.push('\n');
            }
        }
        out.push_str("--- legend ---\n");
        let name = |rgb: Rgb| {
            palette
                .name_of(rgb)
                .map_or_else(|| rgb.to_hex(), str::to_owned)
        };
        for (k, fg, bg) in legend {
            let bg = bg.map_or_else(|| SEE_THROUGH.to_owned(), name);
            let _ = writeln!(out, "{k} = fg:{} bg:{bg}", name(fg));
        }
        if !self.items().is_empty() {
            out.push_str("--- overlays ---\n");
        }
        for item in self.items() {
            let layer = item.layer().name();
            match item {
                Item::Rect(o) => {
                    let _ = writeln!(out, "{layer:<5} {}  {}", px(o.rect), name(o.color));
                }
                Item::Sprite(s) => {
                    let _ = writeln!(out, "{layer:<5} {}", sprite_line(s, &name));
                }
            }
        }
        if let Some(backdrop) = self.backdrop() {
            let _ = writeln!(out, "--- backdrop: {} ---", backdrop_line(backdrop));
            out.push_str(&backdrop.scene().to_snapshot(palette));
        }
        out
    }
}

/// A backdrop's header: its window, origin and zoom.
fn backdrop_line(backdrop: &Backdrop) -> String {
    let (x, y) = backdrop.origin_px();
    format!(
        "clip {}  origin {},{}  zoom {}",
        px(backdrop.clip()),
        tenths(x),
        tenths(y),
        backdrop.zoom()
    )
}

/// `v` to a tenth, never `-0.0`.
fn tenths(v: f32) -> String {
    let rounded = (v * 10.0).round() / 10.0;
    // Adding zero turns a negative zero into a positive one.
    format!("{:.1}", rounded + 0.0)
}

/// `x,y wxh`.
fn px(r: PxRect) -> String {
    format!("{},{} {}x{}", r.x, r.y, r.w, r.h)
}

/// A sprite's snapshot line, after the layer; `name` names a colour.
fn sprite_line(s: &Sprite, name: &dyn Fn(Rgb) -> String) -> String {
    let mut line = format!("sprite {}  {} {}", px(s.dest), s.image.path(), px(s.src));
    if s.flip_x {
        line.push_str(" flip");
    }
    if s.opacity != u8::MAX {
        let _ = write!(line, " opacity={}", s.opacity);
    }
    if s.brightness != u8::MAX {
        let _ = write!(line, " bright={}", s.brightness);
    }
    match s.paint {
        Paint::Image => {}
        Paint::Solid(color) => {
            let _ = write!(line, " solid={}", name(color));
        }
        Paint::Dimmed => line.push_str(" dimmed"),
        Paint::Shaded(light) => {
            let [r, g, b] = light.tint;
            let _ = write!(line, " shade={}:{r},{g},{b}", light.grey);
        }
    }
    if s.clip != s.dest {
        let _ = write!(line, " clip={}", px(s.clip));
    }
    line
}

/// The `n`th legend key.
fn key(n: usize) -> char {
    const KEYS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    if let Some(&k) = KEYS.get(n) {
        return char::from(k);
    }
    // Beyond 62 pairs: CJK ideographs, a contiguous run of 20k+ printable
    // chars. Past that (never reached: a buffer holds at most 65535² cells
    // but a real screen far fewer colours) the key repeats as '?'.
    u32::try_from(n - KEYS.len())
        .ok()
        .and_then(|i| i.checked_add(0x4e00))
        .filter(|&c| c <= 0x9fff)
        .and_then(char::from_u32)
        .unwrap_or('?')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::color::EngineColor;
    use crate::frame::color::tests::engine_palette;
    use crate::frame::glyph_buffer::{Cell, Rect};

    #[test]
    fn keys_sequence() {
        assert_eq!(key(0), 'a');
        assert_eq!(key(25), 'z');
        assert_eq!(key(26), 'A');
        assert_eq!(key(51), 'Z');
        assert_eq!(key(52), '0');
        assert_eq!(key(61), '9');
        assert_eq!(key(62), '\u{4e00}');
        assert_eq!(key(63), '\u{4e01}');
        assert_eq!(key(62 + 0x9fff - 0x4e00), '\u{9fff}');
        assert_eq!(key(62 + 0x9fff - 0x4e00 + 1), '?');
        assert_eq!(key(usize::MAX), '?');
    }

    #[test]
    fn exact_format_small() {
        let p = engine_palette();
        let text = p.get(EngineColor::Text);
        let bg = p.get(EngineColor::Panel);
        let odd = Rgb::new(1, 2, 3);
        let mut b = GlyphBuffer::new(3, 2, Cell::new(' ', text, bg));
        b.print(0, 0, "hi", odd, bg);
        b.set(2, 1, Cell::new('\n', text, odd));
        assert_eq!(
            b.to_snapshot(&p),
            "hi \n  \u{fffd}\n\
             --- colours ---\n\
             aab\nbbc\n\
             --- legend ---\n\
             a = fg:#010203 bg:panel\n\
             b = fg:text bg:panel\n\
             c = fg:text bg:#010203\n"
        );
    }

    #[test]
    fn overlays_are_listed_after_the_legend() {
        use crate::frame::glyph_buffer::{Layer, Overlay};
        let p = engine_palette();
        let text = p.get(EngineColor::Text);
        let mut b = GlyphBuffer::new(3, 1, Cell::new(' ', text, text));
        b.add_overlay(Overlay::new(
            Rect::new(18, 14, 5, 2),
            p.get(EngineColor::Warning),
            Layer::Over,
        ));
        b.add_overlay(Overlay::new(
            Rect::new(0, 7, 24, 3),
            Rgb::new(1, 2, 3),
            Layer::Under,
        ));
        assert_eq!(
            b.to_snapshot(&p),
            "   \n\
             --- colours ---\n\
             aaa\n\
             --- legend ---\n\
             a = fg:text bg:text\n\
             --- overlays ---\n\
             over  18,14 5x2  warning\n\
             under 0,7 24x3  #010203\n"
        );
    }

    #[test]
    fn sprites_are_listed_with_the_rectangles_in_drawing_order() {
        use crate::frame::glyph_buffer::{Layer, Overlay};
        let p = engine_palette();
        let text = p.get(EngineColor::Text);
        let images = shuyi_assets::image::ImageTable::new(
            [(
                "images/test_card.png",
                shuyi_assets::image::ImageInfo::new(16, 16),
            )]
            .into(),
        );
        let card = images.id("images/test_card.png").unwrap();
        let src = Rect::new(0, 0, 16, 16);
        let mut b = GlyphBuffer::new(20, 6, Cell::new(' ', text, text));
        let plain = Sprite::new(card, src, Rect::new(8, 8, 80, 80), Layer::Over);
        b.add_sprite(plain);
        b.add_overlay(Overlay::new(Rect::new(0, 7, 24, 3), text, Layer::Under));
        b.add_sprite(Sprite {
            flip_x: true,
            ..Sprite::new(
                card,
                Rect::new(8, 0, 8, 4),
                Rect::new(0, 0, 16, 8),
                Layer::Under,
            )
        });
        b.add_sprite(Sprite {
            opacity: 128,
            ..plain
        });
        b.add_sprite(Sprite {
            clip: Rect::new(8, 8, 40, 80),
            ..plain
        });
        b.add_sprite(Sprite {
            flip_x: true,
            opacity: 0,
            clip: Rect::new(48, 40, 80, 80),
            ..plain
        });
        b.add_sprite(plain.painted(Paint::Solid(p.get(EngineColor::Warning))));
        b.add_sprite(plain.painted(Paint::Solid(Rgb::new(1, 2, 3))));
        b.add_sprite(Sprite {
            opacity: 128,
            clip: Rect::new(8, 8, 40, 80),
            ..plain.painted(Paint::Dimmed)
        });
        let dusk = shuyi_assets::lighting::Light::new(0.35, [0.62, 0.56, 0.46]);
        b.add_sprite(plain.painted(Paint::Shaded(dusk)));
        b.add_sprite(Sprite {
            brightness: 102,
            ..plain
        });
        assert_eq!(
            b.to_snapshot(&p)
                .split_once("--- overlays ---\n")
                .unwrap()
                .1,
            "over  sprite 8,8 80x80  images/test_card.png 0,0 16x16\n\
             under 0,7 24x3  text\n\
             under sprite 0,0 16x8  images/test_card.png 8,0 8x4 flip\n\
             over  sprite 8,8 80x80  images/test_card.png 0,0 16x16 opacity=128\n\
             over  sprite 8,8 80x80  images/test_card.png 0,0 16x16 clip=8,8 40x80\n\
             over  sprite 8,8 80x80  images/test_card.png 0,0 16x16 flip opacity=0 clip=48,40 40x48\n\
             over  sprite 8,8 80x80  images/test_card.png 0,0 16x16 solid=warning\n\
             over  sprite 8,8 80x80  images/test_card.png 0,0 16x16 solid=#010203\n\
             over  sprite 8,8 80x80  images/test_card.png 0,0 16x16 opacity=128 dimmed clip=8,8 40x80\n\
             over  sprite 8,8 80x80  images/test_card.png 0,0 16x16 shade=350:620,560,460\n\
             over  sprite 8,8 80x80  images/test_card.png 0,0 16x16 bright=102\n"
        );
    }

    #[test]
    fn see_through_cells_get_their_own_key() {
        let p = engine_palette();
        let text = p.get(EngineColor::Text);
        let black = p.get(EngineColor::Panel);
        let mut b = GlyphBuffer::new(4, 1, Cell::new(' ', text, black));
        b.set(1, 0, Cell::see_through(' ', text));
        b.set(2, 0, Cell::see_through('x', text));
        // The hidden background isn't part of the key.
        let mut dimmed = Cell::see_through(' ', text);
        dimmed.bg = text;
        b.set(3, 0, dimmed);
        assert_eq!(
            b.to_snapshot(&p),
            "  x \n\
             --- colours ---\n\
             abbb\n\
             --- legend ---\n\
             a = fg:text bg:panel\n\
             b = fg:text bg:see-through\n"
        );
    }

    #[test]
    fn a_backdrop_is_a_header_line_and_the_scenes_own_snapshot() {
        use alloc::rc::Rc;
        let p = engine_palette();
        let text = p.get(EngineColor::Text);
        let black = p.get(EngineColor::Panel);
        let mut scene = GlyphBuffer::new(2, 1, Cell::new('s', text, black));
        scene.add_overlay(crate::frame::glyph_buffer::Overlay::new(
            Rect::new(0, 0, 3, 2),
            text,
            crate::frame::glyph_buffer::Layer::Over,
        ));
        let scene = Rc::new(scene);
        let mut b = GlyphBuffer::new(3, 2, Cell::new(' ', text, black));
        b.set_backdrop(Rc::clone(&scene), Rect::new(1, 0, 5, 2), (12.34, -0.04), 3);
        let snapshot = b.to_snapshot(&p);
        let plain = GlyphBuffer::new(3, 2, Cell::new(' ', text, black)).to_snapshot(&p);
        assert_eq!(
            snapshot,
            format!(
                "{plain}--- backdrop: clip 1,0 2x2  origin 12.3,0.0  zoom 3 ---\n{}",
                scene.to_snapshot(&p)
            )
        );
        assert!(snapshot.ends_with("--- overlays ---\nover  0,0 3x2  text\n"));
        assert!(!plain.contains("backdrop"));
    }

    #[test]
    fn tenths_round_and_never_print_a_negative_zero() {
        assert_eq!(tenths(0.0), "0.0");
        assert_eq!(tenths(-0.0), "0.0");
        assert_eq!(tenths(-0.04), "0.0");
        assert_eq!(tenths(-0.06), "-0.1");
        assert_eq!(tenths(12.34), "12.3");
        assert_eq!(tenths(12.36), "12.4");
        assert_eq!(tenths(-80.0), "-80.0");
    }

    #[test]
    fn empty_buffers() {
        let p = engine_palette();
        let blank = Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0));
        let expected = "--- colours ---\n--- legend ---\n";
        assert_eq!(GlyphBuffer::new(0, 0, blank).to_snapshot(&p), expected);
        assert_eq!(GlyphBuffer::new(0, 3, blank).to_snapshot(&p), expected);
        assert_eq!(GlyphBuffer::new(3, 0, blank).to_snapshot(&p), expected);
    }

    #[test]
    fn many_pairs_get_distinct_keys() {
        let p = engine_palette();
        let mut b = GlyphBuffer::new(70, 1, Cell::new(' ', Rgb::new(0, 0, 0), Rgb::new(0, 0, 0)));
        for x in 0..70u8 {
            b.set(
                i32::from(x),
                0,
                Cell::new('.', Rgb::new(x, 0, 7), Rgb::new(0, 0, 7)),
            );
        }
        let snap = b.to_snapshot(&p);
        let keys_row = snap.lines().nth(2).unwrap();
        let keys: alloc::collections::BTreeSet<char> = keys_row.chars().collect();
        assert_eq!(keys.len(), 70);
        assert!(keys_row.starts_with("abc"));
        assert_eq!(snap.lines().count(), 1 + 1 + 1 + 1 + 70);
    }
}
