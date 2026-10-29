//! The capture mode (the `capture` feature, ADR-0052): the game run as a
//! plan (`shuyi_platform::capture::Plan`) says, frame by frame, with the
//! window's pixels of the frames it names compared with the raster's
//! picture of the same frame (`shuyi_ui::frame::raster`).
//!
//! [`run`](crate::run) reads `SHUYI_CAPTURE` on native: the path of a
//! plan. Unset, nothing changes. Set, the run follows the plan, then exits
//! ([`PASSED`], [`FAILED`] or [`CANNOT_RUN`]):
//!
//! - **exact frames:** every frame is `App::frame_after` one
//!   `capture::FRAME_DT` after the last, from the first, so the time, the
//!   fixed steps and the game's seed are the same on every run;
//! - **only the plan plays:** the keyboard, controllers, the window's
//!   shortcuts and focus aren't read; the close button still quits;
//! - **nothing outside:** storage is in memory (no save is read or
//!   written), the audio is silent, no voice manifest is read;
//! - **a capture:** the frame's `Show::Game` is painted by the raster too,
//!   at the scale `console::layout` gives the window, and compared with
//!   the console cut out of the window within the plan's tolerance.
//!
//! What it writes, under `SHUYI_CAPTURE_OUT` (`target/capture` unless
//! set), is [`Report`]'s. Each capture also logs its line, after
//! [`PREFIX`].
//!
//! On the web (ticket 2019) a build with the feature always runs the plan
//! [`WEB_PLAN`], beside the page (fetched as macroquad loads a file). It
//! writes no files: the console's lines are the report. The canvas is read
//! as the window is, in WebGL. After the last capture, or the first that
//! fails, or when the plan can't run, it logs [`DONE`] and keeps showing
//! that frame, for a browser driven by a script to see
//! (`tools/web-check/check.py`).
//!
//! The `render_card` example captures the render card
//! (`shuyi_ui::frame::raster::card`) the same way, with no game: the
//! pieces it uses are [`window_layout`], [`read_window`], [`report`] and
//! [`finish`].

use std::path::{Path, PathBuf};

use macroquad::prelude::*;
use shuyi_platform::capture::{FRAME_DT, Plan, PlannedFrame};
use shuyi_platform::{
    App, DisplayInfo, Frame, Look, Platform, Show, Silent, Start, Storage, SystemEvent, TextInput,
};
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::console::{Layout, layout};
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::frame::raster::{self, Comparison, Painter, Tolerance};
use shuyi_ui::storage::MemoryStorage;
use shuyi_ui::{GameData, RawInputEvent};

use crate::AppConfig;
use crate::backend::Macroquad;
use crate::readback;

/// The variable naming the plan to run.
pub const PLAN_VAR: &str = "SHUYI_CAPTURE";
/// The plan a web build runs: the file beside the page.
pub const WEB_PLAN: &str = "capture.ron";
/// What every line the capture mode logs starts with.
pub const PREFIX: &str = "shuyi-capture: ";
/// The line logged on the web when the run is over, whatever its outcome.
pub const DONE: &str = "shuyi-capture: done";
/// The variable naming the folder the captures are written to.
pub const OUT_VAR: &str = "SHUYI_CAPTURE_OUT";
/// The folder the captures are written to unless [`OUT_VAR`] says.
pub const DEFAULT_OUT: &str = "target/capture";

/// The exit code when every capture passed.
pub const PASSED: i32 = 0;
/// The exit code when a capture failed.
pub const FAILED: i32 = 1;
/// The exit code when the plan can't run: it doesn't parse, the game
/// fails to load, a capture falls on a frame that isn't the game's, or
/// the window can't be read or its files written.
pub const CANNOT_RUN: i32 = 2;

/// Whether the run stops at the first capture that fails: on the web,
/// where the page keeps showing that frame.
const STOP_AT_FAILURE: bool = cfg!(target_arch = "wasm32");

/// The plan to run: on native the one [`PLAN_VAR`] names, if it is set;
/// on the web always [`WEB_PLAN`].
pub(crate) fn requested() -> Option<String> {
    if cfg!(target_arch = "wasm32") {
        Some(WEB_PLAN.to_owned())
    } else {
        std::env::var(PLAN_VAR).ok()
    }
}

/// Runs the game as the plan at `path` says, then [`finish`]es.
pub(crate) async fn run<G: GameData>(mut config: AppConfig, start: Start<G>, path: String) {
    let plan = match plan(&path).await {
        Ok(plan) => plan,
        Err(e) => return finish(cannot_run(&format!("{path}: {e}")), || ()).await,
    };
    match report() {
        Ok(report) => play(&mut config, start, &plan, report).await,
        Err(e) => finish(cannot_run(&e), || ()).await,
    }
}

