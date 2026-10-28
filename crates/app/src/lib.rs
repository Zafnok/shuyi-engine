//! Shuyi Engine's macroquad backend (ADR-0008 §2, game ADR-0004): the
//! window, blitting the frame ([`render`]), the keyboard ([`keys`]) and
//! controllers ([`pads`]), the audio player and storage ([`storage`]),
//! behind `shuyi_platform`'s `Platform`, and the loop ([`run`]) that calls
//! `shuyi_platform::App::frame` once per frame of macroquad's, on desktop,
//! the web and the Steam Deck.
//!
//! On desktop, `read_folder` reads a folder on disk (a player's mod) into
//! a bundle layer the game lays over its own files. The web has no
//! folders, so it isn't there.
//!
//! A game on the short path (ADR-0036) runs from a plain `main`, with
//! [`run_game`] and its `BasicGame`:
//!
//! ```ignore
//! fn main() {
//!     shuyi_app::run_game(game::game());
//! }
//! ```
//!
//! A game of its own shape keeps macroquad's entry point, which has to be
//! in the binary, and hands over its [`AppConfig`] and how it starts
//! ([`Start`]):
//!
//! ```ignore
//! fn window_conf() -> macroquad::window::Conf {
//!     shuyi_app::window_conf(&WINDOW)
//! }
//!
//! fn look(game: &MyGame) -> Look<'_> {
//!     Look::new(&game.font, &game.images)
//! }
//!
//! #[macroquad::main(window_conf)]
//! async fn main() {
//!     let start = Start::new(Card::text("My Game"), look, |setup| {
//!         let ctx = setup.ctx(my_ctx()?);
//!         Ok(Game::new(ctx, Box::new(Title), GameHooks::default()))
//!     });
//!     shuyi_app::run(config(), start).await;
//! }
//! ```
//!
//! With the `link` feature, a debug build listens for the live link
//! (ADR-0047), so `shuyi attach` reaches the game running in its window:
//! run it with `SHUYI_LINK=0` (a free port; or a port of your choosing),
//! and its log says `shuyi link: listening on 127.0.0.1:<port>, token
//! <token>`. `SHUYI_LINK_TOKEN` sets the token instead of a random one. A
//! release build, or one without the feature, never listens; on the web
//! the feature does nothing yet.
//!
//! A desktop debug build writes its frames' times as it runs when
//! `SHUYI_PERF` names a file (`SHUYI_PERF=perf.jsonl cargo run`): its log
//! names the file, and after each frame it writes that frame's record as a
//! JSON line (`shuyi_ui::perf::PerfLine`) and flushes, so `tail -f`, or
//! reading the file again from the last line read, follows it (ADR-0046).
//! A build without debug tools logs why and writes nothing; a file that
//! can't be written is one warning, and the game goes on.
//!
//! Every debug build counts the heap: this crate turns on
//! `shuyi_alloc_count`'s `global` feature, whose `#[global_allocator]`
//! counts the bytes the whole program asks for, every thread's, and each
//! frame's record gets the live bytes, their peak and that frame's
//! allocations (`shuyi_ui::perf::FrameRecord::heap`). Its counts are
//! also kept by memory tag: the frame's phases (`load`, `update`, `draw`,
//! `paint`, `perf` and `app`), the game's spans, and each asset kind's
//! decode (`shuyi_alloc_count::tags`). A release build has no counter. A game with a `#[global_allocator]` of its own (a binary
//! has one) opts out in its `.cargo/config.toml`:
//!
//! ```toml
//! [build]
//! rustflags = ["--cfg", "shuyi_no_heap_count"]
//! ```
//!
//! On the web the game's binary must link with `--allow-undefined`: the
//! JS plugins' functions (`crates/cli/web/gamepad.js`'s, `quad-storage`'s) are
//! imports the page provides, not link errors. A link argument only
//! reaches the binary from its own build script
//! (`crates/cli/web/README.md` shows how).

// Every public type here is config a game fills in, so each is
// `#[non_exhaustive]` with a constructor and may gain fields without
// breaking a game (ADR-0039). The lints hold that here only: in the other
// crates most public types are plain values that rightly stay exhaustive.
// They live in this file, not in `Cargo.toml`, because its `[lints]` must
// stay a copy of the workspace's (the test at the bottom).
#![warn(clippy::exhaustive_structs, clippy::exhaustive_enums)]

