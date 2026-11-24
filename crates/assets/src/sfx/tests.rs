//! The sound effects: their checks and lines, the render's waves, envelope
//! and pitch, the WAV, the presets, the hook and the formats; property
//! tests that nothing panics and every render stays in its bounds; and the
//! presets' golden WAV hashes.

use alloc::borrow::Cow;

use proptest::prelude::*;
use shuyi_core::state_hash;

use super::*;
use crate::audio::{decode_to_wav, format_problem, playable};

/// A square at 441 hz (100 samples a cycle), 100 ms, held at full volume.
fn plain(wave: Wave) -> SfxDef {
    SfxDef::new(wave, 441, 100, 0, 100)
}

/// The problems of `source` read as `a.sfx.ron`, each as `line: message`.
fn errors(source: &str) -> Vec<String> {
    from_source("a.sfx.ron", source)
        .unwrap_err()
        .iter()
        .map(|e| format!("{}: {}", e.line.unwrap_or_default(), e.message))
        .collect()
}

/// A file of `fields`, one a line after the wave's.
fn file(fields: &str) -> String {
    format!("(\n    wave: Square(duty: 50),\n{fields})\n")
}

const GOOD: &str = "    hz: 440,\n    hold_ms: 100,\n    fade_ms: 50,\n    volume: 50,\n";

#[test]
fn a_file_reads_with_its_defaults() {
    let def = from_source("a.sfx.ron", &file(GOOD)).unwrap();
    let mut want = SfxDef::new(Wave::Square { duty: 50 }, 440, 100, 50, 50);
    assert_eq!(def, want);
    assert_eq!((def.to_hz, def.attack_ms, def.seed), (440, 0, 0));
    let every = file(
        "    hz: 440,\n    to_hz: 220,\n    attack_ms: 5,\n    hold_ms: 100,\n    fade_ms: 50,\n    \
         volume: 50,\n    vibrato: (hz: 6, percent: 3),\n    step: (at_ms: 20, percent: 150),\n    \
         seed: 9,\n",
    );
    want.to_hz = 220;
    want.attack_ms = 5;
    want.vibrato = Some(Vibrato { hz: 6, percent: 3 });
    want.step = Some(Step {
        at_ms: 20,
        percent: 150,
    });
    want.seed = 9;
    assert_eq!(from_source("a.sfx.ron", &every), Ok(want));
}

#[test]
fn an_unknown_field_is_an_error_on_its_line() {
    let got = from_source("a.sfx.ron", &file("    hz: 440,\n    pitch: 3,\n")).unwrap_err();
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!((got[0].file.as_str(), got[0].line), ("a.sfx.ron", Some(4)));
    assert!(got[0].message.contains("pitch"), "{got:?}");
}

