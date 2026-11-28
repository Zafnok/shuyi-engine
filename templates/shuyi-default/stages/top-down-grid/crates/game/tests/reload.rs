//! The stage's map reloading under `cargo shuyi run --watch`: a folder on
//! disk laid over the game's files, and a broken map there showing its
//! error, which names the file once, with its line.

use std::collections::BTreeMap;

use game::stages::top_down_grid::MAP;
use shuyi::prelude::Layer;
use shuyi::prelude::test::FRAME_DT;

/// The shipped map.
const SHIPPED: &str = include_str!("../files/stage.map");

/// The map's fourth tile row, and the same row with a tile no legend
/// names.
const ROW: &str = "#...####...#";
const BROKEN: &str = "#...##?#...#";

/// The 1-based line of the shipped map whose text is `row`.
fn line_of(row: &str) -> usize {
    let index = SHIPPED.lines().position(|l| l == row);
    index.unwrap_or_else(|| panic!("no row {row:?}")) + 1
}

#[test]
fn a_broken_map_on_disk_shows_its_file_once_and_its_line() {
    let disk = Layer::owned("disk".to_owned(), BTreeMap::new()).unwrap_or_else(|e| panic!("{e:?}"));
    let mut h = game::game().over(disk).harness();
    h.change_file(disk, MAP, SHIPPED.replace(ROW, BROKEN).into_bytes());
    h.wait(FRAME_DT);
    let errors = h.ctx().reload_errors().to_vec();
    assert_eq!(errors.len(), 1, "{errors:?}");
    let at = format!("disk/{MAP}:{}:", line_of(ROW));
    assert!(errors[0].starts_with(&at), "{errors:?}");
    assert_eq!(errors[0].matches(MAP).count(), 1, "{errors:?}");
    // The next good map clears it.
    h.change_file(disk, MAP, SHIPPED.as_bytes().to_vec());
    h.wait(FRAME_DT);
    assert_eq!(h.ctx().reload_errors(), [] as [String; 0]);
}
