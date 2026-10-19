//! A unit's path as the cursor steers it: the player draws a route by
//! moving the cursor, and the path follows. A grid skin draws it
//! ([`MapScene::path`](super::MapScene::path)).

use crate::prelude::*;
use shuyi_core::Pos;

/// The path after the cursor moves to `to`. `path` runs from the unit's
/// tile (never empty); `valid` says whether a path is legal (the game's
/// rules: the unit's move points, who stands where), and `cheapest` gives
/// the cheapest path to a tile, or `None` if the unit can't reach it
/// (`shuyi_core::Reach::path_to`).
///
/// - `to` already on the path: the path is cut back to it.
/// - `to` next to the path's end and the longer path is legal: `to` is
///   appended, so the player can steer a route.
/// - Otherwise the cheapest path to `to`, or the path unchanged if `to`
///   can't be reached.
pub fn steer(
    path: &[Pos],
    to: Pos,
    cheapest: impl FnOnce(Pos) -> Option<Vec<Pos>>,
    valid: impl Fn(&[Pos]) -> bool,
) -> Vec<Pos> {
    if let Some(i) = path.iter().position(|&p| p == to) {
        return path[..=i].to_vec();
    }
    if path.last().is_some_and(|&end| Pos::manhattan(end, to) == 1) {
        let mut longer = path.to_vec();
        longer.push(to);
        if valid(&longer) {
            return longer;
        }
    }
    cheapest(to).unwrap_or_else(|| path.to_vec())
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use shuyi_core::{Reach, reachable};

    use super::*;

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    /// A 10 × 8 field where each step costs 1 but a wall at column 4,
    /// rows 0 to 5, and the unit at (2, 3) with 5 move points.
    fn field() -> Reach {
        reachable(10, 8, p(2, 3), 5, |pos| {
            (pos.x != 4 || pos.y > 5).then_some(1)
        })
    }

    /// Whether `path` is a walk the unit can make: from its tile, a step
    /// at a time, within reach, within its move points.
    fn legal(reach: &Reach, path: &[Pos]) -> bool {
        let steps = path.windows(2).all(|w| Pos::manhattan(w[0], w[1]) == 1);
        let budget = usize::try_from(reach.budget()).unwrap_or(usize::MAX);
        path.first() == Some(&reach.origin())
            && steps
            && path.iter().all(|&q| reach.contains(q))
            && path.len() <= budget + 1
    }

    /// `path` steered to each of `cursor` in turn.
    fn steered(reach: &Reach, cursor: &[Pos]) -> Vec<Pos> {
        let mut path = vec![reach.origin()];
        for &to in cursor {
            path = steer(&path, to, |to| reach.path_to(to), |q| legal(reach, q));
        }
        path
    }

    #[test]
    fn steering_appends_cuts_back_and_falls_back_to_the_cheapest_path() {
        let reach = field();
        let unit = p(2, 3);
        // Down, right, up, up: a detour the cheapest path wouldn't take.
        let walk = [p(2, 4), p(3, 4), p(3, 3), p(3, 2)];
        assert_eq!(
            steered(&reach, &walk),
            [unit, p(2, 4), p(3, 4), p(3, 3), p(3, 2)]
        );
        // Back onto the path: cut back to that tile.
        let mut back = walk.to_vec();
        back.push(p(3, 4));
        assert_eq!(steered(&reach, &back), [unit, p(2, 4), p(3, 4)]);
        // Back onto the unit: just its tile.
        back.push(p(2, 4));
        back.push(unit);
        assert_eq!(steered(&reach, &back), [unit]);
        // Too long to go on (5 move points): the cheapest path instead.
        let far = [p(2, 2), p(2, 1), p(1, 1), p(0, 1), p(0, 2), p(0, 3)];
        let cheapest = reach.path_to(p(0, 3)).unwrap();
        assert_eq!(cheapest.len(), 3);
        assert_eq!(steered(&reach, &far), cheapest);
        // Out of reach (behind the wall): the path stays as it was.
        assert_eq!(steered(&reach, &[p(3, 3), p(5, 3)]), [unit, p(3, 3)]);
        // A tile next to the path's end but not legal to add, though
        // reachable another way: the cheapest path to it.
        let blocked = steer(&[unit, p(3, 3)], p(3, 2), |to| reach.path_to(to), |_| false);
        assert_eq!(blocked, reach.path_to(p(3, 2)).unwrap());
        // Not next to the end: never appended, even when legal.
        let jump = steer(&[unit], p(2, 5), |_| None, |_| true);
        assert_eq!(jump, [unit]);
    }

    proptest! {
        /// Whatever the cursor does, the path stays legal, and it ends on
        /// the cursor whenever the cursor is on a tile the unit can reach.
        #[test]
        fn steered_paths_are_legal_and_follow_the_cursor(
            moves in prop::collection::vec(0..4usize, 0..40),
        ) {
            let reach = field();
            let mut cursor = reach.origin();
            let mut path = vec![cursor];
            for m in moves {
                let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)][m];
                cursor = p((cursor.x + dx).clamp(0, 9), (cursor.y + dy).clamp(0, 7));
                path = steer(&path, cursor, |to| reach.path_to(to), |q| legal(&reach, q));
                prop_assert!(legal(&reach, &path), "{:?}", path);
                if reach.contains(cursor) {
                    prop_assert_eq!(path.last(), Some(&cursor));
                }
            }
        }
    }
}
