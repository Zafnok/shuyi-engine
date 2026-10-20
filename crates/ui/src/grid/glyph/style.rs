//! How the glyph skin shows what lies on its tiles: the ranges, a flash,
//! the cursor and the threat marks, and the room it leaves at the sides
//! of its area. [`GlyphStyle::PLAIN`] is the glyph skin's own look; another
//! style keeps its tiles, its terrain glyphs and its paint order and
//! changes only these (a one-bit screen's look: ranges as fills, a flash
//! inverted, flat cursor marks).

use crate::prelude::*;

use shuyi_assets::ink::Ink;

use super::cursor::{self, CORNER_ARM, LARGE_CORNER_ARM, corner_arms};
use super::{CELL_PX, Layout, OVERLAY_BLEND, TILE_W_CELLS, draw_terrain, tile_to_cell};
use crate::frame::color::{EngineColor, Rgb};
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Layer, Overlay, PxRect, Rect};
use crate::grid::layout::px_rect;
use crate::grid::scene::{CursorStyle, CursorView, MapScene, RangeName};
use crate::grid::skin::{Looks, MapColors};
use crate::grid::threat;

/// How strong a flash must still be to show as [`FlashLook::Invert`]: one
/// bit has no fading, so the tile is flipped for the first half of a flash.
pub const FLASH_SHOWN: f32 = 0.5;

/// How the glyph skin shows ranges, flashes, the cursor and the threat
/// marks, and how many cells it leaves at the sides of its area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphStyle {
    /// How a range shows on its tiles.
    pub range: RangeLook,
    /// How a tile whose terrain just changed shows.
    pub flash: FlashLook,
    /// How the cursor shows.
    pub cursor: CursorLook,
    /// What colors the threat arrows and marks are.
    pub threat: ThreatLook,
    /// The cells left clear at each side of the area: the tiles that fit
    /// between them are centred across it (room for the cursor's marks
    /// beside the first and last tile).
    pub side_cells: i32,
}

/// How a range shows on its tiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeLook {
    /// Each range in turn, the first laid on undermost, tints the tile's
    /// background toward its color ([`OVERLAY_BLEND`]).
    Tint,
    /// The range laid on last fills the tile: its background the range's
    /// color, its glyphs kept, in the palette's color named `glyphs`.
    /// Ranges in `skip` are never a fill (a game shows them another way,
    /// in its painter's [`paint_under`](crate::grid::UnitPainter::paint_under)).
    Fill {
        /// The palette's name of the glyphs' color on a filled tile.
        glyphs: &'static str,
        /// The ranges that don't fill.
        skip: &'static [RangeName],
    },
}

/// How a tile whose terrain just changed shows while it flashes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashLook {
    /// Its background tinted toward its glyph color by the flash's
    /// strength, each flash in turn, fading out.
    Blend,
    /// While its strongest flash is at least [`FLASH_SHOWN`], covered by a
    /// rectangle in the [invert ink](Ink::Invert), which a one-bit frame
    /// draws by flipping the pixels under it.
    Invert,
}

/// How the cursor shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorLook {
    /// Corner marks fading toward the background as it pulses, or a glow
    /// tinting its tile ([`cursor::draw_cursor`]).
    Pulse,
    /// The same corner marks at full strength with no pixel in two arms
    /// (an ink that flips pixels flips each once), or for the tile glow
    /// the whole tile covered: one bit has no fading. Clipped to the area,
    /// so the marks beside a tile at its side show in the side cells.
    Flat,
}

/// The colors of the threat arrows and marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatLook {
    /// The arrows in the skin's threat color, the marks in the engine's
    /// highlight color, both edged in its background color.
    Highlighted,
    /// The arrows and the marks in the skin's threat color, edged in the
    /// palette's color named `edge`.
    Plain {
        /// The palette's name of the edge's color.
        edge: &'static str,
    },
}

impl GlyphStyle {
    /// The glyph skin's own look: ranges tint, flashes blend, the cursor
    /// pulses, the threat marks are highlighted, no side cells.
    pub const PLAIN: Self = Self {
        range: RangeLook::Tint,
        flash: FlashLook::Blend,
        cursor: CursorLook::Pulse,
        threat: ThreatLook::Highlighted,
        side_cells: 0,
    };

    /// How many tiles, across × down, fit in an area of `area` cells
    /// between its side cells: two cells to a tile across, one down. An
    /// odd cell is left over.
    pub fn view_tiles(&self, area: Rect) -> (i32, i32) {
        let across = (area.w - 2 * self.side_cells).max(0) / TILE_W_CELLS;
        (across, area.h.max(0))
    }

    /// Where `scene`'s tiles go in `area`: as many as fit, centred across
    /// it, from its top.
    pub fn layout<U>(&self, scene: &MapScene<U>, area: Rect) -> Layout {
        let fit = self.view_tiles(area);
        let margin = (area.w.max(0) - TILE_W_CELLS * fit.0) / 2;
        Layout::fitted(scene, (area.x + margin, area.y), fit)
    }

