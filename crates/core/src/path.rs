//! Paths over a square grid: where a walker can go on a budget and the
//! cheapest path there ([`reachable`]), and how far every tile is from a
//! set of goals ([`flow_field`]). The game says what a tile costs with a
//! cost function; the rules of who may stand where stay the game's.
//!
//! # Rules
//!
//! - **Steps.** Only the four grid directions ([`Dir`]) are steps. A step
//!   pays the cost of the tile it enters: the cost function's answer for
//!   that tile, or no step at all when it answers `None`. The start costs
//!   nothing. The cost function is only asked about tiles inside the grid.
//! - **Budget.** [`reachable`] keeps the tiles whose cheapest cost is at
//!   most the budget.
//! - **Determinism.** Both searches are one Dijkstra. Its queue is ordered
//!   by `(cost, y, x)`, neighbours are visited in [`Dir::ALL`] order, and a
//!   tile's recorded predecessor only changes for a strictly cheaper cost.
//!   So among equally cheap paths to a tile, the one through the neighbour
//!   expanded first wins, every time. When every step costs at least 1,
//!   tiles are expanded in `(cost, y, x)` order, so that neighbour is the
//!   one with the smallest `(cost, y, x)` among those on a cheapest path.
//!   Steps that cost 0 can reach a tile at the cost being expanded but with
//!   a smaller `(y, x)` than tiles already expanded at that cost, so then
//!   the order is only cheapest first.
//! - A cost that would pass `u32::MAX` is no step.

use alloc::collections::BinaryHeap;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Reverse;

use crate::grid::{Dir, Grid, Pos, TileSet};

/// Where a walker can go from one tile on a budget, with the cheapest path
/// to each tile. Built by [`reachable`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reach {
    origin: Pos,
    budget: u32,
    cost: Grid<Option<u32>>,
    prev: Grid<Option<Pos>>,
}

impl Reach {
    /// The tile the walk starts on.
    pub fn origin(&self) -> Pos {
        self.origin
    }

    /// The budget the walk had.
    pub fn budget(&self) -> u32 {
        self.budget
    }

    /// The cheapest cost to reach `pos`, or `None` if it is out of reach.
    pub fn cost(&self, pos: Pos) -> Option<u32> {
        self.cost.get(pos).copied().flatten()
    }

    /// Whether `pos` is within reach.
    pub fn contains(&self, pos: Pos) -> bool {
        self.cost(pos).is_some()
    }

    /// Every tile within reach, the origin included (unless it lies outside
    /// the grid).
    pub fn tiles(&self) -> TileSet {
        let mut set = TileSet::new(self.cost.width(), self.cost.height());
        for (pos, cost) in self.cost.positions().zip(self.cost.cells()) {
            if cost.is_some() {
                set.insert(pos);
            }
        }
        set
    }

    /// The cheapest path from the origin to `dest`, both included, or
    /// `None` if `dest` is out of reach. Deterministic (see the module
    /// docs).
    pub fn path_to(&self, dest: Pos) -> Option<Vec<Pos>> {
        self.cost(dest)?;
        let mut path = vec![dest];
        let mut cur = dest;
        while let Some(&Some(p)) = self.prev.get(cur) {
            path.push(p);
            cur = p;
        }
        path.reverse();
        Some(path)
    }
}

/// Where a walker on `start` can go with `budget` to spend on a
/// `width × height` grid, where entering a tile costs `cost(tile)` (`None`:
/// it can't be entered). A start outside the grid reaches nothing.
pub fn reachable(
    width: u16,
    height: u16,
    start: Pos,
    budget: u32,
    cost: impl Fn(Pos) -> Option<u32>,
) -> Reach {
    let found = search(width, height, [start], budget, |_| Some(0), cost);
    Reach {
        origin: start,
        budget,
        cost: found.cost,
        prev: found.prev,
    }
}

/// How far every tile of a `width × height` grid is from the nearest of
/// `goals`: the cost of the cheapest walk from the tile onto a goal, where
/// entering a tile costs `cost(tile)`, as in [`reachable`], and there is no
/// budget. A goal is 0 from itself. A tile that can't be entered has no
/// distance (unless it is a goal), and neither has a tile with no walk to a
/// goal: a goal that can't be entered is never walked onto. Goals outside
/// the grid are left out.
pub fn flow_field(
    width: u16,
    height: u16,
    goals: impl IntoIterator<Item = Pos>,
    cost: impl Fn(Pos) -> Option<u32>,
) -> Grid<Option<u32>> {
    // Searched from the goals outwards, so each step is a walk's step
    // backwards: from a tile to the neighbour a walker comes from. It pays
    // for the tile walked onto, and the walker must be able to stand on the
    // neighbour.
    let found = search(width, height, goals, u32::MAX, &cost, |pos| {
        cost(pos).map(|_| 0)
    });
    found.cost
}

/// What [`search`] found: each tile's cheapest cost and the tile it was
/// reached from.
struct Search {
    cost: Grid<Option<u32>>,
    prev: Grid<Option<Pos>>,
}

/// The one Dijkstra (see the module docs): from every tile of `sources` at
/// cost 0, a step from a tile `a` to its neighbour `b` costs
/// `leave(a) + enter(b)` (no step if either is `None`), and only costs
/// within `budget` are kept.
fn search(
    width: u16,
    height: u16,
    sources: impl IntoIterator<Item = Pos>,
    budget: u32,
    leave: impl Fn(Pos) -> Option<u32>,
    enter: impl Fn(Pos) -> Option<u32>,
) -> Search {
    let mut cost = Grid::filled(width, height, None);
    let mut prev = Grid::filled(width, height, None);
    let mut queue: BinaryHeap<Reverse<(u32, i32, i32)>> = BinaryHeap::new();
    for source in sources {
        if let Some(c) = cost.get_mut(source) {
            *c = Some(0);
            queue.push(Reverse((0, source.y, source.x)));
        }
    }
    // Tiles already expanded: their cost and predecessor are final.
    let mut done = TileSet::new(width, height);
    while let Some(Reverse((c, y, x))) = queue.pop() {
        let pos = Pos::new(x, y);
        if !done.insert(pos) {
            continue; // A stale entry: `pos` was expanded at a lower cost.
        }
        let Some(out) = leave(pos) else {
            continue;
        };
        for next in Dir::ALL.map(|dir| pos.step(dir)) {
            if !cost.in_bounds(next) || done.contains(next) {
                continue;
            }
            let step = enter(next).and_then(|step| step.checked_add(out));
            let Some(next_cost) = step
                .and_then(|step| c.checked_add(step))
                .filter(|&total| total <= budget)
            else {
                continue;
            };
            let Some(slot) = cost.get_mut(next) else {
                continue;
            };
            if slot.is_none_or(|old| next_cost < old) {
                *slot = Some(next_cost);
                if let Some(p) = prev.get_mut(next) {
                    *p = Some(pos);
                }
                queue.push(Reverse((next_cost, next.y, next.x)));
            }
        }
    }
    Search { cost, prev }
}

#[cfg(test)]
mod tests;
