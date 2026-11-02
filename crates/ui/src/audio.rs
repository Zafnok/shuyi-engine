//! Audio as data: screens ask for sounds and music through
//! [`Ctx::audio`](crate::Ctx::audio); [`Game`](crate::Game) hands the
//! requests, and the [`MusicState`]'s commands, to the platform in each
//! [`FrameOutput`](crate::FrameOutput). Nothing here plays anything, so
//! screens stay testable without a sound card.
//!
//! Voice clips are asked for by dialogue line id, through
//! [`Ctx::play_voice`](crate::Ctx::play_voice), which knows which lines
//! have a clip that may be played. `V` is the game's
//! [`ClipVariant`]: which of a line's clips plays.
//!
//! The engine's widgets and screens play the same few sounds for the same
//! things on every screen ([`MenuSound`]); which cues those are is the
//! game's ([`AudioConfig`], from [`GameData::audio_config`]).
//!
//! A sound that must be stopped (rain, a hum, a cue that loops) is started
//! with a handle: [`AudioQueue::start_sound`] gives its [`SoundId`], which
//! [`stop_sound`](AudioQueue::stop_sound), with or without a fade, and
//! [`set_sound_volume`](AudioQueue::set_sound_volume) take. It belongs to
//! the screen that started it: when that screen leaves the stack, its
//! sounds stop, so a loop can't outlive it. A loop meant to outlive a
//! screen is started by the screen below it, or is music.
//!
//! ```
//! # use shuyi_ui::audio::{AudioQueue, AudioRequest};
//! let mut audio = AudioQueue::<()>::default();
//! audio.play_sound("menu_beep");
//! audio.play_music("theme");
//! assert_eq!(audio.pending()[1], AudioRequest::play_music("theme", None));
//! ```

use crate::prelude::*;
use shuyi_assets::audio::AudioManifest;
pub use shuyi_assets::audio::MusicSwitch;
use shuyi_assets::dialogue::LineId;
#[cfg(doc)]
use shuyi_assets::voice::ClipVariant;

#[cfg(doc)]
use crate::GameData;
use crate::input::frame_seconds;
use alloc::collections::{BTreeMap, BTreeSet};

mod handles;

use handles::Handle;

/// The sounds the engine's widgets and screens play: the same cue means
/// the same thing on every screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuSound {
    /// The highlight moved.
    Move,
    /// Something was confirmed or opened.
    Select,
    /// Something was backed out of or closed.
    Cancel,
    /// Confirm on something that can't be chosen, e.g. a greyed-out item.
    Denied,
}

/// Which of the game's sound cues the engine's widgets and screens play
/// ([`MenuSound`], and the cursor's tick): the game's
/// ([`GameData::audio_config`]). The default names them after what they
/// are (`ui_move`, `ui_select`, `ui_cancel`, `ui_cursor`), with Denied
/// playing `ui_cancel`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioConfig {
    /// [`MenuSound::Move`]'s cue.
    pub move_cue: &'static str,
    /// [`MenuSound::Select`]'s cue.
    pub select_cue: &'static str,
    /// [`MenuSound::Cancel`]'s cue.
    pub cancel_cue: &'static str,
    /// [`MenuSound::Denied`]'s cue.
    pub denied_cue: &'static str,
    /// The cue a cursor plays once per step it moves.
    pub cursor_cue: &'static str,
}

impl AudioConfig {
    /// The cues named after what they are.
    pub const DEFAULT: Self = Self {
        move_cue: "ui_move",
        select_cue: "ui_select",
        cancel_cue: "ui_cancel",
        denied_cue: "ui_cancel",
        cursor_cue: "ui_cursor",
    };

    /// The cue of `sound`.
    pub const fn cue(&self, sound: MenuSound) -> &'static str {
        match sound {
            MenuSound::Move => self.move_cue,
            MenuSound::Select => self.select_cue,
            MenuSound::Cancel => self.cancel_cue,
            MenuSound::Denied => self.denied_cue,
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// A sound started with a handle ([`AudioQueue::start_sound`]): the
/// queue's count of them, from 1, so the same calls give the same ids on
/// every run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub struct SoundId(u32);

impl SoundId {
    /// The count behind it.
    pub const fn get(self) -> u32 {
        self.0
    }
}

/// Something a screen wants heard. Cue ids are the ones in the game's
/// audio manifest ([`AudioManifest`]).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum AudioRequest<V> {
    /// Play a sound cue once, at `volume` (0–1) times the cue's own volume.
    PlaySound {
        /// The sound cue.
        cue: String,
        /// A multiplier on the cue's volume, 0–1.
        volume: f32,
    },
    /// Play a sound cue until [`StopSound`](Self::StopSound) of its `id`
    /// (looped if the cue says so, else once), at `volume` (0–1) times the
    /// cue's own volume.
    StartSound {
        /// Its handle.
        id: SoundId,
        /// The sound cue.
        cue: String,
        /// A multiplier on the cue's volume, 0–1.
        volume: f32,
    },
    /// Stop the sound started as `id`, at once.
    StopSound {
        /// Its handle.
        id: SoundId,
    },
    /// Set the volume of the sound started as `id`.
    SoundVolume {
        /// Its handle.
        id: SoundId,
        /// The new multiplier on the cue's volume, 0–1.
        volume: f32,
    },
    /// Pause every sound started with a handle that is playing now
    /// ([`AudioQueue::pause_sounds`]); one started after it plays. Sound
    /// cues played once, the music and the voice go on.
    PauseSounds,
    /// Resume the sounds [`PauseSounds`](Self::PauseSounds) paused.
    ResumeSounds,
    /// Switch to a music cue. Asking for the track already playing does
    /// nothing. Build it with [`AudioRequest::play_music`] (ADR-0039).
    #[non_exhaustive]
    PlayMusic {
        /// The music cue.
        cue: String,
        /// How the track playing gives way to it; `None` for the audio
        /// manifest's default ([`AudioManifest::music_switch`]).
        switch: Option<MusicSwitch>,
    },
    /// Fade the music out.
    StopMusic,
    /// Say a dialogue line: play its voice clip once, stopping the voice
    /// that is playing (one voice at a time).
    PlayVoice {
        /// The line.
        line: LineId,
        /// Which of the line's clips.
        variant: V,
    },
    /// Stop the voice that is playing, if any.
    StopVoice,
    /// The clips a scene will ask for, in script order, so the platform can load
    /// the next few ahead of each [`PlayVoice`](Self::PlayVoice). Replaces
    /// the list sent before.
    PreloadVoices {
        /// The clips, in the order they are expected.
        lines: Vec<(LineId, V)>,
    },
}

impl<V> AudioRequest<V> {
    /// A [`PlayMusic`](Self::PlayMusic) of `cue`, switching with `switch`
    /// (`None`: the manifest's default).
    pub fn play_music(cue: impl Into<String>, switch: Option<MusicSwitch>) -> Self {
        Self::PlayMusic {
            cue: cue.into(),
            switch,
        }
    }

