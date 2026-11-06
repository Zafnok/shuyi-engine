//! The debug view's shapes as the frame holds them (ADR-0084): a
//! [`GizmoItem`] is [`Item::Gizmo`](super::Item::Gizmo), drawn over
//! everything else, in a colour the debug skin picked from its
//! [`GizmoRole`]. Screens never make these: they hand
//! [`Gizmo`](crate::debug::gizmo::Gizmo)s in, and
//! [`paint_gizmos`](crate::debug::glyph::paint_gizmos) turns them into
//! items. Each item's pixels come from [`shape`](super::shape)'s integer
//! rules, as runs ([`GizmoItem::for_each_run`]), so every renderer draws
//! the same ones.

use core::fmt;

use super::color::{EngineColor, Rgb};
use super::glyph_buffer::{PxRect, Rect};
use super::shape::{circle_outline_runs, clip_run, line_runs};

/// What a gizmo shows, which picks its colour: the debug skin's
/// ([`color`](Self::color)), never the game's. More roles may come with
/// later channels.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum GizmoRole {
    /// Where something is: a cell, a map's edge, a box round a sprite.
    Bounds,
    /// What blocks movement.
    Solid,
    /// What reacts when something enters it.
    Trigger,
    /// A way something goes.
    Path,
    /// A bone of a skeleton.
    Bone,
    /// Anything else worth pointing out: a cursor, an index, a reading.
    Note,
}

impl GizmoRole {
    /// Every role.
    pub const ALL: &[GizmoRole] = &[
        GizmoRole::Bounds,
        GizmoRole::Solid,
        GizmoRole::Trigger,
        GizmoRole::Path,
        GizmoRole::Bone,
        GizmoRole::Note,
    ];

    /// Its name, as in snapshots.
    pub const fn name(self) -> &'static str {
        match self {
            GizmoRole::Bounds => "bounds",
            GizmoRole::Solid => "solid",
            GizmoRole::Trigger => "trigger",
            GizmoRole::Path => "path",
            GizmoRole::Bone => "bone",
            GizmoRole::Note => "note",
        }
    }

    /// The engine colour the debug skin draws it in.
    pub const fn color(self) -> EngineColor {
        match self {
            GizmoRole::Bounds => EngineColor::TextDim,
            GizmoRole::Solid => EngineColor::Warning,
            GizmoRole::Trigger => EngineColor::Accent,
            GizmoRole::Path => EngineColor::Focus,
            GizmoRole::Bone => EngineColor::Bright,
            GizmoRole::Note => EngineColor::Highlight,
        }
    }
}

/// The most characters a [`LabelText`] keeps.
pub const LABEL_MAX: usize = 24;

/// A gizmo label's text as the frame keeps it: printable ASCII, at most
/// [`LABEL_MAX`] characters (longer text is cut, and anything else is
/// `?`). A fixed array, so an [`Item`](super::Item) stays `Copy`.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct LabelText {
    bytes: [u8; LABEL_MAX],
    len: u8,
}

impl LabelText {
    /// `text`, cut and made ASCII.
    pub fn new(text: &str) -> Self {
        let mut bytes = [0; LABEL_MAX];
        let mut len = 0;
        for (slot, c) in bytes.iter_mut().zip(text.chars()) {
            *slot = if c.is_ascii_graphic() || c == ' ' {
                // Printable ASCII is one byte.
                u8::try_from(c).unwrap_or(b'?')
            } else {
                b'?'
            };
            len += 1;
        }
        Self { bytes, len }
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        let len = usize::from(self.len).min(LABEL_MAX);
        // Only ASCII is ever stored.
        core::str::from_utf8(&self.bytes[..len]).unwrap_or_default()
    }
}

impl fmt::Debug for LabelText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

/// A gizmo's shape as the frame keeps it, in console pixels.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[non_exhaustive]
pub enum GizmoShape {
    /// The outline of a rectangle, one pixel wide, inside it.
    Rect {
        /// The rectangle.
        rect: PxRect,
    },
    /// A line one pixel wide, both ends drawn.
    Line {
        /// Its first pixel.
        from: (i32, i32),
        /// Its last pixel.
        to: (i32, i32),
    },
    /// A plus: `size` pixels each way from its centre.
    Cross {
        /// Its centre.
        at: (i32, i32),
        /// How far each arm reaches.
        size: u8,
    },
    /// The outline of a circle, `2 × radius + 1` pixels across.
    Circle {
        /// Its centre.
        center: (i32, i32),
        /// Its radius.
        radius: u16,
    },
    /// Text in the glyph cells from the one holding `at`: its glyphs are
    /// in the cells, and the item draws no pixels.
    Label {
        /// A pixel of the first cell.
        at: (i32, i32),
        /// The text.
        text: LabelText,
    },
}

