//! Grid geometry: positions, the four directions, a rectangular grid and
//! sets of its tiles, and the terrain a tile is.

use alloc::vec;
use alloc::vec::Vec;

use core::fmt;

use serde::{Deserialize, Serialize};

/// A tile position. Signed so neighbour maths can go negative before bounds
/// checks; `(0, 0)` is the top-left tile, `x` grows right, `y` grows down.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct Pos {
    /// Column.
    pub x: i32,
    /// Row.
    pub y: i32,
}

/// A terrain: its index in the game's table of terrains. The engine knows
/// no terrain of its own; a game's map is a [`Grid`] of these, and its
/// skins look each one up in the game's table.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
pub struct TerrainId(pub u16);

impl Pos {
    /// Creates a position.
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    /// Manhattan (4-way) distance between `a` and `b`.
    pub fn manhattan(a: Pos, b: Pos) -> u32 {
        a.x.abs_diff(b.x) + a.y.abs_diff(b.y)
    }

    /// The adjacent position in direction `dir` (may be out of bounds).
    #[must_use]
    pub fn step(self, dir: Dir) -> Pos {
        let (dx, dy) = dir.delta();
        Pos::new(self.x + dx, self.y + dy)
    }
}

/// One of the four grid directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Dir {
    /// `+x`.
    Right,
    /// `+y`.
    Down,
    /// `-x`.
    Left,
    /// `-y`.
    Up,
}

impl Dir {
    /// Every direction in the fixed order used for deterministic iteration.
    pub const ALL: [Dir; 4] = [Dir::Right, Dir::Down, Dir::Left, Dir::Up];

    /// The `(dx, dy)` offset of one step in this direction.
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Dir::Right => (1, 0),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
            Dir::Up => (0, -1),
        }
    }
}

/// A `width × height` rectangle of cells, stored row-major. The cell count
/// always equals `width * height`; deserialising checks it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawGrid<T>")]
pub struct Grid<T> {
    width: u16,
    height: u16,
    cells: Vec<T>,
}

impl<T> Grid<T> {
    /// Builds a grid from row-major `cells`; `None` if the count is not
    /// `width * height`.
    pub fn from_cells(width: u16, height: u16, cells: Vec<T>) -> Option<Self> {
        (cells.len() == usize::from(width) * usize::from(height)).then_some(Self {
            width,
            height,
            cells,
        })
    }

    /// A `width × height` grid with every cell set to `value`.
    pub fn filled(width: u16, height: u16, value: T) -> Self
    where
        T: Clone,
    {
        Self {
            width,
            height,
            cells: vec![value; usize::from(width) * usize::from(height)],
        }
    }

    /// Number of columns.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Number of rows.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Every cell, row-major.
    pub fn cells(&self) -> &[T] {
        &self.cells
    }

    /// Whether `pos` is inside the grid.
    pub fn in_bounds(&self, pos: Pos) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < i32::from(self.width) && pos.y < i32::from(self.height)
    }

    /// The cell at `pos`, or `None` if out of bounds.
    pub fn get(&self, pos: Pos) -> Option<&T> {
        self.cells.get(self.index(pos)?)
    }

    /// The cell at `pos` for writing, or `None` if out of bounds.
    pub fn get_mut(&mut self, pos: Pos) -> Option<&mut T> {
        let i = self.index(pos)?;
        self.cells.get_mut(i)
    }

    /// The row-major index of `pos`, or `None` if out of bounds.
    fn index(&self, pos: Pos) -> Option<usize> {
        if !self.in_bounds(pos) {
            return None;
        }
        // In bounds, so both coordinates are non-negative and small.
        let x = usize::try_from(pos.x).ok()?;
        let y = usize::try_from(pos.y).ok()?;
        Some(y * usize::from(self.width) + x)
    }

    /// The in-bounds neighbours of `pos`, in [`Dir::ALL`] order.
    pub fn neighbors4(&self, pos: Pos) -> impl Iterator<Item = Pos> + '_ {
        Dir::ALL
            .into_iter()
            .map(move |d| pos.step(d))
            .filter(|&p| self.in_bounds(p))
    }

    /// Every position, row-major (the order of [`Grid::cells`]).
    pub fn positions(&self) -> impl Iterator<Item = Pos> + use<T> {
        let (w, h) = (i32::from(self.width), i32::from(self.height));
        (0..h).flat_map(move |y| (0..w).map(move |x| Pos::new(x, y)))
    }
}

