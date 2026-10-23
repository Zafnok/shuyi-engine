//! The audio manifest: every sound and music cue a game can play, the
//! music pools, and the credits for every third-party work. Screens name
//! cues; the platform layer plays the files.
//!
//! The game says where the files are ([`Files`]): sounds are usually in
//! the bundle, and music in a folder shipped next to the game, which the
//! game itself may not be able to read. [`from_source`] checks what it can
//! see, including each track's `length_ms` against its file
//! ([`ogg_length_ms`]).
//!
//! As formats (ADR-0007): a sound file's header is the [`Sound`] kind
//! ([`Wav`], [`OggVorbis`]), a music track's the [`MusicTrack`] kind
//! ([`OggVorbisTrack`]), and the manifest is read by
//! [`AudioManifestRon`].
use crate::prelude::*;

mod formats;

use alloc::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::{ContentError, line_of};
use crate::ron::parse_ron;

pub use formats::{
    AudioManifestRon, MAX_SOUND_BYTES, MusicTrack, OggVorbis, OggVorbisTrack, Sound, Wav,
};

/// Licenses a credit may have (game ADR-0013, game ADR-0027). `Own` is work we made.
pub const LICENSES: [&str; 4] = ["CC0-1.0", "CC-BY-3.0", "CC-BY-4.0", OWN];

/// The license of work we made ourselves.
pub const OWN: &str = "Own";

/// Highest cue volume, in percent of the file's own loudness.
pub const MAX_VOLUME: u8 = 100;

/// How far a music cue's `length_ms` may be from its file's real length,
/// in milliseconds.
pub const LENGTH_TOLERANCE_MS: u32 = 20;

/// The validated audio manifest.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AudioManifest {
    /// How long a music track fades out before the next one starts, in
    /// milliseconds (`audio.md`: 0.5 s, *tunable*).
    pub music_fade_ms: u32,
    /// Sound cues by id.
    pub sounds: BTreeMap<String, SoundCue>,
    /// Music cues by id.
    pub music: BTreeMap<String, MusicCue>,
    /// Music pools by id: the music cues one is picked from.
    pub pools: BTreeMap<String, Vec<String>>,
    /// Credits by id.
    pub credits: BTreeMap<String, Credit>,
}

/// A sound effect: one of its files plays each time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundCue {
    /// The variants, relative to `base/audio/`; one is picked at random
    /// each time the cue plays. At least one.
    pub files: Vec<String>,
    /// Volume in percent (0–100).
    pub volume: u8,
    /// Who made it.
    pub credit: CreditRef,
}

/// A music track.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MusicCue {
    /// The track, relative to the `music/` folder.
    pub file: String,
    /// Volume in percent (0–100).
    pub volume: u8,
    /// Whether it loops (the default) or plays once.
    pub looped: bool,
    /// How long the track plays, in milliseconds (never 0): where the music
    /// clock wraps a looped track (game ADR-0037). The game can't measure it
    /// (the player library gives no length), so it is data, checked
    /// against the file by a test.
    pub length_ms: u32,
    /// Who made it.
    pub credit: CreditRef,
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
/// does no file I/O).
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

/// The sample rate every file must have: quad-snd (native) plays at
/// 44.1 kHz and resamples anything else crudely (game ADR-0026).
pub const SAMPLE_RATE: u32 = 44_100;

