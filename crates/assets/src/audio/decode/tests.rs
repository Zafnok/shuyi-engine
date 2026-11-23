//! `decode_to_wav`, `pcm16_wav` and `track_length_ms` on ticket 2203's
//! fixtures (`tests/fixtures/formats/`).
use proptest::prelude::*;

use super::*;
use crate::audio::tests::{ogg_file, wav};
use crate::audio::{Flac, FlacTrack, Mp3, Mp3Track, Wav, wav_format};
use crate::format::builtin_tests::{CODEC_FIXTURES, after, bytes, damaged};
use crate::format::codec_tests::{FLAC, MP3, MP3_UNTAGGED};

/// What `format` reads from `bytes`, alone.
fn read<F: Format>(format: &F, name: &str, bytes: &[u8]) -> F::Asset {
    format
        .decode(&AssetPath::new(name), bytes, &DecodeCx::new())
        .unwrap_or_else(|e| panic!("{name}: {e:?}"))
}

/// A WAV's samples: what follows its 44-byte header.
fn samples(wav: &[u8]) -> Vec<i16> {
    wav[HEADER..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes(*pair))
        .collect()
}

#[test]
fn a_pcm16_wav_is_its_44_byte_header_then_its_samples() {
    let mono = pcm16_wav(1, 44_100, &[1, -2]);
    let header: &[u8] = b"RIFF\x28\0\0\0WAVEfmt \x10\0\0\0\x01\0\x01\0\x44\xac\0\0\x88\x58\x01\0\x02\0\x10\0data\x04\0\0\0";
    assert_eq!(&mono[..HEADER], header);
    assert_eq!(&mono[HEADER..], [1, 0, 0xfe, 0xff]);
    let stereo = pcm16_wav(2, 48_000, &[3, 4, 5, 6]);
    let header: &[u8] = b"RIFF\x2c\0\0\0WAVEfmt \x10\0\0\0\x01\0\x02\0\x80\xbb\0\0\0\xee\x02\0\x04\0\x10\0data\x08\0\0\0";
    assert_eq!(&stereo[..HEADER], header);
    assert_eq!(samples(&stereo), [3, 4, 5, 6]);
    // What the engine's own WAV format reads.
    let played = pcm16_wav(2, 44_100, &[3, 4]);
    assert_eq!(read(&Wav, "a.wav", &played), Sound::stereo(44_100));
    assert_eq!(wav_format(&stereo), Some((2, 48_000)));
    assert_eq!(pcm16_wav(1, 44_100, &[]).len(), HEADER);
}

#[test]
fn a_flac_file_becomes_a_wav_of_its_samples_shifted_to_16_bits() {
    let wav = decode_to_wav("sfx/tone.flac", FLAC).unwrap().unwrap();
    assert_eq!(read(&Wav, "a.wav", &wav), read(&Flac, "a.flac", FLAC));
    let mut reader = claxon::FlacReader::new(FLAC).unwrap();
    let bits = reader.streaminfo().bits_per_sample;
    let want: Vec<i16> = reader
        .samples()
        .map(|sample| flac_sample(sample.unwrap(), bits))
        .collect();
    assert_eq!(want.len(), 2 * 2205, "50 ms, stereo");
    assert_eq!(samples(&wav), want);
}

#[test]
fn a_flac_samples_width_becomes_16_bits() {
    assert_eq!(flac_sample(-5, 16), -5);
    assert_eq!(flac_sample(0x7f_ffff, 24), i16::MAX);
    assert_eq!(flac_sample(-0x80_0000, 24), i16::MIN);
    assert_eq!(flac_sample(0x100, 24), 1);
    assert_eq!(flac_sample(127, 8), 127 << 8);
    assert_eq!(flac_sample(-128, 8), i16::MIN);
    // Only a broken file has a sample wider than its bits.
    assert_eq!(flac_sample(1 << 20, 16), i16::MAX);
    assert_eq!(flac_sample(-(1 << 20), 16), i16::MIN);
    assert_eq!(flac_sample(i32::MAX, 32), i16::MAX);
}

#[test]
fn an_mp3_file_becomes_a_wav_of_its_header_gapless() {
    for fixture in [MP3, MP3_UNTAGGED] {
        let wav = decode_to_wav("tone.mp3", fixture).unwrap().unwrap();
        assert_eq!(read(&Wav, "a.wav", &wav), read(&Mp3, "a.mp3", fixture));
        let decoded = samples(&wav);
        // Mono: a sample a frame, as many as the decoder counts.
        let counted = nanomp3::SliceReader::<i16>::new(fixture).total_samples();
        assert_eq!(Some(decoded.len() as u64), counted);
        assert!(
            decoded.iter().any(|&sample| sample != 0),
            "a tone, not silence"
        );
    }
    // The LAME tag says the encoder's padding: what is left is the tone.
    let tagged = decode_to_wav("tone.mp3", MP3).unwrap().unwrap();
    assert_eq!(samples(&tagged).len(), 2205);
}

#[test]
fn only_a_flac_or_mp3_file_not_already_a_wav_is_decoded() {
    let a_wav = pcm16_wav(2, 44_100, &[1, 2]);
    assert_eq!(decode_to_wav("a.wav", &a_wav), None);
    assert_eq!(decode_to_wav("a.ogg", &ogg_file(44_100, 1)), None);
    assert_eq!(decode_to_wav("a.png", FLAC), None);
    assert_eq!(decode_to_wav("flac", FLAC), None);
    assert_eq!(decode_to_wav(".flac", FLAC), None);
    assert_eq!(decode_to_wav("hooked.flac", &a_wav), None);
    assert_eq!(decode_to_wav("hooked.mp3", &wav(2, 44_100, 1)), None);
    // Bytes that only start like a WAV aren't one.
    assert!(
        decode_to_wav("a.flac", b"RIFF\0\0\0\0AVI ")
            .unwrap()
            .is_err()
    );
}

