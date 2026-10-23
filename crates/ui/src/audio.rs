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
//! ```
//! # use shuyi_ui::audio::{AudioQueue, AudioRequest};
//! let mut audio = AudioQueue::<()>::default();
//! audio.play_sound("menu_beep");
//! audio.play_music("theme");
//! assert_eq!(audio.pending()[1], AudioRequest::PlayMusic { cue: "theme".into() });
//! ```

use crate::prelude::*;
use shuyi_assets::audio::AudioManifest;
use shuyi_assets::dialogue::LineId;
#[cfg(doc)]
use shuyi_assets::voice::ClipVariant;

#[cfg(doc)]
use crate::GameData;
use crate::input::frame_seconds;

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

/// Something a screen wants heard. Cue ids are the ones in the game's
/// audio manifest ([`AudioManifest`]).
#[derive(Debug, Clone, PartialEq)]
pub enum AudioRequest<V> {
    /// Play a sound cue once, at `volume` (0–1) times the cue's own volume.
    PlaySound {
        /// The sound cue.
        cue: String,
        /// A multiplier on the cue's volume, 0–1.
        volume: f32,
    },
    /// Switch to a music cue. Asking for the track already playing does
    /// nothing.
    PlayMusic {
        /// The music cue.
        cue: String,
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
    /// The cue this request names, if any.
    pub fn cue(&self) -> Option<&str> {
        match self {
            Self::PlaySound { cue, .. } | Self::PlayMusic { cue } => Some(cue),
            Self::StopMusic
            | Self::PlayVoice { .. }
            | Self::StopVoice
            | Self::PreloadVoices { .. } => None,
        }
    }
}

/// The requests screens made this frame. `Game` empties it every frame.
/// It knows which cues the [`MenuSound`]s are ([`AudioConfig`]).
#[derive(Debug, Clone, PartialEq)]
pub struct AudioQueue<V> {
    requests: Vec<AudioRequest<V>>,
    config: AudioConfig,
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
        let volume = if volume.is_nan() {
            0.0
        } else {
            volume.clamp(0.0, 1.0)
        };
        self.requests.push(AudioRequest::PlaySound {
            cue: cue.to_owned(),
            volume,
        });
    }

    /// Switches the music to `cue` (nothing happens if it's already on).
    pub fn play_music(&mut self, cue: &str) {
        self.requests.push(AudioRequest::PlayMusic {
            cue: cue.to_owned(),
        });
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
#[derive(Debug, Clone, PartialEq)]
pub enum MusicCommand {
    /// Start loading `cue`'s file; it will be started soon.
    Load {
        /// The music cue.
        cue: String,
    },
    /// Play `cue` (loaded earlier) from the start at full gain, looped if
    /// the manifest says so. If it isn't loaded yet, play it once it is.
    Start {
        /// The music cue.
        cue: String,
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

/// Which track plays, and the fade between tracks: the old one fades out,
/// then the new one starts; a cue already playing doesn't restart. Pure:
/// feed it requests and time, read the commands.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicState {
    /// Fade-out length in seconds.
    fade_secs: f32,
    /// The track playing (or fading out).
    current: Option<String>,
    /// The fade in progress, if any.
    fade: Option<Fade>,
}

/// A fade-out of the current track.
#[derive(Debug, Clone, PartialEq)]
struct Fade {
    /// Seconds since it began.
    elapsed: f32,
    /// What starts when it ends (already loading); `None` for silence.
    next: Option<String>,
}

impl MusicState {
    /// Silence, with fades `fade_secs` long (negative or NaN counts as 0).
    pub fn new(fade_secs: f32) -> Self {
        Self {
            fade_secs: fade_secs.max(0.0),
            current: None,
            fade: None,
        }
    }

    /// The track playing or fading out.
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// The track that plays once the fade ends, or the current one.
    pub fn target(&self) -> Option<&str> {
        match &self.fade {
            Some(fade) => fade.next.as_deref(),
            None => self.current(),
        }
    }

    /// Applies one request (sound and voice requests are ignored),
    /// appending the commands it needs to `out`.
    pub fn request<V>(&mut self, request: &AudioRequest<V>, out: &mut Vec<MusicCommand>) {
        match request {
            AudioRequest::PlaySound { .. }
            | AudioRequest::PlayVoice { .. }
            | AudioRequest::StopVoice
            | AudioRequest::PreloadVoices { .. } => {}
            AudioRequest::PlayMusic { cue } => self.play(cue, out),
            AudioRequest::StopMusic => self.stop(out),
        }
    }

    fn play(&mut self, cue: &str, out: &mut Vec<MusicCommand>) {
        let Some(current) = &self.current else {
            out.push(MusicCommand::Load { cue: cue.into() });
            out.push(MusicCommand::Start { cue: cue.into() });
            self.current = Some(cue.into());
            return;
        };
        if self.target() == Some(cue) {
            return;
        }
        if let Some(Fade {
            next: Some(next), ..
        }) = &self.fade
        {
            // A newer request wins: drop the one waiting to start.
            out.push(MusicCommand::Stop { cue: next.clone() });
        }
        if current == cue {
            // Asked again for the track that is fading out: keep it.
            out.push(MusicCommand::Gain {
                cue: cue.into(),
                gain: 1.0,
            });
            self.fade = None;
            return;
        }
        out.push(MusicCommand::Load { cue: cue.into() });
        let elapsed = self.fade.as_ref().map_or(0.0, |f| f.elapsed);
        self.fade = Some(Fade {
            elapsed,
            next: Some(cue.into()),
        });
    }

    fn stop(&mut self, out: &mut Vec<MusicCommand>) {
        if self.current.is_none() {
            return;
        }
        let elapsed = match self.fade.take() {
            Some(Fade { elapsed, next }) => {
                if let Some(next) = next {
                    out.push(MusicCommand::Stop { cue: next });
                }
                elapsed
            }
            None => 0.0,
        };
        self.fade = Some(Fade {
            elapsed,
            next: None,
        });
    }

    /// Advances a fade by `dt` seconds, appending the gain change, or the
    /// stop and next start when it ends.
    pub fn update(&mut self, dt: f32, out: &mut Vec<MusicCommand>) {
        let (Some(current), Some(fade)) = (&self.current, &mut self.fade) else {
            return;
        };
        fade.elapsed += frame_seconds(dt);
        if fade.elapsed < self.fade_secs {
            out.push(MusicCommand::Gain {
                cue: current.clone(),
                gain: 1.0 - fade.elapsed / self.fade_secs,
            });
            return;
        }
        out.push(MusicCommand::Stop {
            cue: current.clone(),
        });
        let next = fade.next.take();
        if let Some(next) = &next {
            out.push(MusicCommand::Start { cue: next.clone() });
        }
        self.current = next;
        self.fade = None;
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
        AudioRequest::PlayMusic { cue: cue.into() }
    }

    fn load(cue: &str) -> MusicCommand {
        MusicCommand::Load { cue: cue.into() }
    }

    fn start(cue: &str) -> MusicCommand {
        MusicCommand::Start { cue: cue.into() }
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
