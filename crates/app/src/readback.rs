//! The window read back as a PNG: what the player sees, for a tool that
//! asks for it (the live link's picture, ADR-0047).

use image::ImageEncoder as _;
use image::codecs::png::PngEncoder;
use macroquad::prelude::get_screen_data;

/// The window as drawn so far this frame, as a PNG, with its width and
/// height in pixels. Called after the frame is drawn and before
/// `next_frame` shows it.
pub(crate) fn window_png() -> Result<(Vec<u8>, u32, u32), String> {
    let image = get_screen_data();
    let (width, height) = (u32::from(image.width), u32::from(image.height));
    let rgba = upright(&image.bytes, usize::from(image.width) * 4);
    Ok((png(&rgba, width, height)?, width, height))
}

/// The rows of `bytes`, `row` bytes each, in the other order (the window
/// reads back bottom row first), every pixel opaque.
pub(crate) fn upright(bytes: &[u8], row: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    if row == 0 {
        return out;
    }
    for line in bytes.chunks_exact(row).rev() {
        out.extend_from_slice(line);
    }
    for alpha in out.iter_mut().skip(3).step_by(4) {
        *alpha = u8::MAX;
    }
    out
}

/// `rgba`, `width` × `height` pixels, as a PNG file.
pub(crate) fn png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, String> {
    let pixels = u64::from(width) * u64::from(height);
    if u64::try_from(rgba.len()).ok() != pixels.checked_mul(4) {
        return Err(format!(
            "the window's picture is {} bytes, not {width} × {height} pixels",
            rgba.len()
        ));
    }
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(rgba, width, height, image::ColorType::Rgba8)
        .map_err(|e| format!("the window's picture didn't encode: {e}"))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_are_turned_upright_and_made_opaque() {
        // Two rows of two pixels, bottom row first, half transparent.
        let bottom = [1, 2, 3, 0, 4, 5, 6, 0];
        let top = [7, 8, 9, 128, 10, 11, 12, 128];
        let read: Vec<u8> = bottom.iter().chain(&top).copied().collect();
        assert_eq!(
            upright(&read, 8),
            [7, 8, 9, 255, 10, 11, 12, 255, 1, 2, 3, 255, 4, 5, 6, 255]
        );
        assert_eq!(upright(&[], 0), Vec::<u8>::new());
    }

    #[test]
    fn a_picture_encodes_as_a_png_that_decodes_to_it() {
        let rgba = upright(&[1, 2, 3, 0, 4, 5, 6, 0], 4);
        let file = png(&rgba, 1, 2).unwrap();
        assert_eq!(&file[..8], b"\x89PNG\r\n\x1a\n");
        let decoded = image::load_from_memory(&file).unwrap().to_rgba8();
        assert_eq!((decoded.width(), decoded.height()), (1, 2));
        assert_eq!(decoded.into_raw(), rgba);
        assert!(png(&rgba, 3, 3).is_err());
    }
}
