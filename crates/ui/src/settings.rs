//! The player's settings: the engine's (how fast dialogue text is
//! revealed, whether the game fills the screen, the color theme, the
//! volume of each sound group ([`Volumes`]), the frame cap and the layout
//! picked) and the
//! game's own
//! ([`Settings::game`], a [`GameSettings`]). [`Ctx`](crate::Ctx) loads
//! them with its storage, from one record under [`SETTINGS_KEY`] in the
//! profile in use, whose form the game decides
//! ([`GameSettings::to_ron`]); it saves them when they change
//! ([`Ctx::change_settings`](crate::Ctx::change_settings)). Key bindings
//! are saved apart.

use crate::prelude::*;
use alloc::collections::BTreeMap;
use core::fmt::Debug;

use shuyi_assets::keymap::Layout;

use crate::audio::groups::{MUSIC, SFX, VOICE};
use crate::frame_cap::{self, FrameCap};

/// [`Storage`](crate::storage::Storage) key under which the settings are
/// saved ([`Settings::to_ron`]).
pub const SETTINGS_KEY: &str = "settings";

/// The loudest volume setting; 0 is silent.
pub const MAX_VOLUME: u8 = 100;

/// The volumes the player set, by sound group
/// ([`groups`](crate::audio::groups)): each a position on the loudness
/// curve, 0 (silent) to [`MAX_VOLUME`]. A group not in it is at its
/// default ([`SoundGroups::default_volume`](crate::audio::SoundGroups::default_volume)),
/// and a group the game no longer has is kept and plays nothing. Change it
/// through [`Ctx::preview_volume`](crate::Ctx::preview_volume), which
/// takes a group set back to its default out of it.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Volumes(BTreeMap<String, u8>);

impl Volumes {
    /// The volume the player set for `group`, if they set one.
    pub fn get(&self, group: &str) -> Option<u8> {
        self.0.get(group).copied()
    }

    /// Sets `group`'s volume (above [`MAX_VOLUME`] counts as it).
    pub fn set(&mut self, group: &str, volume: u8) {
        self.0.insert(group.to_owned(), volume.min(MAX_VOLUME));
    }

    /// Takes `group` out, so it is at its default again: the volume it
    /// had, if any.
    pub fn remove(&mut self, group: &str) -> Option<u8> {
        self.0.remove(group)
    }

    /// Sets `group` where an old linear volume of `percent` (0–100) sounds
    /// the same on the loudness curve
    /// ([`position_of_linear`](crate::audio::loudness::position_of_linear)):
    /// for a saved form that reads the volumes it wrote before the curve.
    pub fn set_from_linear(&mut self, group: &str, percent: u8) {
        self.set(group, crate::audio::loudness::position_of_linear(percent));
    }

    /// Every group the player set, with its volume, by id.
    pub fn iter(&self) -> impl Iterator<Item = (&str, u8)> {
        self.0
            .iter()
            .map(|(group, &volume)| (group.as_str(), volume))
    }

    /// Whether the player set none: every group is at its default.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Keeps only the groups `keep` says to.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&str, u8) -> bool) {
        self.0.retain(|group, volume| keep(group, *volume));
    }
}

/// Reads a volume a saved form wrote before the loudness curve, a bare
/// number, as `Some`: for an `Option<u8>` field that is read and never
/// written, `#[serde(deserialize_with = "…::old_volume", skip_serializing)]`
/// in a record with `#[serde(default)]`, so a save without it reads `None`.
pub fn old_volume<'de, D: serde::Deserializer<'de>>(volume: D) -> Result<Option<u8>, D::Error> {
    <u8 as serde::Deserialize>::deserialize(volume).map(Some)
}

/// Reads the volumes a saved form wrote before the loudness curve, each
/// linear (0–100) and `None` where the save has none, into `volumes`
/// where they sound the same ([`Volumes::set_from_linear`]): `music`,
/// `sfx` and `voice`.
pub fn read_old_volumes(
    volumes: &mut Volumes,
    music: Option<u8>,
    sound: Option<u8>,
    voice: Option<u8>,
) {
    for (group, percent) in [(MUSIC, music), (SFX, sound), (VOICE, voice)] {
        if let Some(percent) = percent {
            volumes.set_from_linear(group, percent);
        }
    }
}

