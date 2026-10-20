//! The audio kinds and formats (ADR-0007): a sound file's header, a music
//! track's, and the manifest. Only headers are read; the platform decodes
//! the samples when it plays them.
use crate::prelude::*;
use serde::{Deserialize, Serialize};

use alloc::borrow::Cow;

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sound {
    /// 1 or 2.
    pub channels: u16,
    /// Samples a second.
    pub sample_rate: u32,
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
            sound: &|name| cx.bytes(&path(name)).map(Cow::Borrowed),
            sound_shown: &|name| cx.shown(&path(name)),
            sound_left_out: &|name| cx.left_out(&path(name)),
            music_dir: "",
            music: None,
        };
        from_source(file.shown(), source, &files)
    }
}
