//! Sounds decoded to a WAV in memory (ticket 2234): a FLAC or MP3 file as
//! 16-bit PCM, which every backend plays, and how long a track plays
//! whatever its format. The WAV is no asset: nothing hashes it, and no
//! test pins its samples, so an MP3 decoder's floats are fine here.
//!
//! The functions are always compiled, also without `std`; only their FLAC
//! and MP3 arms need the `flac` and `mp3` features. With a feature off,
//! a file of its format is an error that names the feature.
use crate::prelude::*;

use super::{MusicTrack, Sound, has_extension, ogg_length_ms, wav_problem};
use crate::format::{AssetPath, DecodeCx, Format};

/// The most bytes a WAV of [`decode_to_wav`] may have: 256 MiB, about 25
/// minutes of stereo at 44.1 kHz. The file may come from a mod (ADR-0007
/// §5), so decoding stops as soon as its WAV would pass this.
pub const MAX_DECODED_BYTES: usize = 256 << 20;

/// A format [`decode_to_wav`] decodes, by a file's extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Coded {
    Flac,
    Mp3,
}

impl Coded {
    /// The format of a file named `name`, if it is one of these.
    fn of(name: &str) -> Option<Self> {
        if has_extension(name, &["flac"]) {
            Some(Self::Flac)
        } else if has_extension(name, &["mp3"]) {
            Some(Self::Mp3)
        } else {
            None
        }
    }

    /// Its extension, which is also the name of its feature.
    const fn extension(self) -> &'static str {
        match self {
            Self::Flac => "flac",
            Self::Mp3 => "mp3",
        }
    }

    /// Its [`Sound`] and [`MusicTrack`] formats, if its feature is on.
    const fn formats(self) -> Readers {
        match self {
            Self::Flac => FLAC,
            Self::Mp3 => MP3,
        }
    }

    /// Its [`Sound`] format, if its feature is on.
    const fn sound_format(self) -> Option<&'static dyn Format<Asset = Sound>> {
        self.formats().0
    }

    /// Its [`MusicTrack`] format, if its feature is on.
    const fn track_format(self) -> Option<&'static dyn Format<Asset = MusicTrack>> {
        self.formats().1
    }
}

/// A coded format's [`Sound`] and [`MusicTrack`] formats, `None` with its
/// feature off.
type Readers = (
    Option<&'static dyn Format<Asset = Sound>>,
    Option<&'static dyn Format<Asset = MusicTrack>>,
);

#[cfg(feature = "flac")]
const FLAC: Readers = (Some(&super::Flac), Some(&super::FlacTrack));
#[cfg(not(feature = "flac"))]
const FLAC: Readers = (None, None);
#[cfg(feature = "mp3")]
const MP3: Readers = (Some(&super::Mp3), Some(&super::Mp3Track));
#[cfg(not(feature = "mp3"))]
const MP3: Readers = (None, None);

/// What a file of `coded`'s format is told when its feature is off.
fn needs_feature(coded: Coded) -> String {
    let extension = coded.extension();
    format!("a .{extension} file needs the {extension} feature of shuyi-assets")
}

/// Whether `bytes` are a WAV file: `RIFF`, then `WAVE`.
fn is_wav(bytes: &[u8]) -> bool {
    bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WAVE")
}

/// A sound file named `name` decoded to a WAV that every backend plays:
///
/// - `None`, nothing to decode: a name that doesn't end in `.flac` or
///   `.mp3`, or bytes that already are a WAV (a game's hook decoded them);
/// - `Some(Ok(wav))`: a 16-bit PCM WAV with the file's channels and rate.
///   FLAC's samples are shifted to 16 bits, MP3's rounded and clamped;
/// - `Some(Err(why))`: bytes that don't decode, a WAV that would pass
///   [`MAX_DECODED_BYTES`], or a format whose feature is off.
pub fn decode_to_wav(name: &str, bytes: &[u8]) -> Option<Result<Vec<u8>, String>> {
    decode_within(name, bytes, MAX_DECODED_BYTES)
}

