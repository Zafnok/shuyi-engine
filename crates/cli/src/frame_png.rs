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
//! ADR-0085): the picture shows what a person sees with the view on.
//!
//! With `--every <seconds> --count <n>`, it writes frames over time
//! instead ([`Capture`]): `n` frames, the first right after the script,
//! as one [`sheet`] of cells labelled with their times (the default), or
//! as an APNG (`--apng`) or a GIF of an exact palette (`--gif`).
//! [`GameTools::frames`] paints them.
//!
//! With `--preview` alone, it takes its arguments from `shuyi.ron`'s
//! `preview` part and writes the game's [`preview`], `preview.png`: an
//! APNG a README and the website's Examples card show, which a test of the
//! game keeps equal to what the part makes ([`preview::check`]).
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
use shuyi_platform::harness::{MAX_FRAMES, MAX_WAIT};
use shuyi_platform::script::{FRAME_DT, check_waits};
pub use shuyi_platform::script::{Script, Step};
use shuyi_ui::frame::raster;
pub use shuyi_ui::frame::raster::{Image, one_bit_picture};
use shuyi_ui::frame::{GlyphBuffer, Rgb};

use crate::codec::{decode_png, encode_apng, encode_gif, encode_png, gif_delay};
use crate::game::{GameArg, GameOption, GameTools, game_arg};

pub mod preview;
pub mod sheet;

/// Largest `--scale`.
pub const MAX_SCALE: u32 = 8;

/// Most frames one capture takes (`--count`).
pub const MAX_CAPTURES: u32 = 64;

/// A copy of `script` with `n` more frames of no input after it: `n`
/// waits of one frame ([`FRAME_DT`]) each.
pub fn then_frames(script: &Script, n: u32) -> Script {
    let mut steps = script.steps.clone();
    steps.extend((0..n).map(|_| Step::Wait(FRAME_DT)));
    Script::new(steps)
}

/// Frames over time: `count` frames, `every` frames apart, the first right
/// after the script. Build it with [`Capture::new`] (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Capture {
    /// Frames between two captures, at least 1.
    pub every: u32,
    /// How many frames are captured, 1 to [`MAX_CAPTURES`].
    pub count: u32,
}

impl Capture {
    /// `count` frames `every` frames apart.
    ///
    /// # Errors
    /// `every` of 0 or over [`MAX_FRAMES`], `count` of 0 or over
    /// [`MAX_CAPTURES`], or a last capture more than [`MAX_FRAMES`] frames
    /// after the first; the error names the cap.
    pub fn new(every: u32, count: u32) -> Result<Self, String> {
        if every == 0 {
            return Err("every: at least one frame (1/60 s)".to_owned());
        }
        if every > MAX_FRAMES {
            return Err(format!("every: at most {MAX_FRAMES} frames, not {every}"));
        }
        if !(1..=MAX_CAPTURES).contains(&count) {
            return Err(format!("count: 1 to {MAX_CAPTURES}, not {count}"));
        }
        let span = u64::from(every) * u64::from(count - 1);
        if span > u64::from(MAX_FRAMES) {
            return Err(format!(
                "every × (count − 1) is {span} frames, more than {MAX_FRAMES}: \
                 take fewer frames or closer together"
            ));
        }
        Ok(Self { every, count })
    }

    /// How many frames after the script capture `i` is.
    pub fn frame_of(&self, i: u32) -> u32 {
        i * self.every
    }

    /// The time of capture `i` since the script ended, in whole
    /// milliseconds, rounded down.
    pub fn millis_of(&self, i: u32) -> u64 {
        u64::from(self.frame_of(i)) * 1000 / 60
    }
}

