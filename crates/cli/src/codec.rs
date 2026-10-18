//! PNG files as RGBA8 pixels, for the tools that read and write pictures.

use std::io::BufWriter;

/// Encodes RGBA8 pixels as PNG.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(BufWriter::new(&mut out), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::High);
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        writer.write_image_data(rgba).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

/// Decodes a PNG to (width, height, RGBA8 pixels).
pub fn decode_png(bytes: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    // Any PNG comes out as RGBA8.
    let mut wanted = png::Transformations::EXPAND;
    wanted.insert(png::Transformations::ALPHA);
    wanted.insert(png::Transformations::STRIP_16);
    decoder.set_transformations(wanted);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let size = reader
        .output_buffer_size()
        .ok_or("the picture is too large to decode")?;
    let mut buf = vec![0; size];
    let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
    buf.truncate(info.buffer_size());
    Ok((info.width, info.height, buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_round_trips() {
        let rgba: Vec<u8> = (0..2 * 3 * 4)
            .map(|i| u8::try_from(i * 10).unwrap())
            .collect();
        let png = encode_png(2, 3, &rgba).unwrap();
        assert_eq!(decode_png(&png).unwrap(), (2, 3, rgba));
        assert!(decode_png(b"nope").is_err());
        assert!(encode_png(2, 3, &[0; 4]).is_err());
    }
}
