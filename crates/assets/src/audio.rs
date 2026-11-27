//! The audio manifest: every sound and music cue a game can play, the
//! music pools, and the credits for every third-party work. Screens name
//! cues; the platform layer plays the files.
//!
//! The game says where the files are ([`Files`]): sounds are usually in
//! the bundle, and music in a folder shipped next to the game, which the
//! game itself may not be able to read. [`from_source`] checks what it can
//! see, including each track's `length_ms` against its file
//! ([`track_length_ms`]).
//!
//! As formats (ADR-0007): a sound file's header is the [`Sound`] kind
//! ([`Wav`], [`OggVorbis`]), a music track's the [`MusicTrack`] kind
//! ([`OggVorbisTrack`]), and the manifest is read by
//! [`AudioManifestRon`]. FLAC and MP3 are formats of both kinds too, each
//! behind its own feature (`flac`, `mp3`): `Flac` and
//! `FlacTrack`, `Mp3` and `Mp3Track`. A sound may be a `.ogg`, `.wav`,
//! `.flac` or `.mp3` file, a track a `.ogg`, `.flac` or `.mp3` one. The
//! platform plays a FLAC or MP3 sound as the WAV [`decode_to_wav`] makes
//! of it. With the `sfx` feature a sound may also be a
//! `.sfx.ron` file, a sound effect written as a few numbers (`sfx`),
//! which the sound hook renders to a WAV once as the game
//! loads it ([`playable`]).
//!
//! Every cue plays in a sound group (ADR-0091): the one its `group` names,
//! else a sound in `ui` if it is a menu's and in `sfx` if not, and a track
//! in `music`; a voice clip plays in `voice`. The built-in groups' ids are
//! in [`groups`]. A game declares its own in the manifest's `groups` list,
//! each under a parent (`master` if it names none) and with the volume a
//! new player starts at, and sets a built-in group's starting volume there
//! too:
//!
//! ```ron
//! groups: [
//!     (id: "ambience", parent: "sfx", volume: 80),
//!     (id: "music", volume: 90),
//! ],
//! sounds: [
//!     (id: "rain", files: ["sfx/rain.ogg"], volume: 60, credit: Own, group: "ambience"),
//! ],
//! ```
//!
//! A `ducks` list lowers a group while a voice plays (ADR-0099), so the
//! words are heard over the music: down to a position on the loudness
//! curve over `attack_ms`, and back over `release_ms` once the voice ends
//! ([`DuckRule`]). A manifest without it lowers nothing.
//!
//! ```ron
//! ducks: [
//!     (when: "voice", lower: "music", to: 80, attack_ms: 100, release_ms: 500),
//! ],
//! ```
use crate::prelude::*;

mod decode;
mod formats;
pub mod groups;

use alloc::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::{ContentError, line_of};
use crate::ron::parse_ron_implicit_some;

pub use decode::{MAX_DECODED_BYTES, decode_to_wav, pcm16_wav, track_length_ms};
pub use formats::{
    AudioManifestRon, MAX_SOUND_BYTES, MusicTrack, OggVorbis, OggVorbisTrack, Sound, Wav,
};
#[cfg(feature = "flac")]
pub use formats::{Flac, FlacTrack};
#[cfg(feature = "mp3")]
pub use formats::{Mp3, Mp3Track};
pub use groups::{DuckRule, GroupDef};

/// Licenses a credit may have. `Own` is work we made.
pub const LICENSES: [&str; 4] = ["CC0-1.0", "CC-BY-3.0", "CC-BY-4.0", OWN];

/// The license of work we made ourselves.
pub const OWN: &str = "Own";

/// Highest cue volume, in percent of the file's own loudness.
pub const MAX_VOLUME: u8 = 100;

/// How far a music cue's `length_ms` may be from its file's real length,
/// in milliseconds.
pub const LENGTH_TOLERANCE_MS: u32 = 20;

/// The validated audio manifest. Build it with `default()` and assign
/// its fields (ADR-0039). Its file may declare sound groups, as `groups:
/// [(id: "ambience", parent: "sfx", volume: 80)]`, and a cue name one with
/// `group: "ambience"`, and lower a group while a voice plays, as `ducks:
/// [(when: "voice", lower: "music", to: 80, attack_ms: 100, release_ms:
/// 500)]` (the module's docs).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AudioManifest {
    /// How long a music track fades out on a switch or a stop, in
    /// milliseconds (`audio.md`: 0.5 s, *tunable*): the default
    /// [`MusicSwitch`]'s `out_ms`.
    pub music_fade_ms: u32,
    /// How long the next track fades in, in milliseconds (0, the file's
    /// default: it starts at full gain): the default switch's `in_ms`.
    pub music_fade_in_ms: u32,
    /// Whether the next track starts as the old one begins to fade (a
    /// crossfade) rather than once it has stopped (`false`, the file's
    /// default): the default switch's `overlap`.
    pub music_overlap: bool,
    /// Sound cues by id.
    pub sounds: BTreeMap<String, SoundCue>,
    /// Music cues by id.
    pub music: BTreeMap<String, MusicCue>,
    /// Music pools by id: the music cues one is picked from.
    pub pools: BTreeMap<String, Vec<String>>,
    /// Credits by id.
    pub credits: BTreeMap<String, Credit>,
    /// The sound groups the file declares, by id: a game's own, and the
    /// built-in ones whose default it sets. The built-ins it doesn't name
    /// aren't here ([`groups::BUILT_IN`]).
    pub groups: BTreeMap<String, GroupDef>,
    /// The ducking rules (ADR-0099), in the file's order: a group lowered
    /// while a voice plays. None by default, and nothing is lowered.
    pub ducks: Vec<DuckRule>,
    /// At most this many copies of all sound cues sound at once (ADR-0139
    /// §4): a play past it takes the voice of the copy with the lowest
    /// [`priority`](CueRules::priority), if that is at most its own, and is
    /// dropped otherwise. `None` (the file's default) is no limit. Never 0.
    /// Music, voice clips and sounds started with a handle don't count.
    #[serde(default)]
    pub max_voices: Option<u16>,
}

impl AudioManifest {
    /// How one music track gives way to the next when a request names no
    /// switch: the manifest's fade-out, fade-in and overlap.
    #[must_use]
    pub const fn music_switch(&self) -> MusicSwitch {
        MusicSwitch::after_fade(self.music_fade_ms)
            .with_in_ms(self.music_fade_in_ms)
            .with_overlap(self.music_overlap)
    }

    /// Measures how long each sound cue's files play into its
    /// [`lengths_ms`](SoundCue::lengths_ms), each file read by `read` by
    /// its name in the manifest (`sfx/hit.wav`): for a manifest built in
    /// code, which [`from_source`] didn't check. A file `read` doesn't
    /// find, or whose length can't be told ([`sound_length_ms`]), is
    /// `None`.
    pub fn measure_sounds(&mut self, read: &dyn Fn(&str) -> Bytes) {
        for cue in self.sounds.values_mut() {
            cue.lengths_ms = cue
                .files
                .iter()
                .map(|file| read(file).and_then(|bytes| sound_length_ms(file, &bytes)))
                .collect();
        }
    }
}

/// How one music track gives way to the next (ADR-0054): the old one
/// fades out over `out_ms`, and the new one fades in over `in_ms`,
/// starting as the old one begins to fade (`overlap`, a crossfade) or once
/// it has stopped. Build it with [`after_fade`](Self::after_fade) or
/// [`crossfade`](Self::crossfade), and change it with
/// [`with_in_ms`](Self::with_in_ms) and [`with_overlap`](Self::with_overlap)
/// (ADR-0039; no constructor takes both lengths, which a call could swap:
/// ADR-0151).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MusicSwitch {
    /// How long the old track fades out, in milliseconds.
    pub out_ms: u32,
    /// How long the new track fades in, in milliseconds (0: it starts at
    /// full gain).
    pub in_ms: u32,
    /// Whether the new track starts when the old one begins to fade (a
    /// crossfade), or after it has stopped.
    pub overlap: bool,
}

impl MusicSwitch {
    /// The old track fades out over `out_ms`, then the new one starts at
    /// full gain: the engine's switch before crossfades.
    #[must_use]
    pub const fn after_fade(out_ms: u32) -> Self {
        Self {
            out_ms,
            in_ms: 0,
            overlap: false,
        }
    }

    /// A crossfade: the new track fades in over `ms` while the old one
    /// fades out over the same time.
    #[must_use]
    pub const fn crossfade(ms: u32) -> Self {
        Self::after_fade(ms).with_in_ms(ms).with_overlap(true)
    }

    /// The same switch, with the new track fading in over `in_ms` (0: it
    /// starts at full gain).
    #[must_use]
    pub const fn with_in_ms(mut self, in_ms: u32) -> Self {
        self.in_ms = in_ms;
        self
    }

    /// The same switch, with the new track starting as the old one begins
    /// to fade (`true`, a crossfade) or once it has stopped.
    #[must_use]
    pub const fn with_overlap(mut self, overlap: bool) -> Self {
        self.overlap = overlap;
        self
    }
}

/// A sound effect: one of its files plays each time. Build it with
/// [`SoundCue::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SoundCue {
    /// The variants, relative to `base/audio/`; one is picked at random
    /// each time the cue plays. At least one.
    pub files: Vec<String>,
    /// Volume in percent (0–100).
    pub volume: u8,
    /// Who made it.
    pub credit: CreditRef,
    /// Whether it loops until it is stopped (`false`, the file's default:
    /// it plays once). A looped cue is started with a handle
    /// (`AudioQueue::start_sound` in `shuyi-ui`), which stops it.
    pub looped: bool,
    /// The sound group it plays in: `None` is `ui` for a menu's cue and
    /// `sfx` for the others.
    pub group: Option<String>,
    /// How each play picks its file and volume, and how soon it may play
    /// again ([`CueRules`], ADR-0139): [`CueRules::DEFAULT`], a file at
    /// random at `volume` every time, unless the manifest says otherwise.
    #[serde(default)]
    pub rules: CueRules,
    /// How long each file plays, in milliseconds, by its place in `files`
    /// (ADR-0139 §4): measured when the manifest is checked
    /// ([`sound_length_ms`]) or by [`AudioManifest::measure_sounds`], and
    /// never written in its file. `None`, or no entry (a list shorter than
    /// `files`), for a file whose length is unknown: one this build leaves
    /// out, one not found or not measured. Read it with
    /// [`length_ms`](Self::length_ms).
    #[serde(default)]
    pub lengths_ms: Vec<Option<u32>>,
}

impl SoundCue {
    /// The sound of `files` (one picked each time), at `volume` percent,
    /// made by `credit`, played once, with the default rules.
    #[must_use]
    pub const fn new(files: Vec<String>, volume: u8, credit: CreditRef) -> Self {
        Self {
            files,
            volume,
            credit,
            looped: false,
            group: None,
            rules: CueRules::DEFAULT,
            lengths_ms: Vec::new(),
        }
    }

    /// How long its file `file` (an index into `files`) plays, in
    /// milliseconds, if that is known ([`lengths_ms`](Self::lengths_ms)).
    #[must_use]
    pub fn length_ms(&self, file: usize) -> Option<u32> {
        self.lengths_ms.get(file).copied().flatten()
    }

    /// The same cue, played by `rules`.
    #[must_use]
    pub const fn with_rules(mut self, rules: CueRules) -> Self {
        self.rules = rules;
        self
    }

    /// The same cue, looped until it is stopped or not.
    #[must_use]
    pub const fn with_looped(mut self, looped: bool) -> Self {
        self.looped = looped;
        self
    }

    /// The same cue, played in sound group `group`.
    #[must_use]
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

/// How a sound cue's plays are picked and spaced (ADR-0139): which file
/// each plays, at what volume, and how soon after the last it may play.
/// `App` keeps them (`shuyi_platform::audio::cues`), so every backend
/// plays the same. In the manifest they are written flat on the sound,
/// each left out for its default:
///
/// ```ron
/// (id: "step", files: ["sfx/step_1.ogg", "sfx/step_2.ogg", "sfx/step_3.ogg"],
///  volume: 60, credit: Own, pick: NoRepeat, volume_min: 45, min_gap_ms: 60),
/// (id: "hit", files: ["sfx/hit_1.wav", "sfx/hit_2.wav"], volume: 80,
///  credit: Own, max_copies: 3, when_full: StopOldest, priority: 200),
/// ```
///
/// Config (ADR-0039, kind 2): start from [`CueRules::DEFAULT`] and change
/// it with the `with_` methods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CueRules {
    /// How each play picks its file ([`Pick::Random`] by default).
    pub pick: Pick,
    /// The lowest volume a play picks, in percent: each play picks a whole
    /// percent from this to the cue's `volume`. `None` (the default) plays
    /// at `volume` every time.
    pub volume_min: Option<u8>,
    /// A play less than this many milliseconds after the cue's last play
    /// that sounded is dropped (0, the default: none is).
    pub min_gap_ms: u32,
    /// At most this many copies of the cue sound at once, each until its
    /// file's length has passed ([`SoundCue::lengths_ms`]); `None` (the
    /// default) is no cap. Never 0.
    #[serde(default)]
    pub max_copies: Option<u8>,
    /// What a play does when `max_copies` copies sound ([`WhenFull::Skip`]
    /// by default: it is dropped).
    #[serde(default)]
    pub when_full: WhenFull,
    /// Which copies lose their voice when the manifest's `max_voices` sound:
    /// higher wins (128 by default).
    #[serde(default = "default_priority")]
    pub priority: u8,
}

