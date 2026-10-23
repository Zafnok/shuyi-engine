//! [`Ctx`]: what every screen shares, passed `&mut` to
//! [`Screen::update`](crate::Screen::update) and `&` to
//! [`Screen::draw`](crate::Screen::draw). The engine's services (text by
//! key in the player's language, the keys and the names help text gives
//! them, the player's settings and profiles, sounds, music and voices,
//! storage) and the game's own part, [`Ctx::game`], whose type says what
//! the engine reads from the game's data ([`GameData`]).
//!
//! A plain struct: no service locator, no globals. A game names its own
//! with an alias (`type Ctx = shuyi_ui::Ctx<MyGame>`).

use crate::prelude::*;
use core::fmt::{Debug, Display};

use shuyi_assets::audio::AudioManifest;
use shuyi_assets::dialogue::LineId;
use shuyi_assets::keymap::{ActionSet, KeymapDef, Layout};
use shuyi_assets::lang::{Lang, LangCode};
use shuyi_assets::palette::PaletteDef;
use shuyi_assets::voice::{ClipVariant, Playable};

use crate::audio::{AudioConfig, AudioQueue, MusicClock, pick_from_pool};
use crate::frame::color::{ColorSet, EngineColor, Palette};
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::Cell;
use crate::input::bindings::{LayoutBindings, PadBindings, PlayerKeys};
use crate::input::help_keys::HelpKeys;
use crate::input::keymap::Keymap;
use crate::input::state::Device;
use crate::log::{LOG_LINES, LogCategory, LogLevel, LogLine, LogRing};
use crate::profiles::{self, ProfileError, ProfileId, ProfileStore, Profiles};
use crate::settings::{GameSettings, SETTINGS_KEY, Settings};
use crate::storage::{Storage, StorageError};
use crate::system_keyboard::SystemKeyboard;

/// [`Storage`] key under which the player's key and controller-button
/// bindings are saved ([`PlayerKeys::to_ron`]).
pub const KEYBINDINGS_KEY: &str = "keybindings";

/// The text key of what help text shows for an action with no key.
pub const NOT_MAPPED_TEXT: &str = "keys.not_mapped";

/// The text key of the arrow keys' name in help text
/// ([`ARROWS`](crate::input::help_keys::ARROWS)).
pub const ARROWS_TEXT: &str = "keys.arrows";

/// The text key of what joins a button chord's two buttons in help text
/// (`A+B`).
pub const CHORD_JOIN_TEXT: &str = "keys.chord_join";

/// The text key of a profile's name when the player hasn't named it:
/// `Player {n}`, made on a first launch.
pub const PROFILE_NUMBERED_TEXT: &str = "profile.numbered";

/// The loudest [`Ctx::voice_volume`].
pub const MAX_VOICE_VOLUME: u8 = 10;
/// [`Ctx::voice_volume`] until the player changes it (*tunable*).
pub const DEFAULT_VOICE_VOLUME: u8 = 8;

/// [`Ctx::music_seed`] until the platform sets it.
pub const DEFAULT_MUSIC_SEED: u64 = 0;

