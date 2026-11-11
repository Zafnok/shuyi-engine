//! `BasicGame` on the blank template's own files: its context, its
//! screens, its start in an `App` on the headless backend, the Harness,
//! and every problem of broken files.

use shuyi_assets::keymap::{Layout, UiAction};
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::{FrameInput, Transition};

use alloc::collections::BTreeMap;

use shuyi_assets::dialogue::{DialogueLimits, NoDirective, NoTokens, SceneRules, Tokens};
use shuyi_ui::dialogue::preview::{Play, PreviewScreen, PreviewView, Showing};
use shuyi_ui::dialogue::{DialoguePlayer, Plain};

use super::*;
use crate::app::App;
use crate::harness::{FRAME_DT, Harness};
use crate::headless::{HeadlessPlatform, Presented};
use crate::platform::Millis;

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

/// The blank template's files.
static GOOD: &[(&str, &[u8])] = &[
    ("en.ron", EN),
    ("font/atlas.png", ATLAS_PNG),
    ("font/atlas.ron", ATLAS_RON),
    ("keymap.ron", KEYMAP),
    ("palette.ron", PALETTE),
];

/// Its files, with the dialogue preview's words in its language file.
static WITH_PREVIEW: &[(&str, &[u8])] = &[
    ("en.ron", include_bytes!("preview_en.ron")),
    ("font/atlas.png", ATLAS_PNG),
    ("font/atlas.ron", ATLAS_RON),
    ("keymap.ron", KEYMAP),
    ("palette.ron", PALETTE),
];

/// Its files with the keymap and the palette missing.
static BROKEN: &[(&str, &[u8])] = &[
    ("en.ron", EN),
    ("font/atlas.png", ATLAS_PNG),
    ("font/atlas.ron", ATLAS_RON),
];

type Data = BasicData<UiAction>;

/// A screen that draws `H` and quits on Cancel.
struct Hello;

impl Screen<Data> for Hello {
    fn name(&self) -> &'static str {
        "hello"
    }

    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Cancel])
    }

    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if input.actions.contains(&UiAction::Cancel) {
            Transition::Quit
        } else {
            Transition::None
        }
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let text = ctx.palette.engine(shuyi_ui::frame::EngineColor::Text);
        buf.print(0, 0, "H", text, text);
    }
}

/// A second screen, for `screens`.
struct Other;

impl Screen<Data> for Other {
    fn name(&self) -> &'static str {
        "other"
    }

    fn update(&mut self, _: &mut Ctx<Data>, _: &FrameInput<UiAction>) -> Transition<Data> {
        Transition::None
    }

    fn draw(&self, _: &Ctx<Data>, _: &mut GlyphBuffer) {}
}

/// A module that puts a number on the context.
struct Marker;

impl Module<Data> for Marker {
    fn name(&self) -> &'static str {
        "marker"
    }

    fn install(&mut self, install: &mut shuyi_ui::module::Install<'_, Data>) {
        install.state(7_u8);
    }
}

fn one_marker() -> Vec<Box<dyn Module<Data>>> {
    vec![Box::new(Marker)]
}

fn two_markers() -> Vec<Box<dyn Module<Data>>> {
    vec![Box::new(Marker), Box::new(Marker)]
}

/// Who the demo is.
const DEMO: AppInfo = AppInfo::new("Demo", "com.example.demo", "0.1.0").with_storage("demo");

fn demo(files: &'static [(&'static str, &'static [u8])]) -> BasicGame<UiAction> {
    BasicGame::new(DEMO, Layer::table("files", files), |_| Box::new(Hello))
}

#[test]
fn it_starts_on_its_first_screen_in_the_keymaps_first_layout() {
    let game = demo(GOOD);
    assert_eq!((game.name(), game.storage_name()), ("Demo", "demo"));
    assert_eq!(game.app(), DEMO);
    let own_storage = BasicGame::<UiAction>::new(
        AppInfo::new("A", "com.example.a", "1.0.0"),
        game.files,
        |_| Box::new(Hello),
    );
    assert_eq!(own_storage.storage_name(), "com.example.a");
    assert_eq!(game.console_spec(), BASIC_CONSOLE);
    assert!(game.bundle().has("keymap.ron"));
    let ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(ctx.layout(), Some(Layout::named("Arrows")));
    assert_eq!(ctx.console, BASIC_CONSOLE);
    let started = game.game(ctx).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(started.screens(), ["hello"]);
    assert_eq!(started.modules(), &[]);
    let small = ConsoleSpec::text(20, 10);
    assert_eq!(game.console(small).ctx().map(|c| c.console), Ok(small));
}