#[test]
fn each_number_out_of_range_is_an_error_on_its_fields_line() {
    let cases = [
        (
            "(\n    wave: Square(duty: 0),\n    hz: 440,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 1,\n)",
            "2: duty 0 is outside 1 to 99 (percent)",
        ),
        (
            "(\n    wave: Square(duty: 100),\n    hz: 440,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 1,\n)",
            "2: duty 100 is outside 1 to 99 (percent)",
        ),
        (
            &file("    hz: 30000,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 1,\n"),
            "3: hz 30000 is over 12000",
        ),
        (
            &file(
                "    hz: 19,\n    to_hz: 440,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 1,\n",
            ),
            "3: hz 19 is under 20",
        ),
        (
            &file(
                "    hz: 440,\n    to_hz: 12001,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 1,\n",
            ),
            "4: to_hz 12001 is over 12000",
        ),
        (
            &file("    hz: 440,\n    hold_ms: 0,\n    fade_ms: 0,\n    volume: 1,\n"),
            "4: the sound is 0 ms long (attack_ms, hold_ms and fade_ms together); it must be 1 to 5000",
        ),
        (
            &file(
                "    hz: 440,\n    attack_ms: 1,\n    hold_ms: 4000,\n    fade_ms: 1000,\n    volume: 1,\n",
            ),
            "5: the sound is 5001 ms long (attack_ms, hold_ms and fade_ms together); it must be 1 to 5000",
        ),
        (
            &file("    hz: 440,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 101,\n"),
            "6: volume 101 is over 100 (percent)",
        ),
        (
            &file(
                "    hz: 440,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 1,\n    vibrato: (hz: 0, percent: 51),\n",
            ),
            "7: the vibrato's hz 0 is outside 1 to 50",
        ),
        (
            &file(
                "    hz: 440,\n    hold_ms: 10,\n    fade_ms: 0,\n    volume: 1,\n    step: (at_ms: 10, percent: 150),\n",
            ),
            "7: the step's at_ms 10 is not inside the sound's 10 ms",
        ),
        (
            &file(
                "    hz: 440,\n    hold_ms: 10,\n    fade_ms: 0,\n    volume: 1,\n    step: (at_ms: 1, percent: 9),\n",
            ),
            "7: the step's percent 9 is outside 10 to 400",
        ),
        (
            &file(
                "    hz: 440,\n    to_hz: 4001,\n    hold_ms: 10,\n    fade_ms: 0,\n    volume: 1,\n    step: (at_ms: 1, percent: 300),\n",
            ),
            "8: the step takes the pitch to 12003 hz, over 12000",
        ),
    ];
    for (source, want) in cases {
        let got = errors(source);
        assert_eq!(got.first().map(String::as_str), Some(want), "{source}");
    }
    // Each problem is reported, not only the first.
    let both = errors(&file(
        "    hz: 440,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 1,\n    vibrato: (hz: 0, percent: 51),\n",
    ));
    assert_eq!(
        both,
        [
            "7: the vibrato's hz 0 is outside 1 to 50",
            "7: the vibrato's percent 51 is outside 1 to 50"
        ]
    );
    // At the top of every range, nothing.
    let top = file(
        "    hz: 12000,\n    hold_ms: 5000,\n    fade_ms: 0,\n    volume: 100,\n    vibrato: (hz: 50, \
         percent: 50),\n    step: (at_ms: 4999, percent: 100),\n",
    );
    assert!(from_source("a.sfx.ron", &top).is_ok());
}

#[test]
fn each_range_holds_its_ends() {
    let mut def = SfxDef::new(Wave::Square { duty: 1 }, MIN_HZ, 1, 0, 0);
    def.to_hz = MAX_HZ;
    assert_eq!(problems(&def), []);
    def.wave = Wave::Square { duty: 99 };
    def.hz = MIN_HZ - 1;
    assert_eq!(problems(&def), [("hz", "hz 19 is under 20".to_owned())]);
}

#[test]
fn the_waves_have_their_values_at_each_point_of_a_cycle() {
    let quarter = 1_u32 << 30;
    // A triangle: low at the start and the end, high at half, 0 between.
    assert_eq!(triangle(0), -32_767);
    assert_eq!(triangle(quarter), 0);
    assert_eq!(triangle(2 * quarter), 32_767);
    assert_eq!(triangle(3 * quarter), 0);
    assert_eq!(triangle(u32::MAX), -32_766);
    assert_eq!(triangle(quarter / 2), -16_383);
    assert_eq!(triangle(5 * (quarter / 2)), 16_384);
    // A square of duty 25 is high exactly for its first quarter.
    assert_eq!(square(quarter - 1, 25), 32_767);
    assert_eq!(square(quarter, 25), -32_767);
    assert_eq!(square(0, 1), 32_767);
    assert_eq!(square(u32::MAX, 99), -32_767);
    // The saw rises from low to high.
    assert_eq!(
        (ramp(0), ramp(1 << 31), ramp(u32::MAX)),
        (-32_767, 0, 32_766)
    );
    // The vibrato's triangle: 0, up to the sample rate, 0, down, 0, at
    // 1 hz over a second.
    let at = |i: i64| wobble(i, 1);
    assert_eq!(
        [at(0), at(11_025), at(22_050), at(33_075), at(44_100)],
        [0, 44_100, 0, -44_100, 0]
    );
    assert_eq!((at(5_512), at(27_562)), (22_048, -22_048));
    assert_eq!(wobble(11_025, 2), 0);
}

