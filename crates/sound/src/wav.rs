#![cfg(feature = "std")]
//! WAV files, over the `hound` crate: integer samples of 8, 16, 24 or 32 bits
//! and float samples of 32. Playdate's IMA ADPCM WAV isn't one of them.

use std::io::Cursor;

use hound::{SampleFormat, WavReader};

use crate::error::Error;
use crate::guard::guarded;
use crate::pcm::{self, Limits, PcmFormat};
use crate::source::{Source, room};

const FORMAT: &str = "WAV";

/// How a file's samples become 16 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Int8,
    Int16,
    Int24,
    Int32,
    Float32,
}

/// A WAV file's samples, read from its bytes as they are asked for.
pub struct WavSource<'a> {
    reader: WavReader<Cursor<&'a [u8]>>,
    format: PcmFormat,
    kind: Kind,
    frames: u64,
    /// How many frames are still to come.
    left: u64,
}

fn malformed(error: &hound::Error) -> Error {
    Error::Malformed {
        format: FORMAT,
        reason: error.to_string(),
    }
}

impl<'a> WavSource<'a> {
    /// A source of `bytes`, whose header says what it holds. What that is must
    /// be within `limits`, and the bytes must be there for it.
    pub fn new(bytes: &'a [u8], limits: &Limits) -> Result<Self, Error> {
        guarded(FORMAT, || Self::open(bytes, limits))
    }

    fn open(bytes: &'a [u8], limits: &Limits) -> Result<Self, Error> {
        // The reader stops at the first sample, and says where only when given up.
        let open = || WavReader::new(Cursor::new(bytes)).map_err(|e| malformed(&e));
        let start = open()?.into_inner().position();
        let reader = open()?;
        let spec = reader.spec();
        let format = PcmFormat::new(spec.sample_rate, u32::from(spec.channels), limits)?;
        let kind = match (spec.sample_format, spec.bits_per_sample) {
            (SampleFormat::Int, 8) => Kind::Int8,
            (SampleFormat::Int, 16) => Kind::Int16,
            (SampleFormat::Int, 24) => Kind::Int24,
            (SampleFormat::Int, 32) => Kind::Int32,
            (SampleFormat::Float, 32) => Kind::Float32,
            (SampleFormat::Int, bits) => {
                return Err(Error::SampleKind {
                    kind: "integer",
                    bits,
                });
            }
            (SampleFormat::Float, bits) => {
                return Err(Error::SampleKind {
                    kind: "float",
                    bits,
                });
            }
        };
        let samples = u64::from(reader.len());
        let channels = u64::from(format.channels.count());
        if !samples.is_multiple_of(channels) {
            return Err(Error::PartialFrame {
                samples,
                channels: format.channels.count(),
            });
        }
        let frames = samples / channels;
        if frames > limits.max_frames {
            return Err(Error::TooLong {
                frames,
                limit: limits.max_frames,
            });
        }
        // What the header promises, against what is there: a corrupt length
        // shows now, and not as the file runs out while it plays.
        let needed = samples * u64::from(spec.bits_per_sample / 8);
        if needed > (bytes.len() as u64).saturating_sub(start) {
            return Err(Error::Malformed {
                format: FORMAT,
                reason: format!("it says it holds {needed} bytes of samples, and has less"),
            });
        }
        Ok(Self {
            reader,
            format,
            kind,
            frames,
            left: frames,
        })
    }
}

/// Fills `out` with `reader`'s next samples of `S`, each through `convert`.
fn fill<S: hound::Sample>(
    reader: &mut WavReader<Cursor<&[u8]>>,
    out: &mut [i16],
    convert: impl Fn(S) -> i16,
) -> Result<(), Error> {
    let mut samples = reader.samples::<S>();
    for slot in out {
        match samples.next() {
            Some(Ok(sample)) => *slot = convert(sample),
            Some(Err(e)) => return Err(malformed(&e)),
            None => {
                return Err(Error::Malformed {
                    format: FORMAT,
                    reason: "it ends early".to_owned(),
                });
            }
        }
    }
    Ok(())
}

/// A float sample, where 1 is full scale, as 16 bits, clamped. The one float in
/// the crate: this is where it stops being one.
#[allow(clippy::float_arithmetic)]
fn from_float(sample: f32) -> i16 {
    // Clamped to within `i16` first, so the cast never truncates; NaN casts to 0.
    #[allow(clippy::cast_possible_truncation)]
    let narrow = (sample * 32_767.0).round().clamp(-32_768.0, 32_767.0) as i16;
    narrow
}

impl Source for WavSource<'_> {
    fn format(&self) -> PcmFormat {
        self.format
    }

    fn total_frames(&self) -> Option<u64> {
        Some(self.frames)
    }

    fn read(&mut self, out: &mut [i16]) -> Result<usize, Error> {
        let room = room(self.format, out)?;
        // At most `room`, which is a `usize`.
        let frames = usize::try_from(self.left).map_or(room, |left| left.min(room));
        let count = frames * usize::from(self.format.channels.count());
        let (kind, reader) = (self.kind, &mut self.reader);
        let out = &mut out[..count];
        guarded(FORMAT, || match kind {
            Kind::Int8 => fill(reader, out, pcm::from_i8),
            Kind::Int16 => fill(reader, out, |sample: i16| sample),
            Kind::Int24 => fill(reader, out, pcm::from_i24),
            Kind::Int32 => fill(reader, out, pcm::from_i32),
            Kind::Float32 => fill(reader, out, from_float),
        })?;
        self.left -= frames as u64;
        Ok(frames)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_float_sample_rounds_and_clamps() {
        assert_eq!(from_float(0.0), 0);
        assert_eq!(from_float(1.0), 32_767);
        assert_eq!(from_float(-1.0), -32_767);
        assert_eq!(from_float(0.5), 16_384);
        assert_eq!(from_float(4.0), 32_767);
        assert_eq!(from_float(-4.0), -32_768);
        assert_eq!(from_float(f32::NAN), 0);
    }
}