/// How far a shape's box may reach, each way (up to `REACH - 1` past
/// 0): its width and height then fit in `i32`, and no buffer reaches that
/// far.
const REACH: i64 = 1 << 30;

impl GizmoShape {
    /// A box holding every pixel it can cover (a label's: its one pixel
    /// `at`), clamped to `-2^30..2^30` on each axis.
    pub fn bounds(&self) -> PxRect {
        let wide = |(x, y): (i32, i32)| (i64::from(x), i64::from(y));
        let (left, top, right, bottom) = match *self {
            GizmoShape::Rect { rect } => {
                let (x, y) = wide((rect.x, rect.y));
                let (w, h) = (i64::from(rect.w.max(0)), i64::from(rect.h.max(0)));
                (x, y, x + w, y + h)
            }
            GizmoShape::Line { from, to } => {
                let ((x0, y0), (x1, y1)) = (wide(from), wide(to));
                (x0.min(x1), y0.min(y1), x0.max(x1) + 1, y0.max(y1) + 1)
            }
            GizmoShape::Cross { at, size } => {
                let ((x, y), s) = (wide(at), i64::from(size));
                (x - s, y - s, x + s + 1, y + s + 1)
            }
            GizmoShape::Circle { center, radius } => {
                let ((x, y), r) = (wide(center), i64::from(radius));
                (x - r, y - r, x + r + 1, y + r + 1)
            }
            GizmoShape::Label { at, .. } => {
                let (x, y) = wide(at);
                (x, y, x + 1, y + 1)
            }
        };
        let clamp = |v: i64| v.clamp(-REACH, REACH - 1);
        let (left, top, right, bottom) = (clamp(left), clamp(top), clamp(right), clamp(bottom));
        let fit = |v: i64| i32::try_from(v).unwrap_or(0);
        Rect::new(fit(left), fit(top), fit(right - left), fit(bottom - top))
    }

    /// The same shape moved by `(dx, dy)`, or `None` if a point no longer
    /// fits in `i32`.
    pub(crate) fn moved(self, dx: i64, dy: i64) -> Option<Self> {
        let point = |(x, y): (i32, i32)| {
            Some((
                i32::try_from(i64::from(x) + dx).ok()?,
                i32::try_from(i64::from(y) + dy).ok()?,
            ))
        };
        Some(match self {
            GizmoShape::Rect { rect } => {
                let (x, y) = point((rect.x, rect.y))?;
                GizmoShape::Rect {
                    rect: Rect::new(x, y, rect.w, rect.h),
                }
            }
            GizmoShape::Line { from, to } => GizmoShape::Line {
                from: point(from)?,
                to: point(to)?,
            },
            GizmoShape::Cross { at, size } => GizmoShape::Cross {
                at: point(at)?,
                size,
            },
            GizmoShape::Circle { center, radius } => GizmoShape::Circle {
                center: point(center)?,
                radius,
            },
            GizmoShape::Label { at, text } => GizmoShape::Label {
                at: point(at)?,
                text,
            },
        })
    }
}

/// A gizmo in the frame: its channel's id and role (for the snapshot), its
/// shape, the colour the debug skin gave its role, and the part of the
/// frame it is drawn in.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
pub struct GizmoItem {
    /// The id of its channel (`grid`).
    pub channel: &'static str,
    /// What it shows.
    pub role: GizmoRole,
    /// Its shape.
    pub shape: GizmoShape,
    /// Its colour.
    pub color: Rgb,
    /// The pixels it may draw in: its shape's [`bounds`](GizmoShape::bounds)
    /// to start with (a label's: the cells its text was printed in),
    /// clipped to the buffer when added.
    pub clip: PxRect,
}

impl GizmoItem {
    /// `shape` in `color`, of `channel` and `role`, drawn wherever it
    /// reaches.
    pub fn new(channel: &'static str, role: GizmoRole, shape: GizmoShape, color: Rgb) -> Self {
        Self {
            channel,
            role,
            shape,
            color,
            clip: shape.bounds(),
        }
    }

