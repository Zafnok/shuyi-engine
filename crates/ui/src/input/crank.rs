//! A crank (the Playdate's, ticket 1035) as two controller buttons: each
//! notch wound backwards is a press and release of [`Button::CrankBack`],
//! each notch forwards one of [`Button::CrankForward`], so the crank reaches
//! a game through the player's bindings like any button (game ADR-0034). How
//! far a notch is comes from the keymap file's `crank` ([`CrankDef`]).
//!
//! [`Crank`] is pure: the platform hands it how far the crank turned since
//! the last frame and gets whole notches back. A notch is that far wound
//! one way: what is left of one is kept for the next frame, so the same
//! turn gives the same notches however the frames split it, but turning
//! the other way starts a fresh notch (so after a notch back, the next
//! notch forwards is a whole notch away, and a hand resting on the crank
//! never gives a notch). A docked crank gives none and forgets what was
//! wound. A backend whose device says how far a crank turned as a float
//! turns it into whole [`STEPS_PER_TURN`]ths of a turn at its boundary and
//! hands those to [`Crank::turn_steps`]: no float goes further.

use core::cmp::Ordering;

use shuyi_assets::keymap::{Button, CrankDef};

/// Thousandths of a degree in a degree: a turn in degrees counts in whole
/// thousandths, so adding them up is exact.
const MILLI: f32 = 1000.0;

/// How many steps make a whole turn ([`Crank::turn_steps`]).
pub const STEPS_PER_TURN: i32 = 256;

/// What a crank counts in, a 32,000th of a degree: a thousandth of a degree
/// and a step (360 / 256 degrees) are each a whole number of them.
const UNITS_PER_DEGREE: i32 = 32_000;

/// A thousandth of a degree, in those units.
const UNITS_PER_MILLI: i32 = UNITS_PER_DEGREE / 1000;

/// A step, in those units.
const UNITS_PER_STEP: i32 = UNITS_PER_DEGREE * 360 / STEPS_PER_TURN;

/// Most steps one frame counts (a hundred turns, as [`MOST_DEGREES`]).
const MOST_STEPS: i32 = 100 * STEPS_PER_TURN;

/// Most degrees one frame counts (a hundred turns): a turn beyond it is a
/// broken reading, not a player.
const MOST_DEGREES: f32 = 36_000.0;

/// Notches wound in one frame: negative backwards, positive forwards.
pub type Notches = i32;

/// A crank's notches (see the [module docs](self)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crank {
    /// One notch, in [`UNITS_PER_DEGREE`]ths of a degree (at least 1).
    notch: i32,
    /// Wound one way since the last notch or the last turn the other
    /// way, in the same units: always less than a notch.
    wound: i32,
}

impl Crank {
    /// A crank with nothing wound yet, `def.notch_degrees` to the notch.
    pub fn new(def: CrankDef) -> Self {
        Self {
            notch: (i32::from(def.notch_degrees) * UNITS_PER_DEGREE).max(1),
            wound: 0,
        }
    }

    /// The crank turned `degrees` since the last call (forwards is
    /// positive: clockwise on a Playdate). Returns the notches that turn
    /// completes; the rest of a notch counts towards the next call in the
    /// same direction. A reading that isn't a number counts as no turn.
    pub fn turn(&mut self, degrees: f32) -> Notches {
        if !degrees.is_finite() {
            return 0;
        }
        let milli = libm::roundf(degrees.clamp(-MOST_DEGREES, MOST_DEGREES) * MILLI);
        // In range by the clamp above.
        #[allow(clippy::cast_possible_truncation)]
        let milli = milli as i32;
        self.wind(milli * UNITS_PER_MILLI)
    }

    /// The crank turned `steps` [`STEPS_PER_TURN`]ths of a turn since the
    /// last call, as [`turn`](Self::turn) takes degrees.
    pub fn turn_steps(&mut self, steps: i32) -> Notches {
        self.wind(steps.clamp(-MOST_STEPS, MOST_STEPS) * UNITS_PER_STEP)
    }

    /// Winds `units` more, and gives the notches that completes.
    fn wind(&mut self, units: i32) -> Notches {
        // Turned the other way: a fresh notch.
        if units.signum() == -self.wound.signum() {
            self.wound = 0;
        }
        let wound = self.wound + units;
        // Towards zero, so the rest keeps the turn's sign and is less than
        // a notch.
        let notches = wound / self.notch;
        self.wound = wound % self.notch;
        notches
    }

    /// The crank was docked: whatever was wound towards a notch is
    /// forgotten.
    pub fn dock(&mut self) {
        self.wound = 0;
    }