/// The report of a run: on native, written into the folder [`OUT_VAR`]
/// names ([`DEFAULT_OUT`] unless set); on the web, the console's lines
/// alone ([`Report::console`]).
pub fn report() -> Result<Report, String> {
    if cfg!(target_arch = "wasm32") {
        return Ok(Report::console());
    }
    let out = std::env::var(OUT_VAR).map_or_else(|_| PathBuf::from(DEFAULT_OUT), PathBuf::from);
    Report::new(out)
}

/// Ends a run with `code`. On native the process exits with it. On the
/// web, where a page doesn't exit, it logs [`DONE`] and calls `redraw`
/// before each frame from then on, so the last frame stays on the page.
pub async fn finish(code: i32, mut redraw: impl FnMut()) {
    if cfg!(target_arch = "wasm32") {
        info!("{} (exit code {})", DONE, code);
        loop {
            redraw();
            next_frame().await;
        }
    }
    std::process::exit(code);
}

/// Logs why the plan can't run, and gives [`CANNOT_RUN`].
fn cannot_run(why: &str) -> i32 {
    error!("{}{}", PREFIX, why);
    CANNOT_RUN
}

/// The plan at `path`.
async fn plan(path: &str) -> Result<Plan, String> {
    let text = macroquad::file::load_string(path)
        .await
        .map_err(|e| format!("can't be read: {e}"))?;
    Plan::parse(&text)
}

/// Plays `plan`'s frames, then [`finish`]es with the exit code, the last
/// frame shown again on the web.
async fn play<G: GameData>(config: &mut AppConfig, start: Start<G>, plan: &Plan, report: Report) {
    let mut platform = Capturing {
        backend: Macroquad::new(config),
        files: config.files,
        font_png: config.font_png,
        tolerance: plan.tolerance(),
        frame: PlannedFrame::default(),
        painter: None,
        report,
        broken: None,
        silent: Silent,
    };
    let mut app = App::new(start);
    let code = play_frames(&mut app, &mut platform, plan).await;
    finish(code, || app.present_again(&mut platform)).await;
}

/// Plays `plan`'s frames and gives the exit code.
async fn play_frames<G: GameData>(app: &mut App<G>, platform: &mut Capturing, plan: &Plan) -> i32 {
    let frames = plan.frames();
    let captures = frames.iter().filter(|f| f.capture.is_some()).count();
    for frame in frames {
        platform.frame = frame;
        let outcome = app.frame_after(platform, FRAME_DT);
        for line in app.take_new_log() {
            crate::print_line(&line);
        }
        if let Some(errors) = app.errors() {
            return cannot_run(&format!("the game didn't load: {errors}"));
        }
        if let Some(broken) = &platform.broken {
            return cannot_run(broken);
        }
        let failed = platform.report.exit_code() == FAILED;
        if outcome == shuyi_platform::FrameOutcome::Quit || (STOP_AT_FAILURE && failed) {
            break;
        }
        next_frame().await;
    }
    let report = &platform.report;
    let stopped = STOP_AT_FAILURE && report.exit_code() == FAILED;
    if report.captured() < captures && !stopped {
        return cannot_run(&format!(
            "the game quit after {} of the plan's {captures} captures",
            report.captured()
        ));
    }
    info!("{}{}", PREFIX, report.summary());
    report.exit_code()
}

/// The macroquad backend in the capture mode: it shows the frames, and
/// takes nothing from outside but the window's size and close button.
struct Capturing {
    backend: Macroquad,
    files: fn(&str) -> Option<&'static [u8]>,
    font_png: &'static str,
    tolerance: Tolerance,
    /// The plan's frame being run.
    frame: PlannedFrame,
    /// The raster's painter, and the background it was made with.
    painter: Option<(Rgb, Painter)>,
    report: Report,
    /// Why the plan can't go on.
    broken: Option<String>,
    silent: Silent,
}

impl Capturing {
    /// Compares the window with the raster's picture of the frame just
    /// drawn, and writes them as `name`.
    fn capture(
        &mut self,
        name: &str,
        buffer: &GlyphBuffer,
        background: Rgb,
        look: Option<Look<'_>>,
    ) -> Result<(), String> {
        let look = look.ok_or("the game gave no font and pictures to paint with")?;
        if self
            .painter
            .as_ref()
            .is_none_or(|(bg, _)| *bg != background)
        {
            let painter = painter(look, background, self.files, self.font_png)?;
            self.painter = Some((background, painter));
        }
        let Some((_, painter)) = &self.painter else {
            return Err("no painter".to_owned());
        };
        let fit = window_layout(buffer);
        let reference = painter.render(buffer, fit.scale);
        let window = read_window(&fit, reference.width, reference.height)?;
        let comparison = raster::compare(&window, &reference, self.tolerance);
        self.report
            .add(name, &window, &reference, &comparison, fit.scale)?;
        Ok(())
    }
}

