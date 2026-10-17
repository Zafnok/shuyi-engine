//! Shuyi Engine's platform shell (ADR-0004): the window, blitting the
//! frame ([`render`]), the keyboard ([`keys`]) and controllers ([`pads`]),
//! the audio backend, storage ([`storage`]) and the run loop ([`run`]).
//!
//! A game's binary keeps macroquad's entry point, which has to be in the
//! binary, and hands over:
//!
//! ```ignore
//! fn window_conf() -> macroquad::window::Conf {
//!     shuyi_app::window_conf(&WINDOW)
//! }
//!
//! #[macroquad::main(window_conf)]
//! async fn main() {
//!     shuyi_app::run(config(), |platform| {
//!         let ctx = platform.setup(my_ctx()?);
//!         Ok(Game::new(ctx, Box::new(Title), GameHooks::default()))
//!     })
//!     .await;
//! }
//! ```
//!
//! On the web the game's binary must link with `--allow-undefined`: the
//! JS plugins' functions (`web/gamepad.js`'s, `quad-storage`'s) are
//! imports the page provides, not link errors. A link argument only
//! reaches the binary from its own build script.

mod audio;
pub mod keys;
pub mod pads;
pub mod render;
pub mod storage;

use macroquad::prelude::*;
use shuyi_assets::audio::Files;
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageTable;
use shuyi_ui::frame::Rgb;
use shuyi_ui::{Ctx, Game, GameData, IdleMotion, KeyPrompt};

use crate::audio::Audio;
use crate::audio::device::Macroquad;
pub use crate::audio::voice::read_manifest as read_voice_manifest;
use crate::pads::PadInput;
use crate::render::Renderer;

/// The window a game opens.
#[derive(Debug, Clone, Copy)]
pub struct Window {
    /// The title bar's text.
    pub title: &'static str,
    /// The window's size at start, in pixels (the window is resizable).
    pub width: i32,
    /// See [`width`](Self::width).
    pub height: i32,
    /// The window's icon, if the game has one.
    pub icon: Option<Icon>,
}

/// A window icon: PNG files of 16, 32 and 64 pixels square.
#[derive(Debug, Clone, Copy)]
pub struct Icon {
    /// The 16 × 16 picture.
    pub small: &'static [u8],
    /// The 32 × 32 picture.
    pub medium: &'static [u8],
    /// The 64 × 64 picture.
    pub big: &'static [u8],
}

/// Where a game's voice clips are (ADR-0046): a folder shipped beside the
/// game (not embedded), with a folder per language holding the manifest
/// and the clips.
#[derive(Debug, Clone, Copy)]
pub struct VoiceFiles {
    /// The folder beside the game, e.g. `voice`.
    pub folder: &'static str,
    /// The script's language, whose folder is read, e.g. `en`.
    pub lang: &'static str,
    /// The manifest's file name in that folder, e.g. `voice.ron`.
    pub manifest: &'static str,
}

/// What a game tells the platform shell. `G` is the game's part of the
/// context ([`GameData`]).
pub struct AppConfig<G> {
    /// The window.
    pub window: Window,
    /// The storage's name: the folder in the OS's data folder on native
    /// ([`storage::data_dir`]), the prefix of every `localStorage` key on
    /// the web ([`storage::web_prefix`]). Changing it loses every save.
    pub storage_name: &'static str,
    /// The game's embedded files by path: the images and the font atlas's
    /// picture are read with it.
    pub files: fn(&str) -> Option<&'static [u8]>,
    /// The font atlas's description in the game's data.
    pub font: fn(&G) -> &FontAtlasDef,
    /// The font atlas's picture: its path for [`files`](Self::files).
    pub font_png: &'static str,
    /// The image table in the game's data.
    pub images: fn(&G) -> &ImageTable,
    /// The sound files the audio manifest names, and the music folder
    /// shipped beside the game (not embedded).
    pub sounds: Files<'static>,
    /// Where the voice clips are; `None` for a game without voices.
    pub voices: Option<VoiceFiles>,
}

