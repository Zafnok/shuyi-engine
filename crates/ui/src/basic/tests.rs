//! `BasicData` on the blank template's own files, in a table layer, and
//! on those files broken.

use shuyi_assets::bundle::Layer;
use shuyi_assets::keymap::{Layout, UiAction};

use super::*;

/// The blank template's files.
const EN: &[u8] = include_bytes!("../../../../templates/blank/crates/game/files/en.ron");
const KEYMAP: &[u8] = include_bytes!("../../../../templates/blank/crates/game/files/keymap.ron");
const PALETTE: &[u8] = include_bytes!("../../../../templates/blank/crates/game/files/palette.ron");
const ATLAS_RON: &[u8] =
    include_bytes!("../../../../templates/blank/crates/game/files/font/atlas.ron");
const ATLAS_PNG: &[u8] =
    include_bytes!("../../../../templates/blank/crates/game/files/font/atlas.png");

/// Every file a game on the short path has.
static GOOD: &[(&str, &[u8])] = &[
    ("en.ron", EN),
    ("font/atlas.png", ATLAS_PNG),
    ("font/atlas.ron", ATLAS_RON),
    ("keymap.ron", KEYMAP),
    ("palette.ron", PALETTE),
];

/// The console of the tests.
const CONSOLE: ConsoleSpec = ConsoleSpec::text(48, 24);

type Data = BasicData<UiAction>;

fn bundle(files: &'static [(&'static str, &'static [u8])]) -> Bundle {
    Bundle::new(vec![Layer::table("files", files)])
}

/// No part of the game's own.
const NO_OWN: Own<()> = |_| Ok(());

/// The files each problem names.
fn files_named(errors: &[ContentError]) -> Vec<String> {
    let mut files: Vec<String> = errors.iter().map(|e| e.file.clone()).collect();
    files.dedup();
    files
}

#[test]
fn the_standard_files_load_into_the_games_data() {
    let data = Data::load(bundle(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        data.lang().english("title.subtitle"),
        Some("made with Shuyi Engine")
    );
    assert_eq!(
        data.keymap_def().first_layout(),
        Some(Layout::named("Arrows"))
    );
    assert!(data.palette_def().get("highlight").is_some());
    assert!(data.audio_manifest().sounds.is_empty());
    assert_eq!(data.console(), CONSOLE);
    assert!(data.font.glyphs.len() >= BASIC_GLYPHS.chars().count());
    // The font's picture is the font's, not a picture of the game's.
    assert!(data.images.images.is_empty());
    assert!(data.bundle.has(FONT_PNG));
}

#[test]
fn what_a_game_leaves_out_is_one_palette_and_no_voices() {
    let data = Data::load(bundle(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    assert!(data.theme_def(()).is_none());
    assert_eq!(data.voice_variant(), ());
}

#[test]
fn every_missing_file_is_named_at_once() {
    static NONE: &[(&str, &[u8])] = &[];
    let errors = Data::load(bundle(NONE), CONSOLE, NO_OWN).unwrap_err();
    let files = files_named(&errors);
    for name in [ENGLISH_RON, KEYMAP_RON, PALETTE_RON, FONT_RON] {
        assert!(
            files.iter().any(|f| f == &format!("files/{name}")),
            "{name}: {errors:?}"
        );
    }
}

#[test]
fn a_palette_without_an_engine_colour_is_refused() {
    static NO_FOCUS: &[(&str, &[u8])] = &[
        ("en.ron", EN),
        ("font/atlas.png", ATLAS_PNG),
        ("font/atlas.ron", ATLAS_RON),
        ("keymap.ron", KEYMAP),
        ("palette.ron", br##"{ "text": "#ffffff" }"##),
    ];
    let errors = Data::load(bundle(NO_FOCUS), CONSOLE, NO_OWN).unwrap_err();
    assert_eq!(files_named(&errors), ["files/palette.ron"]);
    assert!(
        errors.iter().any(|e| e.to_string().contains("focus")),
        "{errors:?}"
    );
}

/// The blank template's files without the keymap.
static NO_KEYMAP: &[(&str, &[u8])] = &[
    ("en.ron", EN),
    ("font/atlas.png", ATLAS_PNG),
    ("font/atlas.ron", ATLAS_RON),
    ("palette.ron", PALETTE),
];

#[test]
fn the_games_own_part_is_read_and_its_problems_join_the_rest() {
    let stage: Own<usize> = |bundle| Ok(bundle.files_under("").len());
    let data = BasicData::<UiAction, usize>::load(bundle(GOOD), CONSOLE, stage)
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(data.own, GOOD.len());
    let broken: Own<usize> = |_| Err(vec![ContentError::new("files/stage.ron", "no stage")]);
    let errors =
        BasicData::<UiAction, usize>::load(bundle(NO_KEYMAP), CONSOLE, broken).unwrap_err();
    assert_eq!(
        files_named(&errors),
        ["files/keymap.ron", "files/stage.ron"]
    );
}
