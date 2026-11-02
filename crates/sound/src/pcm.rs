//! Decoded samples: 16 bits, interleaved, one or two channels.

use alloc::vec::Vec;

use crate::error::Error;
use crate::source::PcmSource;

/// How many channels a frame has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channels {
    /// One sample a frame.
    Mono,
    /// Two samples a frame, left first.
    Stereo,
}

impl Channels {
    /// `count` channels, if a sound can have that many.
    pub const fn from_count(count: u32) -> Result<Self, Error> {
        match count {
            1 => Ok(Self::Mono),
            2 => Ok(Self::Stereo),
            other => Err(Error::Channels(other)),
        }
    }

    /// How many samples a frame has.
    pub const fn count(self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
        }
    }
}

/// What a decoder may produce: a malformed or hostile file stops here, before
/// it costs the memory it asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Limits {
    /// The highest rate, in samples a second.
    pub max_rate: u32,
    /// The most frames one source may yield.
    pub max_frames: u64,
}

impl Limits {
    /// The highest rate of the default limits.
    pub const DEFAULT_MAX_RATE: u32 = 192_000;
    /// The most frames of the default limits: about 25 minutes at 44.1 kHz,
    /// which is 256 MiB of stereo decoded whole.
    pub const DEFAULT_MAX_FRAMES: u64 = 1 << 26;

    /// These limits.
    pub const fn new(max_rate: u32, max_frames: u64) -> Self {
        Self {
            max_rate,
            max_frames,
        }
    }
}

impl Default for Limits {
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_RATE, Self::DEFAULT_MAX_FRAMES)
    }
}

/// The rate and channels of some samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct PcmFormat {
    /// Frames a second.
    pub rate: u32,
    /// How many samples a frame has.
    pub channels: Channels,
}

impl PcmFormat {
    /// `rate` and `channels`, if they are within `limits`.
    pub fn new(rate: u32, channels: u32, limits: &Limits) -> Result<Self, Error> {
        let channels = Channels::from_count(channels)?;
        if rate == 0 || rate > limits.max_rate {
            return Err(Error::Rate {
                rate,
                max: limits.max_rate,
            });
        }
        Ok(Self { rate, channels })
    }

    /// How many samples `frames` frames are, if that fits.
    pub fn samples_of(&self, frames: u64) -> Option<usize> {
        let samples = frames.checked_mul(u64::from(self.channels.count()))?;
        usize::try_from(samples).ok()
    }

    /// How long `frames` frames play, in whole milliseconds rounded down, if
    /// that fits.
    pub fn millis_of(&self, frames: u64) -> Option<u64> {
        let rate = u64::from(self.rate);
        let whole = frames / rate;
        let part = frames % rate;
        whole.checked_mul(1000)?.checked_add(part * 1000 / rate)
    }

    /// How many whole frames play in `millis` milliseconds, if that fits.
    pub fn frames_of_millis(&self, millis: u64) -> Option<u64> {
        let rate = u64::from(self.rate);
        let whole = millis / 1000;
        let part = millis % 1000;
        whole.checked_mul(rate)?.checked_add(part * rate / 1000)
    }
}

/// `sample` of 8 bits as one of 16: the same fraction of full scale.
pub const fn from_i8(sample: i8) -> i16 {
    (sample as i16) << 8
}

/// `sample` of 24 bits, in the low bits of an `i32`, as one of 16: the bits
/// below are dropped, as a truncating converter does. Out of range, it clamps.
pub const fn from_i24(sample: i32) -> i16 {
    clamp_i16(sample >> 8)
}

/// `sample` of 32 bits as one of 16: the bits below are dropped.
pub const fn from_i32(sample: i32) -> i16 {
    clamp_i16(sample >> 16)
}

const fn clamp_i16(value: i32) -> i16 {
    if value > i16::MAX as i32 {
        i16::MAX
    } else if value < i16::MIN as i32 {
        i16::MIN
    } else {
        // Between the two, as just checked.
        #[allow(clippy::cast_possible_truncation)]
        let narrow = value as i16;
        narrow
    }
}

/// A sound decoded whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    format: PcmFormat,
    samples: Vec<i16>,
}

impl Pcm {
    /// `samples`, interleaved, if they are whole frames within `limits`.
    pub fn new(format: PcmFormat, samples: Vec<i16>, limits: &Limits) -> Result<Self, Error> {
        let channels = format.channels.count();
        let count = samples.len() as u64;
        if !count.is_multiple_of(u64::from(channels)) {
            return Err(Error::PartialFrame {
                samples: count,
                channels,
            });
        }
        let frames = count / u64::from(channels);
        if frames > limits.max_frames {
            return Err(Error::TooLong {
                frames,
                limit: limits.max_frames,
            });
        }
        Ok(Self { format, samples })
    }

    /// Its rate and channels.
    pub const fn format(&self) -> PcmFormat {
        self.format
    }

    /// Its samples, interleaved.
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Its samples, handed over.
    pub fn into_samples(self) -> Vec<i16> {
        self.samples
    }

    /// How many frames it has.
    pub fn frames(&self) -> u64 {
        self.samples.len() as u64 / u64::from(self.format.channels.count())
    }

    /// How long it plays, in whole milliseconds rounded down.
    pub fn millis(&self) -> u64 {
        // Under `u64::MAX / 1000` frames, as memory can't hold more.
        self.format.millis_of(self.frames()).unwrap_or(u64::MAX)
    }

    /// A source of its samples, from the start.
    pub const fn source(&self) -> PcmSource<'_> {
        PcmSource::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const STEREO: PcmFormat = PcmFormat {
        rate: 44_100,
        channels: Channels::Stereo,
    };

