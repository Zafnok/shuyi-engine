//! The player's settings: the engine's (how fast dialogue text is
//! revealed, whether the game fills the screen, the color theme, the
//! music and sound volumes, the frame cap and the layout picked) and the
//! game's own
//! ([`Settings::game`], a [`GameSettings`]). [`Ctx`](crate::Ctx) loads
//! them with its storage, from one record under [`SETTINGS_KEY`] in the
//! profile in use, whose form the game decides
//! ([`GameSettings::to_ron`]); it saves them when they change
//! ([`Ctx::change_settings`](crate::Ctx::change_settings)). Key bindings
//! are saved apart.

use crate::prelude::*;
use core::fmt::Debug;

use shuyi_assets::keymap::Layout;

use crate::frame_cap::{self, FrameCap};

/// [`Storage`](crate::storage::Storage) key under which the settings are
/// saved ([`Settings::to_ron`]).
pub const SETTINGS_KEY: &str = "settings";

/// The loudest volume setting; 0 is silent.
pub const MAX_VOLUME: u8 = 100;

/// The volume settings' default. *Tunable.*
pub const DEFAULT_VOLUME: u8 = 80;

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

    /// The saved form of `settings` (RON).
    fn to_ron(settings: &Settings<Self>) -> String;

    /// Reads the saved form. Text that can't be read gives `Err` with why
    /// (the caller then uses the defaults); what can be repaired is.
    // check-text: not player text (warnings for the log)
    fn from_ron(text: &str) -> Result<Settings<Self>, String>;
}

/// The saved form of the settings of a game with none of its own
/// ([`GameSettings`] for `()`): the engine's, and the layout picked by its
/// name. What isn't there reads as its default.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default, rename = "Settings")]
struct EngineOnly {
    text_speed: TextSpeed,
    fullscreen: bool,
    music_volume: u8,
    sound_volume: u8,
    frame_cap: FrameCap,
    layout: Option<String>,
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
            music_volume: s.music_volume,
            sound_volume: s.sound_volume,
            frame_cap: s.frame_cap,
            layout: s.layout.map(|l| l.name().to_owned()),
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
        Ok(Settings {
            text_speed: file.text_speed,
            fullscreen: file.fullscreen,
            music_volume: file.music_volume,
            sound_volume: file.sound_volume,
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
    /// Music volume, 0 (silent) to [`MAX_VOLUME`].
    pub music_volume: u8,
    /// Sound volume, 0 (silent) to [`MAX_VOLUME`].
    pub sound_volume: u8,
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
            music_volume: DEFAULT_VOLUME,
            sound_volume: DEFAULT_VOLUME,
            frame_cap: S::DEFAULT_FRAME_CAP,
            layout: None,
            game: S::default(),
        }
    }
}

/// A volume setting as the multiplier the platform applies to every cue's
/// own volume: 0 at 0, 1 at [`MAX_VOLUME`].
fn volume_factor(volume: u8) -> f32 {
    f32::from(volume.min(MAX_VOLUME)) / f32::from(MAX_VOLUME)
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

    /// The multiplier on the music's volume, 0–1.
    pub fn music_factor(&self) -> f32 {
        volume_factor(self.music_volume)
    }

    /// The multiplier on every sound's volume, 0–1.
    pub fn sound_factor(&self) -> f32 {
        volume_factor(self.sound_volume)
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
        assert_eq!((s.music_volume, s.sound_volume), (80, 80));
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
        s.music_volume = 3;
        s.sound_volume = 4;
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
        s.sound_volume = 12;
        s.frame_cap = FrameCap::Fps(144);
        assert_eq!(super::Settings::from_ron(&s.to_ron()), Ok(s));
        let s = super::Settings::<()>::default();
        assert_eq!(super::Settings::from_ron(&s.to_ron()), Ok(s));
    }

    #[test]
    fn the_blank_templates_saved_settings_read_back_through_no_settings() {
        // What `shuyi new blank`'s `Prefs::to_ron` wrote before its game
        // took `()` for its settings.
        let saved = "(text_speed:Fast,fullscreen:true,music_volume:3,sound_volume:7)";
        let s = super::Settings::<()> {
            text_speed: TextSpeed::Fast,
            fullscreen: true,
            music_volume: 3,
            sound_volume: 7,
            ..super::Settings::default()
        };
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
    fn engine_settings_saved_without_a_frame_cap_read_the_default() {
        let read = |text| super::Settings::<()>::from_ron(text).map(|s| s.frame_cap);
        assert_eq!(read("(text_speed:Fast)"), Ok(FrameCap::Fps(60)));
        assert_eq!(read("(frame_cap:Off)"), Ok(FrameCap::Off));
        assert_eq!(read("(frame_cap:Fps(30))"), Ok(FrameCap::Fps(30)));
    }

    #[test]
    fn volumes_are_factors_from_silent_to_full() {
        let mut s = Settings::default();
        assert!((s.music_factor() - 0.8).abs() < 1e-6);
        assert!((s.sound_factor() - 0.8).abs() < 1e-6);
        s.music_volume = 0;
        s.sound_volume = 100;
        assert!(s.music_factor().abs() < f32::EPSILON);
        s.music_volume = 37;
        assert!((s.music_factor() - 0.37).abs() < 1e-6);
        s.music_volume = 0;
        assert!((s.sound_factor() - 1.0).abs() < f32::EPSILON);
        // Out of range (set in code) is still at most full volume.
        s.sound_volume = 200;
        assert!((s.sound_factor() - 1.0).abs() < f32::EPSILON);
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
