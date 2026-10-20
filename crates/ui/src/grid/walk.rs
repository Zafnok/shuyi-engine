//! How units move on a grid map (game ticket 0440): a unit that can still
//! act steps on the spot, and a moving unit glides from tile to tile along
//! its path, its legs going, turned the way it walks. Every timing is
//! here.
//!
//! All of it is a look (game ADR-0038): the rules, the camera and the
//! sounds go by the tile a walk is on, which changes at each tile's edge.
//! How fast a walk goes is the look's too: each skin's setting
//! ([`GridSkin::walk`](super::GridSkin::walk)).

use shuyi_core::Pos;

#[cfg(not(feature = "std"))]
use crate::float::Float;

/// Which way a unit is turned. A look only: never a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Facing {
    /// Towards the camera: every unit that isn't walking.
    #[default]
    Down,
    /// To the left.
    Left,
    /// To the right.
    Right,
    /// Away from the camera.
    Up,
}

impl Facing {
    /// Its name in text for tests: `down`, `left`, `right` or `up`.
    pub const fn name(self) -> &'static str {
        match self {
            Facing::Down => "down",
            Facing::Left => "left",
            Facing::Right => "right",
            Facing::Up => "up",
        }
    }
}

/// The walking frame of a unit standing with its feet together.
pub const STANDING_FRAME: u8 = 1;

/// How long each frame of a unit stepping on the spot shows, in
/// milliseconds. *Tunable.*
pub const IDLE_STEP_MS: u64 = 250;

/// The frames of a unit stepping on the spot, one per [`IDLE_STEP_MS`]:
/// standing, one foot, standing, the other foot. It starts on the standing
/// frame.
pub const IDLE_FRAMES: [u8; 4] = [STANDING_FRAME, 2, STANDING_FRAME, 0];

/// How long each frame of a walking unit shows, in seconds. *Tunable.*
pub const WALK_FRAME_S: f32 = 0.1;

/// The frames of a walking unit, one per [`WALK_FRAME_S`]: one foot,
/// standing, the other foot, standing. It starts with a foot forward.
pub const WALK_FRAMES: [u8; 4] = [0, STANDING_FRAME, 2, STANDING_FRAME];

/// The frame of a unit stepping on the spot at `clock_ms` on the screen's
/// animation clock. Every such unit steps together.
pub fn idle_frame(clock_ms: u64) -> u8 {
    let step = usize::try_from((clock_ms / IDLE_STEP_MS) % 4).unwrap_or(0);
    IDLE_FRAMES.get(step).copied().unwrap_or(STANDING_FRAME)
}

/// How many steps of a path of `len` tiles a walk has taken once it has
/// gone `tiles` tiles: a step is taken as its tile is reached (at most all
/// of them; none for a negative or NaN distance).
pub fn steps(len: usize, tiles: f32) -> usize {
    let all = len.saturating_sub(1);
    (1..=u16::try_from(all).unwrap_or(u16::MAX))
        .take_while(|&k| f32::from(k) <= tiles)
        .count()
}

/// How a unit between two tiles of its walk looks.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gait {
    /// The way it walks.
    pub facing: Facing,
    /// Its walking frame.
    pub frame: u8,
    /// How far it is from the tile it left towards the next, in tiles
    /// across and down.
    pub offset: (f32, f32),
}

/// The way a step from `from` to `to` goes: left or right if it goes
/// across at all, else up or down.
pub fn facing(from: Pos, to: Pos) -> Facing {
    if to.x > from.x {
        Facing::Right
    } else if to.x < from.x {
        Facing::Left
    } else if to.y < from.y {
        Facing::Up
    } else {
        Facing::Down
    }
}

/// The look of a unit that has walked `tiles` tiles along `path` (its
/// start first) at `pace` tiles per second, beside the tile [`steps`] puts
/// it on; `None` once it has arrived (or has nowhere to go): it then
/// stands, facing the camera.
pub fn gait(path: &[Pos], tiles: f32, pace: f32) -> Option<Gait> {
    let taken = steps(path.len(), tiles);
    let (from, to) = (*path.get(taken)?, *path.get(taken + 1)?);
    let tiles = if tiles.is_finite() {
        tiles.max(0.0)
    } else {
        0.0
    };
    // `taken` is at most `tiles`, and small: exact in f32.
    let part = (tiles - f32::from(u16::try_from(taken).unwrap_or(u16::MAX))).clamp(0.0, 1.0);
    let toward = |a: i32, b: i32| match b.cmp(&a) {
        core::cmp::Ordering::Less => -part,
        core::cmp::Ordering::Equal => 0.0,
        core::cmp::Ordering::Greater => part,
    };
    // Whole frames shown so far: far fewer than f32 holds exactly.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let shown = (tiles * frames_per_tile(pace)).floor() as usize;
    Some(Gait {
        facing: facing(from, to),
        frame: WALK_FRAMES[shown % WALK_FRAMES.len()],
        offset: (toward(from.x, to.x), toward(from.y, to.y)),
    })
}