/// The game's part of the [`Ctx`], and what the engine reads from the
/// game's data through it: its actions, colors, text, keys, audio
/// manifest and palettes, and the hooks where the game has a say.
pub trait GameData: Debug + Sized + 'static {
    /// The game's input actions.
    type Action: ActionSet;
    /// The colors the game's code draws with.
    type Colors: ColorSet;
    /// Which of a voiced line's clips plays.
    type Voice: ClipVariant;
    /// The settings the game adds to the engine's, and their saved form.
    type Settings: GameSettings;

    /// The screen text by key, in every language.
    fn lang(&self) -> &Lang;

    /// The keymap file: every layout's default keys, the layout picker's
    /// and the controller buttons.
    fn keymap_def(&self) -> &KeymapDef<Self::Action>;

    /// The sound and music cues.
    fn audio_manifest(&self) -> &AudioManifest;

    /// The palette the game starts with, before the settings are read.
    fn palette_def(&self) -> &PaletteDef;

    /// The console the game starts on ([`Ctx::console`]).
    fn console(&self) -> ConsoleSpec;

    /// The palette of a color `theme`; `None` keeps the palette there is.
    /// By default the game has one palette: always `None`.
    fn theme_def(&self, theme: <Self::Settings as GameSettings>::Theme) -> Option<&PaletteDef> {
        let _ = theme;
        None
    }

    /// Which of the game's sound cues the engine's widgets and screens
    /// play for a menu's moves and choices and a cursor's steps: by
    /// default the cues named after what they are
    /// ([`AudioConfig::DEFAULT`]).
    fn audio_config(&self) -> AudioConfig {
        AudioConfig::DEFAULT
    }

    /// The language only a build with debug tools shows (a test pack), if
    /// the game has one: without debug tools it is English.
    fn debug_lang(&self) -> Option<&str> {
        None
    }

    /// The text of a `{name}` placeholder that names a key
    /// ([`Ctx::text_with`]): by default the cursor keys for `{Cursor}` and
    /// the key of the action called `name` ([`text::key_text`]); `None`
    /// leaves it as written.
    ///
    /// [`text::key_text`]: crate::text::key_text
    fn key_text(&self, name: &str, keys: HelpKeys<'_, Self::Action>) -> Option<String> {
        crate::text::key_text(name, keys)
    }

    /// Which clip of a line plays now, where a line has several (who the
    /// player made the hero, say). By default the variant's default: a
    /// line's only clip.
    fn voice_variant(&self) -> Self::Voice {
        Self::Voice::default()
    }

    /// The clips that may be played, from the text of the voice manifest
    /// the platform read: those whose line still says what they say. `Err`
    /// says why the manifest can't be used. By default the game has no
    /// voices, and a manifest is refused.
    // check-text: not player text (warnings for the log)
    fn voices(&self, _manifest: &str) -> Result<Playable<Self::Voice>, String> {
        Err("this game has no voices".to_owned())
    }

    /// Takes what the settings say that the game draws with (a map's look,
    /// say): `before` is what they were (`None` when they were just
    /// loaded). The engine applies the theme itself.
    fn apply_settings(
        &mut self,
        _now: &Settings<Self::Settings>,
        _before: Option<&Settings<Self::Settings>>,
    ) {
    }

    /// The cell the frame is cleared to before the screens draw: by
    /// default a blank in the engine's text color on its panel color.
    fn blank_cell(&self, palette: &Palette) -> Cell {
        let text = palette.engine(EngineColor::Text);
        Cell::new(' ', text, palette.engine(EngineColor::Panel))
    }

    /// The [`Storage`] keys the game kept a player's data under before it
    /// had profiles: a first launch with profiles moves them into the
    /// first one ([`profiles::migrate`]). None by default.
    fn keys_before_profiles(&self) -> Vec<String> {
        vec![]
    }

    /// The [`Storage`] key a layout's name was saved under before the
    /// settings held it, if the game had one: still read when the settings
    /// name none, never written.
    fn old_layout_key(&self) -> Option<&'static str> {
        None
    }
}

/// Whether screens show their idle motion ([`Ctx::idle_motion`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdleMotion {
    /// Nothing moves while a screen is left alone.
    #[default]
    Off,
    /// Decoration moves.
    On,
}

/// Whether the platform has a keyboard ([`Ctx::keyboard`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Keyboard {
    /// It does (a computer, a browser): a first key press opens the
    /// game's first-key screen until a layout is chosen
    /// ([`GameHooks::first_key`](crate::GameHooks::first_key)).
    #[default]
    Present,
    /// It has only its own buttons (a console): that screen never opens.
    Absent,
}

/// The "press any key or button" wait ([`Ctx::key_prompt`]): browsers
/// block sound until the player presses a key.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum KeyPrompt {
    /// No prompt (tests that start at a menu).
    #[default]
    Off,
    /// Nothing pressed yet: a screen shows the prompt.
    Waiting,
    /// A key or a controller button has been pressed (on any screen).
    Pressed,
}

