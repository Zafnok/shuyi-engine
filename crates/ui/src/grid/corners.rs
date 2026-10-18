//! The corner rule (game ADR-0052): which tiles a picture drawn *between*
//! tiles joins.
//!
//! A sprite skin's layers draw one picture at every point where four tiles
//! meet, chosen by which of those four are in the layer. Two pictures side
//! by side share two of their tiles, so they always agree on the edge they
//! share, and a terrain's pictures end where its tiles end.
//!
//! [`Shown`] is the terrain a skin joins pictures by: the view's, with the
//! ring of tiles just outside it, so the picture at the view's edge is the
//! one the next tile out would give. A tile off the map counts as the
//! nearest tile on it: the ground runs on to the map's edge unchanged.

use crate::prelude::*;
use alloc::collections::BTreeSet;

use shuyi_assets::tileset::mix;
use shuyi_core::TerrainId;

use super::scene::MapScene;

/// The terrain shown on every tile of a scene's view and of the ring of
/// tiles just outside it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    /// The view's size in tiles, across × down.
    size: (i32, i32),
    /// Row by row from the tile up and left of the view's first:
    /// `(size.0 + 2) × (size.1 + 2)`.
    terrain: Vec<Option<TerrainId>>,
}

impl Shown {
    /// What `scene` shows: each tile's terrain, or what a spell being
    /// aimed would turn it into; a tile off the map (or one the scene
    /// doesn't say) as the nearest tile that has terrain.
    pub fn of<U>(scene: &MapScene<U>) -> Self {
        let (w, h) = scene.size;
        let raw = |dx: i32, dy: i32| {
            let tile = scene.tile_at(dx, dy).filter(|t| t.terrain.is_some());
            match tile {
                Some(tile) => tile.becomes.or(tile.terrain),
                None => scene.terrain_near(dx, dy),
            }
        };
        let places = || (-1..=h).flat_map(|dy| (-1..=w).map(move |dx| (dx, dy)));
        // The box round every tile that has terrain: (left, right, top,
        // bottom).
        let known = places().filter(|&(dx, dy)| raw(dx, dy).is_some());
        let bounds = known.fold(None, |bounds: Option<(i32, i32, i32, i32)>, (dx, dy)| {
            let (left, right, top, bottom) = bounds.unwrap_or((dx, dx, dy, dy));
            Some((left.min(dx), right.max(dx), top.min(dy), bottom.max(dy)))
        });
        let nearest = |dx: i32, dy: i32| {
            let (left, right, top, bottom) = bounds?;
            raw(dx.clamp(left, right), dy.clamp(top, bottom))
        };
        let terrain = places()
            .map(|(dx, dy)| raw(dx, dy).or_else(|| nearest(dx, dy)))
            .collect();
        Self {
            size: scene.size,
            terrain,
        }
    }

    /// The terrain shown on the tile `dx` right of and `dy` below the
    /// view's first: a tile of the view or of the ring round it. `None`
    /// further out, or when no tile has terrain.
    pub fn at(&self, dx: i32, dy: i32) -> Option<TerrainId> {
        let (w, h) = self.size;
        if !(-1..=w).contains(&dx) || !(-1..=h).contains(&dy) {
            return None;
        }
        let index = usize::try_from((dy + 1) * (w + 2) + dx + 1).ok()?;
        self.terrain.get(index).copied().flatten()
    }

    /// The four tiles that meet at the top-left corner of the tile `i`
    /// right of and `j` below the view's first: the ones up and left of
    /// the point, up and right, down and left, down and right.
    pub fn corners(&self, i: i32, j: i32) -> [Option<TerrainId>; 4] {
        [
            self.at(i - 1, j - 1),
            self.at(i, j - 1),
            self.at(i - 1, j),
            self.at(i, j),
        ]
    }

    /// The four neighbours of the tile `dx` right of and `dy` below the
    /// view's first: up, right, down, left.
    pub fn sides(&self, dx: i32, dy: i32) -> [Option<TerrainId>; 4] {
        [
            self.at(dx, dy - 1),
            self.at(dx + 1, dy),
            self.at(dx, dy + 1),
            self.at(dx - 1, dy),
        ]
    }
}

