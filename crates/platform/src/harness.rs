//! A headless test driver for any game on the engine (game ADR-0007 layer 4):
//! scripted key and controller-button presses and waits in, in frames of
//! 1/60 s or by a tick stage's fixed steps ([`Harness::steps`]), screen
//! names, snapshots and the audio asked for out. No window,
//! no clock, no sound card: it runs the game in an [`App`] on the
//! [`HeadlessPlatform`], through the same frame the devices run, and the
//! headless backend's [`Speaker`](crate::headless::Speaker) says what music
//! sounds, so a screen that keeps time with the music is tested too.
//!
//! A game wraps it with its own start-up and lookups (its flow, its
//! battle); the scripts are the same for every game. With the `harness`
//! feature.
//!
//! ```
//! use shuyi_platform::harness::Harness;
//! use shuyi_ui::toy;
//!
//! let mut h = Harness::new(toy::game());
//! h.keys("f");
//! assert_eq!(h.screens(), ["home", "menu"]);
//! h.keys("f");
//! assert_eq!(h.sounds(), ["click"]);
//! ```
//!
//! [`Harness::new`] takes a game already running. [`Harness::start`] runs it
//! from its real [`Start`], card and load first, as a device does;
//! [`Harness::start_recording`] records that run ([`session`]),
//! and [`Harness::from_session`] plays such a record to any frame
//! ([`play_to`](Harness::play_to)).

use crate::alloc_prelude::*;
use alloc::{format, vec};
use core::any::Any;
use core::time::Duration;

#[cfg(feature = "std")]
use shuyi_assets::bundle::Layer;
use shuyi_assets::keymap::{Button, Chord, CrankDef};
use shuyi_assets::target::TargetProfile;
use shuyi_ui::audio::{AudioRequest, MusicClock, MusicCommand};
use shuyi_ui::input::crank::Crank;
use shuyi_ui::input::frame_seconds;
use shuyi_ui::input::pad::PadKind;
use shuyi_ui::input::state::Device;
use shuyi_ui::log::LogRing;
use shuyi_ui::perf::{HeapCounts, HeapSnapshot, MemoryScopes, PerfClock, PerfRing};
use shuyi_ui::storage::Storage;
use shuyi_ui::{Ctx, Game, GameData, KeyPrompt, PlatformFacts, RawInputEvent};

use crate::app::{App, Start, SystemHook};
use crate::headless::{HeadlessPlatform, PlayingSound, Presented};
use crate::platform::{Millis, SystemEvent};
use crate::script::{self, Planned};
use crate::session::{self, Playback, Played, SessionError, SessionRecord, Stamps};

#[cfg(feature = "std")]
mod kept;
#[cfg(feature = "std")]
pub use kept::assert_session;

/// Simulated length of one frame, in seconds (60 fps): a script's frame.
pub use crate::script::FRAME_DT;

/// Most frames one [`Harness::hold`] or [`Harness::wait`] may run (about
/// half a simulated minute), so a runaway script fails instead of hanging.
pub const MAX_FRAMES: u32 = 2_000;

/// The longest wait of one step of a [`Script`](crate::script::Script) the
/// Harness plays, in seconds: [`MAX_FRAMES`] frames.
#[allow(clippy::cast_precision_loss)] // 2000 frames
pub const MAX_WAIT: f32 = MAX_FRAMES as f32 * FRAME_DT;

/// The clock of a Harness on a game's real [`Start`]: the whole milliseconds
/// [`App::frame`] takes, from the microseconds its frames add up to, so
/// frames of [`FRAME_DT`] come out as 16, 17 and 17 ms. All integers: the
/// `no_std` build has no `f64::round`.
#[derive(Debug, Clone, Copy)]
struct FrameClock {
    /// Where it started, whatever a record's clock had reached.
    from: Millis,
    /// The microseconds since.
    micros: u64,
}

impl FrameClock {
    fn at(from: Millis) -> Self {
        Self { from, micros: 0 }
    }

    /// The time `seconds` later, which goes on from here.
    fn after(&mut self, seconds: f32) -> Millis {
        let elapsed = Duration::try_from_secs_f32(frame_seconds(seconds)).unwrap_or(Duration::ZERO);
        let micros = elapsed.as_nanos().saturating_add(500) / 1000;
        let micros = u64::try_from(micros).unwrap_or(u64::MAX);
        self.micros = self.micros.saturating_add(micros);
        Millis(self.from.0.wrapping_add(self.micros / 1000))
    }
}

