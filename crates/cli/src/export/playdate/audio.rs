//! The Playdate's sounds and music, baked for `pdc`: every sound file of the
//! audio the game ships there, and each of its music tracks, decoded (OGG
//! Vorbis or WAV), mixed down to mono and written as an IMA ADPCM WAV at its
//! own 44.1 kHz, under [`BAKED_SOUND_DIR`] and [`BAKED_MUSIC_DIR`], where
//! `shuyi-playdate` opens them. `pdc` turns each into a `.pda`.
//!
//! Decoding music takes seconds, so each file made is kept in the build's
//! cache under its source's checksum, in a folder named after [`FORMAT`], so a
//! new conversion never reuses an old file.

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use shuyi_assets::audio::{BAKED_MUSIC_DIR, BAKED_SOUND_DIR, SAMPLE_RATE, without_extension};

use crate::game::GameAudio;

/// The conversion's version: the cache folder is named after it, so a change
/// to what is written must change it.
pub const FORMAT: u32 = 1;

/// Bytes in one ADPCM block: what both tools the SDK's docs name (ffmpeg's
/// `adpcm_ima_wav` and Audacity) write for mono at 44.1 kHz, so it is what
/// `pdc` is known to read.
pub const BLOCK_BYTES: usize = 1024;

/// Samples in one mono block: the one in its header, then two a byte.
pub const BLOCK_SAMPLES: usize = (BLOCK_BYTES - 4) * 2 + 1;

/// One file to bake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    /// The file's name, for errors.
    pub name: String,
    /// What is in it.
    pub bytes: Vec<u8>,
    /// Where it goes in the `.pdx`'s source, with `.wav`:
    /// `audio/sfx/click.wav`.
    pub out: String,
}

/// The files to bake of what the game ships: each sound file once, unless the
/// build leaves it out, and the file of each music cue, read from `root`'s
/// music folder where the game doesn't read them itself. Two files that would
/// land on one path are an error, and so is a file that can't be read.
pub fn jobs(audio: &GameAudio, root: &Path) -> Result<Vec<Job>, String> {
    let mut jobs: BTreeMap<String, Job> = BTreeMap::new();
    let mut add = |name: String, bytes: Vec<u8>, dir: &str, file: &str| {
        let out = format!("{dir}/{}.wav", without_extension(file));
        match jobs.get(&out) {
            Some(job) if job.name != name => Err(format!(
                "{} and {name} would both be {out} on the Playdate",
                job.name
            )),
            Some(_) => Ok(()),
            None => {
                let job = Job {
                    name,
                    bytes,
                    out: out.clone(),
                };
                jobs.insert(out, job);
                Ok(())
            }
        }
    };
    let files = &audio.files;
    for (cue, sound) in &audio.manifest.sounds {
        for file in &sound.files {
            if (files.sound_left_out)(file) {
                continue;
            }
            let bytes = (files.sound)(file).ok_or_else(|| {
                format!(
                    "sound \"{cue}\": {}: no such file",
                    (files.sound_shown)(file)
                )
            })?;
            add(
                (files.sound_shown)(file),
                bytes.into_owned(),
                BAKED_SOUND_DIR,
                file,
            )?;
        }
    }
    for (cue, track) in &audio.manifest.music {
        let shown = format!("{}/{}", files.music_dir, track.file);
        let bytes = match files.music {
            Some(read) => read(&track.file).map(std::borrow::Cow::into_owned),
            None => fs::read(root.join(&shown)).ok(),
        };
        let bytes = bytes.ok_or_else(|| format!("music \"{cue}\": {shown}: no such file"))?;
        add(shown, bytes, BAKED_MUSIC_DIR, &track.file)?;
    }
    Ok(jobs.into_values().collect())
}

/// The mono samples of the audio file `bytes` named `name` (`.wav`, or else
/// OGG Vorbis), at [`SAMPLE_RATE`]: a stereo file's channels are averaged.
pub fn decode(name: &str, bytes: &[u8]) -> Result<Vec<i16>, String> {
    let is_wav = Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"));
    let (channels, rate, samples) = if is_wav {
        decode_wav(bytes)
    } else {
        decode_ogg(bytes)
    }
    .map_err(|e| format!("{name}: {e}"))?;
    if rate != SAMPLE_RATE {
        return Err(format!("{name}: {rate} Hz; must be {SAMPLE_RATE} Hz"));
    }
    Ok(mono(&samples, channels))
}

