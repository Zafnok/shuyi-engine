//~ `Table` can't be baked
//~ implement or derive what `Table` lacks
//~ `PartialEq` is not implemented for `Table`
//~ `AssetKind`, `Serialize`, `Deserialize`, `PartialEq` and `Debug`
//~ https://zafnok.github.io/shuyi-engine/learn/book/assets-and-formats/
//! A game's code with its table, which lacks `PartialEq`, baked (ticket 2510).

use shuyi::assets::AssetKind;
use shuyi::assets::pack::Bake;

/// A table of the game's data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Table {
    rows: Vec<String>,
}

impl AssetKind for Table {
    const NAME: &str = "table";
}

fn main() {
    let _ = Bake::new("stamp").asset::<Table>("tables/main.table");
}
