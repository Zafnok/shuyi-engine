//! The render card in the window (ticket 2392, ADR-0052 §6): the frame
//! with every kind of thing a frame draws (`shuyi_ui::frame::raster::card`),
//! drawn by the renderer, captured and compared with the raster's picture
//! of it, as a plan's capture is. No game is behind it.
//!
//! ```text
//! cargo run -p shuyi-app --example render_card --features capture
//! ```
//!
//! The window is three times the card's size, so the console's scale is 3
//! (on a display that isn't scaled). It writes `card.window.png`,
//! `card.raster.png`, `card.diff.png` when they differ, and `capture.txt`
//! under `SHUYI_CAPTURE_OUT` (`target/capture` unless set), and exits as a
//! plan's run does: 0 when the window matches the raster, 1 when it
//! doesn't, 2 when it can't run.
//!
//! On the web (ticket 2019; `crates/cli/web/README.md`, *Checked in CI*)
//! it writes nothing: it logs the capture's line to the browser's console,
//! then `shuyi-capture: done`, and keeps showing the card.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use image::ImageEncoder as _;
use image::codecs::png::PngEncoder;
use macroquad::prelude::{error, info, next_frame};
use shuyi_app::capture::{self, read_window, window_layout};
use shuyi_app::render::Renderer;
use shuyi_app::{Window, window_conf};
use shuyi_ui::frame::raster::card::{self, RenderCard};
use shuyi_ui::frame::raster::{self, Tolerance};

/// The window's size in console pixels: an odd scale, so no pixel's centre
/// falls on the edge of a picture's pixel.
const SCALE: i32 = 3;
/// Frames drawn before the one captured, so the window is up.
const FRAMES_BEFORE: u32 = 3;

/// The card's pictures as PNG files by path, encoded once: the renderer
/// reads its files through a `fn`.
static FILES: OnceLock<BTreeMap<&'static str, Vec<u8>>> = OnceLock::new();

fn main() {
    let card = card::build();
    let bounds = card.buffer.pixel_bounds();
    let window = Window::new(
        "Shuyi Engine's render card",
        bounds.w * SCALE,
        bounds.h * SCALE,
    );
    // What `#[macroquad::main]` expands to.
    macroquad::Window::from_config(window_conf(&window), show(card));
}

/// Draws and captures the card, then finishes with the run's code (on the
/// web, still drawing the card).
async fn show(card: RenderCard) {
    let mut renderer = None;
    let code = match draw_and_capture(&card, &mut renderer).await {
        Ok(code) => code,
        Err(e) => {
            error!("{}render card: {}", capture::PREFIX, e);
            capture::CANNOT_RUN
        }
    };
    capture::finish(code, || {
        if let Some(renderer) = &mut renderer {
            renderer.draw(&card.buffer);
        }
    })
    .await;
}

/// Draws the card for a few frames with a renderer it leaves in
/// `renderer`, compares the last with the raster's picture, writes the
/// report, and gives the exit code.
async fn draw_and_capture(
    card: &RenderCard,
    renderer: &mut Option<Renderer>,
) -> Result<i32, String> {
    let mut report = capture::report()?;
    let atlas_png = png(&card.atlas_image)?;
    let files = card
        .images
        .iter()
        .map(|(id, image)| Ok((id.path(), png(image)?)))
        .collect::<Result<_, String>>()?;
    FILES.get_or_init(|| files);
    let renderer = renderer.insert(Renderer::new(
        card.atlas.clone(),
        &atlas_png,
        card.clear,
        &card.table,
        file,
    )?);
    for _ in 0..FRAMES_BEFORE {
        renderer.draw(&card.buffer);
        next_frame().await;
    }
    renderer.draw(&card.buffer);
    let fit = window_layout(&card.buffer);
    let reference = card.painter().render(&card.buffer, fit.scale);
    let window = read_window(&fit, reference.width, reference.height)?;
    let comparison = raster::compare(&window, &reference, Tolerance::default());
    report.add("card", &window, &reference, &comparison, fit.scale)?;
    info!("{}render card: {}", capture::PREFIX, report.summary());
    Ok(report.exit_code())
}

/// The card's picture at `path`, as a PNG file.
fn file(path: &str) -> Option<&'static [u8]> {
    FILES.get()?.get(path).map(Vec::as_slice)
}

/// `image` as a PNG file.
fn png(image: &raster::Image) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(
            &image.rgba,
            image.width,
            image.height,
            image::ColorType::Rgba8,
        )
        .map_err(|e| format!("a picture didn't encode: {e}"))?;
    Ok(out)
}