/// Drives a [`Game`] with simulated key presses and time, as the platform
/// would: in an [`App`], on the [`HeadlessPlatform`].
pub struct Harness<G: GameData> {
    app: App<G>,
    platform: HeadlessPlatform,
    /// The clock of a game from its [`Start`], which runs on [`App::frame`];
    /// `None` for a game already running, on [`App::frame_after`].
    clock: Option<FrameClock>,
    /// The record playing, until a script's frame takes over from it.
    playback: Option<Playback>,
    /// How far it has played.
    played: Played,
    /// Every frame's audio requests, oldest first.
    audio: Vec<Vec<AudioRequest<G::Voice>>>,
    /// Every music command of the run, in order.
    music: Vec<MusicCommand>,
    /// The kind of controller [`pad`](Self::pad) presses buttons on.
    pad_kind: PadKind,
    /// The crank [`crank`](Self::crank) winds, on the [`use_pad`] pad.
    ///
    /// [`use_pad`]: Self::use_pad
    crank: Crank,
    /// How many frames have run.
    frames: u32,
    /// The screens open before the first frame, then after each frame
    /// that changed them: the frame, and the names, bottom first.
    stacks: Vec<(u32, Vec<&'static str>)>,
    /// The frame of each of `music`'s commands.
    music_frames: Vec<u32>,
    /// The frame the game asked to quit in, once it has.
    quit_frame: Option<u32>,
}

impl<G: GameData> Harness<G> {
    /// Drives `game` from where it is.
    pub fn new(game: Game<G>) -> Self {
        Self::on(game, HeadlessPlatform::new())
    }

    /// Drives `game` from where it is on `platform` (one with a system
    /// keyboard, say). Its screens get only the actions they declare
    /// ([`Game::guard_actions`]), so a screen reacting to one it didn't
    /// declare fails its test ([`undeclared`](Self::undeclared) says which
    /// were held back).
    pub fn on(mut game: Game<G>, platform: HeadlessPlatform) -> Self {
        game.guard_actions(true);
        Self::driving(App::running(game), platform, None)
    }

    /// Runs the game from its real `start`, as a device runs it: the card's
    /// frame at 0 ms and the load's one [`FRAME_DT`] later run now, and the
    /// load's errors are the `Err`. Its frames go through [`App::frame`], in
    /// whole milliseconds. As on a device, its screens get the actions they
    /// didn't declare too ([`on`](Self::on) holds those back), and a game
    /// waits for its first press unless its start says not
    /// ([`Setup::key_prompt`](crate::Setup::key_prompt)). The scripts work as
    /// on any Harness.
    pub fn start(start: Start<G>) -> Result<Self, String> {
        Self::started(App::new(start))
    }

    /// [`start`](Self::start), recording from the first frame:
    /// [`session`](Self::session) gives the record so far.
    ///
    /// A record holds what the platform gives the game, so what a test
    /// changes around the platform isn't in it, and plays differently or
    /// not at all: [`act`](Self::act), [`ctx_mut`](Self::ctx_mut),
    /// [`screen_mut`](Self::screen_mut). Press the keys instead. Nor does it
    /// hold a [`with_system_hook`](Self::with_system_hook) hook, so give the
    /// hook in the `Start`, to both runs ([`Start::on_system`]).
    pub fn start_recording(start: Start<G>) -> Result<Self, String> {
        let mut app = App::new(start);
        // No frame has run, so it always turns on.
        let _ = app.record();
        Self::started(app)
    }

    /// The record so far, of a [`start_recording`](Self::start_recording).
    pub fn session(&self) -> Option<&SessionRecord> {
        self.app.session()
    }

    /// Makes ready to play `record` from the game's real `start`, on the
    /// display, the facts and the store it recorded
    /// ([`HeadlessPlatform::from_session`]). It plays nothing yet:
    /// [`play_to`](Self::play_to) does. A record of another version is
    /// refused, and of another build as `stamps` says
    /// ([`session::prepare`]).
    pub fn from_session(
        start: Start<G>,
        record: &SessionRecord,
        stamps: Stamps,
    ) -> Result<Self, SessionError> {
        let (start, limits) = session::prepare(start, record, stamps)?;
        let platform = HeadlessPlatform::from_session(record);
        let mut harness = Self::driving(App::new(start), platform, None);
        harness.playback = Some(Playback::new(record));
        harness.played.of = u32::try_from(record.frames.len()).unwrap_or(u32::MAX);
        harness.played.limits = limits;
        Ok(harness)
    }