    /// The cue this request names, if any: a sound played or started, or
    /// a track.
    pub fn cue(&self) -> Option<&str> {
        match self {
            Self::PlaySound { cue, .. }
            | Self::StartSound { cue, .. }
            | Self::PlayMusic { cue, .. } => Some(cue),
            Self::StopSound { .. }
            | Self::SoundVolume { .. }
            | Self::PauseSounds
            | Self::ResumeSounds
            | Self::StopMusic
            | Self::PlayVoice { .. }
            | Self::StopVoice
            | Self::PreloadVoices { .. } => None,
        }
    }
}

/// The requests screens made this frame. `Game` empties it every frame.
/// It knows which cues the [`MenuSound`]s are ([`AudioConfig`]), and the
/// sounds started with a handle that haven't stopped.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioQueue<V> {
    requests: Vec<AudioRequest<V>>,
    config: AudioConfig,
    /// The last [`SoundId`] given; 0 before the first.
    last_id: u32,
    /// The sounds started with a handle and not yet stopped.
    handles: BTreeMap<SoundId, Handle>,
    /// The depth on the screen stack of the screen running now: the owner
    /// of the sounds it starts.
    owner: usize,
    /// The owners pausing the sounds now: the depths of screens, and
    /// the owner outside the stack that no pop releases.
    pauses: BTreeSet<usize>,
}

impl<V> Default for AudioQueue<V> {
    fn default() -> Self {
        Self::new(AudioConfig::DEFAULT)
    }
}

impl<V> AudioQueue<V> {
    /// An empty queue that plays the [`MenuSound`]s with `config`'s cues.
    pub fn new(config: AudioConfig) -> Self {
        Self {
            requests: Vec::new(),
            config,
            last_id: 0,
            handles: BTreeMap::new(),
            owner: 0,
            pauses: BTreeSet::new(),
        }
    }

    /// Which cues the [`MenuSound`]s are.
    pub fn config(&self) -> AudioConfig {
        self.config
    }

    /// Plays a menu sound: its cue in the [`config`](Self::config).
    pub fn menu(&mut self, sound: MenuSound) {
        self.play_sound(self.config.cue(sound));
    }

    /// Plays the cursor's tick, once per step it moves.
    pub fn cursor_tick(&mut self) {
        self.play_sound(self.config.cursor_cue);
    }

    /// Plays sound `cue` once at its own volume.
    pub fn play_sound(&mut self, cue: &str) {
        self.play_sound_at(cue, 1.0);
    }

    /// Plays sound `cue` once at `volume` (0–1, clamped) times its own
    /// volume.
    pub fn play_sound_at(&mut self, cue: &str, volume: f32) {
        self.requests.push(AudioRequest::PlaySound {
            cue: cue.to_owned(),
            volume: clamp_volume(volume),
        });
    }

    /// Switches the music to `cue` (nothing happens if it's already on),
    /// as the audio manifest's default switch says.
    pub fn play_music(&mut self, cue: &str) {
        self.requests.push(AudioRequest::play_music(cue, None));
    }

    /// Switches the music to `cue` as `switch` says (nothing happens if
    /// it's already on): how long the old track fades out and the new one
    /// fades in, and whether the two overlap.
    pub fn play_music_with(&mut self, cue: &str, switch: MusicSwitch) {
        self.requests
            .push(AudioRequest::play_music(cue, Some(switch)));
    }

    /// Fades the music out.
    pub fn stop_music(&mut self) {
        self.requests.push(AudioRequest::StopMusic);
    }

    /// Plays the `variant` clip of `line`. Screens call
    /// [`Ctx::play_voice`](crate::Ctx::play_voice) instead, which asks only
    /// for clips that exist and only while voices are on.
    pub fn play_voice(&mut self, line: &LineId, variant: V) {
        self.requests.push(AudioRequest::PlayVoice {
            line: line.clone(),
            variant,
        });
    }

    /// Stops the voice that is playing, if any.
    pub fn stop_voice(&mut self) {
        self.requests.push(AudioRequest::StopVoice);
    }

    /// Tells the platform which clips are coming, in order
    /// ([`Ctx::preload_voices`](crate::Ctx::preload_voices)).
    pub fn preload_voices(&mut self, lines: Vec<(LineId, V)>) {
        self.requests.push(AudioRequest::PreloadVoices { lines });
    }

    /// The requests made since the last [`take`](Self::take), in order.
    pub fn pending(&self) -> &[AudioRequest<V>] {
        &self.requests
    }

    /// Empties the queue, returning its requests in order.
    pub fn take(&mut self) -> Vec<AudioRequest<V>> {
        core::mem::take(&mut self.requests)
    }
}

/// `volume` within 0–1; NaN is 0.
fn clamp_volume(volume: f32) -> f32 {
    if volume.is_nan() {
        0.0
    } else {
        volume.clamp(0.0, 1.0)
    }
}

/// The music cue pool `pool` yields for `seed`: the same seed always gives
/// the same cue. `None` if the manifest has no such pool or it is empty.
/// This is not a game's simulation RNG, so a pick never changes a
/// battle or its replay.
pub fn pick_from_pool<'a>(manifest: &'a AudioManifest, pool: &str, seed: u64) -> Option<&'a str> {
    let cues = manifest.pools.get(pool)?;
    let len = u64::try_from(cues.len()).ok().filter(|&n| n > 0)?;
    let index = usize::try_from(splitmix64(seed) % len).ok()?;
    cues.get(index).map(String::as_str)
}

/// One step of splitmix64: spreads nearby seeds (e.g. a counter) over the
/// whole range.
pub fn splitmix64(seed: u64) -> u64 {
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Where the music is in its track, for a screen that keeps time with it,
/// read from [`Ctx::music_clock`](crate::Ctx::music_clock). The platform
/// reports what really sounds (a track starts only once its file has
/// loaded); [`Game::set_music_playing`](crate::Game::set_music_playing)
/// turns that into this.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicClock {
    /// The music cue sounding.
    pub cue: String,
    /// Seconds into the track: from 0 up to, never reaching, `length`. A
    /// looped track is back at 0 each time it starts again.
    pub position: f32,
    /// The track's length in seconds (the manifest's `length_ms`).
    pub length: f32,
}

impl MusicClock {
    /// The clock of `cue`, which started `elapsed` seconds ago (a negative
    /// time counts as 0). `None` if the cue is unknown, or isn't looped and
    /// has ended (or `elapsed` isn't a number).
    pub fn from_elapsed(manifest: &AudioManifest, cue: &str, elapsed: f64) -> Option<Self> {
        let track = manifest.music.get(cue)?;
        let length = f64::from(track.length_ms) / 1000.0;
        if !elapsed.is_finite() || track.length_ms == 0 {
            return None;
        }
        let elapsed = elapsed.max(0.0);
        let position = if track.looped {
            elapsed % length
        } else {
            elapsed
        };
        // Seconds into one track: f32 is exact to far less than a millisecond.
        #[allow(clippy::cast_possible_truncation)]
        let (position, length) = (position as f32, length as f32);
        let position = if position < length {
            position
        } else if track.looped {
            // Rounded up onto the length: that is the loop point.
            0.0
        } else {
            return None;
        };
        Some(Self {
            cue: cue.to_owned(),
            position,
            length,
        })
    }
}