mod audio;
mod backend;
#[cfg(not(target_arch = "wasm32"))]
mod folder;
pub mod keys;
#[cfg(all(feature = "link", not(target_arch = "wasm32")))]
mod link;
mod pacing;
pub mod pads;
#[cfg(not(target_arch = "wasm32"))]
mod perf_file;
#[cfg(all(feature = "link", not(target_arch = "wasm32")))]
mod readback;
pub mod render;
pub mod storage;

use core::fmt::Debug;

use macroquad::prelude::*;
use shuyi_assets::audio::{Bytes, Files};
use shuyi_assets::keymap::ActionSet;
use shuyi_platform::{App, BasicGame, FrameOutcome, Millis, Start, TextInput};
use shuyi_ui::GameData;
use shuyi_ui::basic::FONT_PNG;
use shuyi_ui::log::{LogLevel, LogLine};

pub use crate::audio::voice::read_manifest as read_voice_manifest;
use crate::backend::Macroquad;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::folder::{FolderError, read_folder};
use crate::pacing::{Pacer, WebFrame};

/// The window a game opens. Build it with [`Window::new`]: a struct
/// literal doesn't compile outside this crate, so a new field never breaks
/// a game (ADR-0039).
///
/// ```compile_fail
/// let window = shuyi_app::Window {
///     title: "A game",
///     width: 640,
///     height: 480,
///     icon: None,
///     vsync: true,
/// };
/// ```
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Window {
    /// The title bar's text.
    pub title: &'static str,
    /// The window's size at start, in pixels (the window is resizable).
    pub width: i32,
    /// See [`width`](Self::width).
    pub height: i32,
    /// The window's icon, if the game has one.
    pub icon: Option<Icon>,
    /// Whether the window asks the display's driver for vsync: on by
    /// default ([`Window::new`]), so frames don't tear. The driver may
    /// ignore it, and it is read only when the window opens, so it is the
    /// developer's choice, not the player's.
    ///
    /// The frame cap (`shuyi_ui::frame_cap`, 60 frames a second unless the
    /// game or the player says) limits the rate either way, and never
    /// waits on top of vsync: on a 60 Hz display with vsync on, a cap of 60
    /// leaves vsync to it. On a VRR display (G-Sync, `FreeSync`, adaptive
    /// sync) with vsync on, a cap a few frames under the display's refresh
    /// (141 on a 144 Hz display) keeps it in its variable range, and the
    /// display follows the frames. A tick stage steps 60 times a second, so
    /// above 60 each step shows for whole frames: evenly at 120 and 240, but
    /// 2 and 3 frames in turn at 144 and 165; a cap that is a multiple of 60
    /// avoids it.
    pub vsync: bool,
}

impl Window {
    /// A window titled `title`, `width` × `height` pixels at start, with
    /// no icon ([`with_icon`](Self::with_icon) gives one) and vsync on
    /// ([`with_vsync`](Self::with_vsync) turns it off).
    #[must_use]
    pub const fn new(title: &'static str, width: i32, height: i32) -> Self {
        Self {
            title,
            width,
            height,
            icon: None,
            vsync: true,
        }
    }

    /// The window with `icon` (`None` for none): usable in a `const`
    /// item.
    #[must_use]
    pub const fn with_icon(self, icon: Option<Icon>) -> Self {
        Self { icon, ..self }
    }

    /// The window asking for vsync or not ([`vsync`](Self::vsync)):
    /// usable in a `const` item.
    #[must_use]
    pub const fn with_vsync(self, vsync: bool) -> Self {
        Self { vsync, ..self }
    }
}

/// A window icon: PNG files of 16, 32 and 64 pixels square.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct Icon {
    /// The 16 × 16 picture.
    pub small: &'static [u8],
    /// The 32 × 32 picture.
    pub medium: &'static [u8],
    /// The 64 × 64 picture.
    pub big: &'static [u8],
}

impl Icon {
    /// An icon of the three pictures, 16, 32 and 64 pixels square.
    #[must_use]
    pub const fn new(small: &'static [u8], medium: &'static [u8], big: &'static [u8]) -> Self {
        Self { small, medium, big }
    }
}