/// How fast dialogue text is revealed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TextSpeed {
    /// Half the normal speed.
    Slow,
    /// The normal speed.
    #[default]
    Normal,
    /// Twice the normal speed.
    Fast,
    /// A whole page at once.
    Instant,
}

impl TextSpeed {
    /// Every speed, slowest first.
    pub const ALL: [Self; 4] = [Self::Slow, Self::Normal, Self::Fast, Self::Instant];

    /// Characters revealed per second (*tunable*); `None` shows a page at
    /// once.
    pub const fn chars_per_s(self) -> Option<f32> {
        match self {
            Self::Slow => Some(30.0),
            Self::Normal => Some(60.0),
            Self::Fast => Some(120.0),
            Self::Instant => None,
        }
    }

    /// The key of the name an options screen shows
    /// ([`Ctx::text`](crate::Ctx::text)).
    pub const fn key(self) -> &'static str {
        match self {
            Self::Slow => "options.value.slow",
            Self::Normal => "options.value.normal",
            Self::Fast => "options.value.fast",
            Self::Instant => "options.value.instant",
        }
    }
}

/// What the game's sound does while its window has no focus.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum BackgroundAudio {
    /// The sound goes on as it was.
    #[default]
    Keep,
    /// The music, the sounds and the voices are silent; the music keeps
    /// its place, and the player's volumes come back with the focus.
    Mute,
}

/// What a game does while its window has no focus
/// ([`GameSettings::BACKGROUND`]): whether it pauses, and what its sound
/// does. A game writes it from [`KEEP_RUNNING`](Self::KEEP_RUNNING):
/// `Background::KEEP_RUNNING.paused().muted()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Background {
    /// Whether the game stops: no update, no steps and no time for its
    /// screens, its last frame shown again.
    pub pause: bool,
    /// What its sound does.
    pub audio: BackgroundAudio,
}

impl Background {
    /// The game runs on with its sound, as with the focus.
    pub const KEEP_RUNNING: Self = Self {
        pause: false,
        audio: BackgroundAudio::Keep,
    };

    /// The same, and the game pauses.
    #[must_use]
    pub const fn paused(self) -> Self {
        Self {
            pause: true,
            ..self
        }
    }

    /// The same, and the game's sound is muted.
    #[must_use]
    pub const fn muted(self) -> Self {
        Self {
            audio: BackgroundAudio::Mute,
            ..self
        }
    }
}

impl Default for Background {
    fn default() -> Self {
        Self::KEEP_RUNNING
    }
}

/// The settings a game adds to the engine's, and the saved form of both:
/// one record, so the game decides its form (and keeps the one it has
/// always written).
pub trait GameSettings: Debug + Clone + PartialEq + Eq + Default + 'static {
    /// The game's color themes, each a palette
    /// ([`GameData::theme_def`](crate::GameData::theme_def)); `()` for a
    /// game with one.
    type Theme: Debug + Copy + PartialEq + Eq + Default + 'static;

    /// The frame cap a new player starts with
    /// ([`Settings::frame_cap`]): 60 frames a second unless the game says
    /// otherwise ([`FrameCap`] says why).
    const DEFAULT_FRAME_CAP: FrameCap = FrameCap::DEFAULT;

    /// The frame caps the options screen's Frame rate row steps through,
    /// in order ([`OptionRow::frame_cap`](crate::screens::options::OptionRow::frame_cap)):
    /// 30, 60, 120, 144, 165 and 240 frames a second, then none, unless the
    /// game says otherwise ([`frame_cap::CHOICES`]).
    const FRAME_CAP_CHOICES: &'static [FrameCap] = &frame_cap::CHOICES;

    /// What the game does while its window has no focus (ADR-0077): it
    /// keeps running with its sound ([`Background::KEEP_RUNNING`]) unless
    /// the game says otherwise.
    const BACKGROUND: Background = Background::KEEP_RUNNING;

    /// The saved form of `settings` (RON).
    fn to_ron(settings: &Settings<Self>) -> String;

    /// Reads the saved form. Text that can't be read gives `Err` with why
    /// (the caller then uses the defaults); what can be repaired is.
    // check-text: not player text (warnings for the log)
    fn from_ron(text: &str) -> Result<Settings<Self>, String>;
}

