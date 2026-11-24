//! `BasicData` on the blank template's own files, in a table layer, and
//! on those files broken.

use shuyi_assets::bundle::Layer;
use shuyi_assets::keymap::{Layout, UiAction};

use super::*;

/// The blank template's files.
const EN: &[u8] =
    include_bytes!("../../../../templates/shuyi-default/base/crates/game/files/en.ron");
const KEYMAP: &[u8] =
    include_bytes!("../../../../templates/shuyi-default/base/crates/game/files/keymap.ron");
const PALETTE: &[u8] =
    include_bytes!("../../../../templates/shuyi-default/base/crates/game/files/palette.ron");
const ATLAS_RON: &[u8] =
    include_bytes!("../../../../templates/shuyi-default/base/crates/game/files/font/atlas.ron");
const ATLAS_PNG: &[u8] =
    include_bytes!("../../../../templates/shuyi-default/base/crates/game/files/font/atlas.png");

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
        data.lang().source_text("title.subtitle"),
        Some("made with Shuyi Engine")
    );
    assert_eq!(
        data.keymap_def().first_layout(),
        Some(Layout::named("Arrows"))
    );
    assert!(data.palette_def().get("highlight").is_some());
    let found = &data.audio_manifest().sounds;
    assert!(found.is_empty(), "{found:?}");
    assert_eq!(data.console(), CONSOLE);
    assert!(data.font.glyphs.len() >= BASIC_GLYPHS.chars().count());
    // The font's picture is the font's, not a picture of the game's.
    assert!(data.images.images.is_empty(), "{:?}", data.images.images);
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

/// A 16-bit mono WAV at the engine's rate: two silent samples.
const BEEP: &[u8] = &[
    b'R', b'I', b'F', b'F', 40, 0, 0, 0, b'W', b'A', b'V', b'E', b'f', b'm', b't', b' ', 16, 0, 0,
    0, 1, 0, 1, 0, 0x44, 0xac, 0, 0, 0x88, 0x58, 1, 0, 2, 0, 16, 0, b'd', b'a', b't', b'a', 4, 0,
    0, 0, 0, 0, 0, 0,
];

/// A manifest of one sound, `sfx/beep.wav`.
const AUDIO: &[u8] = b"(music_fade_ms: 500, sounds: [(id: \"ui_move\", files: [\"sfx/beep.wav\"],     volume: 50, credit: Own)], music: [], pools: [], credits: [])";

#[test]
fn an_audio_manifest_in_the_bundle_is_read_with_its_sounds() {
    static WITH_AUDIO: &[(&str, &[u8])] = &[
        ("audio.ron", AUDIO),
        ("en.ron", EN),
        ("font/atlas.png", ATLAS_PNG),
        ("font/atlas.ron", ATLAS_RON),
        ("keymap.ron", KEYMAP),
        ("palette.ron", PALETTE),
        ("sfx/beep.wav", BEEP),
    ];
    let data = Data::load(bundle(WITH_AUDIO), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    let sounds = &data.audio_manifest().sounds;
    assert_eq!(sounds["ui_move"].files, ["sfx/beep.wav"]);
    assert_eq!(data.audio_manifest().music_fade_ms, 500);
    // Its sound file missing is an error of the manifest, among the rest.
    static NO_SOUND: &[(&str, &[u8])] = &[
        ("audio.ron", AUDIO),
        ("en.ron", EN),
        ("font/atlas.png", ATLAS_PNG),
        ("font/atlas.ron", ATLAS_RON),
        ("palette.ron", PALETTE),
    ];
    let errors = Data::load(bundle(NO_SOUND), CONSOLE, NO_OWN).unwrap_err();
    assert_eq!(
        files_named(&errors),
        ["files/keymap.ron", "files/audio.ron"]
    );
    let missing = errors.iter().find(|e| e.file == "files/audio.ron").unwrap();
    assert!(missing.message.contains("sfx/beep.wav"), "{missing}");
}

#[test]
fn a_pack_of_the_files_loads_the_same_data_with_no_text_parsed() {
    let formats = Data::formats();
    let bake = Data::bake("blank 1");
    let source = bundle(GOOD);
    let baked = bake
        .bake(&formats, &source, None)
        .unwrap_or_else(|e| panic!("{e:?}"));
    let pack =
        shuyi_assets::pack::Pack::open(&baked.bytes, "blank 1").unwrap_or_else(|e| panic!("{e}"));
    let packed = Bundle::new(vec![pack.into_owned().into_layer("files")]);
    assert_eq!(bake.check(&formats, &source, &packed), []);
    let kinds: Vec<(&str, &[&str])> = baked
        .files
        .iter()
        .map(|file| (file.path.as_str(), &file.kinds[..]))
        .collect();
    assert_eq!(
        kinds,
        [
            ("en.ron", &["english"][..]),
            ("font/atlas.png", &[][..]),
            ("font/atlas.ron", &["font atlas"][..]),
            ("keymap.ron", &[][..]),
            ("palette.ron", &["palette"][..]),
        ]
    );
    let from_pack = Data::load(packed, CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    let from_text = Data::load(bundle(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(
        from_pack.lang().source_text("title.subtitle"),
        from_text.lang().source_text("title.subtitle")
    );
    assert_eq!(from_pack.palette_def(), from_text.palette_def());
    assert_eq!(from_pack.font, from_text.font);
    assert_eq!(from_pack.keymap_def(), from_text.keymap_def());
}
