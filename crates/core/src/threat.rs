//! Threat areas over a square grid: the tiles a piece can strike next
//! turn ([`strike_area`], [`strike_area_from`]), which threats reach a
//! tile and whether it is safe ([`Threats`]), how much danger a tile
//! stands in by the game's own forecast ([`Threats::danger`]), and how far
//! a piece would walk from a tile to the nearest of some goals
//! ([`GoalDistance`]). It is the danger zone a player reads: a bot that
//! plays like a person keeps out of it, a roguelike draws a monster's
//! reach as an overlay, a tower-defense game shows a tower's coverage.
//! On them, [`TacticsOrder`] lists the few commands a planning bot tries
//! first: every piece's best strike, then each piece's step towards the
//! goals and its step to safety, and the end of the turn always kept
//! ([`OrderHooks`], [`Kind`]).
//!
//! # Rules
//!
//! - **Ranges.** A strike range is an inclusive pair of Manhattan
//!   distances, `(min, max)`, as [`Pos::manhattan`] counts them; a pair
//!   with `min > max` is empty. A piece's [`Ranges`] are all of its
//!   weapons, spells or arcs.
//! - **Areas.** A piece strikes every tile of the grid within one of its
//!   ranges of a tile it can stop on, and the stop tiles themselves when
//!   it has any range at all: a piece that can strike threatens where it
//!   can stand. Which tiles it can stop on is the game's: most often the
//!   tiles of its [`Reach`](crate::Reach) ([`reachable`](crate::reachable))
//!   that no other piece holds. A piece that never moves strikes from
//!   where it stands ([`strike_area_from`]).
//! - **Every threat counts.** [`Threats::danger`] adds up the forecast of
//!   every threat that reaches a tile; it never guesses which one the
//!   other side will choose. A bot that knows the danger zone and the
//!   forecasts a player reads, and never whom the other side will strike,
//!   plays fair.
//! - **Cost.** Building an area walks a diamond around each stop tile,
//!   row by row, clipped to the grid: stops × (rows × ranges + the tiles
//!   in range). [`Threats::reaches`] and [`Threats::safe`] look up one bit
//!   per threat and allocate nothing.
//!
//! A piece at `P` on a 5 × 5 grid stops on the `o` tiles (and on its own)
//! and strikes at 1 to 2. Its area is every tile but the three `.`:
//!
//! ```text
//!  stops      area
//!  . . . . .  x x x x .
//!  . o o . .  x x x x x
//!  . o P o .  x x x x x
//!  . . o . .  x x x x x
//!  . . . . .  . x x x .
//! ```
//!
//! ```
//! use shuyi_core::{Pos, TileSet, strike_area};
//!
//! let mut stops = TileSet::new(5, 5);
//! for (x, y) in [(2, 2), (1, 1), (2, 1), (1, 2), (3, 2), (2, 3)] {
//!     stops.insert(Pos::new(x, y));
//! }
//! let area = strike_area(&stops, &[(1, 2)]);
//! assert_eq!(area.len(), 25 - 3);
//! for (x, y) in [(4, 0), (0, 4), (4, 4)] {
//!     assert!(!area.contains(Pos::new(x, y)));
//! }
//! ```

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::grid::{Grid, Pos, TileSet};
use crate::path::flow_field;

mod order;

pub use order::{Kind, OrderHooks, TacticsOrder};

/// A piece's strike ranges: inclusive pairs of Manhattan distances
/// `(min, max)`. A pair with `min > max` is empty.
pub type Ranges = [(u32, u32)];

/// The tiles a piece that can stop on `stops` strikes with `ranges`:
/// every tile of the set's grid within one of `ranges` of a stop tile,
/// and the stop tiles themselves when `ranges` is not empty. Empty
/// `ranges` strike nothing. See the module docs for the cost.
pub fn strike_area(stops: &TileSet, ranges: &Ranges) -> TileSet {
    let mut area = TileSet::new(stops.width(), stops.height());
    if ranges.is_empty() {
        return area;
    }
    for stop in stops.iter() {
        strike_into(&mut area, stop, ranges);
    }
    area
}