    /// Plays the record's frames up to `frame`, counted from 1, or to its
    /// end, and stops at the first frame whose hash differs, leaving the game
    /// on that frame; after that it plays nothing more. On a frame before the
    /// load, [`app`](Self::app), [`screens`](Self::screens) and
    /// [`screen_log`](Self::screen_log) can be read, while what needs the
    /// game panics. A Harness with no record plays nothing.
    ///
    /// A script's frame ends the playing, wherever it has got to: the game
    /// goes on with the script, its clock from the last frame played
    /// ([`Playback::now`]) and its music from the headless
    /// [`Speaker`](crate::headless::Speaker), and [`Played::limits`] says so.
    pub fn play_to(&mut self, frame: u32) -> &Played {
        while self.played.first_difference.is_none() && self.played.frames < frame {
            let Some(playback) = &mut self.playback else {
                break;
            };
            let Some(answer) = playback.next(&mut self.app, &mut self.platform) else {
                break;
            };
            self.played.frames += 1;
            self.played.first_difference = answer.err();
            self.note_frame();
        }
        &self.played
    }

    /// [`play_to`](Self::play_to) the record's end.
    pub fn play_all(&mut self) -> &Played {
        self.play_to(u32::MAX)
    }

    /// A script's frame is about to run: a record playing stops here.
    fn leave_record(&mut self) {
        let Some(playback) = self.playback.take() else {
            return;
        };
        self.clock = Some(FrameClock::at(playback.now()));
        self.platform.end_session();
        let Played { frames, of, .. } = self.played;
        self.played.limits.push(format!(
            "after frame {frames} the music is the headless speaker's and no system keyboard is there: a script took over from the record"
        ));
        if frames < of {
            self.played.limits.push(format!(
                "the script took over before the record's end: {} of its frames didn't play",
                of - frames
            ));
        }
    }

    /// Runs the first two frames of a Harness on a game's real [`Start`].
    fn started(app: App<G>) -> Result<Self, String> {
        let clock = FrameClock::at(Millis(0));
        let mut harness = Self::driving(app, HeadlessPlatform::new(), Some(clock));
        let _ = harness.app.frame(&mut harness.platform, clock.from);
        harness.note_frame();
        harness.frame(&[], FRAME_DT);
        match harness.app.errors() {
            Some(errors) => Err(errors.to_owned()),
            None => Ok(harness),
        }
    }

    /// A Harness over `app`, which has run no frame.
    fn driving(app: App<G>, platform: HeadlessPlatform, clock: Option<FrameClock>) -> Self {
        let stacks = vec![(0, app.game().map_or_else(Vec::new, Game::screens))];
        Self {
            app,
            platform,
            clock,
            playback: None,
            played: Played::default(),
            audio: Vec::new(),
            music: Vec::new(),
            pad_kind: PadKind::default(),
            crank: Crank::new(CrankDef::default()),
            frames: 0,
            stacks,
            music_frames: Vec::new(),
            quit_frame: None,
        }
    }

    /// Gives the game `hook` for the system's events
    /// ([`system`](Self::system)).
    #[must_use]
    pub fn with_system_hook(mut self, hook: SystemHook<G>) -> Self {
        self.app.set_system_hook(hook);
        self
    }

    /// Waits for a key or button press, as a platform's first screen may
    /// ([`KeyPrompt::Waiting`]), and redraws.
    #[must_use]
    pub fn at_prompt(mut self) -> Self {
        let game = self.game_mut();
        game.ctx_mut().key_prompt = KeyPrompt::Waiting;
        game.redraw();
        self
    }

    /// Runs the game as the target `profile` describes: its facts
    /// ([`PlatformFacts::from_profile`]) go into the context
    /// ([`Ctx::platform`]), and the screen redraws. Call it before the
    /// screen under test opens: a screen may keep what it read as it
    /// opened (the key bindings screen keeps the keyboard).
    #[must_use]
    pub fn as_target(mut self, profile: &TargetProfile) -> Self {
        let game = self.game_mut();
        game.ctx_mut().platform = PlatformFacts::from_profile(profile);
        game.redraw();
        self
    }

    /// Makes music take `seconds` to load: a track starts sounding, and
    /// [`Ctx::music_clock`] starts counting, that long after the game
    /// starts it (at once by default). Negative or NaN counts as 0.
    pub fn music_load_delay(&mut self, seconds: f32) -> &mut Self {
        self.platform
            .speaker_mut()
            .set_load_delay(f64::from(seconds));
        self
    }

