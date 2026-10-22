//! `bake` and `check --baked` on a made-up game.

use std::path::{Path, PathBuf};

use super::*;
use crate::contract::check::{CheckInput, check};
use crate::contract::fake::{Fake, SOURCES};
use crate::project::Config;

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
        parse_args(&args(&["--target", "playdate", "--out", "p.shuyipak"])),
        Ok(BakeInput {
            target: Some("playdate".to_owned()),
            out: Some("p.shuyipak".to_owned()),
        })
    );
    assert_eq!(parse_args(&[]), Ok(BakeInput::default()));
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
        "usage: shuyi bake [--target <id>] [--out <file>] [--json]"
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
        }),
    };
    assert_eq!(
        text(&output),
        "bake: p.shuyipak: 4 files, 1 baked (palette)\n  size: sources 200 bytes, pack 50 bytes \
         (25%)\n  baked in 1.5 ms\n  load: sources 0.9 ms, pack 0.3 ms\n"
    );
}

#[test]
fn times_are_whole_microseconds_and_the_fastest_run_counts() {
    assert_eq!(micros(Duration::from_micros(1_234)), 1_234);
    assert_eq!(micros(Duration::from_nanos(999)), 0);
    let mut runs = 0;
    let slowest_first = fastest(|| {
        runs += 1;
        std::thread::sleep(Duration::from_millis(if runs == 1 { 20 } else { 2 }));
    });
    assert_eq!(runs, LOAD_RUNS);
    assert!((2_000..20_000).contains(&slowest_first), "{slowest_first}");
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
