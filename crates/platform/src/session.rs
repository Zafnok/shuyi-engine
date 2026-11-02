//! Session records: everything the platform gave a game in one run, frame
//! by frame, and its playback (ADR-0072). A record is taken where the
//! platform hands things to [`App`] ([`App::record`]), so no game code
//! records anything: the clock, the system's events, the input, the system
//! keyboard's answers, the music's position, the display, the platform's
//! facts and the store as it was handed over. Each frame also holds a hash
//! of what happened in it, never of its picture.
//!
//! [`Playback`] plays a record on the [`HeadlessPlatform`], from the game's
//! real [`Start`], and says at which frame the game first did something
//! else. Frames count [`App::frame`] calls from 1, as
//! [`App::frame_number`] does.
//!
//! A game's rules records (a seed and commands, a hash per step) are one
//! level up, and stay the game's: playing a session runs the same rules, so
//! a tick game's steps and a turn game's commands come out of it.
//!
//! A record's text is RON, one frame per line, so two records diff frame by
//! frame ([`SessionRecord::to_ron`]). The suggested file name is
//! `*.session.ron`. A record is a run's log, not an asset, so it has no
//! `Format`, and it is never migrated: a field added later is optional and
//! leaves [`FORMAT_VERSION`] alone, and a change to what a field means
//! raises it.

use crate::alloc_prelude::*;
use alloc::collections::BTreeMap;
use alloc::format;
use core::fmt;

use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use shuyi_core::StateHash;
use shuyi_core::made::{Made, Mismatch};
use shuyi_ui::frame_cap::Pacing;
use shuyi_ui::storage::Storage;
use shuyi_ui::system_keyboard::TextAnswer;
use shuyi_ui::{GameData, PlatformFacts, RawInputEvent};

use crate::app::{App, Start};
use crate::headless::HeadlessPlatform;
use crate::platform::{DisplayInfo, Millis, SystemEvent};

/// The version of a record's text this engine reads and writes.
pub const FORMAT_VERSION: u32 = 1;

/// The most frames one record holds: an hour at 60 frames a second.
/// Recording stops there ([`SessionRecord::truncated`]).
pub const MAX_SESSION_FRAMES: usize = 216_000;

/// One run of a game, as the platform gave it ([module docs](self)). `App`
/// fills one in ([`App::session`]); [`from_ron`](Self::from_ron) reads one
/// back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct SessionRecord {
    /// The version of its text ([`FORMAT_VERSION`]).
    pub format_version: u32,
    /// The build that made it ([`Start::made`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub made: Option<Made>,
    /// The display the game's load was given.
    pub display: DisplayInfo,
    /// Where the game ran: what the game's load was given
    /// ([`Platform::facts`](crate::Platform::facts)).
    pub platform: PlatformFacts,
    /// Every key and value in the store when it was handed over.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub storage: BTreeMap<String, String>,
    /// Whether all of the store was read: false if its keys or a value
    /// couldn't be, or it hasn't been handed over.
    pub storage_complete: bool,
    /// The text of the voice manifest a backend put into the game's
    /// [`Setup`](crate::Setup), which `App` can't see: the backend gives it
    /// with [`App::session_mut`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_manifest: Option<String>,
    /// The time of the first frame on the backend's clock.
    pub start: u64,
    /// The first frame whose [`changed_files`](crate::Platform::changed_files)
    /// named files: their new bytes aren't in the record, so frames after it
    /// may play differently.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reloaded_at: Option<u32>,
    /// Whether recording stopped early: at [`MAX_SESSION_FRAMES`], or on an
    /// [`App::frame_after`], whose time isn't whole milliseconds.
    #[serde(default, skip_serializing_if = "core::ops::Not::not")]
    pub truncated: bool,
    /// Its frames, in order.
    pub frames: Vec<FrameRecord>,
}

