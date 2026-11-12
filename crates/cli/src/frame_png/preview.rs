//! A game's preview: an animated PNG of its first seconds, `preview.png` in
//! the project's root, which its README shows and the website's Examples
//! card plays. `shuyi.ron`'s `preview` part holds the `frame-png`
//! arguments that make it ([`crate::project::Preview`]),
//! `frame-png --preview` writes it, and [`check`] fails when the file is no
//! longer what they make: a test of the game calls it, so a change to what
//! the game shows fails until the preview is remade.
//!
//! [`check`] compares decoded frames, delays and plays, never bytes: the
//! same frames written with another compression pass.

use std::fs;

use crate::codec::{Animation, decode_apng};
use crate::game::{GameOption, GameTools};
use crate::project::Project;

use super::{Options, Output, parse_args};

/// The preview's file, in the project's root.
pub const FILE: &str = "preview.png";

/// The largest preview, in bytes (512 KiB).
pub const MAX_BYTES: u64 = 512 * 1024;

/// The part's name, as the errors give it.
const PART: &str = "shuyi.ron's `preview`";

/// `frame-png`'s options for the preview of `project`, from its `preview`
/// part's arguments with [`FILE`] as the output path; `game` is the game's
/// own options.
///
/// # Errors
/// Each names `shuyi.ron`'s `preview`: no part, arguments that don't parse,
/// no `--count` or an output other than `--apng`, an output path among the
/// arguments, or `--target` (a preview shows the game's own colours).
pub fn options(project: &Project, game: &[GameOption]) -> Result<Options, String> {
    let part = project.part(&project.config.preview, "preview")?;
    let mut args = vec![FILE.to_owned()];
    args.extend(part.args.iter().cloned());
    let options = parse_args(&args, game).map_err(|e| {
        match e.strip_prefix("one output path only, not also ") {
            Some(path) => format!(
                "{PART} names an output path, {path}: the preview is always {FILE}, \
                 in the project's root"
            ),
            None => format!("{PART}: {e}"),
        }
    })?;
    if !matches!(options.capture, Some((_, Output::Apng))) {
        return Err(format!(
            "{PART}: a preview is an APNG: add --count and --apng"
        ));
    }
    if options.target.is_some() {
        return Err(format!(
            "{PART}: no --target: a preview shows the game's own colours"
        ));
    }
    Ok(options)
}

/// Whether `preview.png` in the root of `project` is what its `preview`
/// part makes with `game`: the same frames, pixel for pixel, each shown
/// `every`/60 s, looping forever, in at most [`MAX_BYTES`].
///
/// # Errors
/// The part's errors ([`options`]), the game's, or the first difference,
/// named: no file, a file too large, the frame count, a frame's size or
/// pixels (by its number, from 1), a delay or the plays. Each of the file's
/// ends with the command that remakes it.
pub fn check(project: &Project, game: &dyn GameTools) -> Result<(), String> {
    let options = options(project, game.frame_options())?;
    let Some((capture, Output::Apng)) = options.capture else {
        return Err(format!("{PART}: a preview is an APNG"));
    };
    let shots = game.frames(&options.game, &options.script, options.scale, capture)?;
    let remake = |problem: String| {
        format!(
            "{problem}\nremake it: {} frame-png --preview",
            project.config.command()
        )
    };
    let path = project.root.join(FILE);
    let bytes = fs::read(&path).map_err(|e| remake(format!("{}: {e}", path.display())))?;
    check_size(u64::try_from(bytes.len()).unwrap_or(u64::MAX), MAX_BYTES).map_err(remake)?;
    let animation = decode_apng(&bytes).map_err(|e| remake(format!("{FILE}: {e}")))?;
    let every = u64::from(capture.every);
    let want: Vec<_> = shots.iter().map(|shot| &shot.image).collect();
    compare(&animation, &want, every).map_err(|e| remake(format!("{FILE} is stale: {e}")))
}

/// Whether a file of `len` bytes is within `cap`; the error gives its size
/// and how to make it smaller.
fn check_size(len: u64, cap: u64) -> Result<(), String> {
    if len <= cap {
        return Ok(());
    }
    Err(format!(
        "{FILE} is {} KiB, more than the {} KiB a preview may be: take frames \
         further apart (a longer --every), fewer (a smaller --count) or smaller \
         (--scale 1)",
        len.div_ceil(1024),
        cap / 1024
    ))
}

