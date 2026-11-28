//~ `Voice` is not a voice clip's variant
//~ write `impl ClipVariant for Voice`, or use `()`
//~ a game with no voices uses `()`
//~ https://zafnok.github.io/shuyi-engine/learn/book/assets-and-formats/#voices
//! A game's code with its voices, their `impl ClipVariant` missing (ticket 2510).

use shuyi::assets::dialogue::LineId;
use shuyi::assets::voice::clip_file;

/// Who says a line.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug, serde::Deserialize)]
enum Voice {
    #[default]
    Calm,
}

fn file(line: &LineId) -> String {
    clip_file(line, Voice::Calm)
}

fn main() {
    let _ = file;
}
