//! Hit areas: where a painter put what the pointer can land on
//! (ADR-0134). A painter records each area in the frame as it paints
//! it ([`GlyphBuffer::hit`](super::GlyphBuffer::hit)), so the geometry is
//! the look's own and the views stay plain data. The pointer looks a
//! place up with [`GlyphBuffer::hit_at`](super::GlyphBuffer::hit_at): the
//! last area recorded that holds it, since what was painted last is on
//! top. Hits never show in a frame's snapshot.

use crate::frame::glyph_buffer::PxRect;

/// Which widget of a screen a [`Hit`] belongs to, so a screen with two
/// menus tells their hits apart. 0 unless the screen gives its widgets
/// keys of their own ([`Menu::keyed`](crate::widgets::Menu::keyed)).
///
/// Kind 1 of ADR-0039: a plain value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WidgetKey(pub u16);

/// One area the pointer can land on: its pixels, its widget and the item
/// of the widget there (a menu's row), or [`Hit::AREA`] for the widget's
/// whole box.
///
/// Kind 3 of ADR-0039: build it with [`Hit::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Hit {
    /// Its console pixels.
    pub rect: PxRect,
    /// The widget it belongs to.
    pub widget: WidgetKey,
    /// The item of the widget there, or [`Hit::AREA`].
    pub item: u32,
}

impl Hit {
    /// The item of a hit that stands for its widget's whole box, not one
    /// item in it: a click there chooses nothing, but it is over the
    /// widget.
    pub const AREA: u32 = u32::MAX;

    /// The area `rect` of `widget`'s `item`.
    pub const fn new(rect: PxRect, widget: WidgetKey, item: u32) -> Self {
        Self { rect, widget, item }
    }

    /// The item there, unless it is the widget's whole box.
    pub fn item(&self) -> Option<u32> {
        (self.item != Self::AREA).then_some(self.item)
    }

    /// Whether `other` is the same widget's same item: where a press and
    /// its release land for a click.
    pub fn same_target(&self, other: &Hit) -> bool {
        self.widget == other.widget && self.item == other.item
    }
}

/// The hits of `hits` that hold `(x, y)`, the last recorded first.
pub(crate) fn hit_at(hits: &[Hit], x: i32, y: i32) -> Option<Hit> {
    hits.iter().rev().find(|h| h.rect.contains(x, y)).copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Rect};
    use crate::frame::{ConsoleSpec, Rgb};

    fn blank() -> Cell {
        Cell::new(' ', Rgb::new(255, 255, 255), Rgb::new(0, 0, 0))
    }

    #[test]
    fn the_last_hit_recorded_that_holds_a_point_wins() {
        let mut buf = GlyphBuffer::new(10, 4, blank());
        let menu = WidgetKey(0);
        buf.hit(Rect::new(0, 0, 40, 32), menu, Hit::AREA);
        buf.hit(Rect::new(8, 16, 16, 16), menu, 2);
        assert_eq!(buf.hits().len(), 2);
        // Inside the row: the row, recorded after the box.
        assert_eq!(buf.hit_at(10, 20).and_then(|h| h.item()), Some(2));
        // Inside the box only.
        let area = buf.hit_at(2, 2).unwrap();
        assert_eq!((area.item(), area.widget), (None, menu));
        // Outside every hit.
        assert_eq!(buf.hit_at(41, 2), None);
        assert_eq!(buf.hit_at(-1, 0), None);
        assert!(area.same_target(&Hit::new(Rect::new(0, 0, 1, 1), menu, Hit::AREA)));
        assert!(!area.same_target(&Hit::new(Rect::new(0, 0, 1, 1), WidgetKey(1), Hit::AREA)));
    }

    #[test]
    fn cells_are_recorded_in_the_buffers_own_cell_size() {
        let spec = ConsoleSpec {
            cell_h: 8,
            ..ConsoleSpec::text(10, 4)
        };
        let mut buf = GlyphBuffer::with_spec(spec, blank());
        buf.hit_cells(Rect::new(1, 2, 3, 1), WidgetKey(4), 7);
        assert_eq!(
            buf.hits(),
            &[Hit::new(Rect::new(8, 16, 24, 8), WidgetKey(4), 7)]
        );
    }

    #[test]
    fn clearing_the_whole_frame_clears_the_hits_and_a_part_does_not() {
        let mut buf = GlyphBuffer::new(10, 4, blank());
        buf.hit(Rect::new(0, 0, 8, 16), WidgetKey(0), 0);
        buf.fill_rect(Rect::new(0, 0, 2, 2), blank());
        assert_eq!(buf.hits().len(), 1);
        // A see-through fill keeps them, as it keeps the backdrop.
        buf.fill_rect(buf.bounds(), Cell::see_through(' ', Rgb::new(1, 2, 3)));
        assert_eq!(buf.hits().len(), 1);
        buf.fill_rect(buf.bounds(), blank());
        assert_eq!(buf.hits(), &[]);
        buf.hit(Rect::new(0, 0, 8, 16), WidgetKey(0), 0);
        buf.clear_hits();
        assert_eq!(buf.hit_at(1, 1), None);
    }
}
