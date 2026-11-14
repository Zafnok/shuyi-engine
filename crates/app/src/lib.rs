//! Shuyi Engine's macroquad backend (ADR-0008 §2, game ADR-0004): the
//! window, blitting the frame ([`render`]), the keyboard ([`keys`]) and
//! controllers ([`pads`]), the audio player and storage ([`storage`]),
//! behind `shuyi_platform`'s `Platform`, and the loop ([`run`]) that calls
//! `shuyi_platform::App::frame` once per frame of macroquad's, on desktop,
//! the web and the Steam Deck.
//!
//! On desktop, `read_folder` reads a folder on disk (a player's mod) into
//! a bundle layer the game lays over its own files. The web has no
//! folders, so it isn't there: there [`page::page_layer`] reads the files
//! the page holds into one instead. In a build with debug tools the folders it
//! reads are watched, and `watched_layer` gives the one `cargo shuyi run
//! --watch` names (ADR-0071); `read_watched_folder` reads one watched in
//! every build.
//!
//! A shipped build reads its files from the pack a release export bakes
//! beside the program, [`pack_beside_program`] (ADR-0110): on the web the
//! page fetches it before the game starts.
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
//! [`run_game_checked`] runs it with checks a page may ask it to make of
//! the files it was given ([`page::page_check`]), as the book's steps run
//! their exercises' (ADR-0116).
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
//! With the `capture` feature, a desktop build runs a plan instead of the
//! player's input when `SHUYI_CAPTURE` names one, compares the window with
//! the raster on the frames it names, and exits (the `capture` module, ADR-0052):
//!
//! ```text
//! SHUYI_CAPTURE=examples/hello-grid/capture/frames.ron cargo run -p hello-grid --features capture
//! ```
//!
//! It writes each capture's pictures and `capture.txt` under
//! `SHUYI_CAPTURE_OUT` (`target/capture` unless set), and exits 0 when
//! every capture is within the plan's tolerance, 1 when one isn't, and 2
//! when the plan can't run. Without the feature the variable is never
//! read. On the web a build with the feature always runs the plan
//! `capture.ron` beside the page, writes no files and logs each capture's
//! line to the browser's console instead (ticket 2019).
//!
//! With the `sound-probe` feature, `SoundProbeBackend` names the sound
//! backend, for the example `sound_probe` that the parity gates record. No
//! game turns it on.
//!
//! With the `in-house-sound` feature, sound goes through `shuyi-sound`'s mixer and
//! `shuyi-sound-out` instead of quad-snd.
//!
//! A desktop debug build writes its frames' times as it runs when
//! `SHUYI_PERF` names a file (`SHUYI_PERF=perf.jsonl cargo run`): its log
//! names the file, and after each frame it writes that frame's record as a
//! JSON line (`shuyi_ui::perf::PerfLine`) and flushes, so `tail -f`, or
//! reading the file again from the last line read, follows it (ADR-0046).
//! A build without debug tools logs why and writes nothing; a file that
//! can't be written is one warning, and the game goes on.
//!
//! A panic writes a crash report through the game's storage, beside its
//! saves (the [`crash`] module, ADR-0079): `crash-<when>.ron` in its data
//! folder on native, and in `localStorage` on the web. At its next start
//! the game hears of it once (`shuyi_platform::Setup::last_crash`).
//! [`AppConfig::crash`] names the game's version in it, or turns it off.
//!
//! A game's screens hand the player a file and ask them for one
//! (`shuyi_ui::Ctx::offer_file`, `ask_for_file`): on a desktop the file is
//! written into `exports/` in the game's data folder, which the game may
//! open in the system's file manager, and the player drops a file on the
//! window to give one; on the web it is a download, and the browser's file
//! picker opens (ticket 3309).
//!
//! A desktop debug build keeps its last sessions in `sessions/` under the
//! game's data folder (the `sessions` module, ADR-0074): the record of a run,
//! written when the game quits and when the debug menu's "Save session" asks,
//! the newest ten kept. On the web that entry offers a download. `shuyi run
//! --session <file>` plays one.
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
//! JS plugins' functions (`crates/cli/web/gamepad.js`'s, `player-files.js`'s,
//! `page-files.js`'s, `pack-file.js`'s, `storage.js`'s) are
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
#[cfg(feature = "capture")]
pub mod capture;
pub mod crash;
mod drops;
mod facts;
mod focus;
#[cfg(not(target_arch = "wasm32"))]
mod folder;
pub mod keys;
#[cfg(all(feature = "link", not(target_arch = "wasm32")))]
mod link;
mod pacing;
mod pack_file;
pub mod pads;
pub mod page;
mod painted;
#[cfg(not(target_arch = "wasm32"))]
mod perf_file;
mod pictures;
mod player_files;
#[cfg(any(
    all(feature = "link", not(target_arch = "wasm32")),
    feature = "capture"
))]
mod readback;
pub mod render;
mod sessions;
pub mod storage;
#[cfg(not(target_arch = "wasm32"))]
mod watch;

