//! `corners`: a terrain's corner sheet made from its recipe
//! (`<name>.corners.ron`, [`shuyi_assets::corners::recipe`]) and written
//! as a PNG, which the game keeps beside its other pictures and draws as
//! it draws a hand-drawn sheet. Behind the `corners` feature, off by
//! default: a game opts in by turning it on in its tools' dependency on
//! this crate, then runs `cargo shuyi corners <recipe>...`, and keeps
//! each sheet equal to what its recipe makes with [`check`] in a test.
//!
//! Files are read and written only by [`main`] and by the `read` a caller
//! gives; making the sheet is pure (`shuyi_assets::corners`).

use std::fs;
use std::path::Path;

use shuyi_assets::bitmap::convert::png_pixels;
use shuyi_assets::corners::recipe::{self, Recipe};
use shuyi_assets::image::Image;

use crate::codec::encode_png;

/// How a game runs the command.
pub const COMMAND: &str = "cargo shuyi corners";

/// The sheet `recipe` makes, with `read` reading a file the recipe names,
/// at its path as the recipe writes it (from the recipe's folder), and the
/// pictures decoded as PNG.
///
/// # Errors
/// `read`'s, a picture that isn't a PNG, a tile or piece outside its
/// picture, or the sheet's ([`shuyi_assets::corners::CornersError`]).
pub fn make(
    recipe: &Recipe,
    read: impl Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<Image, String> {
    recipe.sheet(|path| {
        let (width, height, rgba) = png_pixels(&read(path)?)?;
        Ok(Image {
            width,
            height,
            rgba,
        })
    })
}

/// Whether the recipe `recipe_text`, the file `recipe_file`, makes what its
/// `out` holds: the same size and pixels (not bytes: an encoder may write
/// them otherwise). `read` reads a file the recipe names, as in [`make`].
/// A game calls it from a test, so a stale sheet fails the game's CI.
///
/// # Errors
/// The recipe's (with its file and line) or [`make`]'s, a missing or
/// unreadable `out`, or one that differs, named. Each of `out`'s ends with
/// the command that remakes it.
pub fn check(
    recipe_file: &str,
    recipe_text: &str,
    read: impl Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let recipe = recipe::parse(recipe_file, recipe_text).map_err(|e| e.to_string())?;
    let want = make(&recipe, &read)?;
    let out = &recipe.out;
    let remake = |problem: String| format!("{problem}\nremake it: {COMMAND} {recipe_file}");
    let bytes = read(out).map_err(|e| remake(format!("{out}: {e}")))?;
    let (width, height, rgba) = png_pixels(&bytes).map_err(|e| remake(format!("{out}: {e}")))?;
    if (width, height) != (want.width, want.height) {
        return Err(remake(format!(
            "{out} is stale: it is {width}×{height} pixels, and {recipe_file} makes {}×{}",
            want.width, want.height
        )));
    }
    if let Some(at) = rgba
        .chunks(4)
        .zip(want.rgba.chunks(4))
        .position(|(have, want)| have != want)
    {
        let at = u32::try_from(at).unwrap_or(u32::MAX);
        let (x, y) = (at % width.max(1), at / width.max(1));
        return Err(remake(format!(
            "{out} is stale: its pixels differ from what {recipe_file} makes, first at ({x}, {y})"
        )));
    }
    Ok(())
}

/// The usage of the command, run as `command`.
pub fn usage(command: &str) -> String {
    format!(
        "usage: {command} corners [--check] <recipe{}>...\n\
         writes each recipe's sheet to its `out`; --check writes nothing and fails\n\
         when a sheet is missing or differs from what its recipe makes",
        recipe::EXTENSION
    )
}

/// `corners [--check] <recipe>...` on the files `args` names, from the
/// working folder: writes each recipe's `out` and says which, or with
/// `--check` writes nothing and checks each ([`check`]). The exit code:
/// 0 when every recipe passed, 1 when one failed, 2 for wrong arguments.
pub fn main(command: &str, args: &[String]) -> u8 {
    let checking = args.iter().any(|arg| arg == "--check");
    let files: Vec<&String> = args.iter().filter(|arg| *arg != "--check").collect();
    if files.is_empty() || files.iter().any(|file| file.starts_with('-')) {
        eprintln!("{}", usage(command));
        return 2;
    }
    let mut failed = false;
    for file in files {
        let done = if checking {
            run_check(file).map(|()| format!("corners: {file}: up to date"))
        } else {
            write(file)
        };
        match done {
            Ok(said) => println!("{said}"),
            Err(e) => {
                eprintln!("corners: {e}");
                failed = true;
            }
        }
    }
    u8::from(failed)
}

/// Reads a file beside the recipe `recipe` (a path from the working folder).
fn beside(recipe: &str) -> impl Fn(&str) -> Result<Vec<u8>, String> + '_ {
    move |path| {
        let path = folder(recipe).join(path);
        fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))
    }
}

/// The folder the file `file` is in.
fn folder(file: &str) -> &Path {
    Path::new(file).parent().unwrap_or_else(|| Path::new(""))
}

/// The recipe in the file `file`'s text.
fn text(file: &str) -> Result<String, String> {
    fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))
}

fn run_check(file: &str) -> Result<(), String> {
    check(file, &text(file)?, beside(file))
}

/// Makes the recipe in `file` and writes its sheet, saying where.
fn write(file: &str) -> Result<String, String> {
    let recipe = recipe::parse(file, &text(file)?).map_err(|e| e.to_string())?;
    let sheet = make(&recipe, beside(file))?;
    let png = encode_png(sheet.width, sheet.height, &sheet.rgba)?;
    let out = folder(file).join(&recipe.out);
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    fs::write(&out, png).map_err(|e| format!("{}: {e}", out.display()))?;
    Ok(format!(
        "corners: wrote {} ({}×{} pixels) from {file}",
        out.display(),
        sheet.width,
        sheet.height
    ))
}

#[cfg(test)]
mod tests;
