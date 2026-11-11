//! [`Samples`]: the sound a game makes at run time, 16-bit samples at a rate,
//! mono or stereo, whole when it is made.
//! [`AudioQueue::add_sound`](super::AudioQueue::add_sound) makes them a cue;
//! every backend reads them as they are.

use crate::prelude::*;

// What two owners both hold. A target without shared atomic counts has one thread.
#[cfg(not(target_has_atomic = "ptr"))]
use alloc::rc::Rc as Shared;
#[cfg(target_has_atomic = "ptr")]
use alloc::sync::Arc as Shared;

/// [`Samples`] that the queue and whatever plays them share, never copied
/// on their way.
pub type SharedSamples = Shared<Samples>;

/// Why [`Samples`] couldn't be made.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SamplesError {
    /// A sound has one or two channels.
    #[error("{0} channels: a sound has one or two")]
    Channels(u32),
    /// A rate of zero, or over [`Samples::MAX_RATE`].
    #[error("{0} samples a second: it must be from 1 to {max}", max = Samples::MAX_RATE)]
    Rate(u32),
    /// Samples that don't divide into frames.
    #[error("{samples} samples aren't whole frames of {channels} channels")]
    PartialFrame {
        /// How many samples there are.
        samples: usize,
        /// How many channels a frame has.
        channels: u16,
    },
}

/// A sound made at run time: 16-bit samples, interleaved (left first in
/// stereo), at a rate. Any rate from 1 to [`Self::MAX_RATE`]: a backend
/// plays it at its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Samples {
    rate: u32,
    channels: u16,
    data: Vec<i16>,
}

impl Samples {
    /// The highest rate, in samples a second.
    pub const MAX_RATE: u32 = 192_000;

    /// `samples`, interleaved, at `rate` with `channels` (1 or 2), if they
    /// are whole frames.
    pub fn new(rate: u32, channels: u32, samples: Vec<i16>) -> Result<Self, SamplesError> {
        let channels = match channels {
            1 => 1,
            2 => 2,
            other => return Err(SamplesError::Channels(other)),
        };
        if rate == 0 || rate > Self::MAX_RATE {
            return Err(SamplesError::Rate(rate));
        }
        if !samples.len().is_multiple_of(usize::from(channels)) {
            return Err(SamplesError::PartialFrame {
                samples: samples.len(),
                channels,
            });
        }
        Ok(Self {
            rate,
            channels,
            data: samples,
        })
    }

    /// `bytes` of 8-bit unsigned samples (128 is silence), as old formats keep
    /// them, interleaved, at `rate` with `channels`: each becomes
    /// `(b − 128) × 256`.
    pub fn from_u8(rate: u32, channels: u32, bytes: &[u8]) -> Result<Self, SamplesError> {
        let samples = bytes.iter().map(|&b| (i16::from(b) - 128) * 256).collect();
        Self::new(rate, channels, samples)
    }

    /// Its rate, in frames a second.
    pub const fn rate(&self) -> u32 {
        self.rate
    }

    /// How many channels it has: 1 or 2.
    pub const fn channels(&self) -> u16 {
        self.channels
    }

    /// Its samples, interleaved.
    pub fn samples(&self) -> &[i16] {
        &self.data
    }

    /// Its samples, handed over.
    pub fn into_samples(self) -> Vec<i16> {
        self.data
    }

    /// How many frames it has.
    pub fn frames(&self) -> u64 {
        u64::try_from(self.data.len() / usize::from(self.channels)).unwrap_or(u64::MAX)
    }

    /// How long it plays, in whole milliseconds rounded down.
    pub fn millis(&self) -> u64 {
        let rate = u64::from(self.rate);
        let frames = self.frames();
        (frames / rate)
            .saturating_mul(1000)
            .saturating_add(frames % rate * 1000 / rate)
    }

    /// The bytes its samples hold: two a sample.
    pub fn bytes(&self) -> u64 {
        u64::try_from(self.data.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(2)
    }

    /// It as a WAV file of 16-bit integer samples, at its rate and channels:
    /// for a device that loads a sound from a file's bytes.
    pub fn to_wav(&self) -> Vec<u8> {
        let block = self.channels * 2;
        let data_len = wav_data_bytes(self.data.len(), block);
        let written = usize::try_from(data_len / 2).unwrap_or(0);
        let mut out = Vec::with_capacity(44 + written * 2);
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(36 + data_len).to_le_bytes());
        out.extend_from_slice(b"WAVEfmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&self.channels.to_le_bytes());
        out.extend_from_slice(&self.rate.to_le_bytes());
        let byte_rate = self.rate.saturating_mul(u32::from(block));
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block.to_le_bytes());
        out.extend_from_slice(&16u16.to_le_bytes());
        out.extend_from_slice(b"data");
        out.extend_from_slice(&data_len.to_le_bytes());
        for sample in &self.data[..written] {
            out.extend_from_slice(&sample.to_le_bytes());
        }
        out
    }
}

