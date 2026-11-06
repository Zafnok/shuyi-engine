//! The grid kit's channel of the debug view (ADR-0084): [`GRID`], the
//! layout of a [`MapScene`] as its skin laid it out ([`Grid`]). A game's
//! map screen offers it by calling [`layout_gizmos`] from its
//! [`debug_draw`](crate::screen::Screen::debug_draw):
//!
//! ```ignore
//! fn debug_draw(&self, _ctx: &Ctx<MyGame>, out: &mut Gizmos) {
//!     let scene = self.scene();
//!     let grid = self.skin.grid(&scene, AREA);
//!     shuyi_ui::grid::debug::layout_gizmos(&scene, &grid, out);
//! }
//! ```

use crate::debug::gizmo::{Channel, Gizmo, GizmoRole, Gizmos};
use crate::frame::glyph_buffer::{PxRect, Rect};
use crate::prelude::*;

use super::layout::Grid;
use super::scene::{MapScene, MapUnit};

/// The grid kit's channel: a map's layout.
pub const GRID: Channel = Channel::new("grid", "Grid layout");

/// Hands in the [`GRID`] channel's gizmos for `scene` laid out as `grid`
/// (its skin's [`GridSkin::grid`](super::GridSkin::grid)), in this order:
///
/// - the cell lines inside the view: each column's and row's first pixel,
///   and the view's last ([`GizmoRole::Bounds`]);
/// - the map's bounds, where they are on screen: round the tiles of the
///   view that are on the map (`Bounds`);
/// - the cursor's cell ([`GizmoRole::Note`]);
/// - each unit's cell, and its index in `scene.units` as a label in the
///   cell holding the tile's top-right pixel (`Note`);
/// - each edge the map goes on past ([`MapScene::more`]): a line along that
///   side of the view (`Note`).
///
/// Everything lies inside the view and the frame ([`Gizmos::frame`]); a
/// cell the frame cuts is outlined where it is cut. Nothing is built while
/// the channel is off.
pub fn layout_gizmos<U: MapUnit>(scene: &MapScene<U>, grid: &Grid, out: &mut Gizmos) {
    if !out.channel_on(GRID) {
        return;
    }
    let Some(view) = grid.bounds().intersect(&out.frame()) else {
        return;
    };
    let (left, top) = (view.x, view.y);
    let (right, bottom) = (view.x + view.w - 1, view.y + view.h - 1);
    let mut push = |role, gizmo| out.push(GRID, role, gizmo);
    let (across, down) = (grid.tiles.0.max(0), grid.tiles.1.max(0));

    let line = |from, to| Gizmo::Line { from, to };
    let columns = (0..across).map(|dx| grid.rect_at(dx, 0).x);
    for x in columns.filter(|&x| x >= left && x < right).chain([right]) {
        push(GizmoRole::Bounds, line((x, top), (x, bottom)));
    }
    let rows = (0..down).map(|dy| grid.rect_at(0, dy).y);
    for y in rows.filter(|&y| y >= top && y < bottom).chain([bottom]) {
        push(GizmoRole::Bounds, line((left, y), (right, y)));
    }

    let on_map = (0..down)
        .flat_map(|dy| (0..across).map(move |dx| (dx, dy)))
        .filter(|&(dx, dy)| scene.tile_at(dx, dy).is_some_and(|t| t.terrain.is_some()))
        .map(|(dx, dy)| grid.rect_at(dx, dy))
        .reduce(union);
    if let Some(rect) = on_map.and_then(|map| map.intersect(&view)) {
        push(GizmoRole::Bounds, Gizmo::Rect { rect });
    }

    let cell = |pos| grid.rect(pos).and_then(|r| r.intersect(&view));
    if let Some(rect) = scene.cursor_tile().and_then(cell) {
        push(GizmoRole::Note, Gizmo::Rect { rect });
    }
    for (i, unit) in scene.units.iter().enumerate() {
        let Some(rect) = cell(unit.pos()) else {
            continue;
        };
        push(GizmoRole::Note, Gizmo::Rect { rect });
        let at = (rect.x + rect.w - 1, rect.y);
        push(
            GizmoRole::Note,
            Gizmo::Label {
                at,
                text: i.to_string(),
            },
        );
    }

    let more = scene.more;
    let edges = [
        (more.left(), (left, top), (left, bottom)),
        (more.right(), (right, top), (right, bottom)),
        (more.up(), (left, top), (right, top)),
        (more.down(), (left, bottom), (right, bottom)),
    ];
    for (_, from, to) in edges.into_iter().filter(|e| e.0) {
        push(GizmoRole::Note, line(from, to));
    }
}

