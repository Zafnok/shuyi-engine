//! Tests of the paths. A [`Costs`] grid answers what entering each tile
//! costs and panics when asked about a tile outside it, so every test also
//! checks that the searches never ask.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::String;

use proptest::prelude::*;

use super::*;

/// What entering each tile of a grid costs (`None`: it can't be entered).
#[derive(Debug, Clone)]
struct Costs {
    width: u16,
    height: u16,
    cells: Vec<Option<u32>>,
}

impl Costs {
    /// A grid from rows of text: a digit is the tile's cost, `#` can't be
    /// entered.
    fn parse(rows: &[&str]) -> Self {
        let width = u16::try_from(rows[0].len()).unwrap();
        let height = u16::try_from(rows.len()).unwrap();
        let cells = rows
            .iter()
            .flat_map(|row| row.chars())
            .map(|ch| ch.to_digit(10))
            .collect();
        Costs {
            width,
            height,
            cells,
        }
    }

    /// The cost of entering `pos`.
    ///
    /// # Panics
    ///
    /// When `pos` is outside the grid: the searches never ask about one.
    fn cost(&self, pos: Pos) -> Option<u32> {
        let x = usize::try_from(pos.x)
            .ok()
            .filter(|&x| x < usize::from(self.width));
        let y = usize::try_from(pos.y)
            .ok()
            .filter(|&y| y < usize::from(self.height));
        let (Some(x), Some(y)) = (x, y) else {
            panic!("asked about {pos:?}, outside the grid");
        };
        self.cells[y * usize::from(self.width) + x]
    }

    fn in_bounds(&self, pos: Pos) -> bool {
        pos.x >= 0 && pos.y >= 0 && pos.x < i32::from(self.width) && pos.y < i32::from(self.height)
    }

    fn positions(&self) -> impl Iterator<Item = Pos> + use<> {
        let (w, h) = (i32::from(self.width), i32::from(self.height));
        (0..h).flat_map(move |y| (0..w).map(move |x| Pos::new(x, y)))
    }

    fn neighbors(&self, pos: Pos) -> Vec<Pos> {
        Dir::ALL
            .map(|d| pos.step(d))
            .into_iter()
            .filter(|&p| self.in_bounds(p))
            .collect()
    }

    fn reach(&self, start: Pos, budget: u32) -> Reach {
        reachable(self.width, self.height, start, budget, |p| self.cost(p))
    }

    fn flow(&self, goals: &[Pos]) -> Grid<Option<u32>> {
        flow_field(self.width, self.height, goals.iter().copied(), |p| {
            self.cost(p)
        })
    }

    /// Cheapest costs from `start` within `budget`, by brute-force
    /// relaxation (Bellman–Ford): every step is tried until nothing gets
    /// cheaper.
    fn brute_reach(&self, start: Pos, budget: u32) -> BTreeMap<Pos, u32> {
        let mut best = BTreeMap::new();
        if self.in_bounds(start) {
            best.insert(start, 0);
        }
        loop {
            let mut changed = false;
            for (&pos, &c) in &best.clone() {
                for next in self.neighbors(pos) {
                    let Some(step) = self.cost(next) else {
                        continue;
                    };
                    let total = c + step;
                    if total <= budget && best.get(&next).is_none_or(|&old| total < old) {
                        best.insert(next, total);
                        changed = true;
                    }
                }
            }
            if !changed {
                return best;
            }
        }
    }

    /// The cheapest walk from every tile onto one of `goals`, by brute-force
    /// relaxation: a tile the walker can stand on is its step onto a
    /// neighbour (paying for the neighbour) plus the neighbour's distance.
    fn brute_flow(&self, goals: &[Pos]) -> BTreeMap<Pos, u32> {
        let mut best: BTreeMap<Pos, u32> = goals
            .iter()
            .filter(|&&g| self.in_bounds(g))
            .map(|&g| (g, 0))
            .collect();
        loop {
            let mut changed = false;
            for pos in self.positions() {
                if self.cost(pos).is_none() {
                    continue;
                }
                for onto in self.neighbors(pos) {
                    let (Some(&rest), Some(step)) = (best.get(&onto), self.cost(onto)) else {
                        continue;
                    };
                    let total = rest + step;
                    if best.get(&pos).is_none_or(|&old| total < old) {
                        best.insert(pos, total);
                        changed = true;
                    }
                }
            }
            if !changed {
                return best;
            }
        }
    }
}

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

