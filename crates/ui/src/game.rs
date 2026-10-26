//! [`Game`]: a whole game's UI behind one call per frame. The platform
//! feeds it raw key and controller-button events and the frame time and
//! shows the buffer it returns; a test drives it the same way without a
//! window.
//!
//! The game gives the screen it starts on and its [`GameHooks`]: the
//! screen the first key opens while no layout is chosen (a layout picker),
//! and the debug menu. Modules from outside the default engine join it
//! with [`Game::with_module`] ([`crate::module`]).

use crate::prelude::*;
use core::any::Any;
use core::fmt;

use shuyi_assets::audio::AudioManifest;
use shuyi_assets::keymap::{ActionSet, Button, Chord, Key, UiAction};

use crate::audio::{AudioRequest, MusicClock, MusicCommand, MusicState};
use crate::ctx::{Ctx, GameData, KeyPrompt, Keyboard};
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::crank::Crank;
use crate::input::frame_seconds;
use crate::input::pad::{PadId, PadKind, PadState, Pads};
use crate::input::state::{Device, InputState};
use crate::module::{self, Hook, Install, InstalledModule, Module, ModuleError};
use crate::perf::{self, TagClass};
use crate::screen::{FrameInput, Screen, ScreenStack, Transition};

/// A keyboard or controller event as the platform reports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawInputEvent {
    /// A key went down, with the Shift state at the time.
    Down(Chord),
    /// A key went up.
    Up(Key),
    /// A character was typed (the platform's text input: the keyboard's own
    /// layout and Shift applied), for text boxes such as a name. Comes with
    /// the key's own `Down`.
    Text(char),
    /// A controller button went down on a pad of this kind (its binding
    /// position, from [`Pads::update`]).
    PadDown(Button, PadKind),
    /// A controller button went up on every pad.
    PadUp(Button),
}

impl RawInputEvent {
    /// This frame's controller events: what [`Pads::update`] reports for
    /// the pads `connected` now, releases first, each press with the kind
    /// of the pad that pressed it.
    pub fn from_pads(pads: &mut Pads, connected: &[(PadId, PadKind, PadState)]) -> Vec<Self> {
        let changes = pads.update(connected);
        let event = |(button, pressed)| {
            if pressed {
                let kind = pads.kind_holding(button, connected).unwrap_or_default();
                Self::PadDown(button, kind)
            } else {
                Self::PadUp(button)
            }
        };
        changes.into_iter().map(event).collect()
    }

    /// This frame's crank events: the crank turned `degrees` since the
    /// last frame (forwards positive), on a pad of `kind`; `docked` if it
    /// is folded away (then nothing, and a part notch is forgotten). Each
    /// notch is a press and a release of [`Button::CrankBack`] or
    /// [`Button::CrankForward`] ([`Crank`]).
    pub fn from_crank(crank: &mut Crank, degrees: f32, docked: bool, kind: PadKind) -> Vec<Self> {
        if docked {
            crank.dock();
            return Vec::new();
        }
        let Some((button, times)) = Crank::presses(crank.turn(degrees)) else {
            return Vec::new();
        };
        (0..times)
            .flat_map(|_| [Self::PadDown(button, kind), Self::PadUp(button)])
            .collect()
    }
}

/// The result of one frame. `V` is the game's voice clip variant.
#[derive(Debug, Clone, Copy)]
pub struct FrameOutput<'a, V> {
    /// What to show.
    pub buffer: &'a GlyphBuffer,
    /// Whether the game has asked to quit (it then ignores further input).
    pub quit: bool,
    /// Every audio request screens made this frame, in order. The platform
    /// plays the sounds; the music requests are already turned into
    /// [`music`].
    ///
    /// [`music`]: Self::music
    pub audio: &'a [AudioRequest<V>],
    /// What to do to the music this frame.
    pub music: &'a [MusicCommand],
    /// The player's music volume, 0–1: the platform multiplies the music's
    /// own volume by it, also for a track already playing.
    pub music_volume: f32,
    /// The player's sound volume, 0–1: the platform multiplies every
    /// sound's own volume by it.
    pub sound_volume: f32,
    /// Whether the game should fill the screen (the Fullscreen setting);
    /// the platform switches when this changes.
    pub fullscreen: bool,
    /// How loud voice clips play, 0–1 ([`Ctx::voice_volume`]). The
    /// platform applies it to the voice that is playing too.
    pub voice_volume: f32,
}