/// A cue's priority when it names none: the middle.
pub const DEFAULT_PRIORITY: u8 = 128;

/// [`DEFAULT_PRIORITY`], for serde.
const fn default_priority() -> u8 {
    DEFAULT_PRIORITY
}

impl Default for CueRules {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl CueRules {
    /// The rules of a cue that names none: a file at random, at the cue's
    /// volume, with no gap, no cap on its copies, and the middle priority.
    /// Equal to `CueRules::default()`, for a `const`.
    pub const DEFAULT: Self = Self {
        pick: Pick::Random,
        volume_min: None,
        min_gap_ms: 0,
        max_copies: None,
        when_full: WhenFull::Skip,
        priority: DEFAULT_PRIORITY,
    };

    /// The same rules, picking files as `pick` says.
    #[must_use]
    pub const fn with_pick(mut self, pick: Pick) -> Self {
        self.pick = pick;
        self
    }

    /// The same rules, each play at a volume from `volume_min` up to the
    /// cue's (`None`: at the cue's).
    #[must_use]
    pub const fn with_volume_min(mut self, volume_min: Option<u8>) -> Self {
        self.volume_min = volume_min;
        self
    }

    /// The same rules, dropping a play less than `min_gap_ms` after the
    /// last that sounded.
    #[must_use]
    pub const fn with_min_gap_ms(mut self, min_gap_ms: u32) -> Self {
        self.min_gap_ms = min_gap_ms;
        self
    }

    /// The same rules, at most `max_copies` copies sounding at once
    /// (`None`: no cap), and `when_full` saying what a play past it does.
    #[must_use]
    pub const fn with_max_copies(mut self, max_copies: Option<u8>, when_full: WhenFull) -> Self {
        self.max_copies = max_copies;
        self.when_full = when_full;
        self
    }

    /// The same rules, at `priority` against the other cues' copies for a
    /// voice (higher wins).
    #[must_use]
    pub const fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

/// What a play of a cue does when [`CueRules::max_copies`] of its copies
/// sound already. A later engine may add a rule, so a `match` keeps a `_`
/// arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum WhenFull {
    /// The new play is dropped.
    #[default]
    Skip,
    /// The cue's oldest copy is cut and the new one plays: a footstep with
    /// `max_copies: 1` restarts.
    StopOldest,
}

/// How a sound cue picks the file each play plays. A cue of one file plays
/// it every time, whatever its pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Pick {
    /// Any file, at random each time: the same one may play twice in a row.
    #[default]
    Random,
    /// At random, but never the file that played last.
    NoRepeat,
    /// Each file once, in a random order, then a new order, never starting
    /// with the file that ended the last.
    Shuffle,
    /// The files in the order they are listed, then from the first again.
    InOrder,
}

/// A music track. Data read from a file (ADR-0039, kind 5): build it with
/// [`MusicCue::new`], and a field added later is `#[serde(default)]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct MusicCue {
    /// The track, relative to the `music/` folder.
    pub file: String,
    /// Volume in percent (0–100).
    pub volume: u8,
    /// Whether it loops (the default) or plays once.
    pub looped: bool,
    /// How long the track plays, in milliseconds (never 0): where the music
    /// clock wraps a looped track. The game can't measure it
    /// (the player library gives no length), so it is data, checked
    /// against the file by a test.
    pub length_ms: u32,
    /// Who made it.
    pub credit: CreditRef,
    /// The sound group it plays in: `None` is `music`.
    pub group: Option<String>,
    /// Where a looped track's region starts, in milliseconds:
    /// it plays from its start up to [`loop_end_ms`](Self::loop_end_ms)
    /// once, then from here to there again and again. With `loop_end_ms`
    /// or not at all: [`loop_region`](Self::loop_region) gives both.
    /// `None` loops the whole track, or the region its file names (an Ogg
    /// file's `LOOPSTART` and `LOOPLENGTH` comments), which these replace.
    #[serde(default)]
    pub loop_start_ms: Option<u32>,
    /// Where a looped track's region ends and it comes round, in
    /// milliseconds: no later than `length_ms`.
    #[serde(default)]
    pub loop_end_ms: Option<u32>,
}

/// Part of a track, from `start_ms` up to, not including, `end_ms`: a
/// looped track plays what comes before it once, then goes round it.
/// A value (ADR-0039, kind 3): build it with
/// [`LoopRegion::new`].
///
/// **Frames.** A backend plays it in frames of the file: a millisecond `ms`
/// is frame `ms × rate / 1000`, rounded down ([`frames`](Self::frames)),
/// so a region's end is never past what the milliseconds say. A region a
/// file names in frames is played exactly as the file says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct LoopRegion {
    /// Its first millisecond.
    pub start_ms: u32,
    /// The millisecond after its last: where the track comes round.
    pub end_ms: u32,
}

impl LoopRegion {
    /// From `start_ms` up to `end_ms`, if that is any time at all.
    #[must_use]
    pub const fn new(start_ms: u32, end_ms: u32) -> Option<Self> {
        if start_ms < end_ms {
            Some(Self { start_ms, end_ms })
        } else {
            None
        }
    }

    /// Its first frame and the frame after its last, in a file of `rate`
    /// frames a second: each millisecond rounded down onto a frame. `None`
    /// if they come to the same frame (a region shorter than a frame).
    #[must_use]
    pub fn frames(self, rate: u32) -> Option<(u64, u64)> {
        // u32 × u32 fits a u64.
        let frame = |ms: u32| u64::from(ms) * u64::from(rate) / 1000;
        let (start, end) = (frame(self.start_ms), frame(self.end_ms));
        (start < end).then_some((start, end))
    }
}

impl MusicCue {
    /// The track `file`, `length_ms` long, made by `credit`, at full
    /// volume (100 percent: [`with_volume`](Self::with_volume) changes it),
    /// looped, in the `music` group, round the whole track (or the region
    /// its file names).
    #[must_use]
    pub const fn new(file: String, length_ms: u32, credit: CreditRef) -> Self {
        Self {
            file,
            volume: 100,
            looped: true,
            length_ms,
            credit,
            group: None,
            loop_start_ms: None,
            loop_end_ms: None,
        }
    }

    /// The same track, at `volume` percent (0–100).
    #[must_use]
    pub const fn with_volume(mut self, volume: u8) -> Self {
        self.volume = volume;
        self
    }

    /// The same track, looping round `region` after its intro.
    #[must_use]
    pub const fn with_loop(mut self, region: LoopRegion) -> Self {
        self.loop_start_ms = Some(region.start_ms);
        self.loop_end_ms = Some(region.end_ms);
        self
    }

    /// The region the cue loops round, if it names one: `None` too for a
    /// cue that names only one end, or an empty region, which the
    /// manifest's check refuses.
    #[must_use]
    pub const fn loop_region(&self) -> Option<LoopRegion> {
        match (self.loop_start_ms, self.loop_end_ms) {
            (Some(start), Some(end)) => LoopRegion::new(start, end),
            _ => None,
        }
    }

    /// The same track, looped or played once.
    #[must_use]
    pub const fn with_looped(mut self, looped: bool) -> Self {
        self.looped = looped;
        self
    }

    /// The same track, played in sound group `group`.
    #[must_use]
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }
}

fn yes() -> bool {
    true
}

/// Who made a sound or a track.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum CreditRef {
    /// We made it.
    Own,
    /// A third-party work: the id of its entry in `credits`.
    Credit(String),
    /// Several third-party works, e.g. a sound whose variants come from
    /// different authors: their ids in `credits`.
    Credits(Vec<String>),
}

impl CreditRef {
    /// The ids in `credits` this names (none for [`CreditRef::Own`]).
    pub fn ids(&self) -> &[String] {
        match self {
            Self::Own => &[],
            Self::Credit(id) => core::slice::from_ref(id),
            Self::Credits(ids) => ids,
        }
    }
}