/// Where a game's voice clips are (game ADR-0046): a folder shipped beside the
/// game (not embedded), with a folder per language holding the manifest
/// and the clips.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct VoiceFiles {
    /// The folder beside the game, e.g. `voice`.
    pub folder: &'static str,
    /// The script's language, whose folder is read, e.g. `en`.
    pub lang: &'static str,
    /// The manifest's file name in that folder, e.g. `voice.ron`.
    pub manifest: &'static str,
}

impl VoiceFiles {
    /// The clips in `folder`, in its folder for `lang`, listed by the
    /// manifest file `manifest`.
    #[must_use]
    pub const fn new(folder: &'static str, lang: &'static str, manifest: &'static str) -> Self {
        Self {
            folder,
            lang,
            manifest,
        }
    }
}

/// What a game tells the macroquad backend. How it starts, loads and is
/// painted is its [`Start`]. Build it with [`AppConfig::new`].
#[non_exhaustive]
pub struct AppConfig {
    /// The window.
    pub window: Window,
    /// The storage's name: the folder in the OS's data folder on native
    /// ([`storage::data_dir`]), the prefix of every `localStorage` key on
    /// the web ([`storage::web_prefix`]). Changing it loses every save.
    pub storage_name: &'static str,
    /// The game's embedded files by path: the images, the font atlas's
    /// picture and a loading card's picture are read with it.
    pub files: fn(&str) -> Option<&'static [u8]>,
    /// The font atlas's picture: its path for [`files`](Self::files).
    pub font_png: &'static str,
    /// The sound files the audio manifest names, and the music folder
    /// shipped beside the game (not embedded).
    pub sounds: Files<'static>,
    /// Where the voice clips are; `None` for a game without voices.
    pub voices: Option<VoiceFiles>,
    /// The system's own keyboard for text, if the game's build has one
    /// (Steam's on-screen keyboard is the game's: its ticket 0903);
    /// `None` where the player types on keys.
    pub text_input: Option<Box<dyn TextInput>>,
}