/// Resources shared by every screen: the engine's, and the game's in
/// [`game`](Self::game).
#[derive(Debug)]
pub struct Ctx<G: GameData> {
    /// The game's data and state, which its screens share.
    pub game: G,
    /// Named colors, from the game's palette and the color theme in the
    /// settings.
    pub palette: Palette,
    /// The console the frame is drawn on: the game's
    /// ([`GameData::console`]) unless a front end sets its own before the
    /// [`Game`](crate::Game) starts.
    pub console: ConsoleSpec,
    /// The active key bindings, for help text that names keys: the chosen
    /// layout's with the player's changes, or the layout picker's
    /// ([`Keymap::layout_picker`]) until one is chosen. Change it with
    /// [`use_layout`](Self::use_layout),
    /// [`choose_layout`](Self::choose_layout) or
    /// [`set_layout_bindings`](Self::set_layout_bindings), never directly.
    pub keymap: Keymap<G::Action>,
    /// What the player pressed last, the keyboard or a controller, so help
    /// text names keys or that pad's buttons ([`help_keys`]). `Game` keeps
    /// it up to date from the input, starting from the one set here when
    /// it starts (the keyboard, unless a front end without one sets its
    /// pad).
    ///
    /// [`help_keys`]: Self::help_keys
    pub device: Device,
    /// The layout in use; `None` until the player has picked one.
    layout: Option<Layout>,
    /// The player's key bindings for every layout and their controller
    /// buttons, loaded from `storage`.
    player_keys: PlayerKeys<G::Action>,
    /// The lines logged and not yet taken ([`log`](Self::log)).
    log_lines: LogRing,
    /// Where saves and settings persist: a
    /// [`MemoryStorage`](crate::storage::MemoryStorage) until the platform
    /// swaps in its own with [`with_storage`](Self::with_storage).
    ///
    /// It reads and writes the keys of the profile in use: its settings,
    /// key bindings and whatever the game keeps per player.
    /// [`computer`](ProfileStore::computer) is the storage of what stays
    /// with the computer.
    pub storage: ProfileStore,
    /// The player profiles and the one in use, loaded from the computer's
    /// storage. Change them with [`switch_profile`](Self::switch_profile),
    /// [`add_profile`](Self::add_profile),
    /// [`rename_profile`](Self::rename_profile) and
    /// [`delete_profile`](Self::delete_profile), which save them.
    profiles: Profiles,
    /// The player's settings, loaded from `storage`. Change them with
    /// [`change_settings`](Self::change_settings), which saves them, or
    /// with [`preview_settings`](Self::preview_settings), which doesn't,
    /// then [`keep_settings`](Self::keep_settings) or
    /// [`restore_settings`](Self::restore_settings).
    settings: Settings<G::Settings>,
    /// The settings as `storage` holds them: what was loaded or last
    /// written. [`settings`](Self::settings) differs from it while a
    /// previewed change is neither kept nor put back.
    stored_settings: Settings<G::Settings>,
    /// Whether debug tools are offered: the Debug action
    /// ([`GameHooks::debug`](crate::GameHooks::debug)) and whatever else
    /// the game offers only to developers. On in debug builds and with the
    /// `debug-tools` feature ([`DEBUG_TOOLS`](crate::DEBUG_TOOLS)).
    pub debug_tools: bool,
    /// Whether screens show their idle motion. Off here, so a screen left
    /// alone in a test looks the same a moment later; the platform turns
    /// it on.
    pub idle_motion: IdleMotion,
    /// Sounds and music screens ask for this frame, e.g.
    /// `ctx.audio.play_sound("click")`. `Game` passes them to the
    /// platform.
    pub audio: AudioQueue<G::Voice>,
    /// Where the music that is sounding is in its track, for a screen that
    /// keeps time with it. `None` in silence: nothing asked for, the track
    /// still loading, its file missing, or a track played once that has
    /// ended. During a fade it is the track fading out. The platform
    /// reports it each frame through
    /// [`Game::set_music_playing`](crate::Game::set_music_playing).
    pub music_clock: Option<MusicClock>,
    /// Whether voice clips play.
    pub voices_on: bool,
    /// How loud voice clips play, 0 to [`MAX_VOICE_VOLUME`].
    pub voice_volume: u8,
    /// The voice clips that may be played: none until the platform hands
    /// over a manifest ([`set_voice_manifest`](Self::set_voice_manifest)).
    voices: Playable<G::Voice>,
    /// Seeds random music picks (e.g. a track from a pool), kept apart
    /// from any simulation's randomness. A fixed [`DEFAULT_MUSIC_SEED`]
    /// here, so tests are repeatable; the platform sets it from the clock
    /// at startup so each launch picks differently.
    pub music_seed: u64,
    /// Random music picks made so far, mixed into
    /// [`music_seed`](Self::music_seed) so each pick rolls afresh
    /// ([`pick_music`](Self::pick_music)).
    music_picks: u64,
    /// Seconds the game has been running: the sum of every frame's time
    /// (`Game` adds it).
    pub clock_s: f64,
    /// Whether a screen waits for a key press before going on (and before
    /// playing music). Off here; the platform sets
    /// [`KeyPrompt::Waiting`], and `Game` moves it on to
    /// [`KeyPrompt::Pressed`] at the first key or button press.
    pub key_prompt: KeyPrompt,
    /// Whether the platform has a keyboard. [`Keyboard::Present`] here; a
    /// front end without one sets [`Keyboard::Absent`] before the
    /// [`Game`](crate::Game) starts, along with the
    /// [`device`](Self::device) it has.
    pub keyboard: Keyboard,
    /// The system's own keyboard for a text box (a console's, Steam's on
    /// the Steam Deck), if the platform has one: absent here, and the
    /// platform's `App` says so before every frame.
    pub system_keyboard: SystemKeyboard,
    /// The language screen text is shown in: English until the player
    /// picks another. The game's debug language
    /// ([`GameData::debug_lang`]) counts only with
    /// [`debug_tools`](Self::debug_tools) on; anywhere else it is English.
    pub lang: LangCode,
    /// English, for [`effective_lang`](Self::effective_lang).
    english: LangCode,
}