/// The smallest rectangle holding both.
fn union(a: PxRect, b: PxRect) -> PxRect {
    let (x0, y0) = (a.x.min(b.x), a.y.min(b.y));
    let (x1, y1) = ((a.x + a.w).max(b.x + b.w), (a.y + a.h).max(b.y + b.h));
    Rect::new(x0, y0, x1 - x0, y1 - y0)
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use shuyi_core::Pos;

    use super::*;
    use crate::debug::gizmo::Drawn;
    use crate::frame::console::ConsoleSpec;
    use crate::grid::camera::Camera;
    use crate::grid::scene::{CursorStyle, CursorView, MapEdges};
    use crate::grid::skin::GridSkin;
    use crate::toy::grid::{GLYPH, Piece};

    /// The toy's console, 40 × 20 cells of 8 × 16 pixels.
    const CONSOLE: ConsoleSpec = ConsoleSpec::text(40, 20);

    /// The scene of a `map` map seen through `area` with the camera as
    /// near `origin` as the map lets it be, its terrain all grass, with
    /// `units` and the cursor at `cursor`.
    fn scene(
        map: (u16, u16),
        area: Rect,
        origin: Pos,
        units: &[Pos],
        cursor: Pos,
    ) -> MapScene<Piece> {
        let view = GridSkin::<Piece>::view_tiles(&GLYPH, area);
        let half = Pos::new(origin.x + view.0 / 2, origin.y + view.1 / 2);
        let camera = Camera::centred_on(half, map.0, map.1, view);
        let mut scene = MapScene::new(camera.origin, view);
        let (w, h) = (i32::from(map.0), i32::from(map.1));
        scene.set_terrain(|p| {
            ((0..w).contains(&p.x) && (0..h).contains(&p.y)).then_some(shuyi_core::TerrainId(0))
        });
        scene.more = MapEdges::of_view(camera.origin, view, (w, h));
        for (i, &pos) in (0..).zip(units) {
            scene.push_unit(Piece::new(i, 'u', pos));
        }
        scene.cursor = Some(CursorView {
            pos: cursor,
            brightness: 1.0,
            style: CursorStyle::Corners,
        });
        scene
    }

    /// The channel's gizmos, the channel on, in a frame of the console.
    fn gizmos(scene: &MapScene<Piece>, area: Rect) -> Vec<Drawn> {
        let grid = GridSkin::<Piece>::grid(&GLYPH, scene, area);
        let frame = frame();
        let mut out = Gizmos::with_on(["grid"]).within(frame);
        layout_gizmos(scene, &grid, &mut out);
        assert_eq!(out.offered(), [GRID]);
        out.drawn().to_vec()
    }

    /// The edge lines: the `Note` lines.
    fn edges(drawn: &[Drawn]) -> Vec<&Gizmo> {
        drawn
            .iter()
            .filter(|d| d.role == GizmoRole::Note && matches!(d.gizmo, Gizmo::Line { .. }))
            .map(|d| &d.gizmo)
            .collect()
    }

    const AREA: Rect = Rect::new(0, 0, 40, 20);

    /// The console's pixels.
    fn frame() -> Rect {
        let (w, h) = CONSOLE.px();
        Rect::new(0, 0, w, h)
    }

    #[test]
    fn nothing_is_built_while_the_channel_is_off() {
        let s = scene((10, 8), AREA, Pos::new(0, 0), &[], Pos::new(0, 0));
        let grid = GridSkin::<Piece>::grid(&GLYPH, &s, AREA);
        let mut off = Gizmos::new();
        layout_gizmos(&s, &grid, &mut off);
        assert!(off.is_empty());
        assert_eq!(off.offered(), [GRID]);
    }

    #[test]
    fn a_map_that_fits_the_view_has_no_edges_and_its_bounds_inside_it() {
        // 10 × 8 tiles in a view of 20 × 20: centred, all of it shown.
        let s = scene(
            (10, 8),
            AREA,
            Pos::new(2, 3),
            &[Pos::new(2, 3), Pos::new(3, 4)],
            Pos::new(2, 3),
        );
        assert_eq!(s.more, MapEdges::NONE);
        let drawn = gizmos(&s, AREA);
        assert_eq!(edges(&drawn), [] as [&Gizmo; 0]);
        let roles: Vec<_> = drawn.iter().map(|d| d.role).collect();
        // 21 + 21 cell lines, the bounds, the cursor, two units and their
        // labels, in that order.
        assert_eq!(roles.len(), 21 + 21 + 1 + 1 + 4);
        assert!(roles[..43].iter().all(|&r| r == GizmoRole::Bounds));
        assert!(roles[43..].iter().all(|&r| r == GizmoRole::Note));
        // The map's bounds: 10 × 8 tiles of 16 × 16 pixels, centred.
        let grid = GridSkin::<Piece>::grid(&GLYPH, &s, AREA);
        let map = Rect::new(
            grid.tile_px(Pos::new(0, 0)).0,
            grid.tile_px(Pos::new(0, 0)).1,
            160,
            128,
        );
        assert_eq!(drawn[42].gizmo, Gizmo::Rect { rect: map });
        let cursor = grid.rect(Pos::new(2, 3)).unwrap();
        assert_eq!(drawn[43].gizmo, Gizmo::Rect { rect: cursor });
        assert_eq!(drawn[44].gizmo, Gizmo::Rect { rect: cursor });
        let label = Gizmo::Label {
            at: (cursor.x + 15, cursor.y),
            text: "0".to_owned(),
        };
        assert_eq!(drawn[45].gizmo, label);
        assert!(matches!(&drawn[47].gizmo, Gizmo::Label { text, .. } if text == "1"));
    }

    #[test]
    fn edges_are_drawn_only_on_the_sides_the_map_goes_on_past() {
        // A 60 × 50 map, seen from its top-left corner: more right and down.
        let s = scene((60, 50), AREA, Pos::new(0, 0), &[], Pos::new(0, 0));
        assert_eq!(s.more.names(), ["right", "down"]);
        let grid = GridSkin::<Piece>::grid(&GLYPH, &s, AREA);
        let view = grid.bounds();
        let (r, b) = (view.x + view.w - 1, view.y + view.h - 1);
        let drawn = gizmos(&s, AREA);
        let right = Gizmo::Line {
            from: (r, view.y),
            to: (r, b),
        };
        let down = Gizmo::Line {
            from: (view.x, b),
            to: (r, b),
        };
        assert_eq!(edges(&drawn), [&right, &down]);
        // From the middle: every side.
        let middle = scene((60, 50), AREA, Pos::new(30, 25), &[], Pos::new(30, 25));
        assert_eq!(middle.more.names(), ["left", "right", "up", "down"]);
        assert_eq!(edges(&gizmos(&middle, AREA)).len(), 4);
    }

    /// Every pixel a gizmo names.
    fn points(gizmo: &Gizmo) -> Vec<(i32, i32)> {
        match gizmo {
            Gizmo::Rect { rect } => {
                vec![(rect.x, rect.y), (rect.x + rect.w - 1, rect.y + rect.h - 1)]
            }
            Gizmo::Line { from, to } => vec![*from, *to],
            Gizmo::Label { at, .. } | Gizmo::Cross { at, .. } => vec![*at],
            Gizmo::Circle { center, .. } => vec![*center],
        }
    }

    proptest! {
        #[test]
        fn no_gizmo_leaves_the_frame(
            map in (1_u16..80, 1_u16..80),
            area in (-6..44i32, -4..24i32, 0..60i32, 0..30i32),
            origin in (-20..90i32, -20..90i32),
            units in prop::collection::vec((-5..85i32, -5..85i32), 0..5),
            cursor in (-5..85i32, -5..85i32),
        ) {
            let area = Rect::new(area.0, area.1, area.2, area.3);
            let units: Vec<_> = units.into_iter().map(|(x, y)| Pos::new(x, y)).collect();
            let s = scene(map, area, Pos::new(origin.0, origin.1), &units, Pos::new(cursor.0, cursor.1));
            let frame = frame();
            for drawn in gizmos(&s, area) {
                if let Gizmo::Rect { rect } = drawn.gizmo {
                    prop_assert!(!rect.is_empty());
                }
                for (x, y) in points(&drawn.gizmo) {
                    prop_assert!(frame.contains(x, y), "{:?} outside {frame:?}", drawn.gizmo);
                }
            }
        }
    }
}