    /// The button `notches` press, and how many times: [`Button::CrankBack`]
    /// for notches backwards, [`Button::CrankForward`] forwards; `None`
    /// for none.
    pub fn presses(notches: Notches) -> Option<(Button, u32)> {
        let button = match notches.cmp(&0) {
            Ordering::Equal => return None,
            Ordering::Less => Button::CrankBack,
            Ordering::Greater => Button::CrankForward,
        };
        Some((button, notches.unsigned_abs()))
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    use crate::prelude::*;

    fn crank(notch_degrees: u16) -> Crank {
        Crank::new(CrankDef { notch_degrees })
    }

    #[test]
    fn a_notch_is_forty_five_degrees_by_default() {
        assert_eq!(CrankDef::default().notch_degrees, 45);
        let mut c = Crank::new(CrankDef::default());
        assert_eq!(c.turn(44.0), 0);
        assert_eq!(c.turn(1.0), 1);
        assert_eq!(c.turn(-45.0), -1);
    }

    /// A step is 360 / 256 degrees, whatever the notch.
    #[test]
    fn steps_of_a_turn_wind_as_degrees_do() {
        let mut c = Crank::new(CrankDef::default());
        assert_eq!(c.turn_steps(31), 0);
        assert_eq!(c.turn_steps(1), 1, "32 steps are 45 degrees");
        assert_eq!(c.turn_steps(-32), -1);
        assert_eq!(c.turn_steps(64), 2, "a quarter turn is two notches");
        let mut c = crank(30);
        assert_eq!(c.turn_steps(21), 0, "21 steps are 29.53 degrees");
        assert_eq!(c.turn_steps(1), 1, "and 22 are 30.94");
        assert_eq!(c.turn(29.0), 0);
        assert_eq!(c.turn_steps(1), 1, "what was left counts: 0.94, 29 and 1.41");
        assert_eq!(c.turn_steps(i32::MAX), 1200, "a hundred turns at most");
    }

    #[test]
    fn what_is_left_of_a_notch_carries_over_one_way() {
        let mut c = crank(30);
        assert_eq!(c.turn(-20.0), 0);
        assert_eq!(c.turn(-20.0), -1);
        // 10 wound backwards: 20 more back is a notch.
        assert_eq!(c.turn(-20.0), -1);
        assert_eq!(c.turn(30.0), 1);
    }

    /// Turning the other way starts a fresh notch: one notch back and a
    /// little more, then a whole notch forwards is one notch forwards.
    #[test]
    fn turning_the_other_way_starts_a_fresh_notch() {
        let mut c = crank(30);
        assert_eq!(c.turn(-40.0), -1);
        assert_eq!(c.turn(29.0), 0);
        assert_eq!(c.turn(1.0), 1);
        assert_eq!(c.turn(-20.0), 0);
        assert_eq!(c.turn(5.0), 0);
        assert_eq!(c.turn(-29.0), 0);
        assert_eq!(c.turn(-1.0), -1);
    }

    #[test]
    fn a_big_turn_is_several_notches_at_once() {
        let mut c = crank(30);
        assert_eq!(c.turn(-95.0), -3);
        assert_eq!(c.turn(65.0), 2);
        assert_eq!(c.turn(0.0), 0);
        assert_eq!(c.turn(25.0), 1, "the 5 left of 65, still forwards");
    }

    #[test]
    fn docking_forgets_a_part_notch() {
        let mut c = crank(30);
        assert_eq!(c.turn(-25.0), 0);
        c.dock();
        assert_eq!(c.turn(-25.0), 0);
        assert_eq!(c.turn(-5.0), -1);
    }

    #[test]
    fn a_broken_reading_is_no_turn() {
        let mut c = crank(30);
        assert_eq!(c.turn(f32::NAN), 0);
        assert_eq!(c.turn(f32::INFINITY), 0);
        assert_eq!(c.turn(1.0e30), 1200, "a hundred turns at most");
    }

    #[test]
    fn notches_press_the_crank_buttons() {
        assert_eq!(Crank::presses(0), None);
        assert_eq!(Crank::presses(-3), Some((Button::CrankBack, 3)));
        assert_eq!(Crank::presses(2), Some((Button::CrankForward, 2)));
    }

    /// The thousandths of a degree in `parts`, each part a frame's turn.
    fn total(parts: &[i32]) -> i32 {
        parts.iter().sum()
    }

    /// The notches each frame gives, one call per part.
    fn wind(c: &mut Crank, parts: &[i32]) -> Vec<Notches> {
        #[allow(clippy::cast_precision_loss)] // Small whole numbers.
        parts.iter().map(|&p| c.turn(p as f32 / MILLI)).collect()
    }

    proptest! {
        /// Frame-rate independence: a turn one way split over any number of
        /// frames gives as many notches as in one frame.
        #[test]
        fn any_split_of_a_one_way_turn_gives_the_same_notches(
            notch in 1u16..=360,
            parts in prop::collection::vec(0i32..100_000, 0..40),
            back in any::<bool>(),
        ) {
            let parts: Vec<i32> = parts.iter().map(|&p| if back { -p } else { p }).collect();
            let split: Notches = wind(&mut crank(notch), &parts).iter().sum();
            let whole = wind(&mut crank(notch), &[total(&parts)]);
            prop_assert_eq!(split, whole[0]);
            // A frame never gives notches the other way.
            for n in wind(&mut crank(notch), &parts) {
                let one_way = if back { n <= 0 } else { n >= 0 };
                prop_assert!(one_way, "{} notches", n);
            }
        }

        /// A hand wobbling on the crank, never a notch's worth either way
        /// at a time, never gives a notch.
        #[test]
        fn wobbling_by_less_than_a_notch_never_gives_one(
            notch in 1u16..=360,
            parts in prop::collection::vec(1i32..1000, 0..40),
        ) {
            let notch_milli = i32::from(notch) * 1000;
            // Back and forth in turns, each less than a notch.
            let wobble: Vec<i32> = parts
                .iter()
                .enumerate()
                .map(|(i, &p)| {
                    let p = p * notch_milli / 1000;
                    let p = p.clamp(1, notch_milli - 1);
                    if i % 2 == 0 { -p } else { p }
                })
                .collect();
            let notches = wind(&mut crank(notch), &wobble);
            prop_assert!(notches.iter().all(|&n| n == 0), "{:?}", notches);
        }
    }
}
