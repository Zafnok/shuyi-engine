//! The Playdate's audio bake, on tones and files made here.

use std::borrow::Cow;
use std::f64::consts::TAU;

use shuyi_assets::audio::{AudioManifest, Bytes, CreditRef, Files, MusicCue, SoundCue};

use super::*;

/// A 440 Hz tone, `len` samples at 44.1 kHz, at half volume.
fn tone(len: usize) -> Vec<i16> {
    (0..len)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f64 / f64::from(SAMPLE_RATE);
            #[allow(clippy::cast_possible_truncation)]
            let s = ((TAU * 440.0 * t).sin() * 16_000.0) as i16;
            s
        })
        .collect()
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// The samples of an ADPCM WAV written by [`adpcm_wav`], as the standard IMA
/// decoder reads them (blocks only, padding included).
fn decode_adpcm(wav: &[u8]) -> Vec<i16> {
    let data = &wav[60..];
    let mut out = Vec::new();
    for block in data.chunks(BLOCK_BYTES) {
        let first = i16::from_le_bytes([block[0], block[1]]);
        let mut coder = Coder {
            predictor: i32::from(first),
            index: i32::from(block[2]),
        };
        out.push(first);
        for &byte in &block[4..] {
            out.push(coder.decode(byte & 0x0f));
            out.push(coder.decode(byte >> 4));
        }
    }
    out
}

/// `samples` (interleaved, `channels`) as a 16-bit PCM WAV.
fn pcm_wav(samples: &[i16], channels: u16) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
    for &s in samples {
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
    bytes.into_inner()
}

/// The test tone through the whole conversion: its WAV header, its length and
/// its sound.
#[test]
fn a_test_tone_becomes_an_ima_adpcm_wav_with_its_length() {
    // 1.5 blocks: the last is padded.
    let samples = tone(BLOCK_SAMPLES * 3 / 2);
    let pcm = pcm_wav(&samples, 1);
    let decoded = decode("tone.wav", &pcm).unwrap();
    assert_eq!(decoded, samples);
    let wav = adpcm_wav(&decoded);
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(u32_at(&wav, 4) as usize, wav.len() - 8);
    assert_eq!(&wav[8..16], b"WAVEfmt ");
    assert_eq!(u32_at(&wav, 16), 20);
    assert_eq!(u16_at(&wav, 20), 0x11, "IMA ADPCM");
    assert_eq!(u16_at(&wav, 22), 1, "mono");
    assert_eq!(u32_at(&wav, 24), 44_100);
    assert_eq!(u32_at(&wav, 28), 22_125);
    assert_eq!(u16_at(&wav, 32), 1024);
    assert_eq!(u16_at(&wav, 34), 4);
    assert_eq!(u16_at(&wav, 36), 2);
    assert_eq!(u16_at(&wav, 38), 2041);
    assert_eq!(&wav[40..44], b"fact");
    assert_eq!(u32_at(&wav, 48) as usize, samples.len());
    assert_eq!(&wav[52..56], b"data");
    assert_eq!(u32_at(&wav, 56) as usize, 2 * BLOCK_BYTES);
    assert_eq!(wav.len(), 60 + 2 * BLOCK_BYTES);
    // It sounds like the tone: once the coder has caught up with the tone's
    // start, each sample is close; the padding is silent.
    let back = decode_adpcm(&wav);
    assert_eq!(back.len(), 2 * BLOCK_SAMPLES);
    let worst = samples
        .iter()
        .zip(&back)
        .skip(64)
        .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
        .max()
        .unwrap();
    assert!(worst < 300, "{worst}");
    assert!(back[samples.len() + 64..].iter().all(|s| s.abs() < 64));
}