/// An OGG Vorbis file's channels, sample rate and interleaved samples.
fn decode_ogg(bytes: &[u8]) -> Result<(u16, u32, Vec<i16>), String> {
    let mut reader =
        lewton::inside_ogg::OggStreamReader::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let channels = u16::from(reader.ident_hdr.audio_channels);
    let rate = reader.ident_hdr.audio_sample_rate;
    let mut samples = Vec::new();
    while let Some(packet) = reader.read_dec_packet_itl().map_err(|e| e.to_string())? {
        samples.extend(packet);
    }
    Ok((channels, rate, samples))
}

/// A PCM (16-bit) or float (32-bit) WAV file's channels, sample rate and
/// interleaved samples.
fn decode_wav(bytes: &[u8]) -> Result<(u16, u32, Vec<i16>), String> {
    let reader = hound::WavReader::new(Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let spec = reader.spec();
    let samples: Result<Vec<i16>, hound::Error> = match (spec.sample_format, spec.bits_per_sample) {
        (hound::SampleFormat::Int, 16) => reader.into_samples::<i16>().collect(),
        (hound::SampleFormat::Float, 32) => reader
            .into_samples::<f32>()
            .map(|s| s.map(float_to_i16))
            .collect(),
        (format, bits) => {
            return Err(format!(
                "{bits}-bit {format:?} samples; must be 16-bit PCM or 32-bit float"
            ));
        }
    };
    Ok((
        spec.channels,
        spec.sample_rate,
        samples.map_err(|e| e.to_string())?,
    ))
}

/// A float sample (−1 to 1) as a 16-bit one, clamped.
fn float_to_i16(sample: f32) -> i16 {
    // Clamped into the i16 range first, so the cast never truncates.
    #[allow(clippy::cast_possible_truncation)]
    let sample = (sample * 32_767.0).round().clamp(-32_768.0, 32_767.0) as i16;
    sample
}

/// Interleaved `samples` of `channels` channels averaged into one.
fn mono(samples: &[i16], channels: u16) -> Vec<i16> {
    let channels = usize::from(channels.max(1));
    samples
        .chunks(channels)
        .map(|frame| {
            let sum: i32 = frame.iter().map(|&s| i32::from(s)).sum();
            // The average of i16s is an i16.
            #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
            let mean = (sum / frame.len() as i32) as i16;
            mean
        })
        .collect()
}

/// IMA ADPCM's step sizes.
const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408, 449,
    494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272,
    2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493,
    10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794, 32767,
];

/// How each code moves the step index.
const INDEX_MOVES: [i32; 8] = [-1, -1, -1, -1, 2, 4, 6, 8];

/// The IMA ADPCM coder's state: the last sample decoded and the step index.
#[derive(Debug, Clone, Copy, Default)]
struct Coder {
    predictor: i32,
    index: i32,
}

impl Coder {
    /// The step for the current index.
    fn step(self) -> i32 {
        STEPS[usize::try_from(self.index).unwrap_or(0)]
    }

    /// The 4-bit code for `sample`, moving the state as a decoder would.
    fn encode(&mut self, sample: i16) -> u8 {
        let mut diff = i32::from(sample) - self.predictor;
        let mut code = 0;
        if diff < 0 {
            code = 8;
            diff = -diff;
        }
        let mut step = self.step();
        for bit in [4, 2, 1] {
            if diff >= step {
                code |= bit;
                diff -= step;
            }
            step >>= 1;
        }
        self.decode(code);
        code
    }