use core::fmt::Debug;
use std::path::Path;

use macroquad::prelude::*;
use shuyi_assets::audio::{Bytes, Files};
use shuyi_assets::bundle::Layer;
use shuyi_assets::keymap::ActionSet;
use shuyi_platform::{App, BasicGame, FrameOutcome, Millis, Platform, Start, TextInput};
use shuyi_ui::GameData;
use shuyi_ui::basic::{BasicData, FONT_PNG};
use shuyi_ui::dialogue::preview::PreviewGame;
use shuyi_ui::log::{LogLevel, LogLine};

#[cfg(feature = "sound-probe")]
pub use crate::audio::Chosen as SoundProbeBackend;
pub use crate::audio::voice::read_manifest as read_voice_manifest;
use crate::backend::Macroquad;
pub use crate::crash::CrashReports;
#[cfg(not(target_arch = "wasm32"))]
pub use crate::folder::{FolderError, read_folder, read_watched_folder, watched_layer};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use crate::folder::{Watched, watched_file, watched_layers};
use crate::pacing::{Pacer, WebFrame};
pub use crate::pack_file::{PACK_EXTENSION, PackFileError, pack_beside_program, pack_file_name};

/// The web has no folders to watch.
#[cfg(target_arch = "wasm32")]
pub fn watched_layer() -> Option<shuyi_assets::bundle::Layer> {
    None
}

/// The web has no folders to read through.
#[cfg(target_arch = "wasm32")]
pub(crate) fn watched_file(_: &str) -> Option<&'static [u8]> {
    None
}

/// What `cargo shuyi run --watch` named: on the web, nothing.
#[cfg(target_arch = "wasm32")]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Watched {
    /// The folder of the game's files.
    pub files: Option<shuyi_assets::bundle::Layer>,
    /// The folder of its private files.
    pub private: Option<shuyi_assets::bundle::Layer>,
}

/// The web has no folders to watch.
#[cfg(target_arch = "wasm32")]
pub(crate) fn watched_layers() -> Watched {
    Watched::default()
}

/// `game` with the folders `cargo shuyi run --watch` named laid over its
/// files: its files' folder [`over`](BasicGame::over) them, and its private
/// files' folder in place of its embedded ones (ADR-0111). `None` when it
/// named none.
fn watched_game<A: ActionSet, T: Debug + 'static>(
    game: &BasicGame<A, T>,
) -> Option<BasicGame<A, T>> {
    let Watched { files, private } = watched_layers();
    let game = game.over(files?);
    Some(match private {
        Some(private) => game.private(private),
        None => game,
    })
}

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