/// The first difference between `animation` and the frames `want`, each
/// shown `every` frames of 1/60 s, looping forever.
fn compare(
    animation: &Animation,
    want: &[&shuyi_assets::image::Image],
    every: u64,
) -> Result<(), String> {
    if animation.frames.len() != want.len() {
        return Err(format!(
            "it has {} frames, and {PART} makes {}",
            animation.frames.len(),
            want.len()
        ));
    }
    for (i, (have, want)) in animation.frames.iter().zip(want).enumerate() {
        let n = i + 1;
        if (have.width, have.height) != (want.width, want.height) {
            return Err(format!(
                "frame {n} is {}×{}, and {PART} makes it {}×{}",
                have.width, have.height, want.width, want.height
            ));
        }
        if let Some(pixel) = have
            .rgba
            .chunks(4)
            .zip(want.rgba.chunks(4))
            .position(|(a, b)| a != b)
        {
            let width = usize::try_from(have.width).unwrap_or(usize::MAX).max(1);
            return Err(format!(
                "frame {n}'s pixels differ from what {PART} makes, first at ({}, {})",
                pixel % width,
                pixel / width
            ));
        }
        let (num, den) = animation.delays[i];
        // A denominator of 0 means hundredths (the APNG specification).
        let den = if den == 0 { 100 } else { u64::from(den) };
        if u64::from(num) * 60 != every * den {
            return Err(format!(
                "frame {n} is shown {num}/{den} s, and {PART} shows each {every}/60 s"
            ));
        }
    }
    if animation.plays != 0 {
        return Err(format!(
            "it plays {} times, and a preview loops forever",
            animation.plays
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::codec::encode_apng;
    use crate::project::{Config, Preview};
    use shuyi_assets::image::Image;

    fn project(args: Option<&[&str]>) -> Project {
        let config = Config {
            command: Some("cargo shuyi".to_owned()),
            preview: args.map(|args| Preview::new(args.iter().map(|a| (*a).to_owned()).collect())),
            ..Config::default()
        };
        Project::new("game", config)
    }

    fn error(args: Option<&[&str]>) -> String {
        options(&project(args), &[]).unwrap_err()
    }

    #[test]
    fn the_options_are_the_parts_args_written_to_the_preview() {
        let args = ["--keys", "Up", "--every", "0.1", "--count", "4", "--apng"];
        let options = options(&project(Some(&args)), &[]).unwrap();
        assert_eq!(options.out, std::path::Path::new(FILE));
        let (capture, output) = options.capture.unwrap();
        assert_eq!((capture.every, capture.count, output), (6, 4, Output::Apng));
        assert_eq!(options.script.steps.len(), 1);
    }

    #[test]
    fn each_error_of_the_options_names_the_part() {
        assert_eq!(error(None), "shuyi.ron has no `preview`");
        assert_eq!(
            error(Some(&["--every", "x"])),
            "shuyi.ron's `preview`: --every: not a number of seconds: x"
        );
        let apng = "shuyi.ron's `preview`: a preview is an APNG: add --count and --apng";
        assert_eq!(error(Some(&["--keys", "Up"])), apng);
        assert_eq!(error(Some(&["--every", "0.1", "--count", "2"])), apng);
        assert_eq!(
            error(Some(&["--every", "0.1", "--count", "2", "--gif"])),
            apng
        );
        assert_eq!(
            error(Some(&[
                "out.png", "--every", "0.1", "--count", "2", "--apng"
            ])),
            "shuyi.ron's `preview` names an output path, out.png: the preview is \
             always preview.png, in the project's root"
        );
        assert_eq!(
            error(Some(&[
                "--target", "playdate", "--every", "0.1", "--count", "2", "--apng"
            ])),
            "shuyi.ron's `preview`: no --target: a preview shows the game's own colours"
        );
    }

    #[test]
    fn the_size_is_checked_against_the_cap() {
        assert!(check_size(2048, 2048).is_ok());
        let error = check_size(2049, 2048).unwrap_err();
        assert!(
            error.starts_with("preview.png is 3 KiB, more than the 2 KiB a preview may be"),
            "{error}"
        );
        assert!(
            error.contains("--every") && error.contains("--count") && error.contains("--scale 1")
        );
        assert_eq!(MAX_BYTES, 524_288);
    }

    fn frame(v: u8) -> Image {
        Image {
            width: 2,
            height: 2,
            rgba: vec![v; 16],
        }
    }

    #[test]
    fn compare_names_the_first_difference() {
        let frames = [frame(1), frame(2)];
        let refs: Vec<&Image> = frames.iter().collect();
        let animation = crate::codec::decode_apng(&encode_apng(&refs, 6).unwrap()).unwrap();
        assert_eq!(compare(&animation, &refs, 6), Ok(()));
        assert!(
            compare(&animation, &refs, 3)
                .unwrap_err()
                .contains("frame 1 is shown 6/60 s")
        );
        assert!(
            compare(&animation, &refs[..1], 6)
                .unwrap_err()
                .starts_with("it has 2 frames")
        );
        let mut other = frames[1].clone();
        other.rgba[13] = 0;
        assert_eq!(
            compare(&animation, &[&frames[0], &other], 6).unwrap_err(),
            "frame 2's pixels differ from what shuyi.ron's `preview` makes, first at (1, 1)"
        );
        let big = Image {
            width: 3,
            height: 2,
            rgba: vec![1; 24],
        };
        assert!(
            compare(&animation, &[&big, &frames[1]], 6)
                .unwrap_err()
                .starts_with("frame 1 is 2×2")
        );
        let mut once = animation.clone();
        once.plays = 1;
        assert!(
            compare(&once, &refs, 6)
                .unwrap_err()
                .contains("loops forever")
        );
        // 1/10 s is 6/60 s.
        let mut tenths = animation;
        tenths.delays = vec![(1, 10), (10, 0)];
        assert_eq!(compare(&tenths, &refs, 6), Ok(()));
    }
}