impl SessionRecord {
    /// A record of no frames, of a game not loaded yet.
    pub fn new() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            made: None,
            display: DisplayInfo::new(0, 0),
            platform: PlatformFacts::default(),
            storage: BTreeMap::new(),
            storage_complete: false,
            voice_manifest: None,
            start: 0,
            reloaded_at: None,
            truncated: false,
            frames: Vec::new(),
        }
    }

    /// The record's text: RON, one frame per line.
    pub fn to_ron(&self) -> String {
        // Depth 2 is the record's fields and the frames' list: each frame
        // stays on its line. RON's line ending otherwise follows the OS, and
        // a record's text is the same everywhere.
        let config = PrettyConfig::new()
            .depth_limit(2)
            .indentor(" ")
            .new_line("\n");
        ron::ser::to_string_pretty(self, config)
            .unwrap_or_else(|error| panic!("a session record has no value RON refuses: {error}"))
    }

    /// The record `text` holds. Another version is refused, whatever else
    /// its text holds: a record is never migrated.
    pub fn from_ron(text: &str) -> Result<Self, SessionError> {
        /// What every version has in common.
        #[derive(Deserialize)]
        struct Head {
            format_version: u32,
        }

        let refused = |version| Err(SessionError::Version(version));
        match ron::from_str::<Self>(text) {
            Ok(record) if record.format_version == FORMAT_VERSION => Ok(record),
            Ok(record) => refused(record.format_version),
            Err(error) => match ron::from_str::<Head>(text) {
                Ok(head) if head.format_version != FORMAT_VERSION => refused(head.format_version),
                _ => Err(SessionError::Text(error.to_string())),
            },
        }
    }

    /// What a playback of this record can't promise, one line each: frames
    /// it doesn't hold, a store it doesn't hold all of, files it doesn't
    /// hold the new bytes of.
    pub fn limits(&self) -> Vec<String> {
        let mut limits = Vec::new();
        if self.truncated {
            limits.push("the record stopped early: it doesn't hold the run's end".to_owned());
        }
        if !self.storage_complete {
            limits.push("the record doesn't hold all of the store".to_owned());
        }
        if let Some(frame) = self.reloaded_at {
            limits.push(format!(
                "files changed at frame {frame}: their new bytes aren't in the record, so frames after it may differ"
            ));
        }
        limits
    }
}

impl Default for SessionRecord {
    fn default() -> Self {
        Self::new()
    }
}

/// One frame of a run, in the order `App` reads the platform: empty lists
/// and unchanged values are left out, so an idle frame is its `dt` alone, or
/// its `dt` and `music` while music sounds.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct FrameRecord {
    /// This frame's time minus the last one's, in milliseconds: signed,
    /// since a clock may go back and `App` must see that again; 0 for the
    /// first frame ([`SessionRecord::start`]).
    pub dt: i64,
    /// The system's events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub system: Vec<SystemEvent>,
    /// The frame's one poll of the input, kept or dropped as `App` did.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input: Vec<RawInputEvent>,
    /// The system keyboard's part in the frame.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keyboard: Option<KeyboardFrame>,
    /// The position [`Audio::music_playing`](crate::Audio::music_playing)
    /// answered, in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub music: Option<f64>,
    /// The cue of the track sounding, when it changes: an empty cue for
    /// silence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track: Option<String>,
    /// The pacing the game was handed, when it changes
    /// ([`App::set_pacing`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pacing: Option<Pacing>,
    /// The frame's hash, when it differs from the last frame's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hash: Option<u64>,
}

/// The system keyboard's part in one frame.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct KeyboardFrame {
    /// Whether the system keyboard is there, when that changes (it isn't,
    /// until said).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub present: Option<bool>,
    /// The answer its poll gave.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answer: Option<TextAnswer>,
    /// What its `open` answered after a screen's ask.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opened: Option<bool>,
}

/// Why a record can't be played.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SessionError {
    /// Its text can't be read: RON's error, with its line and column.
    Text(String),
    /// Its text is of another version.
    Version(u32),
    /// Another build made it.
    Made(Mismatch),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => write!(f, "a session record that can't be read: {text}"),
            Self::Version(version) => write!(
                f,
                "a session record of version {version}; this engine reads version {FORMAT_VERSION}"
            ),
            Self::Made(mismatch) => write!(f, "{mismatch}"),
        }
    }
}

impl core::error::Error for SessionError {}

/// Whether a playback checks that the record's build is this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Stamps {
    /// Refuse a record another build made.
    #[default]
    Check,
    /// Play it anyway: it can differ without a bug in either build.
    Ignore,
}