    /// Makes no music ever sound, as when the music files are missing:
    /// [`Ctx::music_clock`] stays `None`.
    pub fn without_music(&mut self) -> &mut Self {
        self.platform.speaker_mut().set_silent();
        self
    }

    /// Where the music is in its track, as the screens saw it in the last
    /// frame ([`Ctx::music_clock`]).
    pub fn music_clock(&self) -> Option<&MusicClock> {
        self.ctx().music_clock.as_ref()
    }

    /// From now on [`pad`](Self::pad) and [`hold_pad`](Self::hold_pad)
    /// press buttons on a controller of `kind` (a generic one until then).
    /// The button names in scripts stay binding positions: on a Nintendo
    /// pad `South` is its right face button.
    pub fn use_pad(&mut self, kind: PadKind) -> &mut Self {
        self.pad_kind = kind;
        self
    }

    /// Changes the file at `path` in `layer` (one handed in at run time),
    /// as a file changing on disk does: the next frame's modules hear of it
    /// before the update ([`HeadlessPlatform::change_file`]).
    #[cfg(feature = "std")]
    pub fn change_file(&mut self, layer: Layer, path: &str, bytes: Vec<u8>) -> &mut Self {
        self.platform.change_file(layer, path, bytes);
        self
    }

    /// Presses and releases each whitespace-separated chord in turn, e.g.
    /// `"Down Down f"` or `"Shift+Space Enter"`: a frame with the press,
    /// then a frame with the release, each [`FRAME_DT`] long.
    ///
    /// # Panics
    ///
    /// On a chord [`Chord::parse`] rejects.
    pub fn keys(&mut self, script: &str) -> &mut Self {
        let frames =
            script::key_frames(script).unwrap_or_else(|e| panic!("bad chord in test script: {e}"));
        self.play_planned(&frames);
        self
    }

    /// Types `text` one character per frame as the platform reports
    /// typing: the key's press (Shift for capitals) with the character,
    /// then its release. A character without a key of its own is typed
    /// alone.
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        for c in text.chars() {
            let name = match c {
                ' ' => "Space".to_owned(),
                c if c.is_ascii_uppercase() => format!("Shift+{}", c.to_ascii_lowercase()),
                c => c.to_string(),
            };
            let Ok(chord) = Chord::parse(&name) else {
                self.frame(&[RawInputEvent::Text(c)], FRAME_DT);
                continue;
            };
            self.frame(
                &[RawInputEvent::Down(chord), RawInputEvent::Text(c)],
                FRAME_DT,
            );
            self.frame(&[RawInputEvent::Up(chord.key)], FRAME_DT);
        }
        self
    }

    /// Holds `chord` for `seconds` of frames (so held keys repeat), then
    /// releases it in one more frame.
    ///
    /// # Panics
    ///
    /// On a chord [`Chord::parse`] rejects.
    pub fn hold(&mut self, chord: &str, seconds: f32) -> &mut Self {
        let chord = parse(chord);
        self.advance(&[RawInputEvent::Down(chord)], seconds);
        self.frame(&[RawInputEvent::Up(chord.key)], FRAME_DT);
        self
    }

    /// Presses and releases each whitespace-separated controller button in
    /// turn, e.g. `"DpadDown South"`, like [`keys`](Self::keys). The names
    /// are button positions, as in a keymap file. Buttons joined by `+`
    /// (`"South+East"`, `"East+DpadRight"`) go down together in one frame,
    /// in the order written, and up together in the next.
    ///
    /// # Panics
    ///
    /// On a button [`Button::parse`] rejects.
    pub fn pad(&mut self, script: &str) -> &mut Self {
        let frames = script::pad_frames(script, self.pad_kind)
            .unwrap_or_else(|e| panic!("bad button in test script: {e}"));
        self.play_planned(&frames);
        self
    }

    /// The kind of controller [`pad`](Self::pad) presses buttons on.
    pub(crate) fn pad_kind(&self) -> PadKind {
        self.pad_kind
    }

    /// Plays `frames` of a script's plan, each [`FRAME_DT`] long.
    ///
    /// # Panics
    ///
    /// On an idle run of more than [`MAX_FRAMES`] frames.
    pub(crate) fn play_planned(&mut self, frames: &[Planned]) {
        for planned in frames {
            match planned {
                Planned::Events(events) => {
                    self.frame(events, FRAME_DT);
                }
                Planned::Idle(n) => {
                    assert!(
                        *n <= MAX_FRAMES,
                        "Harness: more than {MAX_FRAMES} frames in one wait"
                    );
                    for _ in 0..*n {
                        self.frame(&[], FRAME_DT);
                    }
                }
            }
        }
    }

