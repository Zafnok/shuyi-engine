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
    // The same, with no keymap and no sound file.
    static NO_SOUND: &[(&str, &[u8])] = &[
        ("audio.ron", AUDIO),
        ("en.ron", EN),
        ("font/atlas.png", ATLAS_PNG),
        ("font/atlas.ron", ATLAS_RON),
        ("palette.ron", PALETTE),
    ];
    let data = Data::load(bundle(WITH_AUDIO), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    let sounds = &data.audio_manifest().sounds;
    assert_eq!(sounds["ui_move"].files, ["sfx/beep.wav"]);
    assert_eq!(data.audio_manifest().music_fade_ms, 500);
    // Its sound file missing is an error of the manifest, among the rest.
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

/// `files` in a layer that changes as a folder on disk does
/// ([`Layer::owned`]), as a bundle.
fn disk(files: &[(&str, &[u8])]) -> Bundle {
    let files = files
        .iter()
        .map(|&(path, bytes)| (path.to_owned(), bytes.to_vec()))
        .collect();
    Bundle::new(vec![
        Layer::owned("disk".to_owned(), files).unwrap_or_else(|e| panic!("{e:?}")),
    ])
}

/// `paths` as the platform says they changed.
fn changed(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|&path| path.to_owned()).collect()
}

/// `text` with `from` replaced by `to`, as bytes: an edit of a file.
fn edited(text: &[u8], from: &str, to: &str) -> Vec<u8> {
    let text = core::str::from_utf8(text).unwrap_or_else(|e| panic!("{e}"));
    assert!(text.contains(from), "{from:?}");
    text.replacen(from, to, 1).into_bytes()
}