/// `--every`'s `seconds` as frames: whole frames of 1/60 s, to the
/// nearest (0.05 s is 3). Not a number, more than [`MAX_WAIT`], or under
/// half a frame is an error; the last names the shortest, 1/60 s.
pub fn every_frames(seconds: f32) -> Result<u32, String> {
    if !seconds.is_finite() || seconds > MAX_WAIT {
        return Err(format!(
            "every: a number of seconds, 1/60 to {MAX_WAIT:.1}, not {seconds}"
        ));
    }
    let frames = (f64::from(seconds) * 60.0).round();
    if frames < 1.0 {
        return Err(format!(
            "every: {seconds} seconds is no whole frame; the shortest is 1/60 s (0.0167)"
        ));
    }
    // A whole number from 1 to `MAX_FRAMES` (checked above): exact in a
    // `u32`.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(frames as u32)
}

/// What a capture writes. A set that may grow (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Output {
    /// One picture of the frames in cells labelled with their times
    /// ([`sheet::build`]), `columns` wide (as square as it can be if
    /// `None`).
    Sheet {
        /// Cells in a row.
        columns: Option<u32>,
    },
    /// An animated PNG that loops forever, each frame shown `every`
    /// frames.
    Apng,
    /// A GIF that loops forever, each frame shown `every` frames in
    /// hundredths of a second, to the nearest and at least 2; its palette
    /// is exact (never quantized), or the GIF is refused.
    Gif,
}

/// The usage text, with `command` as the tool's name and the game's own
/// options and notes.
pub fn usage(command: &str, options: &[GameOption], notes: &str) -> String {
    let mut out = format!(
        "\
usage: {command} frame-png <out.png> [steps] [options]
       {command} frame-png --preview

Starts the game in the test Harness, runs the steps in the order given,
and writes the frame to <out.png> (a relative path is from the project's
root).

--preview, alone, writes the game's preview, {preview} in the project's
root: an APNG of the steps and options in shuyi.ron's `preview` part
(`args`, as typed after <out.png>), which a README and the website show.

steps (each may repeat):
  --keys \"F2 Up f\"   press and release each key chord in turn
  --pad \"South\"      press and release each controller button in turn
                     (\"East+DpadDown\": those buttons together)
  --act Confirm      press the game's action by its key, or else its button
  --wait 0.5         let that many seconds pass (60 frames a second, rounded up)
  --crank -30        wind the crank that many degrees in one frame
                     (backwards is negative)
  --drop a.txt       drop that file, from the game's folder, on the window

options:
  --scale N            whole-number scale, 1 to {MAX_SCALE} (the game says what by default)
  --target ID          as target ID's display shows it: on a one-bit one (the
                       Playdate), black and white in the skin's patterns
  --debug CHANNELS     the debug view's channels on from the start, by id,
                       comma-separated (grid,collision; all: every channel),
                       with debug tools on

frames over time (with --count; without it, one frame):
  --every SECONDS      the time between two frames, in whole frames of 1/60 s
  --count N            how many frames, 1 to {MAX_CAPTURES}, the first right after the
                       steps; every × (count − 1) at most {MAX_FRAMES} frames
  --sheet              one picture of the frames, each labelled with its time
                       since the steps ended (the default)
  --columns N          the sheet's cells in a row (as square as it can be by
                       default)
  --apng               an animated PNG that loops forever
  --gif                a GIF that loops forever, its colours exact (at most 256
                       a frame; each frame shown in whole 1/100 s, at least 2)
",
        preview = preview::FILE,
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
/// `--keys` (or `--script`), `--pad`, `--act`, `--wait`, `--crank` or
/// `--drop`. A
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
        "--drop" => Step::Drop(value.to_owned()),
        _ => return None,
    };
    Some(step.check().map(|()| step).map_err(|e| format!("--{e}")))
}

