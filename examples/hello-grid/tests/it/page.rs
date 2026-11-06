//! The page's files laid over the game's, as its web build lays them (the
//! site's playground): a changed file is the one the game plays, and a
//! broken one gives its problems with their lines, which the page hears.

use std::collections::BTreeMap;

use hello_grid::{bundle_over, ctx_from, file, problems_over};
use shuyi::assets::bundle::Layer;
use shuyi::core::Pos;

/// The game's own stage map.
fn own_map() -> String {
    let bytes = file("stage.map").unwrap_or_default();
    String::from_utf8_lossy(bytes).into_owned()
}

/// The game's stage map with `from` replaced by `to`.
fn stage_map(from: &str, to: &str) -> String {
    let own = own_map();
    assert!(own.contains(from), "no `{from}` in stage.map");
    own.replace(from, to)
}

/// A layer of the page's files, as `shuyi_app::page::page_layer` makes it
/// of the page's `files/stage.map`.
fn page(stage_map: String) -> Layer {
    let files = BTreeMap::from([("stage.map".to_owned(), stage_map.into_bytes())]);
    Layer::owned("files".to_owned(), files).unwrap_or_else(|e| panic!("{e:?}"))
}

#[test]
fn a_changed_file_of_the_page_is_the_one_the_game_plays() {
    let page = page(stage_map("(1, 8): Hero", "(2, 8): Hero"));
    assert_eq!(problems_over(page), None);
    let ctx = ctx_from(&bundle_over(Some(page), "web"), "web", 0);
    assert!(ctx.game.stage_map.features.contains_key(&Pos::new(2, 8)));
    // The game's own map has its hero elsewhere.
    let own = ctx_from(&bundle_over(None, "web"), "web", 0);
    assert!(!own.game.stage_map.features.contains_key(&Pos::new(2, 8)));
}

#[test]
fn a_broken_file_of_the_page_gives_its_problems_with_their_lines() {
    let row = "#.##.####..#";
    let page = page(stage_map(row, "#.##.####X.#"));
    let problems = problems_over(page).unwrap_or_else(|| panic!("no problems"));
    let first = problems.lines().next().unwrap_or_default();
    assert!(first.starts_with("files/stage.map:"), "{problems}");
    // Its line, as `cargo shuyi check` prints it: the row with the `X`.
    let line = first.split(':').nth(1).unwrap_or_default();
    let at = own_map().lines().position(|l| l == row).map(|i| i + 1);
    assert_eq!(line.parse::<usize>().ok(), at, "{problems}");
}
