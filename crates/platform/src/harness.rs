//! A headless test driver for any game on the engine (game ADR-0007 layer 4):
//! scripted key and controller-button presses and waits in, in frames of
//! 1/60 s, screen names, snapshots and the audio asked for out. No window,
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

use crate::alloc_prelude::*;
use alloc::{format, vec};
use core::any::Any;

use shuyi_assets::keymap::{Button, Chord, CrankDef};
use shuyi_ui::audio::{AudioRequest, MusicClock, MusicCommand};
use shuyi_ui::input::crank::Crank;
use shuyi_ui::input::pad::PadKind;
use shuyi_ui::input::state::Device;
use shuyi_ui::log::LogRing;
use shuyi_ui::perf::{HeapCounts, HeapSnapshot, MemoryScopes, PerfClock, PerfRing};
use shuyi_ui::storage::Storage;
use shuyi_ui::{Ctx, Game, GameData, KeyPrompt, RawInputEvent};

use crate::app::{App, FrameOutcome, SystemHook};
use crate::headless::{HeadlessPlatform, PlayingSound, Presented};
use crate::platform::SystemEvent;

/// Simulated length of one frame, in seconds (60 fps).
pub const FRAME_DT: f32 = 1.0 / 60.0;

/// Most frames one [`Harness::hold`] or [`Harness::wait`] may run (about
/// half a simulated minute), so a runaway script fails instead of hanging.
pub const MAX_FRAMES: u32 = 2_000;

/// Drives a [`Game`] with simulated key presses and time, as the platform
/// would: in an [`App`], on the [`HeadlessPlatform`].
pub struct Harness<G: GameData> {
    app: App<G>,
    platform: HeadlessPlatform,
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
        let stacks = vec![(0, game.screens())];
        Self {
            app: App::running(game),
            platform,
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

    /// Presses and releases each whitespace-separated chord in turn, e.g.
    /// `"Down Down f"` or `"Shift+Space Enter"`: a frame with the press,
    /// then a frame with the release, each [`FRAME_DT`] long.
    ///
    /// # Panics
    ///
    /// On a chord [`Chord::parse`] rejects.
    pub fn keys(&mut self, script: &str) -> &mut Self {
        for token in script.split_whitespace() {
            let chord = parse(token);
            self.frame(&[RawInputEvent::Down(chord)], FRAME_DT);
            self.frame(&[RawInputEvent::Up(chord.key)], FRAME_DT);
        }
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
        for token in script.split_whitespace() {
            let buttons: Vec<Button> = token.split('+').map(parse_button).collect();
            let down: Vec<_> = buttons
                .iter()
                .map(|&b| RawInputEvent::PadDown(b, self.pad_kind))
                .collect();
            self.frame(&down, FRAME_DT);
            let up: Vec<_> = buttons.iter().map(|&b| RawInputEvent::PadUp(b)).collect();
            self.frame(&up, FRAME_DT);
        }
        self
    }

    /// Notches of `def` for [`crank`](Self::crank) (the game's keymap
    /// file's `crank`; 30° until set).
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
        self.platform.push_input(events);
        let outcome = self.app.frame_after(&mut self.platform, dt);
        self.frames += 1;
        self.audio.push(self.app.audio().to_vec());
        let music = self.app.music();
        self.music.extend_from_slice(music);
        self.music_frames
            .extend(core::iter::repeat_n(self.frames, music.len()));
        if outcome == FrameOutcome::Quit && self.quit_frame.is_none() {
            self.quit_frame = Some(self.frames);
        }
        let screens = self.screens();
        if self.stacks.last().is_none_or(|(_, last)| *last != screens) {
            self.stacks.push((self.frames, screens));
        }
        self
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

    /// Name of the top screen; empty once every screen has closed.
    pub fn top_screen(&self) -> &'static str {
        self.game().top_screen().unwrap_or("")
    }

    /// Names of the open screens, bottom first.
    pub fn screens(&self) -> Vec<&'static str> {
        self.game().screens()
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
    /// [`shuyi_ui::frame::snapshot`].
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
