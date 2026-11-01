//! The sound device on desktop and the web: quad-snd under the shared
//! player's rules ([`shuyi_platform::audio::player`], ADR-0063), and where
//! the folders that ship beside the game are.
//!
//! Sounds are embedded and decoded once at start-up. Music is loaded from
//! the game's music folder when a track is about to play and freed when it
//! stops, so at most two tracks (the one fading out and the next) are in
//! memory, and voice clips are loaded the same way from the voice folder.
//!
//! quad-snd can't pause (ADR-0056), so [`device::Macroquad`] keeps the
//! backend's defaults there, and the player mutes what it holds.

#[cfg(not(target_arch = "wasm32"))]
mod native_music;
pub(crate) mod voice;

use std::path::{Path, PathBuf};

/// Where a `folder` that ships beside the game is: next to the executable
/// when it's there (a shipped build), or in a macOS bundle's
/// `Contents/Resources` when the executable is in its `Contents/MacOS`, else
/// relative to the working directory (`cargo run` from the repo root, or the
/// web build, which fetches `music/…` from beside the page).
pub(crate) fn folder_beside(
    folder: &str,
    exe: Option<&Path>,
    is_dir: impl Fn(&Path) -> bool,
) -> String {
    let exe_dir = exe.and_then(Path::parent);
    let in_bundle = exe_dir
        .filter(|dir| dir.ends_with("Contents/MacOS"))
        .and_then(Path::parent)
        .map(|contents| contents.join("Resources"));
    exe_dir
        .into_iter()
        .chain(in_bundle.as_deref())
        .map(|dir| dir.join(folder))
        .find(|dir| is_dir(dir))
        .as_deref()
        .and_then(Path::to_str)
        .map_or_else(|| folder.to_owned(), |dir| dir.replace('\\', "/"))
}

/// Where a `folder` that ships beside the game is on this platform: beside
/// the executable when it's there ([`folder_beside`]); on the web, beside
/// the page.
pub(crate) fn platform_folder(folder: &str) -> String {
    if cfg!(target_arch = "wasm32") {
        folder.to_owned()
    } else {
        let exe: Option<PathBuf> = std::env::current_exe().ok();
        folder_beside(folder, exe.as_deref(), Path::is_dir)
    }
}

/// The real backend: quad-snd, which macroquad's audio is. Sound effects
/// load into a quad-snd context of the player's own, where each playback
/// stops and changes its volume alone. On native, music is decoded on a
/// worker thread into contexts of its own ([`native_music`], game
/// ADR-0028); on the web the browser decodes it asynchronously, so it goes
/// through macroquad's.
pub(crate) mod device {
    #[cfg(target_arch = "wasm32")]
    use macroquad::audio::{
        PlaySoundParams, Sound, load_sound_from_bytes, play_sound, set_sound_volume, stop_sound,
    };
    #[cfg(target_arch = "wasm32")]
    use macroquad::experimental::coroutines::{Coroutine, start_coroutine};
    #[cfg(target_arch = "wasm32")]
    use macroquad::file::load_file;
    #[cfg(not(target_arch = "wasm32"))]
    use quad_snd::PlaySoundParams;
    use quad_snd::{AudioContext, Playback};

    #[cfg(not(target_arch = "wasm32"))]
    use super::native_music::{Contexts, Music, Pending};

    /// Plays through quad-snd. Only usable inside macroquad's main loop.
    #[derive(Default)]
    pub(crate) struct Macroquad {
        /// Spare music contexts.
        #[cfg(not(target_arch = "wasm32"))]
        contexts: Contexts,
        /// The sound effects' context, opened with the first sound.
        effects: Option<AudioContext>,
    }

    /// A decoded sound, track or voice clip.
    pub(crate) enum Clip {
        /// A sound effect, in the effects' context.
        Effect(quad_snd::Sound),
        /// A music track or voice clip on the web, in macroquad's context,
        /// with its length in milliseconds if it is an OGG file.
        #[cfg(target_arch = "wasm32")]
        Shared(Sound, Option<u32>),
        /// A native music track or voice clip in a context of its own.
        #[cfg(not(target_arch = "wasm32"))]
        Music(Music),
    }

    impl shuyi_platform::audio::player::Backend for Macroquad {
        type Sound = Clip;
        #[cfg(target_arch = "wasm32")]
        type Loading = Coroutine<Result<(Sound, Option<u32>), String>>;
        #[cfg(not(target_arch = "wasm32"))]
        type Loading = Pending<Result<Music, String>>;
        /// A sound effect's playback; music and voices have none (they are
        /// stopped by their clip).
        type Playing = Option<Playback>;

        #[cfg(not(target_arch = "wasm32"))]
        fn load_sound(
            &mut self,
            bytes: &[u8],
        ) -> impl std::future::Future<Output = Result<Clip, String>> {
            use std::panic::{AssertUnwindSafe, catch_unwind};
            let ctx = self.effects.get_or_insert_with(AudioContext::new);
            // quad-snd `unwrap`s the decode, at once on native.
            let loaded = catch_unwind(AssertUnwindSafe(|| quad_snd::Sound::load(ctx, bytes)))
                .map(Clip::Effect)
                .map_err(|_| "not a readable OGG or WAV file".to_owned());
            std::future::ready(loaded)
        }

        #[cfg(target_arch = "wasm32")]
        async fn load_sound(&mut self, bytes: &[u8]) -> Result<Clip, String> {
            let ctx = self.effects.get_or_insert_with(AudioContext::new);
            let sound = quad_snd::Sound::load(ctx, bytes);
            // The browser decodes it in the background, as for
            // macroquad's `load_sound_from_bytes`.
            while !sound.is_loaded() {
                macroquad::window::next_frame().await;
            }
            Ok(Clip::Effect(sound))
        }