/// A window icon: PNG files of 16, 32 and 64 pixels square. It is
/// `shuyi_platform`'s, which a game's `AppInfo` carries (ADR-0061).
pub use shuyi_platform::WindowIcon as Icon;

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
    /// Which folder a desktop keeps the storage in: the storage's name in
    /// the OS's data folder until the game assigns another
    /// ([`storage::DesktopFolder`]).
    pub desktop_folder: storage::DesktopFolder,
    /// The game's embedded files by path: the images, the font atlas's
    /// picture and a loading card's picture are read with it. Under `cargo
    /// shuyi run --watch` the folder of its files on disk lies over it
    /// ([`watched_layer`]), and a picture saved there is drawn from then on.
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
    /// Whether a panic writes a crash report, and the game's version in it
    /// ([`crash`]): reports with no version until the game assigns
    /// [`CrashReports::new`] with its own, or [`CrashReports::off`].
    pub crash: CrashReports,
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
            desktop_folder: storage::DesktopFolder::Data,
            files,
            font_png,
            sounds,
            voices: None,
            text_input: None,
            crash: CrashReports::new(""),
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
/// The game's context gets this build's target's facts
/// (`shuyi_ui::PlatformFacts`): the target is `SHUYI_TARGET` when the
/// build sets it (the Steam Deck's export sets `steam-deck`), else `web`
/// on the web and `desktop` elsewhere. A target this backend has no
/// profile of is shown as the game's error.
///
/// Frames come no faster than the game's frame cap (`shuyi_ui::frame_cap`,
/// 60 a second unless the game or the player says): after each frame is
/// drawn, the loop waits until the next is due, unless vsync or the
/// machine already keeps them that far apart, and reports how it paces to
/// the game before the next. On the web the browser sets the rate: a frame
/// before the due time shows the last one again, and its input waits for
/// the next frame that runs.
///
/// A panic writes a crash report as [`AppConfig::crash`] says ([`crash`]).
/// On native a panic of the game's frame then stops the sound, adds the
/// game's record of its play to the report and ends the process with exit
/// code 101; the game's system hook isn't called, so a broken game never
/// saves.
pub async fn run<G: GameData>(mut config: AppConfig, start: Start<G>) {
    // Which target this build is: an unknown one is the load's error.
    let (start, facts) = facts::start_as(facts::this_target(), start);
    #[cfg(feature = "capture")]
    if let Some(plan) = capture::requested() {
        capture::run(config, start, plan, facts).await;
        return;
    }
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
    let recorded_manifest = voice_manifest.clone();
    let start = start.before_load(move |setup| setup.voice_manifest = voice_manifest);
    // What each decode allocates counts under its kind, from the load on.
    backend::tag_decodes();
    let storage_at = storage::StorageAt::find(config.storage_name, config.desktop_folder);
    let crash_storage = storage_at.clone();
    crash::install(config.crash, move || crash_storage.open());
    let panic_at = crash::panic_at_frame(config.crash);
    let records_play = config.crash.records_play();
    let mut platform = Macroquad::new(&mut config, facts, storage_at.clone());
    let mut app = App::new(start);
    let kept = sessions::Kept::start(&mut app, &storage_at, recorded_manifest);
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
        let outcome = frame_or_crash(&mut app, &mut platform, now, panic_at, records_play);
        if app.take_session_save() {
            kept.save(&app);
        }
        #[cfg(all(feature = "link", not(target_arch = "wasm32")))]
        if let Some(link) = &mut link {
            answer_link(link, &app, &mut platform);
        }
        // E.g. saved key bindings that had to be repaired (game ADR-0031).
        for line in app.take_new_log() {
            if !kept_in_this_build(line.level) {
                continue;
            }
            print_line(&line);
            let (level, category) = (line.level.name(), line.category.name());
            crash::note_log(format!("f{} {level} {category}: {}", line.frame, line.text));
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
            kept.save_at_quit(&app);
            break;
        }
        pace(&mut pacer, &platform, &mut app);
        next_frame().await;
    }
}

