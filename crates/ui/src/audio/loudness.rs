//! The loudness curve (ADR-0091): a volume the player sets, 0 to 100, as
//! the [`Gain`] it plays at. Each step is the same change in loudness
//! ([`STEP_DB_TENTHS`]), so the slider's whole length is heard, where a
//! linear one crowds what a player hears into its top steps. Integers
//! only: a backend's mixer of our own takes the same units.

use crate::settings::MAX_VOLUME;

/// How loud something plays, in units of 1/[`Gain::UNIT`]: 0 is silent,
/// [`Gain::FULL`] the sound as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, serde::Serialize)]
pub struct Gain(u16);

impl Gain {
    /// The units of a full gain: 4096.
    pub const UNIT: u16 = 4096;

    /// Silence.
    pub const SILENT: Self = Self(0);

    /// The sound as it is.
    pub const FULL: Self = Self(Self::UNIT);

    /// The gain of `units` (each 1/4096), at most [`FULL`](Self::FULL).
    #[must_use]
    pub const fn new(units: u16) -> Self {
        if units > Self::UNIT {
            Self::FULL
        } else {
            Self(units)
        }
    }

    /// Its units, 0 to [`UNIT`](Self::UNIT).
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// This gain over `other`, rounded to the nearest unit: a group's gain
    /// times its parent's.
    #[must_use]
    pub fn times(self, other: Self) -> Self {
        let unit = u32::from(Self::UNIT);
        let product = (u32::from(self.0) * u32::from(other.0) + unit / 2) / unit;
        // At most UNIT, since neither is over it.
        Self(u16::try_from(product).unwrap_or(Self::UNIT))
    }
}

/// The change in loudness of one step of a volume, in tenths of a decibel:
/// 0.6 dB, so 1 is 59.4 dB under 100. *Tunable.*
pub const STEP_DB_TENTHS: u8 = 6;

/// The gain of each volume, 0 to 100: entry `v` is
/// `round(4096 × 10^((v − 100) × 0.03))`, and 0 is silent. A test works
/// each one out again.
const TABLE: [u16; 101] = [
    0, 4, 5, 5, 5, 6, 6, 7, 7, 8, //
    8, 9, 9, 10, 11, 12, 12, 13, 14, 15, //
    16, 17, 19, 20, 21, 23, 25, 26, 28, 30, //
    33, 35, 37, 40, 43, 46, 49, 53, 57, 61, //
    65, 70, 75, 80, 86, 92, 98, 105, 113, 121, //
    130, 139, 149, 159, 171, 183, 196, 210, 225, 241, //
    258, 277, 297, 318, 341, 365, 391, 419, 449, 481, //
    516, 553, 592, 634, 680, 728, 780, 836, 896, 960, //
    1029, 1102, 1181, 1266, 1356, 1453, 1557, 1669, 1788, 1916, //
    2053, 2200, 2357, 2526, 2706, 2900, 3107, 3329, 3567, 3823, //
    4096,
];

/// The gain a volume of `volume` (0 to [`MAX_VOLUME`]) plays at; above it
/// counts as [`MAX_VOLUME`].
#[must_use]
pub const fn loudness(volume: u8) -> Gain {
    let at = if volume > MAX_VOLUME {
        MAX_VOLUME
    } else {
        volume
    };
    Gain(TABLE[at as usize])
}

/// The volume whose gain is nearest `percent` of full (0 to 100; above it
/// counts as 100): an old linear volume put where it sounds the same. 80
/// gives 97, 35 gives 85.
#[must_use]
pub fn position_of_linear(percent: u8) -> u8 {
    let percent = percent.min(100);
    // Both sides times 100, to stay in integers.
    let target = u32::from(percent) * u32::from(Gain::UNIT);
    let off = |&(_, gain): &(usize, &u16)| (u32::from(*gain) * 100).abs_diff(target);
    let nearest = TABLE
        .iter()
        .enumerate()
        .min_by_key(off)
        .map_or(0, |(v, _)| v);
    u8::try_from(nearest).unwrap_or(MAX_VOLUME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ends_are_silence_and_full() {
        assert_eq!(loudness(0), Gain::SILENT);
        assert_eq!(loudness(100), Gain::FULL);
        assert_eq!(loudness(200), Gain::FULL);
        assert_eq!(loudness(97).get(), 3329);
    }

    #[test]
    fn the_table_never_goes_down_and_is_the_formula() {
        assert!(TABLE.windows(2).all(|w| w[0] <= w[1]));
        let step = f64::from(STEP_DB_TENTHS) / 200.0;
        for v in 1..=100_u8 {
            let want = 4096.0 * 10_f64.powf((f64::from(v) - 100.0) * step);
            assert_eq!(f64::from(loudness(v).get()), want.round(), "volume {v}");
        }
    }

    #[test]
    fn an_old_linear_volume_is_put_where_it_sounds_the_same() {
        assert_eq!(position_of_linear(80), 97);
        assert_eq!(position_of_linear(35), 85);
        assert_eq!(position_of_linear(0), 0);
        assert_eq!(position_of_linear(100), 100);
        assert_eq!(position_of_linear(250), 100);
    }

    #[test]
    fn a_gain_times_another_rounds_to_the_nearest_unit() {
        assert_eq!(Gain::FULL.times(Gain::new(3329)), Gain::new(3329));
        assert_eq!(Gain::new(2048).times(Gain::new(2048)), Gain::new(1024));
        assert_eq!(Gain::new(3).times(Gain::new(2048)), Gain::new(2));
        assert_eq!(Gain::SILENT.times(Gain::FULL), Gain::SILENT);
        assert_eq!(Gain::new(9000), Gain::FULL);
    }
}