/// `start` made ready to play `record`: the record's stamp checked against
/// the start's as `stamps` says, and the record's voice manifest put in its
/// setup. Beside it, what the playback can't promise, one line each.
pub fn prepare<G: GameData>(
    start: Start<G>,
    record: &SessionRecord,
    stamps: Stamps,
) -> Result<(Start<G>, Vec<String>), SessionError> {
    let mut limits = record.limits();
    match (&record.made, &start.made) {
        (Some(made), Some(now)) => {
            if stamps == Stamps::Check {
                made.check(now).map_err(SessionError::Made)?;
            }
        }
        (None, _) => limits.push("the record has no stamp: its build isn't checked".to_owned()),
        (_, None) => limits.push("the start has no stamp: the build isn't checked".to_owned()),
    }
    if stamps == Stamps::Ignore {
        limits.push("the stamps were ignored: another build may play differently".to_owned());
    }
    let start = match record.voice_manifest.clone() {
        Some(manifest) => start.before_load(move |setup| setup.voice_manifest = Some(manifest)),
        None => start,
    };
    Ok((start, limits))
}

/// The first frame that played differently from its record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct FrameDifference {
    /// The frame, counted from 1.
    pub frame: u32,
    /// The hash the record holds for it.
    pub recorded: StateHash,
    /// The hash it played with.
    pub played: StateHash,
}

/// A record playing, one frame at a time: on an [`App`] made from the game's
/// [`Start`] ([`prepare`]), on a platform from
/// [`HeadlessPlatform::from_session`]. It needs no Harness, so a running
/// build can replay a record too.
#[derive(Debug, Clone)]
pub struct Playback {
    /// The frames still to play.
    frames: alloc::vec::IntoIter<FrameRecord>,
    /// The time of the last frame played, or the record's start.
    now: u64,
    /// The last hash the record wrote.
    hash: Option<u64>,
}

impl Playback {
    /// The playback of `record`, from its first frame.
    pub fn new(record: &SessionRecord) -> Self {
        Self {
            frames: record.frames.clone().into_iter(),
            now: record.start,
            hash: None,
        }
    }

    /// Plays the next frame at its recorded time, and answers its number,
    /// or how its hash differs; a frame with no hash has the last one
    /// written. `None` once the record is played: from its last frame on,
    /// `platform`'s music and keyboard are its own again.
    pub fn next<G: GameData>(
        &mut self,
        app: &mut App<G>,
        platform: &mut HeadlessPlatform,
    ) -> Option<Result<u32, FrameDifference>> {
        let frame = self.frames.next()?;
        app.hash_frames(true);
        platform.push_frame(&frame);
        if let Some(pacing) = frame.pacing {
            app.set_pacing(pacing);
        }
        self.now = self.now.wrapping_add_signed(frame.dt);
        self.hash = frame.hash.or(self.hash);
        let _ = app.frame(platform, Millis(self.now));
        if self.is_done() {
            platform.end_session();
        }
        let number = app.frame_number();
        let (Some(recorded), Some(played)) = (self.hash.map(StateHash), app.frame_hash()) else {
            return Some(Ok(number));
        };
        Some(if recorded == played {
            Ok(number)
        } else {
            Err(FrameDifference {
                frame: number,
                recorded,
                played,
            })
        })
    }

    /// Whether every frame has played.
    pub fn is_done(&self) -> bool {
        self.frames.as_slice().is_empty()
    }

    /// The time of the last frame played, for a driver that goes on from
    /// the record's end; before the first, the record's start.
    pub fn now(&self) -> Millis {
        Millis(self.now)
    }
}

/// What `App` holds while it records: the record, the frame being noted, and
/// the values a frame writes only when they change.
#[derive(Debug)]
pub(crate) struct Recorder {
    pub(crate) record: SessionRecord,
    /// Whether it still records: it stops for good ([`stop`](Self::stop)).
    on: bool,
    frame: FrameRecord,
    /// The last frame's time.
    now: u64,
    present: bool,
    track: String,
    pacing: Option<Pacing>,
    hash: Option<u64>,
}