/// What the platform must do to the music this frame. A track is loaded, started,
/// has its volume changed and is stopped, which also unloads it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum MusicCommand {
    /// Start loading `cue`'s file; it will be started soon.
    Load {
        /// The music cue.
        cue: String,
    },
    /// Play `cue` (loaded earlier) from the start at `gain`, looped if the
    /// manifest says so. If it isn't loaded yet, play it once it is. Build
    /// it with [`MusicCommand::start`] (ADR-0039).
    #[non_exhaustive]
    Start {
        /// The music cue.
        cue: String,
        /// The gain it starts at (0–1, a multiplier on its manifest
        /// volume): 1 for a track that starts at full gain, 0 for one that
        /// fades in.
        gain: f32,
    },
    /// Set `cue`'s gain (0–1, a multiplier on its manifest volume).
    Gain {
        /// The music cue.
        cue: String,
        /// The new gain.
        gain: f32,
    },
    /// Stop `cue` and free it (also cancels a load).
    Stop {
        /// The music cue.
        cue: String,
    },
}

impl MusicCommand {
    /// A [`Start`](Self::Start) of `cue` at `gain`.
    pub fn start(cue: impl Into<String>, gain: f32) -> Self {
        Self::Start {
            cue: cue.into(),
            gain,
        }
    }
}

/// Which tracks play, and how one gives way to the next (ADR-0054): each
/// switch is a [`MusicSwitch`], the request's own or the default one. The
/// old track fades out; the new one starts once it has stopped, or as it
/// begins to fade (a crossfade), and may fade in. A cue already playing
/// doesn't restart, and at most two tracks are loaded at once. A fade-in,
/// and an old track's fade in a crossfade, wait until the new track sounds
/// ([`heard`](Self::heard)), or [`LOAD_WAIT_SECS`](Self::LOAD_WAIT_SECS).
/// Pure: feed it requests, time and what sounds, read the commands.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicState {
    /// The switch of a request that names none, and the fade of a stop.
    default: MusicSwitch,
    /// The older track: the one playing, or fading out during a switch.
    current: Option<Track>,
    /// The newer track: the one a switch brings in, waiting for `current`
    /// to stop or sounding beside it; after a stop during a crossfade,
    /// fading out beside it. Never set without `current`.
    next: Option<Track>,
}

/// One track between its `Load` and its `Stop`.
#[derive(Debug, Clone, PartialEq)]
struct Track {
    cue: String,
    /// Whether it was sent `Start`.
    started: bool,
    /// Whether the music clock has named it since its `Start`.
    heard: bool,
    /// Seconds since its `Start`, while it isn't heard (up to the load
    /// wait).
    waited: f32,
    /// Its gain, rising to 1 (playing or fading in) or falling to 0
    /// (fading out, then stopped).
    ramp: Ramp,
}

/// A gain moving in a straight line to 0 or 1, at one whole fade per
/// `secs`: a fade from a gain part-way takes its share of the time.
#[derive(Debug, Clone, PartialEq)]
struct Ramp {
    /// The gain when it began.
    from: f32,
    /// Whether it ends at 1 (else at 0).
    rising: bool,
    /// Seconds a fade from one end to the other takes.
    secs: f32,
    /// Seconds it has run.
    elapsed: f32,
}

impl Ramp {
    /// A gain that stays at `gain`.
    fn at(gain: f32) -> Self {
        Self {
            from: gain,
            rising: gain >= 1.0,
            secs: 0.0,
            elapsed: 0.0,
        }
    }

    fn to(&self) -> f32 {
        if self.rising { 1.0 } else { 0.0 }
    }

    /// Whether it has reached its end.
    fn done(&self) -> bool {
        self.elapsed >= self.secs * (self.from - self.to()).abs()
    }

    fn gain(&self) -> f32 {
        if self.done() {
            return self.to();
        }
        // Not done, so `secs` is above 0.
        let moved = self.elapsed / self.secs;
        let gain = if self.rising {
            self.from + moved
        } else {
            self.from - moved
        };
        gain.clamp(0.0, 1.0)
    }
}

impl Track {
    /// `cue`, loaded and not started: it starts at full gain, or at 0 to
    /// fade in over `switch`'s `in_ms`.
    fn new(cue: &str, switch: MusicSwitch) -> Self {
        let from = if switch.in_ms == 0 { 1.0 } else { 0.0 };
        Self {
            cue: cue.into(),
            started: false,
            heard: false,
            waited: 0.0,
            ramp: Ramp {
                from,
                rising: true,
                secs: secs(switch.in_ms),
                elapsed: 0.0,
            },
        }
    }

    fn start(&mut self, out: &mut Vec<MusicCommand>) {
        self.started = true;
        out.push(MusicCommand::start(self.cue.clone(), self.ramp.gain()));
    }

    /// Sends the gain toward 1 (`rising`) or 0, a whole fade taking `ms`.
    /// A ramp already going there at that pace keeps its progress.
    fn aim(&mut self, rising: bool, ms: u32) {
        let secs = secs(ms);
        if self.ramp.rising != rising || self.ramp.secs.to_bits() != secs.to_bits() {
            self.ramp = Ramp {
                from: self.ramp.gain(),
                rising,
                secs,
                elapsed: 0.0,
            };
        }
    }

    /// Whether it sounds, or has been waited for long enough.
    fn sounds(&self) -> bool {
        self.heard || self.waited >= MusicState::LOAD_WAIT_SECS
    }

    /// Whether it is started, fading in and not yet sounding: the old
    /// track holds its gain until it does.
    fn coming_in_unheard(&self) -> bool {
        self.started && self.ramp.rising && !self.sounds()
    }

    /// Counts `dt` more seconds waited for it to sound.
    fn wait(&mut self, dt: f32) {
        if self.started && !self.heard {
            self.waited = (self.waited + dt).min(MusicState::LOAD_WAIT_SECS);
        }
    }

    /// Advances its fade by `dt`, appending the gain, or the stop when a
    /// fade-out ends (then it returns `true`). A track waiting to start,
    /// or to sound before it fades in, doesn't move.
    fn step(&mut self, dt: f32, out: &mut Vec<MusicCommand>) -> bool {
        if !self.started {
            return false;
        }
        let rising = self.ramp.rising;
        if rising && (self.ramp.done() || !self.sounds()) {
            return false;
        }
        self.ramp.elapsed += dt;
        if !rising && self.ramp.done() {
            out.push(MusicCommand::Stop {
                cue: self.cue.clone(),
            });
            return true;
        }
        out.push(MusicCommand::Gain {
            cue: self.cue.clone(),
            gain: self.ramp.gain(),
        });
        false
    }
}