/// One third-party work, for the credits screen (`audio.md` rule 3).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Credit {
    /// The work's title.
    pub title: String,
    /// Its author, as they want to be credited.
    pub author: String,
    /// The page it came from.
    pub source: String,
    /// One of [`LICENSES`].
    pub license: String,
    /// Free tags (`music`, `sfx`, the cue, a mood, a place …).
    pub tags: Vec<String>,
    /// Which file or version we use, e.g. "V2 file".
    pub note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawFile {
    music_fade_ms: u32,
    #[serde(default)]
    music_fade_in_ms: u32,
    #[serde(default)]
    music_overlap: bool,
    #[serde(default)]
    groups: Vec<groups::RawGroup>,
    #[serde(default)]
    ducks: Vec<groups::RawDuck>,
    #[serde(default)]
    max_voices: Option<u16>,
    sounds: Vec<RawSound>,
    music: Vec<RawMusic>,
    pools: Vec<RawPool>,
    credits: Vec<RawCredit>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSound {
    id: String,
    files: Vec<String>,
    volume: u8,
    credit: CreditRef,
    #[serde(default)]
    looped: bool,
    #[serde(default)]
    group: Option<String>,
    // The cue rules, flat on the entry: `#[serde(flatten)]` doesn't work
    // with `deny_unknown_fields`.
    #[serde(default)]
    pick: Pick,
    #[serde(default)]
    volume_min: Option<u8>,
    #[serde(default)]
    min_gap_ms: u32,
    #[serde(default)]
    max_copies: Option<u8>,
    #[serde(default)]
    when_full: WhenFull,
    #[serde(default = "default_priority")]
    priority: u8,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawMusic {
    id: String,
    file: String,
    volume: u8,
    #[serde(default = "yes")]
    looped: bool,
    length_ms: u32,
    credit: CreditRef,
    #[serde(default)]
    group: Option<String>,
    #[serde(default)]
    loop_start_ms: Option<u32>,
    #[serde(default)]
    loop_end_ms: Option<u32>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCredit {
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    source: String,
    license: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPool {
    id: String,
    music: Vec<String>,
}

/// A file's contents, or `None` if it's missing.
pub type Bytes = Option<alloc::borrow::Cow<'static, [u8]>>;

/// How to read the referenced files, and how errors name them. `music` is
/// `None` where the music folder can't be seen (the game itself, which
/// does no file I/O). Build it with [`Files::new`] (ADR-0039).
#[non_exhaustive]
pub struct Files<'a> {
    /// Reads a sound file, by its name in the manifest (e.g. `sfx/x.wav`).
    pub sound: &'a dyn Fn(&str) -> Bytes,
    /// A sound file's name as errors show it, by its name in the manifest
    /// (e.g. `base/audio/sfx/x.wav`).
    pub sound_shown: &'a dyn Fn(&str) -> String,
    /// Whether this build leaves a sound file out on purpose, by its name
    /// in the manifest (a bundle's `left_out`): it is then not checked.
    pub sound_left_out: &'a dyn Fn(&str) -> bool,
    /// The music folder's name as errors show it (e.g. `music`).
    pub music_dir: &'a str,
    /// Reads a file in the music folder.
    pub music: Option<&'a dyn Fn(&str) -> Bytes>,
}

impl<'a> Files<'a> {
    /// Sound files read by `sound`, named in errors by `sound_shown`, left
    /// out where `sound_left_out` says, and a music folder shown as
    /// `music_dir` that can't be seen (`music` is `None`:
    /// [`with_music`](Self::with_music) changes it).
    #[must_use]
    pub const fn new(
        sound: &'a dyn Fn(&str) -> Bytes,
        sound_shown: &'a dyn Fn(&str) -> String,
        sound_left_out: &'a dyn Fn(&str) -> bool,
        music_dir: &'a str,
    ) -> Self {
        Self {
            sound,
            sound_shown,
            sound_left_out,
            music_dir,
            music: None,
        }
    }

    /// The files with the music folder read by `music`: usable in a
    /// `const` item.
    #[must_use]
    pub const fn with_music(self, music: Option<&'a dyn Fn(&str) -> Bytes>) -> Self {
        Self { music, ..self }
    }
}

/// Where a sound file goes in a package whose target bakes its audio (the
/// `.pdx`): `sfx/x.wav` is `audio/sfx/x` there.
pub const BAKED_SOUND_DIR: &str = "audio";

/// Where a music file goes in a package whose target bakes its audio.
pub const BAKED_MUSIC_DIR: &str = "music";

/// `file` without its extension, as a device that bakes its audio opens it.
#[must_use]
pub fn without_extension(file: &str) -> &str {
    let name = file.rfind('/').unwrap_or(0);
    match file[name..].rfind('.') {
        Some(dot) => &file[..name + dot],
        None => file,
    }
}

/// The sample rate every file must have: quad-snd (native) plays at
/// 44.1 kHz and resamples anything else crudely.
pub const SAMPLE_RATE: u32 = 44_100;

/// The end of a sound effect's file name (`sfx/coin.sfx.ron`).
pub const SFX_SUFFIX: &str = ".sfx.ron";

/// Whether `name` is a sound effect's: it ends in [`SFX_SUFFIX`], after a
/// name.
pub(crate) fn is_sfx(name: &str) -> bool {
    let file = name.rsplit_once('/').map_or(name, |(_, file)| file);
    file.len() > SFX_SUFFIX.len() && file.ends_with(SFX_SUFFIX)
}

/// Why `bytes` (a file named `name`) isn't something the game can play:
/// its content must match its extension (OGG Vorbis for `.ogg`, PCM or
/// float WAV for `.wav`, FLAC for `.flac`, MP3 for `.mp3`, a sound
/// effect's numbers for `.sfx.ron`), with one or two channels at
/// [`SAMPLE_RATE`]. A `.flac`, `.mp3` or `.sfx.ron` file whose bytes are a
/// WAV (a game's hook decoded or rendered them) is checked as a WAV, and
/// one whose format's feature is off (`flac`, `mp3`, `sfx`) is told to
/// turn it on. A `.sfx.ron` file's first problem is given with its line
/// (`line 3: hz 30000 is over 12000`). quad-snd panics on native, and
/// never finishes loading on the web, on a file it can't read, so this
/// runs before any file ships.
pub fn format_problem(name: &str, bytes: &[u8]) -> Option<String> {
    if is_sfx(name) {
        return decode::sfx_problem(bytes, decode::SFX_PROBLEM);
    }
    if decode::is_coded(name) {
        return decode::coded_problem(name, bytes);
    }
    if has_extension(name, &["wav"]) {
        return wav_problem(bytes);
    }
    match vorbis_format(bytes) {
        None => Some("not an OGG Vorbis file".to_owned()),
        Some((channels, rate)) => playable_problem(channels, rate),
    }
}

/// The bytes the platform plays for the sound file `name`, whose bytes are
/// `bytes`: with the `sfx` feature, a `.sfx.ron` file whose numbers read
/// is rendered to its WAV ([`sfx::render`](crate::sfx), `sfx::to_wav`).
/// Any other name, a file that doesn't read, and every file without the
/// feature give `bytes` unchanged, and the manifest's check
/// ([`format_problem`]) says why. Every sound hook goes through it, so a
/// game that turns `sfx` on gets its sound effects rendered as it loads
/// them.
pub fn playable(name: &str, bytes: Bytes) -> Bytes {
    if !is_sfx(name) {
        return bytes;
    }
    match bytes.as_deref().and_then(|b| decode_to_wav(name, b)) {
        Some(Ok(wav)) => Some(alloc::borrow::Cow::Owned(wav)),
        _ => bytes,
    }
}

/// Why `bytes` aren't a WAV the game can play, if they aren't.
fn wav_problem(bytes: &[u8]) -> Option<String> {
    match wav_format(bytes) {
        None => Some("not a PCM WAV file".to_owned()),
        Some((channels, rate)) => playable_problem(channels, rate),
    }
}

/// Why a sound of `channels` channels at `rate` Hz isn't something the
/// game can play, if it isn't: it must be mono or stereo, at
/// [`SAMPLE_RATE`].
fn playable_problem(channels: u16, rate: u32) -> Option<String> {
    if !(1..=2).contains(&channels) {
        Some(format!("{channels} channels; must be mono or stereo"))
    } else if rate != SAMPLE_RATE {
        Some(format!("{rate} Hz; must be {SAMPLE_RATE} Hz"))
    } else {
        None
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

/// Where the first chunk `want` of a WAV starts its body, and the size its
/// header gives; `None` if `bytes` aren't a WAV or have no such chunk.
fn wav_chunk(bytes: &[u8], want: [u8; 4]) -> Option<(usize, usize)> {
    if bytes.get(..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut at: usize = 12;
    loop {
        let id = bytes.get(at..at + 4)?;
        let size = usize::try_from(u32_at(bytes, at + 4)?).ok()?;
        if id == want.as_slice() {
            return Some((at + 8, size));
        }
        at = at.checked_add(size.checked_add(8 + size % 2)?)?;
    }
}

/// A WAV's (channels, sample rate), from its `fmt ` chunk.
fn wav_format(bytes: &[u8]) -> Option<(u16, u32)> {
    let (fmt, _) = wav_chunk(bytes, *b"fmt ")?;
    // 1 = integer PCM, 3 = float: what quad-snd's decoder reads.
    let encoding = u16_at(bytes, fmt)?;
    [1, 3]
        .contains(&encoding)
        .then_some((u16_at(bytes, fmt + 2)?, u32_at(bytes, fmt + 4)?))
}

/// How long a WAV plays, in milliseconds (to the nearest): its `data`
/// chunk's frames (its size over the block align of its `fmt ` chunk)
/// over its sample rate. A `data` chunk that says more than the file
/// holds counts what it holds. `None` with no `fmt ` or `data` chunk, or a
/// block align or a rate of 0.
fn wav_length_ms(bytes: &[u8]) -> Option<u32> {
    let (fmt, _) = wav_chunk(bytes, *b"fmt ")?;
    let rate = u32_at(bytes, fmt + 4)?;
    let block = u64::from(u16_at(bytes, fmt + 12)?);
    let (data, size) = wav_chunk(bytes, *b"data")?;
    let held = u64::try_from(size.min(bytes.len().saturating_sub(data))).ok()?;
    samples_ms(held.checked_div(block)?, rate)
}

/// How long the sound file `name`, whose bytes are `bytes`, plays, in
/// milliseconds (to the nearest), in integers: a WAV's (by its bytes,
/// whatever its name: a game's hook may have decoded or rendered it) from
/// its `data` chunk, a sound effect's (`.sfx.ron`, with the `sfx` feature)
/// the length it renders to, a FLAC's or an MP3's (by its name) as
/// [`track_length_ms`] reads it, and any other's as an OGG's, from its
/// last page ([`ogg_length_ms`]).
/// `None` when it can't be told: no `data` chunk, a block align or a rate
/// of 0, no last Ogg page, numbers that don't read, or a format whose
/// feature is off.
///
/// The cue rules end a copy of a sound at its start plus this (ADR-0139
/// §4): the manifest keeps it per file ([`SoundCue::lengths_ms`]).
#[must_use]
pub fn sound_length_ms(name: &str, bytes: &[u8]) -> Option<u32> {
    if decode::is_wav(bytes) {
        wav_length_ms(bytes)
    } else if is_sfx(name) {
        decode::sfx_length_ms(bytes)
    } else if decode::is_coded(name) {
        track_length_ms(name, bytes)
    } else {
        ogg_length_ms(bytes)
    }
}

/// An OGG Vorbis file's (channels, sample rate), from the identification
/// header in its first page.
fn vorbis_format(bytes: &[u8]) -> Option<(u16, u32)> {
    if bytes.get(..4)? != b"OggS" {
        return None;
    }
    let packet = 27 + usize::from(*bytes.get(26)?);
    if bytes.get(packet..packet + 7)? != b"\x01vorbis" {
        return None;
    }
    let channels = u16::from(*bytes.get(packet + 11)?);
    Some((channels, u32_at(bytes, packet + 12)?))
}

/// How long an OGG Vorbis file plays, in milliseconds (to the nearest): the
/// granule position of its last page, which for Vorbis is the number of
/// samples in the stream, over the sample rate in its header. `None` if
/// `bytes` isn't a whole OGG Vorbis file.
pub fn ogg_length_ms(bytes: &[u8]) -> Option<u32> {
    let (_, rate) = vorbis_format(bytes)?;
    samples_ms(last_granule(bytes)?, rate)
}

/// How long `samples` samples a channel play at `rate` Hz, in milliseconds
/// (to the nearest); `None` at 0 Hz or past `u32::MAX` ms.
pub(crate) fn samples_ms(samples: u64, rate: u32) -> Option<u32> {
    let rate = u64::from(rate);
    let ms = samples
        .checked_mul(1000)?
        .checked_add(rate / 2)?
        .checked_div(rate)?;
    u32::try_from(ms).ok()
}

/// The granule position of the Ogg page that ends `bytes`. The letters
/// `OggS` can also turn up inside a page's audio data, so the page must
/// run exactly to the end of the file. `None` too if no packet ends on
/// that page (its position is then all ones).
fn last_granule(bytes: &[u8]) -> Option<u64> {
    let ends_file = |&at: &usize| {
        bytes.get(at..at + 4) == Some(b"OggS")
            && ogg_page_len(bytes, at).and_then(|len| at.checked_add(len)) == Some(bytes.len())
    };
    let at = (0..bytes.len()).rev().find(ends_file)?;
    let granule = u64::from_le_bytes(bytes.get(at + 6..at + 14)?.try_into().ok()?);
    (granule != u64::MAX).then_some(granule)
}

/// The length of the Ogg page starting at `at`: its 27-byte header, its
/// segment table and the segments the table lists.
fn ogg_page_len(bytes: &[u8], at: usize) -> Option<usize> {
    let segments = usize::from(*bytes.get(at + 26)?);
    let table = bytes.get(at + 27..at + 27 + segments)?;
    let body: usize = table.iter().map(|&s| usize::from(s)).sum();
    Some(27 + segments + body)
}

/// Parses and validates manifest `source`, attributing errors to `file`.
pub fn from_source(
    file: &str,
    source: &str,
    files: &Files<'_>,
) -> Result<AudioManifest, Vec<ContentError>> {
    let raw: RawFile = parse_ron_implicit_some(file, source).map_err(|e| vec![e])?;
    let mut check = Check {
        file,
        source,
        files,
        errors: Vec::new(),
        cue_ids: BTreeSet::new(),
        manifest: AudioManifest {
            music_fade_ms: raw.music_fade_ms,
            music_fade_in_ms: raw.music_fade_in_ms,
            music_overlap: raw.music_overlap,
            max_voices: raw.max_voices,
            ..AudioManifest::default()
        },
    };
    if raw.max_voices == Some(0) {
        check.err_at(
            "max_voices:",
            "max_voices is 0: leave it out for no limit".into(),
        );
    }
    check.groups(raw.groups);
    check.ducks(raw.ducks);
    for c in raw.credits {
        check.credit(c);
    }
    for s in raw.sounds {
        check.sound(s);
    }
    for m in raw.music {
        check.music(m);
    }
    for p in raw.pools {
        check.pool(p);
    }
    if check.errors.is_empty() {
        Ok(check.manifest)
    } else {
        Err(check.errors)
    }
}

/// Validation state: the manifest built so far and the problems found.
struct Check<'a> {
    file: &'a str,
    source: &'a str,
    files: &'a Files<'a>,
    errors: Vec<ContentError>,
    /// Sound, music and pool ids seen (one namespace).
    cue_ids: BTreeSet<String>,
    manifest: AudioManifest,
}

impl Check<'_> {
    /// Records a problem with entry `id`, at the line that names it.
    fn err(&mut self, id: &str, message: String) {
        let e = ContentError::new(self.file, message);
        self.errors
            .push(match line_of(self.source, &format!("id: \"{id}\"")) {
                Some(l) => e.at(l, None),
                None => e,
            });
    }

    /// Records a problem at the first line that has `key` (one of the
    /// file's own fields), or with no line.
    fn err_at(&mut self, key: &str, message: String) {
        let e = ContentError::new(self.file, message);
        self.errors.push(match line_of(self.source, key) {
            Some(l) => e.at(l, None),
            None => e,
        });
    }

    fn new_cue(&mut self, id: &str) {
        if id.is_empty() {
            self.err(id, "a cue id is empty".into());
        }
        if !self.cue_ids.insert(id.to_owned()) {
            self.err(id, format!("duplicate cue id \"{id}\""));
        }
    }

    fn volume(&mut self, id: &str, volume: u8) {
        if volume > MAX_VOLUME {
            self.err(
                id,
                format!("\"{id}\": volume {volume} is over {MAX_VOLUME} (percent)"),
            );
        }
    }

    fn credit_ref(&mut self, id: &str, credit: &CreditRef) {
        if matches!(credit, CreditRef::Credits(ids) if ids.is_empty()) {
            self.err(id, format!("\"{id}\" has an empty credit list"));
        }
        for c in credit.ids() {
            if !self.manifest.credits.contains_key(c) {
                self.err(id, format!("\"{id}\" names unknown credit \"{c}\""));
            }
        }
    }

    fn credit(&mut self, c: RawCredit) {
        let id = c.id;
        let entry = Credit {
            title: c.title,
            author: c.author,
            source: c.source,
            license: c.license,
            tags: c.tags,
            note: c.note,
        };
        if self.manifest.credits.contains_key(&id) {
            self.err(&id, format!("duplicate credit id \"{id}\""));
        }
        for problem in credit_problems(&entry) {
            self.err(&id, format!("credit \"{id}\": {problem}"));
        }
        self.manifest.credits.insert(id, entry);
    }

    fn sound(&mut self, s: RawSound) {
        let id = s.id;
        self.new_cue(&id);
        if s.files.is_empty() {
            self.err(&id, format!("sound \"{id}\" has no files"));
        }
        let mut lengths_ms = Vec::with_capacity(s.files.len());
        for f in &s.files {
            let path = (self.files.sound_shown)(f);
            let mut length = None;
            if !has_extension(f, &SOUND_EXTENSIONS) && !is_sfx(f) {
                let why = "must be .ogg, .wav, .flac, .mp3 or .sfx.ron";
                self.err(&id, format!("sound \"{id}\": \"{f}\" {why}"));
            } else if !(self.files.sound_left_out)(f) {
                match (self.files.sound)(f) {
                    None => self.err(&id, format!("sound \"{id}\": file {path} not found")),
                    Some(bytes) => {
                        if let Some(why) = format_problem(f, &bytes) {
                            self.err(&id, format!("sound \"{id}\": {path}: {why}"));
                        }
                        length = sound_length_ms(f, &bytes);
                        let counted = s.max_copies.is_some() || self.manifest.max_voices.is_some();
                        if length.is_none() && counted {
                            let why = "its length can't be measured, and max_copies or max_voices counts its copies";
                            self.err(&id, format!("sound \"{id}\": {path}: {why}"));
                        }
                    }
                }
            }
            lengths_ms.push(length);
        }
        if s.max_copies == Some(0) {
            let why = "max_copies is 0: leave it out for no cap";
            self.err(&id, format!("sound \"{id}\": {why}"));
        }
        self.volume(&id, s.volume);
        if let Some(min) = s.volume_min.filter(|&min| min > s.volume) {
            let volume = s.volume;
            let why = format!("volume_min {min} is over its volume {volume}");
            self.err(&id, format!("sound \"{id}\": {why}"));
        }
        self.credit_ref(&id, &s.credit);
        self.cue_group("sound", &id, s.group.as_deref());
        let rules = CueRules {
            pick: s.pick,
            volume_min: s.volume_min,
            min_gap_ms: s.min_gap_ms,
            max_copies: s.max_copies,
            when_full: s.when_full,
            priority: s.priority,
        };
        let entry = SoundCue {
            files: s.files,
            volume: s.volume,
            credit: s.credit,
            looped: s.looped,
            group: s.group,
            rules,
            lengths_ms,
        };
        self.manifest.sounds.insert(id, entry);
    }

    fn music(&mut self, m: RawMusic) {
        let id = m.id;
        self.new_cue(&id);
        let f = &m.file;
        if !has_extension(f, &TRACK_EXTENSIONS) {
            let why = "must be .ogg, .flac or .mp3";
            self.err(&id, format!("music \"{id}\": \"{f}\" {why}"));
        } else if let Some(read) = self.files.music {
            let dir = self.files.music_dir;
            match read(f) {
                None => self.err(&id, format!("music \"{id}\": file {dir}/{f} not found")),
                Some(bytes) => {
                    let problem = format_problem(f, &bytes)
                        .or_else(|| length_problem(f, m.length_ms, track_length_ms(f, &bytes)));
                    if let Some(why) = problem {
                        self.err(&id, format!("music \"{id}\": {dir}/{f}: {why}"));
                    }
                }
            }
        }
        if m.length_ms == 0 {
            let why = "length_ms is 0; it is the track's length in milliseconds";
            self.err(&id, format!("music \"{id}\": {why}"));
        }
        if let Some(why) = loop_problem(m.loop_start_ms, m.loop_end_ms, m.length_ms, m.looped) {
            self.err(&id, format!("music \"{id}\": {why}"));
        }
        self.volume(&id, m.volume);
        self.credit_ref(&id, &m.credit);
        self.cue_group("music", &id, m.group.as_deref());
        let entry = MusicCue {
            file: m.file,
            volume: m.volume,
            looped: m.looped,
            length_ms: m.length_ms,
            credit: m.credit,
            group: m.group,
            loop_start_ms: m.loop_start_ms,
            loop_end_ms: m.loop_end_ms,
        };
        self.manifest.music.insert(id, entry);
    }

    fn pool(&mut self, RawPool { id, music }: RawPool) {
        self.new_cue(&id);
        if music.is_empty() {
            self.err(&id, format!("pool \"{id}\" is empty"));
        }
        for m in &music {
            if self.manifest.sounds.contains_key(m) {
                let why = format!("pool \"{id}\" names \"{m}\", a sound; pools hold music cues");
                self.err(&id, why);
            } else if !self.manifest.music.contains_key(m) {
                self.err(
                    &id,
                    format!("pool \"{id}\" names unknown music cue \"{m}\""),
                );
            }
        }
        self.manifest.pools.insert(id, music);
    }
}

/// The extensions a sound file may have, beside a sound effect's
/// [`SFX_SUFFIX`].
const SOUND_EXTENSIONS: [&str; 4] = ["ogg", "wav", "flac", "mp3"];

/// The extensions a music track may have.
const TRACK_EXTENSIONS: [&str; 3] = ["ogg", "flac", "mp3"];

/// What an OGG track whose length can't be read is told.
const NO_LENGTH: &str = "its length can't be read (no last Ogg page)";
/// What a FLAC track whose length can't be read is told.
const NO_FLAC_LENGTH: &str = "its length can't be read (its STREAMINFO has no sample count)";
/// What an MP3 track whose length can't be read is told.
const NO_MP3_LENGTH: &str = "its length can't be read (no frames)";

/// Why a music cue's `length_ms` doesn't fit its file `file`, which plays
/// for `actual` milliseconds ([`track_length_ms`]): more than
/// [`LENGTH_TOLERANCE_MS`] off, or the file's length can't be read (a
/// message for its format). The message gives the value to write.
fn length_problem(file: &str, length_ms: u32, actual: Option<u32>) -> Option<String> {
    match actual {
        None if has_extension(file, &["flac"]) => Some(NO_FLAC_LENGTH.into()),
        None if has_extension(file, &["mp3"]) => Some(NO_MP3_LENGTH.into()),
        None => Some(NO_LENGTH.into()),
        Some(ms) if ms.abs_diff(length_ms) > LENGTH_TOLERANCE_MS => Some(format!(
            "it plays for {ms} ms, but length_ms is {length_ms}; write length_ms: {ms}"
        )),
        Some(_) => None,
    }
}

/// Why a music cue's loop region isn't one: one end without the other, an
/// end not after its start, past the track's `length_ms`, or on a track
/// played once. Each is refused as written, never clamped.
fn loop_problem(
    start: Option<u32>,
    end: Option<u32>,
    length_ms: u32,
    looped: bool,
) -> Option<String> {
    let (start, end) = match (start, end) {
        (None, None) => return None,
        (Some(_), None) => return Some("loop_start_ms needs loop_end_ms too".into()),
        (None, Some(_)) => return Some("loop_end_ms needs loop_start_ms too".into()),
        (Some(start), Some(end)) => (start, end),
    };
    Some(if start >= end {
        format!("loop_end_ms {end} must be after loop_start_ms {start}")
    } else if end > length_ms {
        format!("loop_end_ms {end} is past the track's end; length_ms is {length_ms}")
    } else if !looped {
        "a loop region needs a looped track; this one has looped: false".into()
    } else {
        return None;
    })
}

/// What's wrong with a credit: an unknown license, or (for third-party
/// work) a missing title, author or source link.
fn credit_problems(c: &Credit) -> Vec<String> {
    let mut problems = Vec::new();
    if !LICENSES.contains(&c.license.as_str()) {
        problems.push(format!(
            "license \"{}\" is not allowed (ADR-0027: {})",
            c.license,
            LICENSES.join(", ")
        ));
    }
    if c.license != OWN {
        for (field, value) in [("title", &c.title), ("author", &c.author)] {
            if value.trim().is_empty() {
                problems.push(format!("{field} is missing"));
            }
        }
        if !c.source.starts_with("https://") && !c.source.starts_with("http://") {
            problems.push("source must be a web link (http:// or https://)".into());
        }
    }
    problems
}

fn has_extension(file: &str, allowed: &[&str]) -> bool {
    file.rsplit_once('.')
        .is_some_and(|(stem, ext)| !stem.is_empty() && allowed.contains(&ext))
}

#[cfg(test)]
pub(crate) mod tests {
    use alloc::borrow::Cow;

    use super::*;
    use crate::format::codec_tests::{FLAC, MP3};

    #[test]
    fn a_sounds_constructors_build_what_a_literal_built() {
        assert_eq!(
            Sound::mono(SAMPLE_RATE),
            Sound {
                channels: 1,
                sample_rate: SAMPLE_RATE,
                length_ms: None,
            }
        );
        assert_eq!(
            Sound::stereo(22_050).with_length_ms(Some(250)),
            Sound {
                channels: 2,
                sample_rate: 22_050,
                length_ms: Some(250),
            }
        );
    }

    #[test]
    fn a_music_cues_constructor_builds_what_a_literal_built() {
        assert_eq!(
            MusicCue::new("a.ogg".into(), 1000, CreditRef::Own).with_volume(80),
            MusicCue {
                file: "a.ogg".into(),
                volume: 80,
                looped: true,
                length_ms: 1000,
                credit: CreditRef::Own,
                group: None,
                loop_start_ms: None,
                loop_end_ms: None,
            }
        );
        let once = MusicCue::new("a.ogg".into(), 1000, CreditRef::Own)
            .with_volume(80)
            .with_looped(false)
            .with_group("weather");
        assert_eq!(
            (once.looped, once.group.as_deref()),
            (false, Some("weather"))
        );
        let region = LoopRegion::new(250, 750);
        let looped = region.map(|r| {
            MusicCue::new("a.ogg".into(), 1000, CreditRef::Own)
                .with_volume(80)
                .with_loop(r)
        });
        assert_eq!(
            looped.map(|m| (m.loop_start_ms, m.loop_end_ms, m.loop_region())),
            Some((Some(250), Some(750), region))
        );
    }

    #[test]
    fn a_loop_region_is_half_open_and_its_frames_round_down() {
        assert_eq!(LoopRegion::new(5, 5), None);
        assert_eq!(LoopRegion::new(6, 5), None);
        let region = LoopRegion::new(1000, 2000).unwrap_or_else(|| panic!("a region"));
        assert_eq!((region.start_ms, region.end_ms), (1000, 2000));
        assert_eq!(region.frames(44_100), Some((44_100, 88_200)));
        // 1 ms is 44.1 frames: both ends go down onto a frame.
        let odd = LoopRegion::new(1, 3).unwrap_or_else(|| panic!("a region"));
        assert_eq!(odd.frames(44_100), Some((44, 132)));
        // At 400 frames a second, 1 ms and 2 ms are both frame 0: no region.
        let short = LoopRegion::new(1, 2).unwrap_or_else(|| panic!("a region"));
        assert_eq!(short.frames(400), None);
        assert_eq!(short.frames(500), Some((0, 1)));
        // The largest of both fits.
        let wide = LoopRegion::new(0, u32::MAX).unwrap_or_else(|| panic!("a region"));
        let most = u64::from(u32::MAX) * u64::from(u32::MAX) / 1000;
        assert_eq!(wide.frames(u32::MAX), Some((0, most)));
    }

    #[test]
    fn a_cue_naming_one_end_has_no_region() {
        let mut m = MusicCue::new("a.ogg".into(), 1000, CreditRef::Own).with_volume(80);
        m.loop_start_ms = Some(10);
        assert_eq!(m.loop_region(), None);
        m.loop_end_ms = Some(10);
        assert_eq!(m.loop_region(), None, "empty");
        m.loop_end_ms = Some(11);
        assert_eq!(m.loop_region(), LoopRegion::new(10, 11));
    }

    #[test]
    fn without_extension_takes_the_last_names_extension_alone() {
        assert_eq!(without_extension("sfx/click.wav"), "sfx/click");
        assert_eq!(without_extension("click.a.ogg"), "click.a");
        assert_eq!(without_extension("v1.2/click"), "v1.2/click");
        assert_eq!(without_extension("deep/v1.2/x.ogg"), "deep/v1.2/x");
        assert_eq!(without_extension("click"), "click");
    }

    const EMPTY: &str = "(music_fade_ms: 500, sounds: [], music: [], pools: [], credits: [])";

    const FULL: &str = r#"(
    music_fade_ms: 250,
    sounds: [
        (id: "ui_tick", files: ["sfx/ui_tick.wav"], volume: 100, credit: Own),
        (id: "footfall", files: ["sfx/step_1.ogg", "sfx/step_2.ogg"], volume: 60, credit: Credit("steps")),
    ],
    music: [
        (id: "title", file: "title.ogg", volume: 80, length_ms: 2000, credit: Credit("sunrise")),
        (id: "sting", file: "sting.ogg", volume: 100, looped: false, length_ms: 2000, credit: Own),
    ],
    pools: [
        (id: "skirmish", music: ["title", "sting"]),
    ],
    credits: [
        (id: "sunrise", title: "New Sunrise", author: "nene",
         source: "https://opengameart.org/content/new-sunrise", license: "CC0-1.0",
         tags: ["music", "title"], note: "V1 file"),
        (id: "steps", title: "Footsteps", author: "someone",
         source: "https://freesound.org/s/1/", license: "CC-BY-4.0"),
        (id: "ours", license: "Own"),
    ],
)"#;

    /// The start of a WAV file: a `LIST` chunk (odd-sized, so padded), then
    /// `fmt `.
    pub(super) fn wav(channels: u16, rate: u32, encoding: u16) -> Vec<u8> {
        let mut w = b"RIFF\0\0\0\0WAVELIST\x05\0\0\0abcde\0fmt \x10\0\0\0".to_vec();
        w.extend_from_slice(&encoding.to_le_bytes());
        w.extend_from_slice(&channels.to_le_bytes());
        w.extend_from_slice(&rate.to_le_bytes());
        w.extend_from_slice(&[0; 8]);
        w
    }

    /// The first page of an OGG Vorbis file (two lacing values).
    fn ogg(channels: u8, rate: u32) -> Vec<u8> {
        let mut o = b"OggS".to_vec();
        o.extend_from_slice(&[0; 22]);
        o.extend_from_slice(&[2, 30, 0]);
        o.extend_from_slice(b"\x01vorbis\0\0\0\0");
        o.push(channels);
        o.extend_from_slice(&rate.to_le_bytes());
        o
    }

    /// An Ogg page after the first: `granule` in its header and `body` as
    /// its one segment.
    fn ogg_page(granule: u64, body: &[u8]) -> Vec<u8> {
        let mut p = b"OggS\0\x04".to_vec();
        p.extend_from_slice(&granule.to_le_bytes());
        p.extend_from_slice(&[0; 12]);
        p.extend_from_slice(&[1, u8::try_from(body.len()).unwrap_or(u8::MAX)]);
        p.extend_from_slice(body);
        p
    }

    /// A whole OGG Vorbis file's framing: the first page of [`ogg`],
    /// filled out to the 30 bytes it declares, then a last page that puts
    /// the stream at `samples`.
    pub(super) fn ogg_file(rate: u32, samples: u64) -> Vec<u8> {
        let mut o = ogg(2, rate);
        o.resize(29 + 30, 0);
        o.extend_from_slice(&ogg_page(samples, &[0; 40]));
        o
    }

    /// A good file for `name`'s extension. An OGG plays for 2 s, the
    /// `length_ms` of [`FULL`]'s tracks.
    fn good(name: &str) -> Vec<u8> {
        if has_extension(name, &["wav"]) {
            wav(2, SAMPLE_RATE, 1)
        } else if has_extension(name, &["flac"]) {
            FLAC.to_vec()
        } else if has_extension(name, &["mp3"]) {
            MP3.to_vec()
        } else {
            ogg_file(SAMPLE_RATE, u64::from(SAMPLE_RATE) * 2)
        }
    }

    /// How the tests read files: sounds are shown under
    /// `base/audio/`, music under `music/`.
    fn reading<'a>(
        sound: &'a dyn Fn(&str) -> Bytes,
        music: Option<&'a dyn Fn(&str) -> Bytes>,
    ) -> Files<'a> {
        Files {
            sound,
            sound_shown: &|f: &str| format!("base/audio/{f}"),
            sound_left_out: &|_: &str| false,
            music_dir: "music",
            music,
        }
    }

    fn all_files() -> Files<'static> {
        reading(
            &|p: &str| Some(Cow::Owned(good(p))),
            Some(&|f: &str| Some(Cow::Owned(good(f)))),
        )
    }

    fn errors(source: &str) -> Vec<String> {
        errors_with(source, &all_files())
    }

    fn errors_with(source: &str, files: &Files<'_>) -> Vec<String> {
        match from_source("a.ron", source, files) {
            Ok(_) => vec![],
            Err(e) => e.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn the_empty_manifest_is_valid() {
        let m = from_source("a.ron", EMPTY, &all_files());
        assert_eq!(
            m,
            Ok(AudioManifest {
                music_fade_ms: 500,
                ..AudioManifest::default()
            })
        );
    }

    /// A file without the fade-in and the overlap reads as before: the
    /// old track fades out, then the new one starts at full gain.
    #[test]
    fn the_default_switch_is_the_fade_out_unless_the_file_says_more() {
        let m = from_source("a.ron", EMPTY, &all_files()).unwrap_or_default();
        assert_eq!(m.music_switch(), MusicSwitch::after_fade(500));
        let source = EMPTY.replace(
            "music_fade_ms: 500,",
            "music_fade_ms: 800, music_fade_in_ms: 300, music_overlap: true,",
        );
        let m = from_source("a.ron", &source, &all_files()).unwrap_or_default();
        assert_eq!((m.music_fade_in_ms, m.music_overlap), (300, true));
        assert_eq!(
            m.music_switch(),
            MusicSwitch::after_fade(800)
                .with_in_ms(300)
                .with_overlap(true)
        );
    }

    #[test]
    fn the_switch_constructors_set_its_three_fields() {
        let s = MusicSwitch::after_fade(1).with_in_ms(2).with_overlap(true);
        assert_eq!((s.out_ms, s.in_ms, s.overlap), (1, 2, true));
        let after = MusicSwitch::after_fade(500);
        assert_eq!((after.out_ms, after.in_ms, after.overlap), (500, 0, false));
        let cross = MusicSwitch::crossfade(750);
        assert_eq!((cross.out_ms, cross.in_ms, cross.overlap), (750, 750, true));
    }

    #[test]
    fn a_full_manifest_loads() {
        let m = from_source("a.ron", FULL, &all_files()).unwrap_or_default();
        assert_eq!(m.music_fade_ms, 250);
        assert_eq!(
            m.sounds
                .get("footfall")
                .map(|s| (&s.files, s.volume, &s.credit)),
            Some((
                &vec!["sfx/step_1.ogg".to_owned(), "sfx/step_2.ogg".to_owned()],
                60,
                &CreditRef::Credit("steps".into())
            ))
        );
        assert_eq!(
            m.sounds.get("ui_tick").map(|s| (&s.credit, s.looped)),
            Some((&CreditRef::Own, false)),
            "a sound plays once unless it says it loops"
        );
        let title = m.music.get("title");
        assert_eq!(
            title.map(|t| (t.file.as_str(), t.volume)),
            Some(("title.ogg", 80))
        );
        assert_eq!(
            title.map(|t| t.looped),
            Some(true),
            "music loops by default"
        );
        assert_eq!(m.music.get("sting").map(|t| t.looped), Some(false));
        assert_eq!(title.map(|t| t.length_ms), Some(2000));
        assert_eq!(
            m.pools.get("skirmish"),
            Some(&vec!["title".to_owned(), "sting".to_owned()])
        );
        let sunrise = m.credits.get("sunrise").cloned().unwrap_or_default();
        assert_eq!(
            (sunrise.title.as_str(), sunrise.author.as_str()),
            ("New Sunrise", "nene")
        );
        assert_eq!(sunrise.license, "CC0-1.0");
        assert_eq!(sunrise.tags, ["music", "title"]);
        assert_eq!(sunrise.note, "V1 file");
        assert_eq!(m.credits.get("ours").map(|c| c.title.as_str()), Some(""));
    }

    #[test]
    fn a_sound_may_loop() {
        let source = r#"(music_fade_ms: 0, music: [], pools: [], credits: [], sounds: [
            (id: "rain", files: ["sfx/ui_tick.wav"], volume: 40, credit: Own, looped: true),
        ])"#;
        let m = from_source("a.ron", source, &all_files()).unwrap_or_default();
        assert_eq!(m.sounds.get("rain").map(|s| s.looped), Some(true));
    }

    /// Every cue of a manifest that names no rules has the default ones.
    #[test]
    fn a_cue_that_names_no_rules_has_the_default_ones() {
        let m = from_source("a.ron", FULL, &all_files()).unwrap_or_default();
        assert_eq!(m.sounds.len(), 2);
        for (id, cue) in &m.sounds {
            assert_eq!(cue.rules, CueRules::default(), "{id}");
        }
        assert_eq!(CueRules::DEFAULT, CueRules::default());
    }

    /// The rules are written flat on the sound, `volume_min` bare.
    #[test]
    fn a_cues_rules_are_read_from_its_entry() {
        let rules =
            "volume: 60, credit: Credit(\"steps\"), pick: NoRepeat, volume_min: 45, min_gap_ms: 60";
        let source = FULL.replace("volume: 60, credit: Credit(\"steps\")", rules);
        let m = from_source("a.ron", &source, &all_files());
        let footfall = m.map(|m| m.sounds.get("footfall").map(|s| s.rules));
        let want = CueRules::DEFAULT
            .with_pick(Pick::NoRepeat)
            .with_volume_min(Some(45))
            .with_min_gap_ms(60);
        assert_eq!(footfall, Ok(Some(want)));
        assert_eq!(
            (want.pick, want.volume_min, want.min_gap_ms),
            (Pick::NoRepeat, Some(45), 60)
        );
        for (word, pick) in [
            ("Random", Pick::Random),
            ("Shuffle", Pick::Shuffle),
            ("InOrder", Pick::InOrder),
        ] {
            let source = FULL.replace("volume: 60,", &format!("volume: 60, pick: {word},"));
            let m = from_source("a.ron", &source, &all_files());
            let got = m.map(|m| m.sounds.get("footfall").map(|s| s.rules.pick));
            assert_eq!(got, Ok(Some(pick)), "{word}");
        }
    }

    /// Every file good and measured: a WAV of 100 ms.
    fn measured_files() -> Files<'static> {
        reading(
            &|p: &str| {
                Some(Cow::Owned(if has_extension(p, &["wav"]) {
                    wav_of_frames(2, SAMPLE_RATE, 4_410)
                } else {
                    good(p)
                }))
            },
            Some(&|f: &str| Some(Cow::Owned(good(f)))),
        )
    }

    /// `FULL` with `fields` on its `footfall` (2 s OGGs, so their lengths
    /// are measured) and `top` at the top of the file.
    fn capped(fields: &str, top: &str) -> String {
        FULL.replace("volume: 60,", &format!("volume: 60, {fields}"))
            .replace("music_fade_ms: 250,", &format!("music_fade_ms: 250, {top}"))
    }

    /// The copy cap, `when_full` and the priority are flat on the sound,
    /// and `max_voices` at the top, the two `Option`s bare.
    #[test]
    fn a_cues_copy_cap_and_priority_and_the_voice_limit_are_read_bare() {
        let source = capped(
            "max_copies: 3, when_full: StopOldest, priority: 200,",
            "max_voices: 12,",
        );
        let m = from_source("a.ron", &source, &measured_files());
        let m = m.unwrap_or_default();
        let want = CueRules::DEFAULT
            .with_max_copies(Some(3), WhenFull::StopOldest)
            .with_priority(200);
        assert_eq!(m.sounds.get("footfall").map(|s| s.rules), Some(want));
        assert_eq!(
            (want.max_copies, want.when_full, want.priority),
            (Some(3), WhenFull::StopOldest, 200)
        );
        assert_eq!(m.max_voices, Some(12));
        // Left out, each is its default.
        let m = from_source("a.ron", FULL, &all_files()).unwrap_or_default();
        assert_eq!(m.max_voices, None);
        let rules = m.sounds.get("footfall").map_or_default(|s| s.rules);
        assert_eq!(
            (rules.max_copies, rules.when_full, rules.priority),
            (None, WhenFull::Skip, 128)
        );
        let m = from_source("a.ron", &capped("when_full: Skip,", ""), &all_files());
        let got = m.map(|m| m.sounds.get("footfall").map(|s| s.rules.when_full));
        assert_eq!(got, Ok(Some(WhenFull::Skip)));
    }

    #[test]
    fn a_cap_or_a_voice_limit_of_zero_is_refused_at_its_line() {
        assert_eq!(
            errors(&capped("max_copies: 0,", "")),
            ["a.ron:5: sound \"footfall\": max_copies is 0: leave it out for no cap"]
        );
        let source = FULL.replace(
            "music_fade_ms: 250,",
            "music_fade_ms: 250,\n    max_voices: 0,",
        );
        let measured = measured_files();
        assert_eq!(
            errors_with(&source, &measured),
            ["a.ron:3: max_voices is 0: leave it out for no limit"]
        );
        let lowest = capped("max_copies: 1, priority: 0,", "max_voices: 1,");
        assert_eq!(errors_with(&lowest, &measured), Vec::<String>::new());
    }

    /// `FULL`'s `ui_tick` is a WAV with no `data` chunk: its length can't be
    /// told, which only matters when its copies are counted.
    #[test]
    fn a_counted_cue_whose_file_cant_be_measured_is_refused_at_its_line() {
        let tick = "volume: 100, credit: Own";
        let why = "a.ron:4: sound \"ui_tick\": base/audio/sfx/ui_tick.wav: its length can't be \
                   measured, and max_copies or max_voices counts its copies";
        assert_eq!(
            errors(FULL),
            Vec::<String>::new(),
            "nothing counts its copies"
        );
        let source = FULL.replace(tick, &format!("{tick}, max_copies: 2"));
        assert_eq!(errors(&source), [why]);
        let source = FULL.replace("music_fade_ms: 250,", "music_fade_ms: 250, max_voices: 8,");
        assert_eq!(errors(&source), [why]);
        // A file left out of the build isn't read, so isn't measured.
        let files = Files {
            sound_left_out: &|p: &str| p == "sfx/ui_tick.wav",
            ..all_files()
        };
        assert_eq!(errors_with(&source, &files), Vec::<String>::new());
    }

    #[test]
    fn a_lowest_volume_over_the_cues_volume_is_refused_at_its_line() {
        assert_eq!(
            errors(&FULL.replace("volume: 60,", "volume: 60, volume_min: 60,")),
            Vec::<String>::new(),
            "a lowest volume equal to the volume plays at the volume"
        );
        assert_eq!(
            errors(&FULL.replace("volume: 60,", "volume: 60, volume_min: 61,")),
            ["a.ron:5: sound \"footfall\": volume_min 61 is over its volume 60"]
        );
        assert_eq!(
            errors(&FULL.replace("volume: 60,", "volume: 60, gap_ms: 61,")).len(),
            1,
            "an unknown rule is refused"
        );
    }

    /// `new` gives the default rules in a `const` item too.
    #[test]
    fn a_cue_built_in_a_const_has_the_default_rules_until_it_names_others() {
        const PLAIN: SoundCue = SoundCue::new(Vec::new(), 60, CreditRef::Own);
        const SPACED: SoundCue = SoundCue::new(Vec::new(), 60, CreditRef::Own)
            .with_rules(CueRules::DEFAULT.with_min_gap_ms(40));
        assert_eq!(PLAIN.rules, CueRules::default());
        assert_eq!(SPACED.rules.min_gap_ms, 40);
        assert_eq!(SPACED.rules.pick, Pick::Random);
    }

    #[test]
    fn duplicate_ids_are_refused() {
        let dup = FULL.replace(r#"(id: "sting""#, r#"(id: "ui_tick""#);
        assert_eq!(
            errors(&dup.replace(r#""title", "sting""#, r#""title""#)),
            ["a.ron:4: duplicate cue id \"ui_tick\""]
        );
        let pool = FULL.replace(r#"(id: "skirmish""#, r#"(id: "title""#);
        assert_eq!(errors(&pool), ["a.ron:8: duplicate cue id \"title\""]);
        let credit = FULL.replace(r#"(id: "ours""#, r#"(id: "steps""#);
        assert_eq!(errors(&credit), ["a.ron:18: duplicate credit id \"steps\""]);
        let empty = FULL.replace(r#"(id: "sting""#, r#"(id: """#);
        assert_eq!(
            errors(&empty.replace(r#""title", "sting""#, r#""title""#)),
            ["a.ron:9: a cue id is empty"]
        );
    }

    #[test]
    fn missing_files_are_refused() {
        let no_step_2 = |p: &str| (p != "sfx/step_2.ogg").then(|| Cow::Owned(good(p)));
        let no_title = |f: &str| (f != "title.ogg").then(|| Cow::Owned(good(f)));
        let files = reading(&no_step_2, Some(&no_title));
        assert_eq!(
            errors_with(FULL, &files),
            [
                "a.ron:5: sound \"footfall\": file base/audio/sfx/step_2.ogg not found",
                "a.ron:8: music \"title\": file music/title.ogg not found",
            ]
        );
        // The game itself can't see the music folder and skips that check.
        let files = reading(&|p: &str| Some(Cow::Owned(good(p))), None);
        assert_eq!(errors_with(FULL, &files), Vec::<String>::new());
    }

    #[test]
    fn a_sound_file_the_build_leaves_out_is_not_checked() {
        let no_step_2 = |p: &str| (p != "sfx/step_2.ogg").then(|| Cow::Owned(good(p)));
        let files = Files {
            sound_left_out: &|p: &str| p == "sfx/step_2.ogg",
            ..reading(&no_step_2, None)
        };
        assert_eq!(errors_with(FULL, &files), Vec::<String>::new());
        let m = from_source("a.ron", FULL, &files).unwrap_or_default();
        assert!(
            m.sounds["footfall"]
                .files
                .contains(&"sfx/step_2.ogg".to_owned())
        );
        // Its name is still checked.
        let wrong = FULL.replace("sfx/step_2.ogg", "sfx/step_2.aiff");
        let files = Files {
            sound_left_out: &|p: &str| p == "sfx/step_2.aiff",
            ..reading(&no_step_2, None)
        };
        assert_eq!(
            errors_with(&wrong, &files),
            [
                "a.ron:5: sound \"footfall\": \"sfx/step_2.aiff\" must be .ogg, .wav, .flac, .mp3 or .sfx.ron"
            ]
        );
    }

    #[test]
    fn unplayable_files_are_refused() {
        let swapped = |name: &str| {
            let bytes = if has_extension(name, &["wav"]) {
                ogg(2, SAMPLE_RATE)
            } else {
                wav(2, SAMPLE_RATE, 1)
            };
            Some(Cow::Owned(bytes))
        };
        let files = reading(&swapped, Some(&swapped));
        assert_eq!(
            errors_with(FULL, &files),
            [
                "a.ron:4: sound \"ui_tick\": base/audio/sfx/ui_tick.wav: not a PCM WAV file",
                "a.ron:5: sound \"footfall\": base/audio/sfx/step_1.ogg: not an OGG Vorbis file",
                "a.ron:5: sound \"footfall\": base/audio/sfx/step_2.ogg: not an OGG Vorbis file",
                "a.ron:8: music \"title\": music/title.ogg: not an OGG Vorbis file",
                "a.ron:9: music \"sting\": music/sting.ogg: not an OGG Vorbis file",
            ]
        );
    }

    #[test]
    fn file_headers_must_be_44_1_khz_mono_or_stereo() {
        let ok = |name, bytes: Vec<u8>| format_problem(name, &bytes);
        assert_eq!(ok("a.wav", wav(1, 44_100, 1)), None);
        assert_eq!(ok("a.wav", wav(2, 44_100, 3)), None, "float WAV");
        assert_eq!(ok("a.ogg", ogg(1, 44_100)), None);
        assert_eq!(ok("a.ogg", ogg(2, 44_100)), None);
        assert_eq!(
            ok("a.wav", wav(2, 48_000, 1)).as_deref(),
            Some("48000 Hz; must be 44100 Hz")
        );
        assert_eq!(
            ok("a.ogg", ogg(2, 22_050)).as_deref(),
            Some("22050 Hz; must be 44100 Hz")
        );
        assert_eq!(
            ok("a.ogg", ogg(6, 44_100)).as_deref(),
            Some("6 channels; must be mono or stereo")
        );
        assert_eq!(
            ok("a.wav", wav(0, 44_100, 1)).as_deref(),
            Some("0 channels; must be mono or stereo")
        );
        let not_wav = Some("not a PCM WAV file");
        assert_eq!(ok("a.wav", wav(2, 44_100, 2)).as_deref(), not_wav, "ADPCM");
        assert_eq!(
            ok("a.wav", b"RIFF\0\0\0\0WAVE".to_vec()).as_deref(),
            not_wav
        );
        assert_eq!(
            ok("a.wav", b"RIFX\0\0\0\0WAVE".to_vec()).as_deref(),
            not_wav
        );
        assert_eq!(
            ok("a.wav", b"RIFF\0\0\0\0AVI ".to_vec()).as_deref(),
            not_wav
        );
        // A good WAV with one of its two magic words wrong.
        let mut rifx = wav(2, 44_100, 1);
        rifx[3] = b'X';
        assert_eq!(ok("a.wav", rifx).as_deref(), not_wav);
        let mut avi = wav(2, 44_100, 1);
        avi[8..12].copy_from_slice(b"AVI ");
        assert_eq!(ok("a.wav", avi).as_deref(), not_wav);
        let mut huge = wav(2, 44_100, 1);
        huge[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(ok("a.wav", huge).as_deref(), not_wav);
        assert_eq!(ok("a.wav", Vec::new()).as_deref(), not_wav);
        let not_ogg = Some("not an OGG Vorbis file");
        let mut opus = ogg(2, 44_100);
        opus[30..36].copy_from_slice(b"opus!!");
        assert_eq!(ok("a.ogg", opus).as_deref(), not_ogg);
        assert_eq!(ok("a.ogg", b"OggS".to_vec()).as_deref(), not_ogg);
        assert_eq!(ok("a.ogg", b"ID3\x03".to_vec()).as_deref(), not_ogg);
        let short = ogg(2, 44_100)[..40].to_vec();
        assert_eq!(ok("a.ogg", short).as_deref(), not_ogg);
    }

    #[test]
    fn an_ogg_length_is_its_last_granule_over_its_sample_rate() {
        // The title track: 5,898,057 samples at 44.1 kHz.
        assert_eq!(ogg_length_ms(&ogg_file(44_100, 5_898_057)), Some(133_743));
        assert_eq!(ogg_length_ms(&ogg_file(44_100, 44_100)), Some(1000));
        assert_eq!(ogg_length_ms(&ogg_file(22_050, 44_100)), Some(2000));
        assert_eq!(ogg_length_ms(&ogg_file(44_100, 0)), Some(0));
        // To the nearest millisecond, halves up.
        assert_eq!(ogg_length_ms(&ogg_file(2000, 2)), Some(1));
        assert_eq!(ogg_length_ms(&ogg_file(2000, 3)), Some(2));
        assert_eq!(ogg_length_ms(&ogg_file(3000, 4)), Some(1));
        assert_eq!(ogg_length_ms(&ogg_file(3000, 5)), Some(2));
        // The page that counts is the last, whatever came before.
        let mut two = ogg_file(44_100, 44_100);
        two.extend_from_slice(&ogg_page(88_200, b"tail"));
        assert_eq!(ogg_length_ms(&two), Some(2000));
        // A page with an empty segment table ends the file too.
        let mut bare = ogg_file(44_100, 44_100);
        let mut page = ogg_page(132_300, &[]);
        page.truncate(27);
        page[26] = 0;
        bare.extend_from_slice(&page);
        assert_eq!(ogg_length_ms(&bare), Some(3000));
    }

    #[test]
    fn oggs_inside_the_audio_data_is_not_a_page() {
        // The letters, then bytes that would make a 27-byte "page" far
        // short of the end of the file.
        let mut body = b"OggS".to_vec();
        body.resize(60, 0);
        let mut o = ogg_file(44_100, 1);
        o.extend_from_slice(&ogg_page(44_100, &body));
        assert_eq!(ogg_length_ms(&o), Some(1000));
        // And at the very end, where there is no room for a header.
        let mut o = ogg_file(44_100, 1);
        o.extend_from_slice(&ogg_page(88_200, b"....OggS"));
        assert_eq!(ogg_length_ms(&o), Some(2000));
    }

    #[test]
    fn a_file_without_a_readable_end_has_no_length() {
        let whole = ogg_file(44_100, 44_100);
        assert_eq!(ogg_length_ms(&whole), Some(1000));
        // Cut short, or with something after the last page.
        assert_eq!(ogg_length_ms(&whole[..whole.len() - 1]), None);
        let mut junk = whole.clone();
        junk.push(0);
        assert_eq!(ogg_length_ms(&junk), None);
        // No packet ends on the last page: it carries no position.
        assert_eq!(ogg_length_ms(&ogg_file(44_100, u64::MAX)), None);
        // A position too big to be a length, and a rate of 0.
        assert_eq!(ogg_length_ms(&ogg_file(44_100, u64::MAX - 1)), None);
        assert_eq!(ogg_length_ms(&ogg_file(1, u64::from(u32::MAX))), None);
        assert_eq!(ogg_length_ms(&ogg_file(0, 44_100)), None);
        // Not OGG Vorbis at all.
        assert_eq!(ogg_length_ms(&wav(2, 44_100, 1)), None);
        assert_eq!(ogg_length_ms(&ogg_page(44_100, &[0; 40])), None);
        assert_eq!(ogg_length_ms(&[]), None);
        // The header-only first page declares more than it holds.
        assert_eq!(ogg_length_ms(&ogg(2, 44_100)), None);
    }

    /// The start of [`wav`]'s file, of 16-bit samples (its block align
    /// `2 × channels`), then a `data` chunk of `frames` frames of silence.
    fn wav_of_frames(channels: u16, rate: u32, frames: u32) -> Vec<u8> {
        let mut w = wav(channels, rate, 1);
        // `fmt `'s body starts at byte 34: its block align is at 12 in it.
        w[46..48].copy_from_slice(&(channels * 2).to_le_bytes());
        w[48..50].copy_from_slice(&16_u16.to_le_bytes());
        let size = frames * u32::from(channels) * 2;
        w.extend_from_slice(b"data");
        w.extend_from_slice(&size.to_le_bytes());
        w.resize(w.len() + usize::try_from(size).unwrap_or(0), 0);
        w
    }

    #[test]
    fn a_wavs_length_is_its_data_over_its_block_align_and_rate() {
        let length = |bytes: &[u8]| sound_length_ms("sfx/a.wav", bytes);
        assert_eq!(length(&wav_of_frames(1, 44_100, 22_050)), Some(500));
        assert_eq!(length(&wav_of_frames(2, 44_100, 22_050)), Some(500));
        assert_eq!(length(&wav_of_frames(2, 44_100, 441)), Some(10));
        assert_eq!(length(&wav_of_frames(1, 22_050, 22_050)), Some(1000));
        assert_eq!(length(&wav_of_frames(1, 44_100, 0)), Some(0));
        // To the nearest millisecond: 66 frames are 1.497 ms, 67 are 1.519.
        assert_eq!(length(&wav_of_frames(1, 44_100, 66)), Some(1));
        assert_eq!(length(&wav_of_frames(1, 44_100, 67)), Some(2));
        // A block left over counts for nothing.
        let mut odd = wav_of_frames(2, 44_100, 441);
        odd.extend_from_slice(&[0; 3]);
        let size = u32::try_from(441 * 4 + 3).unwrap_or(0);
        let at = odd.len() - 441 * 4 - 3 - 4;
        odd[at..at + 4].copy_from_slice(&size.to_le_bytes());
        assert_eq!(length(&odd), Some(10));
        // A `data` chunk that says more than the file holds counts what it
        // holds.
        let cut = wav_of_frames(1, 44_100, 44_100);
        assert_eq!(length(&cut[..cut.len() - 44_100]), Some(500));
        // What the engine writes ([`pcm16_wav`]) has its length too.
        assert_eq!(length(&pcm16_wav(2, 44_100, &[0; 882])), Some(10));
        // No `data` chunk, a block align of 0, a rate of 0, no `fmt `.
        assert_eq!(length(&wav(2, 44_100, 1)), None);
        let mut no_block = wav_of_frames(2, 44_100, 441);
        no_block[46..48].fill(0);
        assert_eq!(length(&no_block), None);
        assert_eq!(length(&wav_of_frames(2, 0, 441)), None);
        assert_eq!(length(&pcm16_wav(2, 44_100, &[0; 4])[..36]), None);
        assert_eq!(length(b"RIFF"), None);
    }

    #[test]
    fn a_sounds_length_is_read_by_its_bytes_then_its_name() {
        let half = wav_of_frames(1, 44_100, 22_050);
        assert_eq!(
            sound_length_ms("a.ogg", &ogg_file(44_100, 22_050)),
            Some(500)
        );
        assert_eq!(sound_length_ms("a.ogg", &ogg(2, 44_100)), None);
        // A WAV is one by its bytes, whatever its name: a game's hook
        // decoded or rendered it.
        for name in ["a.wav", "a.flac", "a.mp3", "a.sfx.ron", "a.ogg"] {
            assert_eq!(sound_length_ms(name, &half), Some(500), "{name}");
        }
        // An OGG too, its name aside (the manifest's check refuses it).
        assert_eq!(
            sound_length_ms("a.wav", &ogg_file(44_100, 22_050)),
            Some(500)
        );
        assert_eq!(sound_length_ms("a.wav", b"RIFF"), None);
        assert_eq!(sound_length_ms("a.sfx.ron", b"(wave: Sine)"), None);
    }

    #[test]
    fn a_cue_keeps_the_length_of_each_of_its_files() {
        let step = |p: &str| match p {
            "sfx/ui_tick.wav" => Some(Cow::Owned(wav_of_frames(2, SAMPLE_RATE, 4_410))),
            "sfx/step_2.ogg" => None,
            _ => Some(Cow::Owned(good(p))),
        };
        let files = Files {
            sound_left_out: &|p: &str| p == "sfx/step_2.ogg",
            ..reading(&step, None)
        };
        let m = from_source("a.ron", FULL, &files).unwrap_or_default();
        assert_eq!(m.sounds["ui_tick"].lengths_ms, [Some(100)]);
        let footfall = &m.sounds["footfall"];
        assert_eq!(footfall.lengths_ms, [Some(2000), None], "one left out");
        assert_eq!(
            (0..3).map(|f| footfall.length_ms(f)).collect::<Vec<_>>(),
            [Some(2000), None, None],
            "past the list is unknown"
        );
        // A WAV with no `data` chunk has none.
        let m = from_source("a.ron", FULL, &all_files()).unwrap_or_default();
        assert_eq!(m.sounds["ui_tick"].lengths_ms, [None]);
        // The Sound kind reads the same lengths.
        let read = |bytes: &[u8], name: &str| {
            crate::format::Formats::standard()
                .decode::<Sound>(
                    &crate::format::AssetPath::new(name),
                    bytes,
                    &crate::format::DecodeCx::new(),
                )
                .map(|sound| sound.length_ms)
        };
        assert_eq!(
            read(&wav_of_frames(2, SAMPLE_RATE, 4_410), "a.wav"),
            Ok(Some(100))
        );
        assert_eq!(read(&good("a.ogg"), "a.ogg"), Ok(Some(2000)));
    }

    #[test]
    fn a_manifest_built_in_code_measures_its_sounds() {
        let mut m = AudioManifest::default();
        let files = ["a.wav", "b.ogg", "gone.wav", "c.wav"].map(String::from);
        m.sounds.insert(
            "hit".into(),
            SoundCue::new(files.to_vec(), 100, CreditRef::Own),
        );
        assert_eq!(m.sounds["hit"].lengths_ms, [], "new measures nothing");
        assert_eq!(m.sounds["hit"].length_ms(0), None);
        m.measure_sounds(&|name: &str| match name {
            "a.wav" => Some(Cow::Owned(wav_of_frames(1, SAMPLE_RATE, 11_025))),
            "b.ogg" => Some(Cow::Owned(good(name))),
            "c.wav" => Some(Cow::Owned(wav(1, SAMPLE_RATE, 1))),
            _ => None,
        });
        assert_eq!(
            m.sounds["hit"].lengths_ms,
            [Some(250), Some(2000), None, None]
        );
    }

    /// [`FULL`] with the `sting` track's `length_ms` changed.
    fn with_sting_length(length_ms: u32) -> String {
        FULL.replace(
            "length_ms: 2000, credit: Own",
            &format!("length_ms: {length_ms}, credit: Own"),
        )
    }

    #[test]
    fn a_music_length_must_match_its_file() {
        // `good` tracks play for 2000 ms; 20 ms either way is allowed.
        for near in [1980, 2020] {
            let errors = errors(&with_sting_length(near));
            assert_eq!(errors, Vec::<String>::new(), "{near}");
        }
        for wrong in [1979, 2021] {
            assert_eq!(
                errors(&with_sting_length(wrong)),
                [format!(
                    "a.ron:9: music \"sting\": music/sting.ogg: it plays for 2000 ms, \
                     but length_ms is {wrong}; write length_ms: 2000"
                )]
            );
        }
        // A file whose end can't be read.
        let headers = reading(
            &|p: &str| Some(Cow::Owned(good(p))),
            Some(&|_: &str| Some(Cow::Owned(ogg(2, SAMPLE_RATE)))),
        );
        assert_eq!(
            errors_with(FULL, &headers),
            [
                "a.ron:8: music \"title\": music/title.ogg: its length can't be read (no last Ogg page)",
                "a.ron:9: music \"sting\": music/sting.ogg: its length can't be read (no last Ogg page)",
            ]
        );
    }

    #[test]
    fn a_music_length_of_zero_is_refused() {
        // Where the music folder can't be seen, any length but 0 passes.
        let unseen = reading(&|p: &str| Some(Cow::Owned(good(p))), None);
        assert_eq!(
            errors_with(&with_sting_length(1), &unseen),
            Vec::<String>::new()
        );
        assert_eq!(
            errors_with(&with_sting_length(0), &unseen),
            ["a.ron:9: music \"sting\": length_ms is 0; it is the track's length in milliseconds"]
        );
        // The field is required.
        let missing = FULL.replace("looped: false, length_ms: 2000, ", "looped: false, ");
        assert_eq!(errors(&missing).len(), 1);
    }

    /// `FULL` with `fields` written into the looped track `title`.
    fn title_with(fields: &str) -> String {
        FULL.replace(
            "length_ms: 2000, credit: Credit(\"sunrise\")",
            &format!("length_ms: 2000, credit: Credit(\"sunrise\"){fields}"),
        )
    }

    #[test]
    fn a_loop_region_is_read_and_kept() {
        let source = title_with(", loop_start_ms: 500, loop_end_ms: 2000");
        let m = from_source("a.ron", &source, &all_files()).unwrap_or_default();
        let title = m.music.get("title");
        assert_eq!(
            title.and_then(MusicCue::loop_region),
            LoopRegion::new(500, 2000)
        );
        // Left out, there is none: the whole track loops, or the file's region.
        let plain = from_source("a.ron", FULL, &all_files()).unwrap_or_default();
        let title = plain.music.get("title");
        assert_eq!(
            title.map(|t| (t.loop_start_ms, t.loop_end_ms)),
            Some((None, None))
        );
    }

    #[test]
    fn a_loop_region_must_have_both_ends_in_order_inside_a_looped_track() {
        let cases = [
            (", loop_start_ms: 0", "loop_start_ms needs loop_end_ms too"),
            (", loop_end_ms: 10", "loop_end_ms needs loop_start_ms too"),
            (
                ", loop_start_ms: 10, loop_end_ms: 10",
                "loop_end_ms 10 must be after loop_start_ms 10",
            ),
            (
                ", loop_start_ms: 11, loop_end_ms: 10",
                "loop_end_ms 10 must be after loop_start_ms 11",
            ),
            (
                ", loop_start_ms: 0, loop_end_ms: 2001",
                "loop_end_ms 2001 is past the track's end; length_ms is 2000",
            ),
            (
                ", loop_start_ms: 0, loop_end_ms: 4294967295",
                "loop_end_ms 4294967295 is past the track's end; length_ms is 2000",
            ),
            (
                ", looped: false, loop_start_ms: 0, loop_end_ms: 10",
                "a loop region needs a looped track; this one has looped: false",
            ),
        ];
        for (fields, why) in cases {
            assert_eq!(
                errors(&title_with(fields)),
                [format!("a.ron:8: music \"title\": {why}")],
                "{fields}"
            );
        }
        // From the very start to the very end is a region; past a u32 isn't a number.
        assert_eq!(
            errors(&title_with(", loop_start_ms: 0, loop_end_ms: 2000")),
            Vec::<String>::new()
        );
        assert_eq!(
            errors(&title_with(", loop_start_ms: 0, loop_end_ms: 4294967296")).len(),
            1
        );
    }

    #[test]
    fn file_formats_are_checked() {
        let wav_music = FULL.replace("title.ogg", "title.wav");
        assert_eq!(
            errors(&wav_music),
            ["a.ron:8: music \"title\": \"title.wav\" must be .ogg, .flac or .mp3"]
        );
        let aiff = FULL.replace("ui_tick.wav", "ui_tick.aiff");
        assert_eq!(
            errors(&aiff),
            [
                "a.ron:4: sound \"ui_tick\": \"sfx/ui_tick.aiff\" must be .ogg, .wav, .flac, .mp3 or .sfx.ron"
            ]
        );
        for sound in ["ui_tick.flac", "ui_tick.mp3"] {
            let coded = FULL.replace("ui_tick.wav", sound);
            assert_eq!(errors(&coded), Vec::<String>::new(), "{sound}");
        }
        assert_eq!(errors(&FULL.replace("sfx/ui_tick.wav", ".wav")).len(), 1);
        assert_eq!(errors(&FULL.replace("sfx/ui_tick.wav", "wav")).len(), 1);
        let none = FULL.replace(r#"["sfx/ui_tick.wav"]"#, "[]");
        assert_eq!(errors(&none), ["a.ron:4: sound \"ui_tick\" has no files"]);
    }

    /// A manifest of a FLAC and an MP3 sound, and a FLAC and an MP3 track
    /// `length_ms` long each.
    fn coded(flac_ms: u32, mp3_ms: u32) -> String {
        format!(
            r#"(
    music_fade_ms: 250,
    sounds: [
        (id: "chime", files: ["sfx/chime.flac", "sfx/chime.mp3"], volume: 100, credit: Own),
    ],
    music: [
        (id: "lossless", file: "lossless.flac", volume: 80, length_ms: {flac_ms}, credit: Own),
        (id: "small", file: "small.mp3", volume: 80, length_ms: {mp3_ms}, credit: Own),
    ],
    pools: [],
    credits: [],
)"#
        )
    }

    #[test]
    fn flac_and_mp3_sounds_and_tracks_are_checked() {
        let mp3_ms = track_length_ms("a.mp3", MP3).unwrap();
        assert_eq!(track_length_ms("a.flac", FLAC), Some(50));
        assert_eq!(errors(&coded(50, mp3_ms)), Vec::<String>::new());
        // A wrong length gets the value to write.
        let wrong = mp3_ms + 100;
        assert_eq!(
            errors(&coded(500, wrong)),
            [
                "a.ron:7: music \"lossless\": music/lossless.flac: it plays for 50 ms, \
                 but length_ms is 500; write length_ms: 50"
                    .to_owned(),
                format!(
                    "a.ron:8: music \"small\": music/small.mp3: it plays for {mp3_ms} ms, \
                     but length_ms is {wrong}; write length_ms: {mp3_ms}"
                ),
            ]
        );
        // Broken bytes: each format's own message.
        let junk = reading(
            &|_: &str| Some(Cow::Borrowed(&b"junk"[..])),
            Some(&|_: &str| Some(Cow::Borrowed(&b"junk"[..]))),
        );
        assert_eq!(
            errors_with(&coded(50, mp3_ms), &junk),
            [
                "a.ron:4: sound \"chime\": base/audio/sfx/chime.flac: not a FLAC file",
                "a.ron:4: sound \"chime\": base/audio/sfx/chime.mp3: not an MP3 file",
                "a.ron:7: music \"lossless\": music/lossless.flac: not a FLAC file",
                "a.ron:8: music \"small\": music/small.mp3: not an MP3 file",
            ]
        );
        // A sound a game's hook decoded is a WAV, and checked as one.
        let hooked = reading(&|_: &str| Some(Cow::Owned(wav(1, 22_050, 1))), None);
        assert_eq!(
            errors_with(&coded(50, mp3_ms), &hooked),
            [
                "a.ron:4: sound \"chime\": base/audio/sfx/chime.flac: 22050 Hz; must be 44100 Hz",
                "a.ron:4: sound \"chime\": base/audio/sfx/chime.mp3: 22050 Hz; must be 44100 Hz",
            ]
        );
    }

    #[test]
    fn a_track_whose_length_cant_be_read_is_told_for_its_format() {
        let unread = |file| length_problem(file, 50, None);
        assert_eq!(unread("a.ogg").as_deref(), Some(NO_LENGTH));
        assert_eq!(unread("a.flac").as_deref(), Some(NO_FLAC_LENGTH));
        assert_eq!(unread("a.mp3").as_deref(), Some(NO_MP3_LENGTH));
        assert_eq!(length_problem("a.flac", 50, Some(70)), None);
    }

    #[test]
    fn unknown_licenses_are_refused() {
        let nc = FULL.replace("\"CC-BY-4.0\"", "\"CC-BY-NC-4.0\"");
        assert_eq!(
            errors(&nc),
            [
                "a.ron:18: credit \"steps\": license \"CC-BY-NC-4.0\" is not allowed \
                 (ADR-0027: CC0-1.0, CC-BY-3.0, CC-BY-4.0, Own)"
            ]
        );
        // CC BY 3.0 is allowed too (Nick, ticket 0214; game ADR-0027).
        let by3 = FULL.replace("\"CC-BY-4.0\"", "\"CC-BY-3.0\"");
        assert_eq!(errors(&by3), Vec::<String>::new());
    }

    #[test]
    fn third_party_credits_need_title_author_and_link() {
        let bare = FULL.replace(
            r#"title: "Footsteps", author: "someone","#,
            r#"title: " ", author: "","#,
        );
        let bare = bare.replace("https://freesound.org/s/1/", "freesound 1");
        assert_eq!(
            errors(&bare),
            [
                "a.ron:18: credit \"steps\": title is missing",
                "a.ron:18: credit \"steps\": author is missing",
                "a.ron:18: credit \"steps\": source must be a web link (http:// or https://)",
            ]
        );
        let http = FULL.replace("https://freesound", "http://freesound");
        assert_eq!(errors(&http), Vec::<String>::new());
    }

    #[test]
    fn credits_must_exist() {
        let unknown = FULL.replace(r#"Credit("steps")"#, r#"Credit("stepz")"#);
        assert_eq!(
            errors(&unknown),
            ["a.ron:5: \"footfall\" names unknown credit \"stepz\""]
        );
        let unknown = FULL.replace(r#"Credit("sunrise")"#, r#"Credit("dawn")"#);
        assert_eq!(
            errors(&unknown),
            ["a.ron:8: \"title\" names unknown credit \"dawn\""]
        );
    }

    #[test]
    fn a_cue_may_name_several_credits() {
        let several = FULL.replace(r#"Credit("steps")"#, r#"Credits(["steps", "sunrise"])"#);
        let m = from_source("a.ron", &several, &all_files()).unwrap_or_default();
        let ids = m.sounds.get("footfall").map(|s| s.credit.ids().to_vec());
        assert_eq!(ids, Some(vec!["steps".to_owned(), "sunrise".to_owned()]));
        let got = CreditRef::Own.ids();
        assert!(got.is_empty(), "{got:?}");
        let unknown = FULL.replace(r#"Credit("steps")"#, r#"Credits(["steps", "stepz"])"#);
        assert_eq!(
            errors(&unknown),
            ["a.ron:5: \"footfall\" names unknown credit \"stepz\""]
        );
        let empty = FULL.replace(r#"Credit("steps")"#, "Credits([])");
        assert_eq!(
            errors(&empty),
            ["a.ron:5: \"footfall\" has an empty credit list"]
        );
    }

    #[test]
    fn pools_hold_only_music_cues() {
        let sound = FULL.replace(r#""title", "sting""#, r#""title", "ui_tick""#);
        assert_eq!(
            errors(&sound),
            ["a.ron:12: pool \"skirmish\" names \"ui_tick\", a sound; pools hold music cues"]
        );
        let unknown = FULL.replace(r#""title", "sting""#, r#""title", "nope""#);
        assert_eq!(
            errors(&unknown),
            ["a.ron:12: pool \"skirmish\" names unknown music cue \"nope\""]
        );
        let empty = FULL.replace(r#""title", "sting""#, "");
        assert_eq!(errors(&empty), ["a.ron:12: pool \"skirmish\" is empty"]);
    }

    #[test]
    fn volumes_are_percentages() {
        assert_eq!(
            errors(&FULL.replace("volume: 60", "volume: 100")),
            Vec::<String>::new()
        );
        assert_eq!(
            errors(&FULL.replace("volume: 60", "volume: 101")),
            ["a.ron:5: \"footfall\": volume 101 is over 100 (percent)"]
        );
        assert_eq!(
            errors(&FULL.replace("volume: 80", "volume: 101")),
            ["a.ron:8: \"title\": volume 101 is over 100 (percent)"]
        );
    }

    /// [`FULL`] with a `groups` list in front of its sounds.
    fn with_groups(groups: &str) -> String {
        FULL.replacen(
            "    sounds: [",
            &format!("    groups: [\n{groups}\n    ],\n    sounds: ["),
            1,
        )
    }

    #[test]
    fn a_manifest_declares_groups_and_a_cue_names_one() {
        let source = with_groups(
            r#"        (id: "ambience", parent: "sfx", volume: 80),
        (id: "weather"),
        (id: "music", volume: 90),"#,
        )
        .replace(
            "credit: Credit(\"steps\")",
            "credit: Credit(\"steps\"), group: \"ambience\"",
        )
        .replace(
            "looped: false, length_ms: 2000, credit: Own",
            "looped: false, length_ms: 2000, credit: Own, group: \"weather\"",
        );
        let m = from_source("a.ron", &source, &all_files()).unwrap_or_default();
        let ambience = GroupDef {
            parent: Some("sfx".into()),
            volume: Some(80),
        };
        let music = GroupDef {
            volume: Some(90),
            ..GroupDef::default()
        };
        let declared: Vec<_> = m.groups.iter().map(|(id, g)| (id.as_str(), g)).collect();
        assert_eq!(
            declared,
            [
                ("ambience", &ambience),
                ("music", &music),
                ("weather", &GroupDef::default())
            ]
        );
        assert_eq!(m.sounds["footfall"].group.as_deref(), Some("ambience"));
        assert_eq!(m.sounds["ui_tick"].group, None);
        assert_eq!(m.music["sting"].group.as_deref(), Some("weather"));
        assert_eq!(m.music["title"].group, None);
        // A manifest without groups declares none.
        let full = from_source("a.ron", FULL, &all_files()).unwrap_or_default();
        assert!(full.groups.is_empty());
        // A cue may name a built-in group, and a group a parent declared
        // after it.
        let built_in = FULL.replace(
            "credit: Credit(\"steps\")",
            "credit: Credit(\"steps\"), group: \"voice\"",
        );
        assert_eq!(errors(&built_in), Vec::<String>::new());
        let later = with_groups(r#"        (id: "rain", parent: "weather"), (id: "weather"),"#);
        assert_eq!(errors(&later), Vec::<String>::new());
    }

    #[test]
    fn a_groups_errors_are_at_the_line_that_names_it() {
        let dup = with_groups(
            r#"        (id: "ambience"),
        (id: "ambience", volume: 50),"#,
        );
        assert_eq!(errors(&dup), ["a.ron:4: duplicate group id \"ambience\""]);
        let unknown = with_groups(
            r#"        (id: "ambience"),
        (id: "rain", parent: "weather"),"#,
        );
        assert_eq!(
            errors(&unknown),
            ["a.ron:5: group \"rain\" names unknown parent \"weather\""]
        );
        let built_in = with_groups(r#"        (id: "ui", parent: "master"),"#);
        assert_eq!(
            errors(&built_in),
            ["a.ron:4: group \"ui\" is built in: it may set only its volume, not its parent"]
        );
        let loud = with_groups(r#"        (id: "ambience", volume: 101),"#);
        assert_eq!(
            errors(&loud),
            ["a.ron:4: \"ambience\": volume 101 is over 100 (percent)"]
        );
        let empty = with_groups(r#"        (id: ""),"#);
        assert_eq!(errors(&empty), ["a.ron:4: a group id is empty"]);
    }

    #[test]
    fn a_loop_of_parents_is_refused() {
        let looped = with_groups(
            r#"        (id: "a", parent: "b"),
        (id: "b", parent: "a"),
        (id: "c", parent: "b"),"#,
        );
        assert_eq!(
            errors(&looped),
            [
                "a.ron:4: group \"a\" is in a loop of parents: a, b, a",
                "a.ron:5: group \"b\" is in a loop of parents: b, a, b",
            ]
        );
        let own = with_groups(r#"        (id: "a", parent: "a"),"#);
        assert_eq!(
            errors(&own),
            ["a.ron:4: group \"a\" is in a loop of parents: a, a"]
        );
    }

    #[test]
    fn a_cue_naming_an_unknown_group_is_refused() {
        let sound = FULL.replace(
            "credit: Credit(\"steps\")",
            "credit: Credit(\"steps\"), group: \"rain\"",
        );
        assert_eq!(
            errors(&sound),
            ["a.ron:5: sound \"footfall\" names unknown group \"rain\""]
        );
        let music = FULL.replace(
            "credit: Credit(\"sunrise\")",
            "credit: Credit(\"sunrise\"), group: \"rain\"",
        );
        assert_eq!(
            errors(&music),
            ["a.ron:8: music \"title\" names unknown group \"rain\""]
        );
    }

    #[test]
    fn syntax_and_unknown_fields_are_refused() {
        assert_eq!(errors("(").len(), 1);
        assert_eq!(
            errors(&FULL.replace("volume: 80", "volume: 80, loud: true")).len(),
            1
        );
        assert_eq!(errors(&EMPTY.replace("pools: [], ", "")).len(), 1);
    }

    #[test]
    fn the_constructors_give_the_literals() {
        fn none(_: &str) -> Bytes {
            None
        }
        fn shown(name: &str) -> String {
            format!("base/audio/{name}")
        }
        fn kept(_: &str) -> bool {
            false
        }
        /// Files as games build theirs, in a `const` item.
        const FILES: Files<'static> = Files::new(&none, &shown, &kept, "music");
        const WITH_MUSIC: Files<'static> = FILES.with_music(Some(&none));
        assert_eq!(
            SoundCue::new(alloc::vec!["sfx/a.wav".into()], 60, CreditRef::Own),
            SoundCue {
                files: alloc::vec!["sfx/a.wav".into()],
                volume: 60,
                credit: CreditRef::Own,
                looped: false,
                group: None,
                rules: CueRules::default(),
                lengths_ms: alloc::vec![],
            }
        );
        assert!(
            SoundCue::new(Vec::new(), 60, CreditRef::Own)
                .with_looped(true)
                .looped
        );
        assert_eq!(
            SoundCue::new(Vec::new(), 60, CreditRef::Own)
                .with_group("ambience")
                .group
                .as_deref(),
            Some("ambience")
        );
        assert_eq!((FILES.music_dir, FILES.music.is_none()), ("music", true));
        assert_eq!((FILES.sound_shown)("x.wav"), "base/audio/x.wav");
        assert!(!(FILES.sound_left_out)("x.wav"));
        assert!((FILES.sound)("x.wav").is_none());
        assert!(
            WITH_MUSIC
                .music
                .is_some_and(|music| music("a.ogg").is_none())
        );
    }

    /// [`FULL`] with a `ducks` list, and a `groups` list declaring
    /// `ambience` under `sfx` and `chatter` under `voice`, in front of its
    /// sounds: the rules start at line 4.
    fn with_ducks(ducks: &str) -> String {
        FULL.replacen(
            "    sounds: [",
            &format!(
                "    ducks: [\n{ducks}\n    ],\n    groups: [(id: \"ambience\", parent: \"sfx\"), \
                 (id: \"chatter\", parent: \"voice\")],\n    sounds: ["
            ),
            1,
        )
    }

    #[test]
    fn a_manifest_lowers_a_group_while_a_voice_plays() {
        let source = with_ducks(
            r#"        (when: "voice", lower: "music", to: 80, attack_ms: 100, release_ms: 500),
        (when: "voice", lower: "ambience", to: 0, attack_ms: 0, release_ms: 0),"#,
        );
        let m = from_source("a.ron", &source, &all_files()).unwrap_or_default();
        let music = DuckRule {
            when: "voice".into(),
            lower: "music".into(),
            to: 80,
            attack_ms: 100,
            release_ms: 500,
        };
        let ambience = DuckRule {
            lower: "ambience".into(),
            to: 0,
            attack_ms: 0,
            release_ms: 0,
            ..music.clone()
        };
        assert_eq!(m.ducks, [music, ambience], "a declared group too");
        // A manifest without ducks lowers nothing.
        let full = from_source("a.ron", FULL, &all_files()).unwrap_or_default();
        assert_eq!(full.ducks, []);
    }

    #[test]
    fn a_ducking_rules_errors_are_at_the_line_with_its_lower() {
        let rule = |when: &str, lower: &str, to: u8| {
            format!(
                "        (when: \"{when}\", lower: \"{lower}\", to: {to}, attack_ms: 10, \
                 release_ms: 10),"
            )
        };
        let not_voice = with_ducks(&rule("sfx", "music", 80));
        assert_eq!(
            errors(&not_voice),
            [
                "a.ron:4: ducks[0] (lower: \"music\") lowers \"music\" when \"sfx\" plays: only \
                 \"voice\" can, the one group a device says is playing"
            ]
        );
        let unknown = with_ducks(&rule("voice", "rain", 80));
        assert_eq!(
            errors(&unknown),
            ["a.ron:4: ducks[0] (lower: \"rain\") names unknown group \"rain\""]
        );
        for lower in ["voice", "master"] {
            let own = with_ducks(&rule("voice", lower, 80));
            assert_eq!(
                errors(&own),
                [format!(
                    "a.ron:4: ducks[0] (lower: \"{lower}\") lowers \"{lower}\", which \"voice\" \
                     plays under, so would lower it too"
                )]
            );
        }
        // A group under voice isn't voice's parent: lowering it is fine.
        let under = with_ducks(&rule("voice", "chatter", 80));
        assert_eq!(errors(&under), Vec::<String>::new());
        let deep = with_ducks(&rule("voice", "ui", 0));
        assert_eq!(errors(&deep), Vec::<String>::new());
        // 100 is the top of the curve: lowering by nothing is allowed.
        let top = with_ducks(&rule("voice", "music", 100));
        assert_eq!(errors(&top), Vec::<String>::new());
        let loud = with_ducks(&rule("voice", "music", 101));
        assert_eq!(
            errors(&loud),
            [
                "a.ron:4: ducks[0] (lower: \"music\") lowers to 101, over 100 on the loudness \
                 curve"
            ]
        );
        let twice = with_ducks(&format!(
            "{}\n{}",
            rule("voice", "sfx", 90),
            rule("voice", "sfx", 80)
        ));
        assert_eq!(
            errors(&twice),
            ["a.ron:4: ducks[1] (lower: \"sfx\") lowers a group another rule lowers already"]
        );
    }
}