#[test]
fn every_screen_is_the_first_alone_until_said() {
    let game = demo(GOOD);
    let mut ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    let names = |screens: Vec<Box<dyn Screen<Data>>>| -> Vec<&'static str> {
        screens.iter().map(|s| s.name()).collect()
    };
    assert_eq!(names(game.every_screen(&mut ctx)), ["hello"]);
    let game = game.screens(|_| vec![Box::new(Hello), Box::new(Other)]);
    assert_eq!(names(game.every_screen(&mut ctx)), ["hello", "other"]);
}

#[test]
fn its_modules_are_installed_in_its_game_and_its_harness() {
    let game = demo(GOOD).modules(one_marker);
    let ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    let started = game.game(ctx).unwrap_or_else(|e| panic!("{e}"));
    let names: Vec<&str> = started.modules().iter().map(|m| m.name).collect();
    assert_eq!(names, ["marker"]);
    assert_eq!(started.ctx().module_state::<u8>(), Some(&7));
    assert_eq!(game.harness().ctx().module_state::<u8>(), Some(&7));
}

#[test]
fn a_module_that_cant_be_installed_is_the_games_error_and_the_loads() {
    let game = demo(GOOD).modules(two_markers);
    let ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    let error = game.game(ctx).err();
    assert_eq!(error, Some(ModuleError::SameName { name: "marker" }));
    let mut app = App::new(game.start());
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert!(app.game().is_none());
    assert_eq!(app.errors(), Some("the module marker is installed twice"));
}

#[test]
fn an_app_shows_its_card_then_its_first_screen() {
    let mut app = App::new(demo(GOOD).start());
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    assert_eq!(p.last_frame(), Some(&Presented::Card(Card::text("Demo"))));
    let _ = app.frame(&mut p, Millis(16));
    let game = app.game().unwrap_or_else(|| panic!("{:?}", app.errors()));
    assert_eq!(game.screens(), ["hello"]);
    let shown = p.last_frame().and_then(Presented::buffer);
    assert_eq!(shown.and_then(|b| b.get(0, 0)).map(|c| c.glyph), Some('H'));
}

#[test]
fn its_crash_record_reaches_the_app() {
    let mut app = App::new(demo(GOOD).start());
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert_eq!(app.crash_record(), None, "none until said");
    let game = demo(GOOD).crash_record(|game| game.top_screen().map(|name| format!("on {name}")));
    let mut app = App::new(game.start());
    let _ = app.frame(&mut p, Millis(32));
    assert_eq!(app.crash_record(), None, "loading");
    let _ = app.frame(&mut p, Millis(48));
    assert_eq!(app.crash_record().as_deref(), Some("on hello"));
}

#[test]
fn its_record_is_in_a_release_builds_report_only_when_said() {
    assert!(!demo(GOOD).records_play_in_release(), "off until said");
    assert!(
        demo(GOOD)
            .crash_record_in_release()
            .records_play_in_release()
    );
}

#[test]
fn broken_files_give_every_problem_to_ctx_and_to_the_load() {
    let game = demo(BROKEN);
    let errors = game.ctx().err().unwrap_or_default();
    let lines: Vec<&str> = errors.lines().collect();
    assert!(
        lines.iter().any(|l| l.starts_with("files/keymap.ron")),
        "{errors}"
    );
    assert!(
        lines.iter().any(|l| l.starts_with("files/palette.ron")),
        "{errors}"
    );
    let mut app = App::new(game.start());
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert!(app.game().is_none());
    assert_eq!(app.errors(), Some(errors.as_str()));
}

#[test]
fn the_harness_plays_a_key() {
    let mut h = demo(GOOD).harness();
    assert_eq!(h.top_screen(), "hello");
    h.keys("Backspace");
    assert!(h.quit_requested());
}

#[test]
#[should_panic(expected = "files/keymap.ron")]
fn the_harness_of_broken_files_panics_with_their_problems() {
    let _ = demo(BROKEN).harness();
}

#[test]
fn a_game_is_copied_and_shown_by_its_name() {
    let game = demo(GOOD);
    let copy = game;
    assert_eq!(game.name(), copy.clone().name());
    let shown = format!("{game:?}");
    assert!(
        shown.starts_with("BasicGame { app: AppInfo { name: \"Demo\""),
        "{shown}"
    );
}

/// The demo's dialogue screen: Confirm reads on, and it pops after its
/// last line.
struct Talk(DialoguePlayer<Plain>);

impl Screen<Data> for Talk {
    fn name(&self) -> &'static str {
        "talk"
    }

    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if input.actions.contains(&UiAction::Confirm) {
            self.0.advance();
            if self.0.is_finished() {
                return Transition::Pop;
            }
        }
        Transition::None
    }

    fn draw(&self, _: &Ctx<Data>, _: &mut GlyphBuffer) {}
}

