//! Where a frame goes in a window, in whole numbers: by a whole scale, or
//! by a fraction that fills the window (ADR-0139). One function places it
//! for every backend and for `shuyi check --target`, so the check can't
//! pass what the window doesn't draw.

use alloc::format;
use alloc::string::{String, ToString};

/// How a frame is scaled to a window or a display (game ADR-0012, ADR-0139).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ScaleMode {
    /// By the largest whole number that fits: pixels stay square and
    /// sharp, and the rest of the window is letterbox.
    #[default]
    Integer,
    /// By the largest fraction that fits, smaller than 1 too, drawn sharp
    /// (each pixel's edge blends over one window pixel at most). When the
    /// largest whole scale already fills [`SNAP`] of that, the whole scale
    /// instead.
    Fit,
}

impl ScaleMode {
    /// Every mode, the default first.
    pub const ALL: [Self; 2] = [Self::Integer, Self::Fit];
}

/// A scale as a fraction: `num / den`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Factor {
    /// Its numerator.
    pub num: u32,
    /// Its denominator, never 0.
    pub den: u32,
}

impl Factor {
    /// The whole number `n`.
    pub const fn whole(n: u32) -> Self {
        Self { num: n, den: 1 }
    }

    /// `px` times this, rounded down.
    pub fn of(self, px: u16) -> u32 {
        self.times(u32::from(px))
    }

    /// `px` times this, rounded down (at most `u32::MAX`).
    fn times(self, px: u32) -> u32 {
        let product = u64::from(px) * u64::from(self.num) / u64::from(self.den.max(1));
        u32::try_from(product).unwrap_or(u32::MAX)
    }

    /// The largest whole number at most this.
    pub fn floor(self) -> u32 {
        self.num / self.den.max(1)
    }

    /// The smallest whole number at least this.
    pub fn ceil(self) -> u32 {
        self.num.div_ceil(self.den.max(1))
    }

    /// Whether this is a whole number.
    pub fn is_whole(self) -> bool {
        self.num.is_multiple_of(self.den.max(1))
    }

    /// The smaller of two.
    fn min(self, other: Self) -> Self {
        // a/b ≤ c/d when a·d ≤ c·b.
        if u64::from(self.num) * u64::from(other.den) <= u64::from(other.num) * u64::from(self.den)
        {
            self
        } else {
            other
        }
    }

    /// The factor as messages show it: `2`, or `1.60` rounded down.
    pub fn shown(self) -> String {
        if self.is_whole() {
            return self.floor().to_string();
        }
        let hundredths = u64::from(self.num) * 100 / u64::from(self.den.max(1));
        format!("{}.{:02}", hundredths / 100, hundredths % 100)
    }
}

/// [`ScaleMode::Fit`] draws at the whole scale when that scale's picture
/// is at least this share of the filled one, side for side: 9 / 10. On a
/// 1080p or a 4K screen an 800 × 512 frame then stays sharp at 2× or 4×,
/// 95 % of the height, and fills only where the whole scale leaves wide
/// bars (1280 × 720: 71 %).
pub const SNAP: Factor = Factor { num: 9, den: 10 };

/// The most pixels a side of the picture drawn at
/// [`draw_scale`](Placement::draw_scale) may have before it is stretched:
/// 4096, a texture side nearly every GPU in use takes (WebGL 1 and GL ES 2
/// promise far less, but few devices give under 4096).
pub const MAX_DRAW_PX: u32 = 4096;

/// The scale `mode` gives a `frame` (width, height, in logical pixels) in
/// `space` (window or display pixels), a frame side of 0 counted as 1.
/// For [`ScaleMode::Integer`], the largest whole number that fits, 0 when
/// even 1 doesn't; for [`ScaleMode::Fit`], the largest fraction that
/// fits, below 1 too, or the largest whole number when that is at least
/// [`SNAP`] of it.
pub fn scale(mode: ScaleMode, frame: (u32, u32), space: (u32, u32)) -> Factor {
    let across = Factor {
        num: space.0,
        den: frame.0.max(1),
    };
    let down = Factor {
        num: space.1,
        den: frame.1.max(1),
    };
    let fit = across.min(down);
    let whole = fit.floor();
    let snaps = whole >= 1
        && u64::from(whole) * u64::from(fit.den) * u64::from(SNAP.den)
            >= u64::from(fit.num) * u64::from(SNAP.num);
    match mode {
        ScaleMode::Fit if !snaps => fit,
        ScaleMode::Integer | ScaleMode::Fit => Factor::whole(whole),
    }
}