    /// Notches of `def` for [`crank`](Self::crank) (the game's keymap
    /// file's `crank`; 45° until set).
    pub fn set_crank(&mut self, def: CrankDef) -> &mut Self {
        self.crank = Crank::new(def);
        self
    }

    /// Winds the crank `degrees` in one frame of [`FRAME_DT`] (forwards is
    /// positive, backwards negative), as the Playdate's front end reports
    /// it: a press and a release of a crank button for each whole notch,
    /// the rest of a notch kept for the next call (ticket 1035).
    pub fn crank(&mut self, degrees: f32) -> &mut Self {
        let events = RawInputEvent::from_crank(&mut self.crank, degrees, false, self.pad_kind);
        self.frame(&events, FRAME_DT)
    }

    /// Hands the top screen `action` from outside the keys, as a platform's
    /// own menu does ([`Game::act`]), then runs a frame of [`FRAME_DT`]:
    /// the menu closes and the game goes on.
    pub fn act(&mut self, action: G::Action) -> &mut Self {
        self.game_mut().act(action);
        self.frame(&[], FRAME_DT)
    }

    /// The system tells the game `event` (a suspend, a resume, a lost
    /// focus), in a frame of [`FRAME_DT`] with no input.
    pub fn system(&mut self, event: SystemEvent) -> &mut Self {
        self.platform.push_system(event);
        self.frame(&[], FRAME_DT)
    }

    /// Holds the controller button `button` for `seconds` of frames (so a
    /// held direction repeats), then releases it in one more frame.
    ///
    /// # Panics
    ///
    /// On a button [`Button::parse`] rejects.
    pub fn hold_pad(&mut self, button: &str, seconds: f32) -> &mut Self {
        let button = parse_button(button);
        self.advance(&[RawInputEvent::PadDown(button, self.pad_kind)], seconds);
        self.frame(&[RawInputEvent::PadUp(button)], FRAME_DT);
        self
    }

    /// Lets `seconds` pass with no input, in frames of at most [`FRAME_DT`].
    pub fn wait(&mut self, seconds: f32) -> &mut Self {
        self.advance(&[], seconds);
        self
    }

    /// Runs frames with no input until exactly `count` fixed simulation
    /// steps were due in them ([`App::ticks`]), so a tick stage steps its
    /// rules `count` times: frames of [`FRAME_DT`], which run at most two
    /// steps, and a half one for the last step, which runs at most one.
    ///
    /// # Panics
    ///
    /// If that takes more than [`MAX_FRAMES`] frames: the game runs no
    /// steps (it is suspended, or hasn't loaded).
    pub fn steps(&mut self, count: u32) -> &mut Self {
        let mut left = count;
        for _ in 0..MAX_FRAMES {
            if left == 0 {
                return self;
            }
            let dt = if left >= 2 { FRAME_DT } else { FRAME_DT / 2.0 };
            self.frame(&[], dt);
            left = left.saturating_sub(self.app.ticks());
        }
        if left == 0 {
            return self;
        }
        panic!("Harness: {left} of {count} steps still due after {MAX_FRAMES} frames");
    }

    /// Runs frames totalling `seconds` (at least one frame); `first` goes
    /// into the first frame.
    ///
    /// # Panics
    ///
    /// If that takes more than [`MAX_FRAMES`] frames.
    fn advance(&mut self, first: &[RawInputEvent], seconds: f32) {
        let mut left = if seconds.is_finite() {
            seconds.max(0.0)
        } else {
            0.0
        };
        let mut events = first;
        for _ in 0..MAX_FRAMES {
            let dt = left.min(FRAME_DT);
            self.frame(events, dt);
            events = &[];
            left -= dt;
            if left <= 0.0 {
                return;
            }
        }
        panic!("Harness: more than {MAX_FRAMES} frames in one hold or wait");
    }

    /// Runs one frame of `dt` seconds with `events`, as a platform does:
    /// the events go to the headless backend, which the [`App`] reads in
    /// its frame (the music sounding, the update, the audio played, the
    /// frame presented); then the audio is recorded. The scripts above
    /// are made of these.
    pub fn frame(&mut self, events: &[RawInputEvent], dt: f32) -> &mut Self {
        self.leave_record();
        self.platform.push_input(events);
        let _ = match &mut self.clock {
            Some(clock) => self.app.frame(&mut self.platform, clock.after(dt)),
            None => self.app.frame_after(&mut self.platform, dt),
        };
        self.note_frame();
        self
    }