/// The saved form of the settings of a game with none of its own
/// ([`GameSettings`] for `()`): the engine's, and the layout picked by its
/// name. What isn't there reads as its default. The linear volumes it
/// wrote before the loudness curve (`music_volume`, `sound_volume`,
/// `voice_volume`) are read ([`read_old_volumes`]) and never written.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default, rename = "Settings")]
struct EngineOnly {
    text_speed: TextSpeed,
    fullscreen: bool,
    volumes: Volumes,
    voices: bool,
    frame_cap: FrameCap,
    layout: Option<String>,
    #[serde(skip_serializing, deserialize_with = "old_volume")]
    music_volume: Option<u8>,
    #[serde(skip_serializing, deserialize_with = "old_volume")]
    sound_volume: Option<u8>,
    #[serde(skip_serializing, deserialize_with = "old_volume")]
    voice_volume: Option<u8>,
}

impl Default for EngineOnly {
    fn default() -> Self {
        Self::of(&Settings::default())
    }
}

impl EngineOnly {
    fn of(s: &Settings<()>) -> Self {
        Self {
            text_speed: s.text_speed,
            fullscreen: s.fullscreen,
            volumes: s.volumes.clone(),
            voices: s.voices,
            frame_cap: s.frame_cap,
            layout: s.layout.map(|l| l.name().to_owned()),
            music_volume: None,
            sound_volume: None,
            voice_volume: None,
        }
    }
}

/// No settings of the game's own, and one palette: the engine's settings
/// alone, saved as one record (`Settings(text_speed: Fast, …)`).
impl GameSettings for () {
    type Theme = ();

    fn to_ron(s: &Settings<Self>) -> String {
        ron::to_string(&EngineOnly::of(s)).unwrap_or_default()
    }

    fn from_ron(text: &str) -> Result<Settings<Self>, String> {
        let file: EngineOnly = ron::from_str(text).map_err(|e| format!("unreadable: {e}"))?;
        let mut volumes = file.volumes;
        let old = (file.music_volume, file.sound_volume, file.voice_volume);
        read_old_volumes(&mut volumes, old.0, old.1, old.2);
        Ok(Settings {
            text_speed: file.text_speed,
            fullscreen: file.fullscreen,
            volumes,
            voices: file.voices,
            frame_cap: file.frame_cap,
            layout: file.layout.as_deref().and_then(Layout::from_name),
            ..Settings::default()
        })
    }
}

/// Everything an options screen sets: the engine's settings and the
/// game's. The defaults are a new player's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings<S: GameSettings> {
    /// How fast dialogue text is revealed.
    pub text_speed: TextSpeed,
    /// Whether the game fills the screen.
    pub fullscreen: bool,
    /// The color theme: the palette the game draws with.
    pub theme: S::Theme,
    /// The volume of each sound group the player set; the others are at
    /// their defaults. Read one with [`Ctx::volume`](crate::Ctx::volume).
    pub volumes: Volumes,
    /// Whether voice clips play
    /// ([`Ctx::play_voice`](crate::Ctx::play_voice)). Turning it off stops
    /// the one playing.
    pub voices: bool,
    /// The most frames a second the platform shows: the game's
    /// [`GameSettings::DEFAULT_FRAME_CAP`] until the player picks another.
    /// Every change of it takes the next serial, which the platform reports
    /// once it has applied it; a game that wants to know when asks with
    /// [`Ctx::request_frame_cap`](crate::Ctx::request_frame_cap).
    pub frame_cap: FrameCap,
    /// The layout the player picked; `None` until they have. Changed only
    /// by [`Ctx::choose_layout`](crate::Ctx::choose_layout), which also
    /// switches the keys.
    pub(crate) layout: Option<Layout>,
    /// The game's own settings.
    pub game: S,
}