impl AppConfig {
    /// The config of a game with every field it must choose, with no
    /// voices and no system keyboard: a game that has them assigns
    /// [`voices`](Self::voices) and [`text_input`](Self::text_input).
    #[must_use]
    pub const fn new(
        window: Window,
        storage_name: &'static str,
        files: fn(&str) -> Option<&'static [u8]>,
        font_png: &'static str,
        sounds: Files<'static>,
    ) -> Self {
        Self {
            window,
            storage_name,
            files,
            font_png,
            sounds,
            voices: None,
            text_input: None,
        }
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
        platform: miniquad::conf::Platform {
            // A hint the driver may ignore; miniquad reads it only here.
            swap_interval: Some(i32::from(window.vsync)),
            ..Default::default()
        },
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

/// Runs a game until it quits, as `start` says: the first frame shows its
/// loading card, the next loads it (or shows why it can't: a wrong command
/// line, a broken asset), and each frame after, `App` reads the keyboard
/// and controllers, steps the game, plays its sounds, music and voices and
/// draws its frame. Closing the window lets the game save first.
///
/// Frames come no faster than the game's frame cap (`shuyi_ui::frame_cap`,
/// 60 a second unless the game or the player says): after each frame is
/// drawn, the loop waits until the next is due, unless vsync or the
/// machine already keeps them that far apart, and reports how it paces to
/// the game before the next. On the web the browser sets the rate: a frame
/// before the due time shows the last one again, and its input waits for
/// the next frame that runs.
pub async fn run<G: GameData>(mut config: AppConfig, start: Start<G>) {
    // Voices (game ADR-0046): none, and nothing said, without the folder.
    let voice_dir = config
        .voices
        .map(|v| audio::voice::platform_voice_dir(v.folder, v.lang));
    let mut voice_manifest = None;
    if let (Some(voices), Some(dir)) = (&config.voices, &voice_dir) {
        match read_voice_manifest(dir, voices.manifest).await {
            Ok(manifest) => voice_manifest = manifest,
            Err(e) => {
                warn!("no voices: {}", e);
            }
        }
    }
    let start = start.before_load(move |setup| setup.voice_manifest = voice_manifest);
    // What each decode allocates counts under its kind, from the load on.
    backend::tag_decodes();
    let mut platform = Macroquad::new(&mut config);
    let mut app = App::new(start);
    let mut pacer = Pacer::new(config.window.vsync);
    #[cfg(all(feature = "link", not(target_arch = "wasm32")))]
    let mut link = link::start(config.window.title);
    #[cfg(not(target_arch = "wasm32"))]
    let mut perf_file = perf_file::start();
    loop {
        let start = pacing::seconds();
        if cfg!(target_arch = "wasm32") {
            if pacer.web_frame(start) == WebFrame::Skip {
                platform.hold_input();
                app.present_again(&mut platform);
                next_frame().await;
                continue;
            }
            app.set_pacing(pacer.pacing());
        } else {
            pacer.start(start);
        }
        // The pacer's reading: one clock for the frame (it never goes back).
        let now = Millis::from_secs_f64(start);
        #[cfg(all(feature = "link", not(target_arch = "wasm32")))]
        if let Some(link) = &mut link {
            link.before_frame(&app, platform.link_input());
        }
        let outcome = app.frame(&mut platform, now);
        #[cfg(all(feature = "link", not(target_arch = "wasm32")))]
        if let Some(link) = &mut link {
            answer_link(link, &app, &mut platform);
        }
        // E.g. saved key bindings that had to be repaired (game ADR-0031).
        for line in app.take_new_log() {
            print_line(&line);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(file) = perf_file.take() {
            let frame = app.frame_number();
            perf_file = file.write(&app.take_new_perf(), frame, || {
                app.heap_snapshot(&mut platform)
            });
        }
        if !platform.started()
            && let Some(game) = app.game()
        {
            let voice_dir = voice_dir.clone().unwrap_or_default();
            platform.start(game, &config.sounds, voice_dir).await;
        }
        // Native: leaving the loop closes the window. The web page can't be
        // closed, so it keeps showing the last frame.
        if outcome == FrameOutcome::Quit && !cfg!(target_arch = "wasm32") {
            break;
        }
        pace(&mut pacer, &platform, &mut app);
        next_frame().await;
    }
}

/// After a frame is drawn: the cap it asked for applies from the next, and
/// on native the loop waits until the next is due, then reports how it
/// paces to `app` (on the web the report goes before each frame that runs).
fn pace<G: GameData>(pacer: &mut Pacer, platform: &Macroquad, app: &mut App<G>) {
    let (cap, serial) = platform.frame_cap();
    pacer.set_cap(cap, serial);
    if cfg!(target_arch = "wasm32") {
        return;
    }
    let wait = pacer.wait(pacing::seconds());
    pacing::sleep(wait, pacing::seconds);
    pacer.waited(pacing::seconds());
    app.set_pacing(pacer.pacing());
}

/// Sends the link's answers due after `app`'s frame, and the window's
/// picture if its client asked: read now, drawn and not yet shown; or a
/// heap snapshot with the frames' records, taken now.
#[cfg(all(feature = "link", not(target_arch = "wasm32")))]
fn answer_link<G: GameData>(link: &mut link::Listener, app: &App<G>, platform: &mut Macroquad) {
    link.after_frame(app);
    if link.wants_screenshot() {
        match readback::window_png() {
            Ok((png, width, height)) => link.screenshot(&png, width, height),
            Err(e) => link.no_screenshot(e),
        }
    }
    if link.wants_snapshot() {
        let snapshot = app.heap_snapshot(platform);
        link.snapshot(app, snapshot);
    }
}

/// Runs a game on the short path until it quits, in a window of its
/// console at twice its size: its files become the process's bundle, and
/// [`run`] starts it as [`BasicGame::start`] says, with no sounds, no
/// voices and no system keyboard. Called from a plain `fn main`.
pub fn run_game<A: ActionSet, T: Debug + 'static>(game: BasicGame<A, T>) {
    // Only the binary installs the bundle: the tools and the tests read
    // the one the game's data holds, since one test process runs many
    // games. A bundle installed before (none is) would stay.
    let _ = shuyi_assets::bundle::install(game.bundle());
    let window = basic_window(&game);
    // What `#[macroquad::main]` expands to (macroquad_macro 0.1.8).
    macroquad::Window::from_config(window_conf(&window), run(basic_config(&game), game.start()));
}

/// The margin around a short-path game's console, in pixels.
const BASIC_MARGIN: i32 = 16;

/// The window of a game on the short path: its console at twice its size
/// and a small margin, titled with its name.
fn basic_window<A: ActionSet, T: Debug + 'static>(game: &BasicGame<A, T>) -> Window {
    let console = game.console_spec();
    let side = |cells: u16, cell: u16| i32::from(cells) * i32::from(cell) * 2 + BASIC_MARGIN;
    Window::new(
        game.name(),
        side(console.cols, console.cell_w),
        side(console.rows, console.cell_h),
    )
}

/// A file of the bundle [`run_game`] installed.
fn installed_file(path: &str) -> Option<&'static [u8]> {
    shuyi_assets::bundle::installed()?.bytes(path)
}

