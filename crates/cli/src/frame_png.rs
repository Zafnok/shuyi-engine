//! `frame-png`: renders a frame of a game in the test Harness to a PNG
//! without a window, for looking at a skin, a picture or a mockup. Never a
//! gate: snapshots stay text.
//!
//! The command plays a [`Script`] of keys, controller buttons and waits in
//! the game's Harness, which the game's [`GameTools::frame`] starts with
//! the game's own options, and writes the picture the game paints of it.
//! A game paints a frame with [`Painter`], or, for a one-bit screen, with
//! the one-bit renderer and [`one_bit_picture`]. With `--target <id>`, a
//! target whose display is black and white gets the picture its device
//! shows ([`GameTools::frame_one_bit`]: the frame drawn in its roles'
//! patterns, ADR-0008 §4).
//!
//! [`Painter`] is a software copy of `shuyi-app`'s renderer, pixel for
//! pixel at whole scales: the clear colour, cell backgrounds, `Under`
//! items in order, glyphs tinted with their fg, `Over` items in order;
//! and before all that, a backdrop's scene in its window. Textures are
//! sampled at the nearest pixel and blended over what is beneath with
//! their alpha, as the GPU does.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::font::{AtlasRect, FALLBACK_GLYPH, FontAtlasDef};
use shuyi_assets::image::{ImageId, ImageTable};
use shuyi_assets::target::{Color, TargetProfile};
use shuyi_platform::harness::{FRAME_DT, Harness, MAX_FRAMES};
use shuyi_ui::GameData;
use shuyi_ui::frame::onebit::{self, Frame};
use shuyi_ui::frame::{Backdrop, GlyphBuffer, Item, Layer, Paint, PxRect, Rgb, Sprite};
use shuyi_ui::input::{Button, Chord};

use crate::codec::{decode_png, encode_png};
use crate::game::{GameArg, GameOption, GameTools};

/// Largest `--scale`.
pub const MAX_SCALE: u32 = 8;

/// The colour of what is missing, as in `shuyi-app` (macroquad's
/// `MAGENTA`).
const MISSING_COLOR: Rgb = Rgb::new(255, 0, 255);

/// The usage text, with `command` as the tool's name and the game's own
/// options and notes.
pub fn usage(command: &str, options: &[GameOption], notes: &str) -> String {
    let mut out = format!(
        "\
usage: {command} frame-png <out.png> [steps] [options]

Starts the game in the test Harness, runs the steps in the order given,
and writes the frame to <out.png> (a relative path is from the project's
root).

steps (each may repeat):
  --keys \"F2 Up f\"   press and release each key chord in turn
  --pad \"South\"      press and release each controller button in turn
                     (\"East+DpadDown\": those buttons together)
  --wait 0.5         let that many seconds pass

options:
  --scale N            whole-number scale, 1 to {MAX_SCALE} (the game says what by default)
  --target ID          as target ID's display shows it: on a one-bit one (the
                       Playdate), black and white in the skin's patterns
"
    );
    for option in options {
        let name = match option.value {
            Some(value) => format!("{} {value}", option.flag),
            None => option.flag.to_owned(),
        };
        let mut lines = option.help.lines();
        // Writing to a `String` can't fail.
        let _ = writeln!(out, "  {name:<20} {}", lines.next().unwrap_or_default());
        for line in lines {
            let _ = writeln!(out, "  {:<20} {line}", "");
        }
    }
    out.push_str(
        "
Key and button names are the Harness's test-script names (Chord::parse,
Button::parse): input to a test script, not game code, so the
never-hard-code-a-key rule doesn't apply to them.",
    );
    if !notes.is_empty() {
        out.push_str("\n\n");
        out.push_str(notes);
    }
    out
}