impl Platform for Capturing {
    fn display(&self) -> DisplayInfo {
        self.backend.display()
    }

    fn present(&mut self, frame: &Frame<'_>) {
        self.backend.present(frame);
        let Some(name) = self.frame.capture.take() else {
            return;
        };
        let captured = match frame.show {
            Show::Game {
                buffer,
                background,
                look,
                ..
            } => self.capture(&name, buffer, background, look),
            Show::Card(_) => Err("the loading card is on the screen, not the game".to_owned()),
            Show::Errors(_) => Err("the game's errors are on the screen".to_owned()),
            _ => Err("the frame doesn't show the game".to_owned()),
        };
        if let Err(e) = captured {
            self.broken = Some(format!("capture {name}: {e}"));
        }
    }

    fn poll_input(&mut self, out: &mut Vec<RawInputEvent>) {
        out.append(&mut self.frame.events);
    }

    fn system_events(&mut self, out: &mut Vec<SystemEvent>) {
        if is_quit_requested() {
            out.push(SystemEvent::QuitRequested);
        }
    }

    fn storage(&mut self) -> Box<dyn Storage> {
        Box::new(MemoryStorage::default())
    }

    fn audio(&mut self) -> &mut dyn shuyi_platform::Audio {
        &mut self.silent
    }

    fn text_input(&mut self) -> Option<&mut dyn TextInput> {
        None
    }
}

/// The raster's painter of `look`, its pictures decoded as the renderer
/// decodes its textures (`render::upload`), over `background`. A picture
/// that doesn't decode is left out, as the renderer leaves its texture
/// out.
fn painter(
    look: Look<'_>,
    background: Rgb,
    files: fn(&str) -> Option<&'static [u8]>,
    font_png: &str,
) -> Result<Painter, String> {
    let atlas = decode(files(font_png).unwrap_or_default())
        .map_err(|e| format!("font atlas image: {e}"))?;
    let images = look
        .images
        .ids()
        .filter_map(|id| Some((id, decode(files(id.path())?).ok()?)))
        .collect();
    Ok(Painter::new(look.font.clone(), atlas, images, background))
}

/// PNG file `png` decoded as the renderer decodes it.
fn decode(png: &[u8]) -> Result<raster::Image, String> {
    let image =
        Image::from_file_with_format(png, Some(ImageFormat::Png)).map_err(|e| e.to_string())?;
    Ok(raster::Image {
        width: u32::from(image.width),
        height: u32::from(image.height),
        rgba: image.bytes,
    })
}

/// Where the renderer puts `buffer`'s console in the window, in physical
/// pixels (`Renderer::draw`'s layout).
pub fn window_layout(buffer: &GlyphBuffer) -> Layout {
    let dpi = screen_dpi_scale();
    let (width, height) = (
        (screen_width() * dpi).round(),
        (screen_height() * dpi).round(),
    );
    layout(buffer.spec(), width, height)
}

/// The console the window shows at `fit`, `width` × `height` pixels, read
/// from the window as drawn so far this frame (the live link's read,
/// `readback::window`). Called after the frame is drawn and before
/// `next_frame` shows it.
pub fn read_window(fit: &Layout, width: u32, height: u32) -> Result<raster::Image, String> {
    let (bytes, window_w, window_h) = readback::window();
    #[allow(clippy::cast_possible_truncation)] // whole pixels, far below 2^31
    let at = (fit.offset_x as i64, fit.offset_y as i64);
    cut_out(&bytes, (window_w, window_h), at, (width, height))
}

/// The `size` pixels at `at` of a window read back bottom row first,
/// `window` pixels big: its rows turned upright and made opaque (the live
/// link's `readback::upright`), then cut out. An error if they aren't all
/// in the window.
pub fn cut_out(
    bottom_up: &[u8],
    window: (u32, u32),
    at: (i64, i64),
    size: (u32, u32),
) -> Result<raster::Image, String> {
    let (window_w, window_h) = window;
    let (width, height) = size;
    let fits = |at: i64, size: u32, room: u32| {
        at >= 0 && at.saturating_add(i64::from(size)) <= i64::from(room)
    };
    if !(fits(at.0, width, window_w) && fits(at.1, height, window_h)) {
        return Err(format!(
            "the console, {width} × {height} pixels at {}, {}, isn't all in the window of {window_w} × {window_h}",
            at.0, at.1
        ));
    }
    let row = window_w as usize * 4;
    if bottom_up.len() != row * window_h as usize {
        return Err(format!(
            "the window read back {} bytes, not {window_w} × {window_h} pixels",
            bottom_up.len()
        ));
    }
    let upright = readback::upright(bottom_up, row);
    let (left, top) = (
        usize::try_from(at.0).unwrap_or(0),
        usize::try_from(at.1).unwrap_or(0),
    );
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for line in upright.chunks_exact(row).skip(top).take(height as usize) {
        rgba.extend_from_slice(&line[left * 4..(left + width as usize) * 4]);
    }
    Ok(raster::Image {
        width,
        height,
        rgba,
    })
}

