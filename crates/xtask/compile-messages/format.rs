//~ `TableFormat` is not a file format
//~ write `impl Format for TableFormat`
//~ `id`, `extensions` and `decode`
//~ https://zafnok.github.io/shuyi-engine/learn/book/assets-and-formats/
//! A game's code with its table format, its `impl Format` missing (ticket 2510).

use shuyi::assets::Formats;

/// The game's table format.
struct TableFormat;

fn register(formats: &mut Formats) {
    let _ = formats.register(TableFormat);
}

fn main() {
    let _ = register;
}
