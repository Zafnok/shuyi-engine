//! A buffer's [`Backdrop`] (game ADR-0048): a second glyph picture, the *scene*,
//! seen through a window of console cells at a pixel offset and a whole
//! zoom. `app` draws it behind the cells marked
//! [`Cell::see_through`](super::Cell::see_through); every other cell draws
//! over it as usual.

#[cfg(not(feature = "std"))]
use crate::float::Float;
use alloc::rc::Rc;

use super::{GlyphBuffer, PxRect, Rect};
use crate::frame::console::ConsoleSpec;

/// The biggest zoom: whole steps 1× to 4× only
/// (`docs/design/title-screen.md`, *Intro cinematic*).
pub const MAX_ZOOM: u8 = 4;

/// A scene shown through a window of a [`GlyphBuffer`]'s cells.
#[derive(Clone, PartialEq, Debug)]
pub struct Backdrop {
    scene: Rc<GlyphBuffer>,
    /// The console of the buffer it is shown in, whose cells `clip` counts.
    console: ConsoleSpec,
    clip: Rect,
    origin_px: (f32, f32),
    zoom: u8,
}

impl Backdrop {
    /// `scene` behind the cells of `clip` of a `console` buffer. A zoom
    /// outside 1 to [`MAX_ZOOM`] is clamped; an origin that isn't a number
    /// counts as 0.
    pub(super) fn new(
        scene: Rc<GlyphBuffer>,
        console: ConsoleSpec,
        clip: Rect,
        origin_px: (f32, f32),
        zoom: u8,
    ) -> Self {
        let finite = |v: f32| if v.is_finite() { v } else { 0.0 };
        Self {
            scene,
            console,
            clip,
            origin_px: (finite(origin_px.0), finite(origin_px.1)),
            zoom: zoom.clamp(1, MAX_ZOOM),
        }
    }

    /// Shows a copy of the scene [faded](GlyphBuffer::fade) to
    /// `brightness` instead. The scene itself, which others may share, is
    /// not touched.
    pub(super) fn fade(&mut self, brightness: f32) {
        let mut scene = (*self.scene).clone();
        scene.fade(brightness);
        self.scene = Rc::new(scene);
    }

    /// The picture shown: its cells, rectangles and sprites.
    pub fn scene(&self) -> &GlyphBuffer {
        &self.scene
    }

    /// The window, in console cells; inside the buffer.
    pub fn clip(&self) -> Rect {
        self.clip
    }

    /// The window in console pixels.
    pub fn clip_px(&self) -> PxRect {
        let (cw, ch) = self.console.cell_px();
        let Rect { x, y, w, h } = self.clip;
        Rect::new(x * cw, y * ch, w * cw, h * ch)
    }

    /// The scene pixel at the window's top-left. Fractions pan smoothly;
    /// it may lie outside the scene (the clear colour shows there).
    pub fn origin_px(&self) -> (f32, f32) {
        self.origin_px
    }

    /// Console pixels per scene pixel: 1 to [`MAX_ZOOM`].
    pub fn zoom(&self) -> u8 {
        self.zoom
    }

    /// Where the scene's top-left pixel is drawn, in window pixels right
    /// of and below the window's top-left, when a console pixel is `scale`
    /// window pixels: the origin times `zoom × scale`, rounded to whole
    /// window pixels so that glyphs stay sharp. Every renderer places the
    /// scene with this, and draws a scene pixel `zoom × scale` pixels big.
    pub fn scene_offset(&self, scale: u32) -> (i64, i64) {
        #[allow(clippy::cast_precision_loss)] // zoom × scale is small
        let per_px = (u32::from(self.zoom) * scale) as f32;
        // Saturating: an origin far outside any scene shows nothing.
        #[allow(clippy::cast_possible_truncation)]
        let whole = |v: f32| (-v * per_px).round() as i64;
        (whole(self.origin_px.0), whole(self.origin_px.1))
    }
}
