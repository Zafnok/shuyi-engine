//! The WAV and Ogg Vorbis adapters on fixtures: files written here from the
//! format's specification, and the parity gates' Ogg files. Whether an Ogg
//! file's samples are right is the gates' to judge, against a separate decoder
//! (`cargo xtask sound-parity`); this holds what is true of them whichever.

use proptest::prelude::*;
use shuyi_sound::vorbis::VorbisSource;
use shuyi_sound::wav::WavSource;
use shuyi_sound::{Channels, Error, Limits, Pcm, Source, read_all};

const MONO: &[u8] = include_bytes!("../../../tools/sound-parity/corpus/fmt_22k_m.ogg");
const STEREO: &[u8] = include_bytes!("../../../tools/sound-parity/corpus/fmt_44k_s.ogg");
const REGION: &[u8] = include_bytes!("../../../tools/sound-parity/corpus/intro_loop.ogg");
const LONG: &[u8] = include_bytes!("../../../tools/sound-parity/corpus/long.ogg");

/// A WAV of `format_tag` (1 is integer, 3 float), with `data` as its samples.
fn wav(format_tag: u16, channels: u16, rate: u32, bits: u16, data: &[u8]) -> Vec<u8> {
    let block = channels * (bits / 8);
    let data_len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    let mut out = Vec::new();
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&format_tag.to_le_bytes());
    out.extend_from_slice(&channels.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * u32::from(block)).to_le_bytes());
    out.extend_from_slice(&block.to_le_bytes());
    out.extend_from_slice(&bits.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    out.extend_from_slice(data);
    out
}

fn wav16(channels: u16, rate: u32, samples: &[i16]) -> Vec<u8> {
    let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
    wav(1, channels, rate, 16, &data)
}

fn whole_wav(bytes: &[u8]) -> Result<Pcm, Error> {
    let limits = Limits::default();
    read_all(&mut WavSource::new(bytes, &limits)?, &limits)
}

fn whole_ogg(bytes: &[u8]) -> Result<Pcm, Error> {
    let limits = Limits::default();
    read_all(&mut VorbisSource::new(bytes, &limits)?, &limits)
}

fn pcm_or_panic(pcm: Result<Pcm, Error>) -> Pcm {
    pcm.unwrap_or_else(|e| panic!("{e}"))
}

/// Everything `source` yields, read `room` samples at a time.
fn in_chunks(source: &mut impl Source, room: usize) -> Vec<i16> {
    let channels = usize::from(source.format().channels.count());
    let mut out = vec![0; room];
    let mut all = Vec::new();
    loop {
        let frames = source.read(&mut out).unwrap_or_else(|e| panic!("{e}"));
        assert!(frames * channels <= room, "a read past its room");
        if frames == 0 {
            return all;
        }
        all.extend_from_slice(&out[..frames * channels]);
    }
}

// -- WAV --

#[test]
fn a_16_bit_wav_gives_its_own_numbers() {
    let samples = [1, -2, 300, -400, i16::MAX, i16::MIN];
    let pcm = pcm_or_panic(whole_wav(&wav16(2, 22_050, &samples)));
    assert_eq!(pcm.samples(), samples);
    assert_eq!(pcm.format().rate, 22_050);
    assert_eq!(pcm.format().channels, Channels::Stereo);
    assert_eq!(pcm.frames(), 3);
}

