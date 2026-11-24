//! The audio kinds and formats (ADR-0007): a sound file's header, a music
//! track's, and the manifest. Only headers are read here. The platform
//! decodes a WAV's and an OGG's samples when it plays them; a FLAC's and
//! an MP3's are decoded to a WAV for it first ([`decode_to_wav`](super::decode_to_wav),
//! ticket 2234).
use crate::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(feature = "flac")]
use super::NO_FLAC_LENGTH;
#[cfg(feature = "mp3")]
use super::NO_MP3_LENGTH;
#[cfg(any(feature = "flac", feature = "mp3"))]
use super::samples_ms;
use super::{
    AudioManifest, Files, NO_LENGTH, from_source, ogg_length_ms, playable_problem, vorbis_format,
    wav_format,
};
use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, join, text};

/// The most bytes a sound or music file may have: 64 MiB, six minutes of
/// stereo WAV at [`SAMPLE_RATE`](super::SAMPLE_RATE).
pub const MAX_SOUND_BYTES: usize = 64 << 20;

/// A sound file's header: what the engine knows of a sound before the
/// platform plays it. Every one is mono or stereo, at
/// [`SAMPLE_RATE`](super::SAMPLE_RATE).
///
/// Data read from a file (ADR-0039 kind 5): a game's own sound format
/// (ADR-0007) builds one with [`Sound::mono`] or [`Sound::stereo`]. A
/// field added later is `#[serde(default)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Sound {
    /// 1 or 2.
    pub channels: u16,
    /// Samples a second.
    pub sample_rate: u32,
}

impl Sound {
    /// A mono sound of `sample_rate` samples a second.
    #[must_use]
    pub const fn mono(sample_rate: u32) -> Self {
        Self {
            channels: 1,
            sample_rate,
        }
    }

    /// A stereo sound of `sample_rate` samples a second.
    #[must_use]
    pub const fn stereo(sample_rate: u32) -> Self {
        Self {
            channels: 2,
            sample_rate,
        }
    }
}

impl AssetKind for Sound {
    const NAME: &'static str = "sound";
}

/// A music track's header, and how long it plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicTrack {
    /// 1 or 2.
    pub channels: u16,
    /// Samples a second.
    pub sample_rate: u32,
    /// How long it plays, in milliseconds (to the nearest).
    pub length_ms: u32,
}

impl AssetKind for MusicTrack {
    const NAME: &'static str = "music track";
}

/// A sound of `(channels, rate)`, from the format of a file `file` that
/// isn't one if `format` is `None` (then `not_one` says so), or every
/// problem with it.
fn sound(
    file: &AssetPath,
    format: Option<(u16, u32)>,
    not_one: &str,
) -> Result<Sound, Vec<ContentError>> {
    let error = |message: String| vec![ContentError::new(file.shown(), message)];
    let (channels, sample_rate) = format.ok_or_else(|| error(not_one.to_owned()))?;
    match playable_problem(channels, sample_rate) {
        Some(problem) => Err(error(problem)),
        None => Ok(Sound {
            channels,
            sample_rate,
        }),
    }
}

/// WAV, PCM or float (`wav`, `.wav`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Wav;

impl Format for Wav {
    type Asset = Sound;

    fn id(&self) -> &'static str {
        "wav"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["wav"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.get(..4) == Some(b"RIFF") && bytes.get(8..12) == Some(b"WAVE")
    }

    fn max_bytes(&self) -> usize {
        MAX_SOUND_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Sound, Vec<ContentError>> {
        sound(file, wav_format(bytes), "not a PCM WAV file")
    }
}

/// The first four bytes of every Ogg page.
const OGG_MAGIC: &[u8; 4] = b"OggS";

/// OGG Vorbis (`ogg-vorbis`, `.ogg`), read for a sound.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OggVorbis;

impl Format for OggVorbis {
    type Asset = Sound;

