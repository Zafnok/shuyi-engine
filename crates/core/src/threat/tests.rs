//! Tests of the threat areas, the danger at a tile and the walking
//! distance to goals.

use alloc::collections::BTreeSet;
use alloc::vec;
use core::cell::Cell;

use proptest::prelude::*;

use super::*;

fn p(x: i32, y: i32) -> Pos {
    Pos::new(x, y)
}

/// A set of `tiles` on a `width × height` grid.
fn set(width: u16, height: u16, tiles: &[(i32, i32)]) -> TileSet {
    let mut set = TileSet::new(width, height);
    for &(x, y) in tiles {
        set.insert(p(x, y));
    }
    set
}

/// The members of `set`, sorted.
fn members(set: &TileSet) -> BTreeSet<Pos> {
    set.iter().collect()
}

/// The brute force: every tile of the grid whose distance to some stop is
/// in some range, plus the stops when there is a range.
fn brute_area(stops: &TileSet, ranges: &Ranges) -> BTreeSet<Pos> {
    let mut out = BTreeSet::new();
    if ranges.is_empty() {
        return out;
    }
    for y in 0..i32::from(stops.height()) {
        for x in 0..i32::from(stops.width()) {
            let tile = p(x, y);
            let struck = stops.iter().any(|stop| {
                let d = Pos::manhattan(stop, tile);
                d == 0 || ranges.iter().any(|&(min, max)| min <= d && d <= max)
            });
            if struck {
                out.insert(tile);
            }
        }
    }
    out
}

