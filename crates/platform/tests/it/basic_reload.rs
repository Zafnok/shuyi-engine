//! A game on the short path reloading its own files as they change on
//! disk, with no code of its own (`BasicReload`): its text, colours and
//! keys on the next frame, and a new own part starting it over at its first
//! screen; a broken file keeps the old value and says so, and a file
//! nothing read changes nothing.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use shuyi_assets::bundle::Layer;
use shuyi_assets::keymap::{Button, Layout, UiAction};
use shuyi_core::StateHash;
use shuyi_platform::harness::{FRAME_DT, Harness};
use shuyi_platform::{AppInfo, BasicGame};
use shuyi_ui::Ctx;
use shuyi_ui::basic::{BasicData, Own};
use shuyi_ui::frame::EngineColor;
use shuyi_ui::frame::color::Rgb;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::screen::{FrameInput, Screen, Transition};

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
/// The top-down grid stage's map, which its own part reads.
const MAP: &[u8] = include_bytes!(
    "../../../../templates/shuyi-default/stages/top-down-grid/crates/game/files/stage.map"
);

/// The blank template's files, with the stage's map.
static FILES: &[(&str, &[u8])] = &[
    ("en.ron", EN),
    ("font/atlas.png", ATLAS_PNG),
    ("font/atlas.ron", ATLAS_RON),
    ("keymap.ron", KEYMAP),
    ("palette.ron", PALETTE),
    ("stage.map", MAP),
];

/// The stage's map as its own part reads it: its lines after the
/// header, which must hold a wall.
type Stage = Vec<String>;
type Data = BasicData<UiAction, Stage>;

/// The stage's part: `stage.map`'s rows, or the error of a map with no
/// wall, at its first line.
const OWN: Own<Stage> = |bundle| {
    let text = bundle.file("stage.map").unwrap_or_default();
    let rows: Stage = text
        .split_once("---")
        .map(|(_, rows)| rows.lines().filter(|r| !r.is_empty()).map(str::to_owned))
        .into_iter()
        .flatten()
        .collect();
    if rows.iter().any(|row| row.contains('#')) {
        Ok(rows)
    } else {
        let file = bundle.display_path("stage.map");
        Err(vec![
            shuyi_assets::ContentError::new(file, "no wall").at(1, None),
        ])
    }
};

/// The first screen: it counts the Confirms and Cancels it hears, and
/// draws the subtitle and the map's first row.
#[derive(Debug, Default)]
struct Counter {
    confirms: u32,
    cancels: u32,
}