/// `ms` in seconds.
fn secs(ms: u32) -> f32 {
    // A fade is far shorter than 2^24 ms.
    #[allow(clippy::cast_precision_loss)]
    let ms = ms as f32;
    ms / 1000.0
}

impl MusicState {
    /// How long a fade waits for the new track to sound, in seconds: after
    /// that it runs anyway, so a file that never loads never holds the old
    /// track.
    pub const LOAD_WAIT_SECS: f32 = 3.0;

    /// Silence, with switches that fade out over `fade_secs` (to the
    /// millisecond; negative or NaN counts as 0), then start the new track
    /// at full gain ([`MusicSwitch::after_fade`]).
    pub fn new(fade_secs: f32) -> Self {
        // Clamped to 0 above, and a fade is far shorter than 2^32 ms.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ms = (fade_secs.max(0.0) * 1000.0).round() as u32;
        Self::with_switch(MusicSwitch::after_fade(ms))
    }

    /// Silence, switching with `default` when a request names no switch,
    /// and stopping with its `out_ms`.
    pub fn with_switch(default: MusicSwitch) -> Self {
        Self {
            default,
            current: None,
            next: None,
        }
    }

    /// The track playing, or fading out during a switch.
    pub fn current(&self) -> Option<&str> {
        self.current.as_ref().map(|t| t.cue.as_str())
    }

    /// The track a switch brings in, or the current one; `None` while the
    /// music fades to silence.
    pub fn target(&self) -> Option<&str> {
        [&self.next, &self.current]
            .into_iter()
            .flatten()
            .find(|t| t.ramp.rising)
            .map(|t| t.cue.as_str())
    }

    /// Tells it what sounds: the music clock the platform reported for this
    /// frame ([`Ctx::music_clock`](crate::Ctx::music_clock)). A track sounds
    /// once the clock names it, and stays sounding after.
    /// [`Game::set_music_playing`](crate::Game::set_music_playing) calls it
    /// every frame.
    pub fn heard(&mut self, clock: Option<&MusicClock>) {
        let Some(clock) = clock else {
            return;
        };
        for track in [&mut self.current, &mut self.next].into_iter().flatten() {
            if track.started && track.cue == clock.cue {
                track.heard = true;
            }
        }
    }

    /// Applies one request (sound and voice requests are ignored),
    /// appending the commands it needs to `out`.
    pub fn request<V>(&mut self, request: &AudioRequest<V>, out: &mut Vec<MusicCommand>) {
        match request {
            AudioRequest::PlaySound { .. }
            | AudioRequest::StartSound { .. }
            | AudioRequest::StopSound { .. }
            | AudioRequest::SoundVolume { .. }
            | AudioRequest::PauseSounds
            | AudioRequest::ResumeSounds
            | AudioRequest::PlayVoice { .. }
            | AudioRequest::StopVoice
            | AudioRequest::PreloadVoices { .. } => {}
            AudioRequest::PlayMusic { cue, switch } => {
                self.play(cue, switch.unwrap_or(self.default), out);
            }
            AudioRequest::StopMusic => self.stop(out),
        }
    }

    fn play(&mut self, cue: &str, switch: MusicSwitch, out: &mut Vec<MusicCommand>) {
        if self.current.is_none() {
            let mut track = Track::new(cue, switch);
            out.push(MusicCommand::Load { cue: cue.into() });
            track.start(out);
            self.current = Some(track);
            return;
        }
        if self.target() == Some(cue) {
            return;
        }
        self.drop_waiting(out);
        if let Some(mut back) = self.take(cue) {
            // Asked again for a track that is fading out: it comes back,
            // and the other one goes.
            if let Some(other) = &mut self.current {
                other.aim(false, switch.out_ms);
            }
            if switch.in_ms == 0 {
                back.ramp = Ramp::at(1.0);
                out.push(MusicCommand::Gain {
                    cue: cue.into(),
                    gain: 1.0,
                });
            } else {
                back.aim(true, switch.in_ms);
            }
            self.push(back);
            return;
        }
        if self.next.is_some() {
            // Two tracks sound: the older one stops at once.
            if let Some(older) = self.current.take() {
                out.push(MusicCommand::Stop { cue: older.cue });
            }
            self.current = self.next.take();
        }
        if let Some(old) = &mut self.current {
            old.aim(false, switch.out_ms);
        }
        out.push(MusicCommand::Load { cue: cue.into() });
        let mut track = Track::new(cue, switch);
        if switch.overlap {
            track.start(out);
        }
        self.next = Some(track);
    }

    /// Stops a track waiting for the old one to fade: a newer request wins.
    fn drop_waiting(&mut self, out: &mut Vec<MusicCommand>) {
        if let Some(next) = self.next.take_if(|t| !t.started) {
            out.push(MusicCommand::Stop { cue: next.cue });
        }
    }

    /// Takes `cue`'s track out, if it has one; the other stays `current`.
    fn take(&mut self, cue: &str) -> Option<Track> {
        if let Some(next) = self.next.take_if(|t| t.cue == cue) {
            return Some(next);
        }
        let current = self.current.take_if(|t| t.cue == cue)?;
        self.current = self.next.take();
        Some(current)
    }

    /// Adds `track` as the newer one.
    fn push(&mut self, track: Track) {
        if self.current.is_none() {
            self.current = Some(track);
        } else {
            self.next = Some(track);
        }
    }

    fn stop(&mut self, out: &mut Vec<MusicCommand>) {
        self.drop_waiting(out);
        for track in [&mut self.current, &mut self.next].into_iter().flatten() {
            track.aim(false, self.default.out_ms);
        }
    }