impl<S: GameSettings> Default for Settings<S> {
    fn default() -> Self {
        Self {
            text_speed: TextSpeed::default(),
            fullscreen: false,
            theme: S::Theme::default(),
            volumes: Volumes::default(),
            voices: true,
            frame_cap: S::DEFAULT_FRAME_CAP,
            layout: None,
            game: S::default(),
        }
    }
}

impl<S: GameSettings> Settings<S> {
    /// The layout the player picked, if they have.
    pub fn layout(&self) -> Option<Layout> {
        self.layout
    }

    /// The same settings with `layout` as the one picked.
    #[must_use]
    pub fn with_layout(mut self, layout: Layout) -> Self {
        self.layout = Some(layout);
        self
    }

    /// The same settings with `other`'s layout picked (or none): to
    /// compare two settings but for the layout.
    #[must_use]
    pub fn with_layout_of(mut self, other: &Self) -> Self {
        self.layout = other.layout;
        self
    }

    /// The defaults, keeping what isn't a preference with a default: the
    /// layout picked (an options screen's "Restore defaults").
    #[must_use]
    pub fn restored(&self) -> Self {
        Self {
            layout: self.layout,
            ..Self::default()
        }
    }

    /// The saved form ([`GameSettings::to_ron`]).
    pub fn to_ron(&self) -> String {
        S::to_ron(self)
    }