/// A screen the engine opens for the game, by name.
pub struct ScreenHook<G: GameData> {
    /// The screen's [`name`](Screen::name).
    pub name: &'static str,
    /// Makes the screen.
    pub open: fn(&Ctx<G>) -> Box<dyn Screen<G>>,
}

impl<G: GameData> Clone for ScreenHook<G> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<G: GameData> Copy for ScreenHook<G> {}

// check-text: not player text (debug output)
impl<G: GameData> fmt::Debug for ScreenHook<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ScreenHook({})", self.name)
    }
}

/// Where the game has a say in the frame loop. [`Default`] has none.
#[derive(Debug)]
pub struct GameHooks<G: GameData> {
    /// What the first key press opens while no layout is chosen, on a
    /// platform with a keyboard (a layout picker): it opens over whatever
    /// shows, and the key does nothing else. Never while a screen of its
    /// name is open.
    pub first_key: Option<ScreenHook<G>>,
    /// What the Debug action ([`UiAction::Debug`]) opens with debug tools
    /// on ([`Ctx::debug_tools`]), unless the top screen is one of
    /// [`debug_screens`](Self::debug_screens).
    pub debug: Option<ScreenHook<G>>,
    /// The debug tools' screens, over which the Debug action does nothing.
    pub debug_screens: &'static [&'static str],
}

impl<G: GameData> Default for GameHooks<G> {
    fn default() -> Self {
        Self {
            first_key: None,
            debug: None,
            debug_screens: &[],
        }
    }
}

/// Owns the screens, input state, shared context and the console buffer.
pub struct Game<G: GameData> {
    stack: ScreenStack<G>,
    input: InputState<G::Action>,
    ctx: Ctx<G>,
    hooks: GameHooks<G>,
    buffer: GlyphBuffer,
    quit: bool,
    music: MusicState,
    /// This frame's audio requests and music commands.
    audio_out: Vec<AudioRequest<G::Voice>>,
    music_out: Vec<MusicCommand>,
    /// The fixed simulation steps due in the next frame
    /// ([`set_ticks`](Self::set_ticks)).
    ticks: u32,
    /// The installed modules, and what each declared, in install order.
    modules: Vec<Box<dyn Module<G>>>,
    installed: Vec<InstalledModule>,
}

impl<G: GameData> Game<G> {
    /// A game showing `root`, with the game's `hooks`.
    pub fn new(ctx: Ctx<G>, root: Box<dyn Screen<G>>, hooks: GameHooks<G>) -> Self {
        Self::with_stack(ctx, ScreenStack::new(root), hooks)
    }

    /// A game showing the screens of `stack`, with the game's `hooks`,
    /// already drawn. Music fades for as long as the audio manifest says.
    pub fn with_stack(ctx: Ctx<G>, stack: ScreenStack<G>, hooks: GameHooks<G>) -> Self {
        let input = InputState::new(ctx.keymap.clone()).starting_on(ctx.device);
        let blank = ctx.game.blank_cell(&ctx.palette);
        #[allow(clippy::cast_precision_loss)] // A fade is far shorter than 2^24 ms.
        let fade_secs = ctx.game.audio_manifest().music_fade_ms as f32 / 1000.0;
        let mut game = Self {
            stack,
            input,
            buffer: GlyphBuffer::with_spec(ctx.console, blank),
            ctx,
            hooks,
            quit: false,
            music: MusicState::new(fade_secs),
            audio_out: Vec::new(),
            music_out: Vec::new(),
            ticks: 0,
            modules: Vec::new(),
            installed: Vec::new(),
        };
        game.redraw();
        game
    }

    /// The same game with `module` installed after those it has: its
    /// [`install`](Module::install) runs, and the frame is drawn again
    /// with what it draws. Refused for a name already installed or a state
    /// another module put on the context.
    pub fn with_module(self, module: impl Module<G>) -> Result<Self, ModuleError> {
        self.with_modules([Box::new(module) as Box<dyn Module<G>>])
    }

    /// The same game with `modules` installed in order, as
    /// [`with_module`](Self::with_module) installs one.
    pub fn with_modules(
        mut self,
        modules: impl IntoIterator<Item = Box<dyn Module<G>>>,
    ) -> Result<Self, ModuleError> {
        for module in modules {
            self.install(module)?;
        }
        self.redraw();
        Ok(self)
    }