/// A [`Grid`] as written in a save, before its cell count is checked.
#[derive(Deserialize)]
struct RawGrid<T> {
    width: u16,
    height: u16,
    cells: Vec<T>,
}

/// A saved grid whose cell count is not `width * height`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSizeError;

impl fmt::Display for GridSizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the grid's cell count is not width × height")
    }
}

impl<T> TryFrom<RawGrid<T>> for Grid<T> {
    type Error = GridSizeError;

    fn try_from(raw: RawGrid<T>) -> Result<Self, GridSizeError> {
        Grid::from_cells(raw.width, raw.height, raw.cells).ok_or(GridSizeError)
    }
}

/// A set of tiles of a `width × height` map, stored as a bitset. Iterates in
/// row-major order. Positions outside the map are never members.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TileSet {
    width: u16,
    height: u16,
    words: Vec<u64>,
}

impl TileSet {
    /// An empty set for a `width × height` map.
    pub fn new(width: u16, height: u16) -> Self {
        let bits = usize::from(width) * usize::from(height);
        Self {
            width,
            height,
            words: vec![0; bits.div_ceil(64)],
        }
    }

    /// Map width.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Map height.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Whether `pos` is in the set.
    pub fn contains(&self, pos: Pos) -> bool {
        self.bit(pos)
            .is_some_and(|(w, mask)| self.words.get(w).is_some_and(|word| word & mask != 0))
    }

    /// Adds `pos`; returns whether it was newly added (`false` if already a
    /// member or outside the map).
    pub fn insert(&mut self, pos: Pos) -> bool {
        let Some((w, mask)) = self.bit(pos) else {
            return false;
        };
        let Some(word) = self.words.get_mut(w) else {
            return false;
        };
        let added = *word & mask == 0;
        *word |= mask;
        added
    }

    /// Adds every member of `other` that lies inside this set's map.
    pub fn union_with(&mut self, other: &TileSet) {
        for pos in other.iter() {
            self.insert(pos);
        }
    }

    /// Number of members.
    pub fn len(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// Whether the set has no members.
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|&w| w == 0)
    }

    /// Every member, row-major.
    pub fn iter(&self) -> impl Iterator<Item = Pos> + '_ {
        let width = usize::from(self.width);
        self.words
            .iter()
            .enumerate()
            .filter(|&(_, &word)| word != 0)
            .flat_map(move |(w, &word)| {
                (0..64)
                    .filter(move |bit| word >> bit & 1 == 1)
                    .filter_map(move |bit| {
                        let index = w * 64 + bit;
                        // Both fit: index < width * height ≤ u16::MAX².
                        let x = i32::try_from(index % width).ok()?;
                        let y = i32::try_from(index / width).ok()?;
                        Some(Pos::new(x, y))
                    })
            })
    }

    /// The word index and bit mask of `pos`, or `None` outside the map.
    fn bit(&self, pos: Pos) -> Option<(usize, u64)> {
        let x = u16::try_from(pos.x).ok().filter(|&x| x < self.width)?;
        let y = u16::try_from(pos.y).ok().filter(|&y| y < self.height)?;
        let index = usize::from(y) * usize::from(self.width) + usize::from(x);
        Some((index / 64, 1 << (index % 64)))
    }
}

#[cfg(test)]
mod tests {
    use alloc::collections::BTreeSet;
    use alloc::string::ToString;

    use proptest::prelude::*;

    use super::*;

    fn grid(w: u16, h: u16) -> Grid<u32> {
        let n = u32::from(w) * u32::from(h);
        Grid::from_cells(w, h, (0..n).collect()).unwrap_or_else(|| unreachable!())
    }

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    #[test]
    fn manhattan_distance() {
        assert_eq!(Pos::manhattan(Pos::new(0, 0), Pos::new(3, 4)), 7);
        assert_eq!(Pos::manhattan(Pos::new(3, 4), Pos::new(0, 0)), 7);
        assert_eq!(Pos::manhattan(Pos::new(-2, 5), Pos::new(1, 1)), 7);
        assert_eq!(Pos::manhattan(Pos::new(2, 2), Pos::new(2, 2)), 0);
    }