/// One step of the script, run in the order given.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Step {
    /// `Harness::keys`: each chord pressed and released in turn
    /// (`"Enter Up Shift+Space"`).
    Keys(String),
    /// `Harness::pad`: each controller button pressed and released in
    /// turn (`"South"`, `"East+DpadDown"`: those buttons together).
    Pad(String),
    /// `Harness::wait`: that many seconds pass, at most [`MAX_WAIT`] in
    /// one step.
    Wait(f32),
}

/// The longest wait of one step, in seconds: the Harness's most frames
/// in one wait.
#[allow(clippy::cast_precision_loss)] // 2000 frames
pub const MAX_WAIT: f32 = MAX_FRAMES as f32 * FRAME_DT;

impl Step {
    /// Whether the Harness can play it: names it knows, a wait it can
    /// run. A step that isn't would panic the Harness.
    pub fn check(&self) -> Result<(), String> {
        match self {
            Step::Keys(script) => {
                for token in script.split_whitespace() {
                    Chord::parse(token).map_err(|e| format!("keys: {e}"))?;
                }
            }
            Step::Pad(script) => {
                for button in script.split_whitespace().flat_map(|t| t.split('+')) {
                    Button::parse(button).map_err(|e| format!("pad: {e}"))?;
                }
            }
            Step::Wait(seconds) => {
                if !(seconds.is_finite() && (0.0..=MAX_WAIT).contains(seconds)) {
                    return Err(format!(
                        "wait: {seconds} is not a number of seconds from 0 to {MAX_WAIT:.1}"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// What to do in the game before taking the frame.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Script {
    /// The steps, in order.
    pub steps: Vec<Step>,
}

impl Script {
    /// Plays the steps in `h`.
    pub fn play<G: GameData>(&self, h: &mut Harness<G>) {
        for step in &self.steps {
            match step {
                Step::Keys(script) => h.keys(script),
                Step::Pad(script) => h.pad(script),
                Step::Wait(seconds) => h.wait(*seconds),
            };
        }
    }

    /// Whether the Harness can play every step ([`Step::check`]).
    pub fn check(&self) -> Result<(), String> {
        self.steps.iter().try_for_each(Step::check)
    }

    /// Reads the script flag `flag` with its value `value`, if it is one:
    /// `--keys` (or `--script`), `--pad` or `--wait`. A step the Harness
    /// can't play is an error, not a panic later.
    pub fn parse_step(flag: &str, value: &str) -> Option<Result<Step, String>> {
        let step = match flag {
            "--keys" | "--script" => Step::Keys(value.to_owned()),
            "--pad" => Step::Pad(value.to_owned()),
            "--wait" => match value.parse::<f32>() {
                Ok(seconds) => Step::Wait(seconds),
                Err(_) => {
                    return Some(Err(format!("--wait: not a number of seconds: {value}")));
                }
            },
            _ => return None,
        };
        Some(step.check().map(|()| step).map_err(|e| format!("--{e}")))
    }
}

/// The command's arguments.
#[derive(Debug, Clone, PartialEq)]
pub struct Options {
    /// Where the PNG goes.
    pub out: PathBuf,
    /// What to do before taking the frame.
    pub script: Script,
    /// Pixels per console pixel, if given.
    pub scale: Option<u32>,
    /// The target whose display to show it as, by its profile's id, if
    /// given.
    pub target: Option<String>,
    /// The game's own options, in the order given.
    pub game: Vec<GameArg>,
}

/// Parses `frame-png`'s arguments, the game's `options` among them,
/// checking key and button names up front so a typo is an error, not a
/// Harness panic.
pub fn parse_args(args: &[String], options: &[GameOption]) -> Result<Options, String> {
    let mut out = None;
    let mut parsed = Options {
        out: PathBuf::new(),
        script: Script::default(),
        scale: None,
        target: None,
        game: Vec::new(),
    };
    let steps = &mut parsed.script.steps;
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        match arg.as_str() {
            "--keys" | "--pad" | "--wait" => {
                let text = value()?;
                if let Some(step) = Script::parse_step(arg, &text) {
                    steps.push(step?);
                }
            }
            "--scale" => {
                let text = value()?;
                parsed.scale = Some(
                    text.parse()
                        .ok()
                        .filter(|s| (1..=MAX_SCALE).contains(s))
                        .ok_or_else(|| {
                            format!("--scale: a whole number 1 to {MAX_SCALE}, not {text}")
                        })?,
                );
            }
            "--target" => parsed.target = Some(value()?),
            flag if flag.starts_with("--") => {
                let option = options
                    .iter()
                    .find(|option| option.flag == flag)
                    .ok_or_else(|| format!("unknown option: {flag}"))?;
                let value = option.value.map(|_| value()).transpose()?;
                parsed.game.push(GameArg {
                    flag: flag.to_owned(),
                    value,
                });
            }
            path if out.is_none() => out = Some(PathBuf::from(path)),
            extra => return Err(format!("one output path only, not also {extra}")),
        }
    }
    parsed.out = out.ok_or("missing <out.png>")?;
    Ok(parsed)
}

/// What a game shows of a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shot {
    /// The picture.
    pub image: Image,
    /// What it shows, for the summary: the top screen's name.
    pub shows: String,
    /// What the summary warns about it, if anything (a picture of art the
    /// game may not hand out).
    pub warning: Option<String>,
}

/// Runs the command: the game plays the script and paints the frame, as
/// the display of `target` (the profile of [`Options::target`]) shows it,
/// which is written under `root` (if `out` is relative); returns what to
/// print.
pub fn run(
    root: &Path,
    options: &Options,
    game: &dyn GameTools,
    target: Option<&TargetProfile>,
) -> Result<String, String> {
    let (game_options, script, scale) = (&options.game, &options.script, options.scale);
    let shot = match target {
        Some(profile) if profile.display.color == Color::OneBit => {
            game.frame_one_bit(&profile.id, game_options, script, scale)?
        }
        _ => game.frame(game_options, script, scale)?,
    };
    let png = encode_png(shot.image.width, shot.image.height, &shot.image.rgba)?;
    let path = root.join(&options.out);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    fs::write(&path, png).map_err(|e| format!("writing {}: {e}", path.display()))?;
    let shown = format!("{} → {}", shot.shows, options.out.display());
    Ok(summary(&shown, &shot.image, shot.warning.as_deref()))
}

/// `frame` as a black and white picture, each pixel `scale` × `scale`.
pub fn one_bit_picture(frame: &Frame, scale: u32) -> Image {
    let (width, height) = (onebit::WIDTH.unsigned_abs(), onebit::HEIGHT.unsigned_abs());
    let mut image = Image::filled(width * scale, height * scale, Rgb::new(0, 0, 0));
    let s = i64::from(scale);
    let at = |v: i64| i32::try_from(v / s).ok();
    for y in 0..i64::from(image.height) {
        for x in 0..i64::from(image.width) {
            if at(x)
                .zip(at(y))
                .is_some_and(|(x, y)| onebit::is_white(frame, x, y))
            {
                image.blend(x, y, [1.0; 3], 1.0);
            }
        }
    }
    image
}

/// What `run` prints for frame `frame` of `shown` (screen → path), with
/// the game's `warning` about it, if any.
fn summary(shown: &str, frame: &Image, warning: Option<&str>) -> String {
    let mut out = format!("frame-png: {shown} ({}×{})", frame.width, frame.height);
    if let Some(warning) = warning {
        out.push_str("\nwarning: ");
        out.push_str(warning);
    }
    out
}

/// An RGBA8 picture, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Four bytes per pixel.
    pub rgba: Vec<u8>,
}

impl Image {
    /// A `width` × `height` picture of `color`, solid.
    fn filled(width: u32, height: u32, color: Rgb) -> Self {
        let mut image = Self {
            width,
            height,
            rgba: vec![0; width as usize * height as usize * 4],
        };
        image.fill((0, 0, i64::from(width), i64::from(height)), color);
        image
    }

    /// Decodes PNG file `png`.
    pub fn decode(png: &[u8]) -> Result<Self, String> {
        let (width, height, rgba) = decode_png(png)?;
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    /// The pixel at `(x, y)`, clamped to the picture (an empty picture is
    /// transparent).
    fn texel(&self, x: i64, y: i64) -> [u8; 4] {
        if self.width == 0 || self.height == 0 {
            return [0; 4];
        }
        let clamp =
            |v: i64, size: u32| usize::try_from(v.clamp(0, i64::from(size) - 1)).unwrap_or(0);
        let i = (clamp(y, self.height) * self.width as usize + clamp(x, self.width)) * 4;
        [
            self.rgba[i],
            self.rgba[i + 1],
            self.rgba[i + 2],
            self.rgba[i + 3],
        ]
    }

    /// The pixel at `(x, y)`, or `None` outside the picture.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        (x < self.width && y < self.height).then(|| self.texel(i64::from(x), i64::from(y)))
    }

    /// Blends `color` with alpha `alpha` (0 to 1) over the pixel at
    /// `(x, y)`, as the GPU does: in floats, rounded once. Off the picture
    /// it does nothing.
    fn blend(&mut self, x: i64, y: i64, color: [f32; 3], alpha: f32) {
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return;
        };
        if x >= self.width || y >= self.height {
            return;
        }
        let i = (y as usize * self.width as usize + x as usize) * 4;
        for (c, &src) in color.iter().enumerate() {
            let dst = f32::from(self.rgba[i + c]) / 255.0;
            let out = src * alpha + dst * (1.0 - alpha);
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // 0..=255
            let byte = (out * 255.0).round().clamp(0.0, 255.0) as u8;
            self.rgba[i + c] = byte;
        }
        self.rgba[i + 3] = 255;
    }

    /// Fills `(x, y, w, h)` with `color`, solid.
    fn fill(&mut self, (x, y, w, h): (i64, i64, i64, i64), color: Rgb) {
        for py in y..y + h {
            for px in x..x + w {
                self.blend(px, py, unit(color), 1.0);
            }
        }
    }
}

/// `c` as floats from 0 to 1.
fn unit(c: Rgb) -> [f32; 3] {
    [c.r, c.g, c.b].map(|v| f32::from(v) / 255.0)
}

/// The decoded font atlas and images, ready to render any number of
/// frames (each image is decoded once).
pub struct Painter {
    atlas: FontAtlasDef,
    atlas_image: Image,
    /// Background drawn behind the console; cells with this bg skip their
    /// fill, as in `app`.
    clear: Rgb,
    /// The images that decoded; the others draw as `MISSING_COLOR`.
    images: HashMap<ImageId, Image>,
}

impl Painter {
    /// Decodes atlas image `atlas_png` and every image in `images`, whose
    /// files `read` returns (an image `read` lacks, or that doesn't decode,
    /// draws as a `MISSING_COLOR` rectangle). `clear` is the console
    /// background (the palette's `black`).
    pub fn new<'a>(
        atlas: FontAtlasDef,
        atlas_png: &[u8],
        images: &ImageTable,
        read: impl Fn(&str) -> Option<&'a [u8]>,
        clear: Rgb,
    ) -> Result<Self, String> {
        let atlas_image = Image::decode(atlas_png).map_err(|e| format!("font atlas image: {e}"))?;
        let images = images
            .ids()
            .filter_map(|id| Some((id, Image::decode(read(id.path())?).ok()?)))
            .collect();
        Ok(Self {
            atlas,
            atlas_image,
            clear,
            images,
        })
    }

    /// Draws `buf` with each console pixel `scale` × `scale` pixels big.
    /// With a backdrop (game ADR-0048): the clear colour, the scene inside its
    /// window, then the console over it, as in `app`.
    pub fn render(&self, buf: &GlyphBuffer, scale: u32) -> Image {
        let s = i64::from(scale);
        let spec = buf.spec();
        let (cell_w, cell_h) = (i64::from(spec.cell_w) * s, i64::from(spec.cell_h) * s);
        let width = u32::try_from(i64::from(buf.width()) * cell_w).unwrap_or(0);
        let height = u32::try_from(i64::from(buf.height()) * cell_h).unwrap_or(0);
        let mut out = Image::filled(width, height, self.clear);
        if let Some(backdrop) = buf.backdrop() {
            self.draw_backdrop(&mut out, backdrop, scale);
        }
        // Over a scene, a cell in the clear colour must hide it.
        let all_fills = buf.backdrop().is_some();
        self.draw_layers(&mut out, buf, (0, 0), s, all_fills);
        out
    }

    /// Draws `backdrop`'s scene in its window of `out`: every scene pixel
    /// `zoom × scale` pixels big, placed by [`Backdrop::scene_offset`].
    /// The scene is drawn into a picture the size of the window, which
    /// clips it, and that is copied in.
    fn draw_backdrop(&self, out: &mut Image, backdrop: &Backdrop, scale: u32) {
        let s = i64::from(scale);
        let clip = backdrop.clip_px();
        let size = |v: i32| u32::try_from(i64::from(v) * s).unwrap_or(0);
        let mut window = Image::filled(size(clip.w), size(clip.h), self.clear);
        let scene_px = i64::from(backdrop.zoom()) * s;
        self.draw_layers(
            &mut window,
            backdrop.scene(),
            backdrop.scene_offset(scale),
            scene_px,
            false,
        );
        let (left, top) = (i64::from(clip.x) * s, i64::from(clip.y) * s);
        for y in 0..i64::from(window.height) {
            for x in 0..i64::from(window.width) {
                let [r, g, b, _] = window.texel(x, y);
                out.blend(left + x, top + y, unit(Rgb::new(r, g, b)), 1.0);
            }
        }
    }

    /// Draws `buf`'s cells and items into `out` with its top-left corner
    /// at `corner` and each of its pixels `px` × `px` big: cell
    /// backgrounds (all of them if `all_fills`, else all but those in the
    /// clear colour; never a see-through cell's), `Under` items, glyphs,
    /// `Over` items.
    fn draw_layers(
        &self,
        out: &mut Image,
        buf: &GlyphBuffer,
        corner: (i64, i64),
        px: i64,
        all_fills: bool,
    ) {
        let spec = buf.spec();
        let (cell_w, cell_h) = (i64::from(spec.cell_w) * px, i64::from(spec.cell_h) * px);
        let cells = || {
            (0..i32::from(buf.height())).flat_map(move |y| {
                (0..i32::from(buf.width())).filter_map(move |x| {
                    let at = (
                        corner.0 + i64::from(x) * cell_w,
                        corner.1 + i64::from(y) * cell_h,
                    );
                    buf.get(x, y).map(|cell| (at, cell))
                })
            })
        };
        for ((x, y), cell) in cells() {
            if !cell.see_through && (all_fills || cell.bg != self.clear) {
                out.fill((x, y, cell_w, cell_h), cell.bg);
            }
        }
        self.draw_items(out, buf, Layer::Under, corner, px);
        for ((x, y), cell) in cells() {
            if cell.glyph == ' ' {
                continue;
            }
            let Some((rect, fg)) = self.atlas_cell(cell.glyph, cell.fg) else {
                continue;
            };
            let fg = unit(fg);
            #[allow(clippy::cast_precision_loss)] // atlas coordinates are small
            let src = [rect.x, rect.y, rect.w, rect.h].map(|v| v as f32);
            blit(
                out,
                &self.atlas_image,
                src,
                (x, y, cell_w, cell_h),
                false,
                Paint::Image,
                [fg[0], fg[1], fg[2], 1.0],
            );
        }
        self.draw_items(out, buf, Layer::Over, corner, px);
    }

    /// Draws `buf`'s items of `layer`, in order, from `corner`, each of
    /// the buffer's pixels `scale` big.
    fn draw_items(
        &self,
        out: &mut Image,
        buf: &GlyphBuffer,
        layer: Layer,
        corner: (i64, i64),
        scale: i64,
    ) {
        for item in buf.items().iter().filter(|item| item.layer() == layer) {
            let to_px = |r: PxRect| {
                (
                    corner.0 + i64::from(r.x) * scale,
                    corner.1 + i64::from(r.y) * scale,
                    i64::from(r.w) * scale,
                    i64::from(r.h) * scale,
                )
            };
            match item {
                Item::Rect(rect) => out.fill(to_px(rect.rect), rect.color),
                Item::Sprite(sprite) => self.draw_sprite(out, sprite, to_px(sprite.clip)),
            }
        }
    }

    /// Draws the visible part of `sprite` into `dest`: the matching part
    /// of its image, or a `MISSING_COLOR` rectangle.
    fn draw_sprite(&self, out: &mut Image, sprite: &Sprite, dest: (i64, i64, i64, i64)) {
        let Some(image) = self.images.get(&sprite.image) else {
            out.fill(dest, MISSING_COLOR);
            return;
        };
        let opacity = f32::from(sprite.opacity) / 255.0;
        // Darker as the sprite's brightness has it (game ADR-0075).
        let bright = f32::from(sprite.brightness) / 255.0;
        let src = sprite.clipped_src();
        blit(
            out,
            image,
            src,
            dest,
            sprite.flip_x,
            sprite.paint,
            [bright, bright, bright, opacity],
        );
    }

    /// Where to find `glyph` in the atlas and what colour to tint it:
    /// `fg`, or the fallback glyph in `MISSING_COLOR`.
    fn atlas_cell(&self, glyph: char, fg: Rgb) -> Option<(AtlasRect, Rgb)> {
        if let Some(rect) = self.atlas.glyph_rect(glyph) {
            return Some((rect, fg));
        }
        Some((self.atlas.glyph_rect(FALLBACK_GLYPH)?, MISSING_COLOR))
    }
}

/// Stretches the `src` part of `image` (`[x, y, w, h]` in image pixels)
/// over `dest` in `out`, mirrored if `flip_x`, each texel coloured as
/// `paint`, multiplied by `tint` (RGBA, 0 to 1) and blended with the
/// result's alpha. Each pixel takes the texel under its centre, as nearest
/// sampling does.
#[allow(clippy::cast_precision_loss)] // pixel counts are small
fn blit(
    out: &mut Image,
    image: &Image,
    src: [f32; 4],
    (left, top, width, height): (i64, i64, i64, i64),
    flip_x: bool,
    paint: Paint,
    tint: [f32; 4],
) {
    let [src_x, src_y, src_w, src_h] = src;
    for dy in 0..height {
        let tex_y = src_y + (dy as f32 + 0.5) / height as f32 * src_h;
        for dx in 0..width {
            let across = (dx as f32 + 0.5) / width as f32;
            let tex_x = if flip_x {
                src_x + src_w * (1.0 - across)
            } else {
                src_x + src_w * across
            };
            #[allow(clippy::cast_possible_truncation)] // floored image coordinates
            let texel = image.texel(tex_x.floor() as i64, tex_y.floor() as i64);
            let [red, green, blue] = unit(paint.apply(Rgb::new(texel[0], texel[1], texel[2])));
            let color = [red * tint[0], green * tint[1], blue * tint[2]];
            let alpha = f32::from(texel[3]) / 255.0;
            out.blend(left + dx, top + dy, color, alpha * tint[3]);
        }
    }
}

#[cfg(test)]
mod tests;
