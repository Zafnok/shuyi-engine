//! The pictures the renderer uploaded, made again when their files change
//! (ADR-0071): which changed paths remake which textures, and what decoded,
//! without a window. A picture that doesn't decode keeps its old texture, and
//! its error shows over the frame until its next good save.
//!
//! A sprite's source stays where the game's data put it, in the picture's
//! pixels from its top-left corner. So a picture that grows shows the same
//! pixels, its new part unused until the data names it, and a sprite whose
//! source is past the edge of one that shrank draws as a missing picture
//! ([`src_fits`]).

use std::collections::BTreeMap;

use macroquad::logging::warn;
use shuyi_assets::image::{ImageId, ImageTable};
use shuyi_ui::frame::glyph_buffer::PxRect;

/// A texture of the renderer's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Remake {
    /// The font atlas's.
    Font,
    /// An image's of the look's table.
    Image(ImageId),
}

/// The textures to make again for the paths in `changed`, each once: the
/// font's if `font_png` is among them, and those of the images of `images`.
/// Any other path remakes nothing.
pub(crate) fn remakes(changed: &[String], font_png: &str, images: &ImageTable) -> Vec<Remake> {
    let mut wanted = Vec::new();
    for path in changed {
        let font = (path == font_png).then_some(Remake::Font);
        for remake in font.into_iter().chain(images.id(path).map(Remake::Image)) {
            if !wanted.contains(&remake) {
                wanted.push(remake);
            }
        }
    }
    wanted
}

/// A picture that decoded, for its texture to be made again.
#[derive(Debug)]
pub(crate) struct Fresh<T> {
    /// The texture it is for.
    pub(crate) remake: Remake,
    /// The decoded picture.
    pub(crate) decoded: T,
    /// The file it decoded from.
    pub(crate) bytes: &'static [u8],
}

/// Why pictures didn't decode, by path, until their next good save.
#[derive(Debug, Default)]
pub(crate) struct PictureErrors(BTreeMap<String, String>);

impl PictureErrors {
    /// Whether every picture's last save was good.
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// A line for each picture that doesn't decode, in path order.
    pub(crate) fn lines(&self) -> impl Iterator<Item = String> + '_ {
        self.0
            .iter()
            .map(|(path, error)| format!("{path}: {error}"))
    }
}

/// Each picture of `wanted` read with `read` and decoded with `decode`. What
/// decodes clears its error; what doesn't is left out, so its old texture
/// stays, and its error is kept in `errors`.
pub(crate) fn decode_each<T>(
    wanted: &[Remake],
    font_png: &str,
    read: impl Fn(&str) -> Option<&'static [u8]>,
    decode: impl Fn(&[u8]) -> Result<T, String>,
    errors: &mut PictureErrors,
) -> Vec<Fresh<T>> {
    let mut fresh = Vec::new();
    for &remake in wanted {
        let path = match remake {
            Remake::Font => font_png,
            Remake::Image(id) => id.path(),
        };
        let bytes = read(path).ok_or_else(|| "not among the game's files".to_owned());
        match bytes.and_then(|bytes| Ok((bytes, decode(bytes)?))) {
            Ok((bytes, decoded)) => {
                errors.0.remove(path);
                fresh.push(Fresh {
                    remake,
                    decoded,
                    bytes,
                });
            }
            Err(error) => {
                warn!("hot reload: {}: {}", path, error);
                errors.0.insert(path.to_owned(), error);
            }
        }
    }
    fresh
}

/// Whether the sprite source `src` is inside a picture `size` pixels big.
pub(crate) fn src_fits(src: PxRect, size: (u16, u16)) -> bool {
    let (x, y) = (i64::from(src.x), i64::from(src.y));
    x >= 0
        && y >= 0
        && x + i64::from(src.w) <= i64::from(size.0)
        && y + i64::from(src.h) <= i64::from(size.1)
}

/// The file at `path`: from `disk` if it has it, else from `files`.
fn read_over(
    disk: Option<&'static [u8]>,
    files: fn(&str) -> Option<&'static [u8]>,
    path: &str,
) -> Option<&'static [u8]> {
    disk.or_else(|| files(path))
}

