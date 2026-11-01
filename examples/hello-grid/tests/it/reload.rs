//! The palette reloading in the running game, through the headless
//! backend's script of changed files: seen on the next frame, a broken file
//! showing its error and keeping the colours, and nothing asked of the
//! platform with the debug tools off.

use std::collections::BTreeMap;

use hello_grid::hot_palette::HotPalette;
use hello_grid::{Hello, bundle_over, ctx_from, file, game};
use shuyi::assets::bundle::Layer;
use shuyi::platform::harness::{FRAME_DT, Harness};
use shuyi::ui::frame::EngineColor;

/// The game's palette file with its panel colour changed to `panel`.
fn palette(panel: &str) -> Vec<u8> {
    let bytes = file("palette.ron").unwrap_or_default();
    let text = String::from_utf8_lossy(bytes);
    text.replace("\"panel\": \"#16161c\"", &format!("\"panel\": \"{panel}\""))
        .into_bytes()
}

/// The game with its palette on a layer of a folder's, as `run --watch`
/// starts it, in a Harness.
fn watched() -> (Harness<Hello>, Layer) {
    watched_with(true)
}

/// [`watched`], with its debug tools on or off.
fn watched_with(debug_tools: bool) -> (Harness<Hello>, Layer) {
    let files = BTreeMap::from([("palette.ron".to_owned(), palette("#16161c"))]);
    let disk = Layer::owned("files".to_owned(), files).unwrap_or_else(|e| panic!("{e:?}"));
    let bundle = bundle_over(Some(disk), "desktop");
    let module = HotPalette::new(&bundle).unwrap_or_else(|e| panic!("{e:?}"));
    let game = game(ctx_from(&bundle, "desktop", 7))
        .with_debug_screens(debug_tools)
        .with_module(module);
    (Harness::new(game.unwrap_or_else(|e| panic!("{e:?}"))), disk)
}

/// The panel colour the screens draw with.
fn panel(h: &Harness<Hello>) -> [u8; 3] {
    let color = h.ctx().palette.engine(EngineColor::Panel);
    [color.r, color.g, color.b]
}

#[test]
fn a_changed_palette_file_is_seen_by_the_game_on_the_next_frame() {
    let (mut h, disk) = watched();
    assert_eq!(panel(&h), [0x16, 0x16, 0x1c]);
    h.change_file(disk, "palette.ron", palette("#102030"));
    assert_eq!(panel(&h), [0x16, 0x16, 0x1c], "not before the frame");
    h.wait(FRAME_DT);
    assert_eq!(panel(&h), [0x10, 0x20, 0x30]);
    assert_eq!(h.platform().reload_errors(), [] as [String; 0]);
}

#[test]
fn a_file_it_doesnt_read_changes_nothing() {
    let (mut h, disk) = watched();
    h.change_file(disk, "en.ron", b"not read by the palette".to_vec());
    h.wait(FRAME_DT);
    assert_eq!(panel(&h), [0x16, 0x16, 0x1c]);
    assert_eq!(h.platform().reload_errors(), [] as [String; 0]);
}

#[test]
fn a_broken_palette_keeps_its_colours_and_shows_its_error_until_the_next_good_one() {
    let (mut h, disk) = watched();
    h.change_file(disk, "palette.ron", palette("not a colour"));
    h.wait(FRAME_DT);
    assert_eq!(panel(&h), [0x16, 0x16, 0x1c]);
    let errors = h.platform().reload_errors().to_vec();
    assert_eq!(errors.len(), 1, "{errors:?}");
    assert!(
        errors[0].starts_with("files/palette.ron:8:") && errors[0].contains("panel"),
        "{errors:?}"
    );
    // It stays, frame after frame.
    for _ in 0..3 {
        h.wait(FRAME_DT);
    }
    assert_eq!(h.platform().reload_errors(), errors);
    h.change_file(disk, "palette.ron", palette("#102030"));
    h.wait(FRAME_DT);
    assert_eq!(panel(&h), [0x10, 0x20, 0x30]);
    assert_eq!(h.platform().reload_errors(), [] as [String; 0]);
}

#[test]
fn the_module_is_listed_with_its_hook() {
    let (h, _) = watched();
    let modules = h.game().modules();
    assert_eq!(modules[0].name, "hot_palette");
    assert_eq!(modules[0].hooks[0].name(), "reloaded");
}

#[test]
fn with_the_debug_tools_off_the_platform_is_never_asked() {
    let (mut on, _) = watched();
    on.wait(FRAME_DT);
    on.wait(FRAME_DT);
    assert_eq!(on.platform().changed_asks(), 2);
    let (mut off, disk) = watched_with(false);
    off.change_file(disk, "palette.ron", palette("#102030"));
    for _ in 0..5 {
        off.wait(FRAME_DT);
    }
    assert_eq!(off.platform().changed_asks(), 0);
    assert_eq!(panel(&off), [0x16, 0x16, 0x1c]);
}