    /// Notes the frame just run: its audio, whether it quit, and the screens
    /// it left, none while the game hasn't loaded.
    fn note_frame(&mut self) {
        self.frames += 1;
        self.audio.push(self.app.audio().to_vec());
        let music = self.app.music();
        self.music.extend_from_slice(music);
        self.music_frames
            .extend(core::iter::repeat_n(self.frames, music.len()));
        if self.app.has_quit() && self.quit_frame.is_none() {
            self.quit_frame = Some(self.frames);
        }
        let screens = self.screens();
        if self.stacks.last().is_none_or(|(_, last)| *last != screens) {
            self.stacks.push((self.frames, screens));
        }
    }

    /// How many frames have run since the Harness started.
    pub fn frame_count(&self) -> u32 {
        self.frames
    }

    /// The screens open before the first frame (frame 0), then after each
    /// frame that changed them: the frame, and their names, bottom first.
    pub fn screen_log(&self) -> &[(u32, Vec<&'static str>)] {
        &self.stacks
    }

    /// The frame the game asked to quit in, if it has (frames count from
    /// 1).
    pub fn quit_frame(&self) -> Option<u32> {
        self.quit_frame
    }

    /// Every audio request since the last [`clear_audio`](Self::clear_audio),
    /// in order, each with the frame that asked for it.
    pub fn audio_log(&self) -> Vec<(u32, &AudioRequest<G::Voice>)> {
        let len = u32::try_from(self.audio.len()).unwrap_or(u32::MAX);
        let first = self.frames.saturating_sub(len) + 1;
        (first..)
            .zip(&self.audio)
            .flat_map(|(frame, requests)| requests.iter().map(move |r| (frame, r)))
            .collect()
    }

    /// Every music command since the last
    /// [`clear_audio`](Self::clear_audio), in order, each with the frame
    /// that gave it.
    pub fn music_log(&self) -> Vec<(u32, &MusicCommand)> {
        self.music_frames.iter().copied().zip(&self.music).collect()
    }

    /// Every audio request of the run so far, in order.
    pub fn audio_requests(&self) -> Vec<AudioRequest<G::Voice>> {
        self.audio.concat()
    }

    /// The sound cues (not music) played or started so far, in order.
    pub fn sounds(&self) -> Vec<String> {
        self.audio
            .iter()
            .flatten()
            .filter_map(|r| match r {
                AudioRequest::PlaySound { cue, .. } | AudioRequest::StartSound { cue, .. } => {
                    Some(cue.clone())
                }
                _ => None,
            })
            .collect()
    }

    /// The sounds started with a handle that are still playing, as the
    /// headless backend's [`Speaker`](crate::headless::Speaker) holds them,
    /// in id order.
    pub fn sounds_playing(&self) -> &[PlayingSound] {
        self.platform.speaker().sounds_playing()
    }

    /// The lines whose voice clip was asked for so far, in order, as line
    /// ids.
    pub fn voices(&self) -> Vec<String> {
        self.audio
            .iter()
            .flatten()
            .filter_map(|r| match r {
                AudioRequest::PlayVoice { line, .. } => Some(line.to_string()),
                _ => None,
            })
            .collect()
    }

    /// The audio requests of the last frame run (a key press runs two
    /// frames, press and release; this is the release).
    pub fn last_frame_audio(&self) -> &[AudioRequest<G::Voice>] {
        self.audio.last().map_or(&[], Vec::as_slice)
    }

    /// The volumes the last frame told the platform to play at, 0–1: the
    /// music's and the sounds' ([`AudioFrame`](crate::AudioFrame)).
    pub fn volumes(&self) -> (f32, f32) {
        let (music, sound, _) = self.platform.speaker().volumes();
        (music, sound)
    }

    /// The voice volume the last frame told the platform to play at, 0–1
    /// ([`AudioFrame`](crate::AudioFrame)).
    pub fn voice_volume(&self) -> f32 {
        self.platform.speaker().volumes().2
    }

    /// Whether the last frame told the platform to fill the screen.
    pub fn fullscreen(&self) -> bool {
        matches!(
            self.platform.last_frame(),
            Some(Presented::Game {
                fullscreen: true,
                ..
            })
        )
    }

    /// Every music command of the run so far, in order.
    pub fn music_commands(&self) -> &[MusicCommand] {
        &self.music
    }

    /// Forgets the audio recorded so far, so the next checks see only
    /// what comes after.
    pub fn clear_audio(&mut self) -> &mut Self {
        self.audio.clear();
        self.music.clear();
        self.music_frames.clear();
        self
    }

    /// Name of the top screen; empty once every screen has closed, and while
    /// the game hasn't loaded.
    pub fn top_screen(&self) -> &'static str {
        self.app.game().and_then(Game::top_screen).unwrap_or("")
    }

