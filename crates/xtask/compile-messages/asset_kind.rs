//~ `Table` is not a kind of asset
//~ write `impl AssetKind for Table`
//~ one constant, `NAME`
//~ https://zafnok.github.io/shuyi-engine/learn/book/assets-and-formats/
//! A game's code with its table, its `impl AssetKind` missing (ticket 2510).

use shuyi::assets::{AssetPath, ContentError, DecodeCx, Format};

/// A table of the game's data.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct Table {
    rows: Vec<String>,
}

/// The game's table format.
struct TableFormat;

impl Format for TableFormat {
    type Asset = Table;

    fn id(&self) -> &'static str {
        "table"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["table"]
    }

    fn decode(
        &self,
        _file: &AssetPath,
        _bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<Table, Vec<ContentError>> {
        Ok(Table { rows: Vec::new() })
    }
}

fn main() {
    let _ = TableFormat;
}