/// Draws `reach` over `rows`: each reached tile's cost (base 36), `.` for
/// the others.
fn picture(costs: &Costs, reach: &Reach) -> Vec<String> {
    (0..i32::from(costs.height))
        .map(|y| {
            (0..i32::from(costs.width))
                .map(|x| match reach.cost(p(x, y)) {
                    Some(c) => char::from_digit(c, 36).unwrap_or('?'),
                    None => '.',
                })
                .collect()
        })
        .collect()
}

/// Draws a flow field the same way.
fn flow_picture(costs: &Costs, flow: &Grid<Option<u32>>) -> Vec<String> {
    (0..i32::from(costs.height))
        .map(|y| {
            (0..i32::from(costs.width))
                .map(|x| match flow.get(p(x, y)).copied().flatten() {
                    Some(c) => char::from_digit(c, 36).unwrap_or('?'),
                    None => '.',
                })
                .collect()
        })
        .collect()
}

// ---------------------------------------------------------------- reachable

#[test]
fn costs_add_up_within_the_budget() {
    let costs = Costs::parse(&["1#11", "1211", "1111"]);
    let reach = costs.reach(p(0, 0), 4);
    assert_eq!(picture(&costs, &reach), ["0...", "134.", "234."]);
    assert_eq!(reach.origin(), p(0, 0));
    assert_eq!(reach.budget(), 4);
    assert_eq!(reach.cost(p(2, 1)), Some(4));
    assert!(reach.contains(p(2, 1)));
    assert!(!reach.contains(p(3, 1)));
    assert!(!reach.contains(p(1, 0)), "a tile that can't be entered");
    assert_eq!(reach.cost(p(-1, 0)), None);
    assert_eq!(reach.cost(p(4, 0)), None);
    assert!(!reach.contains(p(0, 3)));
}

#[test]
fn exactly_the_budget_is_in_reach_and_one_more_is_not() {
    let costs = Costs::parse(&["11111"]);
    assert_eq!(picture(&costs, &costs.reach(p(0, 0), 3)), ["0123."]);
    assert_eq!(picture(&costs, &costs.reach(p(0, 0), 4)), ["01234"]);
    assert_eq!(picture(&costs, &costs.reach(p(2, 0), 0)), ["..0.."]);
}

#[test]
fn the_start_costs_nothing_even_when_it_cant_be_entered() {
    let costs = Costs::parse(&["#9", "11"]);
    let reach = costs.reach(p(0, 0), 9);
    assert_eq!(picture(&costs, &reach), ["09", "12"]);
    assert_eq!(reach.path_to(p(0, 0)), Some(vec![p(0, 0)]));
}

#[test]
fn a_start_outside_the_grid_reaches_nothing() {
    let costs = Costs::parse(&["11", "11"]);
    for start in [p(-1, 0), p(2, 0), p(0, 2), p(0, -1)] {
        let reach = costs.reach(start, 9);
        assert_eq!(reach.origin(), start);
        assert!(reach.tiles().is_empty(), "{start:?}");
        assert_eq!(reach.path_to(p(0, 0)), None);
        assert_eq!(reach.path_to(start), None);
    }
}

#[test]
fn tiles_are_every_tile_in_reach() {
    let costs = Costs::parse(&["11#", "1#1", "111"]);
    let reach = costs.reach(p(0, 0), 4);
    let tiles: Vec<Pos> = reach.tiles().iter().collect();
    assert_eq!(
        tiles,
        [p(0, 0), p(1, 0), p(0, 1), p(0, 2), p(1, 2), p(2, 2)]
    );
    assert_eq!(
        (reach.tiles().width(), reach.tiles().height()),
        (3, 3),
        "a set of the grid's tiles"
    );
}

#[test]
fn ties_prefer_the_neighbour_with_the_smaller_y_x() {
    let costs = Costs::parse(&["111", "111", "111"]);
    let reach = costs.reach(p(0, 0), 4);
    // (1, 0) is expanded before (0, 1), so paths go right first.
    assert_eq!(
        reach.path_to(p(1, 1)),
        Some(vec![p(0, 0), p(1, 0), p(1, 1)])
    );
    assert_eq!(
        reach.path_to(p(2, 2)),
        Some(vec![p(0, 0), p(1, 0), p(2, 0), p(2, 1), p(2, 2)])
    );
    assert_eq!(
        reach.path_to(p(0, 2)),
        Some(vec![p(0, 0), p(0, 1), p(0, 2)])
    );
    // From the bottom right, up comes before left.
    let reach = costs.reach(p(2, 2), 4);
    assert_eq!(
        reach.path_to(p(1, 1)),
        Some(vec![p(2, 2), p(2, 1), p(1, 1)])
    );
}

