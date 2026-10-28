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
//! [`Painter`] decodes a game's font atlas and images from PNG and paints
//! with the raster, `shuyi_ui::frame::raster`: the reference picture of a
//! frame, which `shuyi-app` must match. Its module docs say what it draws.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use shuyi_assets::font::FontAtlasDef;
use shuyi_assets::image::ImageTable;
use shuyi_assets::target::{Color, TargetProfile};
use shuyi_platform::harness::{FRAME_DT, Harness, MAX_FRAMES};
use shuyi_ui::GameData;
use shuyi_ui::frame::raster;
pub use shuyi_ui::frame::raster::{Image, one_bit_picture};
use shuyi_ui::frame::{GlyphBuffer, Rgb};
use shuyi_ui::input::{Button, Chord};

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
  --wait 0.5         let that many seconds pass
  --crank -30        wind the crank that many degrees in one frame
                     (backwards is negative)

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
// A set that grows: a step is added without a break (ADR-0039).
#[non_exhaustive]
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
    /// `Harness::crank`: the crank winds that many degrees in one frame
    /// (backwards is negative), as a Playdate front end reports it.
    Crank(f32),
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
            Step::Crank(degrees) => {
                if !degrees.is_finite() {
                    return Err(format!("crank: {degrees} is not a number of degrees"));
                }
            }
        }
        Ok(())
    }
}

/// What to do in the game before taking the frame.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// Built with `Script::new` or `default()`: it may grow (ADR-0039).
#[non_exhaustive]
pub struct Script {
    /// The steps, in order.
    pub steps: Vec<Step>,
}

impl Script {
    /// The script of `steps`, in order.
    pub fn new(steps: Vec<Step>) -> Self {
        Self { steps }
    }

    /// Plays the steps in `h`.
    pub fn play<G: GameData>(&self, h: &mut Harness<G>) {
        for step in &self.steps {
            match step {
                Step::Keys(script) => h.keys(script),
                Step::Pad(script) => h.pad(script),
                Step::Wait(seconds) => h.wait(*seconds),
                Step::Crank(degrees) => h.crank(*degrees),
            };
        }
    }

    /// Whether the Harness can play every step ([`Step::check`]).
    pub fn check(&self) -> Result<(), String> {
        self.steps.iter().try_for_each(Step::check)
    }

    /// Reads the script flag `flag` with its value `value`, if it is one:
    /// `--keys` (or `--script`), `--pad`, `--wait` or `--crank`. A step the Harness
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
            "--keys" | "--pad" | "--wait" | "--crank" => {
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
                parsed.game.push(game_arg(flag, options, value)?);
            }
            path if out.is_none() => out = Some(PathBuf::from(path)),
            extra => return Err(format!("one output path only, not also {extra}")),
        }
    }
    parsed.out = out.ok_or("missing <out.png>")?;
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