    fn id(&self) -> &'static str {
        "ogg-vorbis"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ogg"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(OGG_MAGIC)
    }

    fn max_bytes(&self) -> usize {
        MAX_SOUND_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Sound, Vec<ContentError>> {
        sound(file, vorbis_format(bytes), "not an OGG Vorbis file")
    }
}

/// OGG Vorbis (`ogg-vorbis`, `.ogg`), read for a music track: a whole
/// file, whose last page gives its length ([`ogg_length_ms`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OggVorbisTrack;

impl Format for OggVorbisTrack {
    type Asset = MusicTrack;

    fn id(&self) -> &'static str {
        "ogg-vorbis"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ogg"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(OGG_MAGIC)
    }

    fn max_bytes(&self) -> usize {
        MAX_SOUND_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<MusicTrack, Vec<ContentError>> {
        let Sound {
            channels,
            sample_rate,
        } = OggVorbis.decode(file, bytes, cx)?;
        let length_ms =
            ogg_length_ms(bytes).ok_or_else(|| vec![ContentError::new(file.shown(), NO_LENGTH)])?;
        Ok(MusicTrack {
            channels,
            sample_rate,
            length_ms,
        })
    }
}

/// The track of `sound` (from the file `file`), `samples` samples a channel
/// long if they are known; else `no_length` is the error.
#[cfg(any(feature = "flac", feature = "mp3"))]
fn track(
    file: &AssetPath,
    sound: Sound,
    samples: Option<u64>,
    no_length: &str,
) -> Result<MusicTrack, Vec<ContentError>> {
    let length_ms = samples
        .and_then(|samples| samples_ms(samples, sound.sample_rate))
        .ok_or_else(|| vec![ContentError::new(file.shown(), no_length)])?;
    Ok(MusicTrack {
        channels: sound.channels,
        sample_rate: sound.sample_rate,
        length_ms,
    })
}

/// The first four bytes of every FLAC file.
#[cfg(feature = "flac")]
const FLAC_MAGIC: &[u8; 4] = b"fLaC";

/// A FLAC file's (channels, sample rate, samples a channel if known), from
/// its `STREAMINFO`, read by `claxon`; `None` if it isn't one.
#[cfg(feature = "flac")]
fn flac_format(bytes: &[u8]) -> Option<(u16, u32, Option<u64>)> {
    let options = claxon::FlacReaderOptions {
        metadata_only: true,
        read_vorbis_comment: false,
    };
    let info = claxon::FlacReader::new_ext(bytes, options)
        .ok()?
        .streaminfo();
    let channels = u16::try_from(info.channels).ok()?;
    Some((channels, info.sample_rate, info.samples))
}

/// FLAC (`flac`, `.flac`), read for a sound, with the `flac` feature: its
/// `STREAMINFO`, by `claxon`.
#[cfg(feature = "flac")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flac;

#[cfg(feature = "flac")]
impl Format for Flac {
    type Asset = Sound;

    fn id(&self) -> &'static str {
        "flac"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["flac"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(FLAC_MAGIC)
    }

    fn max_bytes(&self) -> usize {
        MAX_SOUND_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Sound, Vec<ContentError>> {
        let format = flac_format(bytes).map(|(channels, rate, _)| (channels, rate));
        sound(file, format, "not a FLAC file")
    }
}

/// FLAC (`flac`, `.flac`), read for a music track, with the `flac`
/// feature: its `STREAMINFO`, whose sample count gives its length.
#[cfg(feature = "flac")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FlacTrack;

#[cfg(feature = "flac")]
impl Format for FlacTrack {
    type Asset = MusicTrack;

    fn id(&self) -> &'static str {
        "flac"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["flac"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(FLAC_MAGIC)
    }