/// How many walking frames show while a unit crosses a tile at `pace`
/// tiles per second (none at a pace of zero or one that isn't a number).
fn frames_per_tile(pace: f32) -> f32 {
    let per_tile = 1.0 / (pace * WALK_FRAME_S);
    if per_tile.is_finite() { per_tile } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    use crate::prelude::*;

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    /// A pace of 5 tiles a second: two walking frames a tile.
    const PACE: f32 = 5.0;

    #[test]
    fn a_facing_has_a_name_and_a_unit_stands_on_frame_1() {
        let names = [
            (Facing::Down, "down"),
            (Facing::Left, "left"),
            (Facing::Right, "right"),
            (Facing::Up, "up"),
        ];
        for (facing, name) in names {
            assert_eq!(facing.name(), name);
        }
        assert_eq!((Facing::default(), STANDING_FRAME), (Facing::Down, 1));
    }

    #[test]
    fn a_unit_steps_on_the_spot_every_quarter_second() {
        // Standing, a foot, standing, the other foot, and again.
        let frames: Vec<u8> = (0..9).map(|k| idle_frame(k * 250)).collect();
        assert_eq!(frames, [1, 2, 1, 0, 1, 2, 1, 0, 1]);
        assert_eq!(idle_frame(249), 1);
        assert_eq!(idle_frame(250), 2);
        assert_eq!(idle_frame(499), 2);
        assert_eq!(idle_frame(999), 0);
        assert!(idle_frame(u64::MAX) <= 2);
        assert_eq!(IDLE_STEP_MS, 250);
    }

    #[test]
    fn a_step_is_taken_as_its_tile_is_reached() {
        assert_eq!(steps(4, 0.0), 0);
        assert_eq!(steps(4, 0.99), 0);
        assert_eq!(steps(4, 1.0), 1);
        assert_eq!(steps(4, 2.5), 2);
        assert_eq!(steps(4, 3.0), 3);
        assert_eq!(steps(4, 99.0), 3);
        assert_eq!(steps(4, -1.0), 0);
        assert_eq!(steps(4, f32::NAN), 0);
        assert_eq!(steps(1, 5.0), 0);
        assert_eq!(steps(0, 5.0), 0);
    }

    #[test]
    fn a_walk_changes_frame_every_100_ms_at_any_pace() {
        assert!((WALK_FRAME_S - 0.1).abs() < 1e-6);
        let path = [p(1, 1), p(2, 1), p(3, 1)];
        // Two frames a tile: a foot, standing; the other foot, standing.
        let frame = |tiles| gait(&path, tiles, PACE).map(|g| g.frame);
        assert_eq!(frame(0.0), Some(0));
        assert_eq!(frame(0.25), Some(0));
        assert_eq!(frame(0.5), Some(1));
        assert_eq!(frame(0.75), Some(1));
        assert_eq!(frame(1.0), Some(2));
        assert_eq!(frame(1.5), Some(1));
        assert_eq!(frame(2.0), None);
        // The legs keep their time at any pace: at 10 tiles a second a
        // frame lasts a whole tile; at 2.5, a quarter of one.
        let at = |tiles, pace| gait(&path, tiles, pace).map(|g| g.frame);
        assert_eq!(
            (at(0.9, 10.0), at(1.0, 10.0), at(1.9, 10.0)),
            (Some(0), Some(1), Some(1))
        );
        assert_eq!(
            (at(0.2, 2.5), at(0.25, 2.5), at(0.5, 2.5)),
            (Some(0), Some(1), Some(2))
        );
        // A pace that isn't a positive number: the first frame.
        for bad in [0.0, -3.0, f32::NAN, f32::INFINITY] {
            assert_eq!(at(1.5, bad), Some(0), "{bad}");
        }
        let path: Vec<Pos> = (0..6).map(|x| p(x, 0)).collect();
        assert_eq!(gait(&path, 2.25, PACE).map(|g| g.frame), Some(0));
        assert_eq!(gait(&path, 4.75, PACE).map(|g| g.frame), Some(1));
    }

    #[test]
    fn a_walker_faces_the_way_it_goes_and_glides_to_the_next_tile() {
        // Right, up, left, down.
        let path = [p(1, 1), p(2, 1), p(2, 0), p(1, 0), p(1, 1)];
        let at = |tiles| gait(&path, tiles, PACE).map(|g| (g.facing, g.offset));
        assert_eq!(at(0.0), Some((Facing::Right, (0.0, 0.0))));
        assert_eq!(at(0.25), Some((Facing::Right, (0.25, 0.0))));
        assert_eq!(at(0.75), Some((Facing::Right, (0.75, 0.0))));
        assert_eq!(at(1.0), Some((Facing::Up, (0.0, 0.0))));
        assert_eq!(at(1.5), Some((Facing::Up, (0.0, -0.5))));
        assert_eq!(at(2.5), Some((Facing::Left, (-0.5, 0.0))));
        assert_eq!(at(3.25), Some((Facing::Down, (0.0, 0.25))));
        // Arrived, or nowhere to go: it stands.
        assert_eq!(at(4.0), None);
        assert_eq!(at(99.0), None);
        assert_eq!(gait(&[p(1, 1)], 0.5, PACE), None);
        assert_eq!(gait(&[], 0.5, PACE), None);
        // A bad distance counts as none walked.
        assert_eq!(at(-3.0), at(0.0));
        assert_eq!(at(f32::NAN), at(0.0));
        assert_eq!(at(f32::NEG_INFINITY), at(0.0));
        assert_eq!(at(f32::INFINITY), None);
        // A step that isn't to a neighbour still goes one way, a tile at
        // most.
        assert_eq!(facing(p(0, 0), p(3, -3)), Facing::Right);
        assert_eq!(facing(p(0, 0), p(-3, 3)), Facing::Left);
        assert_eq!(facing(p(0, 0), p(0, 0)), Facing::Down);
        assert_eq!(facing(p(0, 0), p(0, 1)), Facing::Down);
        let far = gait(&[p(0, 0), p(5, -5)], 0.5, PACE).unwrap();
        assert_eq!(far.offset, (0.5, -0.5));
    }

    prop_compose! {
        /// A path of up to 12 tiles, each next to the one before.
        fn any_path()(
            start in (-20..20i32, -20..20i32),
            turns in prop::collection::vec(0u8..4, 0..12),
        ) -> Vec<Pos> {
            let mut path = vec![p(start.0, start.1)];
            let mut at = (start.0, start.1);
            for turn in turns {
                let (dx, dy) = [(1, 0), (-1, 0), (0, 1), (0, -1)][usize::from(turn)];
                at = (at.0 + dx, at.1 + dy);
                path.push(p(at.0, at.1));
            }
            path
        }
    }

    proptest! {
        /// For any path, at any moment of the walk, the unit is shown
        /// within one tile of the path tile it is on, towards the next
        /// one and never past it, with a frame its sheet has; once the
        /// walk is over it is on the path's last tile with no offset.
        #[test]
        fn a_walker_stays_within_a_tile_of_its_path_and_ends_on_its_last_tile(
            path in any_path(),
            tiles in -2.0f32..16.0,
        ) {
            let taken = steps(path.len(), tiles);
            prop_assert!(taken < path.len());
            let on = path[taken];
            if let Some(g) = gait(&path, tiles, PACE) {
                let next = path[taken + 1];
                let (dx, dy) = g.offset;
                prop_assert!((0.0..1.0).contains(&dx.abs().max(dy.abs())), "{:?}", g);
                // Towards the next tile, along one axis.
                prop_assert!(dx == 0.0 || dy == 0.0, "{:?}", g);
                prop_assert!(dx * f32::from(i16::try_from(next.x - on.x).unwrap_or(0)) >= 0.0);
                prop_assert!(dy * f32::from(i16::try_from(next.y - on.y).unwrap_or(0)) >= 0.0);
                prop_assert_eq!(g.facing, facing(on, next));
                prop_assert!(g.frame <= 2);
            } else {
                prop_assert_eq!(Some(&on), path.last());
            }
            // Long enough, and it has arrived.
            let end = f32::from(u8::try_from(path.len()).unwrap_or(u8::MAX));
            prop_assert_eq!(gait(&path, end, PACE), None);
            prop_assert_eq!(path.get(steps(path.len(), end)), path.last());
        }
    }
}