#[test]
fn a_fields_line_is_the_field_not_a_longer_name() {
    let source = "(\n  to_hz: 1,\n  hz: 2,\n)";
    assert_eq!(field_line(source, "hz"), Some(3));
    assert_eq!(field_line(source, "to_hz"), Some(2));
    assert_eq!(field_line(source, "seed"), None);
}

#[test]
fn the_text_reads_back_to_the_same_definition_leaving_defaults_out() {
    let def = preset("coin", 0).unwrap();
    let text = to_text(&def);
    assert_eq!(from_source("a.sfx.ron", &text), Ok(def.clone()));
    assert!(
        !text.contains("seed") && !text.contains("attack_ms"),
        "{text}"
    );
    assert!(text.contains("step: (at_ms: 60, percent: 133)"), "{text}");
    assert!(!text.contains("enable"), "{text}");
    assert_eq!(SfxDefRon.encode(&def), text.as_bytes());
    let mut every = def;
    every.vibrato = Some(Vibrato { hz: 3, percent: 4 });
    every.seed = 7;
    every.attack_ms = 2;
    let text = to_text(&every);
    assert_eq!(from_source("a.sfx.ron", &text), Ok(every));
}

#[test]
fn a_render_has_its_lengths_samples_and_the_wav_its_header() {
    let def = SfxDef::new(Wave::Sine, 441, 7, 3, 100);
    let samples = render(&def);
    // 10 ms at 44 100 samples a second.
    assert_eq!(samples.len(), 441);
    let bytes = to_wav(&samples);
    assert_eq!(bytes.len(), 44 + 441 * 2);
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[4..8], &(36_u32 + 882).to_le_bytes());
    assert_eq!(&bytes[8..16], b"WAVEfmt ");
    // Integer PCM, one channel, at the sample rate.
    assert_eq!(&bytes[20..22], &1_u16.to_le_bytes());
    assert_eq!(&bytes[22..24], &1_u16.to_le_bytes());
    assert_eq!(&bytes[24..28], &44_100_u32.to_le_bytes());
    // Bytes a second, a frame, bits a sample.
    assert_eq!(&bytes[28..32], &88_200_u32.to_le_bytes());
    assert_eq!(&bytes[32..34], &2_u16.to_le_bytes());
    assert_eq!(&bytes[34..36], &16_u16.to_le_bytes());
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(&bytes[40..44], &882_u32.to_le_bytes());
    assert_eq!(i16::from_le_bytes([bytes[46], bytes[47]]), samples[1]);
    assert_eq!(format_problem("a.wav", &bytes), None);
}

#[test]
fn each_wave_starts_as_its_shape() {
    // 100 samples a cycle.
    let first = |wave| render(&plain(wave))[..100].to_vec();
    let square = first(Wave::Square { duty: 25 });
    assert!(square[..25].iter().all(|&s| s == 32_767), "{square:?}");
    assert!(square[26..].iter().all(|&s| s == -32_767), "{square:?}");
    let saw = first(Wave::Saw);
    assert_eq!(saw[0], 0);
    assert!(saw.windows(2).take(49).all(|w| w[1] > w[0]), "{saw:?}");
    assert!(saw[49] > 32_000 && saw[51] < -32_000, "{saw:?}");
    let triangle = first(Wave::Triangle);
    assert_eq!(triangle[0], 0);
    assert!(
        (triangle[25] - 32_767).abs() < 700 && (triangle[75] + 32_767).abs() < 700,
        "{triangle:?}"
    );
    let sine = first(Wave::Sine);
    assert_eq!(sine[0], 0);
    assert!(
        sine[25] > 32_700 && sine[50].abs() < 100 && sine[75] < -32_700,
        "{sine:?}"
    );
    // Noise holds a level for each half cycle, and two seeds differ.
    let noise = first(Wave::Noise);
    assert!(noise[..50].iter().all(|&s| s == noise[0]), "{noise:?}");
    assert_ne!(noise[60], noise[0]);
    let mut other = plain(Wave::Noise);
    other.seed = 1;
    assert_ne!(render(&other)[..100], noise[..]);
}

