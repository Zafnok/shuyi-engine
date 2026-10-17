//! A headless test driver for any game on the engine (ADR-0007 layer 4):
//! scripted key and controller-button presses and waits in, in frames of
//! 1/60 s, screen names, snapshots and the audio asked for out. No window,
//! no clock, no sound card: a stand-in for the music player says what
//! sounds, so a screen that keeps time with the music is tested too.
//!
//! A game wraps it with its own start-up and lookups (its flow, its
//! battle); the scripts are the same for every game. With the `harness`
//! feature.
//!
//! ```
//! use shuyi_ui::harness::Harness;
//! use shuyi_ui::toy;
//!
//! let mut h = Harness::new(toy::game());
//! h.keys("f");
//! assert_eq!(h.screens(), ["home", "menu"]);
//! h.keys("f");
//! assert_eq!(h.sounds(), ["click"]);
//! ```

use crate::prelude::*;
use core::any::Any;

use shuyi_assets::keymap::{Button, Chord};

use crate::audio::{AudioRequest, MusicClock, MusicCommand};
use crate::ctx::{Ctx, GameData, KeyPrompt};
use crate::game::{Game, RawInputEvent};
use crate::input::pad::PadKind;
use crate::input::state::Device;
use crate::storage::Storage;

/// Simulated length of one frame, in seconds (60 fps).
pub const FRAME_DT: f32 = 1.0 / 60.0;

/// Most frames one [`Harness::hold`] or [`Harness::wait`] may run (about
/// half a simulated minute), so a runaway script fails instead of hanging.
pub const MAX_FRAMES: u32 = 2_000;

/// Drives a [`Game`] with simulated key presses and time, as the platform
/// would.
pub struct Harness<G: GameData> {
    game: Game<G>,
    /// Every frame's audio requests, oldest first.
    audio: Vec<Vec<AudioRequest<G::Voice>>>,
    /// Every music command of the run, in order.
    music: Vec<MusicCommand>,
    /// The simulated music player, for [`Ctx::music_clock`].
    player: Player,
    /// The kind of controller [`pad`](Self::pad) presses buttons on.
    pad_kind: PadKind,
    /// What the last frame told the platform: the music and sound volumes
    /// and whether to fill the screen.
    mixer: (f32, f32, bool),
}

/// The Harness's stand-in for the platform's music player: which track
/// "sounds" and for how long, worked out from the music commands and the
/// frame times.
#[derive(Debug, Default)]
struct Player {
    /// Seconds a track takes to load: it sounds that long after its
    /// `Start`.
    load_delay: f64,
    /// Whether no track ever sounds (the music files are missing).
    silent: bool,
    /// The track started last, and the seconds since its `Start`.
    started: Option<(String, f64)>,
}

impl Player {
    /// Lets `dt` seconds pass (nonsense frame times don't count, as in
    /// [`Game`]).
    fn advance(&mut self, dt: f32) {
        if let Some((_, since)) = &mut self.started
            && dt.is_finite()
        {
            *since += f64::from(dt.max(0.0));
        }
    }

    /// What the platform would report: the track sounding and for how
    /// long.
    fn playing(&self) -> Option<(&str, f64)> {
        let (cue, since) = self.started.as_ref().filter(|_| !self.silent)?;
        let elapsed = since - self.load_delay;
        (elapsed >= 0.0).then_some((cue.as_str(), elapsed))
    }

    /// Follows one music command: a `Start` starts the count from 0; a
    /// `Stop` (or a fresh `Load`) of that track ends it.
    fn apply(&mut self, command: &MusicCommand) {
        match command {
            MusicCommand::Start { cue } => self.started = Some((cue.clone(), 0.0)),
            MusicCommand::Stop { cue } | MusicCommand::Load { cue } => {
                if self.started.as_ref().is_some_and(|(c, _)| c == cue) {
                    self.started = None;
                }
            }
            MusicCommand::Gain { .. } => {}
        }
    }
}

impl<G: GameData> Harness<G> {
    /// Drives `game` from where it is.
    pub fn new(game: Game<G>) -> Self {
        Self {
            game,
            audio: Vec::new(),
            music: Vec::new(),
            player: Player::default(),
            pad_kind: PadKind::default(),
            // Nothing told yet: as the platform starts.
            mixer: (1.0, 1.0, false),
        }
    }

    /// Waits for a key or button press, as a platform's first screen may
    /// ([`KeyPrompt::Waiting`]), and redraws.
    #[must_use]
    pub fn at_prompt(mut self) -> Self {
        self.game.ctx_mut().key_prompt = KeyPrompt::Waiting;
        self.game.redraw();
        self
    }