/// The game's file at `path`: the folder `cargo shuyi run --watch` named lies
/// over `files`, its [`AppConfig::files`](crate::AppConfig::files).
pub(crate) fn read(files: fn(&str) -> Option<&'static [u8]>, path: &str) -> Option<&'static [u8]> {
    read_over(crate::watched_file(path), files, path)
}

#[cfg(test)]
mod tests {
    use shuyi_assets::image::ImageInfo;

    use super::*;

    /// A table of the images at `paths`.
    fn table(paths: &[&'static str]) -> ImageTable {
        let info = ImageInfo::new(8, 8);
        ImageTable::new(paths.iter().map(|&path| (path, info)).collect())
    }

    fn changed(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|&path| path.to_owned()).collect()
    }

    /// The files of a test: `good.png` and `bad.png`.
    fn file(path: &str) -> Option<&'static [u8]> {
        match path {
            "good.png" | "font.png" => Some(b"good"),
            "bad.png" => Some(b"bad"),
            _ => None,
        }
    }

    /// A decode of the test's files, to their first byte.
    fn decode(bytes: &[u8]) -> Result<u8, String> {
        (bytes == b"good")
            .then_some(bytes[0])
            .ok_or_else(|| "not a picture".to_owned())
    }

    #[test]
    fn a_changed_path_remakes_its_own_texture_and_no_other() {
        let images = table(&["a.png", "b.png"]);
        let a = Remake::Image(images.id("a.png").unwrap());
        let b = Remake::Image(images.id("b.png").unwrap());
        let wanted = |paths: &[&str]| remakes(&changed(paths), "font.png", &images);
        assert_eq!(wanted(&["a.png"]), [a]);
        assert_eq!(wanted(&["font.png"]), [Remake::Font]);
        assert_eq!(
            wanted(&["b.png", "font.png", "a.png"]),
            [b, Remake::Font, a]
        );
        // Files that are no picture of the look's: a palette, a picture's
        // settings, a picture the game doesn't draw.
        assert_eq!(wanted(&["palette.ron", "a.png.meta", "c.png"]), []);
        assert_eq!(wanted(&[]), []);
    }

    #[test]
    fn a_path_told_twice_remakes_once() {
        let images = table(&["a.png"]);
        let a = Remake::Image(images.id("a.png").unwrap());
        let wanted = remakes(&changed(&["a.png", "a.png"]), "font.png", &images);
        assert_eq!(wanted, [a]);
    }

    #[test]
    fn a_font_picture_also_in_the_table_remakes_both() {
        let images = table(&["font.png"]);
        let image = Remake::Image(images.id("font.png").unwrap());
        let wanted = remakes(&changed(&["font.png"]), "font.png", &images);
        assert_eq!(wanted, [Remake::Font, image]);
    }

    #[test]
    fn a_picture_that_does_not_decode_keeps_its_texture_until_it_mends() {
        let images = table(&["good.png", "bad.png"]);
        let (good, bad) = (
            images.id("good.png").unwrap(),
            images.id("bad.png").unwrap(),
        );
        let wanted = [Remake::Image(good), Remake::Image(bad), Remake::Font];
        let mut errors = PictureErrors::default();
        let fresh = decode_each(&wanted, "font.png", file, decode, &mut errors);
        // No `Fresh` for the broken one: nothing replaces its old texture.
        let made: Vec<Remake> = fresh.iter().map(|f| f.remake).collect();
        assert_eq!(made, [Remake::Image(good), Remake::Font]);
        assert_eq!(fresh[0].decoded, b'g');
        assert_eq!(fresh[0].bytes, b"good");
        // Its error shows, and a good picture beside it doesn't clear it.
        let lines = |errors: &PictureErrors| errors.lines().collect::<Vec<_>>();
        assert_eq!(lines(&errors), ["bad.png: not a picture"]);
        let _ = decode_each(
            &[Remake::Image(good)],
            "font.png",
            file,
            decode,
            &mut errors,
        );
        assert!(!errors.is_empty());
        // The next good save clears it.
        let mended = |path: &str| Some(file(path).map_or(&b"good"[..], |_| b"good"));
        let fresh = decode_each(
            &[Remake::Image(bad)],
            "font.png",
            mended,
            decode,
            &mut errors,
        );
        assert_eq!(fresh.len(), 1);
        assert!(errors.is_empty(), "{:?}", lines(&errors));
    }

    #[test]
    fn a_picture_gone_from_the_files_is_an_error_too() {
        let mut errors = PictureErrors::default();
        let fresh = decode_each(&[Remake::Font], "gone.png", file, decode, &mut errors);
        assert!(fresh.is_empty());
        let lines: Vec<String> = errors.lines().collect();
        assert_eq!(lines, ["gone.png: not among the game's files"]);
    }

    /// The real decode: what the backend hands [`decode_each`].
    #[test]
    fn junk_saved_over_a_png_does_not_decode() {
        assert!(crate::render::decode(b"not a png").is_err());
        let png = include_bytes!(
            "../../../templates/shuyi-default/base/crates/game/files/font/atlas.png"
        );
        let image = crate::render::decode(png).unwrap();
        assert!(image.width > 0 && image.height > 0);
    }

    #[test]
    fn a_sprites_source_fits_a_picture_it_is_inside() {
        let rect = |x, y, w, h| PxRect { x, y, w, h };
        let size = (16, 16);
        // A picture that grew: the same pixels are still there.
        assert!(src_fits(rect(8, 8, 8, 8), (32, 32)));
        // Right up to its edges.
        assert!(src_fits(rect(0, 0, 16, 16), size));
        assert!(src_fits(rect(8, 8, 8, 8), size));
        // A picture that shrank past it, on either side.
        assert!(!src_fits(rect(8, 8, 9, 8), size));
        assert!(!src_fits(rect(8, 8, 8, 9), size));
        assert!(!src_fits(rect(-1, 0, 8, 8), size));
        assert!(!src_fits(rect(0, -1, 8, 8), size));
        // Far past it, without overflowing.
        assert!(!src_fits(rect(i32::MAX, 0, i32::MAX, 8), size));
    }

    #[test]
    fn a_file_on_disk_lies_over_the_embedded_one() {
        let disk: &'static [u8] = b"disk";
        assert_eq!(read_over(Some(disk), file, "good.png"), Some(disk));
        assert_eq!(read_over(None, file, "good.png"), Some(&b"good"[..]));
        assert_eq!(read_over(Some(disk), file, "new.png"), Some(disk));
        assert_eq!(read_over(None, file, "new.png"), None);
    }
}
