//! Frames over time: `--every` and `--count`, [`then_frames`],
//! [`GameTools::frames`] and what `run` writes of them.

use shuyi_platform::harness::Harness;
use shuyi_ui::toy;

use super::super::*;
use super::{GAME, Snapshot, args, temp_dir};
use crate::codec::decode_png;
use crate::toy::ToyTools;

#[test]
fn then_frames_runs_exactly_n_more_frames() {
    let script = Script::new(vec![Step::Keys("Down".into()), Step::Wait(0.1)]);
    let frames = |script: &Script| {
        let mut h = Harness::new(toy::game_on(toy::ctx()));
        script.play(&mut h);
        h.frame_count()
    };
    let before = frames(&script);
    for n in [0, 1, 7] {
        let more = then_frames(&script, n);
        assert_eq!(frames(&more), before + n, "{n}");
        assert_eq!(more.steps[..2], script.steps[..]);
    }
    assert_eq!(then_frames(&script, 0), script);
}

#[test]
fn every_and_count_read_as_a_capture_and_an_output() {
    let capture = |extra: &[&str]| {
        let mut all = vec!["a.png", "--keys", "f"];
        all.extend_from_slice(extra);
        parse_args(&args(&all), &GAME).map(|options| options.capture)
    };
    let sheet = |columns| Output::Sheet { columns };
    assert_eq!(capture(&[]), Ok(None));
    // Seconds as whole frames, to the nearest; the sheet by default.
    assert_eq!(
        capture(&["--every", "0.05", "--count", "8"]),
        Ok(Some((Capture { every: 3, count: 8 }, sheet(None))))
    );
    assert_eq!(
        capture(&[
            "--count", "4", "--light", "dusk", "--every", "0.0167", "--sheet"
        ]),
        Ok(Some((Capture { every: 1, count: 4 }, sheet(None))))
    );
    assert_eq!(
        capture(&["--every", "1", "--count", "6", "--columns", "2"]),
        Ok(Some((
            Capture {
                every: 60,
                count: 6
            },
            sheet(Some(2))
        )))
    );
    assert_eq!(
        capture(&["--every", "0.1", "--count", "2", "--apng"]),
        Ok(Some((Capture { every: 6, count: 2 }, Output::Apng)))
    );
    assert_eq!(
        capture(&["--every", "0.05", "--count", "8", "--gif"]),
        Ok(Some((Capture { every: 3, count: 8 }, Output::Gif)))
    );
    // The game's options and the steps are read as before.
    let parsed = parse_args(
        &args(&[
            "a.png", "--every", "0.1", "--keys", "f", "--count", "2", "--map",
        ]),
        &GAME,
    )
    .unwrap();
    assert_eq!(parsed.script.steps, [Step::Keys("f".into())]);
    assert_eq!(parsed.game.len(), 1);
}

#[test]
fn each_capture_flag_alone_or_wrong_is_an_error() {
    let error = |extra: &[&str]| {
        let mut all = vec!["a.png"];
        all.extend_from_slice(extra);
        parse_args(&args(&all), &GAME).unwrap_err()
    };
    let cases: [(&[&str], &str); 17] = [
        (&["--every", "0.1"], "--every needs --count"),
        (&["--count", "3"], "--count needs --every"),
        (&["--apng"], "--apng needs --count"),
        (&["--gif"], "--gif needs --count"),
        (
            &["--every", "0.1", "--count", "3", "--gif", "--apng"],
            "one output only, not --gif and --apng",
        ),
        (
            &["--every", "0.1", "--count", "3", "--sheet", "--gif"],
            "one output only, not --sheet and --gif",
        ),
        (
            &["--every", "0.1", "--count", "3", "--gif", "--columns", "2"],
            "--columns is for the sheet only",
        ),
        (&["--sheet"], "--sheet needs --count"),
        (&["--columns", "2"], "--columns needs --count"),
        (
            &["--every", "0.1", "--count", "3", "--apng", "--sheet"],
            "one output only, not --apng and --sheet",
        ),
        (
            &["--every", "0.1", "--count", "3", "--apng", "--columns", "2"],
            "--columns is for the sheet only",
        ),
        (
            &["--every", "0.1", "--count", "3", "--columns", "4"],
            "--columns: 1 to 3 (--count), not 4",
        ),
        (
            &["--every", "0.005", "--count", "3"],
            "--every: 0.005 seconds is no whole frame; the shortest is 1/60 s (0.0167)",
        ),
        (
            &["--every", "soon", "--count", "3"],
            "--every: not a number of seconds: soon",
        ),
        (
            &["--every", "0.1", "--count", "many"],
            "--count: not a whole number: many",
        ),
        (
            &["--every", "0.1", "--count", "65"],
            "--count: 1 to 64, not 65",
        ),
        (
            &["--every", "1", "--count", "35"],
            "--every × (count − 1) is 2040 frames, more than 2000: \
             take fewer frames or closer together",
        ),
    ];
    for (extra, want) in cases {
        assert_eq!(error(extra), want, "{extra:?}");
    }
    assert!(error(&["--every", "40", "--count", "2"]).starts_with("--every: a number"));
    assert!(error(&["--every"]).contains("needs a value"));
}