#[test]
fn the_sine_is_within_a_fifth_of_a_percent_of_a_true_sine() {
    let steps = 4096_u32;
    for i in 0..steps {
        let phase = (u64::from(i) << 32) / u64::from(steps);
        let phase = u32::try_from(phase).unwrap();
        let angle = f64::from(i) / f64::from(steps) * core::f64::consts::TAU;
        let want = 32_767.0 * angle.sin();
        let got = f64::from(i32::try_from(sine(phase)).unwrap());
        assert!((got - want).abs() <= 66.0, "{i}: {got} against {want}");
    }
}

#[test]
fn the_loudness_rises_holds_and_falls_to_nothing() {
    let mut def = SfxDef::new(Wave::Square { duty: 50 }, 441, 10, 10, 100);
    def.attack_ms = 10;
    let samples = render(&def);
    assert_eq!(samples.len(), 1323);
    // Silent at the start, rising through the attack.
    assert_eq!(samples[0], 0);
    assert!(samples[220].abs() < samples[440].abs(), "{samples:?}");
    // Full in the hold.
    assert!(samples[441..882].iter().all(|s| s.abs() == 32_767));
    // Falling through the fade, silent at the end.
    assert!(samples[1100].abs() < samples[900].abs());
    assert_eq!(samples[1322], 0);
    // The volume scales it.
    def.volume = 50;
    let half = render(&def);
    assert_eq!(half[500].abs(), 32_767 / 2);
}

/// How many times `samples` cross zero going up.
fn rises(samples: &[i16]) -> usize {
    samples.windows(2).filter(|w| w[0] < 0 && w[1] >= 0).count()
}

#[test]
fn a_slide_and_a_step_change_how_often_the_wave_crosses_zero() {
    // 441 hz for 100 ms: 44 cycles.
    let steady = render(&plain(Wave::Sine));
    assert_eq!(rises(&steady), 44);
    let mut slide = plain(Wave::Sine);
    slide.to_hz = 1323;
    // From 441 to 1 323 hz, on average 882: about twice as many.
    let slid = rises(&render(&slide));
    assert!((86..=90).contains(&slid), "{slid}");
    let mut step = plain(Wave::Sine);
    step.step = Some(Step {
        at_ms: 50,
        percent: 200,
    });
    // Half the time at 441 hz, half at 882.
    let stepped = rises(&render(&step));
    assert!((65..=67).contains(&stepped), "{stepped}");
    // A vibrato keeps the pitch on average.
    let mut wobble = plain(Wave::Sine);
    wobble.vibrato = Some(Vibrato {
        hz: 10,
        percent: 20,
    });
    let wobbled = render(&wobble);
    assert!((43..=45).contains(&rises(&wobbled)), "{}", rises(&wobbled));
    assert_ne!(wobbled, steady);
}

#[test]
fn a_definition_out_of_range_renders_bounded() {
    let mut wild = SfxDef::new(Wave::Square { duty: 0 }, u32::MAX, u32::MAX, u32::MAX, 255);
    wild.attack_ms = u32::MAX;
    wild.to_hz = 0;
    wild.vibrato = Some(Vibrato {
        hz: 255,
        percent: 255,
    });
    wild.step = Some(Step {
        at_ms: u32::MAX,
        percent: u16::MAX,
    });
    let found = problems(&wild);
    assert!(!found.is_empty(), "{found:?}");
    assert_eq!(render(&wild).len(), 220_500);
    // The lengths are cut to the longest in their order: the fade last.
    let mut long = SfxDef::new(Wave::Sine, 440, 3000, 5000, 100);
    long.attack_ms = 1000;
    assert_eq!(render(&long).len(), 220_500);
    let silent = SfxDef::new(Wave::Sine, 440, 0, 0, 100);
    let none = render(&silent);
    assert!(none.is_empty(), "{none:?}");
}

#[test]
fn every_preset_reads_and_its_seed_zero_is_its_file() {
    for name in PRESETS {
        let source = preset_source(name).unwrap();
        let def = from_source(name, source).unwrap_or_else(|e| panic!("{name}: {e:?}"));
        assert_eq!(preset(name, 0), Some(def), "{name}");
        assert!(source.starts_with(&format!("// {name}: ")), "{name}");
    }
    assert_eq!(preset("nope", 0), None);
    assert_eq!(preset_source("nope"), None);
}