#[test]
fn paths_take_the_cheapest_way_round() {
    let costs = Costs::parse(&["19", "11"]);
    let reach = costs.reach(p(0, 0), 9);
    assert_eq!(reach.cost(p(1, 0)), Some(9));
    assert_eq!(reach.path_to(p(1, 0)), Some(vec![p(0, 0), p(1, 0)]));
    let costs = Costs::parse(&["191", "111"]);
    let reach = costs.reach(p(0, 0), 9);
    assert_eq!(reach.cost(p(2, 0)), Some(4));
    assert_eq!(
        reach.path_to(p(2, 0)),
        Some(vec![p(0, 0), p(0, 1), p(1, 1), p(2, 1), p(2, 0)])
    );
    assert_eq!(reach.path_to(p(9, 9)), None);
}

#[test]
fn free_steps_are_steps() {
    let costs = Costs::parse(&["000", "#00"]);
    let reach = costs.reach(p(2, 0), 0);
    assert_eq!(picture(&costs, &reach), ["000", ".00"]);
    assert_eq!(
        reach.path_to(p(0, 0)),
        Some(vec![p(2, 0), p(1, 0), p(0, 0)])
    );
}

#[test]
fn a_cost_past_u32_max_is_no_step() {
    let costs = Costs {
        width: 3,
        height: 1,
        cells: vec![Some(1), Some(u32::MAX), Some(1)],
    };
    let reach = costs.reach(p(0, 0), u32::MAX);
    assert_eq!(reach.cost(p(1, 0)), Some(u32::MAX));
    assert_eq!(reach.cost(p(2, 0)), None);
    let flow = costs.flow(&[p(2, 0)]);
    assert_eq!(flow.get(p(1, 0)), Some(&Some(1)));
    assert_eq!(flow.get(p(0, 0)), Some(&None));
}

// --------------------------------------------------------------- flow_field

#[test]
fn flow_is_the_cheapest_walk_onto_the_nearest_goal() {
    let costs = Costs::parse(&["1111", "1#21", "1111"]);
    let flow = costs.flow(&[p(3, 0)]);
    assert_eq!(flow_picture(&costs, &flow), ["3210", "4.21", "5432"]);
    let both = costs.flow(&[p(3, 0), p(0, 2)]);
    assert_eq!(flow_picture(&costs, &both), ["2210", "1.21", "0122"]);
}

#[test]
fn walking_onto_a_goal_pays_for_the_goal() {
    let costs = Costs::parse(&["151"]);
    let flow = costs.flow(&[p(1, 0)]);
    assert_eq!(flow_picture(&costs, &flow), ["505"]);
}

#[test]
fn a_goal_that_cant_be_entered_is_never_walked_onto() {
    let costs = Costs::parse(&["1#1", "111"]);
    let flow = costs.flow(&[p(1, 0)]);
    assert_eq!(flow_picture(&costs, &flow), [".0.", "..."]);
    // A tile that can't be entered has no distance of its own.
    let flow = costs.flow(&[p(2, 0)]);
    assert_eq!(flow_picture(&costs, &flow), ["4.0", "321"]);
}

#[test]
fn goals_outside_the_grid_are_left_out() {
    let costs = Costs::parse(&["11"]);
    let flow = costs.flow(&[p(-1, 0), p(2, 0), p(0, 5)]);
    assert_eq!(flow_picture(&costs, &flow), [".."]);
    assert_eq!(flow_picture(&costs, &costs.flow(&[])), [".."]);
    assert_eq!((flow.width(), flow.height()), (2, 1));
}

#[test]
fn a_cheaper_way_found_later_wins() {
    // From the left tile, walking onto the 9 first looks closer to the goal
    // than going round, which then turns out cheaper.
    let costs = Costs::parse(&["191", "111"]);
    let flow = costs.flow(&[p(2, 0)]);
    assert_eq!(flow_picture(&costs, &flow), ["410", "321"]);
}

// ---------------------------------------------------------------- properties