/// The tiles a piece that never moves, standing on `at`, strikes with
/// `ranges` on a `width × height` grid: [`strike_area`] with `at` its
/// only stop. A tile outside the grid strikes nothing.
pub fn strike_area_from(width: u16, height: u16, at: Pos, ranges: &Ranges) -> TileSet {
    let mut stops = TileSet::new(width, height);
    stops.insert(at);
    strike_area(&stops, ranges)
}

/// Adds to `area` every tile within one of `ranges` of `from` (a tile of
/// `area`'s grid), and `from` itself: the diamond row by row, each row's
/// two bands clipped to the grid.
fn strike_into(area: &mut TileSet, from: Pos, ranges: &Ranges) {
    let width = i32::from(area.width());
    area.insert(from);
    for y in 0..i32::from(area.height()) {
        let dy = from.y.abs_diff(y);
        for &(min, max) in ranges {
            // The row's tiles at a distance in `min..=max` are those with
            // `near <= |dx| <= far`.
            let Some(far) = max.checked_sub(dy) else {
                continue;
            };
            let far = i32::try_from(far).unwrap_or(i32::MAX);
            let near = i32::try_from(min.saturating_sub(dy)).unwrap_or(i32::MAX);
            let left = from.x.saturating_sub(far).max(0)..=from.x.saturating_sub(near);
            let right = from.x.saturating_add(near)
                ..from.x.saturating_add(far).saturating_add(1).min(width);
            for x in left.chain(right) {
                area.insert(Pos::new(x, y));
            }
        }
    }
}

/// One piece that threatens tiles: the game's id for it and the tiles it
/// can strike next turn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Threat<P> {
    /// The game's id of the piece.
    pub piece: P,
    /// The tiles it can strike ([`strike_area`], [`strike_area_from`], or
    /// the game's own).
    pub area: TileSet,
}

impl<P> Threat<P> {
    /// A threat of `piece` over `area`.
    pub fn new(piece: P, area: TileSet) -> Self {
        Self { piece, area }
    }
}

/// The threats of one side, in the game's order. Answers which reach a
/// tile, whether a tile is safe, the danger zone, and the danger at a
/// tile by the game's forecast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Threats<P> {
    threats: Vec<Threat<P>>,
}

impl<P> Default for Threats<P> {
    fn default() -> Self {
        Self {
            threats: Vec::new(),
        }
    }
}

impl<P> FromIterator<Threat<P>> for Threats<P> {
    fn from_iter<I: IntoIterator<Item = Threat<P>>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

impl<P> Threats<P> {
    /// The threats `threats`, kept in their order.
    pub fn new(threats: Vec<Threat<P>>) -> Self {
        Self { threats }
    }

    /// Adds a threat after the others.
    pub fn push(&mut self, threat: Threat<P>) {
        self.threats.push(threat);
    }

    /// Every threat, in order.
    pub fn as_slice(&self) -> &[Threat<P>] {
        &self.threats
    }

    /// How many threats there are.
    pub fn len(&self) -> usize {
        self.threats.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.threats.is_empty()
    }

    /// The pieces whose area holds `at`, in order. Allocates nothing.
    pub fn reaches(&self, at: Pos) -> impl Iterator<Item = &P> + '_ {
        self.threats
            .iter()
            .filter(move |threat| threat.area.contains(at))
            .map(|threat| &threat.piece)
    }

    /// Whether no threat reaches `at`. Allocates nothing.
    pub fn safe(&self, at: Pos) -> bool {
        self.reaches(at).next().is_none()
    }

    /// Every tile some threat reaches: the danger zone. Its grid is the
    /// first threat's (an empty `0 × 0` set when there is none); a tile
    /// of another threat outside that grid is left out.
    pub fn zone(&self) -> TileSet {
        let mut threats = self.threats.iter();
        let Some(first) = threats.next() else {
            return TileSet::new(0, 0);
        };
        let mut zone = first.area.clone();
        for threat in threats {
            zone.union_with(&threat.area);
        }
        zone
    }

