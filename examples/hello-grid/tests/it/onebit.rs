//! ADR-0008 §4 on Hello, Grid: its look on a black and white screen.
//! `files/palette@onebit.ron` gives every role of its palette patterns
//! and keeps apart what its colours tell apart, and `frame-png --target
//! playdate` paints its title as the Playdate shows it, checked as a
//! snapshot (the `.png` beside the `.snap` is the picture).

use std::fs;
use std::path::PathBuf;

use hello_grid::tools::{HelloTools, one_bit_skin};
use shuyi_assets::onebit::Pattern;
use shuyi_cli::GameTools;
use shuyi_cli::contract::target::{Level, engine_profiles, one_bit};
use shuyi_cli::frame_png::{self, Script};

#[test]
fn every_role_has_patterns_and_the_pieces_and_the_ground_stay_apart() {
    let skin = one_bit_skin().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(skin.palette_file, "files/palette.ron");
    assert_eq!(skin.map_file, "files/palette@onebit.ron");
    assert_eq!(one_bit(&skin), [], "no error, no warning");
    let offered = HelloTools.one_bit("playdate");
    assert_eq!(offered, Some(Ok(skin.clone())));
    // The foe drawn white like your piece: the check names both.
    let mut alike = skin;
    let foe = alike.map.roles.get_mut("foe").unwrap_or_else(|| panic!());
    foe.fg = Pattern::WHITE;
    let threat = alike
        .map
        .roles
        .get_mut("threat")
        .unwrap_or_else(|| panic!());
    threat.fg = Pattern::WHITE;
    let problems = one_bit(&alike);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert_eq!(problems[0].level, Level::Error);
    assert_eq!(problems[0].roles, ["hero", "foe"]);
}

/// A fresh empty folder under the system's temp folder.
fn temp_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hello-grid-onebit-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    dir
}

#[test]
fn frame_png_on_the_playdate_paints_the_title_in_black_and_white() {
    let dir = temp_dir();
    let options = frame_png::parse_args(
        &[
            "title.png".to_owned(),
            "--target".to_owned(),
            "playdate".to_owned(),
        ],
        HelloTools.frame_options(),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let profiles = engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
    let said = frame_png::run(
        &dir,
        &options,
        &HelloTools,
        Some(&profiles["playdate"].profile),
    )
    .unwrap_or_else(|e| panic!("{e}"));
    // The Playdate's whole screen, though the frame is taller: the
    // README says why it doesn't fit.
    assert_eq!(said, "frame-png: title → title.png (400×240)");
    let png = fs::read(dir.join("title.png")).unwrap_or_else(|e| panic!("{e}"));
    let _ = fs::remove_dir_all(&dir);
    insta::assert_binary_snapshot!("title_playdate.png", png);
    // Twice the size with a scale of 2.
    let big = HelloTools
        .frame_one_bit("playdate", &[], &Script::default(), Some(2))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!((big.image.width, big.image.height), (800, 480));
    assert_eq!(big.shows, "title");
}