/// Whether the Harness can play every step of `script`: each step
/// ([`Script::check`]), waits of at most [`MAX_WAIT`] seconds, and a file
/// to drop that is there, from the working folder (the game's project
/// folder). An error names the step.
pub fn check(script: &Script) -> Result<(), String> {
    script.check()?;
    check_waits(&script.steps, MAX_WAIT)?;
    for (i, step) in script.steps.iter().enumerate() {
        if let Step::Drop(path) = step
            && !Path::new(path).is_file()
        {
            return Err(format!(
                "step {}: drop: {path} is no file in the game's folder",
                i + 1
            ));
        }
    }
    Ok(())
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
    /// Frames over time and what they are written as, if `--count` is
    /// given; one frame if `None`.
    pub capture: Option<(Capture, Output)>,
}

/// The capture flags as given, before they are checked together.
#[derive(Default)]
struct CaptureFlags {
    every: Option<u32>,
    count: Option<u32>,
    columns: Option<u32>,
    outputs: Vec<&'static str>,
}

impl CaptureFlags {
    /// Reads capture flag `flag`, taking its value from `value`; `false` if
    /// `flag` is none of them.
    fn read(
        &mut self,
        flag: &str,
        value: impl FnOnce() -> Result<String, String>,
    ) -> Result<bool, String> {
        let whole = |text: String| {
            text.parse::<u32>()
                .map_err(|_| format!("{flag}: not a whole number: {text}"))
        };
        match flag {
            "--every" => {
                let text = value()?;
                let seconds = text
                    .parse::<f32>()
                    .map_err(|_| format!("--every: not a number of seconds: {text}"))?;
                self.every = Some(every_frames(seconds).map_err(|e| format!("--{e}"))?);
            }
            "--count" => self.count = Some(whole(value()?)?),
            "--columns" => self.columns = Some(whole(value()?)?),
            "--sheet" => self.outputs.push("--sheet"),
            "--apng" => self.outputs.push("--apng"),
            "--gif" => self.outputs.push("--gif"),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// The capture the flags ask for, if any.
    fn capture(self) -> Result<Option<(Capture, Output)>, String> {
        let Some(count) = self.count else {
            if self.every.is_some() {
                return Err("--every needs --count".to_owned());
            }
            if let Some(flag) = self.outputs.first() {
                return Err(format!("{flag} needs --count"));
            }
            if self.columns.is_some() {
                return Err("--columns needs --count".to_owned());
            }
            return Ok(None);
        };
        let every = self.every.ok_or("--count needs --every")?;
        let capture = Capture::new(every, count).map_err(|e| format!("--{e}"))?;
        let output = match self.outputs.as_slice() {
            [] | ["--sheet"] => Output::Sheet {
                columns: self.columns,
            },
            [_] if self.columns.is_some() => {
                return Err("--columns is for the sheet only".to_owned());
            }
            ["--apng"] => Output::Apng,
            ["--gif"] => Output::Gif,
            _ => {
                return Err(format!(
                    "one output only, not {}",
                    self.outputs.join(" and ")
                ));
            }
        };
        if let Output::Sheet {
            columns: Some(columns),
        } = output
            && !(1..=count).contains(&columns)
        {
            return Err(format!("--columns: 1 to {count} (--count), not {columns}"));
        }
        Ok(Some((capture, output)))
    }
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
        capture: None,
    };
    let steps = &mut parsed.script.steps;
    let mut debug = Vec::new();
    let mut capture = CaptureFlags::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let mut value = || {
            iter.next()
                .ok_or_else(|| format!("{arg} needs a value"))
                .cloned()
        };
        match arg.as_str() {
            "--keys" | "--pad" | "--act" | "--wait" | "--crank" | "--drop" => {
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
            flag if capture.read(flag, &mut value)? => {}
            flag if flag.starts_with("--") => {
                parsed.game.push(game_arg(flag, options, value)?);
            }
            path if out.is_none() => out = Some(PathBuf::from(path)),
            extra => return Err(format!("one output path only, not also {extra}")),
        }
    }
    parsed.out = out.ok_or("missing <out.png>")?;
    parsed.capture = capture.capture()?;
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
    let one_bit = target.filter(|profile| profile.display.color == Color::OneBit);
    let painted = |script: &Script| match one_bit {
        Some(profile) => game.frame_one_bit(&profile.id, game_options, script, scale),
        None => game.frame(game_options, script, scale),
    };
    let Some((capture, output)) = options.capture else {
        let shot = painted(script)?;
        let png = encode_png(shot.image.width, shot.image.height, &shot.image.rgba)?;
        write(root, &options.out, &png)?;
        let shown = format!("{} → {}", shot.shows, options.out.display());
        return Ok(summary(&shown, &shot.image, shot.warning.as_deref()));
    };
    let shots = match one_bit {
        // A one-bit target's frames are painted one by one, as the default
        // `frames` paints them.
        Some(_) => (0..capture.count)
            .map(|i| painted(&then_frames(script, capture.frame_of(i))))
            .collect::<Result<Vec<_>, _>>()?,
        None => game.frames(game_options, script, scale, capture)?,
    };
    let (png, width, height) = match output {
        Output::Sheet { columns } => {
            let sheet = sheet::build(&shots, capture.every, columns)?;
            let png = encode_png(sheet.width, sheet.height, &sheet.rgba)?;
            (png, sheet.width, sheet.height)
        }
        Output::Apng => {
            let images: Vec<&Image> = shots.iter().map(|shot| &shot.image).collect();
            // `every` is at most `MAX_FRAMES` (`Capture::new`).
            let every = u16::try_from(capture.every).map_err(|_| "--every: too long")?;
            let png = encode_apng(&images, every)?;
            let first = images.first().map_or((0, 0), |i| (i.width, i.height));
            (png, first.0, first.1)
        }
        Output::Gif => {
            let images: Vec<&Image> = shots.iter().map(|shot| &shot.image).collect();
            let gif = encode_gif(&images, capture.every)?;
            let first = images.first().map_or((0, 0), |i| (i.width, i.height));
            (gif, first.0, first.1)
        }
    };
    write(root, &options.out, &png)?;
    let mut said = capture_summary(
        capture,
        &format!("{} ({width}×{height})", options.out.display()),
        &shots,
    );
    if output == Output::Gif
        && let Some(speed) = gif_speed(capture.every)
    {
        said.push('\n');
        said.push_str(&speed);
    }
    Ok(said)
}

/// What the summary says of a GIF whose frames are `every` frames of
/// 1/60 s apart, when its delay ([`gif_delay`]) isn't exactly that: the
/// time it shows each frame for, beside `every`'s.
fn gif_speed(every: u32) -> Option<String> {
    let delay = gif_delay(every);
    if delay * 60 == every * 100 {
        return None;
    }
    Some(format!(
        "the GIF shows each frame {}.{:02} seconds (every is {:.3} seconds)",
        delay / 100,
        delay % 100,
        f64::from(every) / 60.0
    ))
}

/// Writes `bytes` to `out`, under `root` if it is relative, making its
/// folder.
fn write(root: &Path, out: &Path, bytes: &[u8]) -> Result<(), String> {
    let path = root.join(out);
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("creating {}: {e}", dir.display()))?;
    }
    fs::write(&path, bytes).map_err(|e| format!("writing {}: {e}", path.display()))
}

/// What `run` prints for the frames `shots` of `capture`, written as
/// `written` (the path and its size): the count and the step, then each
/// frame's time and screen, then the first warning, once.
fn capture_summary(capture: Capture, written: &str, shots: &[Shot]) -> String {
    let every = capture.every;
    let mut out = format!(
        "frame-png: {} frames, every {:.3} s ({every} frames) → {written}",
        capture.count,
        f64::from(every) / 60.0,
    );
    for (i, shot) in (0..).zip(shots) {
        // Writing to a `String` can't fail.
        let _ = write!(out, "\n  +{}ms {}", capture.millis_of(i), shot.shows);
    }
    if let Some(warning) = shots.iter().find_map(|shot| shot.warning.as_deref()) {
        out.push_str("\nwarning: ");
        out.push_str(warning);
    }
    out
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