/// The standard IMA ADPCM tables and steps, worked by hand: the step at index
/// 10 is 19.
#[test]
fn the_coder_follows_the_ima_standard() {
    let at = |index| Coder {
        predictor: 100,
        index,
    };
    // Codes 0–3 step the index down one; 4–7 up 2, 4, 6, 8.
    for (code, index) in [
        (0, 9),
        (1, 9),
        (2, 9),
        (3, 9),
        (4, 12),
        (5, 14),
        (6, 16),
        (7, 18),
    ] {
        let mut coder = at(10);
        coder.decode(code);
        assert_eq!(coder.index, index, "code {code}");
    }
    // 7: 19/8 + 19 + 19/2 + 19/4 = 2 + 19 + 9 + 4; 15 is its negative.
    assert_eq!(at(10).decode(7), 134);
    assert_eq!(at(10).decode(15), 66);
    assert_eq!(at(10).decode(0), 102);
    // The same sample again is code 0, not a negative 0.
    assert_eq!(at(10).encode(100), 0);
    assert_eq!(at(10).encode(134), 7);
    assert_eq!(at(10).encode(66), 15);
    // A byte holds two codes, the first in its low half.
    let wav = adpcm_wav(&[0, 34, -34]);
    assert_eq!(wav[64] & 0x0f, 7);
    assert_eq!(wav[64] >> 4, 0x0f);
}

#[test]
fn a_block_starts_with_its_first_sample_exactly() {
    let samples = tone(BLOCK_SAMPLES * 2);
    let back = decode_adpcm(&adpcm_wav(&samples));
    assert_eq!(back[0], samples[0]);
    assert_eq!(back[BLOCK_SAMPLES], samples[BLOCK_SAMPLES]);
    // Nothing at all is still one (silent) block.
    let empty = adpcm_wav(&[]);
    assert_eq!(empty.len(), 60 + BLOCK_BYTES);
    assert_eq!(u32_at(&empty, 48), 0);
    // A full-scale square wave never wraps around: once caught up, each half
    // stays on its side.
    let square: Vec<i16> = (0..4000)
        .map(|i| if i / 50 % 2 == 0 { i16::MAX } else { i16::MIN })
        .collect();
    let back = decode_adpcm(&adpcm_wav(&square));
    assert!(back[90..100].iter().all(|&s| s < -20_000));
    assert!(back[140..150].iter().all(|&s| s > 20_000));
}

#[test]
fn stereo_is_averaged_and_float_is_scaled() {
    let pcm = pcm_wav(&[100, 300, -100, -301, 7, 7], 2);
    assert_eq!(decode("a.wav", &pcm), Ok(vec![200, -200, 7]));
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut bytes = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
    for s in [0.0f32, 0.5, -1.0, 2.0] {
        writer.write_sample(s).unwrap();
    }
    writer.finalize().unwrap();
    assert_eq!(
        decode("f.wav", &bytes.into_inner()),
        Ok(vec![0, 16_384, -32_767, 32_767])
    );
}

#[test]
fn a_file_it_cannot_play_is_an_error_naming_it() {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 22_050,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut bytes = Cursor::new(Vec::new());
    hound::WavWriter::new(&mut bytes, spec)
        .unwrap()
        .finalize()
        .unwrap();
    assert_eq!(
        decode("slow.wav", &bytes.into_inner()),
        Err("slow.wav: 22050 Hz; must be 44100 Hz".to_owned())
    );
    let eight = hound::WavSpec {
        bits_per_sample: 8,
        sample_rate: SAMPLE_RATE,
        ..spec
    };
    let mut bytes = Cursor::new(Vec::new());
    hound::WavWriter::new(&mut bytes, eight)
        .unwrap()
        .finalize()
        .unwrap();
    let error = decode("byte.wav", &bytes.into_inner()).unwrap_err();
    assert!(error.contains("8-bit"), "{error}");
    let not_ogg = decode("x.ogg", b"not ogg").unwrap_err();
    assert!(not_ogg.starts_with("x.ogg: "), "{not_ogg}");
    let not_wav = decode("x.wav", b"not wav").unwrap_err();
    assert!(not_wav.starts_with("x.wav: "), "{not_wav}");
}

/// A sound file of the game's: `sfx/gone.wav` isn't there.
fn sound(file: &str) -> Bytes {
    (file != "sfx/gone.wav").then(|| Cow::Owned(file.as_bytes().to_vec()))
}

fn shown(file: &str) -> String {
    format!("base/{file}")
}

fn left_out(file: &str) -> bool {
    file == "sfx/out.wav"
}

fn music(file: &str) -> Bytes {
    (file == "theme.ogg").then_some(Cow::Borrowed(b"theme" as &[u8]))
}