impl Screen<Data> for Counter {
    fn name(&self) -> &'static str {
        "counter"
    }

    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm, UiAction::Cancel])
    }

    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        for action in &input.actions {
            match action {
                UiAction::Confirm => self.confirms += 1,
                UiAction::Cancel => self.cancels += 1,
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let text = ctx.palette.engine(EngineColor::Text);
        let back = ctx.palette.engine(EngineColor::Background);
        buf.print(0, 0, ctx.text("title.subtitle"), text, back);
        let row = ctx.game.own.first().map_or("", String::as_str);
        buf.print(0, 1, row, text, back);
    }

    fn state_hash(&self) -> Option<StateHash> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        (self.confirms, self.cancels).hash(&mut hasher);
        Some(StateHash(hasher.finish()))
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

/// The game, its files on disk in `disk` over the embedded ones.
fn game(disk: Layer) -> BasicGame<UiAction, Stage> {
    let app = AppInfo::new("Reloads", "com.example.reloads", "0.1.0");
    BasicGame::with_own(app, Layer::table("files", FILES), OWN, |_| {
        Box::new(Counter::default())
    })
    .over(disk)
}

/// A folder on disk with nothing in it yet.
fn disk() -> Layer {
    Layer::owned("disk".to_owned(), BTreeMap::new()).unwrap_or_else(|e| panic!("{e:?}"))
}

/// `text` with `from` replaced by `to`: a saved edit of a file.
fn edited(text: &[u8], from: &str, to: &str) -> Vec<u8> {
    let text = std::str::from_utf8(text).unwrap_or_else(|e| panic!("{e}"));
    assert!(text.contains(from), "{from:?}");
    text.replacen(from, to, 1).into_bytes()
}

/// How many Confirms and Cancels the counter heard.
fn heard(h: &Harness<Data>) -> (u32, u32) {
    let counter = h
        .screen::<Counter>()
        .unwrap_or_else(|| panic!("no counter"));
    (counter.confirms, counter.cancels)
}

#[test]
fn a_short_path_game_with_debug_tools_reloads_first_and_without_them_not() {
    let game = game(disk());
    let h = game.harness();
    let names: Vec<&str> = h.game().modules().iter().map(|m| m.name).collect();
    assert_eq!(names, ["basic_reload"]);
    let mut ctx = game.ctx().unwrap_or_else(|e| panic!("{e}"));
    ctx.debug_tools = false;
    let shipped = game.game(ctx).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(shipped.modules(), []);
}

#[test]
fn a_changed_line_of_text_shows_in_the_next_frame() {
    let disk = disk();
    let mut h = game(disk).harness();
    h.wait(FRAME_DT);
    assert!(h.snapshot().contains("made with Shuyi Engine"));
    h.change_file(
        disk,
        "en.ron",
        edited(EN, "made with Shuyi Engine", "made anew"),
    );
    h.wait(FRAME_DT);
    let snapshot = h.snapshot();
    assert!(snapshot.contains("made anew"), "{snapshot}");
    assert_eq!(h.ctx().reload_errors(), [] as [String; 0]);
}

#[test]
fn a_changed_colour_is_the_palettes_on_the_next_frame() {
    let disk = disk();
    let mut h = game(disk).harness();
    let new = edited(PALETTE, "\"text\": \"#e4e4dc\"", "\"text\": \"#102030\"");
    h.change_file(disk, "palette.ron", new);
    h.wait(FRAME_DT);
    assert_eq!(
        h.ctx().palette.engine(EngineColor::Text),
        Rgb::new(0x10, 0x20, 0x30)
    );
}

#[test]
fn a_changed_binding_works_on_the_next_frame_and_the_players_own_stay() {
    let disk = disk();
    let mut h = game(disk).harness();
    // The player moves Confirm to the pad's North button: theirs in every
    // layout, the keys the file's.
    let mut buttons = h.ctx().pad_bindings();
    buttons
        .bind(UiAction::Confirm, 0, Button::North)
        .unwrap_or_else(|e| panic!("{e:?}"));
    h.ctx_mut()
        .set_pad_bindings(buttons)
        .unwrap_or_else(|e| panic!("{e}"));
    h.keys("q");
    assert_eq!(heard(&h), (0, 0), "q is nothing yet");
    // The file gives Cancel `q`, and Confirm the pad's West button.
    let keymap = edited(
        KEYMAP,
        "\"Cancel\": [\"Backspace\", \"x\"]",
        "\"Cancel\": [\"Backspace\", \"q\"]",
    );
    let keymap = edited(
        &keymap,
        "\"Confirm\": [\"South\"]",
        "\"Confirm\": [\"West\"]",
    );
    h.change_file(disk, "keymap.ron", keymap);
    h.keys("q");
    assert_eq!(heard(&h), (0, 1), "the file's new key");
    h.pad("North");
    assert_eq!(heard(&h), (1, 1), "the player's own button stayed");
    h.pad("West");
    assert_eq!(heard(&h), (1, 1), "over the file's");
    assert_eq!(h.ctx().layout(), Some(Layout::named("Arrows")));
}

#[test]
fn a_changed_map_starts_the_game_over_at_its_first_screen_on_the_same_context() {
    let disk = disk();
    let mut h = game(disk).harness();
    h.ctx_mut()
        .change_settings(|s| s.fullscreen = true)
        .unwrap_or_else(|e| panic!("{e}"));
    h.keys("Enter Enter");
    assert_eq!(heard(&h), (2, 0));
    let map = edited(
        MAP,
        "############\n#.........>#",
        "############\n#>.........#",
    );
    h.change_file(disk, "stage.map", map);
    h.wait(FRAME_DT);
    assert_eq!(h.screens(), ["counter"]);
    assert_eq!(heard(&h), (0, 0), "a new first screen");
    assert_eq!(h.ctx().game.own[1], "#>.........#");
    assert!(h.ctx().settings().fullscreen, "the settings stay");
    assert_eq!(h.ctx().layout(), Some(Layout::named("Arrows")));
}

#[test]
fn a_changed_file_nothing_read_changes_nothing() {
    let disk = disk();
    let mut h = game(disk).harness();
    h.keys("Enter");
    let hash = h.game().state_hash();
    assert!(hash.is_some());
    h.change_file(disk, "notes.txt", b"later".to_vec());
    h.wait(FRAME_DT);
    assert_eq!(h.game().state_hash(), hash);
    assert_eq!(heard(&h), (1, 0));
}

#[test]
fn a_broken_file_keeps_the_old_value_and_the_screen_until_a_good_save() {
    let disk = disk();
    let mut h = game(disk).harness();
    h.keys("Enter");
    let palette = h.ctx().palette.engine(EngineColor::Focus);
    let broken = [
        (
            "en.ron",
            edited(EN, "\n", "\n\"title.name\" \"no colon\",\n"),
        ),
        (
            "palette.ron",
            edited(PALETTE, "\"focus\": \"#5ac8ff\",", ""),
        ),
        ("keymap.ron", edited(KEYMAP, "\"Cancel\": [", "\"Jump\": [")),
        ("stage.map", b"(no map)\n---\n....\n".to_vec()),
    ];
    for (file, bytes) in broken {
        h.change_file(disk, file, bytes);
        h.wait(FRAME_DT);
        let errors = h.ctx().reload_errors().join("\n");
        assert!(
            errors.contains(&format!("disk/{file}:")),
            "{file}: {errors}"
        );
    }
    // The old values, on the same screen.
    assert_eq!(heard(&h), (1, 0));
    assert!(h.snapshot().contains("made with Shuyi Engine"));
    assert_eq!(h.ctx().palette.engine(EngineColor::Focus), palette);
    assert_eq!(h.ctx().game.own[0], "############");
    h.keys("x");
    assert_eq!(heard(&h), (1, 1), "the old keys");
    // Good saves clear them, one by one.
    let good = [
        ("en.ron", EN),
        ("palette.ron", PALETTE),
        ("keymap.ron", KEYMAP),
        ("stage.map", MAP),
    ];
    for (file, bytes) in good {
        h.change_file(disk, file, bytes.to_vec());
    }
    h.wait(FRAME_DT);
    assert_eq!(h.ctx().reload_errors(), [] as [String; 0]);
}