#[test]
fn another_seed_moves_a_preset_within_its_ranges_the_same_each_time() {
    for name in PRESETS {
        let file = preset(name, 0).unwrap();
        for seed in 1..200 {
            let def = preset(name, seed).unwrap();
            assert_eq!(problems(&def), [], "{name} {seed}");
            assert_eq!(preset(name, seed), Some(def.clone()));
            assert_eq!(def.seed, seed);
            let near = |was: u32, is: u32, percent: u32| {
                u64::from(is.abs_diff(was)) * 100 <= u64::from(was) * u64::from(percent) + 100
            };
            assert!(near(file.hz, def.hz, 10), "{name} {seed}: {def:?}");
            assert!(near(file.to_hz, def.to_hz, 10), "{name} {seed}: {def:?}");
            assert!(
                near(file.hold_ms, def.hold_ms, 20),
                "{name} {seed}: {def:?}"
            );
        }
        assert_ne!(preset(name, 1), preset(name, 2), "{name}");
    }
    // A step near the end stays inside a hold that shrinks.
    let mut late = SfxDef::new(Wave::Sine, 440, 100, 0, 50);
    late.step = Some(Step {
        at_ms: 99,
        percent: 150,
    });
    let mut moved = 0;
    for seed in 1..50 {
        let mut def = late.clone();
        vary(&mut def, seed);
        assert_eq!(problems(&def), [], "{seed}: {def:?}");
        moved += usize::from(def.step.unwrap().at_ms < 99);
    }
    assert!(moved > 0);
    // Near the top, a step's preset keeps its pitch under the highest.
    let mut high = preset("coin", 0).unwrap();
    high.hz = 9000;
    high.to_hz = 9000;
    for seed in 1..50 {
        let mut def = high.clone();
        vary(&mut def, seed);
        assert_eq!(problems(&def), [], "{seed}: {def:?}");
    }
}

#[test]
fn the_hook_renders_a_sound_effect_and_passes_the_rest_unchanged() {
    let coin = preset_source("coin").unwrap().as_bytes();
    let wav = playable("sfx/coin.sfx.ron", Some(Cow::Borrowed(coin))).unwrap();
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(&*wav, to_wav(&render(&preset("coin", 0).unwrap())));
    assert_eq!(format_problem("sfx/coin.sfx.ron", &wav), None);
    // Rendered twice, the same bytes.
    assert_eq!(
        playable("coin.sfx.ron", Some(Cow::Borrowed(coin))).unwrap(),
        wav
    );
    // Another name, a broken file, a missing one: unchanged.
    for (name, bytes) in [
        ("sfx/coin.ron", Some(Cow::Borrowed(coin))),
        ("sfx/coin.wav", Some(Cow::Borrowed(coin))),
        (".sfx.ron", Some(Cow::Borrowed(coin))),
        ("sfx/bad.sfx.ron", Some(Cow::Borrowed(&b"(hz: 1)"[..]))),
        ("sfx/coin.sfx.ron", None),
    ] {
        assert_eq!(playable(name, bytes.clone()), bytes, "{name}");
    }
    // A WAV already rendered is left as it is.
    assert_eq!(decode_to_wav("a.sfx.ron", &wav), None);
}

#[test]
fn the_manifests_check_reads_a_sound_effect_as_text_or_as_its_wav() {
    let coin = preset_source("coin").unwrap().as_bytes();
    assert_eq!(format_problem("sfx/coin.sfx.ron", coin), None);
    let broken = file("    hz: 30000,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 1,\n");
    assert_eq!(
        format_problem("sfx/a.sfx.ron", broken.as_bytes()).as_deref(),
        Some("line 3: hz 30000 is over 12000")
    );
    assert_eq!(
        format_problem("sfx/a.sfx.ron", &[0xff, 0xfe]).as_deref(),
        Some("not valid UTF-8")
    );
    let unknown = format_problem("sfx/a.sfx.ron", b"(wave: Sine, pitch: 2)").unwrap();
    assert!(
        unknown.starts_with("line 1: ") && unknown.contains("pitch"),
        "{unknown}"
    );
    // Its WAV, rendered by a hook, is checked as a WAV.
    let stereo_at_8k = crate::audio::pcm16_wav(2, 8000, &[0; 4]);
    assert_eq!(
        format_problem("sfx/a.sfx.ron", &stereo_at_8k).as_deref(),
        Some("8000 Hz; must be 44100 Hz")
    );
}