    fn install(&mut self, mut module: Box<dyn Module<G>>) -> Result<(), ModuleError> {
        let name = module.name();
        debug_assert!(
            module::is_snake_case(name),
            "a module's name is snake_case ASCII: {name:?}"
        );
        if self.installed.iter().any(|m| m.name == name) {
            return Err(ModuleError::SameName { name });
        }
        let mut install = Install::new(&mut self.ctx, name);
        module.install(&mut install);
        self.installed.push(install.finish()?);
        self.modules.push(module);
        Ok(())
    }

    /// The installed modules, in install order, with what each declared.
    pub fn modules(&self) -> &[InstalledModule] {
        &self.installed
    }

    /// Turns the debug tools (the Debug action) on or off
    /// ([`Ctx::debug_tools`]).
    #[must_use]
    pub fn with_debug_screens(mut self, on: bool) -> Self {
        self.ctx.debug_tools = on;
        self
    }

    /// Turns the screens' action guard on or off
    /// ([`ScreenStack::guard_actions`]): while it is on, a screen that
    /// declares its actions is handed only those. The test Harness turns
    /// it on.
    pub fn guard_actions(&mut self, on: bool) {
        self.stack.guard_actions(on);
    }

    /// Every action held back from a screen that doesn't declare it, with
    /// the screen's name ([`ScreenStack::undeclared`]).
    pub fn undeclared_actions(&self) -> &[(&'static str, G::Action)] {
        self.stack.undeclared()
    }

    /// Tells the game what music is sounding, for [`Ctx::music_clock`]: the
    /// cue and the seconds since the track really started, or `None` in
    /// silence. The platform knows (a track starts once its file has
    /// loaded) and calls this before every [`frame`](Self::frame), so
    /// screens read a clock at most one frame old.
    pub fn set_music_playing(&mut self, playing: Option<(&str, f64)>) {
        let manifest = self.ctx.game.audio_manifest();
        self.ctx.music_clock =
            playing.and_then(|(cue, elapsed)| MusicClock::from_elapsed(manifest, cue, elapsed));
    }

    /// Says how many fixed simulation steps are due in the next
    /// [`frame`](Self::frame) (ADR-0003 §2): its screen reads them as
    /// [`FrameInput::ticks`]. The platform's `App` counts them from its
    /// clock and calls this before every frame; a frame after none has 0.
    pub fn set_ticks(&mut self, ticks: u32) {
        self.ticks = ticks;
    }

    /// Flips the Fullscreen setting, as a window's own shortcut does. The
    /// platform then switches the window on the next frame, and an options
    /// screen shows the new value.
    pub fn toggle_fullscreen(&mut self) {
        let _ = self.ctx.change_settings(|s| s.fullscreen = !s.fullscreen);
    }

    /// Runs one frame: applies `events` (in order), advances input by `dt`
    /// seconds, updates the top screen with the resulting actions, collects
    /// its audio requests and redraws. After a quit, frames do nothing. In
    /// a frame the platform measures ([`Ctx::begin_perf_frame`]) it times
    /// the update and the drawing, and with memory scopes
    /// ([`Ctx::set_memory_scopes`]) it tags what each allocates `update`
    /// and `draw` ([`crate::perf`]).
    pub fn frame(&mut self, events: &[RawInputEvent], dt: f32) -> FrameOutput<'_, G::Voice> {
        self.audio_out.clear();
        self.music_out.clear();
        // Nonsense frame times pass no time, for the screens, the clock and
        // the music alike.
        let dt = frame_seconds(dt);
        let ticks = core::mem::take(&mut self.ticks);
        if !self.quit {
            // The update's and the drawing's times, in a frame the
            // platform measures, and with memory scopes their tags
            // (crate::perf).
            let tagged = self.ctx.enter_tag(TagClass::Phase, "update");
            let start = self.ctx.perf_now_in_frame();
            self.step(events, dt, ticks);
            let drawing = self.ctx.perf_now_in_frame();
            perf::leave_tag(tagged);
            let tagged = self.ctx.enter_tag(TagClass::Phase, "draw");
            self.redraw();
            let drawn = self.ctx.perf_now_in_frame();
            perf::leave_tag(tagged);
            self.ctx.set_perf_frame_times(start, drawing, drawn);
            self.collect_audio(dt);
        }
        FrameOutput {
            buffer: &self.buffer,
            quit: self.quit,
            audio: &self.audio_out,
            music: &self.music_out,
            music_volume: self.ctx.settings().music_factor(),
            sound_volume: self.ctx.settings().sound_factor(),
            fullscreen: self.ctx.settings().fullscreen,
            voice_volume: self.ctx.voice_gain(),
        }
    }