#[test]
fn a_capture_is_checked_against_its_caps() {
    assert_eq!(Capture::new(3, 8), Ok(Capture { every: 3, count: 8 }));
    assert_eq!(
        Capture::new(2000, 2),
        Ok(Capture {
            every: 2000,
            count: 2
        })
    );
    assert_eq!(
        Capture::new(40, 51),
        Ok(Capture {
            every: 40,
            count: 51
        })
    );
    assert!(Capture::new(40, 52).is_err());
    assert!(Capture::new(0, 2).is_err());
    assert!(Capture::new(2001, 1).is_err());
    assert!(Capture::new(1, 0).is_err());
    assert!(Capture::new(1, MAX_CAPTURES + 1).is_err());
    let capture = Capture::new(3, 8).unwrap();
    assert_eq!(capture.frame_of(5), 15);
    // 64 frames is 1066.67 ms: rounded down.
    assert_eq!(
        (
            capture.millis_of(1),
            Capture::new(64, 2).unwrap().millis_of(1)
        ),
        (50, 1066)
    );
    assert_eq!(every_frames(0.05), Ok(3));
    assert_eq!(every_frames(1.0 / 60.0), Ok(1));
    assert_eq!(every_frames(0.009), Ok(1));
    assert!(every_frames(0.008).is_err());
    assert!(every_frames(f32::NAN).is_err());
    assert!(every_frames(-1.0).is_err());
}