/// Where a frame goes in a window ([`place`]), in window pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Placement {
    /// The scale it is shown at: whole for [`ScaleMode::Integer`] (at
    /// least 1) and for a snapped [`ScaleMode::Fit`].
    pub scale: Factor,
    /// The whole scale the frame is drawn at before it is stretched to
    /// `w × h`: [`scale`](Self::scale) when that is whole, else it
    /// rounded up, at least 1, and lowered while the picture would pass
    /// [`MAX_DRAW_PX`] on a side.
    pub draw_scale: u32,
    /// Its left edge (negative when a too-small window crops it).
    pub x: i32,
    /// Its top edge (negative when a too-small window crops it).
    pub y: i32,
    /// Its width.
    pub w: u32,
    /// Its height.
    pub h: u32,
}

impl Placement {
    /// Whether it is the frame drawn at [`draw_scale`](Self::draw_scale)
    /// with nothing stretched: then a backend draws it straight into the
    /// window.
    pub fn is_whole(self, frame: (u32, u32)) -> bool {
        self.scale.is_whole()
            && u64::from(self.w) == u64::from(frame.0) * u64::from(self.draw_scale)
            && u64::from(self.h) == u64::from(frame.1) * u64::from(self.draw_scale)
    }

    /// The frame pixel of a `frame` this placement shows that holds the
    /// window pixel `window` (`x` right, `y` down, in the window pixels it
    /// was made in), rounded down; `None` in the letterbox, or for a place
    /// that isn't a number. It inverts the placement: frame x is
    /// `(window x - x) × den / num` (ADR-0134 §2).
    pub fn frame_px(self, frame: (u32, u32), window: (f32, f32)) -> Option<(i32, i32)> {
        let (num, den) = (
            f64::from(self.scale.num.max(1)),
            f64::from(self.scale.den.max(1)),
        );
        let axis = |at: f32, edge: i32, side: u32| -> Option<i32> {
            let px = libm::floor((f64::from(at) - f64::from(edge)) * den / num);
            let inside = px >= 0.0 && px < f64::from(side);
            // `px` is a whole number in `0..side` here, and a side fits i32.
            #[allow(clippy::cast_possible_truncation)]
            inside.then_some(px as i32)
        };
        Some((
            axis(window.0, self.x, frame.0)?,
            axis(window.1, self.y, frame.1)?,
        ))
    }
}

/// Places a `frame` (width, height, in logical pixels) in a `window`
/// (width, height, in window pixels), centred, a leftover odd pixel
/// rounded down (a 799-pixel window gives `x = -1`):
/// - [`ScaleMode::Integer`]: the largest whole scale that fits, at least
///   1, so a too-small window crops the edges evenly;
/// - [`ScaleMode::Fit`]: that whole scale when it fills [`SNAP`] of the
///   largest fraction that fits; otherwise the fraction, its limiting
///   side equal to the window's, the other rounded down.
pub fn place(mode: ScaleMode, frame: (u32, u32), window: (u32, u32)) -> Placement {
    let shown = scale(mode, frame, window);
    let (scale, draw_scale) = if shown.is_whole() {
        let whole = shown.floor().max(1);
        (Factor::whole(whole), whole)
    } else {
        (shown, fit_draw_scale(shown, frame))
    };
    let (w, h) = (scale.times(frame.0), scale.times(frame.1));
    let centre = |space: u32, side: u32| -> i32 {
        let at = (i64::from(space) - i64::from(side)).div_euclid(2);
        i32::try_from(at).unwrap_or(if at < 0 { i32::MIN } else { i32::MAX })
    };
    Placement {
        scale,
        draw_scale,
        x: centre(window.0, w),
        y: centre(window.1, h),
        w,
        h,
    }
}

/// The whole scale a frame shown at `fit` is drawn at: `fit` rounded up,
/// at least 1, lowered while the frame times it passes [`MAX_DRAW_PX`].
fn fit_draw_scale(fit: Factor, frame: (u32, u32)) -> u32 {
    let side = u64::from(frame.0.max(frame.1));
    let mut draw = fit.ceil().max(1);
    while draw > 1 && side * u64::from(draw) > u64::from(MAX_DRAW_PX) {
        draw -= 1;
    }
    draw
}

#[cfg(test)]
mod tests;