    /// Advances the fades by `dt` seconds, appending the gain changes, the
    /// stop of a track faded out and the start of the one waiting for it.
    pub fn update(&mut self, dt: f32, out: &mut Vec<MusicCommand>) {
        let dt = frame_seconds(dt);
        for track in [&mut self.current, &mut self.next].into_iter().flatten() {
            track.wait(dt);
        }
        // In a crossfade the old track holds its gain until the new one
        // sounds.
        let waiting = self.next.as_ref().is_some_and(Track::coming_in_unheard);
        if let Some(current) = &mut self.current {
            let held = waiting && !current.ramp.rising;
            if !held && current.step(dt, out) {
                self.current = None;
            }
        }
        if let Some(next) = &mut self.next
            && next.step(dt, out)
        {
            self.next = None;
        }
        if self.current.is_none() {
            self.current = self.next.take();
            if let Some(track) = self.current.as_mut().filter(|t| !t.started) {
                track.start(out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toy::Voice;
    use shuyi_assets::audio::{CreditRef, MusicCue};

    type AudioRequest = super::AudioRequest<Voice>;
    type AudioQueue = super::AudioQueue<Voice>;

    fn play(cue: &str) -> AudioRequest {
        AudioRequest::PlayMusic {
            cue: cue.into(),
            switch: None,
        }
    }

    fn load(cue: &str) -> MusicCommand {
        MusicCommand::Load { cue: cue.into() }
    }

    fn start(cue: &str) -> MusicCommand {
        MusicCommand::Start {
            cue: cue.into(),
            gain: 1.0,
        }
    }

    fn stop(cue: &str) -> MusicCommand {
        MusicCommand::Stop { cue: cue.into() }
    }

    fn gain(cue: &str, gain: f32) -> MusicCommand {
        MusicCommand::Gain {
            cue: cue.into(),
            gain,
        }
    }

    /// Applies `requests` then advances `dt`, returning the commands.
    fn frame(m: &mut MusicState, requests: &[AudioRequest], dt: f32) -> Vec<MusicCommand> {
        let mut out = Vec::new();
        for r in requests {
            m.request(r, &mut out);
        }
        m.update(dt, &mut out);
        out
    }

    /// Music with a 0.5 s fade, playing `cue`.
    fn playing(cue: &str) -> MusicState {
        let mut m = MusicState::new(0.5);
        frame(&mut m, &[play(cue)], 0.0);
        m
    }

    #[test]
    fn the_first_track_starts_at_once() {
        let mut m = MusicState::new(0.5);
        assert_eq!(m.current(), None);
        assert_eq!(
            frame(&mut m, &[play("title")], 0.1),
            [load("title"), start("title")]
        );
        assert_eq!(m.current(), Some("title"));
        assert_eq!(m.target(), Some("title"));
        assert_eq!(frame(&mut m, &[], 0.1), []);
    }

    #[test]
    fn the_same_cue_does_not_restart() {
        let mut m = playing("title");
        assert_eq!(frame(&mut m, &[play("title"), play("title")], 0.1), []);
        assert_eq!(m.current(), Some("title"));
    }

    #[test]
    fn a_switch_fades_out_then_starts_the_new_track() {
        let mut m = playing("title");
        assert_eq!(
            frame(&mut m, &[play("battle")], 0.0),
            [load("battle"), gain("title", 1.0)]
        );
        assert_eq!(m.current(), Some("title"));
        assert_eq!(m.target(), Some("battle"));
        assert_eq!(frame(&mut m, &[], 0.25), [gain("title", 0.5)]);
        assert_eq!(frame(&mut m, &[], 0.125), [gain("title", 0.25)]);
        // Asking again for the incoming track changes nothing.
        assert_eq!(frame(&mut m, &[play("battle")], 0.0), [gain("title", 0.25)]);
        assert_eq!(frame(&mut m, &[], 0.125), [stop("title"), start("battle")]);
        assert_eq!(m.current(), Some("battle"));
        assert_eq!(frame(&mut m, &[], 0.1), []);
    }

    #[test]
    fn stop_music_fades_out_to_silence() {
        let mut m = playing("title");
        let stop_req = AudioRequest::StopMusic;
        assert_eq!(
            frame(&mut m, core::slice::from_ref(&stop_req), 0.25),
            [gain("title", 0.5)]
        );
        assert_eq!(m.target(), None);
        assert_eq!(
            frame(&mut m, core::slice::from_ref(&stop_req), 0.0),
            [gain("title", 0.5)]
        );
        assert_eq!(frame(&mut m, &[], 0.3), [stop("title")]);
        assert_eq!(m.current(), None);
        // Stopping silence does nothing.
        assert_eq!(frame(&mut m, &[stop_req], 1.0), []);
    }

    #[test]
    fn a_new_request_during_a_fade_wins() {
        let mut m = playing("title");
        frame(&mut m, &[play("battle")], 0.25);
        // The waiting track is dropped; the fade keeps its progress.
        assert_eq!(
            frame(&mut m, &[play("boss")], 0.0),
            [stop("battle"), load("boss"), gain("title", 0.5)]
        );
        assert_eq!(frame(&mut m, &[], 0.25), [stop("title"), start("boss")]);
        assert_eq!(m.current(), Some("boss"));
        // A stop during a switch drops the waiting track too.
        frame(&mut m, &[play("town")], 0.25);
        assert_eq!(
            frame(&mut m, &[AudioRequest::StopMusic], 0.0),
            [stop("town"), gain("boss", 0.5)]
        );
        assert_eq!(frame(&mut m, &[], 0.25), [stop("boss")]);
        assert_eq!(m.current(), None);
        // And a play during a fade to silence switches to it.
        let mut m = playing("title");
        frame(&mut m, &[AudioRequest::StopMusic], 0.25);
        assert_eq!(
            frame(&mut m, &[play("battle")], 0.25),
            [load("battle"), stop("title"), start("battle")]
        );
    }

    #[test]
    fn asking_for_the_fading_track_keeps_it() {
        let mut m = playing("title");
        frame(&mut m, &[play("battle")], 0.25);
        assert_eq!(
            frame(&mut m, &[play("title")], 1.0),
            [stop("battle"), gain("title", 1.0)]
        );
        assert_eq!(m.current(), Some("title"));
        assert_eq!(m.target(), Some("title"));
        let mut m = playing("title");
        frame(&mut m, &[AudioRequest::StopMusic], 0.25);
        assert_eq!(frame(&mut m, &[play("title")], 1.0), [gain("title", 1.0)]);
        assert_eq!(m.current(), Some("title"));
    }

    #[test]
    fn a_zero_fade_switches_in_the_same_frame() {
        for secs in [0.0, -1.0, f32::NAN] {
            let mut m = MusicState::new(secs);
            frame(&mut m, &[play("title")], 0.0);
            assert_eq!(
                frame(&mut m, &[play("battle")], 0.0),
                [load("battle"), stop("title"), start("battle")]
            );
        }
    }

    #[test]
    fn bad_frame_times_do_not_advance_a_fade() {
        let mut m = playing("title");
        frame(&mut m, &[play("battle")], 0.25);
        for dt in [-1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(frame(&mut m, &[], dt), [gain("title", 0.5)]);
        }
    }

    fn play_with(cue: &str, switch: MusicSwitch) -> AudioRequest {
        AudioRequest::play_music(cue, Some(switch))
    }

    fn start_at(cue: &str, gain: f32) -> MusicCommand {
        MusicCommand::start(cue, gain)
    }

    /// The clock of `cue`, as the platform reports a track that sounds.
    fn clock(cue: &str) -> MusicClock {
        MusicClock {
            cue: cue.into(),
            position: 0.0,
            length: 10.0,
        }
    }

    /// Music with a 0.5 s fade, playing `cue`, which sounds.
    fn sounding(cue: &str) -> MusicState {
        let mut m = playing(cue);
        m.heard(Some(&clock(cue)));
        m
    }

    /// `sounding("title")`, then a crossfade of 1 s to `battle`, which
    /// sounds and has run `secs`.
    fn crossfading(secs: f32) -> MusicState {
        let mut m = sounding("title");
        frame(
            &mut m,
            &[play_with("battle", MusicSwitch::crossfade(1000))],
            0.0,
        );
        m.heard(Some(&clock("battle")));
        frame(&mut m, &[], secs);
        m
    }

    #[test]
    fn a_crossfade_starts_the_new_track_at_once_and_fades_both_once_it_sounds() {
        let mut m = sounding("title");
        let crossfade = play_with("battle", MusicSwitch::crossfade(1000));
        assert_eq!(
            frame(&mut m, &[crossfade], 0.0),
            [load("battle"), start_at("battle", 0.0)]
        );
        assert_eq!((m.current(), m.target()), (Some("title"), Some("battle")));
        // Until the new track sounds, the old one keeps its gain.
        assert_eq!(frame(&mut m, &[], 0.25), []);
        m.heard(Some(&clock("title")));
        assert_eq!(frame(&mut m, &[], 0.25), []);
        m.heard(Some(&clock("battle")));
        assert_eq!(
            frame(&mut m, &[], 0.5),
            [gain("title", 0.5), gain("battle", 0.5)]
        );
        assert_eq!(
            frame(&mut m, &[], 0.5),
            [stop("title"), gain("battle", 1.0)]
        );
        assert_eq!((m.current(), m.target()), (Some("battle"), Some("battle")));
        assert_eq!(frame(&mut m, &[], 0.5), []);
    }

    #[test]
    fn a_fade_in_starts_at_silence_and_rises_once_the_track_sounds() {
        let mut m = MusicState::with_switch(MusicSwitch::new(500, 1000, false));
        assert_eq!(
            frame(&mut m, &[play("title")], 0.5),
            [load("title"), start_at("title", 0.0)]
        );
        // No clock, or another cue's, isn't this track sounding.
        m.heard(None);
        m.heard(Some(&clock("battle")));
        assert_eq!(frame(&mut m, &[], 0.5), []);
        m.heard(Some(&clock("title")));
        assert_eq!(frame(&mut m, &[], 0.5), [gain("title", 0.5)]);
        // It stays sounding without being named again.
        m.heard(None);
        assert_eq!(frame(&mut m, &[], 0.5), [gain("title", 1.0)]);
        assert_eq!(frame(&mut m, &[], 0.5), []);
    }

    #[test]
    fn without_overlap_the_new_track_fades_in_after_the_old_one_stops() {
        let mut m = sounding("title");
        let switch = play_with("battle", MusicSwitch::new(500, 1000, false));
        assert_eq!(
            frame(&mut m, core::slice::from_ref(&switch), 0.25),
            [load("battle"), gain("title", 0.5)]
        );
        // The clock can't name a track before its `Start`.
        m.heard(Some(&clock("battle")));
        assert_eq!(
            frame(&mut m, &[], 0.25),
            [stop("title"), start_at("battle", 0.0)]
        );
        assert_eq!(frame(&mut m, &[], 0.5), []);
        m.heard(Some(&clock("battle")));
        assert_eq!(frame(&mut m, &[], 0.25), [gain("battle", 0.25)]);
        // A track waiting to start doesn't count the load wait: after a
        // fade-out as long as the wait, it still waits to sound.
        let mut m = sounding("title");
        let switch = play_with("battle", MusicSwitch::new(3000, 1000, false));
        frame(&mut m, &[switch], 0.0);
        assert_eq!(frame(&mut m, &[], 1.5), [gain("title", 0.5)]);
        assert_eq!(
            frame(&mut m, &[], 1.5),
            [stop("title"), start_at("battle", 0.0)]
        );
        assert_eq!(frame(&mut m, &[], 1.0), []);
    }

    #[test]
    fn a_track_that_never_sounds_is_faded_in_after_the_load_wait() {
        let mut m = sounding("title");
        frame(
            &mut m,
            &[play_with("battle", MusicSwitch::crossfade(1000))],
            0.0,
        );
        assert_eq!(frame(&mut m, &[], 2.5), []);
        assert_eq!(MusicState::LOAD_WAIT_SECS, 3.0);
        assert_eq!(
            frame(&mut m, &[], 0.5),
            [gain("title", 0.5), gain("battle", 0.5)]
        );
        assert_eq!(
            frame(&mut m, &[], 0.5),
            [stop("title"), gain("battle", 1.0)]
        );
    }

    #[test]
    fn a_switch_during_a_crossfade_stops_the_older_track_and_fades_out_the_newer() {
        let mut m = crossfading(0.5);
        assert_eq!(
            frame(
                &mut m,
                &[play_with("boss", MusicSwitch::crossfade(1000))],
                0.0
            ),
            [stop("title"), load("boss"), start_at("boss", 0.0)]
        );
        assert_eq!((m.current(), m.target()), (Some("battle"), Some("boss")));
        m.heard(Some(&clock("boss")));
        // `battle` falls from the 0.5 it had reached.
        assert_eq!(
            frame(&mut m, &[], 0.25),
            [gain("battle", 0.25), gain("boss", 0.25)]
        );
        assert_eq!(
            frame(&mut m, &[], 0.25),
            [stop("battle"), gain("boss", 0.5)]
        );
        // A switch without overlap during a crossfade: the new track
        // waits for the newer one to fade out.
        let mut m = crossfading(0.5);
        assert_eq!(
            frame(&mut m, &[play("boss")], 0.125),
            [stop("title"), load("boss"), gain("battle", 0.25)]
        );
        assert_eq!(frame(&mut m, &[], 0.125), [stop("battle"), start("boss")]);
    }

    #[test]
    fn asking_again_for_the_track_fading_out_in_a_crossfade_brings_it_back() {
        let mut m = crossfading(0.25);
        let back = play_with("title", MusicSwitch::crossfade(1000));
        assert_eq!(
            frame(&mut m, &[back], 0.0),
            [gain("battle", 0.25), gain("title", 0.75)]
        );
        assert_eq!((m.current(), m.target()), (Some("battle"), Some("title")));
        assert_eq!(
            frame(&mut m, &[], 0.125),
            [gain("battle", 0.125), gain("title", 0.875)]
        );
        assert_eq!(
            frame(&mut m, &[], 0.125),
            [stop("battle"), gain("title", 1.0)]
        );
        assert_eq!((m.current(), m.target()), (Some("title"), Some("title")));
        // From 0.75 it took a quarter of the fade, and is done.
        assert_eq!(frame(&mut m, &[], 0.125), []);
        // With no fade-in it is back at full gain at once.
        let mut m = crossfading(0.25);
        let back = play_with("title", MusicSwitch::new(500, 0, true));
        assert_eq!(
            frame(&mut m, &[back], 0.0),
            [gain("title", 1.0), gain("battle", 0.25)]
        );
    }

    #[test]
    fn stop_music_during_a_crossfade_fades_both_tracks_from_their_gains() {
        let mut m = crossfading(0.25);
        assert_eq!(
            frame(&mut m, &[AudioRequest::StopMusic], 0.125),
            [gain("title", 0.5), stop("battle")]
        );
        assert_eq!((m.current(), m.target()), (Some("title"), None));
        assert_eq!(frame(&mut m, &[], 0.25), [stop("title")]);
        assert_eq!(m.current(), None);
    }

    #[test]
    fn the_default_switch_is_the_one_a_request_without_one_gets() {
        let mut m = MusicState::with_switch(MusicSwitch::crossfade(1000));
        frame(&mut m, &[play("title")], 0.0);
        m.heard(Some(&clock("title")));
        assert_eq!(frame(&mut m, &[], 1.0), [gain("title", 1.0)]);
        assert_eq!(
            frame(&mut m, &[play("battle")], 0.0),
            [load("battle"), start_at("battle", 0.0)]
        );
        // A request's own switch wins over it.
        let mut m = MusicState::with_switch(MusicSwitch::crossfade(1000));
        frame(&mut m, &[play("title")], 0.0);
        assert_eq!(
            frame(
                &mut m,
                &[play_with("battle", MusicSwitch::after_fade(0))],
                0.0
            ),
            [load("battle"), stop("title"), start("battle")]
        );
        assert_eq!(
            MusicState::new(0.5),
            MusicState::with_switch(MusicSwitch::after_fade(500))
        );
    }

    #[test]
    fn sound_requests_leave_the_music_alone() {
        let mut m = playing("title");
        let sound = AudioRequest::PlaySound {
            cue: "click".into(),
            volume: 1.0,
        };
        assert_eq!(frame(&mut m, &[sound], 0.1), []);
        assert_eq!(m.current(), Some("title"));
    }

    #[test]
    fn voice_requests_leave_the_music_alone() {
        let mut m = playing("title");
        let line = LineId::new("test_1a2b3c4d");
        let mut q = AudioQueue::default();
        q.play_voice(&line, Voice::High);
        q.stop_voice();
        q.preload_voices(vec![(line.clone(), Voice::Only)]);
        let expected = [
            AudioRequest::PlayVoice {
                line: line.clone(),
                variant: Voice::High,
            },
            AudioRequest::StopVoice,
            AudioRequest::PreloadVoices {
                lines: vec![(line, Voice::Only)],
            },
        ];
        assert_eq!(q.pending(), expected);
        assert!(expected.iter().all(|r| r.cue().is_none()));
        assert_eq!(frame(&mut m, &expected, 0.1), []);
        assert_eq!(m.current(), Some("title"));
    }

    #[test]
    fn the_queue_records_requests_in_order() {
        let mut q = AudioQueue::default();
        q.play_sound("a");
        q.play_sound_at("b", 0.6);
        q.play_sound_at("c", 2.0);
        q.play_sound_at("d", -1.0);
        q.play_sound_at("e", f32::NAN);
        q.play_music("m");
        q.stop_music();
        let sound = |cue: &str, volume| AudioRequest::PlaySound {
            cue: cue.into(),
            volume,
        };
        let expected = [
            sound("a", 1.0),
            sound("b", 0.6),
            sound("c", 1.0),
            sound("d", 0.0),
            sound("e", 0.0),
            play("m"),
            AudioRequest::StopMusic,
        ];
        assert_eq!(q.pending(), expected);
        assert_eq!(q.take(), expected);
        let found = q.pending();
        assert!(found.is_empty(), "{found:?}");
        assert_eq!(
            expected.map(|r| r.cue().map(str::to_owned))[4..],
            [Some("e".to_owned()), Some("m".to_owned()), None]
        );
    }

    /// The queue plays each menu sound and the cursor's tick as its
    /// config names them; by default after what they are.
    #[test]
    fn menu_sounds_and_the_cursor_play_the_configs_cues() {
        let cues = |q: &mut AudioQueue| {
            for sound in [
                MenuSound::Move,
                MenuSound::Select,
                MenuSound::Cancel,
                MenuSound::Denied,
            ] {
                q.menu(sound);
            }
            q.cursor_tick();
            let taken = q.take();
            taken
                .iter()
                .map(|r| r.cue().unwrap_or_default().to_owned())
                .collect::<Vec<_>>()
        };
        let mut q = AudioQueue::default();
        assert_eq!(q.config(), AudioConfig::default());
        assert_eq!(
            cues(&mut q),
            [
                "ui_move",
                "ui_select",
                "ui_cancel",
                "ui_cancel",
                "ui_cursor"
            ]
        );
        let config = AudioConfig {
            move_cue: "tick",
            select_cue: "ding",
            cancel_cue: "dong",
            denied_cue: "buzz",
            cursor_cue: "step",
        };
        let mut q = AudioQueue::new(config);
        assert_eq!(q.config(), config);
        assert_eq!(cues(&mut q), ["tick", "ding", "dong", "buzz", "step"]);
    }

    /// A manifest with a `fights` pool of three looped tracks and an
    /// empty `quiet` pool.
    fn manifest() -> AudioManifest {
        let mut m = AudioManifest::default();
        let tracks = ["drums", "horns", "pipes"];
        for cue in tracks {
            m.music.insert(cue.into(), track(cue, true, 10_000));
        }
        m.pools
            .insert("fights".into(), tracks.map(str::to_owned).to_vec());
        m.pools.insert("quiet".into(), Vec::new());
        m
    }

    /// A track of `cue`, `looped` or not, `length_ms` long.
    fn track(cue: &str, looped: bool, length_ms: u32) -> MusicCue {
        MusicCue {
            file: format!("{cue}.ogg"),
            volume: 100,
            looped,
            length_ms,
            credit: CreditRef::Own,
        }
    }

    #[test]
    fn a_pool_pick_is_a_cue_of_that_pool_and_fixed_by_the_seed() {
        let m = manifest();
        let pool = &m.pools["fights"];
        for seed in [0, 1, 2, u64::MAX] {
            let cue = pick_from_pool(&m, "fights", seed).unwrap();
            assert!(pool.iter().any(|c| c == cue), "{cue}");
            assert_eq!(pick_from_pool(&m, "fights", seed), Some(cue));
        }
        // Two seeds can pick two tracks.
        let picks: alloc::collections::BTreeSet<&str> = (0..8)
            .filter_map(|seed| pick_from_pool(&m, "fights", seed))
            .collect();
        assert!(picks.len() > 1, "{picks:?}");
    }

    /// The published splitmix64 outputs for state 0 (the first two steps).
    #[test]
    fn splitmix64_matches_the_reference() {
        assert_eq!(splitmix64(0), 0xE220_A839_7B1D_CDAF);
        assert_eq!(splitmix64(0x9E37_79B9_7F4A_7C15), 0x6E78_9E6A_A1B9_65F4);
    }

    #[test]
    fn an_unknown_or_empty_pool_picks_nothing() {
        let m = manifest();
        assert_eq!(pick_from_pool(&m, "no_such_pool", 1), None);
        assert_eq!(pick_from_pool(&m, "quiet", 1), None);
    }

    /// A manifest with one track, `looped` or not, 2.5 s long.
    fn one_track(looped: bool) -> AudioManifest {
        let mut m = AudioManifest::default();
        m.music.insert("theme".into(), track("theme", looped, 2500));
        m
    }

    /// The position of `theme`'s clock `elapsed` seconds after its start.
    fn position(looped: bool, elapsed: f64) -> Option<f32> {
        let clock = MusicClock::from_elapsed(&one_track(looped), "theme", elapsed)?;
        assert_eq!(clock.cue, "theme");
        assert!((clock.length - 2.5).abs() < f32::EPSILON, "{clock:?}");
        Some(clock.position)
    }

    fn close(position: Option<f32>, expected: f32) -> bool {
        position.is_some_and(|p| (p - expected).abs() < 1e-5)
    }

    #[test]
    fn the_clock_of_a_looped_track_wraps_at_its_length() {
        assert_eq!(position(true, 0.0), Some(0.0));
        assert!(close(position(true, 1.25), 1.25));
        assert!(close(position(true, 2.499), 2.499));
        assert_eq!(position(true, 2.5), Some(0.0));
        assert!(close(position(true, 3.0), 0.5));
        assert!(close(position(true, 2.5 * 1000.0 + 2.0), 2.0));
        // A hair before the loop point rounds onto the length as f32: that
        // is the loop point, never a position equal to the length.
        assert_eq!(position(true, 2.5 - 1e-12), Some(0.0));
    }

    #[test]
    fn the_clock_of_a_track_played_once_ends_with_it() {
        assert_eq!(position(false, 0.0), Some(0.0));
        assert!(close(position(false, 2.499), 2.499));
        assert_eq!(position(false, 2.5 - 1e-12), None);
        assert_eq!(position(false, 2.5), None);
        assert_eq!(position(false, 3.0), None);
    }

    #[test]
    fn the_clock_needs_a_known_cue_a_length_and_a_real_time() {
        let m = one_track(true);
        assert_eq!(MusicClock::from_elapsed(&m, "nope", 1.0), None);
        // A start in the future (the system clock stepped back) is 0.
        assert_eq!(position(true, -3.0), Some(0.0));
        assert_eq!(position(false, -3.0), Some(0.0));
        for looped in [true, false] {
            for elapsed in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert_eq!(position(looped, elapsed), None, "{looped} {elapsed}");
            }
            // The validator refuses a length of 0; a hand-made manifest
            // with one has no clock.
            let mut m = one_track(looped);
            if let Some(t) = m.music.get_mut("theme") {
                t.length_ms = 0;
            }
            assert_eq!(MusicClock::from_elapsed(&m, "theme", 0.0), None);
            assert_eq!(MusicClock::from_elapsed(&m, "theme", 1.0), None);
        }
    }

    proptest::proptest! {
        /// A looped track's position is always inside the track, however
        /// long it has played and whatever its length.
        #[test]
        fn a_looped_position_is_always_inside_the_track(
            length_ms in 1u32..=600_000,
            elapsed in 0.0f64..1.0e7,
        ) {
            let mut m = one_track(true);
            if let Some(t) = m.music.get_mut("theme") {
                t.length_ms = length_ms;
            }
            let clock = MusicClock::from_elapsed(&m, "theme", elapsed);
            let clock = clock.unwrap();
            proptest::prop_assert!(
                (0.0..clock.length).contains(&clock.position),
                "{clock:?}"
            );
        }
    }

    /// One step of [`at_most_two_tracks_load_and_every_gain_is_a_gain`].
    #[derive(Debug, Clone)]
    enum Step {
        Play(usize, Option<MusicSwitch>),
        Stop,
        Heard(Option<usize>),
        Frame(f32),
    }

    fn step() -> impl proptest::strategy::Strategy<Value = Step> {
        use proptest::prelude::*;
        let switch = (0u32..1500, 0u32..1500, any::<bool>())
            .prop_map(|(out_ms, in_ms, overlap)| MusicSwitch::new(out_ms, in_ms, overlap));
        prop_oneof![
            (0usize..4, proptest::option::of(switch)).prop_map(|(c, s)| Step::Play(c, s)),
            Just(Step::Stop),
            proptest::option::of(0usize..4).prop_map(Step::Heard),
            (0.0f32..1.0).prop_map(Step::Frame),
        ]
    }

    proptest::proptest! {
        /// Whatever is asked, heard and however time passes, at most two
        /// tracks are between their `Load` and their `Stop`; a track is
        /// loaded once, started, changed and stopped only while loaded;
        /// and every gain is 0 to 1.
        #[test]
        fn at_most_two_tracks_load_and_every_gain_is_a_gain(
            default in (0u32..1500, 0u32..1500, proptest::prelude::any::<bool>()),
            steps in proptest::collection::vec(step(), 0..80),
        ) {
            const CUES: [&str; 4] = ["a", "b", "c", "d"];
            let mut m = MusicState::with_switch(MusicSwitch::new(default.0, default.1, default.2));
            let mut loaded: alloc::collections::BTreeSet<String> = alloc::collections::BTreeSet::new();
            let mut out = Vec::new();
            for step in steps {
                match step {
                    Step::Play(c, switch) => m.request(&AudioRequest::play_music(CUES[c], switch), &mut out),
                    Step::Stop => m.request(&AudioRequest::StopMusic, &mut out),
                    Step::Heard(c) => m.heard(c.map(|c| clock(CUES[c])).as_ref()),
                    Step::Frame(dt) => m.update(dt, &mut out),
                }
                for command in out.drain(..) {
                    match &command {
                        MusicCommand::Load { cue } => {
                            proptest::prop_assert!(loaded.insert(cue.clone()), "{command:?}");
                        }
                        MusicCommand::Start { cue, gain } | MusicCommand::Gain { cue, gain } => {
                            proptest::prop_assert!(loaded.contains(cue), "{command:?}");
                            proptest::prop_assert!((0.0..=1.0).contains(gain), "{command:?}");
                        }
                        MusicCommand::Stop { cue } => {
                            proptest::prop_assert!(loaded.remove(cue), "{command:?}");
                        }
                    }
                    proptest::prop_assert!(loaded.len() <= 2, "{loaded:?}");
                }
                let current = m.current().map(str::to_owned);
                proptest::prop_assert!(current.is_none_or(|c| loaded.contains(&c)));
            }
        }
    }

    proptest::proptest! {
        /// Over enough seeds, every track in the pool comes up.
        #[test]
        fn every_track_in_a_pool_can_be_picked(start in proptest::prelude::any::<u64>()) {
            let m = manifest();
            let pool = &m.pools["fights"];
            let picked: alloc::collections::BTreeSet<&str> = (0..200)
                .filter_map(|i| pick_from_pool(&m, "fights", start.wrapping_add(i)))
                .collect();
            proptest::prop_assert_eq!(picked.len(), pool.len());
        }
    }
}
