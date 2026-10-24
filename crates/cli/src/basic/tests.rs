//! `BasicTools` on the blank template's own files: the frame, the text,
//! `check` on good and on broken files, a session and the controls.

use shuyi_assets::bundle::Layer;
use shuyi_assets::keymap::UiAction;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::{Ctx, FrameInput, Screen, Transition};

use super::*;
use crate::frame_png::Step;

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

/// Its files with a palette that isn't RON.
static BROKEN: &[(&str, &[u8])] = &[
    ("en.ron", EN),
    ("font/atlas.png", ATLAS_PNG),
    ("font/atlas.ron", ATLAS_RON),
    ("keymap.ron", KEYMAP),
    ("palette.ron", b"{ nope"),
];

type Data = BasicData<UiAction>;

/// A screen that prints its text's subtitle and quits on Cancel.
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
        let bg = ctx.palette.engine(shuyi_ui::frame::EngineColor::Background);
        buf.print(0, 0, ctx.text("title.subtitle"), text, bg);
    }
}

fn tools(files: &'static [(&'static str, &'static [u8])]) -> BasicTools<UiAction> {
    let files = Layer::table("files", files);
    BasicTools::new(BasicGame::new("Demo", "demo", files, |_| Box::new(Hello)))
}

#[test]
fn the_frame_is_the_first_screen_painted_with_the_games_font() {
    let shot = tools(GOOD)
        .frame(&[], &Script::default(), Some(2))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(shot.shows, "hello");
    assert_eq!(
        (shot.image.width, shot.image.height),
        (48 * 8 * 2, 24 * 16 * 2)
    );
    assert!(shot.warning.is_none());
    let option = GameArg {
        flag: "--light".to_owned(),
        value: None,
    };
    assert_eq!(
        tools(GOOD).frame(&[option], &Script::default(), None).err(),
        Some("unknown option --light".to_owned())
    );
}

#[test]
fn the_text_and_check_come_from_the_files() {
    let lang = tools(GOOD).lang().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        lang.lang.english("title.subtitle"),
        Some("made with Shuyi Engine")
    );
    assert!(lang.variants.is_empty(), "{:?}", lang.variants);
    assert_eq!(tools(GOOD).check(), Ok(Vec::new()));
    let errors = tools(BROKEN).check().unwrap_or_default();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert_eq!(errors[0].file, "files/palette.ron");
    assert!(
        tools(BROKEN)
            .lang()
            .is_err_and(|e| e.starts_with("files/palette.ron"))
    );
    assert!(tools(BROKEN).frame(&[], &Script::default(), None).is_err());
}

#[test]
fn a_session_plays_the_game_and_paints_it() {
    let mut session = tools(GOOD).session(&[]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(session.screens(), ["hello"]);
    assert!(session.picture(Some(1)).is_ok());
    assert!(session.scene().is_err());
    session.play(&Script {
        steps: vec![Step::Keys("Backspace".to_owned())],
    });
    assert_eq!(session.quit_frame(), Some(1));
}

#[test]
fn the_controls_are_the_keymap_and_the_first_screen() {
    let profiles = crate::contract::target::engine_profiles().unwrap_or_else(|e| panic!("{e:?}"));
    let controls = tools(GOOD)
        .controls()
        .unwrap_or_else(|| panic!("no controls"))
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(controls.fit(&profiles["desktop"].profile), []);
    // A Playdate table without Cancel: the first screen's Cancel doesn't
    // fit it.
    let keymap = std::str::from_utf8(KEYMAP).unwrap_or_default().replace(
        "targets: {}",
        r#"targets: { "playdate": {
                "Up": ["DpadUp"], "Down": ["DpadDown"], "Left": ["DpadLeft"],
                "Right": ["DpadRight"], "Confirm": ["East"], "Cancel": [],
            } }"#,
    );
    let keymap: &'static [u8] = Box::leak(keymap.into_bytes().into_boxed_slice());
    let files: &'static [(&str, &[u8])] = Box::leak(Box::new([
        ("en.ron", EN),
        ("font/atlas.png", ATLAS_PNG),
        ("font/atlas.ron", ATLAS_RON),
        ("keymap.ron", keymap),
        ("palette.ron", PALETTE),
    ]));
    let problems = tools(files)
        .controls()
        .unwrap_or_else(|| panic!("no controls"))
        .unwrap_or_else(|e| panic!("{e}"))
        .fit(&profiles["playdate"].profile);
    assert!(
        problems
            .iter()
            .any(|p| p.screen == Some("hello") && p.actions == ["Cancel"]),
        "{problems:?}"
    );
    assert!(tools(BROKEN).controls().is_some_and(|c| c.is_err()));
}
