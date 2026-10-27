//! [`Macroquad`]: the [`Platform`] over macroquad, gilrs and quad-snd
//! (ADR-0008 §2): the window it presents frames in, the keyboard and the
//! controllers, the window's own shortcut and close button, the OS's data
//! folder or the browser's `localStorage`, the sound player, and the
//! clock frames are measured with, and the heap counter they read, its
//! memory tags and its snapshots, in debug builds. It keeps the frame cap
//! the last frame asked for, for [`run`](crate::run)'s pacer, and the input
//! of the web's skipped frames for the next that runs.
//!
//! The game's look, its sounds and its stick thresholds come from its data,
//! which loads in a frame of [`App`](shuyi_platform::App)'s: the renderer
//! is made from the first game frame's [`Look`], and the sound player and
//! the pads once the game has loaded ([`Macroquad::start`]).

use macroquad::prelude::*;
use shuyi_alloc_count::{global, tags};
use shuyi_assets::audio::Files;
use shuyi_assets::format::{DecodeWatch, Decoded, watch_decodes};
use shuyi_platform::heap;
use shuyi_platform::{
    AudioFrame, DisplayInfo, Frame, Look, Millis, Platform, Show, Storage, SystemEvent, TextInput,
};
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::frame_cap::FrameCap;
use shuyi_ui::perf::{HeapCounts, HeapSnapshot, MemoryReading, MemoryScopes, PerfClock, TagClass};
use shuyi_ui::{Game, GameData, RawInputEvent};

use crate::audio::{self, Audio, device};
use crate::pacing::Held;
use crate::pads::PadInput;
use crate::render::{Renderer, color, upload};
use crate::{AppConfig, keys, storage};

/// The sound player, once the game has loaded: before, there is nothing
/// to play.
#[derive(Default)]
pub(crate) struct Sound {
    player: Option<Audio<device::Macroquad>>,
    speaker: device::Macroquad,
}

impl shuyi_platform::Audio for Sound {
    fn music_playing(&self, now: Millis) -> Option<(&str, f64)> {
        self.player.as_ref()?.music_playing(now.as_secs_f64())
    }

    fn play(&mut self, frame: &AudioFrame<'_>, now: Millis) {
        let Some(player) = &mut self.player else {
            return;
        };
        let speaker = &mut self.speaker;
        player.set_volumes(speaker, frame.music_volume, frame.sound_volume);
        player.set_voice_volume(speaker, frame.voice_volume);
        player.play(speaker, frame.requests, frame.music, now.as_secs_f64());
        for warning in player.take_warnings() {
            warn!("audio: {}", warning);
        }
    }

    fn stop_all(&mut self) {
        if let Some(player) = &mut self.player {
            player.stop_all(&mut self.speaker);
        }
    }
}

/// The macroquad backend.
pub(crate) struct Macroquad {
    storage_name: &'static str,
    files: fn(&str) -> Option<&'static [u8]>,
    font_png: &'static str,
    /// What draws the game's frames, from the first one's look.
    renderer: Option<Renderer>,
    /// Why the game's frames can't be drawn (its font picture doesn't
    /// decode): shown instead of them.
    broken: Option<String>,
    /// The loading card's picture and its path, once decoded (`None` if it
    /// doesn't decode).
    card_picture: Option<(String, Option<Texture2D>)>,
    /// Whether the window fills the screen: it starts windowed.
    fullscreen: bool,
    /// The controllers, once the game's stick thresholds are known.
    pads: Option<PadInput>,
    sound: Sound,
    text_input: Option<Box<dyn TextInput>>,
    /// Whether [`start`](Self::start) has run.
    started: bool,
    /// The frame cap the last frame presented asked for, and its serial.
    frame_cap: (FrameCap, u32),
    /// The input of frames that didn't run the game, for the next that
    /// does ([`hold_input`](Self::hold_input)).
    held: Held,
    /// The live link's input for the next frame, after the devices'.
    #[cfg(all(feature = "link", not(target_arch = "wasm32")))]
    link_input: Vec<RawInputEvent>,
}