/// What entering a tile costs, from rows of text: a digit is the tile's
/// cost, `#` can't be entered.
fn costs(rows: &[&'static str]) -> (u16, u16, impl Fn(Pos) -> Option<u32> + Clone) {
    let width = u16::try_from(rows[0].len()).unwrap();
    let height = u16::try_from(rows.len()).unwrap();
    let rows = rows.to_vec();
    let cost = move |pos: Pos| {
        let row = rows[usize::try_from(pos.y).unwrap()];
        let ch = row.as_bytes()[usize::try_from(pos.x).unwrap()];
        char::from(ch).to_digit(10)
    };
    (width, height, cost)
}

// ---------------------------------------------------------------- areas

#[test]
fn a_range_of_one_is_the_stop_and_its_four_neighbours() {
    let area = strike_area(&set(5, 5, &[(2, 2)]), &[(1, 1)]);
    let want = set(5, 5, &[(2, 2), (3, 2), (1, 2), (2, 1), (2, 3)]);
    assert_eq!(members(&area), members(&want));
}

#[test]
fn a_range_of_two_is_the_stop_and_the_ring_of_eight() {
    let area = strike_area(&set(5, 5, &[(2, 2)]), &[(2, 2)]);
    let ring = [
        (2, 2),
        (0, 2),
        (4, 2),
        (2, 0),
        (2, 4),
        (1, 1),
        (3, 1),
        (1, 3),
        (3, 3),
    ];
    assert_eq!(members(&area), members(&set(5, 5, &ring)));
}

#[test]
fn an_area_is_clipped_at_a_corner() {
    let stops = set(4, 4, &[(0, 0)]);
    let one = strike_area(&stops, &[(1, 1)]);
    assert_eq!(
        members(&one),
        members(&set(4, 4, &[(0, 0), (1, 0), (0, 1)]))
    );
    let two = strike_area(&stops, &[(2, 2)]);
    let want = set(4, 4, &[(0, 0), (2, 0), (1, 1), (0, 2)]);
    assert_eq!(members(&two), members(&want));
}

#[test]
fn no_ranges_strike_nothing_and_an_empty_range_strikes_the_stops() {
    let stops = set(3, 3, &[(1, 1)]);
    assert!(strike_area(&stops, &[]).is_empty());
    assert_eq!(members(&strike_area(&stops, &[(3, 1)])), members(&stops));
}

#[test]
fn a_huge_range_covers_the_grid() {
    let area = strike_area(&set(3, 2, &[(0, 0)]), &[(1, u32::MAX)]);
    assert_eq!(area.len(), 6);
}

#[test]
fn a_piece_that_never_moves_strikes_from_where_it_stands() {
    let from = strike_area_from(5, 5, p(2, 2), &[(1, 2)]);
    let one_stop = strike_area(&set(5, 5, &[(2, 2)]), &[(1, 2)]);
    assert_eq!(from, one_stop);
    assert_eq!(from.len(), 13);
    assert!(strike_area_from(5, 5, p(-1, 2), &[(1, 2)]).is_empty());
}

// ---------------------------------------------------------------- threats

fn two_threats() -> Threats<char> {
    Threats::new(vec![
        Threat::new('a', strike_area_from(5, 5, p(0, 0), &[(1, 2)])),
        Threat::new('b', strike_area_from(5, 5, p(2, 0), &[(1, 1)])),
    ])
}

#[test]
fn reaches_names_the_threats_on_a_tile_in_order() {
    let threats = two_threats();
    assert_eq!(threats.reaches(p(1, 0)).collect::<Vec<_>>(), [&'a', &'b']);
    assert_eq!(threats.reaches(p(3, 0)).collect::<Vec<_>>(), [&'b']);
    assert_eq!(threats.reaches(p(4, 4)).count(), 0);
    assert!(threats.safe(p(4, 4)));
    assert!(!threats.safe(p(0, 2)));
}

#[test]
fn no_threats_have_an_empty_zone() {
    let threats: Threats<u8> = Threats::default();
    assert!(threats.is_empty());
    assert!(threats.zone().is_empty());
    assert!(threats.safe(p(0, 0)));
}

#[test]
fn threats_collect_and_push_in_order() {
    let mut threats: Threats<u8> = (0..2).map(|n| Threat::new(n, TileSet::new(2, 2))).collect();
    threats.push(Threat::new(9, TileSet::new(2, 2)));
    let pieces: Vec<u8> = threats.as_slice().iter().map(|t| t.piece).collect();
    assert_eq!(pieces, [0, 1, 9]);
    assert_eq!(threats.len(), 3);
    assert!(!threats.is_empty());
}

#[test]
fn danger_adds_only_the_threats_the_forecast_answers() {
    let threats = two_threats();
    // Both reach (1, 0); `b` wouldn't strike.
    let danger = threats.danger(p(1, 0), |&piece| {
        (piece == 'a').then_some(Strike::new(450, 6, 120))
    });
    let want = Danger {
        expected: 450,
        worst: 6,
        struck_back: 120,
        threats: 1,
    };
    assert_eq!(danger, want);
}

#[test]
fn danger_adds_every_threat_that_reaches() {
    let threats = two_threats();
    let mut asked = Vec::new();
    let danger = threats.danger(p(1, 0), |&piece| {
        asked.push(piece);
        Some(Strike::new(100, 2, 50))
    });
    assert_eq!(asked, ['a', 'b']);
    assert_eq!((danger.expected, danger.worst), (200, 4));
    assert_eq!((danger.struck_back, danger.threats), (100, 2));
    // A tile no threat reaches is never forecast.
    let calm = threats.danger(p(4, 4), |_| unreachable!("no threat reaches"));
    assert_eq!(calm, Danger::default());
}

#[test]
fn danger_saturates() {
    let threats = two_threats();
    let danger = threats.danger(p(1, 0), |_| Some(Strike::new(i64::MAX, 1, 0)));
    assert_eq!(danger.expected, i64::MAX);
}

// ---------------------------------------------------------------- goals

#[test]
fn a_goal_is_zero_from_itself() {
    let (w, h, cost) = costs(&["111", "111"]);
    let mut goals = GoalDistance::new(w, h, vec![p(2, 1)]);
    assert_eq!(goals.distance(0u8, p(2, 1), &cost), Some(0));
    assert_eq!(goals.distance(0u8, p(0, 0), &cost), Some(2));
    assert_eq!(goals.goals(), [p(2, 1)]);
}

#[test]
fn the_distance_walks_round_a_wall() {
    // The goal is 2 below; the walk goes round the wall, the last step onto
    // the goal free.
    let (w, h, cost) = costs(&["11111", "1###1", "11111"]);
    let mut goals = GoalDistance::new(w, h, vec![p(2, 2)]);
    let walk = goals.distance(0u8, p(2, 0), &cost);
    assert_eq!(walk, Some(5));
    assert_ne!(walk, Some(Pos::manhattan(p(2, 0), p(2, 2))));
}

#[test]
fn a_goal_that_cannot_be_entered_is_still_walked_onto() {
    // A piece stands on the goal: its tile is closed to the walk's cost,
    // and free to it here.
    let (w, h, cost) = costs(&["11#"]);
    let mut goals = GoalDistance::new(w, h, vec![p(2, 0)]);
    assert_eq!(goals.distance(0u8, p(1, 0), &cost), Some(0));
    assert_eq!(goals.distance(0u8, p(0, 0), &cost), Some(1));
}

#[test]
fn nothing_crosses_a_river() {
    let (w, h, cost) = costs(&["11#11", "11#11", "11#11"]);
    let mut goals = GoalDistance::new(w, h, vec![p(4, 1)]);
    assert_eq!(goals.distance(0u8, p(0, 1), &cost), None);
    assert_eq!(goals.distance(0u8, p(3, 1), &cost), Some(0));
}

#[test]
fn no_goals_have_no_distance() {
    let (w, h, cost) = costs(&["11"]);
    let mut goals = GoalDistance::<u8>::new(w, h, Vec::new());
    assert_eq!(goals.distance(0, p(0, 0), cost), None);
}

#[test]
fn one_field_is_built_per_kind() {
    let (w, h, cost) = costs(&["111", "1#1", "111"]);
    let asked = Cell::new(0u32);
    let counted = |pos: Pos| {
        asked.set(asked.get() + 1);
        cost(pos)
    };
    let mut goals = GoalDistance::new(w, h, vec![p(0, 0)]);
    assert_eq!(goals.distance('w', p(2, 2), counted), Some(3));
    let first = asked.get();
    assert!(first > 0);
    assert_eq!(goals.distance('w', p(2, 0), counted), Some(1));
    assert_eq!(asked.get(), first, "the field of `w` is kept");
    // A kind that flies costs 1 everywhere, wall included.
    let flying = |_: Pos| {
        asked.set(asked.get() + 1);
        Some(1)
    };
    assert_eq!(goals.distance('f', p(1, 1), flying), Some(1));
    assert!(asked.get() > first, "`f` builds its own field");
    let both = asked.get();
    assert_eq!(goals.distance('w', p(1, 1), counted), None);
    assert_eq!(goals.distance('f', p(2, 2), flying), Some(3));
    assert_eq!(asked.get(), both);
}

// ---------------------------------------------------------------- properties

/// A random grid of up to 8 × 8 with some stop tiles on it.
fn arb_stops() -> impl Strategy<Value = TileSet> {
    (1u16..=8, 1u16..=8).prop_flat_map(|(width, height)| {
        let tile = (0..i32::from(width), 0..i32::from(height));
        prop::collection::vec(tile, 0..6).prop_map(move |tiles| set(width, height, &tiles))
    })
}

/// Up to three ranges, some empty, some far past the grid.
fn arb_ranges() -> impl Strategy<Value = Vec<(u32, u32)>> {
    let range = prop_oneof![
        4 => (0u32..12, 0u32..12),
        1 => (0u32..4).prop_map(|min| (min, u32::MAX)),
    ];
    prop::collection::vec(range, 0..=3)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    #[test]
    fn strike_area_equals_brute_force(stops in arb_stops(), ranges in arb_ranges()) {
        let area = strike_area(&stops, &ranges);
        prop_assert_eq!(members(&area), brute_area(&stops, &ranges));
        prop_assert_eq!((area.width(), area.height()), (stops.width(), stops.height()));
    }

    #[test]
    fn safe_is_false_exactly_on_the_zone(
        areas in prop::collection::vec((arb_stops(), arb_ranges()), 0..4),
    ) {
        // Every threat on the first one's grid, as a side's threats are.
        let (width, height) = areas.first().map_or((1, 1), |(s, _)| (s.width(), s.height()));
        let threats: Threats<usize> = areas
            .iter()
            .enumerate()
            .map(|(n, (stops, ranges))| {
                let mut on_grid = TileSet::new(width, height);
                on_grid.union_with(stops);
                Threat::new(n, strike_area(&on_grid, ranges))
            })
            .collect();
        let zone = threats.zone();
        for y in -1..=i32::from(height) {
            for x in -1..=i32::from(width) {
                prop_assert_eq!(threats.safe(p(x, y)), !zone.contains(p(x, y)), "{:?}", p(x, y));
            }
        }
    }
}
