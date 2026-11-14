//! The step's game as its checks' tests load it: with its own files, or
//! with an exercise's edits made to one of them, as the page lays the
//! reader's files over the game's.

use std::collections::BTreeMap;

use game::Data;
use shuyi::prelude::*;

/// The game's data from its own files.
pub fn own() -> Data {
    game::game()
        .data()
        .unwrap_or_else(|e| panic!("the step's files: {e:?}"))
}

/// The game's data with each `(find, replace)` of `edits` made, in order,
/// to its file `file` (`levels/1.map`), which must hold each `find` once.
pub fn edited(file: &str, edits: &[(&str, &str)]) -> Data {
    let bundle = game::game().bundle();
    let mut text = bundle
        .file(file)
        .unwrap_or_else(|| panic!("the step has no files/{file}"))
        .to_owned();
    for (find, replace) in edits {
        assert_eq!(
            text.matches(find).count(),
            1,
            "files/{file} holds `{find}` once"
        );
        text = text.replacen(find, replace, 1);
    }
    let files = BTreeMap::from([(file.to_owned(), text.into_bytes())]);
    let layer = Layer::owned("files".to_owned(), files).unwrap_or_else(|e| panic!("{e}"));
    game::game()
        .over(layer)
        .data()
        .unwrap_or_else(|e| panic!("files/{file} edited: {e:?}"))
}