    #[test]
    fn a_sound_has_one_or_two_channels() {
        assert_eq!(Channels::from_count(1).map(Channels::count), Ok(1));
        assert_eq!(Channels::from_count(2).map(Channels::count), Ok(2));
        assert_eq!(Channels::from_count(0), Err(Error::Channels(0)));
        assert_eq!(Channels::from_count(3), Err(Error::Channels(3)));
    }

    #[test]
    fn a_rate_is_from_one_to_the_limit() {
        let limits = Limits::new(48_000, 10);
        let bad = |rate| Err(Error::Rate { rate, max: 48_000 });
        assert_eq!(PcmFormat::new(0, 1, &limits), bad(0));
        assert_eq!(PcmFormat::new(48_001, 1, &limits), bad(48_001));
        assert_eq!(
            PcmFormat::new(48_000, 1, &limits).map(|f| f.rate),
            Ok(48_000)
        );
        assert_eq!(PcmFormat::new(1, 2, &limits).map(|f| f.rate), Ok(1));
        assert_eq!(PcmFormat::new(1, 5, &limits), Err(Error::Channels(5)));
    }

    #[test]
    fn the_default_limits_are_the_ones_written() {
        let limits = Limits::default();
        assert_eq!(limits.max_rate, 192_000);
        assert_eq!(limits.max_frames, 67_108_864);
    }

    #[test]
    fn arithmetic_is_exact_and_checked() {
        assert_eq!(STEREO.samples_of(3), Some(6));
        assert_eq!(STEREO.samples_of(u64::MAX), None);
        assert_eq!(STEREO.millis_of(44_100), Some(1000));
        assert_eq!(STEREO.millis_of(66_150), Some(1500));
        assert_eq!(STEREO.millis_of(44), Some(0));
        assert_eq!(STEREO.millis_of(45), Some(1));
        assert_eq!(STEREO.frames_of_millis(1500), Some(66_150));
        assert_eq!(STEREO.frames_of_millis(1), Some(44));
        let fast = PcmFormat {
            rate: 1,
            channels: Channels::Mono,
        };
        assert_eq!(fast.millis_of(u64::MAX), None);
        assert_eq!(
            fast.millis_of(u64::MAX / 1000),
            Some(u64::MAX / 1000 * 1000)
        );
        let high = PcmFormat {
            rate: u32::MAX,
            channels: Channels::Mono,
        };
        assert_eq!(high.frames_of_millis(u64::MAX), None);
    }

    #[test]
    fn widths_keep_the_fraction_of_full_scale() {
        assert_eq!(from_i8(i8::MAX), 0x7F00);
        assert_eq!(from_i8(i8::MIN), i16::MIN);
        assert_eq!(from_i8(1), 256);
        assert_eq!(from_i24(0x7F_FFFF), i16::MAX);
        assert_eq!(from_i24(-0x80_0000), i16::MIN);
        assert_eq!(from_i24(0x100), 1);
        assert_eq!(from_i24(0x0FF), 0);
        assert_eq!(from_i24(i32::MAX), i16::MAX);
        assert_eq!(from_i24(i32::MIN), i16::MIN);
        assert_eq!(from_i32(i32::MAX), i16::MAX);
        assert_eq!(from_i32(i32::MIN), i16::MIN);
        assert_eq!(from_i32(0x1_0000), 1);
    }

    #[test]
    fn pcm_holds_whole_frames_within_the_limit() {
        let limits = Limits::new(48_000, 2);
        let pcm = Pcm::new(STEREO, vec![1, 2, 3, 4], &limits);
        let pcm = pcm.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(pcm.frames(), 2);
        assert_eq!(pcm.samples(), [1, 2, 3, 4]);
        assert_eq!(pcm.format(), STEREO);
        assert_eq!(pcm.millis(), 0);
        assert_eq!(pcm.clone().into_samples(), [1, 2, 3, 4]);
        assert_eq!(
            Pcm::new(STEREO, vec![1, 2, 3], &limits),
            Err(Error::PartialFrame {
                samples: 3,
                channels: 2
            })
        );
        assert_eq!(
            Pcm::new(STEREO, vec![0; 6], &limits),
            Err(Error::TooLong {
                frames: 3,
                limit: 2
            })
        );
    }

    #[test]
    fn a_seconds_worth_of_pcm_plays_a_second() {
        let samples = vec![0; 88_200];
        let pcm = Pcm::new(STEREO, samples, &Limits::default());
        assert_eq!(pcm.map(|p| p.millis()), Ok(1000));
    }

    proptest! {
        /// Frames to milliseconds and back loses less than a millisecond's worth.
        #[test]
        fn frames_and_millis_round_trip(rate in 1u32..=192_000, frames in 0u64..1 << 40) {
            let format = PcmFormat { rate, channels: Channels::Mono };
            let millis = format.millis_of(frames).unwrap_or(0);
            let back = format.frames_of_millis(millis).unwrap_or(0);
            prop_assert!(back <= frames);
            prop_assert!(frames - back <= u64::from(rate) / 1000 + 1);
        }

        /// However many samples, `new` answers and never panics, and what it
        /// holds is whole frames.
        #[test]
        fn pcm_new_never_panics(count in 0usize..64, stereo: bool, limit in 0u64..64) {
            let channels = if stereo { Channels::Stereo } else { Channels::Mono };
            let format = PcmFormat { rate: 8_000, channels };
            let limits = Limits::new(8_000, limit);
            if let Ok(pcm) = Pcm::new(format, vec![0; count], &limits) {
                prop_assert!(pcm.frames() <= limit);
                prop_assert_eq!(pcm.samples().len() % usize::from(channels.count()), 0);
            }
        }
    }
}