/// The captures so far. Each logs its line of `capture.txt` after
/// [`PREFIX`]: a pass at the info level, a failure at
/// the error level. A report in a folder also writes, for each,
/// `<name>.window.png` and `<name>.raster.png`, `<name>.diff.png` when it
/// fails, and its line of `capture.txt`.
#[derive(Debug)]
pub struct Report {
    /// The folder the files go in; none for the console alone.
    dir: Option<PathBuf>,
    lines: Vec<String>,
    failed: usize,
}

impl Report {
    /// A report written into `dir`, made if missing, with an empty
    /// `capture.txt`.
    pub fn new(dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        let report = Self {
            dir: Some(dir),
            ..Self::console()
        };
        report.write_lines()?;
        Ok(report)
    }

    /// A report that writes no files: its lines are logged alone, as on
    /// the web.
    pub fn console() -> Self {
        Self {
            dir: None,
            lines: Vec::new(),
            failed: 0,
        }
    }

    /// Adds the capture `name`: logs its line and, in a folder, writes the
    /// `window`'s picture, the raster's `reference` and, if `comparison`
    /// fails, its diff, and the line. `scale` is the window's. Gives
    /// whether it passed.
    pub fn add(
        &mut self,
        name: &str,
        window: &raster::Image,
        reference: &raster::Image,
        comparison: &Comparison,
        scale: u32,
    ) -> Result<bool, String> {
        self.write_png(&format!("{name}.window.png"), window)?;
        self.write_png(&format!("{name}.raster.png"), reference)?;
        let passes = comparison.passes();
        if !passes {
            self.failed += 1;
            self.write_png(&format!("{name}.diff.png"), &comparison.diff)?;
        }
        let line = line(name, window, reference, comparison, scale);
        if passes {
            info!("{}", console(&line));
        } else {
            error!("{}", console(&line));
        }
        self.lines.push(line);
        self.write_lines()?;
        Ok(passes)
    }

    /// How many captures were written.
    pub fn captured(&self) -> usize {
        self.lines.len()
    }

    /// [`PASSED`] if every capture passed, else [`FAILED`].
    pub fn exit_code(&self) -> i32 {
        if self.failed == 0 { PASSED } else { FAILED }
    }

    /// How many passed, and where they were written, as a line of the
    /// log.
    pub fn summary(&self) -> String {
        let passed = format!(
            "{} of {} captures passed",
            self.lines.len() - self.failed,
            self.lines.len()
        );
        match &self.dir {
            Some(dir) => format!("{passed}, in {}", dir.display()),
            None => passed,
        }
    }

    /// `image` as the PNG file `file` of the folder, if there is one.
    fn write_png(&self, file: &str, image: &raster::Image) -> Result<(), String> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        let png = readback::png(&image.rgba, image.width, image.height)?;
        write(&dir.join(file), &png)
    }

    /// `capture.txt`, a line per capture, if there is a folder.
    fn write_lines(&self) -> Result<(), String> {
        let Some(dir) = &self.dir else {
            return Ok(());
        };
        let mut text = self.lines.join("\n");
        if !text.is_empty() {
            text.push('\n');
        }
        write(&dir.join("capture.txt"), text.as_bytes())
    }
}

/// Writes `bytes` to `path`.
fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// A line of `capture.txt` as it is logged: after [`PREFIX`]. On the web
/// the console's lines are the report.
fn console(line: &str) -> String {
    format!("{PREFIX}{line}")
}

/// A capture's line of `capture.txt`: its name, its size and scale, the
/// pixels over the tolerance, the worst difference and the first such
/// pixel, and whether it passed.
fn line(
    name: &str,
    window: &raster::Image,
    reference: &raster::Image,
    comparison: &Comparison,
    scale: u32,
) -> String {
    let size = if comparison.same_size {
        format!("{} × {}", reference.width, reference.height)
    } else {
        format!(
            "window {} × {}, raster {} × {}",
            window.width, window.height, reference.width, reference.height
        )
    };
    let first = comparison
        .first
        .map_or_else(|| "none".to_owned(), |(x, y)| format!("{x}, {y}"));
    let tolerance = comparison.tolerance;
    let verdict = if comparison.passes() {
        "passed"
    } else {
        "FAILED"
    };
    format!(
        "{name}: {size} at scale {scale}; {} pixels over (channel {}, pixels {}); worst {}; first {first}; {verdict}",
        comparison.over, tolerance.channel, tolerance.pixels, comparison.worst
    )
}

#[cfg(test)]
mod tests;
