//! The scene camera (the game's ticket 0228, moved here in 1154): a
//! picture (a whole map, [`MapPicture`](crate::grid::MapPicture)) as a
//! backdrop (game ADR-0048) behind a window of the console, panned by the pixel
//! and zoomed in whole steps, with a caption and a text box over it. For
//! checking by eye that a renderer keeps the scene sharp and seamless.
//!
//! The game gives the picture, where the window and the box go, and what
//! to say when it has no picture. The screen says what it shows as a
//! [`SceneCameraView`] and [`glyph::paint_camera`](super::glyph::paint_camera)
//! draws it.

use crate::prelude::*;
use alloc::rc::Rc;

use shuyi_assets::keymap::{ActionSet, UiAction};

use crate::audio::MenuSound;
use crate::cinematic::PanFrame;
use crate::ctx::{Ctx, GameData};
use crate::frame::cinema::{self, MAX_ZOOM};
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{GlyphBuffer, Rect};
use crate::grid::picture::pixel_size;
use crate::input::help_keys::{cursor_keys_name, help_line, key_name};
use crate::screen::{FrameInput, Screen, Transition};

/// How fast a held cursor key pans, in scene pixels a second.
pub const PAN_SPEED: f32 = 60.0;

/// The lines in the text box.
// check-text: not player text (debug tools are English only)
pub const BOX_TEXT: [&str; 3] = ["A text box drawn", "over the scene", "hides it."];

/// Where the scene camera's window and text box go on the console.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraLayout {
    /// The cells the scene shows through.
    pub window: Rect,
    /// The text box drawn over part of the window: at least 20 cells
    /// across and 5 down, for [`BOX_TEXT`].
    pub text_box: Rect,
}

/// What the scene camera shows.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneCameraView {
    /// Its title.
    pub title: String,
    /// The line under it: the zoom and where the window starts, or what
    /// is missing.
    pub info: String,
    /// The help line.
    pub help: String,
    /// Where the window and the text box go.
    pub layout: CameraLayout,
    /// The picture behind the window, and where it is; `None` without
    /// one, and then no window is drawn.
    pub shown: Option<(Rc<GlyphBuffer>, PanFrame)>,
    /// The text over the scene, with no box behind it.
    pub caption: String,
    /// The lines in the text box.
    pub box_text: Vec<String>,
}

/// Shows a picture through a window: the cursor actions pan (held),
/// Confirm steps the zoom 1× to [`MAX_ZOOM`] and round again, Cancel
/// closes.
#[derive(Debug, Clone)]
pub struct SceneCameraScreen {
    /// The picture; `None` if the game has none to show.
    picture: Option<Rc<GlyphBuffer>>,
    /// What to say without it.
    missing: String,
    /// Where the window and the text box go.
    layout: CameraLayout,
    /// The window's size in console pixels.
    window_px: (u32, u32),
    /// The scene pixel in the middle of the window.
    centre: (f32, f32),
    /// Console pixels per scene pixel.
    zoom: u8,
}

