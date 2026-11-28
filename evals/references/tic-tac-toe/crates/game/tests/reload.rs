//! The board under `cargo shuyi run --watch`: a folder on disk laid over
//! the game's files. The board reads no file of its own, so the grid
//! template's map changing there, even broken, shows no error.

use std::collections::BTreeMap;

use shuyi::prelude::Layer;
use shuyi::prelude::test::FRAME_DT;

#[test]
fn a_broken_map_on_disk_changes_nothing() {
    let disk = Layer::owned("disk".to_owned(), BTreeMap::new()).unwrap_or_else(|e| panic!("{e:?}"));
    let mut h = game::game().over(disk).harness();
    h.keys("Enter");
    h.change_file(disk, "stage.map", b"not a map".to_vec());
    h.wait(FRAME_DT);
    assert_eq!(h.ctx().reload_errors(), [] as [String; 0]);
    assert_eq!(h.screens(), ["title", "stage"]);
}