impl Recorder {
    pub(crate) fn new(made: Option<Made>) -> Self {
        let mut record = SessionRecord::new();
        record.made = made;
        Self {
            record,
            on: true,
            frame: FrameRecord::default(),
            now: 0,
            present: false,
            track: String::new(),
            pacing: None,
            hash: None,
        }
    }

    pub(crate) fn is_on(&self) -> bool {
        self.on
    }

    /// Stops recording, early: the record says so.
    pub(crate) fn stop(&mut self) {
        if self.on {
            self.on = false;
            self.record.truncated = true;
        }
    }

    /// A frame starts at `now` on the backend's clock.
    pub(crate) fn begin(&mut self, now: u64) {
        if self.record.frames.len() >= MAX_SESSION_FRAMES {
            self.stop();
        }
        if !self.on {
            return;
        }
        self.frame = FrameRecord::default();
        if self.record.frames.is_empty() {
            self.record.start = now;
        } else {
            self.frame.dt = now.wrapping_sub(self.now).cast_signed();
        }
        self.now = now;
    }

    /// The system's events the frame read.
    pub(crate) fn system(&mut self, events: &[SystemEvent]) {
        if self.on {
            self.frame.system.extend_from_slice(events);
        }
    }

    /// What the game's load was handed.
    pub(crate) fn handed_over(
        &mut self,
        display: DisplayInfo,
        storage: &dyn Storage,
        facts: &PlatformFacts,
    ) {
        if self.on {
            self.record.display = display;
            self.record.platform = facts.clone();
            self.record.storage_complete = read_store(storage, &mut self.record.storage);
        }
    }

    /// The files the platform said changed reached the game in `frame`.
    pub(crate) fn reloaded(&mut self, frame: u32) {
        if self.on {
            self.record.reloaded_at.get_or_insert(frame);
        }
    }

    /// The pacing the game was handed.
    pub(crate) fn handed_pacing(&mut self, pacing: Pacing) {
        if self.on && self.pacing != Some(pacing) {
            self.pacing = Some(pacing);
            self.frame.pacing = Some(pacing);
        }
    }

    fn keyboard(&mut self) -> &mut KeyboardFrame {
        self.frame.keyboard.get_or_insert_default()
    }

    /// Whether the system keyboard is there.
    pub(crate) fn keyboard_present(&mut self, present: bool) {
        if self.on && self.present != present {
            self.present = present;
            self.keyboard().present = Some(present);
        }
    }

    /// The answer the system keyboard's poll gave.
    pub(crate) fn keyboard_answer(&mut self, answer: &TextAnswer) {
        if self.on {
            self.keyboard().answer = Some(answer.clone());
        }
    }

    /// What the system keyboard's `open` answered.
    pub(crate) fn keyboard_opened(&mut self, opened: bool) {
        if self.on {
            self.keyboard().opened = Some(opened);
        }
    }

    /// What the sound device said is sounding.
    pub(crate) fn music(&mut self, playing: Option<(&str, f64)>) {
        if !self.on {
            return;
        }
        let cue = playing.map_or("", |(cue, _)| cue);
        if cue != self.track {
            cue.clone_into(&mut self.track);
            self.frame.track = Some(cue.to_owned());
        }
        self.frame.music = playing.map(|(_, position)| position);
    }

    /// The frame ends: its poll of the input, and its hash.
    pub(crate) fn end(&mut self, input: &[RawInputEvent], hash: StateHash) {
        if !self.on {
            return;
        }
        let mut frame = core::mem::take(&mut self.frame);
        frame.input.extend_from_slice(input);
        if self.hash != Some(hash.0) {
            self.hash = Some(hash.0);
            frame.hash = Some(hash.0);
        }
        self.record.frames.push(frame);
    }
}

/// Reads all of `storage` into `into`, and says whether it could.
fn read_store(storage: &dyn Storage, into: &mut BTreeMap<String, String>) -> bool {
    let Ok(keys) = storage.list() else {
        return false;
    };
    let mut complete = true;
    for key in keys {
        if let Ok(Some(value)) = storage.read(&key) {
            into.insert(key, value);
        } else {
            complete = false;
        }
    }
    complete
}

#[cfg(test)]
mod tests;