impl SceneCameraScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "scene_camera";

    /// The camera at 1× over the middle of `picture`, with `layout` on a
    /// console of `console`'s cells; `missing` is what it says without a
    /// picture.
    pub fn new(
        console: ConsoleSpec,
        picture: Option<Rc<GlyphBuffer>>,
        layout: CameraLayout,
        missing: String,
    ) -> Self {
        let (w, h) = picture.as_deref().map_or((0, 0), pixel_size);
        #[allow(clippy::cast_precision_loss)] // pixel sizes are small
        let centre = (w as f32 / 2.0, h as f32 / 2.0);
        let px = |cells: i32, cell: u16| u32::try_from(cells).unwrap_or(0) * u32::from(cell);
        Self {
            picture,
            missing,
            layout,
            window_px: (
                px(layout.window.w, console.cell_w),
                px(layout.window.h, console.cell_h),
            ),
            centre,
            zoom: 1,
        }
    }

    /// The picture shown, if there is one.
    pub fn picture(&self) -> Option<&Rc<GlyphBuffer>> {
        self.picture.as_ref()
    }

    /// The window's size in console pixels.
    pub fn window_px(&self) -> (u32, u32) {
        self.window_px
    }

    /// The scene pixel in the middle of the window.
    pub fn centre(&self) -> (f32, f32) {
        self.centre
    }

    /// Console pixels per scene pixel: 1 to [`MAX_ZOOM`].
    pub fn zoom(&self) -> u8 {
        self.zoom
    }

    /// The scene pixel at the window's top-left ([`cinema::view`]).
    pub fn origin_px(&self) -> (f32, f32) {
        let scene = self.picture.as_deref().map_or((0, 0), pixel_size);
        cinema::view(scene, self.window_px, self.centre, self.zoom)
    }

    /// Moves the centre by `(dx, dy)` scene pixels, kept to the middle of
    /// what the window can show: holding a key at an edge doesn't run the
    /// centre away from it. Along an axis the whole scene fits in, that is
    /// the scene's middle.
    fn pan(&mut self, dx: f32, dy: f32) {
        let scene = self.picture.as_deref().map_or((0, 0), pixel_size);
        let clip = self.window_px;
        let zoom = f32::from(self.zoom);
        #[allow(clippy::cast_precision_loss)] // pixel sizes are small
        let axis = |centre: f32, scene: u32, clip: u32| {
            let (half, middle) = (clip as f32 / zoom / 2.0, scene as f32 / 2.0);
            centre.clamp(half.min(middle), (scene as f32 - half).max(middle))
        };
        self.centre = (
            axis(self.centre.0 + dx, scene.0, clip.0),
            axis(self.centre.1 + dy, scene.1, clip.1),
        );
    }

    /// What the camera shows, naming the keys of the device used last.
    pub fn view<G: GameData>(&self, ctx: &Ctx<G>) -> SceneCameraView {
        let km = ctx.help_keys();
        let key = |ui| G::Action::for_ui(ui).map(|a| key_name(km, a));
        // check-text: not player text (debug tools are English only)
        let help = help_line(&[
            (Some(cursor_keys_name(km)), "pan (hold)"),
            (key(UiAction::Confirm), "zoom"),
            (key(UiAction::Cancel), "back"),
        ]);
        let (x, y) = self.origin_px();
        let frame = PanFrame {
            origin_px: (x, y),
            zoom: self.zoom,
        };
        let info = match self.picture {
            // check-text: not player text (debug tools are English only)
            Some(_) => format!("zoom {}x   window starts at {x:.1},{y:.1}", self.zoom),
            None => self.missing.clone(),
        };
        SceneCameraView {
            // check-text: not player text (debug tools are English only)
            title: "Scene camera (debug)".to_owned(),
            info,
            help,
            layout: self.layout,
            shown: self.picture.as_ref().map(|p| (Rc::clone(p), frame)),
            // check-text: not player text (debug tools are English only)
            caption: "Text over the scene".to_owned(),
            box_text: BOX_TEXT.map(str::to_owned).to_vec(),
        }
    }
}

impl<G: GameData> Screen<G> for SceneCameraScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        for &action in &input.actions {
            match action.ui() {
                Some(UiAction::Cancel) => {
                    ctx.audio.menu(MenuSound::Cancel);
                    return Transition::Pop;
                }
                Some(UiAction::Confirm) => {
                    ctx.audio.menu(MenuSound::Select);
                    self.zoom = self.zoom % MAX_ZOOM + 1;
                    self.pan(0.0, 0.0);
                }
                _ => {}
            }
        }
        let step = if input.dt.is_finite() {
            PAN_SPEED * input.dt.max(0.0)
        } else {
            0.0
        };
        let held = |ui| {
            let down = G::Action::for_ui(ui).is_some_and(|a| input.is_held(a));
            f32::from(u8::from(down))
        };
        let dx = held(UiAction::Right) - held(UiAction::Left);
        let dy = held(UiAction::Down) - held(UiAction::Up);
        self.pan(dx * step, dy * step);
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        super::glyph::paint_camera(&ctx.palette, &self.view(ctx), buf);
    }

    fn as_any(&self) -> Option<&dyn core::any::Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests;
