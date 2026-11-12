//! PNG files as RGBA8 pixels, for the tools that read and write pictures,
//! and frames over time as an APNG or a GIF.

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
    let (width, height) = one_size(frames)?;
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

/// An animated PNG read back: what [`decode_apng`] returns. A set of
/// fields that may grow (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Animation {
    /// The frames, in order, each RGBA8 and the whole picture.
    pub frames: Vec<Image>,
    /// Each frame's delay, as a fraction of a second: (numerator,
    /// denominator), as the file writes it.
    pub delays: Vec<(u16, u16)>,
    /// How many times it plays: 0 is forever.
    pub plays: u32,
}

/// Decodes an animated PNG as [`encode_apng`] writes it: 8-bit RGBA, every
/// frame the whole picture at (0, 0).
///
/// # Errors
/// Bytes that aren't a PNG, a PNG that isn't animated, another colour type
/// or depth than 8-bit RGBA, or a frame that isn't the whole picture.
pub fn decode_apng(bytes: &[u8]) -> Result<Animation, String> {
    let decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    let (width, height) = (info.width, info.height);
    if (info.color_type, info.bit_depth) != (png::ColorType::Rgba, png::BitDepth::Eight) {
        return Err(format!(
            "not 8-bit RGBA but {:?} of {} bits: not an APNG frame-png writes",
            info.color_type, info.bit_depth as u8
        ));
    }
    let control = info
        .animation_control()
        .ok_or("not an animated PNG: it has no acTL chunk")?;
    let (count, plays) = (control.num_frames, control.num_plays);
    let size = reader
        .output_buffer_size()
        .ok_or("the picture is too large to decode")?;
    let mut animation = Animation {
        frames: Vec::new(),
        delays: Vec::new(),
        plays,
    };
    for i in 0..count {
        let mut buf = vec![0; size];
        let shown = reader
            .next_frame(&mut buf)
            .map_err(|e| format!("frame {}: {e}", i + 1))?;
        buf.truncate(shown.buffer_size());
        let control = reader
            .info()
            .frame_control()
            .ok_or_else(|| format!("frame {} has no fcTL chunk", i + 1))?;
        if (
            control.x_offset,
            control.y_offset,
            control.width,
            control.height,
        ) != (0, 0, width, height)
        {
            return Err(format!(
                "frame {} is {}×{} at ({}, {}), not the whole {width}×{height} picture",
                i + 1,
                control.width,
                control.height,
                control.x_offset,
                control.y_offset
            ));
        }
        animation
            .delays
            .push((control.delay_num, control.delay_den));
        animation.frames.push(Image {
            width,
            height,
            rgba: buf,
        });
    }
    Ok(animation)
}

/// Checks that `frames` has a frame and all are the size of the first;
/// returns that size.
fn one_size(frames: &[&Image]) -> Result<(u32, u32), String> {
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
    Ok((width, height))
}

/// Most colours a GIF's palette holds.
const GIF_COLOURS: usize = 256;

/// The palettes of a GIF: one for every frame, or one per frame. Each
/// lists the RGB colours in the order a scan of the pixels first meets
/// them.
#[derive(Debug, PartialEq, Eq)]
enum Palettes {
    /// All frames together have at most 256 colours.
    Global(Vec<[u8; 3]>),
    /// Each frame's own colours, at most 256 each.
    PerFrame(Vec<Vec<[u8; 3]>>),
}

/// The colours of `frames`, in scan order.
fn scan_colours<'a>(frames: impl IntoIterator<Item = &'a Image>) -> Vec<[u8; 3]> {
    let mut seen = std::collections::HashSet::new();
    let mut order = Vec::new();
    for frame in frames {
        for &[r, g, b, _] in frame.rgba.as_chunks::<4>().0 {
            let colour = [r, g, b];
            if seen.insert(colour) {
                order.push(colour);
            }
        }
    }
    order
}

/// The exact palettes of `frames`: one global palette if they have at
/// most 256 colours together, else one per frame. A pixel not opaque, or
/// a frame of more than 256 colours, is an error naming the frame (from
/// 1): a GIF of them would not be exact, and we never quantize.
fn palettes(frames: &[&Image]) -> Result<Palettes, String> {
    for (i, frame) in frames.iter().enumerate() {
        if frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|pixel| pixel[3] != 255)
        {
            return Err(format!(
                "frame {} has pixels that are not opaque, which a GIF can't show \
                 exactly: write it with --apng",
                i + 1
            ));
        }
    }
    let all = scan_colours(frames.iter().copied());
    if all.len() <= GIF_COLOURS {
        return Ok(Palettes::Global(all));
    }
    frames
        .iter()
        .enumerate()
        .map(|(i, frame)| {
            let colours = scan_colours([*frame]);
            if colours.len() > GIF_COLOURS {
                Err(format!(
                    "frame {} has {} colours, more than the {GIF_COLOURS} of a GIF: \
                     write it with --apng",
                    i + 1,
                    colours.len()
                ))
            } else {
                Ok(colours)
            }
        })
        .collect::<Result<_, _>>()
        .map(Palettes::PerFrame)
}