    /// Moves the state by `code` and returns the sample it stands for.
    fn decode(&mut self, code: u8) -> i16 {
        let step = self.step();
        let mut delta = step >> 3;
        if code & 4 != 0 {
            delta += step;
        }
        if code & 2 != 0 {
            delta += step >> 1;
        }
        if code & 1 != 0 {
            delta += step >> 2;
        }
        if code & 8 != 0 {
            delta = -delta;
        }
        self.predictor = (self.predictor + delta).clamp(-32_768, 32_767);
        self.index = (self.index + INDEX_MOVES[usize::from(code & 7)]).clamp(0, 88);
        // Clamped to the i16 range above.
        #[allow(clippy::cast_possible_truncation)]
        let sample = self.predictor as i16;
        sample
    }
}

/// `samples` (mono, [`SAMPLE_RATE`]) as an IMA ADPCM WAV file: blocks of
/// [`BLOCK_BYTES`], the last padded with silence, and a `fact` chunk with the
/// real number of samples, to which `pdc` trims it, so a looped track loops
/// with no gap.
pub fn adpcm_wav(samples: &[i16]) -> Vec<u8> {
    let blocks = samples.len().div_ceil(BLOCK_SAMPLES).max(1);
    let mut data = Vec::with_capacity(blocks * BLOCK_BYTES);
    let mut coder = Coder::default();
    for block in 0..blocks {
        let start = block * BLOCK_SAMPLES;
        let sample = |i: usize| samples.get(start + i).copied().unwrap_or(0);
        // The header: the block's first sample as is, and the step index.
        let first = sample(0);
        coder.predictor = i32::from(first);
        data.extend_from_slice(&first.to_le_bytes());
        data.push(u8::try_from(coder.index).unwrap_or(0));
        data.push(0);
        for pair in 0..(BLOCK_BYTES - 4) {
            let low = coder.encode(sample(1 + pair * 2));
            let high = coder.encode(sample(2 + pair * 2));
            // Two 4-bit codes: the first in the low half of the byte.
            data.push(low + (high << 4));
        }
    }
    let u32_of = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let block_bytes = u16::try_from(BLOCK_BYTES).unwrap_or(u16::MAX);
    let block_samples = u16::try_from(BLOCK_SAMPLES).unwrap_or(u16::MAX);
    // A whole number of blocks a second, rounded down, as ffmpeg writes it.
    let bytes_per_second = SAMPLE_RATE * u32::from(block_bytes) / u32::from(block_samples);
    let mut wav = Vec::with_capacity(60 + data.len());
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&u32_of(52 + data.len()).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&20u32.to_le_bytes());
    wav.extend_from_slice(&0x11u16.to_le_bytes()); // IMA ADPCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&bytes_per_second.to_le_bytes());
    wav.extend_from_slice(&block_bytes.to_le_bytes());
    wav.extend_from_slice(&4u16.to_le_bytes()); // bits a sample
    wav.extend_from_slice(&2u16.to_le_bytes()); // the extra bytes below
    wav.extend_from_slice(&block_samples.to_le_bytes());
    wav.extend_from_slice(b"fact");
    wav.extend_from_slice(&4u32.to_le_bytes());
    wav.extend_from_slice(&u32_of(samples.len()).to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&u32_of(data.len()).to_le_bytes());
    wav.extend_from_slice(&data);
    wav
}

/// The cache of baked files under the build's folder `build`.
pub fn cache_dir(build: &Path) -> PathBuf {
    build.join(format!("audio-v{FORMAT}"))
}

/// Bakes `jobs` into `into`, through the cache in `cache`, which holds each
/// result under its source's checksum and length.
pub fn bake(jobs: &[Job], cache: &Path, into: &Path) -> Result<(), String> {
    for job in jobs {
        let key = format!(
            "{:08x}-{}.wav",
            crc32fast::hash(&job.bytes),
            job.bytes.len()
        );
        let cached = cache.join(key);
        if !cached.is_file() {
            write(&cached, &adpcm_wav(&decode(&job.name, &job.bytes)?))?;
        }
        let to = into.join(&job.out);
        if let Some(dir) = to.parent() {
            fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        }
        fs::copy(&cached, &to)
            .map_err(|e| format!("copy {} to {}: {e}", cached.display(), to.display()))?;
    }
    Ok(())
}

/// Writes `bytes` to `path`, making its folder.
fn write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    fs::write(path, bytes).map_err(|e| format!("write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests;