#[test]
fn broken_bytes_are_an_error() {
    for name in ["a.flac", "a.mp3"] {
        for bytes in [&b""[..], b"fLaC", b"junk", &FLAC[..40], &[0xff; 64]] {
            let decoded = decode_to_wav(name, bytes).unwrap();
            assert!(decoded.is_err(), "{name}: {bytes:?}");
        }
    }
    let flac = decode_to_wav("a.flac", b"junk").unwrap().unwrap_err();
    assert!(flac.starts_with("not a readable FLAC file: "), "{flac}");
    let mp3 = decode_to_wav("a.mp3", b"junk").unwrap();
    assert_eq!(mp3, Err("not an MP3 file".to_owned()));
}

#[test]
fn decoding_stops_at_the_limit() {
    let whole = decode_to_wav("a.flac", FLAC).unwrap().unwrap();
    // 2,205 stereo frames of 16 bits, and the header.
    assert_eq!(whole.len(), HEADER + 2205 * 4);
    let limit = |limit| decode_within("a.flac", FLAC, limit).unwrap();
    assert_eq!(limit(whole.len()), Ok(whole.clone()));
    assert_eq!(limit(whole.len() - 1), Err(too_big(whole.len() - 1)));
    assert_eq!(limit(HEADER), Err(too_big(HEADER)));
    let mp3 = decode_to_wav("a.mp3", MP3).unwrap().unwrap();
    let limit = |limit| decode_within("a.mp3", MP3, limit).unwrap();
    assert_eq!(limit(mp3.len()), Ok(mp3.clone()));
    assert_eq!(limit(mp3.len() - 2), Err(too_big(mp3.len() - 2)));
    assert_eq!(
        too_big(MAX_DECODED_BYTES),
        "decoded, it would be more than 268435456 bytes"
    );
}

#[test]
fn a_tracks_length_is_read_by_its_format() {
    assert_eq!(
        track_length_ms("a.ogg", &ogg_file(44_100, 88_200)),
        Some(2000)
    );
    assert_eq!(track_length_ms("a.ogg", FLAC), None);
    let flac = read(&FlacTrack, "a.flac", FLAC).length_ms;
    assert_eq!(flac, 50);
    assert_eq!(track_length_ms("music/a.flac", FLAC), Some(flac));
    for mp3 in [MP3, MP3_UNTAGGED] {
        let length = read(&Mp3Track, "a.mp3", mp3).length_ms;
        assert_eq!(track_length_ms("a.mp3", mp3), Some(length));
    }
    assert_eq!(track_length_ms("a.mp3", b"junk"), None);
    assert_eq!(track_length_ms("a.wav", &wav(2, 44_100, 1)), None);
    assert_eq!(track_length_ms("a.flac.txt", FLAC), None);
}

#[test]
fn a_flac_or_mp3_sound_is_checked_by_its_format() {
    assert_eq!(coded_problem("a.flac", FLAC), Some(None));
    assert_eq!(coded_problem("a.mp3", MP3), Some(None));
    assert_eq!(coded_problem("a.ogg", MP3), None);
    // Bytes a game's hook decoded are checked as a WAV.
    assert_eq!(coded_problem("a.flac", &wav(2, 44_100, 1)), Some(None));
    assert_eq!(
        coded_problem("a.mp3", &wav(2, 48_000, 1)),
        Some(Some("48000 Hz; must be 44100 Hz".to_owned()))
    );
    assert_eq!(
        coded_problem("a.flac", b"junk"),
        Some(Some("not a FLAC file".to_owned()))
    );
    assert_eq!(
        coded_problem("a.mp3", b"junk"),
        Some(Some("not an MP3 file".to_owned()))
    );
}

#[test]
fn a_format_whose_feature_is_off_names_it() {
    let off = |name, coded| read_problem(name, FLAC, coded, None);
    assert_eq!(
        off("a.flac", Coded::Flac).as_deref(),
        Some("a .flac file needs the flac feature of shuyi-assets")
    );
    assert_eq!(
        off("a.mp3", Coded::Mp3).as_deref(),
        Some("a .mp3 file needs the mp3 feature of shuyi-assets")
    );
    // A WAV needs no feature.
    assert_eq!(
        read_problem("a.flac", &wav(1, 44_100, 1), Coded::Flac, None),
        None
    );
    assert_eq!(Coded::of("A.FLAC"), None, "extensions are lower case");
}

proptest! {
    #[test]
    fn no_bytes_panic_decode_to_wav_or_pass_its_limit(
        random in bytes(),
        flac in after(b"fLaC"),
        mp3 in after(&[0xff, 0xfb]),
        tagged in after(b"ID3"),
        damaged in prop::sample::select(CODEC_FIXTURES).prop_flat_map(damaged),
        limit in HEADER..20_000_usize,
    ) {
        for bytes in [&random, &flac, &mp3, &tagged, &damaged] {
            for name in ["a.flac", "a.mp3"] {
                let _ = track_length_ms(name, bytes);
                let _ = coded_problem(name, bytes);
                if let Some(Ok(wav)) = decode_to_wav(name, bytes) {
                    prop_assert!(wav.len() <= MAX_DECODED_BYTES);
                }
                if let Some(Ok(wav)) = decode_within(name, bytes, limit) {
                    prop_assert!(wav.len() <= limit);
                }
            }
        }
    }
}