/// [`decode_to_wav`], with a WAV of at most `limit` bytes.
fn decode_within(name: &str, bytes: &[u8], limit: usize) -> Option<Result<Vec<u8>, String>> {
    let coded = Coded::of(name)?;
    if is_wav(bytes) {
        return None;
    }
    Some(match coded {
        Coded::Flac => flac_wav(bytes, limit),
        Coded::Mp3 => mp3_wav(bytes, limit),
    })
}

/// How long a music track named `name` plays, in milliseconds: an OGG
/// file's from its last page ([`ogg_length_ms`]), a FLAC or MP3 file's as
/// its [`MusicTrack`] format reads it. `None` if it can't be read, or its
/// format's feature is off.
pub fn track_length_ms(name: &str, bytes: &[u8]) -> Option<u32> {
    if has_extension(name, &["ogg"]) {
        return ogg_length_ms(bytes);
    }
    let format = Coded::of(name)?.track_format()?;
    let track = format.decode(&AssetPath::new(name), bytes, &DecodeCx::new());
    track.ok().map(|track| track.length_ms)
}

/// For a sound file named `name` that ends in `.flac` or `.mp3`, why it
/// isn't something the game can play, if it isn't (the inner `Option`);
/// `None` for any other name.
pub(super) fn coded_problem(name: &str, bytes: &[u8]) -> Option<Option<String>> {
    let coded = Coded::of(name)?;
    Some(read_problem(name, bytes, coded, coded.sound_format()))
}

/// Why a file of `coded`'s format isn't something the game can play: as a
/// WAV if its bytes are one (a game's hook decoded them), else as `format`
/// (its [`Sound`] format, `None` if its feature is off) reads it.
fn read_problem(
    name: &str,
    bytes: &[u8],
    coded: Coded,
    format: Option<&dyn Format<Asset = Sound>>,
) -> Option<String> {
    if is_wav(bytes) {
        return wav_problem(bytes);
    }
    let Some(format) = format else {
        return Some(needs_feature(coded));
    };
    let errors = format
        .decode(&AssetPath::new(name), bytes, &DecodeCx::new())
        .err()?;
    let messages: Vec<String> = errors.into_iter().map(|e| e.message).collect();
    Some(messages.join("; "))
}

/// What a WAV that would be too big is told.
fn too_big(limit: usize) -> String {
    format!("decoded, it would be more than {limit} bytes")
}

/// A 16-bit PCM WAV being written: its 44-byte header, then its samples,
/// at most `limit` bytes in all.
struct WavWriter {
    bytes: Vec<u8>,
    limit: usize,
}

/// The length of a WAV's header.
const HEADER: usize = 44;

