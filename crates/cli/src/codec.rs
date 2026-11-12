//! PNG files as RGBA8 pixels, for the tools that read and write pictures.

use std::io::BufWriter;

use shuyi_assets::image::Image;

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

/// Encodes `frames`, all of one size, as an animated PNG that loops
/// forever, each frame shown for `every` frames of 1/60 s.
pub fn encode_apng(frames: &[&Image], every: u16) -> Result<Vec<u8>, String> {
    let first = frames.first().ok_or("an animation needs a frame")?;
    let (width, height) = (first.width, first.height);
    if let Some(i) = frames
        .iter()
        .position(|f| (f.width, f.height) != (width, height))
    {
        return Err(format!(
            "frame {} is {}×{}, not {width}×{height} as the first",
            i + 1,
            frames[i].width,
            frames[i].height
        ));
    }
    let count = u32::try_from(frames.len()).map_err(|_| "too many frames")?;
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(BufWriter::new(&mut out), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::High);
        // 0 plays: it loops forever.
        encoder.set_animated(count, 0).map_err(|e| e.to_string())?;
        encoder
            .set_frame_delay(every, 60)
            .map_err(|e| e.to_string())?;
        let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
        for frame in frames {
            writer
                .write_image_data(&frame.rgba)
                .map_err(|e| e.to_string())?;
        }
        writer.finish().map_err(|e| e.to_string())?;
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

    #[test]
    fn an_apng_decodes_to_its_frames_and_their_delays_looping_forever() {
        let frame = |v: u8| Image {
            width: 2,
            height: 1,
            rgba: vec![v, 0, 255 - v, 255, 0, v, 0, 255],
        };
        let frames = [frame(10), frame(20), frame(30)];
        let apng = encode_apng(&frames.iter().collect::<Vec<_>>(), 3).unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(&apng))
            .read_info()
            .unwrap();
        let control = *reader.info().animation_control().unwrap();
        assert_eq!((control.num_frames, control.num_plays), (3, 0));
        for want in &frames {
            let mut buf = vec![0; reader.output_buffer_size().unwrap()];
            reader.next_frame(&mut buf).unwrap();
            let shown = reader.info().frame_control().unwrap();
            assert_eq!((shown.delay_num, shown.delay_den), (3, 60));
            assert_eq!(buf, want.rgba);
        }
        // No frame, or frames of two sizes, are errors.
        assert!(encode_apng(&[], 3).is_err());
        let wide = Image {
            width: 3,
            height: 1,
            rgba: vec![0; 12],
        };
        let error = encode_apng(&[&frames[0], &wide], 3).unwrap_err();
        assert_eq!(error, "frame 2 is 3×1, not 2×1 as the first");
    }
}