/// Runs `app`'s frame at `now`, and panics first if it is the one
/// `SHUYI_PANIC_AT_FRAME` names (`panic_at`). A panic has written its
/// report: the sound stops, the game's record is added to it if
/// `records_play` ([`CrashReports::records_play`]), and the process ends
/// with exit code 101.
fn frame_or_crash<G: GameData>(
    app: &mut App<G>,
    platform: &mut Macroquad,
    now: Millis,
    panic_at: Option<u32>,
    records_play: bool,
) -> FrameOutcome {
    let frame = app.frame_number().saturating_add(1);
    crash::set_frame(frame, app.game().and_then(shuyi_ui::Game::top_screen));
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        assert!(panic_at != Some(frame), "{}", crash::PANIC_ON_PURPOSE);
        app.frame(platform, now)
    }));
    if let Ok(outcome) = run {
        return outcome;
    }
    platform.audio().stop_all();
    if records_play {
        let record = crash::quiet(|| {
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| app.crash_record()))
        });
        crash::add_record(record.ok().flatten());
    }
    std::process::exit(101);
}

/// Whether a line of the log at `level` is printed and kept for a crash
/// report: every line with debug tools, and warnings and errors without.
fn kept_in_this_build(level: LogLevel) -> bool {
    shuyi_ui::DEBUG_TOOLS || matches!(level, LogLevel::Error | LogLevel::Warn)
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
/// voices and no system keyboard. Called from a plain `fn main`. Under
/// `cargo shuyi run --watch` the game's files on disk are laid over the
/// embedded ones ([`watched_layer`]). On the web, the files the page holds
/// under `files/` (the site's playground, [`page`]) are laid over them when
/// the game's data loads with them, and the page hears the game's problems
/// with them either way. The game's private files (`BasicGame::private`)
/// are read before all of them, and `run --watch` watches their folder
/// too when the game has a checkout of them (ADR-0111).
pub fn run_game<A: ActionSet, T: Debug + 'static>(game: BasicGame<A, T>) {
    run_game_checked(game, &[]);
}

/// A check a page may ask a game on the short path to make of its files
/// ([`run_game_checked`]): its name, and a function of the game's data as
/// it loaded with the page's files, answering `Ok` with why they pass, or
/// `Err` with why not, each a sentence the reader can act on.
pub type PageCheck<A, T> = (&'static str, fn(&BasicData<A, T>) -> Result<String, String>);

/// [`run_game`], with checks a page may ask for (the book's exercises,
/// ADR-0116): when the page names one ([`page::page_check`]) and its files
/// load, the game's data is read with them, the check of that name in
/// `checks` runs on it, and the page hears its result
/// ([`page::report_check`]); a name with no check is reported failed. Files
/// with problems are reported as `run_game` reports them, with no check.
/// Then the game plays as usual.
pub fn run_game_checked<A: ActionSet, T: Debug + 'static>(
    game: BasicGame<A, T>,
    checks: &[PageCheck<A, T>],
) {
    let game = if let Some(watched) = watched_game(&game) {
        watched
    } else {
        let page = page::page_layer(PAGE_FOLDER, page::PageLimits::default());
        let (game, heard) = lay_page(&game, page);
        if let Some(text) = &heard {
            page::report(text);
        }
        if let Some(report) = page_checked(&game, heard.as_deref(), page::page_check(), checks) {
            page::report_check(&report);
        }
        game
    };
    // Only the binary installs the bundle: the tools and the tests read
    // the one the game's data holds, since one test process runs many
    // games. A bundle installed before (none is) would stay.
    let _ = shuyi_assets::bundle::install(game.bundle());
    let window = basic_window(&game);
    // What `#[macroquad::main]` expands to (macroquad_macro 0.1.8).
    macroquad::Window::from_config(window_conf(&window), run(basic_config(&game), game.start()));
}