/// The names of `reloads` and whether each reloaded, failed or not.
fn outcomes(reloads: &[(&'static str, Reload)]) -> Vec<(&'static str, &'static str)> {
    reloads
        .iter()
        .map(|(id, reload)| {
            let outcome = match reload {
                Reload::Reloaded => "reloaded",
                Reload::Failed(_) => "failed",
                _ => "unchanged",
            };
            (*id, outcome)
        })
        .collect()
}

/// The errors of `reloads`' one failure, as the overlay shows them.
fn failure(reloads: &[(&'static str, Reload)]) -> Vec<String> {
    match reloads {
        [(_, Reload::Failed(errors))] => errors.iter().map(ToString::to_string).collect(),
        other => panic!("not one failure: {other:?}"),
    }
}

/// What a reload that read nothing again answers.
const NOTHING: [(&str, Reload); 0] = [];

#[test]
fn a_file_nothing_read_reloads_nothing() {
    let mut data = Data::load(disk(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(data.reload(&changed(&["notes.txt"])), NOTHING);
    assert_eq!(data.reload(&[]), NOTHING);
    // The font atlas and the pictures don't reload here.
    assert_eq!(data.reload(&changed(&[FONT_RON, FONT_PNG])), NOTHING);
}

#[test]
fn a_changed_english_line_replaces_the_text() {
    let mut data = Data::load(disk(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    let new = edited(EN, "made with Shuyi Engine", "made anew");
    assert!(data.bundle.replace(ENGLISH_RON, new));
    let reloads = data.reload(&changed(&["notes.txt", ENGLISH_RON]));
    assert_eq!(outcomes(&reloads), [(ENGLISH_RON, "reloaded")]);
    assert_eq!(data.lang().source_text("title.subtitle"), Some("made anew"));
}

#[test]
fn a_changed_palette_colour_and_keymap_binding_replace_theirs() {
    let mut data = Data::load(disk(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    let palette = edited(PALETTE, "\"text\": \"#e4e4dc\"", "\"text\": \"#102030\"");
    let keymap = edited(
        KEYMAP,
        "\"Cancel\": [\"Backspace\", \"x\"]",
        "\"Cancel\": [\"q\"]",
    );
    assert!(data.bundle.replace(PALETTE_RON, palette));
    assert!(data.bundle.replace(KEYMAP_RON, keymap));
    let reloads = data.reload(&changed(&[PALETTE_RON, KEYMAP_RON]));
    assert_eq!(
        outcomes(&reloads),
        [(KEYMAP_RON, "reloaded"), (PALETTE_RON, "reloaded")]
    );
    assert_eq!(data.palette_def().get("text"), Some([0x10, 0x20, 0x30]));
    let layout = Layout::named("Arrows");
    let cancel = data.keymap_def().chords(layout, UiAction::Cancel);
    assert_eq!(cancel.len(), 1, "{cancel:?}");
}

#[test]
fn a_broken_file_keeps_the_old_value_until_a_good_save() {
    let mut data = Data::load(disk(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    // The second line broken.
    let broken = edited(EN, "\n", "\n\"title.name\" \"no colon\",\n");
    assert!(data.bundle.replace(ENGLISH_RON, broken));
    let reloads = data.reload(&changed(&[ENGLISH_RON]));
    let errors = failure(&reloads);
    assert!(errors[0].starts_with("disk/en.ron:2"), "{errors:?}");
    assert_eq!(
        data.lang().source_text("title.subtitle"),
        Some("made with Shuyi Engine")
    );
    // Still watched: the good save reloads it.
    assert!(data.bundle.replace(ENGLISH_RON, EN.to_vec()));
    let reloads = data.reload(&changed(&[ENGLISH_RON]));
    assert_eq!(outcomes(&reloads), [(ENGLISH_RON, "reloaded")]);
}

#[test]
fn a_reloaded_english_has_its_key_placeholders_checked() {
    let mut data = Data::load(disk(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    let unknown = edited(EN, "{Cancel} leave", "{Jump} leave");
    assert!(data.bundle.replace(ENGLISH_RON, unknown));
    let errors = failure(&data.reload(&changed(&[ENGLISH_RON])));
    assert!(
        errors[0].contains("unknown action \"{Jump}\""),
        "{errors:?}"
    );
    assert_eq!(
        data.lang().source_text("title.help"),
        Some("{Cancel} leave")
    );
}

#[test]
fn a_reloaded_palette_without_an_engine_colour_fails_and_keeps_the_old_one() {
    let mut data = Data::load(disk(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    let before = data.palette_def().clone();
    let no_focus = edited(PALETTE, "\"focus\": \"#5ac8ff\",", "");
    assert!(data.bundle.replace(PALETTE_RON, no_focus));
    let errors = failure(&data.reload(&changed(&[PALETTE_RON])));
    assert!(errors.iter().any(|e| e.contains("focus")), "{errors:?}");
    assert_eq!(*data.palette_def(), before);
}

#[test]
fn a_broken_keymap_keeps_the_old_one() {
    let mut data = Data::load(disk(GOOD), CONSOLE, NO_OWN).unwrap_or_else(|e| panic!("{e:?}"));
    let before = data.keymap_def().clone();
    let unknown = edited(KEYMAP, "\"Cancel\": [", "\"Jump\": [");
    assert!(data.bundle.replace(KEYMAP_RON, unknown));
    let errors = failure(&data.reload(&changed(&[KEYMAP_RON])));
    assert!(errors[0].starts_with("disk/keymap.ron:"), "{errors:?}");
    assert_eq!(*data.keymap_def(), before);
}

/// A stage of the game's own: the text of `stage.map`, which must not say
/// "broken".
const STAGE: Own<String> = |bundle| {
    let text = bundle.file("stage.map").unwrap_or_default();
    if text.contains("broken") {
        Err(vec![
            ContentError::new("disk/stage.map", "broken").at(1, None),
        ])
    } else {
        Ok(text.into_owned())
    }
};

#[test]
fn the_own_part_reloads_when_a_file_it_read_changes() {
    let mut files = GOOD.to_vec();
    files.push(("stage.map", b"first"));
    let mut data = BasicData::<UiAction, String>::load(disk(&files), CONSOLE, STAGE)
        .unwrap_or_else(|e| panic!("{e:?}"));
    // What the standard files read is not its.
    assert!(
        data.reload(&changed(&[ENGLISH_RON]))[..]
            .iter()
            .all(|r| r.0 != OWN_PART)
    );
    assert!(data.bundle.replace("stage.map", b"second".to_vec()));
    let reloads = data.reload(&changed(&["stage.map"]));
    assert_eq!(outcomes(&reloads), [(OWN_PART, "reloaded")]);
    assert_eq!(data.own, "second");
    assert!(data.bundle.replace("stage.map", b"broken".to_vec()));
    let errors = failure(&data.reload(&changed(&["stage.map"])));
    assert_eq!(errors, ["disk/stage.map:1: broken"]);
    assert_eq!(data.own, "second");
}

/// The game's data with a stage of its own ([`STAGE`]).
type Staged = BasicData<UiAction, String>;

/// A screen that does nothing but say its name: `"old"` or `"first"`.
struct Named(&'static str);

impl Screen<Staged> for Named {
    fn name(&self) -> &'static str {
        self.0
    }

    fn update(
        &mut self,
        _: &mut Ctx<Staged>,
        _: &crate::FrameInput<UiAction>,
    ) -> crate::Transition<Staged> {
        crate::Transition::None
    }

    fn draw(&self, _: &Ctx<Staged>, _: &mut crate::frame::glyph_buffer::GlyphBuffer) {}
}

/// The screen a restart starts over at.
fn first(_: &mut Ctx<Staged>) -> Box<dyn Screen<Staged>> {
    Box::new(Named("first"))
}

/// A game on the blank template's files and a stage, on disk, in the
/// keymap's first layout, at an "old" screen, with [`BasicReload`].
fn reloading() -> crate::Game<Staged> {
    let mut files = GOOD.to_vec();
    files.push(("stage.map", b"first"));
    let data = Staged::load(disk(&files), CONSOLE, STAGE).unwrap_or_else(|e| panic!("{e:?}"));
    let mut ctx = Ctx::new(data).unwrap_or_else(|e| panic!("{e:?}"));
    ctx.use_layout(Layout::named("Arrows"));
    let game = crate::Game::new(ctx, Box::new(Named("old")), crate::GameHooks::default());
    game.with_module(BasicReload::new(first))
        .unwrap_or_else(|e| panic!("{e}"))
}

/// Saves `bytes` at `path` in `game`'s files and tells its modules.
fn save(game: &mut crate::Game<Staged>, path: &str, bytes: Vec<u8>) {
    assert!(game.ctx().game.bundle.replace(path, bytes));
    game.reloaded(&changed(&[path]));
}

#[test]
fn the_reload_module_hears_of_changed_files_by_its_name() {
    let game = reloading();
    let module = &game.modules()[0];
    assert_eq!(module.name, "basic_reload");
    assert_eq!(module.hooks, [Hook::Reloaded]);
    assert_eq!(
        format!("{:?}", BasicReload::new(first)),
        "BasicReload { .. }"
    );
}

#[test]
fn a_new_english_shows_on_the_same_screens() {
    let mut game = reloading();
    save(
        &mut game,
        ENGLISH_RON,
        edited(EN, "made with Shuyi Engine", "made anew"),
    );
    assert_eq!(game.ctx().text("title.subtitle"), "made anew");
    assert_eq!(game.screens(), ["old"]);
}

#[test]
fn a_new_palette_is_the_contexts_and_a_broken_one_is_noted() {
    let mut game = reloading();
    let palette = edited(PALETTE, "\"text\": \"#e4e4dc\"", "\"text\": \"#102030\"");
    save(&mut game, PALETTE_RON, palette);
    let text = game.ctx().palette.engine(EngineColor::Text);
    assert_eq!(text, crate::frame::color::Rgb::new(0x10, 0x20, 0x30));
    assert_eq!(game.screens(), ["old"]);
    save(
        &mut game,
        PALETTE_RON,
        edited(PALETTE, "\"focus\": \"#5ac8ff\",", ""),
    );
    assert_eq!(game.ctx().palette.engine(EngineColor::Text), text);
    let errors = game.ctx().reload_errors();
    assert!(
        errors.iter().any(|e| e.contains("palette.ron")),
        "{errors:?}"
    );
    save(&mut game, PALETTE_RON, PALETTE.to_vec());
    assert_eq!(game.ctx().reload_errors(), [] as [String; 0]);
}

#[test]
fn a_new_keymap_is_the_layouts_keys_on_the_same_screens() {
    let mut game = reloading();
    let arrows = Layout::named("Arrows");
    let before = game.ctx().keymap.clone();
    save(
        &mut game,
        KEYMAP_RON,
        edited(
            KEYMAP,
            "\"Cancel\": [\"Backspace\", \"x\"]",
            "\"Cancel\": [\"q\"]",
        ),
    );
    assert_ne!(game.ctx().keymap, before);
    assert_eq!(game.ctx().keymap, game.ctx().keymap_for(arrows));
    assert_eq!(game.screens(), ["old"]);
}

#[test]
fn a_new_own_part_starts_the_game_over_at_its_first_screen() {
    let mut game = reloading();
    save(&mut game, "stage.map", b"second".to_vec());
    assert_eq!(game.ctx().game.own, "second");
    assert_eq!(game.screens(), ["first"]);
    // A broken one keeps the screens and the old part.
    save(&mut game, "stage.map", b"broken".to_vec());
    assert_eq!(game.screens(), ["first"]);
    assert_eq!(game.ctx().reload_errors(), ["disk/stage.map:1: broken"]);
}