#[test]
fn run_writes_a_sheet_and_lists_each_frame() {
    let dir = temp_dir("sheet");
    let options = parse_args(
        &args(&[
            "s.png", "--keys", "f", "--light", "dusk", "--every", "0.05", "--count", "3",
        ]),
        &GAME,
    )
    .unwrap();
    let said = run(&dir, &options, &Snapshot, None).unwrap();
    // Snapshot's 2 × 1 frames in two columns: 2 + 2 × 4 by 2 + 2 × 17.
    assert_eq!(
        said,
        "frame-png: 3 frames, every 0.050 s (3 frames) → s.png (10×36)\n  \
         +0ms 1 steps\n  +50ms 4 steps\n  +100ms 7 steps\nwarning: GameArg { flag: \
         \"--light\", value: Some(\"dusk\") }"
    );
    let (w, h, _) = decode_png(&fs::read(dir.join("s.png")).unwrap()).unwrap();
    assert_eq!((w, h), (10, 36));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn run_writes_an_apng_of_the_frames() {
    let dir = temp_dir("apng");
    let options = parse_args(
        &args(&[
            "a.apng", "--every", "0.1", "--count", "2", "--apng", "--scale", "3",
        ]),
        &GAME,
    )
    .unwrap();
    let said = run(&dir, &options, &Snapshot, None).unwrap();
    assert_eq!(
        said,
        "frame-png: 2 frames, every 0.100 s (6 frames) → a.apng (3×1)\n  \
         +0ms 0 steps\n  +100ms 6 steps"
    );
    let apng = fs::read(dir.join("a.apng")).unwrap();
    let reader = png::Decoder::new(std::io::Cursor::new(&apng))
        .read_info()
        .unwrap();
    let control = reader.info().animation_control().unwrap();
    assert_eq!((control.num_frames, control.num_plays), (2, 0));
    let shown = reader.info().frame_control().unwrap();
    assert_eq!((shown.delay_num, shown.delay_den), (6, 60));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn run_writes_a_gif_of_the_frames_and_says_its_speed() {
    let dir = temp_dir("gif");
    let options = parse_args(
        &args(&["a.gif", "--every", "0.1", "--count", "2", "--gif"]),
        &GAME,
    )
    .unwrap();
    // 6 frames is 10 hundredths: exact, no speed line.
    let said = run(&dir, &options, &Snapshot, None).unwrap();
    assert_eq!(
        said,
        "frame-png: 2 frames, every 0.100 s (6 frames) → a.gif (2×1)\n  \
         +0ms 0 steps\n  +100ms 6 steps"
    );
    let bytes = fs::read(dir.join("a.gif")).unwrap();
    let mut decoder = gif::DecodeOptions::new().read_info(&bytes[..]).unwrap();
    assert_eq!(decoder.repeat(), gif::Repeat::Infinite);
    let mut delays = Vec::new();
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        delays.push(frame.delay);
    }
    assert_eq!(delays, [10, 10]);
    // 1 frame shows for 2 hundredths, not 1/60 s: said.
    let options = parse_args(
        &args(&["a.gif", "--every", "0.0167", "--count", "2", "--gif"]),
        &GAME,
    )
    .unwrap();
    let said = run(&dir, &options, &Snapshot, None).unwrap();
    assert!(
        said.ends_with(
            "+16ms 1 steps\nthe GIF shows each frame 0.02 seconds (every is 0.017 seconds)"
        ),
        "{said}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn the_summary_says_a_gifs_speed_only_when_it_differs() {
    assert_eq!(gif_speed(3), None);
    assert_eq!(gif_speed(6), None);
    assert_eq!(gif_speed(60), None);
    assert_eq!(
        gif_speed(2).as_deref(),
        Some("the GIF shows each frame 0.03 seconds (every is 0.033 seconds)")
    );
    assert_eq!(
        gif_speed(2000).as_deref(),
        Some("the GIF shows each frame 33.33 seconds (every is 33.333 seconds)")
    );
}

/// A game whose every frame has `width` colours, one a pixel.
struct Colours(u32);

impl GameTools for Colours {
    fn frame_options(&self) -> &[crate::GameOption] {
        &GAME
    }

    fn frame(&self, _: &[GameArg], _: &Script, _: Option<u32>) -> Result<Shot, String> {
        let rgba = (0..self.0)
            .flat_map(|c| {
                let [_, r, g, b] = c.to_be_bytes();
                [r, g, b, 255]
            })
            .collect();
        Ok(Shot::new(
            Image {
                width: self.0,
                height: 1,
                rgba,
            },
            "colours",
        ))
    }

    fn lang(&self) -> Result<crate::GameLang, String> {
        Err("no languages".to_owned())
    }
}

#[test]
fn a_gif_of_too_many_colours_is_refused_and_nothing_written() {
    let dir = temp_dir("gif-colours");
    let options = parse_args(
        &args(&["a.gif", "--every", "0.05", "--count", "2", "--gif"]),
        &GAME,
    )
    .unwrap();
    assert_eq!(
        run(&dir, &options, &Colours(257), None).unwrap_err(),
        "frame 1 has 257 colours, more than the 256 of a GIF: write it with --apng"
    );
    assert!(!dir.join("a.gif").exists());
    // 256 is a GIF.
    run(&dir, &options, &Colours(256), None).unwrap();
    assert!(dir.join("a.gif").exists());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_one_bit_targets_frames_are_painted_as_its_display_shows_them() {
    let dir = temp_dir("sheet-target");
    let profiles = crate::contract::target::engine_profiles().unwrap();
    let options = parse_args(
        &args(&[
            "a.png", "--every", "0.1", "--count", "2", "--target", "playdate",
        ]),
        &GAME,
    )
    .unwrap();
    let playdate = Some(&profiles["playdate"].profile);
    assert!(
        run(&dir, &options, &Snapshot, playdate)
            .unwrap_err()
            .contains("frame_one_bit")
    );
    let said = run(&dir, &options, &ToyTools, playdate).unwrap();
    assert!(said.contains("+100ms home on playdate"), "{said}");
    fs::remove_dir_all(&dir).unwrap();
}

/// The toy's tools with the default [`GameTools::frames`].
struct ByDefault;

impl GameTools for ByDefault {
    fn frame_options(&self) -> &[crate::GameOption] {
        ToyTools.frame_options()
    }

    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        ToyTools.frame(options, script, scale)
    }

    fn lang(&self) -> Result<crate::GameLang, String> {
        ToyTools.lang()
    }
}

#[test]
fn the_toys_frames_are_the_defaults() {
    let lobby = [GameArg {
        flag: "--lobby".to_owned(),
        value: None,
    }];
    let script = Script::new(vec![Step::Keys("Down Down f".into())]);
    let capture = Capture::new(2, 6).unwrap();
    let own = ToyTools.frames(&lobby, &script, Some(1), capture).unwrap();
    let default = ByDefault.frames(&lobby, &script, Some(1), capture).unwrap();
    assert_eq!(own.len(), 6);
    assert_eq!(own, default);
    // The first is the frame the script ends on.
    assert_eq!(own[0], ToyTools.frame(&lobby, &script, Some(1)).unwrap());
}
