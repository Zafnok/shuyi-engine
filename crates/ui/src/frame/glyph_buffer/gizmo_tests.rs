//! The debug view's gizmos in the buffer: added over everything, cut with
//! the cells they were drawn over, darkened and moved with a blit.

use super::*;
use crate::frame::gizmo::{GizmoRole, GizmoShape, LabelText};

const INK: Rgb = Rgb::new(200, 100, 50);

fn blank(w: u16, h: u16) -> GlyphBuffer {
    GlyphBuffer::new(w, h, Cell::new(' ', INK, Rgb::new(0, 0, 0)))
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> GizmoItem {
    let shape = GizmoShape::Rect {
        rect: Rect::new(x, y, w, h),
    };
    GizmoItem::new("t", GizmoRole::Bounds, shape, INK)
}

#[test]
fn a_gizmo_is_clipped_to_its_shape_and_the_buffer_and_dropped_when_off_it() {
    let mut b = blank(2, 1);
    b.add_gizmo(rect(-4, 2, 30, 4));
    b.add_gizmo(rect(100, 100, 4, 4));
    let mut wide = rect(1, 1, 2, 2);
    wide.clip = Rect::new(0, 0, 100, 100);
    b.add_gizmo(wide);
    let clips: Vec<Rect> = b.gizmos().iter().map(|g| g.clip).collect();
    assert_eq!(clips, [Rect::new(0, 2, 16, 4), Rect::new(1, 1, 2, 2)]);
    assert!(matches!(b.items()[0], Item::Gizmo(_)));
    assert_eq!(b.items()[0].layer(), Layer::Over);
    // A label keeps the cells it was given, not its one pixel.
    let label = GizmoShape::Label {
        at: (9, 3),
        text: LabelText::new("a"),
    };
    let mut placed = GizmoItem::new("t", GizmoRole::Note, label, INK);
    placed.clip = Rect::new(8, 0, 8, 16);
    b.add_gizmo(placed);
    assert_eq!(b.gizmos()[2].clip, Rect::new(8, 0, 8, 16));
}

#[test]
fn cells_replaced_cut_a_gizmo_and_a_fade_darkens_it() {
    let mut b = blank(2, 1);
    b.add_gizmo(rect(0, 0, 16, 16));
    b.fill_rect(Rect::new(1, 0, 1, 1), Cell::new(' ', INK, INK));
    let cut = b.gizmos();
    assert_eq!(cut.len(), 1);
    assert_eq!(cut[0].clip, Rect::new(0, 0, 8, 16));
    // Its shape stays whole: only its clip shrinks.
    assert_eq!(cut[0].shape, rect(0, 0, 16, 16).shape);
    b.fade(0.5);
    assert_eq!(b.gizmos()[0].color, INK.scale(0.5));
}

#[test]
fn a_blit_moves_a_gizmos_shape_and_clip_with_it() {
    let mut small = blank(1, 1);
    small.add_gizmo(rect(1, 2, 4, 4));
    let mut b = blank(3, 2);
    b.blit(&small, 1, 1);
    let moved = b.gizmos();
    assert_eq!(moved.len(), 1);
    assert_eq!(moved[0].shape, rect(9, 18, 4, 4).shape);
    assert_eq!(moved[0].clip, Rect::new(9, 18, 4, 4));
    // One the blit carries off the buffer is dropped.
    let mut c = blank(1, 1);
    c.blit(&small, 5, 5);
    assert_eq!(c.gizmos().len(), 0);
}