impl Macroquad {
    /// The backend for a game `config` describes. Closing the window asks
    /// the game first ([`SystemEvent::QuitRequested`]) on native, where a
    /// window can be closed.
    pub(crate) fn new(config: &mut AppConfig) -> Self {
        if !cfg!(target_arch = "wasm32") {
            prevent_quit();
        }
        Self {
            storage_name: config.storage_name,
            files: config.files,
            font_png: config.font_png,
            renderer: None,
            broken: None,
            card_picture: None,
            fullscreen: false,
            pads: None,
            sound: Sound::default(),
            text_input: config.text_input.take(),
            started: false,
            frame_cap: (FrameCap::DEFAULT, 0),
            held: Held::default(),
            #[cfg(all(feature = "link", not(target_arch = "wasm32")))]
            link_input: Vec::new(),
        }
    }

    /// Where the live link puts its input for the next frame: `poll_input`
    /// adds it after the keyboard's and the pads'.
    #[cfg(all(feature = "link", not(target_arch = "wasm32")))]
    pub(crate) fn link_input(&mut self) -> &mut Vec<RawInputEvent> {
        &mut self.link_input
    }

    /// Whether the game has loaded and [`start`](Self::start) has run.
    pub(crate) fn started(&self) -> bool {
        self.started
    }

    /// The frame cap the last frame presented asked for, and its serial.
    pub(crate) fn frame_cap(&self) -> (FrameCap, u32) {
        self.frame_cap
    }

    /// Reads the input and the system's events of a frame that doesn't run
    /// the game (the web's, before the cap's due time) and keeps them, in
    /// order, for the next frame that does: macroquad forgets a frame's key
    /// presses and releases at its end.
    pub(crate) fn hold_input(&mut self) {
        let (mut input, mut system) = (Vec::new(), Vec::new());
        self.poll_input(&mut input);
        self.system_events(&mut system);
        self.held.keep(input, system);
    }

    /// Readies what plays the loaded `game`: decodes its sounds (which takes
    /// frames on the web) and starts watching for controllers with its
    /// stick thresholds.
    pub(crate) async fn start<G: GameData>(
        &mut self,
        game: &Game<G>,
        sounds: &Files<'static>,
        voice_dir: String,
    ) {
        let data = &game.ctx().game;
        let player = Audio::load(
            &mut self.sound.speaker,
            data.audio_manifest().clone(),
            sounds,
            audio::platform_folder(sounds.music_dir),
            voice_dir,
            miniquad::date::now().to_bits(),
        )
        .await;
        self.sound.player = Some(player);
        self.pads = Some(PadInput::new(data.keymap_def().stick));
        self.started = true;
    }

    /// Draws a game frame with its look: the renderer is made the first
    /// time; if that fails the reason shows instead, from then on.
    fn draw_game(&mut self, buffer: &GlyphBuffer, background: Rgb, look: Option<Look<'_>>) {
        if self.renderer.is_none() && self.broken.is_none() {
            let made = match look {
                Some(look) => {
                    let png = (self.files)(self.font_png).unwrap_or_default();
                    Renderer::new(look.font.clone(), png, background, look.images, self.files)
                }
                None => Err("the game gave no font and pictures to draw with".to_owned()),
            };
            match made {
                Ok(renderer) => self.renderer = Some(renderer),
                Err(e) => {
                    error!("{}", e);
                    self.broken = Some(e);
                }
            }
        }
        if let Some(broken) = &self.broken {
            Self::draw_errors(broken);
        } else if let Some(renderer) = &mut self.renderer {
            renderer.set_clear(background);
            renderer.draw(buffer);
        }
    }

    /// Shows errors instead of the game (with macroquad's built-in font,
    /// since the game's may be what failed): asset errors, only reachable if
    /// a broken asset slipped past the game's tests, or a wrong command line.
    /// `App` logs a failed load, and [`draw_game`](Self::draw_game) a
    /// renderer that can't start.
    fn draw_errors(text: &str) {
        clear_background(BLACK);
        let mut y = 40.0;
        for line in crate::wrap(text, crate::ERROR_WIDTH).lines() {
            draw_text(line, 20.0, y, 22.0, RED);
            y += 24.0;
        }
    }