/// What the platform gives a game as it starts: what goes into its
/// context ([`setup`](Self::setup)).
#[derive(Debug)]
pub struct Platform {
    storage_name: &'static str,
    /// The text of the voice manifest, if one was found.
    voice_manifest: Option<String>,
}

impl Platform {
    /// `ctx` on this platform: its saves and settings in the platform's
    /// storage, the voices the voice manifest allows, idle motion on, a
    /// key prompt waiting and a music seed from the clock, so each launch
    /// picks differently. Problems found on the way are logged when the
    /// game starts.
    pub fn setup<G: GameData>(&self, mut ctx: Ctx<G>) -> Ctx<G> {
        ctx.idle_motion = IdleMotion::On;
        ctx.music_seed = miniquad::date::now().to_bits();
        ctx.key_prompt = KeyPrompt::Waiting;
        let mut ctx = ctx.with_storage(storage::platform(self.storage_name));
        if let Some(manifest) = &self.voice_manifest {
            ctx.set_voice_manifest(manifest);
        }
        ctx
    }
}

/// The window settings for macroquad's entry point
/// (`#[macroquad::main(window_conf)]`).
pub fn window_conf(window: &Window) -> Conf {
    Conf {
        window_title: window.title.to_owned(),
        window_width: window.width,
        window_height: window.height,
        window_resizable: true,
        // Full-resolution framebuffer on scaled displays, so the console's
        // integer scale is in real pixels (see render.rs).
        high_dpi: true,
        icon: window.icon.and_then(|icon| window_icon(&icon)),
        ..Default::default()
    }
}

/// The window's icon, or `None` if a picture doesn't decode or isn't the
/// right size.
fn window_icon(icon: &Icon) -> Option<miniquad::conf::Icon> {
    fn pixels<const N: usize>(png: &[u8]) -> Option<[u8; N]> {
        let image = Image::from_file_with_format(png, None).ok()?;
        image.bytes.try_into().ok()
    }
    Some(miniquad::conf::Icon {
        small: pixels(icon.small)?,
        medium: pixels(icon.medium)?,
        big: pixels(icon.big)?,
    })
}

/// Runs a game until it quits: `make` builds it on the [`Platform`] (or
/// says why it can't, which is shown instead of the game: a wrong command
/// line, a broken asset). Each frame the keyboard and controllers go in,
/// the frame is drawn, and its sounds, music and voices play.
pub async fn run<G: GameData>(
    config: AppConfig<G>,
    make: impl FnOnce(&Platform) -> Result<Game<G>, String>,
) {
    // Voices (ADR-0046): none, and nothing said, without the folder.
    let voice_dir = config
        .voices
        .map(|v| audio::voice::platform_voice_dir(v.folder, v.lang));
    let mut voice_manifest = None;
    if let (Some(voices), Some(dir)) = (&config.voices, &voice_dir) {
        match read_voice_manifest(dir, voices.manifest).await {
            Ok(manifest) => voice_manifest = manifest,
            Err(e) => warn!("no voices: {}", e),
        }
    }
    let platform = Platform {
        storage_name: config.storage_name,
        voice_manifest,
    };
    let mut game = match make(&platform) {
        Ok(game) => game,
        Err(e) => return show_errors(&e).await,
    };
    // E.g. saved key bindings that had to be repaired (ADR-0031).
    for warning in game.ctx_mut().take_warnings() {
        warn!("{}", warning);
    }
    #[cfg(debug_assertions)]
    storage::smoke_check(game.ctx_mut().storage.computer_mut());
    let ctx = game.ctx();
    let png = (config.files)(config.font_png).unwrap_or_default();
    let font = (config.font)(&ctx.game).clone();
    let clear = clear_color(&game);
    let images = (config.images)(&ctx.game);
    let mut renderer = match Renderer::new(font, png, clear, images, config.files) {
        Ok(renderer) => renderer,
        Err(e) => return show_errors(&e).await,
    };
    let mut speaker = Macroquad::default();
    let mut audio = Audio::load(
        &mut speaker,
        ctx.game.audio_manifest().clone(),
        &config.sounds,
        audio::platform_folder(config.sounds.music_dir),
        voice_dir.unwrap_or_default(),
        miniquad::date::now().to_bits(),
    )
    .await;
    let mut pads = PadInput::new(ctx.game.keymap_def().stick);
    let mut running = true;
    // The window starts windowed; the Fullscreen option switches it.
    let mut fullscreen = false;
    loop {
        if keys::fullscreen_shortcut() {
            game.toggle_fullscreen();
        }
        // A colour theme changes the console background.
        renderer.set_clear(clear_color(&game));
        let mut events = keys::poll();
        events.extend(pads.poll());
        if running {
            // The music clock (ADR-0037): wall-clock time, which keeps
            // counting while a web tab is hidden and no frames run.
            let now = miniquad::date::now();
            game.set_music_playing(audio.music_playing(now));
            let out = game.frame(&events, get_frame_time());
            if out.fullscreen != fullscreen {
                fullscreen = out.fullscreen;
                set_fullscreen(fullscreen);
            }
            audio.set_volumes(&mut speaker, out.music_volume, out.sound_volume);
            audio.set_voice_volume(&mut speaker, out.voice_volume);
            audio.play(&mut speaker, out.audio, out.music, now);
            for warning in audio.take_warnings() {
                warn!("audio: {}", warning);
            }
            renderer.draw(out.buffer);
            if out.quit {
                audio.stop_all(&mut speaker);
                // Native: leaving the loop closes the window. The web page
                // can't be closed, so it stops updating and keeps the last
                // frame.
                if cfg!(target_arch = "wasm32") {
                    running = false;
                } else {
                    break;
                }
            }
        } else {
            renderer.draw(game.buffer());
        }
        next_frame().await;
    }
}