impl<G: GameData> Ctx<G> {
    /// The shared context over the game's part. Fails with the names of
    /// the colors the game's palette lacks. Starts with a
    /// [`MemoryStorage`](crate::storage::MemoryStorage), the default
    /// settings and no layout chosen (so the layout picker's keys are
    /// active).
    pub fn new(game: G) -> Result<Self, Vec<&'static str>> {
        let palette = Palette::new::<G::Colors>(game.palette_def())?;
        let keymap = Keymap::layout_picker(game.keymap_def());
        let settings = Settings::default();
        let audio = AudioQueue::new(game.audio_config());
        let profiles = Profiles::first(&numbered_profile(game.lang(), ProfileId::FIRST.get()));
        Ok(Self {
            console: game.console(),
            game,
            palette,
            keymap,
            device: Device::default(),
            layout: None,
            player_keys: PlayerKeys::default(),
            log_lines: LogRing::new(LOG_LINES),
            storage: ProfileStore::default(),
            profiles,
            stored_settings: settings.clone(),
            settings,
            debug_tools: crate::DEBUG_TOOLS,
            idle_motion: IdleMotion::Off,
            audio,
            music_clock: None,
            voices_on: true,
            voice_volume: DEFAULT_VOICE_VOLUME,
            voices: Playable::default(),
            music_seed: DEFAULT_MUSIC_SEED,
            music_picks: 0,
            clock_s: 0.0,
            key_prompt: KeyPrompt::Off,
            keyboard: Keyboard::Present,
            system_keyboard: SystemKeyboard::default(),
            lang: LangCode::english(),
            english: LangCode::english(),
        })
    }

    /// The screen text for `key` in the player's language, placeholders
    /// and all: the language's text if it has one made from today's
    /// English, else English.
    ///
    /// A key English lacks is a bug in the screen: it panics in debug
    /// builds, and shows as the key itself in release builds.
    pub fn text<'a>(&'a self, key: &'a str) -> &'a str {
        let lang = self.game.lang();
        debug_assert!(lang.has(key), "text key not in the English text: {key:?}");
        lang.text(self.effective_lang(), key)
    }

    /// [`text`](Self::text) with its placeholders filled in: each `{name}`
    /// that `args` names by its value, and each that names a key (an
    /// action's name, [`GameData::key_text`]) by the key the player has
    /// for it ([`help_keys`](Self::help_keys)).
    pub fn text_with(&self, key: &str, args: &[(&str, &dyn Display)]) -> String {
        self.text_with_keys(key, self.help_keys(), args)
    }

    /// What `label` says to the player: a key's text filled in by
    /// [`text_with`](Self::text_with), or the text as it is. What a skin
    /// calls on the labels of the view it paints.
    pub fn label(&self, label: &crate::text::Label) -> String {
        match label {
            crate::text::Label::Key { key, args } => {
                let args: Vec<(&str, &dyn Display)> = args
                    .iter()
                    .map(|(name, value)| (name.as_str(), value as &dyn Display))
                    .collect();
                self.text_with(key, &args)
            }
            crate::text::Label::Text(text) => text.clone(),
        }
    }

    /// [`text_with`](Self::text_with), naming the keys `keys` has rather
    /// than the active ones: for a screen steered by the keys it was
    /// opened with while the player changes them.
    pub fn text_with_keys(
        &self,
        key: &str,
        keys: HelpKeys<'_, G::Action>,
        args: &[(&str, &dyn Display)],
    ) -> String {
        crate::text::fill(self.text(key), args, |name| self.game.key_text(name, keys))
    }

    /// The language text is shown in: [`lang`](Self::lang), but English
    /// for the game's debug language without
    /// [`debug_tools`](Self::debug_tools).
    pub fn effective_lang(&self) -> &LangCode {
        let debug = self.game.debug_lang();
        if debug == Some(self.lang.as_str()) && !self.debug_tools {
            return &self.english;
        }
        &self.lang
    }

    /// What help text names keys from: the active bindings on the device
    /// the player pressed last, and the words of a key's name in the
    /// player's language.
    pub fn help_keys(&self) -> HelpKeys<'_, G::Action> {
        let not_mapped = self.text(NOT_MAPPED_TEXT);
        let arrows = self.text(ARROWS_TEXT);
        let join = self.text(CHORD_JOIN_TEXT);
        HelpKeys::new(&self.keymap, self.device)
            .worded(not_mapped, arrows)
            .chord_joined(join)
    }

    /// A music cue picked at random from the audio manifest's `pool`
    /// ([`pick_from_pool`]): the n-th pick of this run uses
    /// [`music_seed`](Self::music_seed) mixed with n, so a fight started
    /// again may get another track. `None` if there is no such pool or it
    /// is empty.
    pub fn pick_music(&mut self, pool: &str) -> Option<String> {
        let seed = self.music_seed ^ self.music_picks;
        self.music_picks = self.music_picks.wrapping_add(1);
        pick_from_pool(self.game.audio_manifest(), pool, seed).map(str::to_owned)
    }

    /// Takes the text of the voice manifest the platform read, and keeps
    /// the clips that may be played ([`GameData::voices`]). A manifest the
    /// game can't use leaves no voices and a warning in the log
    /// ([`log`](Self::log)).
    pub fn set_voice_manifest(&mut self, source: &str) {
        self.voices = match self.game.voices(source) {
            Ok(playable) => playable,
            Err(why) => {
                self.log(
                    LogLevel::Warn,
                    LogCategory::Audio,
                    format!("no voices: {why}"),
                );
                Playable::default()
            }
        };
    }

    /// The clip `line` would play now, if voices are on and it has one
    /// that may be played.
    fn voice_for(&self, line: &LineId) -> Option<G::Voice> {
        if !self.voices_on {
            return None;
        }
        self.voices.variant_for(line, self.game.voice_variant())
    }

    /// Whether `line` has a voice clip that would play now: voices are on
    /// and the line has a clip that isn't stale.
    pub fn has_voice(&self, line: &LineId) -> bool {
        self.voice_for(line).is_some()
    }

    /// Says `line`: plays its voice clip (the one for
    /// [`GameData::voice_variant`], where the line has several), stopping
    /// the voice that is playing. Does nothing when voices are off or the
    /// line has no clip that may be played.
    pub fn play_voice(&mut self, line: &LineId) {
        if let Some(variant) = self.voice_for(line) {
            self.audio.play_voice(line, variant);
        }
    }

    /// Stops the voice that is playing, if any.
    pub fn stop_voice(&mut self) {
        self.audio.stop_voice();
    }

    /// Tells the platform the lines a scene is about to say, in script
    /// order, so it loads their clips a few ahead. Lines without a
    /// playable clip are left out; nothing is sent when voices are off or
    /// none has one.
    pub fn preload_voices(&mut self, lines: &[LineId]) {
        let clips: Vec<_> = lines
            .iter()
            .filter_map(|line| Some((line.clone(), self.voice_for(line)?)))
            .collect();
        if !clips.is_empty() {
            self.audio.preload_voices(clips);
        }
    }

    /// [`voice_volume`](Self::voice_volume) as a 0–1 factor.
    pub fn voice_gain(&self) -> f32 {
        f32::from(self.voice_volume.min(MAX_VOICE_VOLUME)) / f32::from(MAX_VOICE_VOLUME)
    }

    /// The layout in use, or `None` if the player hasn't picked one yet.
    pub fn layout(&self) -> Option<Layout> {
        self.layout
    }

    /// Switches to `layout`'s bindings (the player's own for that layout,
    /// else its defaults) for this session, without saving the choice.
    pub fn use_layout(&mut self, layout: Layout) {
        self.layout = Some(layout);
        self.refresh_keymap();
    }

    /// Rebuilds [`keymap`](Self::keymap) from the layout in use (the
    /// layout picker's keys while there is none) and the player's
    /// bindings: also after the game changed its keymap file
    /// ([`GameData::keymap_def`]), such as a console's own buttons.
    pub fn refresh_keymap(&mut self) {
        self.keymap = match self.layout {
            Some(layout) => self.keymap_for(layout),
            None => self
                .player_keys
                .layout_picker_keymap(self.game.keymap_def()),
        };
    }

    /// The keymap `layout` would have: the player's bindings for it, else
    /// its defaults (a layout picker draws every layout with this).
    pub fn keymap_for(&self, layout: Layout) -> Keymap<G::Action> {
        self.player_keys.keymap(self.game.keymap_def(), layout)
    }

    /// The player's key bindings for every layout.
    pub fn player_keys(&self) -> &PlayerKeys<G::Action> {
        &self.player_keys
    }

    /// `layout`'s current bindings, for a key bindings screen to edit and
    /// hand back to [`set_layout_bindings`](Self::set_layout_bindings).
    pub fn layout_bindings(&self, layout: Layout) -> LayoutBindings<G::Action> {
        self.player_keys.bindings(self.game.keymap_def(), layout)
    }

    /// Replaces `layout`'s bindings and saves every layout's under
    /// [`KEYBINDINGS_KEY`]. If `layout` is in use, its keys work from the
    /// next key press (`Game` hands the new keymap to its input at once).
    /// The change applies even if saving fails (it is then lost on quit).
    pub fn set_layout_bindings(
        &mut self,
        layout: Layout,
        bindings: LayoutBindings<G::Action>,
    ) -> Result<(), StorageError> {
        self.player_keys
            .set(self.game.keymap_def(), layout, bindings);
        if self.layout == Some(layout) {
            self.use_layout(layout);
        }
        self.storage
            .write(KEYBINDINGS_KEY, &self.player_keys.to_ron())
    }

    /// The player's controller buttons (the defaults until they change
    /// them), for a key bindings screen to edit and hand back to
    /// [`set_pad_bindings`](Self::set_pad_bindings). The same in every
    /// layout.
    pub fn pad_bindings(&self) -> PadBindings<G::Action> {
        self.player_keys.pad_bindings(self.game.keymap_def())
    }

    /// Replaces the controller buttons and saves them under
    /// [`KEYBINDINGS_KEY`]. They work from the next press, whatever the
    /// layout (and before one is chosen). The change applies even if
    /// saving fails (it is then lost on quit).
    pub fn set_pad_bindings(
        &mut self,
        bindings: PadBindings<G::Action>,
    ) -> Result<(), StorageError> {
        self.player_keys.set_pad(self.game.keymap_def(), bindings);
        self.refresh_keymap();
        self.storage
            .write(KEYBINDINGS_KEY, &self.player_keys.to_ron())
    }

    /// Loads the player's key bindings from `storage` (repairing what it
    /// must, with a warning per fix) and re-applies the layout in use.
    fn load_player_keys(&mut self) {
        let (keys, warnings) = match self.storage.read(KEYBINDINGS_KEY) {
            Ok(Some(text)) => PlayerKeys::from_ron(&text, self.game.keymap_def()),
            Ok(None) => (PlayerKeys::default(), Vec::new()),
            Err(e) => (
                PlayerKeys::default(),
                // check-text: not player text (a warning for the log)
                vec![format!("can't be read, using the default keys: {e}")],
            ),
        };
        self.player_keys = keys;
        for warning in warnings {
            let (level, category) = (LogLevel::Warn, LogCategory::Input);
            self.log(level, category, format!("{KEYBINDINGS_KEY}: {warning}"));
        }
        self.refresh_keymap();
    }

    /// Logs `text` at `level` about `category`: kept, with frame 0, until
    /// the platform takes it ([`take_log`](Self::take_log)) and stamps it
    /// with its frame. Past [`LOG_LINES`] waiting, the oldest is dropped
    /// and counted. English, never translated: no player reads it.
    pub fn log(&mut self, level: LogLevel, category: LogCategory, text: impl Into<String>) {
        self.log_lines.push(LogLine {
            frame: 0,
            level,
            category,
            text: text.into(),
        });
    }

    /// Takes the lines waiting, oldest first, and how many were dropped
    /// before they were taken: for the platform's `App`.
    pub fn take_log(&mut self) -> (Vec<LogLine>, u32) {
        self.log_lines.take()
    }

    /// Takes every line waiting, and returns the text of the warnings and
    /// errors among them (e.g. repaired key bindings): for a front end
    /// without the platform's `App`.
    pub fn take_warnings(&mut self) -> Vec<String> {
        let (lines, _) = self.take_log();
        lines
            .into_iter()
            .filter(|line| line.level <= LogLevel::Warn)
            .map(|line| line.text)
            .collect()
    }

    /// Builder form of [`use_layout`](Self::use_layout).
    #[must_use]
    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.use_layout(layout);
        self
    }

    /// The player picked `layout`: switches to it (its own keys if the
    /// player changed them, else its defaults) and saves it in the
    /// settings. The switch happens even if saving fails (the player is
    /// then asked again next launch).
    pub fn choose_layout(&mut self, layout: Layout) -> Result<(), StorageError> {
        self.use_layout(layout);
        self.settings.layout = Some(layout);
        // Only the layout is saved: a change an options screen hasn't
        // kept yet stays unsaved.
        let mut stored = self.stored_settings.clone();
        stored.layout = Some(layout);
        self.write_settings(stored)
    }

    /// The layout saved by an earlier [`choose_layout`](Self::choose_layout):
    /// the saved settings', else the one under the game's
    /// [old key](GameData::old_layout_key). `None` if nothing is saved, or
    /// if the saved value can't be read or isn't a layout the keymap file
    /// lists (the player is simply asked again).
    pub fn saved_layout(&self) -> Option<Layout> {
        let read = |key| self.storage.read(key).ok().flatten();
        let known = |l: &Layout| self.game.keymap_def().has_layout(*l);
        let in_settings = read(SETTINGS_KEY)
            .and_then(|text| Settings::<G::Settings>::from_ron(&text).ok())
            .and_then(|s| s.layout())
            .filter(known);
        in_settings.or_else(|| {
            let old = read(self.game.old_layout_key()?)?;
            Layout::from_name(old.trim()).filter(known)
        })
    }

    /// The layout a screen about the keys shows: the one in use, else the
    /// keymap file's first (before a layout is chosen). `None` only for a
    /// keymap without layouts, which no loaded one is.
    pub fn shown_layout(&self) -> Option<Layout> {
        self.layout
            .or_else(|| self.game.keymap_def().first_layout())
    }

    /// The player's settings.
    pub fn settings(&self) -> &Settings<G::Settings> {
        &self.settings
    }

    /// Lets `change` edit the settings and, if it changed anything, saves
    /// them under [`SETTINGS_KEY`]. The change applies even if saving
    /// fails (it is then lost on quit).
    pub fn change_settings(
        &mut self,
        change: impl FnOnce(&mut Settings<G::Settings>),
    ) -> Result<(), StorageError> {
        if !self.preview_settings(change) {
            return Ok(());
        }
        self.write_settings(self.settings.clone())
    }

    /// Lets `change` edit the settings without saving them: the change
    /// applies at once (the look, the sound), and is saved only by
    /// [`keep_settings`](Self::keep_settings) or the next
    /// [`change_settings`](Self::change_settings). Returns whether it
    /// changed anything. The layout changes only with the keys
    /// ([`choose_layout`](Self::choose_layout)).
    pub fn preview_settings(&mut self, change: impl FnOnce(&mut Settings<G::Settings>)) -> bool {
        let before = self.settings.clone();
        change(&mut self.settings);
        self.settings.layout = before.layout;
        if self.settings == before {
            return false;
        }
        self.game.apply_settings(&self.settings, Some(&before));
        if self.settings.theme != before.theme {
            self.apply_theme();
        }
        true
    }

    /// Saves the settings if they differ from what `storage` holds (a
    /// previewed change, [`preview_settings`](Self::preview_settings)).
    pub fn keep_settings(&mut self) -> Result<(), StorageError> {
        if self.settings == self.stored_settings {
            return Ok(());
        }
        self.write_settings(self.settings.clone())
    }

    /// Puts the settings back to `to` (all but the layout, which only the
    /// keys change), as a previewed change thrown away. It saves only if
    /// `storage` holds something else, which is so only if another change
    /// was saved meanwhile (a fullscreen shortcut, a layout picked).
    pub fn restore_settings(&mut self, to: &Settings<G::Settings>) -> Result<(), StorageError> {
        let to = to.clone();
        self.preview_settings(|s| *s = to);
        self.keep_settings()
    }

    /// Builds [`palette`](Self::palette) from the color theme in the
    /// settings. Keeps the palette if the theme has none, or one that
    /// lacks colors.
    fn apply_theme(&mut self) {
        let def = self.game.theme_def(self.settings.theme);
        if let Some(palette) = def.and_then(|d| Palette::new::<G::Colors>(d).ok()) {
            self.palette = palette;
        }
    }

    /// Writes `settings` under [`SETTINGS_KEY`]; once written they are
    /// what `storage` holds.
    fn write_settings(&mut self, settings: Settings<G::Settings>) -> Result<(), StorageError> {
        self.storage.write(SETTINGS_KEY, &settings.to_ron())?;
        self.stored_settings = settings;
        Ok(())
    }

    /// Loads the settings from `storage`: the defaults if none are saved,
    /// or (with a warning) if they can't be read. A layout saved under the
    /// game's old key is taken over ([`saved_layout`](Self::saved_layout)).
    fn load_settings(&mut self) {
        let loaded = match self.storage.read(SETTINGS_KEY) {
            Ok(Some(text)) => Settings::from_ron(&text),
            Ok(None) => Ok(Settings::default()),
            // check-text: not player text (a warning for the log)
            Err(e) => Err(format!("can't be read: {e}")),
        };
        self.settings = loaded.unwrap_or_else(|why| {
            let (level, category) = (LogLevel::Warn, LogCategory::Storage);
            self.log(
                level,
                category,
                format!("{SETTINGS_KEY}: {why}, using the defaults"),
            );
            Settings::default()
        });
        self.settings.layout = self.saved_layout();
        self.stored_settings = self.settings.clone();
        self.game.apply_settings(&self.settings, None);
        self.apply_theme();
    }

    /// Replaces the storage backend (tests keep the
    /// [`MemoryStorage`](crate::storage::MemoryStorage); the platform
    /// installs its own), loads the profiles from it ([`profiles::load`]:
    /// the last one used, the first made on a first launch, a player's
    /// keys from before profiles moved into it) and that profile's key
    /// bindings and settings.
    #[must_use]
    pub fn with_storage(mut self, mut storage: Box<dyn Storage>) -> Self {
        let old_keys = self.game.keys_before_profiles();
        let lang = self.game.lang();
        let (profiles, warnings) =
            profiles::load(&mut *storage, &old_keys, |n| numbered_profile(lang, n));
        for warning in warnings {
            self.log(LogLevel::Warn, LogCategory::Storage, warning);
        }
        self.storage = ProfileStore::new(storage, profiles.current());
        self.profiles = profiles;
        self.load_player_keys();
        self.load_settings();
        self
    }

    /// The player profiles and the one in use.
    pub fn profiles(&self) -> &Profiles {
        &self.profiles
    }

    /// Makes `id` the profile in use: its settings, key bindings and layout
    /// replace these (`None` if it has none yet: the layout picker's keys),
    /// and whatever else it keeps is its own from the next read
    /// ([`storage`](Self::storage)). It is the one the next launch opens
    /// on. The switch happens even if the list can't be saved (the next
    /// launch then opens on the profile used before).
    pub fn switch_profile(&mut self, id: ProfileId) -> Result<(), ProfileError> {
        self.profiles.set_current(id)?;
        self.load_profile();
        self.profiles
            .save(self.storage.computer_mut())
            .map_err(ProfileError::Storage)
    }

    /// Loads everything the profile in use holds.
    fn load_profile(&mut self) {
        self.storage.set_profile(self.profiles.current());
        self.load_player_keys();
        self.load_settings();
        self.layout = self.settings.layout();
        self.refresh_keymap();
    }

    /// Adds a profile called `name`, starting from the default options and
    /// keys, and returns its id; the one in use stays. Refused (and
    /// nothing changes) for a name that's empty, too long or taken, past
    /// [`MAX_PROFILES`](profiles::MAX_PROFILES), or if the list can't be
    /// saved.
    pub fn add_profile(&mut self, name: &str) -> Result<ProfileId, ProfileError> {
        let mut changed = self.profiles.clone();
        let id = changed.add(name)?;
        self.keep_profiles(changed)?;
        Ok(id)
    }

    /// Renames the profile `id`. Refused (and nothing changes) as
    /// [`add_profile`](Self::add_profile) is.
    pub fn rename_profile(&mut self, id: ProfileId, name: &str) -> Result<(), ProfileError> {
        let mut changed = self.profiles.clone();
        changed.rename(id, name)?;
        self.keep_profiles(changed)
    }

    /// Deletes the profile `id` and every key it holds, and no other
    /// profile's. If it was in use, the first profile left is
    /// ([`switch_profile`](Self::switch_profile)). Refused (and nothing
    /// changes) for the only profile, or if the list can't be saved; once
    /// the list is saved, an error deleting its keys is returned with the
    /// profile gone (they are never read again).
    pub fn delete_profile(&mut self, id: ProfileId) -> Result<(), ProfileError> {
        let mut changed = self.profiles.clone();
        changed.remove(id)?;
        let was_current = self.profiles.current() == id;
        self.keep_profiles(changed)?;
        if was_current {
            self.load_profile();
        }
        profiles::delete_keys(self.storage.computer_mut(), id).map_err(ProfileError::Storage)
    }

    /// Saves `changed` and, once it is saved, takes it as the list.
    fn keep_profiles(&mut self, changed: Profiles) -> Result<(), ProfileError> {
        changed
            .save(self.storage.computer_mut())
            .map_err(ProfileError::Storage)?;
        self.profiles = changed;
        Ok(())
    }
}

/// The name of the profile numbered `n` when the player hasn't named it
/// ([`PROFILE_NUMBERED_TEXT`], in English).
fn numbered_profile(lang: &Lang, n: u32) -> String {
    let text = lang.text(&LangCode::english(), PROFILE_NUMBERED_TEXT);
    crate::text::fill_values(text, &[("n", &n)])
}

#[cfg(test)]
mod tests;