impl PreviewScreen<Data> for Talk {
    fn showing(&self) -> Showing {
        Showing::of(&self.0)
    }
}

/// What the demo hands its preview: its limits, and its talk.
struct DemoPreview;

impl PreviewGame<Data> for DemoPreview {
    type Directive = NoDirective;
    type Rules = DemoRules;

    fn rules(&self, _: &Ctx<Data>) -> DemoRules {
        DemoRules(LIMITS)
    }

    fn play(
        &self,
        _: &mut Ctx<Data>,
        play: &Play<'_, NoDirective>,
    ) -> Box<dyn PreviewScreen<Data>> {
        Box::new(Talk(play.player(Plain)))
    }
}

/// The engine's usual limits, in a box of 40 × 3.
const LIMITS: DialogueLimits = DialogueLimits {
    box_width: 40,
    box_lines: 3,
    max_text: 120,
    max_option: 30,
    min_options: 2,
    max_options: 4,
    max_reaction_texts: 3,
};

/// The rules the demo's scenes are checked against.
struct DemoRules(DialogueLimits);

impl SceneRules<NoDirective> for DemoRules {
    fn limits(&self) -> &DialogueLimits {
        &self.0
    }

    fn tokens(&self) -> &dyn Tokens {
        &NoTokens
    }
}

/// A folder of one scene of one line, `words`.
fn scenes(words: &str) -> Layer {
    let files = BTreeMap::from([("dusk.dlg".to_owned(), scene(words))]);
    Layer::owned("scenes".to_owned(), files).unwrap_or_else(|e| panic!("{e:?}"))
}

fn scene(words: &str) -> Vec<u8> {
    format!("@scene dusk\n> {words}\n@end\n").into_bytes()
}

type Preview = DialoguePreview<Data, DemoPreview>;

fn preview_view(h: &Harness<Data>) -> PreviewView {
    let preview = h
        .screen::<Preview>()
        .unwrap_or_else(|| panic!("no preview"));
    preview.view(h.ctx())
}

#[test]
fn its_preview_plays_a_scene_of_the_folder_and_its_saved_file_again() {
    let folder = scenes("The moon is up.");
    let mut h = demo(WITH_PREVIEW)
        .modules(one_marker)
        .preview_harness(folder, DemoPreview);
    assert_eq!(h.ctx().module_state::<u8>(), Some(&7), "its modules");
    h.wait(FRAME_DT);
    let view = preview_view(&h);
    assert_eq!(view.scenes.len(), 1);
    assert_eq!(view.scenes[0].id, "dusk");
    // Play it.
    h.keys("Enter");
    let view = preview_view(&h);
    let playing = view.playing.unwrap_or_else(|| panic!("not playing"));
    assert_eq!((playing.scene.as_str(), playing.finished), ("dusk", false));
    assert_eq!(view.lines[0].words, "The moon is up.");
    // The writer saves the file: it is read again.
    h.watch_files(true);
    h.change_file(folder, "dusk.dlg", scene("The garden sleeps."));
    h.wait(FRAME_DT);
    assert_eq!(preview_view(&h).lines[0].words, "The garden sleeps.");
    // Read on past its one line: the scene is over.
    h.keys("Enter");
    assert!(preview_view(&h).playing.is_some_and(|p| p.finished));
}

#[test]
fn its_preview_starts_after_its_card_with_no_debug_screen() {
    let mut app = App::new(demo(WITH_PREVIEW).preview_start(Ok(scenes("Hi.")), DemoPreview));
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    assert_eq!(p.last_frame(), Some(&Presented::Card(Card::text("Demo"))));
    let _ = app.frame(&mut p, Millis(16));
    let game = app.game().unwrap_or_else(|| panic!("{:?}", app.errors()));
    assert_eq!(game.top_screen(), Some("dialogue_preview"));
    assert!(!game.ctx().debug_tools);
}

#[test]
fn a_folder_that_couldnt_be_read_is_the_previews_error() {
    let unread = Err("scenes: no such folder".to_owned());
    let mut app = App::new(demo(WITH_PREVIEW).preview_start(unread, DemoPreview));
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert!(app.game().is_none());
    assert_eq!(app.errors(), Some("scenes: no such folder"));
}

#[test]
fn a_preview_whose_module_cant_be_installed_is_the_loads_error() {
    let game = demo(WITH_PREVIEW).modules(two_markers);
    let mut app = App::new(game.preview_start(Ok(scenes("Hi.")), DemoPreview));
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    assert_eq!(app.errors(), Some("the module marker is installed twice"));
}