#[test]
fn a_manifest_names_its_sound_effects_and_checks_them() {
    let files = |name: &str| -> crate::audio::Bytes {
        match name {
            "sfx/coin.sfx.ron" => Some(Cow::Borrowed(preset_source("coin")?.as_bytes())),
            "sfx/bad.sfx.ron" => Some(Cow::Borrowed(&b"(\n    wave: Sine,\n    hz: 5,\n    hold_ms: 1,\n    fade_ms: 0,\n    volume: 1,\n)"[..])),
            _ => None,
        }
    };
    let shown = |name: &str| format!("files/{name}");
    let left_out = |_: &str| false;
    let read = crate::audio::Files::new(&files, &shown, &left_out, "music");
    let manifest = "(music_fade_ms: 0, music: [], pools: [], credits: [], sounds: [(id: \"coin\", files: [\"sfx/coin.sfx.ron\"], volume: 80, credit: Own)])";
    let read_ok = crate::audio::from_source("audio.ron", manifest, &read);
    assert!(read_ok.is_ok(), "{read_ok:?}");
    let manifest = "(music_fade_ms: 0, music: [], pools: [], credits: [], sounds: [\n(id: \"bad\", files: [\"sfx/bad.sfx.ron\"], volume: 80, credit: Own),\n(id: \"odd\", files: [\"sfx/odd.ron\"], volume: 80, credit: Own),\n])";
    let errors: Vec<String> = crate::audio::from_source("audio.ron", manifest, &read)
        .unwrap_err()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        errors,
        [
            "audio.ron:2: sound \"bad\": files/sfx/bad.sfx.ron: line 3: hz 5 is under 20",
            "audio.ron:3: sound \"odd\": \"sfx/odd.ron\" must be .ogg, .wav, .flac, .mp3 or .sfx.ron",
        ]
    );
}

#[test]
fn the_two_formats_read_a_file_and_the_sound_one_only_a_sound_effects_name() {
    let coin = preset_source("coin").unwrap().as_bytes();
    let cx = DecodeCx::new();
    assert_eq!(
        SfxDefRon.decode(&AssetPath::new("a.sfx.ron"), coin, &cx),
        Ok(preset("coin", 0).unwrap())
    );
    assert_eq!(
        SfxRon.decode(&AssetPath::new("sfx/a.sfx.ron"), coin, &cx),
        Ok(Sound::mono(SAMPLE_RATE))
    );
    let stray = SfxRon
        .decode(&AssetPath::new("sfx/palette.ron"), coin, &cx)
        .unwrap_err();
    assert_eq!(
        stray[0].message,
        "a sound in RON is a sound effect: its name must end in .sfx.ron"
    );
    let broken = SfxRon
        .decode(&AssetPath::new("a.sfx.ron"), b"(wave: Sine)", &cx)
        .unwrap_err();
    assert_eq!(broken[0].file, "a.sfx.ron");
    // Through the registry, by its extension.
    // Both read RON text, up to the text formats' limit.
    assert_eq!(
        (SfxDefRon.extensions(), SfxRon.extensions()),
        (&["ron"][..], &["ron"][..])
    );
    assert_eq!(
        (SfxDefRon.max_bytes(), SfxRon.max_bytes()),
        (TEXT_MAX_BYTES, TEXT_MAX_BYTES)
    );
    let formats = crate::format::Formats::standard();
    assert_eq!(formats.extensions::<SfxDef>(), ["ron"]);
    let path = AssetPath::new("sfx/a.sfx.ron");
    assert_eq!(
        formats.decode::<Sound>(&path, coin, &cx),
        Ok(Sound::mono(SAMPLE_RATE))
    );
    let wav = to_wav(&render(&preset("coin", 0).unwrap()));
    assert_eq!(
        formats.decode::<Sound>(&path, &wav, &cx),
        Ok(Sound::mono(SAMPLE_RATE))
    );
    assert!(is_sfx_name("a.sfx.ron") && !is_sfx_name(".sfx.ron") && !is_sfx_name("a.ron"));
}

