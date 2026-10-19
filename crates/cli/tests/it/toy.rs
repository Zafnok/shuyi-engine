//! The toy game's tools, as its tool program (the `toy` example) runs
//! them on `toy/shuyi.ron`.

use std::fs;
use std::path::Path;

use shuyi_cli::codec::decode_png;
use shuyi_cli::toy::ToyTools;
use shuyi_cli::{Project, run};

use crate::{args, scratch};

/// The toy's project, with its root moved to `root` (where pictures go).
fn toy_project(root: &Path) -> Project {
    let toy = Path::new(env!("CARGO_MANIFEST_DIR")).join("toy");
    let mut project = Project::load(&toy).unwrap_or_else(|e| panic!("{e}"));
    project.root = root.to_path_buf();
    project
}

#[test]
fn the_toys_project_names_its_tool_program() {
    let project = toy_project(Path::new("."));
    let game = project.config.game.unwrap();
    assert_eq!(
        game.cargo_args("frame-png", &[]).join(" "),
        "run --quiet -p shuyi-cli --example toy --features toy -- frame-png"
    );
}

#[test]
fn frame_png_renders_the_toy() {
    let dir = scratch("toy-frame");
    let project = toy_project(&dir);
    let picture = |extra: &[&str]| {
        let mut all = vec!["frame-png", "toy.png", "--scale", "1"];
        all.extend_from_slice(extra);
        assert_eq!(run(&project, &args(&all), Some(&ToyTools)), 0, "{extra:?}");
        decode_png(&fs::read(dir.join("toy.png")).unwrap()).unwrap()
    };
    let (width, height, home) = picture(&[]);
    assert!(width > 0 && height > 0);
    // Something is drawn on the clear colour: the toy's text, as boxes.
    let first = &home[..4];
    assert!(home.chunks(4).any(|p| p != first));
    // Its own option and a step change the picture.
    let (_, _, lobby) = picture(&["--lobby"]);
    assert_ne!(lobby, home);
    let (_, _, moved) = picture(&["--lobby", "--keys", "Down"]);
    assert_ne!(moved, lobby);
    // Help and bad arguments.
    assert_eq!(
        run(&project, &args(&["frame-png", "--help"]), Some(&ToyTools)),
        0
    );
    let bad = args(&["frame-png", "t.png", "--light", "dusk"]);
    assert_eq!(run(&project, &bad, Some(&ToyTools)), 2);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn lang_status_reports_the_toys_debug_pack() {
    let project = toy_project(Path::new("."));
    assert_eq!(
        run(&project, &args(&["lang-status", "dbg"]), Some(&ToyTools)),
        0
    );
    assert_eq!(
        run(&project, &args(&["lang-status", "zz"]), Some(&ToyTools)),
        2
    );
    assert_eq!(run(&project, &args(&["lang-status"]), Some(&ToyTools)), 2);
    let report = shuyi_cli::lang_status::report(
        &shuyi_cli::GameTools::lang(&ToyTools).unwrap().lang,
        &[] as &[&str],
        "dbg",
        &shuyi_cli::lang_status::Options::default(),
    )
    .unwrap();
    assert!(
        report.starts_with("lang-status dbg (Debug, machine-made): "),
        "{report}"
    );
}