    #[test]
    fn step_follows_dir_delta() {
        let p = Pos::new(5, 5);
        assert_eq!(p.step(Dir::Right), Pos::new(6, 5));
        assert_eq!(p.step(Dir::Down), Pos::new(5, 6));
        assert_eq!(p.step(Dir::Left), Pos::new(4, 5));
        assert_eq!(p.step(Dir::Up), Pos::new(5, 4));
    }

    #[test]
    fn from_cells_checks_count() {
        assert!(Grid::from_cells(2, 3, vec![0; 6]).is_some());
        assert!(Grid::from_cells(2, 3, vec![0; 5]).is_none());
        assert!(Grid::from_cells(2, 3, vec![0; 7]).is_none());
        let g = Grid::from_cells(0, 0, Vec::<u8>::new());
        assert!(g.is_some_and(|g| !g.in_bounds(Pos::new(0, 0))));
    }

    #[test]
    fn dimensions_and_cells() {
        let g = grid(3, 2);
        assert_eq!((g.width(), g.height()), (3, 2));
        assert_eq!(g.cells(), &[0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn bounds_and_get() {
        let g = grid(3, 2);
        assert!(g.in_bounds(Pos::new(0, 0)));
        assert!(g.in_bounds(Pos::new(2, 1)));
        assert!(!g.in_bounds(Pos::new(3, 0)));
        assert!(!g.in_bounds(Pos::new(0, 2)));
        assert!(!g.in_bounds(Pos::new(-1, 0)));
        assert!(!g.in_bounds(Pos::new(0, -1)));
        assert_eq!(g.get(Pos::new(0, 0)), Some(&0));
        assert_eq!(g.get(Pos::new(2, 0)), Some(&2));
        assert_eq!(g.get(Pos::new(1, 1)), Some(&4));
        assert_eq!(g.get(Pos::new(2, 1)), Some(&5));
        assert_eq!(g.get(Pos::new(3, 0)), None);
        assert_eq!(g.get(Pos::new(-1, 1)), None);
    }

    #[test]
    fn filled_and_get_mut() {
        let mut g = Grid::filled(3, 2, 7u8);
        assert_eq!((g.width(), g.height()), (3, 2));
        assert_eq!(g.cells(), &[7; 6]);
        if let Some(c) = g.get_mut(Pos::new(2, 1)) {
            *c = 9;
        }
        if let Some(c) = g.get_mut(Pos::new(1, 0)) {
            *c = 5;
        }
        assert_eq!(g.cells(), &[7, 5, 7, 7, 7, 9]);
        assert!(g.get_mut(Pos::new(3, 0)).is_none());
        assert!(g.get_mut(Pos::new(0, -1)).is_none());
    }

    #[test]
    fn neighbors_in_fixed_order() {
        let g = grid(3, 3);
        let n: Vec<Pos> = g.neighbors4(Pos::new(1, 1)).collect();
        assert_eq!(
            n,
            [
                Pos::new(2, 1),
                Pos::new(1, 2),
                Pos::new(0, 1),
                Pos::new(1, 0)
            ]
        );
        let corner: Vec<Pos> = g.neighbors4(Pos::new(0, 0)).collect();
        assert_eq!(corner, [Pos::new(1, 0), Pos::new(0, 1)]);
        let far: Vec<Pos> = g.neighbors4(Pos::new(2, 2)).collect();
        assert_eq!(far, [Pos::new(1, 2), Pos::new(2, 1)]);
    }

    #[test]
    fn positions_are_row_major() {
        let g = grid(2, 2);
        let p: Vec<Pos> = g.positions().collect();
        assert_eq!(
            p,
            [
                Pos::new(0, 0),
                Pos::new(1, 0),
                Pos::new(0, 1),
                Pos::new(1, 1)
            ]
        );
        for (pos, cell) in g.positions().zip(g.cells()) {
            assert_eq!(g.get(pos), Some(cell));
        }
    }

    #[test]
    fn grid_serde_round_trips_and_checks_the_cell_count() {
        let g = grid(3, 2);
        let text = ron::to_string(&g).unwrap();
        assert_eq!(text, "(width:3,height:2,cells:[0,1,2,3,4,5])");
        assert_eq!(ron::from_str::<Grid<u32>>(&text).unwrap(), g);
        let bad = ron::from_str::<Grid<u32>>("(width:3,height:2,cells:[0,1])");
        assert!(bad.unwrap_err().to_string().contains("not width × height"));
        assert_eq!(ron::from_str::<Pos>("(x:-1,y:4)").unwrap(), Pos::new(-1, 4));
    }

    #[test]
    fn tile_set_basics() {
        let mut set = TileSet::new(10, 10);
        assert_eq!((set.width(), set.height()), (10, 10));
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert!(set.insert(p(4, 6)));
        assert!(!set.insert(p(4, 6)));
        assert!(set.insert(p(3, 6)));
        assert!(set.insert(p(9, 9)));
        assert!(set.insert(p(0, 0)));
        for outside in [p(-1, 0), p(10, 0), p(0, 10), p(0, -1), p(10, 10)] {
            assert!(!set.insert(outside));
            assert!(!set.contains(outside));
        }
        assert!(!set.is_empty());
        assert_eq!(set.len(), 4);
        assert!(set.contains(p(4, 6)));
        assert!(set.contains(p(3, 6)));
        assert!(!set.contains(p(5, 6)));
        assert!(!set.contains(p(6, 4)));
        // Row-major, across the 64-bit word boundary (index 63 | 64).
        let members: Vec<Pos> = set.iter().collect();
        assert_eq!(members, [p(0, 0), p(3, 6), p(4, 6), p(9, 9)]);
    }

    #[test]
    fn tile_set_word_boundaries() {
        // 3 × 43 = 129 tiles: three words, the last with one bit.
        let mut set = TileSet::new(3, 43);
        let all: Vec<Pos> = (0..43).flat_map(|y| (0..3).map(move |x| p(x, y))).collect();
        for &pos in &all {
            assert!(set.insert(pos));
        }
        assert_eq!(set.len(), 129);
        assert_eq!(set.iter().collect::<Vec<_>>(), all);
        assert!(!set.contains(p(0, 43)));
        assert!(TileSet::new(0, 0).is_empty());
        assert_eq!(TileSet::new(0, 5).iter().count(), 0);
    }

    #[test]
    fn tile_set_union() {
        let mut a = TileSet::new(3, 2);
        a.insert(p(0, 0));
        let mut b = TileSet::new(4, 4);
        b.insert(p(2, 1));
        b.insert(p(3, 3)); // Outside `a`'s map: dropped.
        b.insert(p(0, 0));
        a.union_with(&b);
        assert_eq!(a.iter().collect::<Vec<_>>(), [p(0, 0), p(2, 1)]);
        assert_eq!(a.len(), 2);
    }

    proptest! {
        #[test]
        fn neighbors_stay_in_bounds(w in 0u16..20, h in 0u16..20, x in -3i32..25, y in -3i32..25) {
            let g = grid(w, h);
            let pos = Pos::new(x, y);
            for n in g.neighbors4(pos) {
                prop_assert!(g.in_bounds(n));
                prop_assert_eq!(Pos::manhattan(pos, n), 1);
            }
            if g.in_bounds(pos) {
                let expected = Dir::ALL.iter().filter(|&&d| g.in_bounds(pos.step(d))).count();
                prop_assert_eq!(g.neighbors4(pos).count(), expected);
            }
        }

        #[test]
        fn tile_set_matches_a_btree_set(
            w in 0u16..20,
            h in 0u16..20,
            ops in prop::collection::vec((-2i32..22, -2i32..22), 0..60),
        ) {
            let mut set = TileSet::new(w, h);
            let mut model = BTreeSet::new();
            for (x, y) in ops {
                let pos = p(x, y);
                let inside = x >= 0 && y >= 0 && x < i32::from(w) && y < i32::from(h);
                let added = inside && model.insert((y, x));
                prop_assert_eq!(set.insert(pos), added);
            }
            prop_assert_eq!(set.len(), model.len());
            prop_assert_eq!(set.is_empty(), model.is_empty());
            let members: Vec<(i32, i32)> = set.iter().map(|q| (q.y, q.x)).collect();
            prop_assert_eq!(members, model.into_iter().collect::<Vec<_>>());
        }
    }
}