#[test]
fn golden_every_presets_wav_is_the_same_on_every_platform() {
    let mut golden = crate::golden!("tests/golden/sfx.ron");
    for name in PRESETS {
        let def = preset(name, 0).unwrap();
        let wav = to_wav(&render(&def));
        assert_eq!(to_wav(&render(&def)), wav, "{name}");
        golden.check(name, state_hash(&wav).unwrap());
    }
    // A seed's variant too.
    let wav = to_wav(&render(&preset("hit", 7).unwrap()));
    golden.check("hit seed 7", state_hash(&wav).unwrap());
    golden.assert();
}

/// Any in-range sound effect.
fn in_range() -> impl Strategy<Value = SfxDef> {
    let wave = prop_oneof![
        (1..=99_u8).prop_map(|duty| Wave::Square { duty }),
        Just(Wave::Triangle),
        Just(Wave::Saw),
        Just(Wave::Sine),
        Just(Wave::Noise),
    ];
    let lengths = (0..=MAX_MS, 0..=MAX_MS, 0..=MAX_MS).prop_filter("1 to 5000 ms", |(a, h, f)| {
        (1..=MAX_MS).contains(&(a + h + f))
    });
    (
        wave,
        (MIN_HZ..=MAX_HZ, MIN_HZ..=MAX_HZ),
        lengths,
        0..=100_u8,
        proptest::option::of((1..=50_u8, 1..=50_u8)),
        proptest::option::of((0..=u32::MAX, 10..=400_u16)),
        any::<u64>(),
    )
        .prop_map(
            |(wave, (hz, to_hz), (attack_ms, hold_ms, fade_ms), volume, vibrato, step, seed)| {
                let mut def = SfxDef::new(wave, hz, hold_ms, fade_ms, volume);
                def.to_hz = to_hz;
                def.attack_ms = attack_ms;
                def.vibrato = vibrato.map(|(hz, percent)| Vibrato { hz, percent });
                def.step = step.map(|(at, percent)| {
                    let length = u32::try_from(def.length_ms()).unwrap();
                    let cap = u32::try_from(u64::from(MAX_HZ) * 100 / u64::from(percent)).unwrap();
                    def.hz = def.hz.min(cap);
                    def.to_hz = def.to_hz.min(cap);
                    Step {
                        at_ms: at % length,
                        percent,
                    }
                });
                def.seed = seed;
                def
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn an_in_range_sound_effect_renders_its_samples_within_its_volume(def in in_range()) {
        prop_assert_eq!(problems(&def), []);
        let samples = render(&def);
        let n = u64::from(SAMPLE_RATE) * def.length_ms() / 1000;
        prop_assert_eq!(u64::try_from(samples.len()).unwrap(), n);
        let loudest = 32_767 * i32::from(def.volume) / 100;
        prop_assert!(samples.iter().all(|&s| i32::from(s).abs() <= loudest));
    }

    #[test]
    fn a_sound_effect_written_reads_back_the_same(def in in_range()) {
        let text = to_text(&def);
        prop_assert_eq!(from_source("a.sfx.ron", &text), Ok(def));
    }

    #[test]
    fn no_bytes_panic_either_format(
        random in proptest::collection::vec(any::<u8>(), 0..256),
        preset in 0..PRESETS.len(),
        at in any::<usize>(),
        with in any::<char>(),
    ) {
        let source = preset_source(PRESETS[preset]).unwrap();
        let mut changed: Vec<char> = source.chars().collect();
        let at = at % changed.len();
        changed[at] = with;
        let changed: String = changed.into_iter().collect();
        let cx = DecodeCx::new();
        let path = AssetPath::new("a.sfx.ron");
        for bytes in [&random[..], changed.as_bytes()] {
            let _ = SfxDefRon.decode(&path, bytes, &cx);
            let _ = SfxRon.decode(&path, bytes, &cx);
            let _ = format_problem("a.sfx.ron", bytes);
            let _ = playable("a.sfx.ron", Some(Cow::Owned(bytes.to_vec())));
        }
    }
}