/// A GIF frame's delay for `every` frames of 1/60 s: hundredths of a
/// second, to the nearest, at least 2 (browsers slow a GIF whose delay is
/// under 2).
pub fn gif_delay(every: u32) -> u32 {
    (every.saturating_mul(100).saturating_add(30) / 60).max(2)
}

/// Encodes `frames`, all of one size, as a GIF that loops forever, each
/// frame shown for `every` frames of 1/60 s ([`gif_delay`]). Its palette
/// is exact: the colours of the frames, global if they have at most 256
/// together, else one per frame; never quantized or dithered.
///
/// # Errors
/// No frame, frames of two sizes, a frame larger than 65 535 pixels a
/// side, a pixel not opaque, or a frame of more than 256 colours (the
/// error names the frame and suggests `--apng`).
pub fn encode_gif(frames: &[&Image], every: u32) -> Result<Vec<u8>, String> {
    let (width, height) = one_size(frames)?;
    let side = |n: u32| {
        u16::try_from(n).map_err(|_| format!("a GIF is at most 65535 pixels a side, not {n}"))
    };
    let (width, height) = (side(width)?, side(height)?);
    let delay = u16::try_from(gif_delay(every)).map_err(|_| "--every: too long for a GIF")?;
    let palettes = palettes(frames)?;
    let flat = |colours: &[[u8; 3]]| colours.concat();
    let global = match &palettes {
        Palettes::Global(colours) => flat(colours),
        Palettes::PerFrame(_) => Vec::new(),
    };
    let mut encoder =
        gif::Encoder::new(Vec::new(), width, height, &global).map_err(|e| e.to_string())?;
    encoder
        .set_repeat(gif::Repeat::Infinite)
        .map_err(|e| e.to_string())?;
    for (i, frame) in frames.iter().enumerate() {
        let colours = match &palettes {
            Palettes::Global(colours) => colours,
            Palettes::PerFrame(each) => &each[i],
        };
        // At most 256 colours (`palettes`): each has an index.
        let index: std::collections::HashMap<[u8; 3], u8> = colours
            .iter()
            .zip(0..=u8::MAX)
            .map(|(colour, i)| (*colour, i))
            .collect();
        let pixels: Vec<u8> = frame
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&[r, g, b, _]| index[&[r, g, b]])
            .collect();
        let mut gif_frame = match &palettes {
            Palettes::Global(_) => gif::Frame::from_indexed_pixels(width, height, pixels, None),
            Palettes::PerFrame(_) => {
                gif::Frame::from_palette_pixels(width, height, pixels, flat(colours), None)
            }
        };
        gif_frame.delay = delay;
        encoder.write_frame(&gif_frame).map_err(|e| e.to_string())?;
    }
    encoder.into_inner().map_err(|e| e.to_string())
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
        let animation = decode_apng(&apng).unwrap();
        assert_eq!(animation.frames, frames);
        assert_eq!(animation.delays, [(3, 60); 3]);
        assert_eq!(animation.plays, 0);
        // A still PNG, or no PNG, is no APNG.
        let still = encode_png(2, 1, &frames[0].rgba).unwrap();
        assert!(
            decode_apng(&still)
                .unwrap_err()
                .contains("not an animated PNG")
        );
        assert!(decode_apng(b"nope").is_err());
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

    /// A two-frame APNG of a 2 × 2 picture, colour type `color`, whose
    /// second frame is the 1 × 1 pixel at (1, 1) if `part` is set.
    fn apng_of(color: png::ColorType, part: bool) -> Vec<u8> {
        let channels = color.samples();
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 2, 2);
            encoder.set_color(color);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_animated(2, 0).unwrap();
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&vec![7; 4 * channels]).unwrap();
            if part {
                writer.set_frame_dimension(1, 1).unwrap();
                writer.set_frame_position(1, 1).unwrap();
                writer.write_image_data(&vec![9; channels]).unwrap();
            } else {
                writer.write_image_data(&vec![9; 4 * channels]).unwrap();
            }
            writer.finish().unwrap();
        }
        out
    }

    #[test]
    fn an_apng_that_isnt_whole_rgba_frames_is_refused() {
        let whole = decode_apng(&apng_of(png::ColorType::Rgba, false)).unwrap();
        assert_eq!(whole.frames.len(), 2);
        assert_eq!(whole.frames[1].rgba, [9; 16]);
        assert_eq!(
            decode_apng(&apng_of(png::ColorType::Rgba, true)).unwrap_err(),
            "frame 2 is 1×1 at (1, 1), not the whole 2×2 picture"
        );
        assert!(
            decode_apng(&apng_of(png::ColorType::Rgb, false))
                .unwrap_err()
                .starts_with("not 8-bit RGBA but Rgb of 8 bits")
        );
        assert!(decode_apng(&apng_of(png::ColorType::Grayscale, false)).is_err());
    }

    /// An opaque frame of `n` pixels in a row, colours `first` to
    /// `first + n − 1` (each a different RGB).
    fn colours(first: u32, n: u32) -> Image {
        let rgba = (first..first + n)
            .flat_map(|c| {
                let [_, r, g, b] = c.to_be_bytes();
                [r, g, b, 255]
            })
            .collect();
        Image {
            width: n,
            height: 1,
            rgba,
        }
    }

    #[test]
    fn the_palette_is_global_per_frame_or_too_many() {
        // 256 together: one global palette, in scan order.
        let (a, b) = (colours(0, 200), colours(56, 200));
        let Palettes::Global(global) = palettes(&[&a, &b]).unwrap() else {
            panic!("not global");
        };
        assert_eq!(global.len(), 256);
        assert_eq!((global[0], global[255]), ([0, 0, 0], [0, 0, 255]));
        // 200 each, 300 together: one palette each.
        let c = colours(100, 200);
        let b = colours(300, 200);
        let Palettes::PerFrame(each) = palettes(&[&c, &b]).unwrap() else {
            panic!("not per frame");
        };
        assert_eq!((each[0].len(), each[1].len()), (200, 200));
        assert_eq!(each[1][0], [0, 1, 44]);
        // 257 in frame 2: an error naming it.
        let many = colours(0, 257);
        assert_eq!(
            palettes(&[&c, &many]).unwrap_err(),
            "frame 2 has 257 colours, more than the 256 of a GIF: write it with --apng"
        );
        // A pixel not opaque.
        let mut clear = colours(0, 2);
        clear.rgba[7] = 128;
        assert!(
            palettes(&[&clear])
                .unwrap_err()
                .starts_with("frame 1 has pixels that are not opaque")
        );
    }

    #[test]
    fn a_gif_delay_is_hundredths_to_the_nearest_at_least_2() {
        let delays = [1, 3, 6, 2000].map(gif_delay);
        assert_eq!(delays, [2, 5, 10, 3333]);
        assert_eq!(gif_delay(2), 3);
    }

    /// The frames, delays and repeat of `bytes`, decoded as RGBA, and
    /// whether each frame has its own palette.
    fn decode_gif(bytes: &[u8]) -> (Vec<Vec<u8>>, Vec<u16>, gif::Repeat, Vec<bool>) {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(bytes).unwrap();
        let (mut frames, mut delays, mut local) = (Vec::new(), Vec::new(), Vec::new());
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            frames.push(frame.buffer.to_vec());
            delays.push(frame.delay);
            local.push(frame.palette.is_some());
        }
        (frames, delays, decoder.repeat(), local)
    }

    #[test]
    fn a_gif_decodes_to_its_frames_and_their_delays_looping_forever() {
        let shape = |mut image: Image| {
            // 2 rows of 100.
            image.width = 100;
            image.height = 2;
            image
        };
        let frames = [shape(colours(0, 200)), shape(colours(56, 200))];
        let refs: Vec<&Image> = frames.iter().collect();
        let (decoded, delays, repeat, local) = decode_gif(&encode_gif(&refs, 3).unwrap());
        assert_eq!(decoded, [frames[0].rgba.clone(), frames[1].rgba.clone()]);
        assert_eq!(
            (delays, repeat, local),
            (vec![5, 5], gif::Repeat::Infinite, vec![false, false])
        );
        // 300 together: a palette each, and still exact.
        let frames = [shape(colours(0, 200)), shape(colours(100, 200))];
        let refs: Vec<&Image> = frames.iter().collect();
        let (decoded, delays, _, local) = decode_gif(&encode_gif(&refs, 1).unwrap());
        assert_eq!(decoded, [frames[0].rgba.clone(), frames[1].rgba.clone()]);
        assert_eq!((delays, local), (vec![2, 2], vec![true, true]));
        // No frame, two sizes, or too many colours are errors.
        assert!(encode_gif(&[], 3).is_err());
        let wide = colours(0, 3);
        assert_eq!(
            encode_gif(&[&colours(0, 2), &wide], 3).unwrap_err(),
            "frame 2 is 3×1, not 2×1 as the first"
        );
        assert!(encode_gif(&[&colours(0, 257)], 3).is_err());
        assert!(
            encode_gif(&[&colours(0, 70_000)], 3)
                .unwrap_err()
                .contains("65535")
        );
    }
}
