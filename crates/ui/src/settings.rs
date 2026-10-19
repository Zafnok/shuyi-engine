//! The player's settings: the engine's (how fast dialogue text is
//! revealed, whether the game fills the screen, the color theme, the
//! music and sound volumes and the layout picked) and the game's own
//! ([`Settings::game`], a [`GameSettings`]). [`Ctx`](crate::Ctx) loads
//! them with its storage, from one record under [`SETTINGS_KEY`] in the
//! profile in use, whose form the game decides
//! ([`GameSettings::to_ron`]); it saves them when they change
//! ([`Ctx::change_settings`](crate::Ctx::change_settings)). Key bindings
//! are saved apart.

use crate::prelude::*;
use core::fmt::Debug;

use shuyi_assets::keymap::Layout;

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

    /// The saved form of `settings` (RON).
    fn to_ron(settings: &Settings<Self>) -> String;

    /// Reads the saved form. Text that can't be read gives `Err` with why
    /// (the caller then uses the defaults); what can be repaired is.
    // check-text: not player text (warnings for the log)
    fn from_ron(text: &str) -> Result<Settings<Self>, String>;
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