    /// Names of the open screens, bottom first: none while the game hasn't
    /// loaded.
    pub fn screens(&self) -> Vec<&'static str> {
        self.app.game().map_or_else(Vec::new, Game::screens)
    }

    /// The top-most screen of type `T` on the stack, if it opts in
    /// ([`Game::screen`]).
    pub fn screen<T: Any>(&self) -> Option<&T> {
        self.game().screen()
    }

    /// [`screen`](Self::screen), mutable, to drive it with scripted
    /// commands.
    pub fn screen_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.game_mut().screen_mut()
    }

    /// The current frame in the snapshot format of
    /// [`shuyi_ui::frame::snapshot`]. The second cell of a wide character
    /// (Japanese, Chinese) prints as nothing, so a row of wide text reads
    /// as it is drawn; its colour row still has a key a cell.
    pub fn snapshot(&self) -> String {
        let game = self.game();
        game.buffer().to_snapshot(&game.ctx().palette)
    }

    /// The current frame as it would look had the player pressed `device`
    /// last: the same screen with that device's key or button names. For
    /// comparing play on a controller with the same play on the keyboard.
    pub fn snapshot_as(&mut self, device: Device) -> String {
        let game = self.game_mut();
        let used = core::mem::replace(&mut game.ctx_mut().device, device);
        game.redraw();
        let snapshot = self.snapshot();
        let game = self.game_mut();
        game.ctx_mut().device = used;
        game.redraw();
        snapshot
    }

    /// Every action held back from a screen that doesn't declare it
    /// ([`Screen::actions`](shuyi_ui::Screen::actions)), oldest first, with
    /// the screen's name: why a screen didn't react to a key.
    pub fn undeclared(&self) -> &[(&'static str, G::Action)] {
        self.game().undeclared_actions()
    }

    /// Whether the game has asked to quit.
    pub fn quit_requested(&self) -> bool {
        self.game().quit_requested()
    }

    /// The game being driven.
    pub fn game(&self) -> &Game<G> {
        self.app
            .game()
            .unwrap_or_else(|| panic!("Harness: the game isn't running"))
    }

    /// The game being driven, mutable.
    fn game_mut(&mut self) -> &mut Game<G> {
        self.app
            .game_mut()
            .unwrap_or_else(|| panic!("Harness: the game isn't running"))
    }

    /// The [`App`] running the game.
    pub fn app(&self) -> &App<G> {
        &self.app
    }

    /// The [`App`] running the game, mutable, to take what it hands a
    /// backend ([`App::take_session_save`]).
    pub fn app_mut(&mut self) -> &mut App<G> {
        &mut self.app
    }

    /// The game's log, kept by its [`App`]: every line from frame 0 (what
    /// its context logged before the Harness started), each with the frame
    /// it was given in.
    pub fn log(&self) -> &LogRing {
        self.app.log()
    }

    /// The frames' records, kept by its [`App`] (ADR-0046): one for each
    /// frame measured, with a clock set ([`set_clock`](Self::set_clock))
    /// and debug tools on.
    pub fn perf(&self) -> &PerfRing {
        self.app.perf()
    }

    /// Measures frames with `clock` from the next frame, or stops (`None`):
    /// the headless backend's clock, and the game's context's, since the
    /// game runs. A fake clock ([`PerfClock::fake`]) gives the same records
    /// on every run.
    pub fn set_clock(&mut self, clock: Option<PerfClock>) -> &mut Self {
        self.platform.set_clock(clock);
        self.ctx_mut().set_perf_clock(clock);
        self
    }

    /// Tags what the game allocates with `scopes` from the next frame, or
    /// stops (`None`): the headless backend's memory tags, and the game's
    /// context's, since the game runs ([`MemoryScopes`]).
    pub fn set_scopes(&mut self, scopes: Option<MemoryScopes>) -> &mut Self {
        self.platform.set_scopes(scopes);
        self.ctx_mut().set_memory_scopes(scopes);
        self
    }

    /// Reads the heap with `memory` (a real counter's reading) at the end
    /// of each frame measured from now on, or reads none (`None`): each
    /// record's [`heap`](shuyi_ui::perf::FrameRecord::heap). A fake memory
    /// that moves by fixed amounts is the platform's
    /// ([`HeadlessPlatform::with_fake_memory`], with [`Harness::on`]).
    pub fn set_memory(&mut self, memory: Option<fn() -> HeapCounts>) -> &mut Self {
        self.platform.set_memory(memory);
        self
    }

    /// Takes heap snapshots with `snapshot` (a real counter's) from now
    /// on, or none (`None`). A fake memory takes its own
    /// ([`HeadlessPlatform::with_fake_memory`]).
    pub fn set_heap_snapshots(&mut self, snapshot: Option<fn() -> HeapSnapshot>) -> &mut Self {
        self.platform.set_heap_snapshots(snapshot);
        self
    }

    /// What holds the heap now, by memory tag, stamped with the frames run
    /// ([`App::heap_snapshot`]): a fake memory's, or a counter's set with
    /// [`set_heap_snapshots`](Self::set_heap_snapshots); `None` without
    /// either, or with debug tools off. A test takes two and compares them
    /// ([`HeapSnapshot::diff`]).
    pub fn heap_snapshot(&mut self) -> Option<HeapSnapshot> {
        self.app.heap_snapshot(&mut self.platform)
    }

    /// The headless backend it runs on: the frames presented, the audio
    /// played.
    pub fn platform(&self) -> &HeadlessPlatform {
        &self.platform
    }

    /// The shared context.
    pub fn ctx(&self) -> &Ctx<G> {
        self.game().ctx()
    }

    /// The shared context, to change it between key presses, e.g.
    /// `h.ctx_mut().set_layout_bindings(…)` to rebind keys as a key
    /// bindings screen would. The next frame uses the new keys.
    pub fn ctx_mut(&mut self) -> &mut Ctx<G> {
        self.game_mut().ctx_mut()
    }

    /// Ends the run, handing back its storage, to start the next launch
    /// with it.
    pub fn into_storage(self) -> Box<dyn Storage> {
        let game = self
            .app
            .into_game()
            .unwrap_or_else(|| panic!("Harness: the game isn't running"));
        game.into_ctx().storage.into_computer()
    }
}

