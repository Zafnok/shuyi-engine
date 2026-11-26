use super::*;

fn args(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

/// A fresh empty folder under the system temp folder.
fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-sfx-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn input(source: &str) -> SfxInput {
    SfxInput {
        source: source.to_owned(),
        ..SfxInput::default()
    }
}

#[test]
fn parse_args_takes_a_source_and_each_flag_once_in_any_order() {
    let parsed = parse_args(&args(&[
        "--wav",
        "c.wav",
        "coin",
        "--seed",
        "3",
        "--out",
        "c.sfx.ron",
    ]))
    .unwrap();
    assert_eq!(
        parsed,
        SfxInput {
            source: "coin".to_owned(),
            seed: Some(3),
            out: Some("c.sfx.ron".to_owned()),
            wav: Some("c.wav".to_owned()),
        }
    );
    assert_eq!(parse_args(&args(&["coin"])).unwrap(), input("coin"));
    for (wrong, why) in [
        (&[][..], "a preset or a .sfx.ron file"),
        (&["coin", "jump"], "one preset or file"),
        (&["coin", "--seed"], "--seed needs a value"),
        (&["coin", "--seed", "x"], "--seed: a whole number, not x"),
        (&["coin", "--seed", "-1"], "--seed: a whole number, not -1"),
        (&["coin", "--out", "--wav", "a"], "--out needs a value"),
        (&["coin", "--out", "a", "--out", "b"], "--out given twice"),
        (&["coin", "--wav", "a", "--wav", "b"], "--wav given twice"),
        (
            &["coin", "--seed", "1", "--seed", "2"],
            "--seed given twice",
        ),
        (&["coin", "--loud"], "no flag --loud"),
    ] {
        assert_eq!(parse_args(&args(wrong)), Err(why.to_owned()), "{wrong:?}");
    }
}

#[test]
fn a_preset_gives_its_text_and_measures_and_a_seed_varies_it() {
    let dir = scratch("preset");
    let answer = sfx(&input("coin"), &dir);
    assert!(answer.ok, "{answer:?}");
    let output = answer.result.unwrap();
    assert_eq!(output.presets.len(), sfx::PRESETS.len());
    assert!(output.text.starts_with("// The engine's preset `coin`"));
    let def = sfx::preset("coin", 0).unwrap();
    assert_eq!(sfx::from_source("coin", &output.text), Ok(def.clone()));
    assert_eq!(output.length_ms, def.length_ms());
    // 44 100 samples a second.
    assert_eq!(output.samples, 441 * def.length_ms() / 10);
    // As loud as its volume lets it be, and no louder.
    let top = 32_767 * u32::from(def.volume) / 100;
    assert!(
        output.peak > 0 && u32::from(output.peak) <= top,
        "{}",
        output.peak
    );
    assert_eq!(output.written, Vec::<String>::new());
    // Another seed is another sound of the same preset, and says so.
    let mut seeded = input("coin");
    seeded.seed = Some(7);
    let output = sfx(&seeded, &dir).result.unwrap();
    assert!(output.text.contains("seed 7"), "{}", output.text);
    let varied = sfx::from_source("coin", &output.text).unwrap();
    assert_eq!(Some(varied), sfx::preset("coin", 7));
    // Nothing was written.
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn out_and_wav_write_the_text_and_the_sound_making_their_folder() {
    let dir = scratch("write");
    let mut asked = input("jump");
    asked.out = Some("files/sfx/jump.sfx.ron".to_owned());
    asked.wav = Some("jump.wav".to_owned());
    let answer = sfx(&asked, &dir);
    assert!(answer.ok, "{answer:?}");
    let output = answer.result.unwrap();
    assert_eq!(output.written, ["files/sfx/jump.sfx.ron", "jump.wav"]);
    let text = fs::read_to_string(dir.join("files/sfx/jump.sfx.ron")).unwrap();
    assert_eq!(text, output.text);
    // LF line ends alone, on Windows too: the same bytes on every platform.
    assert!(!text.contains('\r'), "{text:?}");
    let def = sfx::preset("jump", 0).unwrap();
    assert_eq!(sfx::from_source("jump", &text), Ok(def.clone()));
    let wav = fs::read(dir.join("jump.wav")).unwrap();
    assert_eq!(wav, sfx::to_wav(&sfx::render(&def)));
    // The file written reads back as a file: its text in the engine's form.
    let again = sfx(&input("files/sfx/jump.sfx.ron"), &dir).result.unwrap();
    assert_eq!(again.text, sfx::to_text(&def));
    assert_eq!(again.samples, output.samples);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_file_with_problems_fails_with_each_on_its_line_and_writes_nothing() {
    let dir = scratch("broken");
    let broken = "(\n    wave: Sine,\n    hz: 440,\n    hold_ms: 10,\n    fade_ms: 10,\n    volume: 150,\n)\n";
    fs::write(dir.join("loud.sfx.ron"), broken).unwrap();
    let mut asked = input("loud.sfx.ron");
    asked.out = Some("out.sfx.ron".to_owned());
    let answer = sfx(&asked, &dir);
    assert!(!answer.ok);
    assert_eq!(answer.code, None);
    let output = answer.result.unwrap();
    assert_eq!(output.text, broken);
    assert_eq!((output.length_ms, output.samples, output.peak), (0, 0, 0));
    let [problem] = output.problems.as_slice() else {
        panic!("{:?}", output.problems);
    };
    assert_eq!(
        (problem.file.as_str(), problem.line),
        ("loud.sfx.ron", Some(6))
    );
    assert_eq!(problem.message, "volume 150 is over 100 (percent)");
    assert_eq!(output.written, Vec::<String>::new());
    assert!(!dir.join("out.sfx.ron").exists());
    assert_eq!(
        text(&output),
        "loud.sfx.ron:6: volume 150 is over 100 (percent)\n"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn an_unknown_name_lists_the_presets_and_a_missing_file_says_so() {
    let dir = scratch("unknown");
    let answer = sfx(&input("nope"), &dir);
    assert_eq!(answer.code.as_deref(), Some("SH0001"));
    let error = answer.error.unwrap();
    assert!(
        error.starts_with("no preset nope: the presets are blip, tick, select"),
        "{error}"
    );
    assert!(error.ends_with("power_up, laser"), "{error}");
    let answer = sfx(&input("missing.sfx.ron"), &dir);
    assert_eq!(answer.code.as_deref(), Some("SH3004"));
    assert!(answer.error.unwrap().starts_with("missing.sfx.ron: "));
    // A seed varies a preset, not a file.
    let mut seeded = input("missing.sfx.ron");
    seeded.seed = Some(1);
    assert_eq!(sfx(&seeded, &dir).code.as_deref(), Some("SH0001"));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn a_file_that_cant_be_written_fails_with_what_was_written_before_it() {
    let dir = scratch("unwritable");
    // A file where the folder should be.
    fs::write(dir.join("taken"), "").unwrap();
    let mut asked = input("blip");
    asked.out = Some("blip.sfx.ron".to_owned());
    asked.wav = Some("taken/blip.wav".to_owned());
    let answer = sfx(&asked, &dir);
    assert!(!answer.ok);
    assert_eq!(answer.code.as_deref(), Some("SH3004"));
    assert!(answer.error.unwrap().starts_with("taken/blip.wav: "));
    assert_eq!(answer.result.unwrap().written, ["blip.sfx.ron"]);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn text_gives_the_sound_its_measures_and_the_files_written() {
    let output = SfxOutput {
        text: "(wave: Sine)\n".to_owned(),
        length_ms: 50,
        samples: 2_205,
        peak: 9_000,
        written: vec!["a.wav".to_owned()],
        ..SfxOutput::default()
    };
    assert_eq!(
        text(&output),
        "(wave: Sine)\n50 ms, 2205 samples, peak 9000\nwrote a.wav\n"
    );
    assert!(usage("cargo shuyi").starts_with("usage: cargo shuyi sfx <preset | file.sfx.ron>"));
    assert!(usage("shuyi").ends_with("power_up, laser."));
}
