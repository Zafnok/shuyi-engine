//! Scenes and areas of every shape, for the grid's property tests: the
//! toy's pieces, its terrain ids and ids past them, ranges by the toy's
//! names, every cursor style, areas partly off the console.

use proptest::prelude::*;
use shuyi_assets::grid_map::MapLook;
use shuyi_core::{Pos, TerrainId};

use super::scene::{
    CursorStyle, CursorView, MapScene, RangeLayer, RangeName, ThreatView, TileView,
};
use crate::frame::glyph_buffer::Rect;
use crate::toy::grid::Piece;

/// The toy's ranges, and one its palette lacks.
const RANGES: [RangeName; 5] = [
    RangeName("move"),
    RangeName("attack"),
    RangeName("danger"),
    RangeName("heal"),
    RangeName("unknown"),
];

prop_compose! {
    pub(crate) fn any_pos()(x in -25..60i32, y in -25..60i32) -> Pos {
        Pos::new(x, y)
    }
}

prop_compose! {
    pub(crate) fn any_tile()(
        // Past the last terrain too: an id no terrain has.
        terrain in prop::option::of(0u16..6),
        flashes in prop::collection::vec(0.0f32..1.0, 0..2),
        becomes in prop::option::of(0u16..6),
    ) -> TileView {
        TileView {
            terrain: terrain.map(TerrainId),
            flashes,
            becomes: becomes.map(TerrainId),
        }
    }
}

prop_compose! {
    pub(crate) fn any_range()(
        kind in 0usize..5,
        tiles in prop::collection::vec(any_pos(), 0..30),
    ) -> RangeLayer {
        RangeLayer { kind: RANGES[kind], tiles }
    }
}

prop_compose! {
    pub(crate) fn any_piece()(
        id in 0u32..10,
        letter in prop::char::range('a', 'z'),
        pos in any_pos(),
    ) -> Piece {
        Piece::new(id, letter, pos)
    }
}

prop_compose! {
    pub(crate) fn any_cursor()(
        pos in any_pos(),
        brightness in 0.5f32..1.0,
        style in prop::sample::select(vec![
            CursorStyle::Corners,
            CursorStyle::LargeCorners,
            CursorStyle::TileGlow,
        ]),
    ) -> CursorView {
        CursorView { pos, brightness, style }
    }
}

prop_compose! {
    pub(crate) fn any_scene()(
        origin in any_pos(),
        size in (-2..45i32, -2..40i32),
        tiles in prop::collection::vec(any_tile(), 0..200),
        ranges in prop::collection::vec(any_range(), 0..4),
        rim in prop::collection::vec(prop::option::of(0u16..6), 0..180),
        look in prop::sample::select(vec!["outdoor", "indoor", "cave"]),
        light in prop::sample::select(vec!["noon", "dusk", "midnight"]),
        units in prop::collection::vec(any_piece(), 0..8),
        cursor in prop::option::of(any_cursor()),
        path in prop::collection::vec(any_pos(), 0..6),
        threat in prop::option::of((any_pos(), prop::collection::vec(any_pos(), 1..4))),
        clock_ms in 0u64..5000,
    ) -> MapScene<Piece> {
        let rim = rim.into_iter().map(|id| id.map(TerrainId)).collect();
        let look = MapLook { tiles: look.to_owned(), light: light.to_owned() };
        let threat = threat.map(|(tile, enemies)| ThreatView { tile, enemies });
        MapScene { origin, size, tiles, ranges, rim, look, units, cursor, path, threat, clock_ms }
    }
}

prop_compose! {
    pub(crate) fn any_area()(area in (-6..104i32, -4..34i32, -2..110i32, -2..40i32)) -> Rect {
        Rect::new(area.0, area.1, area.2, area.3)
    }
}