    fn max_bytes(&self) -> usize {
        MAX_SOUND_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<MusicTrack, Vec<ContentError>> {
        let sound = Flac.decode(file, bytes, cx)?;
        let samples = flac_format(bytes).and_then(|(_, _, samples)| samples);
        track(file, sound, samples, NO_FLAC_LENGTH)
    }
}

/// Whether `bytes` start as an MP3 file: an `ID3v2` tag, or a frame's sync
/// (eleven bits set).
#[cfg(feature = "mp3")]
fn mp3_sniff(bytes: &[u8]) -> bool {
    bytes.starts_with(b"ID3") || matches!(bytes, [0xff, second, ..] if second & 0xe0 == 0xe0)
}

/// An MP3 file's (channels, sample rate, samples a channel if known), read
/// by `nanomp3` past its tags; `None` if it has no frame. With `scan`, it
/// walks every frame for the length (gapless: the encoder's delay and
/// padding off, when its LAME tag says them); without, it reads the
/// first frame only.
#[cfg(feature = "mp3")]
fn mp3_format(bytes: &[u8], scan: bool) -> Option<(u16, u32, Option<u64>)> {
    let options = nanomp3::Options::default().skip_scan(!scan);
    let reader = nanomp3::SliceReader::<i16>::with_options(bytes, options);
    let channels = match reader.channels()? {
        nanomp3::Channels::Mono => 1,
        nanomp3::Channels::Stereo => 2,
    };
    Some((channels, reader.sample_rate(), reader.total_samples()))
}

/// MP3 (`mp3`, `.mp3`), read for a sound, with the `mp3` feature: its
/// first frame's header, by `nanomp3`.
#[cfg(feature = "mp3")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mp3;

#[cfg(feature = "mp3")]
impl Format for Mp3 {
    type Asset = Sound;

    fn id(&self) -> &'static str {
        "mp3"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mp3"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        mp3_sniff(bytes)
    }

    fn max_bytes(&self) -> usize {
        MAX_SOUND_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Sound, Vec<ContentError>> {
        let format = mp3_format(bytes, false).map(|(channels, rate, _)| (channels, rate));
        sound(file, format, "not an MP3 file")
    }
}

/// MP3 (`mp3`, `.mp3`), read for a music track, with the `mp3` feature:
/// its first frame's header, and every frame for its length, without the
/// silence its encoder added at the ends when its LAME tag says how much.
#[cfg(feature = "mp3")]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mp3Track;

#[cfg(feature = "mp3")]
impl Format for Mp3Track {
    type Asset = MusicTrack;

    fn id(&self) -> &'static str {
        "mp3"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["mp3"]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        mp3_sniff(bytes)
    }

    fn max_bytes(&self) -> usize {
        MAX_SOUND_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<MusicTrack, Vec<ContentError>> {
        let sound = Mp3.decode(file, bytes, cx)?;
        let samples = mp3_format(bytes, true).and_then(|(_, _, samples)| samples);
        track(file, sound, samples, NO_MP3_LENGTH)
    }
}

impl AssetKind for AudioManifest {
    const NAME: &'static str = "audio manifest";
}

/// The audio manifest (`audio-manifest`, `.ron`), checked as
/// [`from_source`] says. Its sound files are named from its own folder and
/// read from the bundle beside it (`audio/audio.ron` names `sfx/a.wav`,
/// `audio/sfx/a.wav`). The music folder isn't in the bundle, so its tracks
/// aren't read here: a tool that can see the folder checks them with
/// [`from_source`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AudioManifestRon;

impl Format for AudioManifestRon {
    type Asset = AudioManifest;

    fn id(&self) -> &'static str {
        "audio-manifest"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ron"]
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<AudioManifest, Vec<ContentError>> {
        let source = text(file, bytes)?;
        let path = |name: &str| join(file.dir(), name);
        let files = Files {
            sound: &|name| cx.bytes(&path(name)),
            sound_shown: &|name| cx.shown(&path(name)),
            sound_left_out: &|name| cx.left_out(&path(name)),
            music_dir: "",
            music: None,
        };
        from_source(file.shown(), source, &files)
    }
}