/// The fixture `name` of this test's dropped files, by its whole path:
/// wasm32's tests run in another folder.
#[cfg(feature = "std")]
fn dropped(name: &str) -> String {
    format!("{}/src/basic/drop/{name}", env!("CARGO_MANIFEST_DIR"))
}

#[test]
#[cfg(feature = "std")]
fn a_scene_dropped_on_the_preview_joins_its_folder_and_plays_again_dropped_changed() {
    use crate::script::{Script, Step};
    use shuyi_ui::FileDrops;
    let empty =
        Layer::owned("scenes".to_owned(), BTreeMap::new()).unwrap_or_else(|e| panic!("{e:?}"));
    let mut h = demo(WITH_PREVIEW).preview_harness(empty, DemoPreview);
    // Nothing is listed: where files can't be dropped, the note says to
    // save one, and where they can, to drop one.
    let note = |h: &Harness<Data>| preview_view(h).note;
    assert_eq!(
        note(&h).as_deref(),
        Some("No scenes: save a .dlg file in the folder")
    );
    let facts = h.ctx().platform.clone().with_file_drops(FileDrops::Heard);
    h.ctx_mut().platform = facts;
    assert_eq!(
        note(&h).as_deref(),
        Some("No scenes: drop a .dlg file here")
    );
    // A file of another kind is left out.
    h.drop_file("notes.txt", b"not a scene".to_vec());
    assert_eq!(preview_view(&h).scenes, []);

    let drop = |name: &str| Step::Drop(dropped(name));
    Script::new(vec![drop("first/dusk.dlg")]).play(&mut h);
    let view = preview_view(&h);
    assert_eq!(view.scenes.len(), 1);
    assert_eq!(
        (view.scenes[0].file.as_str(), view.scenes[0].id.as_str()),
        ("dusk.dlg", "dusk")
    );
    assert_eq!(view.note, None);
    h.keys("Enter");
    assert_eq!(preview_view(&h).lines[0].words, "The moon is up.");
    // The writer changes it and drops it again: it plays again.
    Script::new(vec![drop("changed/dusk.dlg")]).play(&mut h);
    h.wait(FRAME_DT);
    let view = preview_view(&h);
    assert_eq!(view.scenes.len(), 1);
    assert_eq!(view.lines[0].words, "The garden sleeps.");
    assert!(view.playing.is_some_and(|p| !p.finished));
}

#[test]
fn a_game_whose_text_lacks_the_drop_note_shows_the_folders() {
    let empty =
        Layer::owned("scenes".to_owned(), BTreeMap::new()).unwrap_or_else(|e| panic!("{e:?}"));
    // The preview's words as a game had them before the drop's note.
    let words = String::from_utf8_lossy(include_bytes!("preview_en.ron"));
    let words = words
        .lines()
        .filter(|line| !line.contains("no_scenes_drop"))
        .collect::<Vec<_>>()
        .join("\n");
    let files = BTreeMap::from([("en.ron".to_owned(), words.into_bytes())]);
    let older = Layer::owned("older".to_owned(), files).unwrap_or_else(|e| panic!("{e:?}"));
    let mut h = demo(WITH_PREVIEW)
        .over(older)
        .preview_harness(empty, DemoPreview);
    let facts = h
        .ctx()
        .platform
        .clone()
        .with_file_drops(shuyi_ui::FileDrops::Heard);
    h.ctx_mut().platform = facts;
    let note = preview_view(&h).note;
    assert_eq!(
        note.as_deref(),
        Some("No scenes: save a .dlg file in the folder")
    );
}

#[test]
#[cfg(feature = "std")]
fn the_previews_start_hears_a_drop() {
    use crate::dropped::DroppedFile;
    let empty = Layer::owned("scenes".to_owned(), BTreeMap::new());
    let mut app =
        App::new(demo(WITH_PREVIEW).preview_start(empty.map_err(|e| e.to_string()), DemoPreview));
    let mut p = HeadlessPlatform::new();
    let _ = app.frame(&mut p, Millis(0));
    let _ = app.frame(&mut p, Millis(16));
    let dusk = DroppedFile::new(r"C:\writer\dusk.dlg", scene("Hi."));
    p.push_system(crate::SystemEvent::FilesDropped(vec![dusk]));
    let _ = app.frame(&mut p, Millis(32));
    let game = app.game().unwrap_or_else(|| panic!("{:?}", app.errors()));
    let files = game
        .ctx()
        .module_state::<shuyi_ui::dialogue::preview::DialogueFiles>();
    let names: Vec<_> = files
        .map(|f| f.files().map(|(path, _)| path).collect())
        .unwrap_or_default();
    assert_eq!(names, ["dusk.dlg"]);
}
