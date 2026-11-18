//! How a grid skin shows its ranges' tiles beyond a flat tint (ticket
//! 3224, game ticket 0923): spreading out from where they were opened,
//! a wave of light across them, one outline around them all, and relief
//! (each tile raised, or the whole range a raised slab). Drawn from the
//! scene and its clock alone: [`MapScene::ranges_shown`] says where and
//! when the ranges were opened.
//!
//! [`RangeShow::TINT`] is the skins' look before this: every range tints
//! its tiles at [`OVERLAY_BLEND`], still.

use crate::prelude::*;
use core::f32::consts::TAU;

use shuyi_core::Pos;

use crate::frame::color::Rgb;
use crate::frame::glyph_buffer::{Layer, Overlay, PxRect, Rect};
use crate::grid::glyph::OVERLAY_BLEND;
use crate::grid::scene::{MapScene, RangeName, RangesShown, Tints};

/// How long the spread takes to go one tile further, in milliseconds
/// (Manhattan distance from where the ranges were opened). *Tunable.*
pub const SPREAD_STEP_MS: u64 = 45;

/// How long a tile takes to fade in once the spread reaches it, in
/// milliseconds. *Tunable.*
pub const SPREAD_FADE_MS: u64 = 90;

/// How strongly a range tints its tiles when it moves or is outlined:
/// lighter than [`OVERLAY_BLEND`], so the wave shows. *Tunable.*
pub const SHOWN_BLEND: f32 = 0.6;

/// How far the [shimmer](Wave::Shimmer) moves the tint either way.
/// *Tunable.*
pub const SHIMMER_DEPTH: f32 = 0.15;

/// The tint under the [glint](Wave::Glint) band, and how much stronger the
/// band's middle is. *Tunable.*
pub const GLINT_BASE: f32 = 0.55;
/// See [`GLINT_BASE`].
pub const GLINT_DEPTH: f32 = 0.2;

/// How many tiles from its middle the glint band reaches. *Tunable.*
pub const GLINT_HALF_WIDTH: f32 = 2.5;

/// How long one wave takes: the shimmer's period, and the glint's sweep
/// across the view and back to its start. *Tunable.*
pub const WAVE_MS: u64 = 2400;

/// How much of a wave one tile further along the diagonal is behind.
const SHIMMER_PER_TILE: f32 = 0.09;

/// How far toward black a raised tile's shaded edges are, and how far a
/// slab's shadow is from black toward the range's color.
const SHADE: f32 = 0.55;
const SHADOW: f32 = 0.25;

/// A wave of light across a range's tiles.
///
/// Kind 6 of ADR-0039: more waves may come, so a `match` on it has a `_` arm.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Wave {
    /// None: the tint is still.
    #[default]
    None,
    /// The tint brightens and dims in a slow wave running along the
    /// diagonal.
    Shimmer,
    /// A light band sweeps across the view along the diagonal, the tint
    /// still elsewhere.
    Glint,
}

/// How raised a range's tiles look.
///
/// Kind 6 of ADR-0039: more reliefs may come, so a `match` on it has a `_` arm.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Relief {
    /// Flat.
    #[default]
    Flat,
    /// Each tile a raised panel: its top and left edges lit, its bottom
    /// and right edges shaded.
    Tiles,
    /// The whole range a raised slab: a shadow below and to the right of
    /// its outline.
    Slab,
}

/// How a skin shows its ranges' tiles.
///
/// Kind 2 of ADR-0039: start from [`RangeShow::TINT`] and change it with its methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RangeShow {
    /// Ranges spread out from where they were opened
    /// ([`MapScene::ranges_shown`]) instead of showing at once.
    pub spread: bool,
    /// The wave of light across them.
    pub wave: Wave,
    /// One outline around all the ranges together, each stretch in its
    /// tile's range color, lightened; none between two ranges.
    pub outline: bool,
    /// How raised they look.
    pub relief: Relief,
}

impl Default for RangeShow {
    fn default() -> Self {
        Self::TINT
    }
}

impl RangeShow {
    /// Every range tints its tiles at [`OVERLAY_BLEND`], still: the look
    /// before ticket 3224.
    pub const TINT: Self = Self {
        spread: false,
        wave: Wave::None,
        outline: false,
        relief: Relief::Flat,
    };

    /// This look, spreading out from where the ranges were opened.
    #[must_use]
    pub const fn spreading(mut self) -> Self {
        self.spread = true;
        self
    }

    /// This look with `wave`.
    #[must_use]
    pub const fn with_wave(mut self, wave: Wave) -> Self {
        self.wave = wave;
        self
    }

    /// This look with one outline around all the ranges.
    #[must_use]
    pub const fn outlined(mut self) -> Self {
        self.outline = true;
        self
    }

    /// This look with `relief`.
    #[must_use]
    pub const fn with_relief(mut self, relief: Relief) -> Self {
        self.relief = relief;
        self
    }

    /// Whether this is [`TINT`](Self::TINT).
    pub fn is_tint(&self) -> bool {
        *self == Self::TINT
    }

