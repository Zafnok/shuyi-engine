//! The player's settings: the engine's (the text speed, fullscreen, the
//! volumes, the frame cap) and the game's own, the map's look; saved as one
//! RON record.

use serde::{Deserialize, Serialize};
use shuyi::ui::frame_cap::FrameCap;
use shuyi::ui::settings::{GameSettings, Settings, TextSpeed};

/// How the stage is drawn: which of the engine's skins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapLook {
    /// The glyph skin: two cells a tile.
    #[default]
    Glyphs,
    /// The sprite skin, with the tiles of `tiles.png`.
    Tiles,
}

impl MapLook {
    /// Every look, in the options' order, with the text key of its name.
    pub const ALL: [(MapLook, &'static str); 2] = [
        (MapLook::Glyphs, "look.glyphs"),
        (MapLook::Tiles, "look.tiles"),
    ];

    /// Its place in [`ALL`](Self::ALL).
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&(l, _)| l == self).unwrap_or(0)
    }
}

/// The game's own settings.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prefs {
    /// How the stage is drawn.
    pub look: MapLook,
}

/// The saved form of the settings.
#[derive(Serialize, Deserialize)]
#[serde(default, rename = "Settings")]
struct File {
    look: MapLook,
    text_speed: TextSpeed,
    fullscreen: bool,
    music_volume: u8,
    sound_volume: u8,
    frame_cap: FrameCap,
}

impl Default for File {
    fn default() -> Self {
        Self::of(&Settings::default())
    }
}

impl File {
    fn of(s: &Settings<Prefs>) -> Self {
        Self {
            look: s.game.look,
            text_speed: s.text_speed,
            fullscreen: s.fullscreen,
            music_volume: s.music_volume,
            sound_volume: s.sound_volume,
            frame_cap: s.frame_cap,
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
        s.game.look = file.look;
        s.text_speed = file.text_speed;
        s.fullscreen = file.fullscreen;
        s.music_volume = file.music_volume;
        s.sound_volume = file.sound_volume;
        s.frame_cap = file.frame_cap;
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_read_back_what_was_saved() {
        let mut s = Settings::<Prefs>::default();
        s.game.look = MapLook::Tiles;
        s.text_speed = TextSpeed::Fast;
        s.fullscreen = true;
        s.music_volume = 3;
        s.sound_volume = 7;
        s.frame_cap = FrameCap::Off;
        let saved = Prefs::to_ron(&s);
        assert_eq!(Prefs::from_ron(&saved), Ok(s));
        assert_eq!(
            Prefs::from_ron("Settings()"),
            Ok(Settings::<Prefs>::default())
        );
        assert!(Prefs::from_ron("(look: Sideways)").is_err());
        // A save from before the frame cap reads the default.
        let old = Prefs::from_ron("Settings(look: Tiles)").map(|s| s.frame_cap);
        assert_eq!(old, Ok(FrameCap::Fps(60)));
    }

    #[test]
    fn each_look_has_its_place_and_name() {
        assert_eq!(MapLook::Glyphs.index(), 0);
        assert_eq!(MapLook::Tiles.index(), 1);
        assert_eq!(MapLook::default(), MapLook::Glyphs);
    }
}