/// No sound file: a game on the short path has none.
fn no_sound(_: &str) -> Bytes {
    None
}

/// How errors name a sound file.
fn sound_shown(name: &str) -> String {
    name.to_owned()
}

/// Whether a build leaves a sound file out on purpose: none does.
fn sound_left_out(_: &str) -> bool {
    false
}

/// What the backend needs to know about a game on the short path: its
/// storage's name, its files from the installed bundle and its font's
/// picture; no sounds, no voices and no system keyboard.
fn basic_config<A: ActionSet, T: Debug + 'static>(game: &BasicGame<A, T>) -> AppConfig {
    let sounds = Files::new(&no_sound, &sound_shown, &sound_left_out, "music");
    AppConfig::new(
        basic_window(game),
        game.storage_name(),
        installed_file,
        FONT_PNG,
        sounds,
    )
}

/// Prints a line of App's log with macroquad's macro for its level.
fn print_line(line: &LogLine) {
    let (category, text) = (line.category.name(), &line.text);
    match line.level {
        LogLevel::Error => {
            error!("{}: {}", category, text);
        }
        LogLevel::Warn => {
            warn!("{}: {}", category, text);
        }
        LogLevel::Info => {
            info!("{}: {}", category, text);
        }
        _ => {
            debug!("{}: {}", category, text);
        }
    }
}