    /// Calls `f(y, x0, x1)` for each run of its pixels inside its
    /// `clip`: row `y`, pixels `x0..x1`. Every renderer fills these, in
    /// whole console pixels, in its colour. A label gives none (its glyphs
    /// are in the cells).
    pub fn for_each_run(&self, mut f: impl FnMut(i32, i32, i32)) {
        let clip = self.clip;
        match self.shape {
            GizmoShape::Rect { rect } => rect_runs(rect, clip, &mut f),
            GizmoShape::Line { from, to } => line_runs(from, to, clip, f),
            GizmoShape::Cross { at, size } => {
                let (x, y, s) = (i64::from(at.0), i64::from(at.1), i64::from(size));
                clip_run(clip, y, x - s, x + s + 1, &mut f);
                let rows = (y - s).max(i64::from(clip.y))..(y + s + 1).min(clip_bottom(clip));
                for row in rows.filter(|&row| row != y) {
                    clip_run(clip, row, x, x + 1, &mut f);
                }
            }
            GizmoShape::Circle { center, radius } => circle_outline_runs(center, radius, clip, f),
            GizmoShape::Label { .. } => {}
        }
    }
}

/// The row past `clip`'s last.
fn clip_bottom(clip: PxRect) -> i64 {
    i64::from(clip.y) + i64::from(clip.h.max(0))
}

