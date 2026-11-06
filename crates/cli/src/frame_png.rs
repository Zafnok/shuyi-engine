//! `frame-png`: renders a frame of a game in the test Harness to a PNG
//! without a window, for looking at a skin, a picture or a mockup. Never a
//! gate: snapshots stay text.
//!
//! The command plays a [`Script`] of keys, controller buttons, actions by
//! name and waits in the game's Harness (`shuyi_platform::script`'s, the one
//! the live link and the capture plans play), which the game's [`GameTools::frame`] starts with
//! the game's own options, and writes the picture the game paints of it.
//! A game paints a frame with [`Painter`], or, for a one-bit screen, with
//! the one-bit renderer and [`one_bit_picture`]. With `--target <id>`, a
//! target whose display is black and white gets the picture its device
//! shows ([`GameTools::frame_one_bit`]: the frame drawn in its roles'
//! patterns, ADR-0008 §4). With `--debug <channels>`, the debug view's
//! channels are on from the start (a [`Step::Debug`] first in the script,
//! ADR-0084): the picture shows what a person sees with the view on.
//!
//! [`Painter`] decodes a game's font atlas and images from PNG and paints
//! with the raster, `shuyi_ui::frame::raster`: the reference picture of a
//! frame, which `shuyi-app` must match. Its module docs say what it draws.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageTable;
use shuyi_assets::target::{Color, TargetProfile};
use shuyi_platform::harness::MAX_WAIT;
use shuyi_platform::script::check_waits;
pub use shuyi_platform::script::{Script, Step};
use shuyi_ui::frame::raster;
pub use shuyi_ui::frame::raster::{Image, one_bit_picture};
use shuyi_ui::frame::{GlyphBuffer, Rgb};

use crate::codec::{decode_png, encode_png};
use crate::game::{GameArg, GameOption, GameTools, game_arg};

/// Largest `--scale`.
pub const MAX_SCALE: u32 = 8;

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
  --act Confirm      press the game's action by its key, or else its button
  --wait 0.5         let that many seconds pass (60 frames a second, rounded up)
  --crank -30        wind the crank that many degrees in one frame
                     (backwards is negative)

options:
  --scale N            whole-number scale, 1 to {MAX_SCALE} (the game says what by default)
  --target ID          as target ID's display shows it: on a one-bit one (the
                       Playdate), black and white in the skin's patterns
  --debug CHANNELS     the debug view's channels on from the start, by id,
                       comma-separated (grid,collision; all: every channel),
                       with debug tools on
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
never-hard-code-a-key rule doesn't apply to them. --act names the game's
action instead, and presses whatever the player's keymap binds to it.",
    );
    if !notes.is_empty() {
        out.push_str("\n\n");
        out.push_str(notes);
    }
    out
}

/// Reads the script flag `flag` with its value `value`, if it is one:
/// `--keys` (or `--script`), `--pad`, `--act`, `--wait` or `--crank`. A
/// step no player can play ([`Step::check`]) is an error, not a panic
/// later. The longest wait is the player's to check (the Harness's with
/// [`check`]), and an action's name is looked up in the game's keymap.
pub fn parse_step(flag: &str, value: &str) -> Option<Result<Step, String>> {
    let step = match flag {
        "--keys" | "--script" => Step::Keys(value.to_owned()),
        "--pad" => Step::Pad(value.to_owned()),
        "--act" => Step::Act(value.to_owned()),
        "--wait" => match value.parse::<f32>() {
            Ok(seconds) => Step::Wait(seconds),
            Err(_) => {
                return Some(Err(format!("--wait: not a number of seconds: {value}")));
            }
        },
        "--crank" => match value.parse::<f32>() {
            Ok(degrees) => Step::Crank(degrees),
            Err(_) => {
                return Some(Err(format!("--crank: not a number of degrees: {value}")));
            }
        },
        _ => return None,
    };
    Some(step.check().map(|()| step).map_err(|e| format!("--{e}")))
}

/// Whether the Harness can play every step of `script`: each step
/// ([`Script::check`]), and waits of at most [`MAX_WAIT`] seconds. An error
/// names the step.
pub fn check(script: &Script) -> Result<(), String> {
    script.check()?;
    check_waits(&script.steps, MAX_WAIT)
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
/// checking key and button names and waits up front ([`check`]) so a typo
/// is an error, not a Harness panic.
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
    let mut debug = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        match arg.as_str() {
            "--keys" | "--pad" | "--act" | "--wait" | "--crank" => {
                let text = value()?;
                if let Some(step) = parse_step(arg, &text) {
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
            "--debug" => debug.push(value()?),
            flag if flag.starts_with("--") => {
                parsed.game.push(game_arg(flag, options, value)?);
            }
            path if out.is_none() => out = Some(PathBuf::from(path)),
            extra => return Err(format!("one output path only, not also {extra}")),
        }
    }
    parsed.out = out.ok_or("missing <out.png>")?;
    // The channels are on from the first frame, wherever the flag is.
    if !debug.is_empty() {
        let step = Step::Debug(debug.join(","));
        step.check().map_err(|e| format!("--{e}"))?;
        parsed.script.steps.insert(0, step);
    }
    check(&parsed.script)?;
    Ok(parsed)
}

/// What a game shows of a frame. Build it with [`Shot::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Shot {
    /// The picture.
    pub image: Image,
    /// What it shows, for the summary: the top screen's name.
    pub shows: String,
    /// What the summary warns about it, if anything (a picture of art the
    /// game may not hand out).
    pub warning: Option<String>,
}

impl Shot {
    /// The picture `image` of the screen `shows`, with no warning.
    pub fn new(image: Image, shows: impl Into<String>) -> Self {
        Self {
            image,
            shows: shows.into(),
            warning: None,
        }
    }
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

/// Decodes PNG file `png`.
pub fn decode_image(png: &[u8]) -> Result<Image, String> {
    let (width, height, rgba) = decode_png(png)?;
    Ok(Image {
        width,
        height,
        rgba,
    })
}

/// The raster's [`raster::Painter`], with its font atlas and images
/// decoded from PNG files, ready to render any number of frames (each
/// image is decoded once).
pub struct Painter(raster::Painter);

impl Painter {
    /// Decodes atlas image `atlas_png` and every image in `images`, whose
    /// files `read` returns (an image `read` lacks, or that doesn't decode,
    /// draws as a [`MISSING_COLOR`](raster::MISSING_COLOR) rectangle).
    /// `clear` is the console background (the palette's `black`).
    pub fn new<'a>(
        atlas: FontAtlasDef,
        atlas_png: &[u8],
        images: &ImageTable,
        read: impl Fn(&str) -> Option<&'a [u8]>,
        clear: Rgb,
    ) -> Result<Self, String> {
        let atlas_image = decode_image(atlas_png).map_err(|e| format!("font atlas image: {e}"))?;
        let images = images
            .ids()
            .filter_map(|id| Some((id, decode_image(read(id.path())?).ok()?)))
            .collect();
        Ok(Self(raster::Painter::new(
            atlas,
            atlas_image,
            images,
            clear,
        )))
    }

    /// Draws `buf` with each console pixel `scale` × `scale` pixels big
    /// ([`raster::Painter::render`]).
    pub fn render(&self, buf: &GlyphBuffer, scale: u32) -> Image {
        self.0.render(buf, scale)
    }

    /// The raster's painter this one decoded.
    pub fn raster(&self) -> &raster::Painter {
        &self.0
    }
}

#[cfg(test)]
mod tests;