    /// Moves the screens' audio requests into this frame's output and runs
    /// the music state machine. In debug builds, a cue the manifest lacks
    /// is a bug in the screen and panics.
    fn collect_audio(&mut self, dt: f32) {
        self.audio_out = self.ctx.audio.take();
        for request in &self.audio_out {
            debug_assert!(
                is_known(self.ctx.game.audio_manifest(), request),
                "audio request for a cue not in the audio manifest: {request:?}"
            );
            self.music.request(request, &mut self.music_out);
        }
        self.music.update(dt, &mut self.music_out);
    }

    fn step(&mut self, events: &[RawInputEvent], dt: f32, ticks: u32) {
        // Bindings changed between frames (a test rebinding keys).
        self.sync_keymap();
        let mut pressed = Vec::new();
        let mut text = Vec::new();
        let mut buttons_down = Vec::new();
        let mut buttons_up = Vec::new();
        // Until a layout is chosen no key acts: the first one opens the
        // game's first-key screen over whatever is showing and does nothing
        // else; that screen then takes the keys. Without a keyboard, never.
        let first_key = self.hooks.first_key.filter(|hook| {
            self.ctx.keyboard == Keyboard::Present
                && self.ctx.layout().is_none()
                && !self.stack.names().contains(&hook.name)
                && events.iter().any(|e| matches!(e, RawInputEvent::Down(_)))
        });
        for &event in events {
            // Any key or any controller button ends the wait for one.
            let press = matches!(event, RawInputEvent::Down(_) | RawInputEvent::PadDown(..));
            let ends_wait = press && self.ctx.key_prompt == KeyPrompt::Waiting;
            if ends_wait {
                self.ctx.key_prompt = KeyPrompt::Pressed;
            }
            match event {
                RawInputEvent::Down(_) if first_key.is_some() => self.input.keyboard_used(),
                RawInputEvent::Down(chord) => {
                    self.input.key_down(chord);
                    pressed.push(chord);
                }
                RawInputEvent::Up(key) => self.input.key_up(key),
                // Control characters (Enter, Backspace, Escape on some
                // platforms) are keys, not text.
                RawInputEvent::Text(c) if !c.is_control() => text.push(c),
                RawInputEvent::Text(_) => {}
                RawInputEvent::PadDown(button, kind) => {
                    self.input.pad_down(button, kind);
                    if ends_wait {
                        // The wait takes it: nothing when let go either
                        // (a chord button's action).
                        self.input.pad_taken(button);
                    }
                    buttons_down.push(button);
                }
                RawInputEvent::PadUp(button) => {
                    self.input.pad_up(button);
                    buttons_up.push(button);
                }
            }
        }
        self.ctx.clock_s += f64::from(dt);
        self.ctx.device = self.input.device();
        let actions = self.input.update(dt);
        let held = self.held();
        for (module, installed) in self.modules.iter_mut().zip(&self.installed) {
            if installed.hooks.contains(&Hook::BeforeUpdate) {
                module.before_update(&mut self.ctx, dt);
            }
        }
        let debug = self.hooks.debug.filter(|_| {
            self.ctx.debug_tools
                && actions.iter().any(|a| a.ui() == Some(UiAction::Debug))
                && !self
                    .stack
                    .top_name()
                    .is_some_and(|n| self.hooks.debug_screens.contains(&n))
        });
        if let Some(hook) = first_key {
            self.open(hook);
        }
        if let Some(hook) = debug {
            self.open(hook);
        } else {
            // A chord button gives its own action when let go or after the
            // chord window: that is pad input too.
            let from_pad = !buttons_down.is_empty()
                || (!actions.is_empty() && matches!(self.ctx.device, Device::Pad(_)));
            let input = FrameInput::new(actions, dt, held)
                .with_typing(pressed, text)
                .with_buttons(buttons_down, buttons_up)
                .with_pad(from_pad)
                .with_ticks(ticks);
            self.quit = self.stack.update(&mut self.ctx, &input);
            self.sync_keymap();
        }
    }