/// The runs of `rect`'s outline, one pixel wide, inside `clip`: its top and
/// bottom rows whole, and its two sides between them, taking only the rows
/// inside `clip`.
fn rect_runs(rect: PxRect, clip: PxRect, f: &mut impl FnMut(i32, i32, i32)) {
    if rect.is_empty() {
        return;
    }
    let (x, y) = (i64::from(rect.x), i64::from(rect.y));
    let (right, bottom) = (x + i64::from(rect.w), y + i64::from(rect.h));
    clip_run(clip, y, x, right, f);
    if bottom - 1 > y {
        clip_run(clip, bottom - 1, x, right, f);
    }
    for row in (y + 1).max(i64::from(clip.y))..(bottom - 1).min(clip_bottom(clip)) {
        clip_run(clip, row, x, x + 1, f);
        if right - 1 > x {
            clip_run(clip, row, right - 1, right, f);
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeSet;

    use super::*;
    use crate::prelude::*;

    const INK: Rgb = Rgb::new(1, 2, 3);

    fn pixels(item: &GizmoItem) -> BTreeSet<(i32, i32)> {
        let mut out = BTreeSet::new();
        item.for_each_run(|y, x0, x1| out.extend((x0..x1).map(|x| (x, y))));
        out
    }

    fn item(shape: GizmoShape) -> GizmoItem {
        GizmoItem::new("test", GizmoRole::Note, shape, INK)
    }

    #[test]
    fn a_rect_is_its_outline_one_pixel_wide() {
        let rect = item(GizmoShape::Rect {
            rect: Rect::new(2, 3, 4, 3),
        });
        let want: BTreeSet<_> = [
            (2, 3),
            (3, 3),
            (4, 3),
            (5, 3),
            (2, 4),
            (5, 4),
            (2, 5),
            (3, 5),
            (4, 5),
            (5, 5),
        ]
        .into();
        assert_eq!(pixels(&rect), want);
        assert_eq!(rect.clip, Rect::new(2, 3, 4, 3));
        // One pixel high or wide, and empty.
        let flat = item(GizmoShape::Rect {
            rect: Rect::new(0, 0, 3, 1),
        });
        assert_eq!(pixels(&flat).len(), 3);
        let thin = item(GizmoShape::Rect {
            rect: Rect::new(0, 0, 1, 3),
        });
        assert_eq!(pixels(&thin).len(), 3);
        let none = item(GizmoShape::Rect {
            rect: Rect::new(0, 0, 0, 3),
        });
        assert!(pixels(&none).is_empty());
    }

    #[test]
    fn a_cross_reaches_its_size_each_way_and_a_label_draws_nothing() {
        let cross = item(GizmoShape::Cross {
            at: (5, 5),
            size: 2,
        });
        assert_eq!(cross.clip, Rect::new(3, 3, 5, 5));
        let got = pixels(&cross);
        assert_eq!(got.len(), 9);
        assert!(got.contains(&(3, 5)) && got.contains(&(5, 7)) && !got.contains(&(4, 4)));
        let label = item(GizmoShape::Label {
            at: (8, 16),
            text: LabelText::new("0"),
        });
        assert!(pixels(&label).is_empty());
        assert_eq!(label.clip, Rect::new(8, 16, 1, 1));
    }

    #[test]
    fn a_clip_cuts_every_shape() {
        let clip = Rect::new(0, 0, 4, 4);
        for shape in [
            GizmoShape::Rect {
                rect: Rect::new(-3, -3, 6, 6),
            },
            GizmoShape::Line {
                from: (-10, 2),
                to: (10, 2),
            },
            GizmoShape::Cross {
                at: (2, 2),
                size: 9,
            },
            GizmoShape::Circle {
                center: (2, 2),
                radius: 2,
            },
        ] {
            let cut = GizmoItem {
                clip,
                ..item(shape)
            };
            let got = pixels(&cut);
            assert!(!got.is_empty(), "{shape:?}");
            assert!(got.iter().all(|&(x, y)| clip.contains(x, y)), "{shape:?}");
        }
    }

    #[test]
    fn bounds_hold_every_pixel_and_stay_in_reach() {
        let circle = GizmoShape::Circle {
            center: (10, 10),
            radius: 3,
        };
        assert_eq!(circle.bounds(), Rect::new(7, 7, 7, 7));
        let line = GizmoShape::Line {
            from: (5, 1),
            to: (-2, 4),
        };
        assert_eq!(line.bounds(), Rect::new(-2, 1, 8, 4));
        let far = GizmoShape::Line {
            from: (i32::MIN, i32::MIN),
            to: (i32::MAX, i32::MAX),
        };
        let b = far.bounds();
        assert_eq!((b.x, b.y), (-(1 << 30), -(1 << 30)));
        assert_eq!((b.w, b.h), (i32::MAX, i32::MAX));
    }

    #[test]
    fn a_shape_moves_unless_it_leaves_i32() {
        let cross = GizmoShape::Cross {
            at: (1, 2),
            size: 1,
        };
        assert_eq!(
            cross.moved(10, -20),
            Some(GizmoShape::Cross {
                at: (11, -18),
                size: 1
            })
        );
        assert_eq!(cross.moved(i64::from(i32::MAX), 0), None);
    }

    #[test]
    fn a_label_keeps_printable_ascii_and_is_cut() {
        assert_eq!(LabelText::new("unit 3").as_str(), "unit 3");
        assert_eq!(LabelText::new("é\n").as_str(), "??");
        let long = "x".repeat(40);
        assert_eq!(LabelText::new(&long).as_str().len(), LABEL_MAX);
        assert_eq!(format!("{:?}", LabelText::new("a")), "\"a\"");
    }

    #[test]
    fn each_gizmo_has_its_snapshot_line() {
        use crate::frame::snapshot::gizmo_line;
        let line = |role, shape| gizmo_line(&GizmoItem::new("grid", role, shape, INK));
        let rect = GizmoShape::Rect {
            rect: Rect::new(0, 16, 8, 8),
        };
        assert_eq!(
            line(GizmoRole::Bounds, rect),
            "gizmo grid bounds rect 0,16 8x8"
        );
        let to = GizmoShape::Line {
            from: (1, 2),
            to: (-3, 4),
        };
        assert_eq!(line(GizmoRole::Path, to), "gizmo grid path line 1,2 -3,4");
        let cross = GizmoShape::Cross {
            at: (5, 6),
            size: 3,
        };
        assert_eq!(
            line(GizmoRole::Trigger, cross),
            "gizmo grid trigger cross 5,6 3"
        );
        let circle = GizmoShape::Circle {
            center: (7, 8),
            radius: 9,
        };
        assert_eq!(
            line(GizmoRole::Solid, circle),
            "gizmo grid solid circle 7,8 9"
        );
        let label = GizmoShape::Label {
            at: (8, 16),
            text: LabelText::new("unit \"2\""),
        };
        assert_eq!(
            line(GizmoRole::Note, label),
            r#"gizmo grid note label 8,16 "unit \"2\"""#
        );
        let bone = GizmoShape::Cross {
            at: (0, 0),
            size: 0,
        };
        assert_eq!(line(GizmoRole::Bone, bone), "gizmo grid bone cross 0,0 0");
        // Cut by its clip, it says so; a label never does.
        let cut = GizmoItem {
            clip: Rect::new(0, 16, 4, 8),
            ..GizmoItem::new("grid", GizmoRole::Bounds, rect, INK)
        };
        assert_eq!(
            gizmo_line(&cut),
            "gizmo grid bounds rect 0,16 8x8 clip=0,16 4x8"
        );
        let placed = GizmoItem {
            clip: Rect::new(8, 16, 8, 16),
            ..GizmoItem::new("grid", GizmoRole::Note, label, INK)
        };
        assert!(!gizmo_line(&placed).contains("clip"));
    }

    #[test]
    fn each_role_has_a_name_and_an_engine_colour() {
        let names: BTreeSet<_> = GizmoRole::ALL.iter().map(|r| r.name()).collect();
        assert_eq!(names.len(), GizmoRole::ALL.len());
        assert_eq!(GizmoRole::Bounds.color(), EngineColor::TextDim);
        assert_eq!(GizmoRole::Note.color(), EngineColor::Highlight);
    }
}