/// Why `bytes` (a file named `name`) isn't something the game can play:
/// its content must match its extension (OGG Vorbis for `.ogg`, PCM or
/// float WAV for `.wav`), with one or two channels at [`SAMPLE_RATE`].
/// quad-snd panics on native, and never finishes loading on the web, on a
/// file it can't read, so this runs before any file ships.
pub fn format_problem(name: &str, bytes: &[u8]) -> Option<String> {
    let format = if has_extension(name, &["wav"]) {
        wav_format(bytes).ok_or("not a PCM WAV file")
    } else {
        vorbis_format(bytes).ok_or("not an OGG Vorbis file")
    };
    match format {
        Err(why) => Some(why.to_owned()),
        Ok((channels, rate)) => playable_problem(channels, rate),
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

/// A WAV's (channels, sample rate), from its `fmt ` chunk.
fn wav_format(bytes: &[u8]) -> Option<(u16, u32)> {
    if bytes.get(..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut at = 12;
    loop {
        let id = bytes.get(at..at + 4)?;
        let size = usize::try_from(u32_at(bytes, at + 4)?).ok()?;
        if id == b"fmt " {
            // 1 = integer PCM, 3 = float: what quad-snd's decoder reads.
            let encoding = u16_at(bytes, at + 8)?;
            return [1, 3]
                .contains(&encoding)
                .then_some((u16_at(bytes, at + 10)?, u32_at(bytes, at + 12)?));
        }
        at = at.checked_add(size.checked_add(8 + size % 2)?)?;
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
    let rate = u64::from(rate);
    let samples = last_granule(bytes)?;
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
    let raw: RawFile = parse_ron(file, source).map_err(|e| vec![e])?;
    let mut check = Check {
        file,
        source,
        files,
        errors: Vec::new(),
        cue_ids: BTreeSet::new(),
        manifest: AudioManifest {
            music_fade_ms: raw.music_fade_ms,
            ..AudioManifest::default()
        },
    };
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
        for f in &s.files {
            let path = (self.files.sound_shown)(f);
            if !has_extension(f, &["ogg", "wav"]) {
                self.err(&id, format!("sound \"{id}\": \"{f}\" must be .ogg or .wav"));
            } else if !(self.files.sound_left_out)(f) {
                match (self.files.sound)(f) {
                    None => self.err(&id, format!("sound \"{id}\": file {path} not found")),
                    Some(bytes) => {
                        if let Some(why) = format_problem(f, &bytes) {
                            self.err(&id, format!("sound \"{id}\": {path}: {why}"));
                        }
                    }
                }
            }
        }
        self.volume(&id, s.volume);
        self.credit_ref(&id, &s.credit);
        let entry = SoundCue {
            files: s.files,
            volume: s.volume,
            credit: s.credit,
        };
        self.manifest.sounds.insert(id, entry);
    }

    fn music(&mut self, m: RawMusic) {
        let id = m.id;
        self.new_cue(&id);
        let f = &m.file;
        if !has_extension(f, &["ogg"]) {
            self.err(&id, format!("music \"{id}\": \"{f}\" must be .ogg"));
        } else if let Some(read) = self.files.music {
            let dir = self.files.music_dir;
            match read(f) {
                None => self.err(&id, format!("music \"{id}\": file {dir}/{f} not found")),
                Some(bytes) => {
                    let problem = format_problem(f, &bytes)
                        .or_else(|| length_problem(m.length_ms, ogg_length_ms(&bytes)));
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
        self.volume(&id, m.volume);
        self.credit_ref(&id, &m.credit);
        let entry = MusicCue {
            file: m.file,
            volume: m.volume,
            looped: m.looped,
            length_ms: m.length_ms,
            credit: m.credit,
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

/// What a track whose length can't be read is told.
const NO_LENGTH: &str = "its length can't be read (no last Ogg page)";

/// Why a music cue's `length_ms` doesn't fit its file, which plays for
/// `actual` milliseconds ([`ogg_length_ms`]): more than
/// [`LENGTH_TOLERANCE_MS`] off, or the file's length can't be read. The
/// message gives the value to write.
fn length_problem(length_ms: u32, actual: Option<u32>) -> Option<String> {
    match actual {
        None => Some(NO_LENGTH.into()),
        Some(ms) if ms.abs_diff(length_ms) > LENGTH_TOLERANCE_MS => Some(format!(
            "it plays for {ms} ms, but length_ms is {length_ms}; write length_ms: {ms}"
        )),
        Some(_) => None,
    }
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
mod tests {
    use alloc::borrow::Cow;

    use super::*;

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
    fn wav(channels: u16, rate: u32, encoding: u16) -> Vec<u8> {
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
    fn ogg_file(rate: u32, samples: u64) -> Vec<u8> {
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
            m.sounds.get("ui_tick").map(|s| &s.credit),
            Some(&CreditRef::Own)
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
        let wrong = FULL.replace("sfx/step_2.ogg", "sfx/step_2.mp3");
        let files = Files {
            sound_left_out: &|p: &str| p == "sfx/step_2.mp3",
            ..reading(&no_step_2, None)
        };
        assert_eq!(
            errors_with(&wrong, &files),
            ["a.ron:5: sound \"footfall\": \"sfx/step_2.mp3\" must be .ogg or .wav"]
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

    #[test]
    fn file_formats_are_checked() {
        let mp3 = FULL.replace("title.ogg", "title.mp3");
        assert_eq!(
            errors(&mp3),
            ["a.ron:8: music \"title\": \"title.mp3\" must be .ogg"]
        );
        let wav_music = FULL.replace("title.ogg", "title.wav");
        assert_eq!(errors(&wav_music).len(), 1);
        let flac = FULL.replace("ui_tick.wav", "ui_tick.flac");
        assert_eq!(
            errors(&flac),
            ["a.ron:4: sound \"ui_tick\": \"sfx/ui_tick.flac\" must be .ogg or .wav"]
        );
        assert_eq!(errors(&FULL.replace("sfx/ui_tick.wav", ".wav")).len(), 1);
        assert_eq!(errors(&FULL.replace("sfx/ui_tick.wav", "wav")).len(), 1);
        let none = FULL.replace(r#"["sfx/ui_tick.wav"]"#, "[]");
        assert_eq!(errors(&none), ["a.ron:4: sound \"ui_tick\" has no files"]);
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

    #[test]
    fn syntax_and_unknown_fields_are_refused() {
        assert_eq!(errors("(").len(), 1);
        assert_eq!(
            errors(&FULL.replace("volume: 80", "volume: 80, loud: true")).len(),
            1
        );
        assert_eq!(errors(&EMPTY.replace("pools: [], ", "")).len(), 1);
    }
}