    /// Draws the loading card: its line in macroquad's built-in font, or its
    /// picture at the largest whole scale that fits, centred.
    fn draw_card(&mut self, card: &shuyi_platform::Card) {
        clear_background(color(card.background()));
        match card {
            shuyi_platform::Card::Text { line, ink, .. } => {
                let size = text_size();
                let measured = measure_text(line, None, size, 1.0);
                let x = (screen_width() - measured.width) / 2.0;
                let y = f32::midpoint(screen_height(), measured.offset_y);
                draw_text(line, x, y, f32::from(size), color(*ink));
            }
            shuyi_platform::Card::Picture { path, .. } => {
                if self.card_picture.as_ref().is_none_or(|(p, _)| p != path) {
                    let texture = (self.files)(path)
                        .ok_or_else(|| "not among the game's files".to_owned())
                        .and_then(upload);
                    if let Err(e) = &texture {
                        warn!("loading card: {}: {}", path, e);
                    }
                    self.card_picture = Some((path.clone(), texture.ok()));
                }
                if let Some((_, Some(texture))) = &self.card_picture {
                    draw_centred(texture);
                }
            }
            _ => {}
        }
    }
}

/// The card's text size: a twelfth of the window's height, within reason.
fn text_size() -> u16 {
    let size = (screen_height() / 12.0).clamp(16.0, 64.0);
    // Between 16 and 64, so it fits.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let size = size as u16;
    size
}

/// Draws `texture` centred at the largest whole scale that fits the window
/// (at least 1).
fn draw_centred(texture: &Texture2D) {
    let (w, h) = (texture.width(), texture.height());
    let fit = (screen_width() / w)
        .min(screen_height() / h)
        .floor()
        .max(1.0);
    let (dw, dh) = (w * fit, h * fit);
    let params = DrawTextureParams {
        dest_size: Some(vec2(dw, dh)),
        ..Default::default()
    };
    let (x, y) = ((screen_width() - dw) / 2.0, (screen_height() - dh) / 2.0);
    draw_texture_ex(texture, x.floor(), y.floor(), WHITE, params);
}

/// `x` window pixels as a whole number.
fn pixels(x: f32) -> u32 {
    // A window's size is positive and far below 2^32.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let pixels = x.max(0.0).round() as u32;
    pixels
}

impl Platform for Macroquad {
    fn display(&self) -> DisplayInfo {
        let dpi = screen_dpi_scale();
        DisplayInfo::new(pixels(screen_width() * dpi), pixels(screen_height() * dpi))
    }

    fn present(&mut self, frame: &Frame<'_>) {
        self.frame_cap = (frame.frame_cap, frame.frame_cap_serial);
        if frame.fullscreen != self.fullscreen {
            self.fullscreen = frame.fullscreen;
            set_fullscreen(self.fullscreen);
        }
        match frame.show {
            Show::Card(card) => self.draw_card(card),
            Show::Errors(text) => Self::draw_errors(text),
            Show::Game {
                buffer,
                background,
                look,
                ..
            } => self.draw_game(buffer, background, look),
            _ => clear_background(BLACK),
        }
    }

    fn poll_input(&mut self, out: &mut Vec<RawInputEvent>) {
        self.held.take_input(out);
        out.extend(keys::poll());
        if let Some(pads) = &mut self.pads {
            out.extend(pads.poll());
        }
        #[cfg(all(feature = "link", not(target_arch = "wasm32")))]
        out.append(&mut self.link_input);
    }

    fn system_events(&mut self, out: &mut Vec<SystemEvent>) {
        self.held.take_system(out);
        if keys::fullscreen_shortcut() {
            out.push(SystemEvent::ToggleFullscreen);
        }
        if is_quit_requested() {
            out.push(SystemEvent::QuitRequested);
        }
    }

    fn storage(&mut self) -> Box<dyn Storage> {
        let mut storage = storage::platform(self.storage_name);
        if cfg!(debug_assertions) {
            storage::smoke_check(&mut *storage);
        }
        storage
    }

    fn audio(&mut self) -> &mut dyn shuyi_platform::Audio {
        &mut self.sound
    }

    fn text_input(&mut self) -> Option<&mut dyn TextInput> {
        self.text_input
            .as_deref_mut()
            .map(|t| t as &mut dyn TextInput)
    }

    fn clock(&self) -> Option<PerfClock> {
        Some(PerfClock::Real(now_us))
    }

    /// The global heap counter's reading (`shuyi_alloc_count::global`):
    /// `None` in a release build, or one built with `--cfg
    /// shuyi_no_heap_count`.
    fn memory(&mut self) -> Option<MemoryReading> {
        let counts = global::reading()?;
        let heap = HeapCounts::new(counts.live_bytes, counts.peak_bytes, counts.allocs);
        Some(MemoryReading::new(heap))
    }