    /// Hands the top screen `action` alone, in an update of its own with no
    /// time passing, as if a key bound to it had just been pressed: an
    /// action from outside the keys and buttons, such as an entry of the
    /// platform's own menu (the Playdate's system menu, ticket 1035). The
    /// screen does with it what it does with the key, or nothing. Its
    /// sounds come out with the next frame. After a quit, nothing.
    pub fn act(&mut self, action: G::Action) {
        if self.quit {
            return;
        }
        let held = self.held();
        let from_pad = matches!(self.ctx.device, Device::Pad(_));
        let input = FrameInput::new(vec![action], 0.0, held).with_pad(from_pad);
        self.quit = self.stack.update(&mut self.ctx, &input);
        self.sync_keymap();
        self.redraw();
    }

    /// Opens `hook`'s screen over the top one, which hears it is covered.
    fn open(&mut self, hook: ScreenHook<G>) {
        let screen = (hook.open)(&self.ctx);
        self.stack.apply_in(&mut self.ctx, Transition::Push(screen));
    }

    /// The actions whose keys or buttons are held.
    fn held(&self) -> Vec<G::Action> {
        G::Action::ALL
            .iter()
            .copied()
            .filter(|&a| self.input.is_held(a))
            .collect()
    }

    /// Hands `ctx`'s keymap to the input if it changed (a layout was
    /// picked, keys were rebound), so the new keys work from the next press.
    fn sync_keymap(&mut self) {
        if *self.input.keymap() != self.ctx.keymap {
            self.input.set_keymap(self.ctx.keymap.clone());
        }
    }

    /// Clears the buffer and draws the stack into it, then the modules
    /// over it: after a change made from outside the frame loop (a test
    /// changing the context).
    pub fn redraw(&mut self) {
        let blank = self.ctx.game.blank_cell(&self.ctx.palette);
        self.buffer.fill_rect(self.buffer.bounds(), blank);
        self.stack.draw(&self.ctx, &mut self.buffer);
        for (module, installed) in self.modules.iter().zip(&self.installed) {
            if installed.hooks.contains(&Hook::DrawOver) {
                module.draw_over(&self.ctx, &mut self.buffer);
            }
        }
    }

    /// The top-most screen of type `T` on the stack, if it opts in
    /// ([`Screen::as_any`]).
    pub fn screen<T: Any>(&self) -> Option<&T> {
        self.stack.find()
    }

    /// [`screen`](Self::screen), mutable, for scripted tests.
    pub fn screen_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.stack.find_mut()
    }

    /// The music state machine (which track plays).
    pub fn music(&self) -> &MusicState {
        &self.music
    }

    /// The input state (held keys, the keymap in use).
    pub fn input(&self) -> &InputState<G::Action> {
        &self.input
    }

    /// The last frame drawn.
    pub fn buffer(&self) -> &GlyphBuffer {
        &self.buffer
    }

    /// Whether the game has asked to quit.
    pub fn quit_requested(&self) -> bool {
        self.quit
    }

    /// The shared context.
    pub fn ctx(&self) -> &Ctx<G> {
        &self.ctx
    }

    /// The shared context, to change settings mid-run (tests and tools;
    /// screens get theirs in [`Screen::update`]).
    pub fn ctx_mut(&mut self) -> &mut Ctx<G> {
        &mut self.ctx
    }

    /// Ends the game, handing back the shared context (and so its storage).
    pub fn into_ctx(self) -> Ctx<G> {
        self.ctx
    }

    /// Name of the top screen, or `None` once the last one has closed.
    pub fn top_screen(&self) -> Option<&'static str> {
        self.stack.top_name()
    }

    /// Names of the open screens, bottom first.
    pub fn screens(&self) -> Vec<&'static str> {
        self.stack.names()
    }
}

/// Whether `request` names a cue of the right kind in `manifest`.
fn is_known<V>(manifest: &AudioManifest, request: &AudioRequest<V>) -> bool {
    match request {
        AudioRequest::PlaySound { cue, .. } => manifest.sounds.contains_key(cue),
        AudioRequest::PlayMusic { cue } => manifest.music.contains_key(cue),
        AudioRequest::StopMusic
        | AudioRequest::PlayVoice { .. }
        | AudioRequest::StopVoice
        | AudioRequest::PreloadVoices { .. } => true,
    }
}

#[cfg(test)]
mod tests;