        #[cfg(target_arch = "wasm32")]
        fn load_music(&mut self, path: &str) -> Self::Loading {
            let path = path.to_owned();
            start_coroutine(async move {
                let bytes = load_file(&path).await.map_err(|e| e.to_string())?;
                let length = shuyi_assets::audio::ogg_length_ms(&bytes);
                load_sound_from_bytes(&bytes)
                    .await
                    .map(|sound| (sound, length))
                    .map_err(|e| e.to_string())
            })
        }

        #[cfg(not(target_arch = "wasm32"))]
        fn load_music(&mut self, path: &str) -> Self::Loading {
            let path = path.to_owned();
            let contexts = self.contexts.clone();
            Pending::spawn("music-load", move || Music::load(&path, contexts))
        }

        #[cfg(target_arch = "wasm32")]
        fn poll_music(&mut self, loading: &mut Self::Loading) -> Option<Result<Clip, String>> {
            loading
                .retrieve()
                .map(|r| r.map(|(sound, length)| Clip::Shared(sound, length)))
        }

        #[cfg(not(target_arch = "wasm32"))]
        fn poll_music(&mut self, loading: &mut Self::Loading) -> Option<Result<Clip, String>> {
            loading.poll().map(|r| r.flatten().map(Clip::Music))
        }

        fn length(&self, sound: &Clip) -> Option<f64> {
            let ms = match sound {
                Clip::Effect(_) => None,
                #[cfg(target_arch = "wasm32")]
                Clip::Shared(_, length) => *length,
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.length_ms(),
            };
            ms.map(|ms| f64::from(ms) / 1000.0)
        }

        fn play(&mut self, sound: &Clip, volume: f32, looped: bool) -> Option<Playback> {
            let params = PlaySoundParams { looped, volume };
            match sound {
                Clip::Effect(s) => return Some(s.play(self.effects.as_ref()?, params)),
                #[cfg(target_arch = "wasm32")]
                Clip::Shared(s, _) => play_sound(s, params),
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.play(volume, looped),
            }
            None
        }

        fn set_volume(&mut self, sound: &Clip, volume: f32) {
            match sound {
                Clip::Effect(s) => {
                    if let Some(ctx) = &self.effects {
                        s.set_volume(ctx, volume);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                Clip::Shared(s, _) => set_sound_volume(s, volume),
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.set_volume(volume),
            }
        }

        fn stop(&mut self, sound: &Clip) {
            match sound {
                Clip::Effect(s) => {
                    if let Some(ctx) = &self.effects {
                        s.stop(ctx);
                    }
                }
                #[cfg(target_arch = "wasm32")]
                Clip::Shared(s, _) => stop_sound(s),
                #[cfg(not(target_arch = "wasm32"))]
                Clip::Music(m) => m.stop(),
            }
        }

        fn set_playing_volume(&mut self, playing: &Option<Playback>, volume: f32) {
            if let (Some(playback), Some(ctx)) = (playing, &self.effects) {
                playback.set_volume(ctx, volume);
            }
        }

        fn stop_playing(&mut self, playing: Option<Playback>) {
            if let (Some(playback), Some(ctx)) = (playing, &self.effects) {
                playback.stop(ctx);
            }
        }

        fn warn(&mut self, line: &str) {
            macroquad::logging::warn!("audio: {}", line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_music_folder_is_beside_the_exe_when_there() {
        let exe = Path::new("/games/some-game/some-game.exe");
        let music = |exe, is_dir: fn(&Path) -> bool| folder_beside("music", exe, is_dir);
        assert_eq!(music(Some(exe), |_| true), "/games/some-game/music");
        assert_eq!(music(Some(exe), |_| false), "music");
        assert_eq!(music(None, |_| true), "music");
        let windows = Path::new(r"C:\games\some-game\some-game.exe");
        let found = music(Some(windows), |d| d.ends_with("music"));
        assert!(!found.contains('\\'), "{found}");
        assert!(found.ends_with("music"), "{found}");
        let exe = std::env::current_exe().ok();
        assert_eq!(
            platform_folder("music"),
            folder_beside("music", exe.as_deref(), Path::is_dir)
        );
    }

    #[test]
    fn a_macos_bundles_folder_is_in_its_resources() {
        let exe = Path::new("/Applications/Game.app/Contents/MacOS/game");
        let resources = "/Applications/Game.app/Contents/Resources/music";
        assert_eq!(
            folder_beside("music", Some(exe), |d| d.ends_with("Resources/music")),
            resources
        );
        assert_eq!(
            folder_beside("music", Some(exe), |_| true),
            "/Applications/Game.app/Contents/MacOS/music"
        );
        let plain = Path::new("/games/MacOS/game");
        assert_eq!(
            folder_beside("music", Some(plain), |d| d.ends_with("Resources/music")),
            "music"
        );
    }

    #[test]
    fn a_folder_beside_the_game_is_found_by_its_name() {
        let exe = Path::new("/games/some-game/some-game.exe");
        assert_eq!(
            folder_beside("voice", Some(exe), |_| true),
            "/games/some-game/voice"
        );
        assert_eq!(folder_beside("voice", Some(exe), |_| false), "voice");
        assert_eq!(folder_beside("voice", None, |_| true), "voice");
        let only_voice = |d: &Path| d.ends_with("voice");
        assert_eq!(folder_beside("music", Some(exe), only_voice), "music");
    }
}