/// The [`mix`] of `tiles` being one of `of`.
pub fn mix_of(tiles: [Option<TerrainId>; 4], of: &BTreeSet<TerrainId>) -> u8 {
    mix(tiles.map(|tile| tile.is_some_and(|id| of.contains(&id))))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use shuyi_assets::tileset::mix_name;
    use shuyi_core::Pos;

    use super::*;
    use crate::grid::testing::any_scene;
    use crate::toy::grid::{BRIDGE, GRASS, Piece, WATER};

    /// A scene showing `rows` (`.` grass, `~` water, `=` bridge, a space
    /// off the map), its ring included: the view is the rows without
    /// their first and last, and each without its first and last tile.
    fn scene(rows: &[&str]) -> MapScene<Piece> {
        let at = |pos: Pos| {
            let row = rows.get(usize::try_from(pos.y + 1).ok()?)?;
            match row.chars().nth(usize::try_from(pos.x + 1).ok()?)? {
                '.' => Some(GRASS),
                '~' => Some(WATER),
                '=' => Some(BRIDGE),
                _ => None,
            }
        };
        let across = rows.first().map_or(0, |r| r.chars().count());
        let size = |n: usize| i32::try_from(n).unwrap() - 2;
        let mut scene = MapScene::new(Pos::new(0, 0), (size(across), size(rows.len())));
        scene.set_terrain(at);
        scene
    }

    /// The mixes of `of` at every corner point of `scene`'s view, row by
    /// row, each as its four places written out.
    fn mixes(scene: &MapScene<Piece>, of: &[TerrainId]) -> Vec<String> {
        let shown = Shown::of(scene);
        let of: BTreeSet<TerrainId> = of.iter().copied().collect();
        let (w, h) = scene.size;
        (0..=h)
            .map(|j| {
                let row: Vec<String> = (0..=w)
                    .map(|i| mix_name(mix_of(shown.corners(i, j), &of)))
                    .collect();
                row.join(" ")
            })
            .collect()
    }

    #[test]
    fn a_lake_is_ringed_by_the_mixes_of_its_corners() {
        // One water tile in the middle of a 3 × 3 view of grass.
        let lake = scene(&[".....", ".....", "..~..", ".....", "....."]);
        assert_eq!(
            mixes(&lake, &[WATER]),
            [
                ".... .... .... ....",
                ".... ...# ..#. ....",
                ".... .#.. #... ....",
                ".... .... .... ....",
            ]
        );
        // The grass is everywhere else.
        assert_eq!(
            mixes(&lake, &[GRASS])[1..3],
            ["#### ###. ##.# ####", "#### #.## .### ####"]
        );
        let shown = Shown::of(&lake);
        assert_eq!(shown.at(1, 1), Some(WATER));
        assert_eq!(shown.at(0, 1), Some(GRASS));
        // The ring is there; one tile further isn't.
        assert_eq!(shown.at(-1, -1), Some(GRASS));
        assert_eq!(shown.at(3, 3), Some(GRASS));
        for (dx, dy) in [(-2, 0), (4, 0), (0, -2), (0, 4)] {
            assert_eq!(shown.at(dx, dy), None, "({dx}, {dy})");
        }
        // Up, right, down, left of the lake and of the tile left of it.
        assert_eq!(shown.sides(1, 1), [Some(GRASS); 4]);
        assert_eq!(
            shown.sides(0, 1),
            [Some(GRASS), Some(WATER), Some(GRASS), Some(GRASS)]
        );
        let water = BTreeSet::from([WATER]);
        assert_eq!(mix_of(shown.sides(0, 1), &water), 0b0100);
        assert_eq!(mix_of(shown.sides(1, 0), &water), 0b0010);
        assert_eq!(mix_of(shown.sides(2, 1), &water), 0b0001);
        assert_eq!(mix_of(shown.sides(1, 2), &water), 0b1000);
    }

    #[test]
    fn a_river_runs_on_past_the_view_and_a_bridge_is_what_its_layer_says() {
        // A river down the middle column, running on above and below the
        // view, with a bridge on its middle tile.
        let river = scene(&["..~..", "..~..", "..=..", "..~..", "..~.."]);
        // The bridge counts as water for the shore: it is unbroken, and
        // runs to the view's edge.
        assert_eq!(
            mixes(&river, &[WATER, BRIDGE]),
            [
                ".... .#.# #.#. ....",
                ".... .#.# #.#. ....",
                ".... .#.# #.#. ....",
                ".... .#.# #.#. ....",
            ]
        );
        // As its own layer it is one tile.
        assert_eq!(
            mixes(&river, &[BRIDGE])[1..3],
            [".... ...# ..#. ....", ".... .#.. #... ...."]
        );
        // The bridge has water above and below: it crosses left to right.
        let shown = Shown::of(&river);
        let water = BTreeSet::from([WATER]);
        assert_eq!(mix_of(shown.sides(1, 1), &water), 0b1010);
        // A river the view cuts off, with nothing given outside it, runs
        // on the same.
        let mut cut = river.clone();
        cut.rim = vec![None; cut.rim.len()];
        assert_eq!(
            mixes(&cut, &[WATER, BRIDGE]),
            mixes(&river, &[WATER, BRIDGE])
        );
    }

    #[test]
    fn a_tile_off_the_map_counts_as_the_nearest_tile_on_it() {
        // A 2 × 2 map in the middle of a 4 × 4 view: grass, but water at
        // its bottom-right.
        let small = scene(&["      ", "      ", "  ..  ", "  .~  ", "      ", "      "]);
        let shown = Shown::of(&small);
        // Above and left of the map: its top-left tile's.
        assert_eq!(shown.at(0, 0), Some(GRASS));
        assert_eq!(shown.at(-1, 2), Some(GRASS));
        // Right of and below the water: water.
        assert_eq!(shown.at(3, 2), Some(WATER));
        assert_eq!(shown.at(2, 4), Some(WATER));
        assert_eq!(shown.at(4, 4), Some(WATER));
        // Right of the grass above it: grass.
        assert_eq!(shown.at(3, 1), Some(GRASS));
        // So the water's shore is only inside the map: at the point in
        // its middle. Along the map's edge it runs straight out.
        assert_eq!(
            mixes(&small, &[WATER]),
            [
                ".... .... .... .... ....",
                ".... .... .... .... ....",
                ".... .... ...# ..## ..##",
                ".... .... .#.# #### ####",
                ".... .... .#.# #### ####",
            ]
        );
        // No terrain anywhere: nothing shown, nothing in any layer.
        let empty = MapScene::<Piece>::new(Pos::new(0, 0), (2, 2));
        let shown = Shown::of(&empty);
        assert_eq!(shown.at(0, 0), None);
        assert_eq!(mix_of(shown.corners(1, 1), &BTreeSet::from([GRASS])), 0);
        // A view of no tiles still has its ring.
        let none = Shown::of(&MapScene::<Piece>::new(Pos::new(0, 0), (0, 0)));
        assert_eq!(none.terrain.len(), 4);
        assert_eq!(none.at(0, 0), None);
    }

    #[test]
    fn a_tile_a_spell_would_change_is_shown_as_what_it_would_become() {
        let mut lake = scene(&[".....", ".....", "..~..", ".....", "....."]);
        lake.tiles[4].becomes = Some(GRASS);
        lake.tiles[0].becomes = Some(WATER);
        let shown = Shown::of(&lake);
        assert_eq!(shown.at(1, 1), Some(GRASS));
        assert_eq!(shown.at(0, 0), Some(WATER));
        assert_eq!(mixes(&lake, &[WATER])[0], "...# ..#. .... ....");
        // A tile off the map becomes nothing.
        let mut off = scene(&["   ", " . ", "   "]);
        off.tiles[0].terrain = None;
        off.tiles[0].becomes = Some(WATER);
        assert_eq!(Shown::of(&off).at(0, 0), None);
    }

    #[test]
    fn a_mix_counts_only_the_tiles_of_the_layer() {
        let of = BTreeSet::from([WATER, BRIDGE]);
        let tiles = [Some(WATER), Some(GRASS), None, Some(BRIDGE)];
        assert_eq!(mix_of(tiles, &of), 0b1001);
        assert_eq!(mix_of([None; 4], &of), 0);
        assert_eq!(mix_of([Some(WATER); 4], &of), 15);
        assert_eq!(mix_of([Some(WATER); 4], &BTreeSet::new()), 0);
    }

    proptest! {
        /// Whatever the scene, two pictures side by side (or one above
        /// the other) join the same two tiles along the edge they share,
        /// and the picture round a tile's corners all name that tile.
        #[test]
        fn neighbouring_corner_points_share_their_tiles(scene in any_scene()) {
            let shown = Shown::of(&scene);
            let (w, h) = scene.size;
            for j in 0..=h {
                for i in 0..=w {
                    let [top_left, top_right, bottom_left, bottom_right] = shown.corners(i, j);
                    let right = shown.corners(i + 1, j);
                    let below = shown.corners(i, j + 1);
                    if i < w {
                        prop_assert_eq!((top_right, bottom_right), (right[0], right[2]));
                    }
                    if j < h {
                        prop_assert_eq!((bottom_left, bottom_right), (below[0], below[1]));
                    }
                    prop_assert_eq!(bottom_right, shown.at(i, j));
                    prop_assert_eq!(top_left, shown.at(i - 1, j - 1));
                }
            }
        }

        /// Whatever the scene, a tile of the view that is on the map
        /// shows its own terrain (or what it would become), and the rest
        /// show terrain only if some tile has any.
        #[test]
        fn a_tile_on_the_map_shows_its_own_terrain(scene in any_scene()) {
            let shown = Shown::of(&scene);
            let (w, h) = scene.size;
            let mut any = false;
            for dy in -1..=h {
                for dx in -1..=w {
                    let own = scene.tile_at(dx, dy).filter(|t| t.terrain.is_some());
                    if let Some(tile) = own {
                        prop_assert_eq!(shown.at(dx, dy), tile.becomes.or(tile.terrain));
                    }
                    any |= scene.terrain_near(dx, dy).is_some();
                }
            }
            if !any {
                prop_assert!(shown.terrain.iter().all(Option::is_none));
            }
        }
    }
}