/// The most characters of an error shown on one line.
const ERROR_WIDTH: usize = 150;

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
        let broken = Icon::new(b"not a png", b"not a png", b"not a png");
        assert!(window_icon(&broken).is_none());
        let window = Window::new("A game", 640, 480).with_icon(Some(broken));
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

    /// A window built in a `const` item, as games build theirs.
    const ICONED: Window =
        Window::new("A game", 640, 480).with_icon(Some(Icon::new(b"small", b"medium", b"big")));

    /// A window without vsync, built in a `const` item.
    const NO_VSYNC: Window = Window::new("A game", 640, 480).with_vsync(false);

    #[test]
    fn a_window_asks_for_vsync_unless_told_not_to() {
        let window = Window::new("A game", 640, 480);
        assert!(window.vsync);
        assert_eq!(window_conf(&window).platform.swap_interval, Some(1));
        let off = NO_VSYNC;
        assert!(!off.vsync);
        assert_eq!(window_conf(&off).platform.swap_interval, Some(0));
        assert!(off.with_vsync(true).vsync);
        assert!(basic_window(&basic()).vsync);
    }

    #[test]
    fn a_new_window_has_no_icon_and_with_icon_sets_one() {
        let window = Window::new("A game", 640, 480);
        assert_eq!(
            (window.title, window.width, window.height),
            ("A game", 640, 480)
        );
        assert!(window.icon.is_none());
        let icon = ICONED.icon.unwrap();
        assert_eq!(
            (icon.small, icon.medium, icon.big),
            (&b"small"[..], &b"medium"[..], &b"big"[..])
        );
        assert!(ICONED.with_icon(None).icon.is_none());
    }

    #[test]
    fn a_new_config_has_no_voices_and_no_system_keyboard() {
        let mut config = basic_config(&basic());
        config = AppConfig::new(
            config.window,
            "a-game",
            installed_file,
            FONT_PNG,
            config.sounds,
        );
        assert!(config.voices.is_none() && config.text_input.is_none());
        config.voices = Some(VoiceFiles::new("voice", "en", "voice.ron"));
        let voices = config.voices.unwrap();
        assert_eq!(
            (voices.folder, voices.lang, voices.manifest),
            ("voice", "en", "voice.ron")
        );
    }

    /// A game on the short path with no files.
    fn basic() -> BasicGame<shuyi_assets::keymap::UiAction> {
        let files = shuyi_assets::bundle::Layer::table("files", &[]);
        BasicGame::new("A game", "a-game", files, |_| unreachable!("never started"))
    }

    #[test]
    fn a_short_path_games_window_is_its_console_twice_and_a_margin() {
        let window = basic_window(&basic());
        assert_eq!(
            (window.title, window.width, window.height),
            ("A game", 784, 784)
        );
        assert!(window.icon.is_none());
        let small = basic().console(shuyi_ui::frame::ConsoleSpec::text(20, 10));
        let window = basic_window(&small);
        assert_eq!((window.width, window.height), (336, 336));
    }

    #[test]
    fn a_short_path_games_config_is_silent_and_reads_its_font() {
        let config = basic_config(&basic());
        assert_eq!(config.storage_name, "a-game");
        assert_eq!(config.font_png, "font/atlas.png");
        assert_eq!(config.window.title, "A game");
        assert_eq!((config.sounds.sound)("sfx/move.wav"), None);
        assert_eq!((config.sounds.sound_shown)("sfx/move.wav"), "sfx/move.wav");
        assert!(!(config.sounds.sound_left_out)("sfx/move.wav"));
        assert!(config.sounds.music.is_none());
        assert!(config.voices.is_none() && config.text_input.is_none());
        // Nothing is installed in the tests: no file is found.
        assert_eq!((config.files)("font/atlas.png"), None);
    }

    /// Without the `link` feature the backend never reads `SHUYI_LINK`:
    /// only the link's module names it, and it is compiled with the
    /// feature alone.
    #[test]
    fn only_the_link_feature_reads_shuyi_link() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = vec![src.clone()];
        let mut naming = Vec::new();
        while let Some(path) = files.pop() {
            if path.is_dir() {
                files.extend(std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                if text.contains("SHUYI_LINK") {
                    naming.push(path.strip_prefix(&src).unwrap().to_owned());
                }
            }
        }
        naming.sort();
        let link = std::path::PathBuf::from("link.rs");
        let lib = std::path::PathBuf::from("lib.rs");
        assert_eq!(naming, [lib, link]);
        let here = include_str!("lib.rs");
        let gate = "#[cfg(all(feature = \"link\", not(target_arch = \"wasm32\")))]
mod link;";
        assert!(here.contains(gate));
    }

    /// The lines of `manifest`'s `[header]` table, without comments and
    /// blank lines.
    fn table<'a>(manifest: &'a str, header: &str) -> Vec<&'a str> {
        let lines = manifest.lines().skip_while(|l| l.trim() != header).skip(1);
        lines
            .take_while(|l| !l.starts_with('['))
            .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
            .collect()
    }

    /// This crate copies the workspace lints because one of them differs
    /// (`unsafe_code`, game ADR-0034); the copy must not drift. Every other
    /// crate inherits the workspace's `forbid`.
    #[test]
    fn the_lints_are_the_workspace_lints_except_unsafe_code() {
        let here = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let read = |path: &std::path::Path| std::fs::read_to_string(path).unwrap();
        let root = here.parent().unwrap().parent().unwrap();
        let workspace = read(&root.join("Cargo.toml"));
        let clippy = table(&workspace, "[workspace.lints.clippy]");
        assert!(clippy.len() > 5, "{clippy:?}");
        let rust: Vec<String> = table(&workspace, "[workspace.lints.rust]")
            .iter()
            .map(|l| l.replace("unsafe_code = \"forbid\"", "unsafe_code = \"deny\""))
            .collect();
        assert!(
            rust.contains(&"unsafe_code = \"deny\"".to_owned()),
            "{rust:?}"
        );
        let manifest = read(&here.join("Cargo.toml"));
        assert_eq!(table(&manifest, "[lints.clippy]"), clippy);
        assert_eq!(table(&manifest, "[lints.rust]"), rust);
        for other in [
            "crates/assets",
            "crates/cli",
            "crates/core",
            "crates/platform",
            "crates/ui",
            "examples/hello-grid",
        ] {
            let manifest = read(&root.join(other).join("Cargo.toml"));
            assert_eq!(table(&manifest, "[lints]"), ["workspace = true"], "{other}");
        }
    }
}