impl WavWriter {
    /// A WAV of `channels` channels at `sample_rate` Hz, its sizes written
    /// by [`finish`](Self::finish).
    fn new(channels: u16, sample_rate: u32, limit: usize) -> Self {
        let block = channels.saturating_mul(2);
        let mut bytes = Vec::with_capacity(HEADER);
        bytes.extend_from_slice(b"RIFF\0\0\0\0WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes()); // integer PCM
        bytes.extend_from_slice(&channels.to_le_bytes());
        bytes.extend_from_slice(&sample_rate.to_le_bytes());
        let per_second = sample_rate.saturating_mul(u32::from(block));
        bytes.extend_from_slice(&per_second.to_le_bytes());
        bytes.extend_from_slice(&block.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes()); // bits a sample
        bytes.extend_from_slice(b"data\0\0\0\0");
        Self { bytes, limit }
    }

    /// Adds `samples`, interleaved; an error, and nothing added, if they
    /// would take the WAV past its limit.
    fn extend(&mut self, samples: &[i16]) -> Result<(), String> {
        let wanted = samples.len().saturating_mul(2);
        if self.bytes.len().saturating_add(wanted) > self.limit {
            return Err(too_big(self.limit));
        }
        self.bytes.reserve(wanted);
        for sample in samples {
            self.bytes.extend_from_slice(&sample.to_le_bytes());
        }
        Ok(())
    }

    /// The whole file, its two sizes written.
    fn finish(mut self) -> Vec<u8> {
        let size = |bytes: usize| u32::try_from(bytes).unwrap_or(u32::MAX).to_le_bytes();
        let (riff, data) = (size(self.bytes.len() - 8), size(self.bytes.len() - HEADER));
        self.bytes[4..8].copy_from_slice(&riff);
        self.bytes[40..44].copy_from_slice(&data);
        self.bytes
    }
}

/// A 16-bit PCM WAV of `samples`, interleaved, of `channels` channels at
/// `sample_rate` Hz: the 44-byte header, then the samples. What
/// [`decode_to_wav`] makes, and what a game's own decoder can hand the
/// platform the same way.
pub fn pcm16_wav(channels: u16, sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let mut wav = WavWriter::new(channels, sample_rate, usize::MAX);
    // No limit to pass.
    let _ = wav.extend(samples);
    wav.finish()
}

/// A FLAC file's sample of `bits` bits as 16 bits: shifted, then clamped
/// (only a broken file has a sample wider than its bits).
#[cfg(feature = "flac")]
fn flac_sample(sample: i32, bits: u32) -> i16 {
    let shifted = if bits >= 16 {
        sample >> (bits - 16).min(31)
    } else {
        sample << (16 - bits)
    };
    i16::try_from(shifted).unwrap_or(if shifted < 0 { i16::MIN } else { i16::MAX })
}

/// A FLAC file decoded to a WAV of at most `limit` bytes, by `claxon`.
#[cfg(feature = "flac")]
fn flac_wav(bytes: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    let broken = |e: claxon::Error| format!("not a readable FLAC file: {e}");
    let mut reader = claxon::FlacReader::new(bytes).map_err(broken)?;
    let info = reader.streaminfo();
    let channels = u16::try_from(info.channels).map_err(|_| "too many channels".to_owned())?;
    let mut wav = WavWriter::new(channels, info.sample_rate, limit);
    let mut blocks = reader.blocks();
    let mut buffer = Vec::new();
    let mut frame = Vec::new();
    while let Some(block) = blocks.read_next_or_eof(buffer).map_err(broken)? {
        if block.channels() != info.channels {
            return Err(format!(
                "a block has {} channels; the file has {}",
                block.channels(),
                info.channels
            ));
        }
        frame.clear();
        for at in 0..block.duration() {
            for channel in 0..block.channels() {
                let sample = block.sample(channel, at);
                frame.push(flac_sample(sample, info.bits_per_sample));
            }
        }
        wav.extend(&frame)?;
        buffer = block.into_buffer();
    }
    Ok(wav.finish())
}

#[cfg(not(feature = "flac"))]
fn flac_wav(_: &[u8], _: usize) -> Result<Vec<u8>, String> {
    Err(needs_feature(Coded::Flac))
}

/// An MP3 file decoded to a WAV of at most `limit` bytes, by `nanomp3`:
/// gapless (without the silence its encoder added at the ends, when its
/// LAME tag says how much), its samples rounded and clamped to 16 bits.
#[cfg(feature = "mp3")]
fn mp3_wav(bytes: &[u8], limit: usize) -> Result<Vec<u8>, String> {
    let mut reader = nanomp3::SliceReader::<i16>::new(bytes);
    let channels = match reader.channels() {
        Some(nanomp3::Channels::Mono) => 1,
        Some(nanomp3::Channels::Stereo) => 2,
        None => return Err("not an MP3 file".to_owned()),
    };
    let mut wav = WavWriter::new(channels, reader.sample_rate(), limit);
    let broken = |e: nanomp3::Error| format!("not a readable MP3 file: {e}");
    while let Some(samples) = reader.read_frame().map_err(broken)? {
        wav.extend(samples)?;
    }
    Ok(wav.finish())
}

#[cfg(not(feature = "mp3"))]
fn mp3_wav(_: &[u8], _: usize) -> Result<Vec<u8>, String> {
    Err(needs_feature(Coded::Mp3))
}

#[cfg(test)]
mod tests;