    /// How strongly a range tints its tiles in this look, on the tile `at`
    /// (tiles right of and below the view's first) of a view of `size`
    /// tiles, at `clock_ms`, before the spread: [`OVERLAY_BLEND`] for
    /// [`TINT`](Self::TINT).
    #[expect(
        clippy::cast_precision_loss,
        reason = "a view's tiles and a wave's milliseconds, a few thousand"
    )]
    pub fn blend(&self, at: (i32, i32), size: (i32, i32), clock_ms: u64) -> f32 {
        let diagonal = (at.0 + at.1) as f32;
        let t = (clock_ms % WAVE_MS) as f32 / WAVE_MS as f32;
        match self.wave {
            Wave::Shimmer => {
                SHOWN_BLEND + SHIMMER_DEPTH * (TAU * (diagonal * SHIMMER_PER_TILE - t)).sin()
            }
            Wave::Glint => {
                // The band's middle runs from one band-width before the
                // view's first diagonal to one past its last.
                let span = (size.0 + size.1) as f32 + 2.0 * GLINT_HALF_WIDTH;
                let middle = t * span - GLINT_HALF_WIDTH;
                let near = 1.0 - (diagonal - middle).abs() / GLINT_HALF_WIDTH;
                GLINT_BASE + GLINT_DEPTH * near.clamp(0.0, 1.0)
            }
            _ if self.is_tint() => OVERLAY_BLEND,
            _ => SHOWN_BLEND,
        }
    }

    /// How far the spread has shown the tile at `pos` at `clock_ms`: 0
    /// before it gets there, 1 once it has faded in. Always 1 when this
    /// look doesn't spread or the ranges weren't opened anywhere
    /// (`shown` is `None`).
    #[expect(
        clippy::cast_precision_loss,
        reason = "milliseconds of a fade, at most SPREAD_FADE_MS"
    )]
    pub fn spread_at(&self, pos: Pos, shown: Option<RangesShown>, clock_ms: u64) -> f32 {
        let Some(shown) = shown.filter(|_| self.spread) else {
            return 1.0;
        };
        let distance = pos.x.abs_diff(shown.from.x) + pos.y.abs_diff(shown.from.y);
        let reached = shown.at_ms + u64::from(distance) * SPREAD_STEP_MS;
        let Some(since) = clock_ms.checked_sub(reached) else {
            return 0.0;
        };
        let since = since.min(SPREAD_FADE_MS);
        since as f32 / SPREAD_FADE_MS as f32
    }

    /// How strongly the tile `at` of `scene` (right of and below its
    /// first) is tinted by each of its ranges now: [`blend`](Self::blend)
    /// times [`spread_at`](Self::spread_at).
    pub fn strength<U>(&self, scene: &MapScene<U>, at: (i32, i32)) -> f32 {
        let pos = Pos::new(scene.origin.x + at.0, scene.origin.y + at.1);
        let spread = self.spread_at(pos, scene.ranges_shown, scene.clock_ms);
        spread * self.blend(at, scene.size, scene.clock_ms)
    }

    /// What this look draws over the tile `at` of `scene`, whose pixels are
    /// `rect`, besides its tint: the outline on its sides that face no
    /// range, a raised tile's edges, a slab's shadow on the tiles beside
    /// it. Lines are `line` pixels thick, lit `lit` of the way from the
    /// range's color (`color`) to white; all of it under the glyphs and
    /// units, and none of it outside `clip`. Nothing for a tile in no
    /// range, or that the spread hasn't reached yet.
    #[allow(clippy::too_many_arguments)]
    pub fn overlays<U>(
        &self,
        scene: &MapScene<U>,
        tints: &Tints,
        at: (i32, i32),
        rect: PxRect,
        line: i32,
        lit: f32,
        color: impl Fn(RangeName) -> Rgb,
        clip: PxRect,
    ) -> Vec<Overlay> {
        let mut out = Vec::new();
        let Some(&kind) = tints.at(at.0, at.1).last() else {
            return out;
        };
        let pos = Pos::new(scene.origin.x + at.0, scene.origin.y + at.1);
        if self.spread_at(pos, scene.ranges_shown, scene.clock_ms) <= 0.0 {
            return out;
        }
        let base = color(kind);
        let light = base.lerp(Rgb::new(255, 255, 255), lit);
        let none = |dx: i32, dy: i32| tints.at(at.0 + dx, at.1 + dy).is_empty();
        // Top, right, bottom, left.
        let open = [none(0, -1), none(1, 0), none(0, 1), none(-1, 0)];
        let mut push = |rect: Rect, color: Rgb| {
            if let Some(inside) = rect.intersect(&clip) {
                out.push(Overlay::new(inside, color, Layer::Under));
            }
        };
        let (left, top, wide, high) = (rect.x, rect.y, rect.w, rect.h);
        let sides = [
            Rect::new(left, top, wide, line),
            Rect::new(left + wide - line, top, line, high),
            Rect::new(left, top + high - line, wide, line),
            Rect::new(left, top, line, high),
        ];
        match self.relief {
            Relief::Tiles => {
                let shade = base.lerp(Rgb::new(0, 0, 0), SHADE);
                for (i, side) in sides.into_iter().enumerate() {
                    push(side, if i == 0 || i == 3 { light } else { shade });
                }
            }
            Relief::Slab => {
                let shadow = Rgb::new(0, 0, 0).lerp(base, SHADOW);
                let depth = 2 * line;
                if open[2] {
                    push(Rect::new(left + depth, top + high, wide, depth), shadow);
                }
                if open[1] {
                    push(Rect::new(left + wide, top + depth, depth, high), shadow);
                }
            }
            _ => {}
        }
        if self.outline {
            for (side, open) in sides.into_iter().zip(open) {
                if open {
                    push(side, light);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests;