    /// The global heap counter's tags, in a build that counts the heap.
    fn memory_scopes(&self) -> Option<MemoryScopes> {
        memory_scopes()
    }

    /// The global heap counter's rows by tag, in a build that counts the
    /// heap.
    fn heap_snapshot(&mut self) -> Option<HeapSnapshot> {
        heap_snapshot()
    }
}

/// The global heap counter's snapshot (`shuyi_platform::heap::snapshot`),
/// in a build that counts the heap.
pub(crate) fn heap_snapshot() -> Option<HeapSnapshot> {
    let (counts, rows) = (global::reading()?, global::rows()?);
    Some(heap::snapshot(counts, &rows, tags::overflowed()))
}

/// The global heap counter's memory tags (`shuyi_alloc_count::tags`), in
/// a build that counts the heap (ADR-0046 §6).
pub(crate) fn memory_scopes() -> Option<MemoryScopes> {
    global::COUNTING.then(|| MemoryScopes::new(tags::tag, tags::enter, tags::leave))
}

/// In a build that counts the heap, tags what each asset decode allocates
/// with its kind ([`TagClass::Asset`]): called when the backend starts,
/// before the game's load. Another watch set first stays.
pub(crate) fn tag_decodes() {
    if global::COUNTING {
        let _ = watch_decodes(DecodeWatch::new(decode_before, decode_after));
    }
}

/// Enters the tag of the asset kind `kind`: the token is the tag before.
fn decode_before(kind: &'static str) -> u32 {
    u32::from(tags::enter(tags::tag(TagClass::Asset as u8, kind)))
}

/// Leaves the tag [`decode_before`] entered.
fn decode_after(token: u32, _: &Decoded<'_>) {
    tags::leave(u16::try_from(token).unwrap_or(tags::OTHER));
}

/// Microseconds since the start, from macroquad's clock (seconds as an
/// `f64`, on native and the web): `App` measures frames with it in debug
/// builds (ADR-0046).
fn now_us() -> u64 {
    let since = core::time::Duration::try_from_secs_f64(get_time());
    since.map_or(0, |d| u64::try_from(d.as_micros()).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_build_that_counts_the_heap_has_memory_tags() {
        assert_eq!(memory_scopes().is_some(), global::COUNTING);
    }

    /// Other test threads allocate too: only what holds whatever they do.
    #[test]
    fn a_build_that_counts_the_heap_takes_snapshots_by_tag() {
        let snapshot = heap_snapshot();
        assert_eq!(snapshot.is_some(), global::COUNTING);
        let Some(snapshot) = snapshot else {
            return;
        };
        assert!(snapshot.live_bytes > 0, "{snapshot:?}");
        assert!(
            snapshot.tags.iter().any(|r| r.name == "other"),
            "{snapshot:?}"
        );
        assert_eq!(snapshot.frame, 0);
    }

    /// What a decode allocates counts under its kind's tag, in a build
    /// that counts: a kind of this test's own, so no other thread adds to
    /// it.
    #[test]
    fn a_decode_counts_under_its_kind() {
        let Some(_) = memory_scopes() else {
            return;
        };
        let start = tags::current();
        let token = decode_before("backend-test kind");
        let block = vec![1u8; 4096];
        decode_after(token, &Decoded::new("backend-test kind", "a.png", 9, true));
        assert_eq!(tags::current(), start);
        let rows = global::rows().unwrap_or_default();
        let row = rows.iter().find(|r| r.name == "backend-test kind").unwrap();
        assert_eq!(row.class, TagClass::Asset as u8);
        assert_eq!(row.live_bytes, 4096);
        drop(block);
    }

    #[test]
    fn the_decode_watch_is_set_once_where_the_heap_is_counted() {
        tag_decodes();
        tag_decodes();
        let other = DecodeWatch::new(|_| 0, |_, _| {});
        assert_eq!(watch_decodes(other).is_err(), global::COUNTING);
    }

    #[test]
    fn other_is_the_counters_class_for_other_and_overflow() {
        assert_eq!(TagClass::Other as u8, tags::OTHER_CLASS);
        assert_eq!(TagClass::from_u8(tags::OTHER_CLASS), Some(TagClass::Other));
    }
}