/// Plays the session record in `text` from `start`, stamps checked, to its
/// end ([`Harness::from_session`]). The error says why it can't be played,
/// or names the first frame that played differently.
pub fn check_session<G: GameData>(start: Start<G>, text: &str) -> Result<Played, String> {
    let record = SessionRecord::from_ron(text).map_err(|e| e.to_string())?;
    let mut harness =
        Harness::from_session(start, &record, Stamps::Check).map_err(|e| e.to_string())?;
    let played = harness.play_all().clone();
    match played.first_difference {
        Some(difference) => Err(format!(
            "frame {} played differently: the record says {}, this build {}",
            difference.frame, difference.recorded, difference.played
        )),
        None => Ok(played),
    }
}

/// The session record in `text` made again from `start`: every frame played,
/// differences and all, and recorded. The same inputs, this build's stamp
/// and hashes. The error says why it can't be played, or that this build
/// reads the platform differently, so its inputs are no longer the record's.
pub fn bless_session<G: GameData>(start: Start<G>, text: &str) -> Result<String, String> {
    let old = SessionRecord::from_ron(text).map_err(|e| e.to_string())?;
    let (start, _) = session::prepare(start, &old, Stamps::Ignore).map_err(|e| e.to_string())?;
    let mut platform = HeadlessPlatform::from_session(&old);
    let mut app = App::new(start);
    // No frame has run, so it always turns on.
    let _ = app.record();
    let mut playback = Playback::new(&old);
    while playback.next(&mut app, &mut platform).is_some() {}
    let mut new = app
        .take_session()
        .ok_or("the record stopped being recorded")?;
    new.voice_manifest.clone_from(&old.voice_manifest);
    let inputs = |record: &SessionRecord| -> Vec<Vec<RawInputEvent>> {
        let frames = record.frames.iter();
        frames.map(|frame| frame.input.clone()).collect()
    };
    if inputs(&new) != inputs(&old) {
        return Err("this build reads the platform differently: record the session again".into());
    }
    Ok(new.to_ron())
}

fn parse(chord: &str) -> Chord {
    match Chord::parse(chord) {
        Ok(chord) => chord,
        Err(e) => panic!("bad chord in test script: {e}"),
    }
}

fn parse_button(button: &str) -> Button {
    match Button::parse(button) {
        Ok(button) => button,
        Err(e) => panic!("bad button in test script: {e}"),
    }
}
