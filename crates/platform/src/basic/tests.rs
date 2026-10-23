//! `BasicGame` on the blank template's own files: its context, its
//! screens, its start in an `App` on the headless backend, the Harness,
//! and every problem of broken files.

use shuyi_assets::keymap::{Layout, UiAction};
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::{FrameInput, Transition};

use super::*;
use crate::app::App;
use crate::headless::{HeadlessPlatform, Presented};
use crate::platform::Millis;

const EN: &[u8] = include_bytes!("../../../../templates/blank/crates/game/files/en.ron");
const KEYMAP: &[u8] = include_bytes!("../../../../templates/blank/crates/game/files/keymap.ron");
const PALETTE: &[u8] = include_bytes!("../../../../templates/blank/crates/game/files/palette.ron");
const ATLAS_RON: &[u8] =
    include_bytes!("../../../../templates/blank/crates/game/files/font/atlas.ron");
const ATLAS_PNG: &[u8] =
    include_bytes!("../../../../templates/blank/crates/game/files/font/atlas.png");

/// The blank template's files.
static GOOD: &[(&str, &[u8])] = &[
    ("en.ron", EN),
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

fn demo(files: &'static [(&'static str, &'static [u8])]) -> BasicGame<UiAction> {
    BasicGame::new("Demo", "demo", Layer::table("files", files), |_| {
        Box::new(Hello)
    })
}

#[test]
fn it_starts_on_its_first_screen_in_the_keymaps_first_layout() {
    let game = demo(GOOD);
    assert_eq!((game.name(), game.storage_name()), ("Demo", "demo"));
    assert_eq!(game.console_spec(), BASIC_CONSOLE);
    assert!(game.bundle().has("keymap.ron"));
    let ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(ctx.layout(), Some(Layout::named("Arrows")));
    assert_eq!(ctx.console, BASIC_CONSOLE);
    assert_eq!(game.game(ctx).screens(), ["hello"]);
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
    assert!(shown.starts_with("BasicGame { name: \"Demo\""), "{shown}");
}
