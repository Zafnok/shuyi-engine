//! The player's settings: the engine's (the text speed, fullscreen, the
//! volumes), saved as one RON record. A setting of the game's own goes in
//! [`Prefs`] and in [`File`].

use serde::{Deserialize, Serialize};
use shuyi::ui::settings::{GameSettings, Settings, TextSpeed};

/// The game's own settings: none yet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prefs;

/// The saved form of the settings.
#[derive(Serialize, Deserialize)]
#[serde(default, rename = "Settings")]
struct File {
    text_speed: TextSpeed,
    fullscreen: bool,
    music_volume: u8,
    sound_volume: u8,
}

impl Default for File {
    fn default() -> Self {
        Self::of(&Settings::default())
    }
}

impl File {
    fn of(s: &Settings<Prefs>) -> Self {
        Self {
            text_speed: s.text_speed,
            fullscreen: s.fullscreen,
            music_volume: s.music_volume,
            sound_volume: s.sound_volume,
        }
    }
}

impl GameSettings for Prefs {
    /// One palette: no themes.
    type Theme = ();

    fn to_ron(s: &Settings<Self>) -> String {
        ron::to_string(&File::of(s)).unwrap_or_default()
    }

    fn from_ron(text: &str) -> Result<Settings<Self>, String> {
        let file: File = ron::from_str(text).map_err(|e| format!("unreadable: {e}"))?;
        let mut s = Settings::<Self>::default();
        s.text_speed = file.text_speed;
        s.fullscreen = file.fullscreen;
        s.music_volume = file.music_volume;
        s.sound_volume = file.sound_volume;
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_read_back_what_was_saved() {
        let mut s = Settings::<Prefs>::default();
        s.text_speed = TextSpeed::Fast;
        s.fullscreen = true;
        s.music_volume = 3;
        s.sound_volume = 7;
        let saved = Prefs::to_ron(&s);
        assert_eq!(Prefs::from_ron(&saved), Ok(s));
        assert_eq!(
            Prefs::from_ron("Settings()"),
            Ok(Settings::<Prefs>::default())
        );
        assert!(Prefs::from_ron("(fullscreen: maybe)").is_err());
    }
}