/// The manifest with `sounds` as one cue each, and the track `theme`.
fn manifest(sounds: &[&str]) -> AudioManifest {
    let mut m = AudioManifest::default();
    for (at, file) in sounds.iter().enumerate() {
        let cue = SoundCue::new(vec![(*file).to_owned()], 100, CreditRef::Own);
        m.sounds.insert(format!("cue{at}"), cue);
    }
    let track = MusicCue {
        file: "theme.ogg".into(),
        volume: 100,
        looped: true,
        length_ms: 1000,
        credit: CreditRef::Own,
    };
    m.music.insert("theme".into(), track);
    m
}

fn audio(sounds: &[&str]) -> GameAudio {
    let files = Files::new(&sound, &shown, &left_out, "music").with_music(Some(&music));
    GameAudio::new(manifest(sounds), files)
}

#[test]
fn each_sound_file_goes_once_and_each_track_of_what_ships() {
    let audio = audio(&["sfx/a.wav", "sfx/b.ogg", "sfx/a.wav", "sfx/out.wav"]);
    let jobs = jobs(&audio, Path::new("/game")).unwrap();
    let outs: Vec<&str> = jobs.iter().map(|job| job.out.as_str()).collect();
    assert_eq!(
        outs,
        ["audio/sfx/a.wav", "audio/sfx/b.wav", "music/theme.wav"]
    );
    assert_eq!(jobs[0].name, "base/sfx/a.wav");
    assert_eq!(jobs[0].bytes, b"sfx/a.wav");
    assert_eq!(jobs[2].name, "music/theme.ogg");
    assert_eq!(jobs[2].bytes, b"theme");
}

#[test]
fn a_track_the_game_leaves_out_of_what_ships_is_not_baked() {
    let mut audio = audio(&[]);
    audio.manifest.music.clear();
    assert_eq!(jobs(&audio, Path::new("/game")), Ok(Vec::new()));
}

#[test]
fn a_file_that_is_not_there_is_an_error_naming_it() {
    assert_eq!(
        jobs(&audio(&["sfx/gone.wav"]), Path::new("/game")),
        Err("sound \"cue0\": base/sfx/gone.wav: no such file".to_owned())
    );
    // The game can't read its music folder: it is beside its project.
    let mut unseen = audio(&[]);
    unseen.files.music = None;
    assert_eq!(
        jobs(&unseen, Path::new("/nowhere")),
        Err("music \"theme\": music/theme.ogg: no such file".to_owned())
    );
}

#[test]
fn two_files_on_one_path_are_an_error() {
    assert_eq!(
        jobs(&audio(&["sfx/a.wav", "sfx/a.ogg"]), Path::new("/game")),
        Err(
            "base/sfx/a.wav and base/sfx/a.ogg would both be audio/sfx/a.wav on the Playdate"
                .to_owned()
        )
    );
}

#[test]
fn the_bake_is_cached_by_what_the_source_holds() {
    let dir = std::env::temp_dir().join(format!("shuyi-playdate-audio-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let cache = cache_dir(&dir);
    assert!(cache.ends_with(format!("audio-v{FORMAT}")));
    let into = dir.join("Source");
    let job = |bytes: Vec<u8>| Job {
        name: "beep.wav".to_owned(),
        bytes,
        out: "audio/beep.wav".to_owned(),
    };
    let first = job(pcm_wav(&tone(3000), 1));
    bake(std::slice::from_ref(&first), &cache, &into).unwrap();
    let out = into.join("audio/beep.wav");
    assert_eq!(
        fs::metadata(&out).unwrap().len(),
        60 + 2 * BLOCK_BYTES as u64
    );
    // The same source again doesn't decode: the cached file is used.
    let key = fs::read_dir(&cache)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    write(&key, b"cached").unwrap();
    bake(std::slice::from_ref(&first), &cache, &into).unwrap();
    assert_eq!(fs::read(&out).unwrap(), b"cached");
    // Another source is baked anew.
    bake(&[job(pcm_wav(&tone(10), 1))], &cache, &into).unwrap();
    assert_eq!(fs::metadata(&out).unwrap().len(), 60 + BLOCK_BYTES as u64);
    assert!(bake(&[job(b"not wav".to_vec())], &cache, &into).is_err());
    let _ = fs::remove_dir_all(&dir);
}