/// The result of the check `asked` (the page's [`page::page_check`]) of
/// `checks` on `game` as [`lay_page`] left it, once the page heard `heard`:
/// none when the page asks for none, or heard problems.
fn page_checked<A: ActionSet, T: Debug + 'static>(
    game: &BasicGame<A, T>,
    heard: Option<&str>,
    asked: Option<String>,
    checks: &[PageCheck<A, T>],
) -> Option<page::CheckReport> {
    let name = asked?;
    if heard.is_some_and(|text| !text.is_empty()) {
        return None;
    }
    let Some((_, check)) = checks.iter().find(|(known, _)| *known == name) else {
        let text = format!("This game has no check named `{name}`.");
        return Some(page::CheckReport::new(name, false, text));
    };
    let result = game.data().map_err(|errors| {
        let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
        lines.join("\n")
    });
    let report = match result.and_then(|data| check(&data)) {
        Ok(text) => page::CheckReport::new(name, true, text),
        Err(text) => page::CheckReport::new(name, false, text),
    };
    Some(report)
}

/// The folder of a short-path game's project its files are in, and the
/// name of their layer: the page's files under it are laid over them.
const PAGE_FOLDER: &str = "files";

/// `game` with the page's files (`page`, [`page::page_layer`]) laid over its
/// own when its data loads with them, and what the page hears: `""` then,
/// every problem one a line when it doesn't (and `game` plays its own
/// files), and nothing when the page holds no file.
fn lay_page<A: ActionSet, T: Debug + 'static>(
    game: &BasicGame<A, T>,
    page: Result<Option<Layer>, page::PageError>,
) -> (BasicGame<A, T>, Option<String>) {
    let layer = match page {
        Ok(Some(layer)) => layer,
        Ok(None) => return (*game, None),
        Err(error) => return (*game, Some(error.to_string())),
    };
    let over = game.over(layer);
    match over.ctx() {
        Ok(_) => (over, Some(String::new())),
        Err(problems) => (*game, Some(problems)),
    }
}

/// Runs the dialogue preview of a game on the short path until it quits, as
/// [`run_game`] runs the game: its window, its files and its settings, with
/// the engine's preview as its first screen (`BasicGame::preview_start`),
/// played as `preview` says. The scenes are the `.dlg` files of the folder
/// at `folder`, read with [`read_watched_folder`] and read again a second
/// after one is saved, in an optimized build too. A folder that can't be
/// read shows why in the window, as a file that doesn't load does. A `.dlg`
/// file dropped on the window joins the folder. The web has no folders:
/// there the preview starts with none, and the writer drops their scenes
/// on the page.
///
/// A visual novel hands its writer a second binary of one call:
/// `run_dialogue_preview(game(), Path::new("scenes"), MyPreview)`.
pub fn run_dialogue_preview<A, T, P>(game: BasicGame<A, T>, folder: &Path, preview: P)
where
    A: ActionSet,
    T: Debug + 'static,
    P: PreviewGame<BasicData<A, T>>,
{
    let scenes = preview_folder(folder);
    let game = watched_game(&game).unwrap_or(game);
    let _ = shuyi_assets::bundle::install(game.bundle());
    let window = basic_window(&game);
    let start = game.preview_start(scenes, preview);
    macroquad::Window::from_config(window_conf(&window), run(basic_config(&game), start));
}

/// The name of the layer a dialogue preview reads its folder into.
const PREVIEW_LAYER: &str = "scenes";

/// The folder at `path`, read to be watched in every build, or why it
/// can't be read.
#[cfg(not(target_arch = "wasm32"))]
fn preview_folder(path: &Path) -> Result<Layer, String> {
    let limit = u64::try_from(shuyi_assets::format::DEFAULT_MAX_BYTES).unwrap_or(u64::MAX);
    read_watched_folder(path, PREVIEW_LAYER, limit).map_err(|e| e.to_string())
}

/// On the web, which has no folders: an empty layer, which the files the
/// writer drops on the page join.
#[cfg(target_arch = "wasm32")]
fn preview_folder(_: &Path) -> Result<Layer, String> {
    Layer::owned(PREVIEW_LAYER.to_owned(), std::collections::BTreeMap::new())
        .map_err(|e| e.to_string())
}

