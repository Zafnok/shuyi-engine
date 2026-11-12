//! `bake` and `check --baked` on a made-up game.

use std::path::{Path, PathBuf};

use super::*;
use crate::contract::check::{CheckInput, check};
use crate::contract::fake::{Fake, SOURCES};
use crate::project::{Config, GameProgram};

/// The project of no game file at `root`, which has the engine's target
/// profiles.
fn project(root: &Path) -> Project {
    Project {
        root: root.to_path_buf(),
        config: Config::default(),
    }
}

/// A fresh folder for one test.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("shuyi-bake-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Sources whose names table is broken and whose palette is missing.
static BROKEN: &[(&str, &[u8])] = &[("data/names.ron", b"[")];

#[test]
fn bake_writes_the_pack_and_says_what_went_in_and_how_long_it_took() {
    let dir = folder("writes");
    let input = BakeInput {
        target: Some("playdate".to_owned()),
        out: None,
        sheet: None,
        content: None,
        audio: None,
        check: false,
    };
    let answer = bake(&Fake::default(), &project(&dir), &input);
    assert!(answer.ok, "{answer:?}");
    let pack = answer.result.as_ref().unwrap().pack.clone().unwrap();
    assert_eq!(pack.file, "target/shuyi/playdate.shuyipak");
    assert_eq!((pack.files, pack.baked), (3, 2));
    assert_eq!(pack.kinds, ["names table", "palette"]);
    let bytes = fs::read(dir.join(&pack.file)).unwrap();
    assert_eq!(pack.pack_bytes, bytes.len() as u64);
    let opened = Pack::open(&bytes, "fake for playdate").unwrap();
    assert_eq!(opened.info().target.as_deref(), Some("playdate"));
    let source: usize = SOURCES.iter().map(|f| f.1.len()).sum();
    assert_eq!(pack.source_bytes, source as u64);
    assert_eq!(answer.limits.len(), 1);
    let shown = text(answer.result.as_ref().unwrap());
    assert!(
        shown.starts_with(
            "bake: target/shuyi/playdate.shuyipak for playdate: 3 files, 2 baked (names table, \
             palette)\n"
        ),
        "{shown}"
    );
    assert!(shown.contains("\n  load: sources "), "{shown}");
    // Somewhere else, for no target.
    let input = BakeInput {
        target: None,
        out: Some("out/p.shuyipak".to_owned()),
        sheet: None,
        content: None,
        audio: None,
        check: false,
    };
    assert!(bake(&Fake::default(), &project(&dir), &input).ok);
    assert!(dir.join("out/p.shuyipak").is_file());
    assert_eq!(
        out_file(&BakeInput::default()),
        "target/shuyi/game.shuyipak"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bake_refuses_sources_with_errors_and_makes_no_pack() {
    let game = Fake {
        sources: Some(BROKEN),
        ..Fake::default()
    };
    let dir = folder("broken");
    let answer = bake(&game, &project(&dir), &BakeInput::default());
    assert!(!answer.ok);
    let output = answer.result.unwrap();
    assert_eq!(output.pack, None);
    let shown = text(&output);
    assert!(
        shown.starts_with("bake: 2 error(s) in the sources; no pack made\n"),
        "{shown}"
    );
    assert!(
        shown.contains("base/data/names.ron:1:1: error[SH2001]: "),
        "{shown}"
    );
    assert!(
        shown.contains("base/data/palette: error[SH2003]: file not found"),
        "{shown}"
    );
    assert!(shown.contains("\nhelp[SH2003]: "), "{shown}");
    assert!(shown.ends_with(&format!(
        "help[SH2001]: {}\n",
        codes::help_of("SH2001").unwrap()
    )));
    assert!(!dir.join(OUT_DIR).exists());
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bake_refuses_a_target_with_no_profile() {
    let dir = folder("target");
    let input = BakeInput {
        target: Some("toaster".to_owned()),
        out: None,
        sheet: None,
        content: None,
        audio: None,
        check: false,
    };
    let answer = bake(&Fake::default(), &project(&dir), &input);
    let error = answer.error.unwrap_or_default();
    assert!(
        error.starts_with("no target profile \"toaster\"; the profiles are android, desktop, "),
        "{error}"
    );
    assert!(!dir.join(OUT_DIR).exists());
    // A game's own profiles that don't load say so.
    let broken = Project {
        root: dir.clone(),
        config: Config {
            target_profiles: Some("nowhere".to_owned()),
            ..Config::default()
        },
    };
    let error = bake(&Fake::default(), &broken, &input)
        .error
        .unwrap_or_default();
    assert!(
        error.starts_with("the target profiles don't load: nowhere: "),
        "{error}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn bake_says_why_it_could_not_run() {
    let dir = folder("cant");
    let none = Fake {
        sources: None,
        ..Fake::default()
    };
    assert_eq!(
        bake(&none, &project(&dir), &BakeInput::default())
            .error
            .as_deref(),
        Some("no bake")
    );
    // A file where the pack's folder should be.
    fs::write(dir.join("target"), "").unwrap();
    let answer = bake(&Fake::default(), &project(&dir), &BakeInput::default());
    let error = answer.error.unwrap_or_default();
    assert!(error.starts_with("target/shuyi/game.shuyipak: "), "{error}");
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_baked_proves_the_pack_equal_to_its_sources() {
    let baked = CheckInput {
        baked: true,
        ..CheckInput::default()
    };
    let answer = check(&Fake::default(), &baked);
    assert!(answer.ok, "{answer:?}");
    assert_eq!(check_baked(&Fake::default()), Ok(Vec::new()));
    let broken = Fake {
        sources: Some(BROKEN),
        ..Fake::default()
    };
    // The plain check doesn't bake; the baked one finds the sources' errors.
    assert!(check(&broken, &CheckInput::default()).ok);
    let answer = check(&broken, &baked);
    assert_eq!(answer.result.unwrap().errors.len(), 2);
    let none = Fake {
        sources: None,
        ..Fake::default()
    };
    assert_eq!(check(&none, &baked).error.as_deref(), Some("no bake"));
}

#[test]
fn bakes_arguments_are_a_target_and_a_file() {
    let args = |items: &[&str]| items.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
    assert_eq!(
        parse_args(&args(&[
            "--target",
            "playdate",
            "--out",
            "p.shuyipak",
            "--sheet",
            "s.png"
        ])),
        Ok(BakeInput {
            target: Some("playdate".to_owned()),
            out: Some("p.shuyipak".to_owned()),
            sheet: Some("s.png".to_owned()),
            content: None,
            audio: None,
            check: false,
        })
    );
    assert_eq!(
        parse_args(&args(&["--content", "demo/files", "--audio", "out/audio"])),
        Ok(BakeInput {
            content: Some("demo/files".to_owned()),
            audio: Some("out/audio".to_owned()),
            ..BakeInput::default()
        })
    );
    assert_eq!(parse_args(&[]), Ok(BakeInput::default()));
    assert_eq!(
        parse_args(&args(&["--out", "g.pack", "--check"])),
        Ok(BakeInput {
            out: Some("g.pack".to_owned()),
            check: true,
            ..BakeInput::default()
        })
    );
    assert_eq!(
        parse_args(&args(&["--target"])),
        Err("--target needs a value".to_owned())
    );
    assert_eq!(
        parse_args(&args(&["x"])),
        Err("unknown argument: x".to_owned())
    );
    assert_eq!(
        usage("shuyi"),
        "usage: shuyi bake [--target <id>] [--out <file>] [--sheet <file>] [--content <folder>] \
         [--audio <folder>] [--check] [--json]"
    );
    assert_eq!(bytes(1_234_567), "1,234,567");
    assert_eq!(bytes(123_456), "123,456");
    assert_eq!(bytes(12), "12");
    assert_eq!(ms(1_234), "1.2 ms");
}

#[test]
fn the_text_says_how_big_the_pack_is_beside_its_sources() {
    let output = BakeOutput {
        errors: Vec::new(),
        pack: Some(PackMade {
            file: "p.shuyipak".to_owned(),
            target: None,
            files: 4,
            baked: 1,
            kinds: vec!["palette".to_owned()],
            source_bytes: 200,
            pack_bytes: 50,
            bake_us: 1_500,
            load_sources_us: 900,
            load_pack_us: 300,
            dithered: Vec::new(),
            replaced: Vec::new(),
            sheet: None,
            audio_files: None,
        }),
    };
    assert_eq!(
        text(&output),
        "bake: p.shuyipak: 4 files, 1 baked (palette)\n  size: sources 200 bytes, pack 50 bytes \
         (25%)\n  baked in 1.5 ms\n  load: sources 0.9 ms, pack 0.3 ms\n"
    );
    // A one-bit bake names the pictures it dithered and those the
    // target's layer replaced, and the contact sheet.
    let mut one_bit = output;
    if let Some(pack) = &mut one_bit.pack {
        pack.dithered = vec!["assets/a.png".to_owned(), "assets/b.png".to_owned()];
        pack.replaced = vec!["assets@playdate/c.png".to_owned()];
        pack.sheet = Some("sheet.png".to_owned());
    }
    let shown = text(&one_bit);
    assert!(
        shown.ends_with(
            "  dithered 2 picture(s), to check: assets/a.png, assets/b.png\n  from the target's \
             layer: assets@playdate/c.png\n  contact sheet: sheet.png\n"
        ),
        "{shown}"
    );
}

#[test]
fn times_are_whole_microseconds_and_the_fastest_run_counts() {
    assert_eq!(micros(Duration::from_micros(1_234)), 1_234);
    assert_eq!(micros(Duration::from_nanos(999)), 0);
    // Measured runs of 20, 7, 3, 9 and 4 milliseconds, the slowest first.
    let measured = [20_000, 7_000, 3_000, 9_000, 4_000];
    let mut runs = 0;
    let slowest_first = fastest_of(|| {
        let took = Duration::from_micros(measured[runs]);
        runs += 1;
        took
    });
    assert_eq!(u32::try_from(runs), Ok(LOAD_RUNS));
    assert_eq!(slowest_first, 3_000);
    // A real run through the clock is measured as many times.
    let mut timed = 0;
    fastest(|| timed += 1);
    assert_eq!(timed, LOAD_RUNS);
}

#[test]
fn a_load_is_timed_from_the_sources_and_from_the_pack() {
    // A names table big enough that reading it takes milliseconds.
    let entries: Vec<String> = (0..20_000)
        .map(|i| format!("\"name{i}\": \"Name {i}\""))
        .collect();
    let table = format!("{{{}}}", entries.join(", "));
    let table: &'static [u8] = table.into_bytes().leak();
    let files: &'static [(&str, &[u8])] = vec![("data/names.ron", table)].leak();
    let game = Fake {
        sources: Some(files),
        ..Fake::default()
    };
    let GameBake {
        bake,
        bundle,
        formats,
    } = prepared(&game, None).unwrap();
    let bake = Bake::new(bake.stamp()).asset::<shuyi_assets::names::Names>("data/names");
    let baked = bake.bake(&formats, &bundle, None).unwrap();
    assert!(load_time(&bake, &formats, &bundle, None) > 100);
    assert!(load_time(&bake, &formats, &bundle, Some(&baked.bytes)) > 100);
}

#[test]
fn a_content_root_is_baked_by_the_tool_program_built_with_it() {
    let program = GameProgram {
        package: "moon-garden".to_owned(),
        bin: None,
        example: Some("tools".to_owned()),
        features: vec!["tools".to_owned()],
        args: Vec::new(),
    };
    let input = BakeInput {
        target: Some("playdate".to_owned()),
        sheet: Some("sheet.png".to_owned()),
        content: Some("demo/files".to_owned()),
        audio: Some("out/audio".to_owned()),
        ..BakeInput::default()
    };
    let out = out_file(&input);
    assert_eq!(out, "target/shuyi/playdate.shuyipak");
    // Its own `bake` takes all the input but the folder, which it is built
    // with, and answers in JSON; it is built in a profile of its own.
    assert_eq!(
        content_cargo_args(&program, &input, &out),
        [
            "run",
            "--profile",
            "shuyi-content",
            "--config",
            "profile.shuyi-content.inherits=\"dev\"",
            "--quiet",
            "-p",
            "moon-garden",
            "--example",
            "tools",
            "--features",
            "tools",
            "--",
            "bake",
            "--target",
            "playdate",
            "--out",
            "target/shuyi/playdate.shuyipak",
            "--sheet",
            "sheet.png",
            "--audio",
            "out/audio",
            "--json",
        ]
    );
    // A check of a pack through it is the same, with `--check`.
    let check = BakeInput {
        check: true,
        ..BakeInput::default()
    };
    assert_eq!(
        content_args(&check, "g.pack"),
        ["--out", "g.pack", "--check", "--json"]
    );
}

#[test]
fn a_content_root_is_a_folder_of_the_project_and_needs_the_tool_program() {
    let dir = folder("content");
    fs::create_dir_all(dir.join("demo/files")).unwrap();
    let found = content_folder(&dir, "demo/files").unwrap();
    assert!(found.is_absolute() && found.ends_with("files"), "{found:?}");
    for wrong in [
        "/demo",
        "demo/../files",
        "demo\\files",
        "c:/demo",
        "demo//files",
    ] {
        let refused = content_folder(&dir, wrong).unwrap_err();
        assert!(
            refused.contains("from the project's root"),
            "{wrong}: {refused}"
        );
    }
    let missing = content_folder(&dir, "demo/none").unwrap_err();
    assert!(missing.contains("isn't a folder"), "{missing}");
    let bake_of = |content: &str| {
        let input = BakeInput {
            content: Some(content.to_owned()),
            ..BakeInput::default()
        };
        bake(&Fake::default(), &project(&dir), &input)
    };
    let answer = bake_of("demo/none");
    assert_eq!(answer.code.as_deref(), Some(codes::FILE.0));
    // The tools running now embed their own files: a folder needs the tool
    // program the project names, built with it.
    let answer = bake_of("demo/files");
    assert_eq!(answer.code.as_deref(), Some(codes::PROJECT_FILE.0));
    assert!(
        answer.error.unwrap().contains("names no game tool program"),
        "a project with no `game` part"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// A WAV file of `samples` of silence, mono, 16-bit, at the audio bake's
/// rate.
fn silence(samples: usize) -> Vec<u8> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: shuyi_assets::audio::SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut bytes = std::io::Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut bytes, spec).unwrap();
    for _ in 0..samples {
        writer.write_sample(0_i16).unwrap();
    }
    writer.finalize().unwrap();
    bytes.into_inner()
}

/// The files under `dir`, by their paths from it with `/`, sorted.
fn files_under(dir: &Path) -> Vec<String> {
    let mut found = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(at) = todo.pop() {
        for entry in fs::read_dir(&at).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                todo.push(path);
            } else {
                let name = path.strip_prefix(dir).unwrap().to_string_lossy();
                found.push(name.replace(std::path::MAIN_SEPARATOR, "/"));
            }
        }
    }
    found.sort();
    found
}

#[test]
fn the_audio_is_baked_for_the_target_into_its_folder_emptied_first() {
    let dir = folder("audio");
    let game = Fake {
        sounds: Some(vec![
            ("sfx/blip.wav".to_owned(), silence(100)),
            ("sfx/door.wav".to_owned(), silence(3000)),
        ]),
        ..Fake::default()
    };
    // What an earlier bake left there is gone.
    let stale = dir.join("out/audio/audio/sfx/old.wav");
    fs::create_dir_all(stale.parent().unwrap()).unwrap();
    fs::write(&stale, b"old").unwrap();
    let input = BakeInput {
        target: Some("playdate".to_owned()),
        audio: Some("out/audio".to_owned()),
        ..BakeInput::default()
    };
    let answer = bake(&game, &project(&dir), &input);
    assert!(answer.ok, "{answer:?}");
    let output = answer.result.unwrap();
    assert_eq!(output.pack.as_ref().unwrap().audio_files, Some(2));
    assert_eq!(
        files_under(&dir.join("out/audio")),
        ["audio/sfx/blip.wav", "audio/sfx/door.wav"]
    );
    assert!(
        text(&output).ends_with("  sounds and music: 2 file(s) baked\n"),
        "{}",
        text(&output)
    );
    // Each file made is kept, so the next bake doesn't decode it again.
    assert!(audio_cache(&project(&dir)).starts_with(dir.join("target/shuyi")));
    assert_eq!(files_under(&audio_cache(&project(&dir))).len(), 2);
    // A game with no audio: the folder isn't made, and the pack is.
    let answer = bake(&Fake::default(), &project(&dir), &input);
    assert!(answer.ok, "{answer:?}");
    assert_eq!(answer.result.unwrap().pack.unwrap().audio_files, None);
    assert!(!dir.join("out/audio").exists());
    // A sound file that isn't one, and audio with no target.
    let missing = Fake {
        sounds: Some(vec![("sfx/broken.wav".to_owned(), b"not a wav".to_vec())]),
        ..Fake::default()
    };
    let answer = bake(&missing, &project(&dir), &input);
    assert_eq!(answer.code.as_deref(), Some(codes::FILE.0));
    assert!(
        answer
            .error
            .unwrap()
            .starts_with("the game's audio: sounds/sfx/broken.wav"),
        "the file, by its name"
    );
    let untargeted = BakeInput {
        target: None,
        ..input
    };
    let answer = bake(&game, &project(&dir), &untargeted);
    assert_eq!(answer.code.as_deref(), Some(codes::WRONG_ARGUMENTS.0));
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn check_proves_the_pack_file_equal_to_the_sources_and_names_a_changed_one() {
    // The sources with the names table changed after the bake.
    static CHANGED: &[(&str, &[u8])] = &[
        ("data/names.ron", br#"{"hero": "Bea"}"#),
        ("data/palette.ron", br##"{"black": "#000000"}"##),
        ("images/a.png", b"not really a picture"),
    ];
    let dir = folder("check");
    let input = BakeInput {
        target: Some("desktop".to_owned()),
        out: Some("out/game.pack".to_owned()),
        ..BakeInput::default()
    };
    let checked = BakeInput {
        check: true,
        ..input.clone()
    };
    // Nothing baked yet: no file to check.
    let answer = bake(&Fake::default(), &project(&dir), &checked);
    assert_eq!(answer.code.as_deref(), Some(codes::FILE.0), "{answer:?}");
    assert!(answer.error.unwrap().starts_with("out/game.pack: "));
    let answer = bake(&Fake::default(), &project(&dir), &input);
    assert!(answer.ok, "{answer:?}");
    let answer = bake(&Fake::default(), &project(&dir), &checked);
    assert!(answer.ok, "{answer:?}");
    let output = answer.result.unwrap();
    assert_eq!((output.errors.len(), output.pack.is_none()), (0, true));
    assert_eq!(
        text(&output),
        "bake --check: the pack reads as its sources\n"
    );
    // A source changed after the bake: the check names it.
    let changed = Fake {
        sources: Some(CHANGED),
        ..Fake::default()
    };
    let answer = bake(&changed, &project(&dir), &checked);
    assert!(!answer.ok, "{answer:?}");
    let files: Vec<String> = answer
        .result
        .unwrap()
        .errors
        .iter()
        .map(|problem| problem.file.clone())
        .collect();
    assert!(
        files.iter().any(|file| file.ends_with("data/names.ron")),
        "{files:?}"
    );
    fs::remove_dir_all(&dir).unwrap();
}