    /// Draws every tile of `layout`: its terrain (tiles off the map are
    /// left as they are, blank); a tile whose terrain just changed flashes;
    /// its ranges show on it; and a tile that would change is drawn as the
    /// terrain it would become instead.
    pub fn draw_tiles<U>(
        &self,
        looks: &Looks,
        buf: &mut GlyphBuffer,
        scene: &MapScene<U>,
        layout: &Layout,
    ) {
        let tints = scene.tints();
        let grid = layout.grid();
        for dy in 0..layout.tiles.1 {
            for dx in 0..layout.tiles.0 {
                let Some(tile) = scene.tile_at(dx, dy) else {
                    continue;
                };
                let (x, y) = (layout.cell.0 + TILE_W_CELLS * dx, layout.cell.1 + dy);
                let cells = Rect::new(x, y, TILE_W_CELLS, 1);
                if let Some(id) = tile.terrain {
                    draw_terrain(looks, buf, id, (x, y));
                }
                match self.flash {
                    FlashLook::Blend => {
                        for &strength in &tile.flashes {
                            if let Some(fg) = buf.get(x, y).map(|c| c.fg) {
                                buf.blend_bg(cells, fg, OVERLAY_BLEND * strength);
                            }
                        }
                    }
                    FlashLook::Invert => {
                        if tile.flashes.iter().any(|&s| s >= FLASH_SHOWN) {
                            let invert = Rgb::from(Ink::Invert.code());
                            let rect = grid.rect_at(dx, dy);
                            buf.add_overlay(Overlay::new(rect, invert, Layer::Over));
                        }
                    }
                }
                let ranges = tints.at(dx, dy);
                match self.range {
                    RangeLook::Tint => {
                        for &kind in ranges {
                            buf.blend_bg(cells, looks.range(kind), OVERLAY_BLEND);
                        }
                    }
                    RangeLook::Fill { glyphs, skip } => {
                        let last = ranges.iter().rev().find(|kind| !skip.contains(kind));
                        if let Some(&kind) = last {
                            let fg = looks.named(glyphs, EngineColor::Text);
                            fill(buf, cells, fg, looks.range(kind));
                        }
                    }
                }
                if let Some(id) = tile.becomes {
                    draw_terrain(looks, buf, id, (x, y));
                }
            }
        }
    }

    /// Draws the scene's cursor in `ink`, if it is on a tile of `layout`
    /// in `area`.
    pub fn draw_cursor<U>(
        &self,
        ink: cursor::CursorInk,
        buf: &mut GlyphBuffer,
        scene: &MapScene<U>,
        layout: &Layout,
        area: Rect,
    ) {
        let Some(c) = &scene.cursor else {
            return;
        };
        let Some(at) = tile_to_cell(c.pos, layout) else {
            return;
        };
        match self.cursor {
            CursorLook::Pulse => cursor::draw_cursor(buf, ink, c, layout, at),
            CursorLook::Flat => draw_flat_cursor(buf, ink, c, at, px_rect(area)),
        }
    }

    /// The colors of the threat arrows and marks in `looks`, for a skin of
    /// `colors`.
    pub fn threat_colors(&self, looks: &Looks, colors: &MapColors) -> threat::Colors {
        match self.threat {
            ThreatLook::Highlighted => looks.threat(colors),
            ThreatLook::Plain { edge } => {
                let arrow = looks.named(colors.threat, EngineColor::Text);
                threat::Colors {
                    arrow,
                    mark: arrow,
                    edge: looks.named(edge, EngineColor::Background),
                }
            }
        }
    }
}

impl Default for GlyphStyle {
    fn default() -> Self {
        Self::PLAIN
    }
}

/// Fills the drawn cells of `cells`: their glyphs kept, in `fg` on `bg`.
fn fill(buf: &mut GlyphBuffer, cells: Rect, fg: Rgb, bg: Rgb) {
    for x in cells.x..cells.x + cells.w {
        if let Some(&cell) = buf.get(x, cells.y) {
            buf.set(x, cells.y, Cell::new(cell.glyph, fg, bg));
        }
    }
}

/// Draws `cursor` flat ([`CursorLook::Flat`]) on the tile whose left cell
/// is `(x, y)`, clipped to `clip`.
fn draw_flat_cursor(
    buf: &mut GlyphBuffer,
    ink: cursor::CursorInk,
    cursor: &CursorView,
    (x, y): (i32, i32),
    clip: PxRect,
) {
    let rects = match cursor.style {
        CursorStyle::Corners => flat_arms(x, y, CORNER_ARM, ink.foot_px),
        CursorStyle::LargeCorners => flat_arms(x, y, LARGE_CORNER_ARM, ink.foot_px),
        CursorStyle::TileGlow => {
            let (cw, ch) = CELL_PX;
            vec![Rect::new(x * cw, y * ch, TILE_W_CELLS * cw, ch)]
        }
    };
    for rect in rects {
        if let Some(rect) = rect.intersect(&clip) {
            buf.add_overlay(Overlay::new(rect, ink.color, Layer::Over));
        }
    }
}

/// The glyph skin's corner marks ([`corner_arms`]) round the tile whose
/// left cell is `(x, y)`, with no pixel in two arms: flipped twice, a
/// corner would show as if not flipped at all. Each corner's upright arm
/// gives its flat one the pixel they share.
fn flat_arms(x: i32, y: i32, arm: i32, foot_px: i32) -> Vec<Rect> {
    let arms = corner_arms(x, y, arm, foot_px);
    let mut out = Vec::with_capacity(arms.len());
    for &[flat, upright] in arms.as_chunks::<2>().0 {
        let from = i32::from(upright.y == flat.y);
        out.push(flat);
        out.push(Rect::new(upright.x, upright.y + from, 1, upright.h - 1));
    }
    out
}

#[cfg(test)]
mod tests;