#[test]
fn a_wav_says_how_long_it_is_before_it_is_read() {
    let bytes = wav16(1, 8_000, &[1, 2, 3]);
    let source = WavSource::new(&bytes, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(source.total_frames(), Some(3));
}

#[test]
fn other_widths_become_16_bits() {
    let eight = whole_wav(&wav(1, 1, 8_000, 8, &[0xFF, 0x80, 0x00]));
    assert_eq!(pcm_or_panic(eight).samples(), [0x7F00, 0, i16::MIN]);

    let twenty_four = whole_wav(&wav(
        1,
        1,
        8_000,
        24,
        &[0xFF, 0xFF, 0x7F, 0, 0, 0x80, 0, 1, 0],
    ));
    assert_eq!(pcm_or_panic(twenty_four).samples(), [i16::MAX, i16::MIN, 1]);

    let thirty_two = whole_wav(&wav(1, 1, 8_000, 32, &[0, 0, 1, 0, 0, 0, 0, 0x80]));
    assert_eq!(pcm_or_panic(thirty_two).samples(), [1, i16::MIN]);

    let floats: Vec<u8> = [1.0f32, -1.0, 0.5, 8.0]
        .iter()
        .flat_map(|s| s.to_le_bytes())
        .collect();
    let float = whole_wav(&wav(3, 1, 8_000, 32, &floats));
    assert_eq!(
        pcm_or_panic(float).samples(),
        [32_767, -32_767, 16_384, 32_767]
    );
}

#[test]
fn a_wav_of_another_kind_is_named() {
    let adpcm = whole_wav(&wav(0x11, 1, 8_000, 16, &[0; 16]));
    assert!(matches!(adpcm, Err(Error::Malformed { .. })), "{adpcm:?}");
    let float = whole_wav(&wav(3, 1, 8_000, 16, &[0; 16]));
    assert!(float.is_err(), "{float:?}");
}

#[test]
fn a_wav_outside_the_limits_stops_at_its_header() {
    let limits = Limits::new(8_000, 2);
    let open = |bytes: &[u8]| WavSource::new(bytes, &limits).err();
    assert_eq!(
        open(&wav16(1, 8_001, &[0])),
        Some(Error::Rate {
            rate: 8_001,
            max: 8_000
        })
    );
    assert_eq!(
        open(&wav16(1, 8_000, &[0; 3])),
        Some(Error::TooLong {
            frames: 3,
            limit: 2
        })
    );
    assert_eq!(open(&wav16(3, 8_000, &[0; 3])), Some(Error::Channels(3)));
    assert_eq!(open(&wav16(1, 8_000, &[0; 2])), None);
}

#[test]
fn a_wav_that_says_it_holds_more_than_it_has_stops_at_its_header() {
    let mut bytes = wav16(2, 8_000, &[0; 8]);
    bytes.truncate(bytes.len() - 4);
    let error = WavSource::new(&bytes, &Limits::default()).err();
    assert!(
        matches!(&error, Some(Error::Malformed { format: "WAV", reason }) if reason.contains("says")),
        "{error:?}"
    );
}

#[test]
fn a_wav_read_with_too_little_room_says_so() {
    let bytes = wav16(2, 8_000, &[1, 2]);
    let mut source = WavSource::new(&bytes, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        source.read(&mut [0; 1]),
        Err(Error::NoRoom {
            samples: 1,
            channels: 2
        })
    );
    // And nothing was lost.
    let mut out = [0; 2];
    assert_eq!(source.read(&mut out), Ok(1));
    assert_eq!(out, [1, 2]);
    assert_eq!(source.read(&mut out), Ok(0));
}

// -- Ogg Vorbis --

#[test]
fn an_ogg_file_gives_its_format_and_length() {
    let pcm = pcm_or_panic(whole_ogg(MONO));
    assert_eq!(pcm.format().rate, 22_050);
    assert_eq!(pcm.format().channels, Channels::Mono);
    assert_eq!(pcm.frames(), 22_050, "the source of it is a second long");

    let pcm = pcm_or_panic(whole_ogg(STEREO));
    assert_eq!(pcm.format().rate, 44_100);
    assert_eq!(pcm.format().channels, Channels::Stereo);
    assert_eq!(pcm.frames(), 44_100 * 13 / 10, "a marker and a second");
}

#[test]
fn an_ogg_file_keeps_its_comments() {
    let limits = Limits::default();
    let source = VorbisSource::new(REGION, &limits).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(source.comment("LOOPSTART"), Some("44100"));
    assert_eq!(source.comment("looplength"), Some("44100"));
    assert_eq!(source.comment("nothing"), None);
    assert!(source.comments().iter().any(|(k, _)| k == "LOOPSTART"));
    assert_ne!(source.vendor(), "");
    assert_eq!(source.total_frames(), None);
}

#[test]
fn chunks_of_any_size_make_the_same_ogg_samples() {
    let whole = pcm_or_panic(whole_ogg(STEREO));
    for room in [2, 3, 100, 4096, 1 << 18] {
        let mut source =
            VorbisSource::new(STEREO, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(in_chunks(&mut source, room), whole.samples(), "{room}");
    }
}

#[test]
fn an_ogg_file_over_the_limit_stops_early() {
    let limits = Limits::new(192_000, 1000);
    let mut source = VorbisSource::new(STEREO, &limits).unwrap_or_else(|e| panic!("{e}"));
    let error = read_all(&mut source, &limits);
    assert!(
        matches!(error, Err(Error::TooLong { limit: 1000, frames }) if frames < 1000 + 4096 + 8192),
        "{error:?}"
    );
    let low = Limits::new(8_000, 1000);
    assert_eq!(
        VorbisSource::new(STEREO, &low).err(),
        Some(Error::Rate {
            rate: 44_100,
            max: 8_000
        })
    );
}

#[test]
fn an_ogg_read_with_too_little_room_says_so() {
    let mut source =
        VorbisSource::new(STEREO, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        source.read(&mut [0; 1]),
        Err(Error::NoRoom {
            samples: 1,
            channels: 2
        })
    );
}

#[test]
fn an_ogg_file_of_one_page_of_sound_ends_where_its_last_page_says() {
    let mut source = VorbisSource::new(MONO, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(source.end_granule(), Some(22_050));
    assert_eq!(source.position(), Some(0));
    let all = in_chunks(&mut source, 4096);
    assert_eq!(all.len(), 22_050);
    assert_eq!(source.position(), Some(22_050));
}

#[test]
fn the_position_counts_frames_as_they_go_out() {
    let mut source =
        VorbisSource::new(STEREO, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    let mut out = [0; 2000];
    let mut read = 0;
    loop {
        let frames = source.read(&mut out).unwrap_or_else(|e| panic!("{e}"));
        read += frames as u64;
        assert_eq!(source.position(), Some(read));
        if frames == 0 {
            break;
        }
    }
    assert_eq!(read, 57_330);
}

/// What an exact boundary builds on: after a page seek the position is unknown
/// until a page ends, and from then on the samples are the whole decode's.
#[test]
fn after_a_page_seek_the_position_says_which_samples_these_are() {
    let whole = pcm_or_panic(whole_ogg(LONG));
    let mut source = VorbisSource::new(LONG, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    source.seek_page(200_000).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(source.position(), None);
    let mut frame = [0; 2];
    let position = loop {
        assert_eq!(
            source.read(&mut frame),
            Ok(1),
            "the file ended, position unknown"
        );
        if let Some(position) = source.position() {
            break position;
        }
    };
    assert!(position >= 200_000 - 16_384, "{position}");
    assert!(position < 241_000, "{position}");
    let rest = in_chunks(&mut source, 4096);
    let start = usize::try_from(position).unwrap_or(usize::MAX) * 2;
    assert_eq!(rest.len(), whole.samples().len() - start);
    let worst = rest
        .iter()
        .zip(&whole.samples()[start..])
        .map(|(a, b)| (i32::from(*a) - i32::from(*b)).abs())
        .max();
    assert_eq!(worst, Some(0), "the same decoder, from the same place");
    assert_eq!(source.position(), Some(441_000));
}

#[test]
fn a_seek_among_the_headers_reads_on_into_the_sound() {
    let mut source = VorbisSource::new(MONO, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    source.seek_page(0).unwrap_or_else(|e| panic!("{e}"));
    assert_ne!(in_chunks(&mut source, 4096), Vec::<i16>::new());
    let mut far = VorbisSource::new(STEREO, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
    far.seek_page(u64::MAX).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(in_chunks(&mut far, 4096), Vec::<i16>::new());
}

// -- Malformed input --

/// `bytes` with up to a few of them changed, and maybe cut short.
fn damaged(bytes: &'static [u8]) -> impl Strategy<Value = Vec<u8>> {
    (
        proptest::collection::vec((0..bytes.len(), any::<u8>()), 1..24),
        0..=bytes.len(),
        any::<bool>(),
    )
        .prop_map(move |(changes, cut, cutting)| {
            let mut out = bytes.to_vec();
            for (at, byte) in changes {
                out[at] = byte;
            }
            if cutting {
                out.truncate(cut);
            }
            out
        })
}

/// Decodes it all, however it goes; a panic escaping is what fails.
fn survives_ogg(bytes: &[u8]) {
    let limits = Limits::new(192_000, 1 << 20);
    if let Ok(mut source) = VorbisSource::new(bytes, &limits) {
        let _ = read_all(&mut source, &limits);
    }
}

fn survives_wav(bytes: &[u8]) {
    let limits = Limits::new(192_000, 1 << 20);
    if let Ok(mut source) = WavSource::new(bytes, &limits) {
        let _ = read_all(&mut source, &limits);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn random_bytes_are_an_error(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        prop_assert!(whole_ogg(&bytes).is_err());
        prop_assert!(whole_wav(&bytes).is_err());
    }

    #[test]
    fn a_damaged_ogg_file_never_panics(bytes in damaged(MONO)) {
        survives_ogg(&bytes);
    }

    #[test]
    fn a_damaged_wav_never_panics(bytes in proptest::collection::vec(any::<u8>(), 44..256), cut in 0usize..256) {
        let good = wav16(2, 8_000, &[1, 2, 3, 4, 5, 6, 7, 8]);
        // A real header, then its parts changed.
        let mut damaged = bytes;
        damaged[..44].copy_from_slice(&good[..44]);
        for at in [22, 23, 24, 25, 32, 33, 34, 35, 40, 41, 42, 43] {
            damaged[at] = damaged[(at * 7) % damaged.len()];
        }
        survives_wav(&damaged);
        damaged.truncate(cut);
        survives_wav(&damaged);
    }

    /// A cut anywhere gives an error or a prefix of what the whole file gives,
    /// to within a sound's last overlap.
    #[test]
    fn a_cut_wav_is_an_error(cut in 0usize..=59) {
        let good = wav16(1, 8_000, &[9; 8]);
        prop_assert!(whole_wav(&good[..cut]).is_err());
    }

    #[test]
    fn chunks_rebuild_a_wav(samples in proptest::collection::vec(any::<i16>(), 0..256), room in 1usize..64) {
        let bytes = wav16(1, 8_000, &samples);
        let mut source = WavSource::new(&bytes, &Limits::default()).unwrap_or_else(|e| panic!("{e}"));
        prop_assert_eq!(in_chunks(&mut source, room), samples);
    }
}