/// The margin around a short-path game's console, in pixels.
const BASIC_MARGIN: i32 = 16;

/// The window of a game on the short path: its console at twice its size
/// and a small margin, titled with its name (`AppInfo::name`), with its
/// icon (`AppInfo::window_icon`).
fn basic_window<A: ActionSet, T: Debug + 'static>(game: &BasicGame<A, T>) -> Window {
    let console = game.console_spec();
    let side = |cells: u16, cell: u16| i32::from(cells) * i32::from(cell) * 2 + BASIC_MARGIN;
    Window::new(
        game.name(),
        side(console.cols, console.cell_w),
        side(console.rows, console.cell_h),
    )
    .with_icon(game.app().window_icon)
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
/// storage's name (`AppInfo::storage`), its files from the installed bundle and its font's
/// picture, and its version in its crash reports (`AppInfo::version`); no
/// sounds, no voices and no system keyboard.
fn basic_config<A: ActionSet, T: Debug + 'static>(game: &BasicGame<A, T>) -> AppConfig {
    let sounds = Files::new(&no_sound, &sound_shown, &sound_left_out, "music");
    let mut config = AppConfig::new(
        basic_window(game),
        game.storage_name(),
        installed_file,
        FONT_PNG,
        sounds,
    );
    config.crash = CrashReports::new(game.app().version);
    if game.records_play_in_release() {
        config.crash = config.crash.with_record_in_release();
    }
    config
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
    use shuyi_platform::AppInfo;

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
        let app = AppInfo::new("A game", "com.example.a-game", "0.1.0").with_storage("a-game");
        BasicGame::new(app, files, |_| unreachable!("never started"))
    }

    /// The blank template's files, a short-path game's own.
    static TEMPLATE: &[(&str, &[u8])] = &[
        (
            "en.ron",
            include_bytes!("../../../templates/shuyi-default/base/crates/game/files/en.ron"),
        ),
        (
            "font/atlas.png",
            include_bytes!(
                "../../../templates/shuyi-default/base/crates/game/files/font/atlas.png"
            ),
        ),
        (
            "font/atlas.ron",
            include_bytes!(
                "../../../templates/shuyi-default/base/crates/game/files/font/atlas.ron"
            ),
        ),
        (
            "keymap.ron",
            include_bytes!("../../../templates/shuyi-default/base/crates/game/files/keymap.ron"),
        ),
        (
            "palette.ron",
            include_bytes!("../../../templates/shuyi-default/base/crates/game/files/palette.ron"),
        ),
    ];

    /// The blank template's game, whose files load.
    fn template() -> BasicGame<shuyi_assets::keymap::UiAction> {
        let files = shuyi_assets::bundle::Layer::table(PAGE_FOLDER, TEMPLATE);
        let app = AppInfo::new("A game", "com.example.a-game", "0.1.0");
        BasicGame::new(app, files, |_| unreachable!("never started"))
    }

    /// A page's layer of one file, `files/<path>`.
    fn page_file(path: &str, text: &str) -> Layer {
        let files = std::collections::BTreeMap::from([(path.to_owned(), text.as_bytes().to_vec())]);
        Layer::owned(PAGE_FOLDER.to_owned(), files).unwrap()
    }

    /// The template's palette with its `text` colour changed to `colour`.
    fn palette_with_text(colour: &str) -> String {
        let own = core::str::from_utf8(TEMPLATE[4].1).unwrap();
        let line = own
            .lines()
            .find(|line| line.trim_start().starts_with("\"text\":"))
            .expect("the template's palette has a text colour");
        own.replace(line, &format!("    \"text\": \"{colour}\","))
    }

    #[test]
    fn a_page_with_no_files_leaves_the_game_alone_and_hears_nothing() {
        let (game, heard) = lay_page(&template(), Ok(None));
        assert_eq!(heard, None);
        assert_eq!(
            game.bundle().file("palette.ron"),
            template().bundle().file("palette.ron")
        );
    }

    #[test]
    fn the_pages_files_that_load_are_played_and_the_page_hears_the_empty_text() {
        let palette = palette_with_text("#5a9bff");
        let (game, heard) = lay_page(&template(), Ok(Some(page_file("palette.ron", &palette))));
        assert_eq!(heard.as_deref(), Some(""));
        assert_eq!(game.bundle().file("palette.ron"), Some(palette.as_str()));
        assert_eq!(
            game.bundle().display_path("palette.ron"),
            "files/palette.ron"
        );
        assert!(game.bundle().has("keymap.ron"));
    }

    #[test]
    fn broken_page_files_are_reported_by_file_and_line_and_the_game_plays_its_own() {
        let palette = palette_with_text("blue");
        let (game, heard) = lay_page(&template(), Ok(Some(page_file("palette.ron", &palette))));
        let heard = heard.unwrap();
        assert!(heard.starts_with("files/palette.ron:"), "{heard}");
        assert!(heard.contains("blue"), "{heard}");
        assert_eq!(
            game.bundle().file("palette.ron"),
            template().bundle().file("palette.ron")
        );
    }

    /// A check of the palette's `text` colour: passes when it is blue.
    fn text_is_blue(data: &BasicData<shuyi_assets::keymap::UiAction>) -> Result<String, String> {
        match data.palette_def().get("text") {
            Some([0x5a, 0x9b, 0xff]) => Ok("the text is blue".to_owned()),
            other => Err(format!("the text is {other:?}")),
        }
    }

    /// The checks the tests' game has.
    const CHECKS: &[PageCheck<shuyi_assets::keymap::UiAction, ()>] = &[("text-blue", text_is_blue)];

    #[test]
    fn a_page_that_asks_no_check_gets_no_result() {
        assert_eq!(page_checked(&template(), Some(""), None, CHECKS), None);
        assert_eq!(page_checked(&template(), None, None, CHECKS), None);
    }

    #[test]
    fn the_check_the_page_asks_for_runs_on_its_files() {
        let (game, heard) = lay_page(
            &template(),
            Ok(Some(page_file(
                "palette.ron",
                &palette_with_text("#5a9bff"),
            ))),
        );
        let report = page_checked(&game, heard.as_deref(), Some("text-blue".into()), CHECKS);
        assert_eq!(
            report,
            Some(page::CheckReport::new(
                "text-blue",
                true,
                "the text is blue"
            ))
        );
    }

    #[test]
    fn with_no_page_files_the_check_runs_on_the_games_own() {
        let (game, heard) = lay_page(&template(), Ok(None));
        let report = page_checked(&game, heard.as_deref(), Some("text-blue".into()), CHECKS)
            .expect("a result");
        assert!(!report.passed);
        assert!(report.text.starts_with("the text is"), "{report:?}");
    }

    #[test]
    fn page_files_with_problems_get_no_check_result() {
        let (game, heard) = lay_page(
            &template(),
            Ok(Some(page_file("palette.ron", &palette_with_text("blue")))),
        );
        assert!(heard.as_deref().is_some_and(|text| !text.is_empty()));
        assert_eq!(
            page_checked(&game, heard.as_deref(), Some("text-blue".into()), CHECKS),
            None
        );
    }

    #[test]
    fn a_check_the_game_lacks_is_reported_failed_naming_it() {
        let report = page_checked(&template(), None, Some("walls-red".into()), CHECKS);
        assert_eq!(
            report,
            Some(page::CheckReport::new(
                "walls-red",
                false,
                "This game has no check named `walls-red`."
            ))
        );
    }

    #[test]
    fn page_files_that_cannot_be_read_are_reported_and_the_game_plays_its_own() {
        let error = page::PageError::TooMuch { limit: 7 };
        let (game, heard) = lay_page(&template(), Err(error));
        assert_eq!(
            heard.as_deref(),
            Some("the page's files are over 7 bytes in all")
        );
        assert_eq!(
            game.bundle().file("palette.ron"),
            template().bundle().file("palette.ron")
        );
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
    fn a_short_path_games_crash_reports_name_its_version() {
        assert_eq!(basic_config(&basic()).crash, CrashReports::new("0.1.0"));
        assert_eq!(basic_config(&basic()).crash.game_version, "0.1.0");
    }

    #[test]
    fn a_short_path_game_that_says_so_keeps_its_record_in_release() {
        let crash = basic_config(&basic()).crash;
        assert!(!crash.record_in_release);
        assert_eq!(crash.records_play(), shuyi_ui::DEBUG_TOOLS);
        let crash = basic_config(&basic().crash_record_in_release()).crash;
        assert_eq!(crash, CrashReports::new("0.1.0").with_record_in_release());
        assert!(crash.records_play());
    }

    #[test]
    fn a_build_without_debug_tools_prints_and_keeps_warnings_and_errors_only() {
        assert!(kept_in_this_build(LogLevel::Error));
        assert!(kept_in_this_build(LogLevel::Warn));
        assert_eq!(kept_in_this_build(LogLevel::Info), shuyi_ui::DEBUG_TOOLS);
        assert_eq!(kept_in_this_build(LogLevel::Debug), shuyi_ui::DEBUG_TOOLS);
    }

    #[test]
    fn the_panic_at_a_frame_is_read_as_the_reports_record_play() {
        // `cargo test` builds with debug tools, so both read it; a release
        // build reads it only when the game said.
        assert!(CrashReports::new("").records_play());
        assert!(
            CrashReports::new("")
                .with_record_in_release()
                .records_play()
        );
        assert!(!CrashReports::off().record_in_release);
        let off = CrashReports {
            record_in_release: false,
            ..CrashReports::new("")
        };
        assert_eq!(off.records_play(), shuyi_ui::DEBUG_TOOLS);
    }

    /// A config built in a `const fn`, as `AppConfig::new` stays one.
    const fn const_config(sounds: Files<'static>) -> AppConfig {
        AppConfig::new(
            Window::new("A game", 640, 480),
            "a-game",
            installed_file,
            FONT_PNG,
            sounds,
        )
    }

    #[test]
    fn a_new_config_writes_reports_with_no_version_and_off_keeps_none() {
        let sounds = Files::new(&no_sound, &sound_shown, &sound_left_out, "music");
        let config = const_config(sounds);
        assert_eq!(config.crash, CrashReports::default());
        assert_eq!((config.crash.game_version, config.crash.keep), ("", 5));
        assert_eq!(CrashReports::off().keep, 0);
    }

    #[test]
    fn a_short_path_games_storage_is_its_ids_unless_it_says() {
        let files = shuyi_assets::bundle::Layer::table("files", &[]);
        let app = AppInfo::new("A game", "com.example.a-game", "0.1.0");
        let game: BasicGame<shuyi_assets::keymap::UiAction> =
            BasicGame::new(app, files, |_| unreachable!("never started"));
        assert_eq!(basic_config(&game).storage_name, "com.example.a-game");
        assert_eq!(basic_window(&game).title, "A game");
    }

    #[test]
    fn a_short_path_games_window_has_its_apps_icon() {
        let files = shuyi_assets::bundle::Layer::table("files", &[]);
        let icon = Icon::new(b"small", b"medium", b"big");
        let app =
            AppInfo::new("A game", "com.example.a-game", "0.1.0").with_window_icon(Some(icon));
        let game: BasicGame<shuyi_assets::keymap::UiAction> =
            BasicGame::new(app, files, |_| unreachable!("never started"));
        assert_eq!(basic_window(&game).icon, Some(icon));
        assert_eq!(basic_config(&game).window.icon, Some(icon));
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
