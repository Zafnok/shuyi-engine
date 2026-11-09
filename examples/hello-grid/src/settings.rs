//! The player's settings: the engine's (the text speed, fullscreen, the
//! sound groups' volumes, the frame cap) and the game's own, the map's
//! look; saved as one RON record.

#[allow(unused_imports)]
use crate::prelude::*;

use serde::{Deserialize, Serialize};
use shuyi::ui::frame_cap::FrameCap;
use shuyi::ui::settings::{GameSettings, Settings, TextSpeed, Volumes, read_old_volumes};

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

/// The saved form of the settings. The linear music and sound volumes it
/// wrote before the loudness curve are read, and never written.
#[derive(Serialize, Deserialize)]
#[serde(default, rename = "Settings")]
struct File {
    look: MapLook,
    text_speed: TextSpeed,
    fullscreen: bool,
    volumes: Volumes,
    frame_cap: FrameCap,
    #[serde(skip_serializing, deserialize_with = "shuyi::ui::settings::old_volume")]
    music_volume: Option<u8>,
    #[serde(skip_serializing, deserialize_with = "shuyi::ui::settings::old_volume")]
    sound_volume: Option<u8>,
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
            volumes: s.volumes.clone(),
            frame_cap: s.frame_cap,
            music_volume: None,
            sound_volume: None,
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
        s.volumes = file.volumes;
        read_old_volumes(&mut s.volumes, file.music_volume, file.sound_volume, None);
        s.frame_cap = file.frame_cap;
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shuyi::ui::audio::groups::{MUSIC, SFX};

    #[test]
    fn the_settings_read_back_what_was_saved() {
        let mut s = Settings::<Prefs>::default();
        s.game.look = MapLook::Tiles;
        s.text_speed = TextSpeed::Fast;
        s.fullscreen = true;
        s.volumes.set(MUSIC, 3);
        s.volumes.set(SFX, 7);
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

    /// A save from before the loudness curve: its linear volumes are read
    /// where they sound the same, and a new save writes no old field.
    #[test]
    fn an_old_save_reads_its_volumes_where_they_sound_the_same() {
        let old = Prefs::from_ron("Settings(look: Tiles, music_volume: 35, sound_volume: 80)");
        let s = old.unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            (s.volumes.get(MUSIC), s.volumes.get(SFX)),
            (Some(85), Some(97))
        );
        assert_eq!(s.game.look, MapLook::Tiles);
        let saved = Prefs::to_ron(&s);
        assert!(saved.contains("volumes:{"), "{saved}");
        assert!(!saved.contains("_volume"), "{saved}");
        assert_eq!(Prefs::from_ron(&saved), Ok(s));
    }

    #[test]
    fn each_look_has_its_place_and_name() {
        assert_eq!(MapLook::Glyphs.index(), 0);
        assert_eq!(MapLook::Tiles.index(), 1);
        assert_eq!(MapLook::default(), MapLook::Glyphs);
    }
}