/// The console's background: that of the cell the frame is cleared to
/// ([`GameData::blank_cell`]).
fn clear_color<G: GameData>(game: &Game<G>) -> Rgb {
    let ctx = game.ctx();
    ctx.game.blank_cell(&ctx.palette).bg
}

/// The most characters of an error shown on one line.
const ERROR_WIDTH: usize = 150;

/// Shows errors instead of the game (with macroquad's built-in font, since
/// the game's may be what failed): asset errors, only reachable if a broken
/// asset slipped past the game's tests, or a wrong command line.
async fn show_errors(text: &str) {
    error!("{}", text);
    let text = wrap(text, ERROR_WIDTH);
    loop {
        clear_background(BLACK);
        let mut y = 40.0;
        for line in text.lines() {
            draw_text(line, 20.0, y, 22.0, RED);
            y += 24.0;
        }
        next_frame().await;
    }
}

/// `text` with every line broken at spaces so that none is longer than
/// `width` characters (a single longer word stays whole).
fn wrap(text: &str, width: usize) -> String {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut current = String::new();
        for word in line.split(' ') {
            let fits = current.chars().count() + 1 + word.chars().count() <= width;
            if !current.is_empty() && !fits {
                out.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        out.push(current);
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_lines_are_broken_at_spaces() {
        assert_eq!(wrap("aa bb cc", 5), "aa bb\ncc");
        assert_eq!(wrap("aa bb cc", 4), "aa\nbb\ncc");
        assert_eq!(wrap("aa bb cc", 8), "aa bb cc");
        assert_eq!(wrap("aa bb cc", 7), "aa bb\ncc");
        assert_eq!(wrap("abcdefgh ij", 3), "abcdefgh\nij");
        assert_eq!(wrap("one\n\ntwo three", 5), "one\n\ntwo\nthree");
        assert_eq!(wrap("", 5), "");
    }

    #[test]
    fn an_icon_that_does_not_decode_is_none() {
        let broken = Icon {
            small: b"not a png",
            medium: b"not a png",
            big: b"not a png",
        };
        assert!(window_icon(&broken).is_none());
        let window = Window {
            title: "A game",
            width: 640,
            height: 480,
            icon: Some(broken),
        };
        let conf = window_conf(&window);
        assert!(conf.icon.is_none());
        assert_eq!(
            (
                conf.window_title.as_str(),
                conf.window_width,
                conf.window_height
            ),
            ("A game", 640, 480)
        );
    }
}