    /// The danger at `at`: the sum of `strike(piece)` over every threat
    /// that reaches `at` and that `strike` answers `Some` for (`None`: the
    /// piece wouldn't strike there, as a healer wouldn't). Every threat
    /// counts, with no guess at which the other side will choose (module
    /// docs). Rounding and caps (at most the target's health, a share of
    /// the striker's) are the game's, on the sums or in `strike`. Sums
    /// saturate.
    pub fn danger(&self, at: Pos, mut strike: impl FnMut(&P) -> Option<Strike>) -> Danger {
        let mut danger = Danger::default();
        for piece in self.reaches(at) {
            let Some(hit) = strike(piece) else {
                continue;
            };
            danger.expected = danger.expected.saturating_add(hit.expected);
            danger.worst = danger.worst.saturating_add(hit.all_hit);
            danger.struck_back = danger.struck_back.saturating_add(hit.struck_back);
            danger.threats = danger.threats.saturating_add(1);
        }
        danger
    }
}

/// The game's forecast of one threat's strike on whoever stands on a tile.
/// Expected values are in hundredths of a point: a hit chance in percent
/// times the damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Strike {
    /// The damage expected, in hundredths.
    pub expected: i64,
    /// The damage if every strike hits, in points.
    pub all_hit: i64,
    /// The damage expected to be struck back at the threat, in hundredths.
    pub struck_back: i64,
}

impl Strike {
    /// A forecast of `expected` damage (hundredths), `all_hit` damage if
    /// every strike hits (points) and `struck_back` damage expected back
    /// (hundredths).
    pub const fn new(expected: i64, all_hit: i64, struck_back: i64) -> Self {
        Self {
            expected,
            all_hit,
            struck_back,
        }
    }
}

/// The danger at one tile: [`Threats::danger`]'s sums of the [`Strike`]s
/// of every threat that reaches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Danger {
    /// The damage expected, in hundredths.
    pub expected: i64,
    /// The damage if every strike of every threat hits, in points.
    pub worst: i64,
    /// The damage expected to be struck back, in hundredths.
    pub struck_back: i64,
    /// How many threats were added up.
    pub threats: u32,
}

/// How far a piece would walk from a tile to the nearest of some goals,
/// with one [`flow_field`] per movement kind `K`, built on first use and
/// kept. Stepping onto a goal's own tile is free and always allowed: a
/// goal is often a tile another piece stands on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalDistance<K> {
    width: u16,
    height: u16,
    goals: Vec<Pos>,
    fields: BTreeMap<K, Grid<Option<u32>>>,
}

impl<K: Ord + Copy> GoalDistance<K> {
    /// The goals `goals` on a `width × height` grid. Goals outside it are
    /// left out of every walk.
    pub fn new(width: u16, height: u16, goals: Vec<Pos>) -> Self {
        Self {
            width,
            height,
            goals,
            fields: BTreeMap::new(),
        }
    }

    /// The goals, as given.
    pub fn goals(&self) -> &[Pos] {
        &self.goals
    }

    /// How far a piece of movement kind `kind` would walk from `from` to
    /// the nearest goal, where entering a tile costs `cost(tile)` (`None`:
    /// it can't be entered) and entering a goal costs 0. `None` with no
    /// goals, or no walk. 0 on a goal.
    ///
    /// The first call for a `kind` builds its field with `cost`; later
    /// calls read it and never call `cost`, so `cost` must answer the same
    /// for every call with one `kind`. Make a new `GoalDistance` when the
    /// costs change.
    pub fn distance(
        &mut self,
        kind: K,
        from: Pos,
        cost: impl Fn(Pos) -> Option<u32>,
    ) -> Option<u32> {
        let (width, height, goals) = (self.width, self.height, &self.goals);
        let field = self.fields.entry(kind).or_insert_with(|| {
            let free = |tile: Pos| {
                if goals.contains(&tile) {
                    Some(0)
                } else {
                    cost(tile)
                }
            };
            flow_field(width, height, goals.iter().copied(), free)
        });
        field.get(from).copied().flatten()
    }
}

#[cfg(test)]
mod tests;