/// A random grid, plain-heavy, with tiles that can't be entered; steps cost
/// `min..=4`.
fn arb_costs(min: u32) -> impl Strategy<Value = Costs> {
    (1u16..=8, 1u16..=8).prop_flat_map(move |(width, height)| {
        let cell = prop_oneof![
            4 => Just(Some(1)),
            4 => (min..=4).prop_map(Some),
            1 => Just(None),
        ];
        let n = usize::from(width) * usize::from(height);
        prop::collection::vec(cell, n).prop_map(move |cells| Costs {
            width,
            height,
            cells,
        })
    })
}

/// A tile in or just around an 8 × 8 grid.
fn arb_pos() -> impl Strategy<Value = Pos> {
    (-1i32..=8, -1i32..=8).prop_map(|(x, y)| Pos::new(x, y))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn costs_equal_brute_force(costs in arb_costs(0), start in arb_pos(), budget in 0u32..16) {
        let reach = costs.reach(start, budget);
        let brute = costs.brute_reach(start, budget);
        for pos in costs.positions() {
            prop_assert_eq!(reach.cost(pos), brute.get(&pos).copied(), "{:?}", pos);
        }
        let tiles: BTreeSet<Pos> = reach.tiles().iter().collect();
        prop_assert_eq!(tiles, brute.keys().copied().collect::<BTreeSet<Pos>>());
    }

    #[test]
    fn paths_are_contiguous_and_add_up_to_the_cost(
        costs in arb_costs(0),
        start in arb_pos(),
        budget in 0u32..16,
    ) {
        let reach = costs.reach(start, budget);
        for pos in costs.positions() {
            let Some(path) = reach.path_to(pos) else {
                prop_assert!(!reach.contains(pos));
                continue;
            };
            prop_assert_eq!(path.first(), Some(&start));
            prop_assert_eq!(path.last(), Some(&pos));
            let mut total = 0;
            for pair in path.windows(2) {
                prop_assert_eq!(Pos::manhattan(pair[0], pair[1]), 1);
                total += costs.cost(pair[1]).unwrap();
            }
            prop_assert_eq!(Some(total), reach.cost(pos));
        }
    }

    /// The documented tie-break, with every step costing at least 1: a
    /// tile's predecessor is, of its neighbours on a cheapest path, the one
    /// with the smallest `(cost, y, x)`.
    #[test]
    fn ties_break_as_documented(costs in arb_costs(1), start in arb_pos(), budget in 0u32..16) {
        let reach = costs.reach(start, budget);
        let brute = costs.brute_reach(start, budget);
        for (&pos, &cost) in &brute {
            if pos == start {
                continue;
            }
            let expected = costs
                .neighbors(pos)
                .into_iter()
                .filter_map(|n| Some((*brute.get(&n)?, n.y, n.x)))
                .filter(|&(c, _, _)| c + costs.cost(pos).unwrap() == cost)
                .min()
                .map(|(_, y, x)| p(x, y));
            let path = reach.path_to(pos).unwrap();
            prop_assert_eq!(path.iter().rev().nth(1).copied(), expected, "{:?}", pos);
        }
    }

    #[test]
    fn the_same_walk_gives_the_same_reach(costs in arb_costs(0), start in arb_pos(), budget in 0u32..16) {
        prop_assert_eq!(costs.reach(start, budget), costs.reach(start, budget));
    }

    #[test]
    fn flow_distances_equal_brute_force(
        costs in arb_costs(0),
        goals in prop::collection::vec(arb_pos(), 0..4),
    ) {
        let flow = costs.flow(&goals);
        let brute = costs.brute_flow(&goals);
        prop_assert_eq!((flow.width(), flow.height()), (costs.width, costs.height));
        for pos in costs.positions() {
            prop_assert_eq!(flow.get(pos).copied().flatten(), brute.get(&pos).copied(), "{:?}", pos);
        }
    }

    /// A walker's distance to a goal is the cost of reaching the goal from
    /// its tile with [`reachable`].
    #[test]
    fn flow_matches_reachable_from_the_tile(costs in arb_costs(0), goal in arb_pos()) {
        let flow = costs.flow(&[goal]);
        for pos in costs.positions().filter(|&q| q != goal && costs.cost(q).is_some()) {
            let reach = costs.reach(pos, u32::MAX);
            prop_assert_eq!(flow.get(pos).copied().flatten(), reach.cost(goal), "{:?}", pos);
        }
    }
}