/// The bytes of samples a WAV of `samples` samples, `block` bytes a frame,
/// holds: all of them, or the whole frames its 32-bit sizes can say.
fn wav_data_bytes(samples: usize, block: u16) -> u32 {
    let room = (u32::MAX - 36) / u32::from(block) * u32::from(block);
    u32::try_from(samples.saturating_mul(2)).map_or(room, |bytes| bytes.min(room))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_bit_unsigned_samples_centre_on_128() {
        let made = Samples::from_u8(11_025, 1, &[128, 255, 0, 129, 127]);
        let made = made.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(made.samples(), [0, 0x7F00, i16::MIN, 256, -256]);
        assert_eq!((made.rate(), made.channels()), (11_025, 1));
        let stereo = Samples::from_u8(8_000, 2, &[0, 255]);
        let stereo = stereo.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((stereo.channels(), stereo.frames()), (2, 1));
        assert_eq!(
            Samples::from_u8(8_000, 2, &[1, 2, 3]),
            Err(SamplesError::PartialFrame {
                samples: 3,
                channels: 2
            })
        );
        assert_eq!(Samples::from_u8(0, 1, &[1]), Err(SamplesError::Rate(0)));
        assert_eq!(
            Samples::from_u8(192_001, 1, &[1]),
            Err(SamplesError::Rate(192_001))
        );
        assert_eq!(
            Samples::from_u8(8_000, 3, &[1]),
            Err(SamplesError::Channels(3))
        );
        assert_eq!(
            Samples::new(8_000, 0, vec![]),
            Err(SamplesError::Channels(0))
        );
    }

    #[test]
    fn length_and_size_are_exact() {
        let second = Samples::new(44_100, 2, vec![0; 88_200]);
        let second = second.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            (second.frames(), second.millis(), second.bytes()),
            (44_100, 1000, 176_400)
        );
        let short = Samples::new(11_025, 1, vec![0; 1102]).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(short.millis(), 99);
        assert_eq!(short.clone().into_samples().len(), 1102);
        let empty = Samples::new(1, 1, Vec::new()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((empty.frames(), empty.millis()), (0, 0));
    }

    #[test]
    fn a_wav_holds_the_samples_after_a_44_byte_header() {
        let made = Samples::new(44_100, 2, vec![1, -2, 0x7FFF, i16::MIN]);
        let wav = made.unwrap_or_else(|e| panic!("{e}")).to_wav();
        assert_eq!(wav.len(), 44 + 8);
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(wav[4..8], 44u32.to_le_bytes());
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        assert_eq!(wav[16..20], 16u32.to_le_bytes());
        // Integer samples, two channels, at 44.1 kHz, four bytes a frame, 16 bits.
        assert_eq!(wav[20..22], 1u16.to_le_bytes());
        assert_eq!(wav[22..24], 2u16.to_le_bytes());
        assert_eq!(wav[24..28], 44_100u32.to_le_bytes());
        assert_eq!(wav[28..32], (44_100u32 * 4).to_le_bytes());
        assert_eq!(wav[32..34], 4u16.to_le_bytes());
        assert_eq!(wav[34..36], 16u16.to_le_bytes());
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(wav[40..44], 8u32.to_le_bytes());
        assert_eq!(wav[44..], [1, 0, 0xFE, 0xFF, 0xFF, 0x7F, 0, 0x80]);
    }

    #[test]
    fn a_mono_wav_has_two_bytes_a_frame() {
        let made = Samples::new(8_000, 1, vec![5, -5, 7]).unwrap_or_else(|e| panic!("{e}"));
        let wav = made.to_wav();
        assert_eq!(wav.len(), 44 + 6);
        assert_eq!(wav[4..8], 42u32.to_le_bytes());
        assert_eq!(wav[22..24], 1u16.to_le_bytes());
        assert_eq!(wav[28..32], 16_000u32.to_le_bytes());
        assert_eq!(wav[32..34], 2u16.to_le_bytes());
        assert_eq!(wav[40..44], 6u32.to_le_bytes());
        assert_eq!(wav[44..], [5, 0, 0xFB, 0xFF, 7, 0]);
    }

    #[test]
    fn a_wav_says_all_its_samples_or_the_whole_frames_32_bits_can() {
        // Room for 4,294,967,259 bytes after the header, in whole frames.
        assert_eq!(wav_data_bytes(0, 2), 0);
        assert_eq!(wav_data_bytes(3, 2), 6);
        assert_eq!(wav_data_bytes(2_147_483_629, 2), 4_294_967_258);
        assert_eq!(wav_data_bytes(2_147_483_630, 2), 4_294_967_258);
        assert_eq!(wav_data_bytes(2_147_483_628, 4), 4_294_967_256);
        assert_eq!(wav_data_bytes(2_147_483_630, 4), 4_294_967_256);
        assert_eq!(wav_data_bytes(usize::MAX, 2), 4_294_967_258);
        assert_eq!(wav_data_bytes(usize::MAX, 4), 4_294_967_256);
    }

    proptest::proptest! {
        /// Whatever it is asked, `new` answers and never panics, and what it
        /// holds is whole frames at a rate within the limit.
        #[test]
        fn new_never_panics(count in 0usize..64, channels in 0u32..4, rate in 0u32..200_000) {
            if let Ok(made) = Samples::new(rate, channels, vec![0; count]) {
                proptest::prop_assert!((1..=Samples::MAX_RATE).contains(&made.rate()));
                let channels = usize::from(made.channels());
                proptest::prop_assert_eq!(made.samples().len() % channels, 0);
                proptest::prop_assert_eq!(made.to_wav().len(), 44 + count * 2);
            }
        }
    }
}