    /// Makes music take `seconds` to load: a track starts sounding, and
    /// [`Ctx::music_clock`] starts counting, that long after the game
    /// starts it (at once by default). Negative or NaN counts as 0.
    pub fn music_load_delay(&mut self, seconds: f32) -> &mut Self {
        self.player.load_delay = f64::from(seconds.max(0.0));
        self
    }

    /// Makes no music ever sound, as when the music files are missing:
    /// [`Ctx::music_clock`] stays `None`.
    pub fn without_music(&mut self) -> &mut Self {
        self.player.silent = true;
        self
    }

    /// Where the music is in its track, as the screens saw it in the last
    /// frame ([`Ctx::music_clock`]).
    pub fn music_clock(&self) -> Option<&MusicClock> {
        self.game.ctx().music_clock.as_ref()
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

    /// Runs one frame of `dt` seconds with `events`, as the platform does:
    /// reports the music sounding, runs the frame, then records and
    /// "plays" its audio. The scripts above are made of these.
    pub fn frame(&mut self, events: &[RawInputEvent], dt: f32) -> &mut Self {
        self.player.advance(dt);
        self.game.set_music_playing(self.player.playing());
        let out = self.game.frame(events, dt);
        self.mixer = (out.music_volume, out.sound_volume, out.fullscreen);
        self.audio.push(out.audio.to_vec());
        self.music.extend_from_slice(out.music);
        for command in out.music {
            self.player.apply(command);
        }
        self
    }

    /// Every audio request of the run so far, in order.
    pub fn audio_requests(&self) -> Vec<AudioRequest<G::Voice>> {
        self.audio.concat()
    }

    /// The sound cues (not music) played so far, in order.
    pub fn sounds(&self) -> Vec<String> {
        self.audio
            .iter()
            .flatten()
            .filter_map(|r| match r {
                AudioRequest::PlaySound { cue, .. } => Some(cue.clone()),
                _ => None,
            })
            .collect()
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
    /// music's and the sounds' ([`FrameOutput`](crate::FrameOutput)).
    pub fn volumes(&self) -> (f32, f32) {
        (self.mixer.0, self.mixer.1)
    }

    /// Whether the last frame told the platform to fill the screen.
    pub fn fullscreen(&self) -> bool {
        self.mixer.2
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
        self
    }

    /// Name of the top screen; empty once every screen has closed.
    pub fn top_screen(&self) -> &'static str {
        self.game.top_screen().unwrap_or("")
    }

    /// Names of the open screens, bottom first.
    pub fn screens(&self) -> Vec<&'static str> {
        self.game.screens()
    }

    /// The top-most screen of type `T` on the stack, if it opts in
    /// ([`Game::screen`]).
    pub fn screen<T: Any>(&self) -> Option<&T> {
        self.game.screen()
    }

    /// [`screen`](Self::screen), mutable, to drive it with scripted
    /// commands.
    pub fn screen_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.game.screen_mut()
    }

    /// The current frame in the snapshot format of
    /// [`crate::frame::snapshot`].
    pub fn snapshot(&self) -> String {
        self.game.buffer().to_snapshot(&self.game.ctx().palette)
    }

    /// The current frame as it would look had the player pressed `device`
    /// last: the same screen with that device's key or button names. For
    /// comparing play on a controller with the same play on the keyboard.
    pub fn snapshot_as(&mut self, device: Device) -> String {
        let used = core::mem::replace(&mut self.game.ctx_mut().device, device);
        self.game.redraw();
        let snapshot = self.snapshot();
        self.game.ctx_mut().device = used;
        self.game.redraw();
        snapshot
    }

    /// Whether the game has asked to quit.
    pub fn quit_requested(&self) -> bool {
        self.game.quit_requested()
    }

    /// The game being driven.
    pub fn game(&self) -> &Game<G> {
        &self.game
    }

    /// The shared context.
    pub fn ctx(&self) -> &Ctx<G> {
        self.game.ctx()
    }

    /// The shared context, to change it between key presses, e.g.
    /// `h.ctx_mut().set_layout_bindings(…)` to rebind keys as a key
    /// bindings screen would. The next frame uses the new keys.
    pub fn ctx_mut(&mut self) -> &mut Ctx<G> {
        self.game.ctx_mut()
    }

    /// Ends the run, handing back its storage, to start the next launch
    /// with it.
    pub fn into_storage(self) -> Box<dyn Storage> {
        self.game.into_ctx().storage.into_computer()
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

#[cfg(test)]
mod tests;