    /// Reads the saved form ([`GameSettings::from_ron`]).
    // check-text: not player text (warnings for the log)
    pub fn from_ron(text: &str) -> Result<Self, String> {
        S::from_ron(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::groups::{MUSIC, SFX, VOICE};
    use crate::input::sample::LEFT_HANDED;
    use crate::toy::{Prefs, Theme};

    type Settings = super::Settings<Prefs>;

    #[test]
    fn the_defaults_are_a_new_players() {
        let s = Settings::default();
        assert_eq!(s.text_speed, TextSpeed::Normal);
        assert_eq!(s.text_speed.chars_per_s(), Some(60.0));
        assert!(!s.fullscreen);
        assert_eq!(s.theme, Theme::default());
        assert!(s.volumes.is_empty(), "every group at its default");
        assert!(s.voices);
        assert_eq!(s.frame_cap, FrameCap::Fps(60));
        assert_eq!(s.layout(), None);
        assert_eq!(s.game, Prefs::default());
    }

    #[test]
    fn restoring_keeps_only_the_layout() {
        let mut s = Settings::default().with_layout(LEFT_HANDED);
        s.text_speed = TextSpeed::Fast;
        s.fullscreen = true;
        s.theme = Theme::Night;
        s.volumes.set(MUSIC, 3);
        s.volumes.set(SFX, 4);
        s.voices = false;
        s.volumes.set(VOICE, 5);
        s.frame_cap = FrameCap::Off;
        s.game.speed = 9;
        assert_eq!(s.layout(), Some(LEFT_HANDED));
        assert_eq!(s.restored(), Settings::default().with_layout(LEFT_HANDED));
    }

    #[test]
    fn another_settings_layout_is_taken_with_or_without_one() {
        let mut s = Settings::default();
        s.game.speed = 4;
        let left = Settings::default().with_layout(LEFT_HANDED);
        let taken = s.clone().with_layout_of(&left);
        assert_eq!(taken.layout(), Some(LEFT_HANDED));
        assert_eq!(taken.game.speed, 4);
        assert_eq!(taken.with_layout_of(&Settings::default()), s);
    }

    #[test]
    fn the_saved_form_is_the_games() {
        let mut s = Settings::default().with_layout(LEFT_HANDED);
        s.game.speed = 7;
        s.theme = Theme::Night;
        let text = s.to_ron();
        assert_eq!(text, Prefs::to_ron(&s));
        assert_eq!(Settings::from_ron(&text), Ok(s));
        assert!(Settings::from_ron("nonsense").is_err());
    }

    #[test]
    fn a_game_with_no_settings_of_its_own_saves_the_engines() {
        let mut s = super::Settings::<()>::default().with_layout(LEFT_HANDED);
        s.text_speed = TextSpeed::Instant;
        s.volumes.set(SFX, 12);
        s.voices = false;
        s.volumes.set("ambience", 30);
        s.frame_cap = FrameCap::Fps(144);
        let text = s.to_ron();
        assert!(
            text.contains("volumes:{\"ambience\":30,\"sfx\":12}"),
            "{text}"
        );
        assert!(!text.contains("_volume"), "no old field is written: {text}");
        assert_eq!(super::Settings::from_ron(&text), Ok(s));
        let s = super::Settings::<()>::default();
        assert_eq!(super::Settings::from_ron(&s.to_ron()), Ok(s));
    }

    #[test]
    fn the_blank_templates_saved_settings_read_back_through_no_settings() {
        // What `shuyi new blank`'s `Prefs::to_ron` wrote before its game
        // took `()` for its settings.
        let saved = "(text_speed:Fast,fullscreen:true,music_volume:3,sound_volume:7)";
        let mut s = super::Settings::<()> {
            text_speed: TextSpeed::Fast,
            fullscreen: true,
            ..super::Settings::default()
        };
        s.volumes.set_from_linear(MUSIC, 3);
        s.volumes.set_from_linear(SFX, 7);
        assert_eq!(super::Settings::from_ron(saved), Ok(s));
        assert_eq!(
            super::Settings::from_ron("Settings()"),
            Ok(super::Settings::<()>::default())
        );
        assert!(super::Settings::<()>::from_ron("(fullscreen: maybe)").is_err());
        // A layout no keymap could name is dropped, not an error.
        let odd = super::Settings::<()>::from_ron("(layout: Some(\"two words\"))");
        assert_eq!(odd.map(|s| s.layout()), Ok(None));
    }

    /// A game with a default frame cap of its own.
    #[derive(Debug, Clone, Default, PartialEq, Eq)]
    struct Fast;

    impl GameSettings for Fast {
        type Theme = ();
        const DEFAULT_FRAME_CAP: FrameCap = FrameCap::Fps(120);
        const FRAME_CAP_CHOICES: &'static [FrameCap] = &[FrameCap::Fps(120), FrameCap::Off];

        fn to_ron(_: &super::Settings<Self>) -> String {
            String::new()
        }

        fn from_ron(_: &str) -> Result<super::Settings<Self>, String> {
            Ok(super::Settings::default())
        }
    }

    #[test]
    fn the_frame_cap_is_the_games_default_until_the_player_picks() {
        assert_eq!(
            super::Settings::<()>::default().frame_cap,
            FrameCap::Fps(60)
        );
        assert_eq!(<() as GameSettings>::FRAME_CAP_CHOICES, &frame_cap::CHOICES);
        let mut s = super::Settings::<Fast>::default();
        assert_eq!(s.frame_cap, FrameCap::Fps(120));
        assert_eq!(Fast::FRAME_CAP_CHOICES.len(), 2);
        s.frame_cap = FrameCap::Off;
        assert_eq!(s.restored().frame_cap, FrameCap::Fps(120));
    }

    #[test]
    fn the_background_keeps_running_until_the_game_says() {
        let keep = Background::KEEP_RUNNING;
        assert_eq!((keep.pause, keep.audio), (false, BackgroundAudio::Keep));
        assert_eq!(Background::default(), keep);
        assert_eq!(<() as GameSettings>::BACKGROUND, keep);
        assert_eq!(Prefs::BACKGROUND, keep);
        let paused = keep.paused();
        assert_eq!((paused.pause, paused.audio), (true, BackgroundAudio::Keep));
        let muted = keep.muted();
        assert_eq!((muted.pause, muted.audio), (false, BackgroundAudio::Mute));
        let both = keep.paused().muted();
        assert_eq!(both, keep.muted().paused());
        assert_eq!((both.pause, both.audio), (true, BackgroundAudio::Mute));
    }

    #[test]
    fn a_toy_save_without_a_frame_cap_reads_the_default() {
        let read = |text| Settings::from_ron(text).map(|s| s.frame_cap);
        assert_eq!(read("Toy(speed: 1)"), Ok(FrameCap::Fps(60)));
        assert_eq!(read("Toy(frame_cap: Off)"), Ok(FrameCap::Off));
        let s = Settings {
            frame_cap: FrameCap::Fps(240),
            ..Settings::default()
        };
        assert!(s.to_ron().contains("frame_cap:Fps(240)"), "{}", s.to_ron());
        assert_eq!(Settings::from_ron(&s.to_ron()), Ok(s));
    }

    #[test]
    fn an_old_toy_save_reads_its_linear_volumes_where_they_sound_the_same() {
        let old = "Toy(speed: 2, music_volume: 35, sound_volume: 80)";
        let s = Settings::from_ron(old).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            (s.volumes.get(MUSIC), s.volumes.get(SFX)),
            (Some(85), Some(97))
        );
        assert_eq!(s.game.speed, 2);
        let text = s.to_ron();
        assert!(text.contains("volumes:{\"music\":85,\"sfx\":97}"), "{text}");
        assert!(!text.contains("_volume"), "no old field is written: {text}");
        assert_eq!(Settings::from_ron(&text), Ok(s));
    }

    #[test]
    fn engine_settings_saved_without_a_frame_cap_read_the_default() {
        let read = |text| super::Settings::<()>::from_ron(text).map(|s| s.frame_cap);
        assert_eq!(read("(text_speed:Fast)"), Ok(FrameCap::Fps(60)));
        assert_eq!(read("(frame_cap:Off)"), Ok(FrameCap::Off));
        assert_eq!(read("(frame_cap:Fps(30))"), Ok(FrameCap::Fps(30)));
    }

    #[test]
    fn volumes_are_set_taken_back_to_the_default_and_restored() {
        let mut s = Settings::default();
        s.volumes.set(MUSIC, 40);
        assert!(!s.volumes.is_empty());
        s.volumes.set(SFX, 200);
        assert_eq!(
            (s.volumes.get(MUSIC), s.volumes.get(SFX)),
            (Some(40), Some(100))
        );
        assert_eq!(s.volumes.get(VOICE), None);
        assert_eq!(s.volumes.remove(MUSIC), Some(40));
        assert_eq!(s.volumes.remove(MUSIC), None);
        let set: Vec<_> = s.volumes.iter().collect();
        assert_eq!(set, [(SFX, 100)]);
        assert!(s.restored().volumes.is_empty());
        s.volumes.set_from_linear(VOICE, 80);
        assert_eq!(s.volumes.get(VOICE), Some(97));
    }

    #[test]
    fn an_old_engine_save_reads_its_linear_volumes_where_they_sound_the_same() {
        let old = "(music_volume: 35, sound_volume: 80, voice_volume: 100)";
        let s = super::Settings::<()>::from_ron(old).unwrap_or_else(|e| panic!("{e}"));
        let read = [MUSIC, SFX, VOICE].map(|g| s.volumes.get(g));
        assert_eq!(read, [Some(85), Some(97), Some(100)]);
        let text = s.to_ron();
        assert!(text.contains("volumes:{"), "{text}");
        assert!(
            !text.contains("music_volume") && !text.contains("sound_volume"),
            "{text}"
        );
        assert_eq!(super::Settings::from_ron(&text), Ok(s));
    }

    #[test]
    fn text_speeds_double_each_step_and_instant_has_none() {
        let speeds: Vec<_> = TextSpeed::ALL.iter().map(|s| s.chars_per_s()).collect();
        assert_eq!(speeds, [Some(30.0), Some(60.0), Some(120.0), None]);
        let keys: Vec<_> = TextSpeed::ALL.iter().map(|s| s.key()).collect();
        let value = |name: &str| format!("options.value.{name}");
        assert_eq!(keys, ["slow", "normal", "fast", "instant"].map(value));
    }
}
