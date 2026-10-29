//! Hello, Grid's golden table (ticket 2116), `tests/golden/hello-grid.ron`:
//! a checked-in record's state hash at each step and its outcome, replayed
//! as `shuyi replay` replays it, and the `state_hash` of each asset the
//! game loads from its files. The same assets read back from a pack baked
//! here equal them. CI checks it natively on x64 and ARM64 (`test`) and on
//! wasm32 under wasmtime (`determinism`), where the pack is baked by a
//! 32-bit LZ4 (ADR-0023): so a record made on one replays the same on the
//! others, and a pack baked on one reads the same everywhere.
//!
//! The record, `tests/records/seed-11.ron`, is seed 11's play from `cargo
//! shuyi bots 1 --seed 11 --json`, without its stamp, so a change to the
//! game's files shows as rows moving rather than a refusal. It and the
//! table are read with `include_str!`: the test reads no file at run time.

use serde::Serialize;
use shuyi::assets::bundle::Bundle;
use shuyi::assets::dialogue::DialogueFile;
use shuyi::assets::font::FontAtlasDef;
use shuyi::assets::format::{AssetKind, Formats};
use shuyi::assets::grid_map::GridMap;
use shuyi::assets::image::ImageInfo;
use shuyi::assets::keymap::KeymapDef;
use shuyi::assets::pack::Pack;
use shuyi::assets::palette::PaletteDef;
use shuyi::core::{TerrainId, state_hash};
use shuyi_assets::golden::Golden;
use shuyi_cli::GameTools;
use shuyi_cli::contract::replay::Record;

use hello_grid::rules::Feature;
use hello_grid::tools::HelloTools;
use hello_grid::{Action, English, FONT_PNG, FONT_RON, PACK_STAMP};

/// The checked-in record.
const RECORD: &str = "tests/records/seed-11.ron";

/// A bundle and the formats it is read with.
struct Source {
    bundle: Bundle,
    formats: Formats,
}

/// The game's files, read as `Hello::load` reads them.
fn files() -> Source {
    Source {
        bundle: Bundle::new(vec![hello_grid::layer()]),
        formats: hello_grid::formats(),
    }
}

/// The same files baked into a pack here, as `cargo shuyi bake` bakes them,
/// and read as `Hello::from_pack` reads them. The pack's own bytes are not
/// pinned: LZ4 may write them differently on a 32-bit target.
fn packed(files: &Source) -> Source {
    let baked = hello_grid::bake()
        .bake(&files.formats, &files.bundle, None)
        .unwrap_or_else(|e| panic!("{e:?}"));
    let pack = Pack::open(baked.bytes.leak(), PACK_STAMP).unwrap_or_else(|e| panic!("{e}"));
    // The pack carries the fingerprint of the files it was baked from.
    assert_eq!(pack.info().content, files.bundle.fingerprint());
    let mut formats = hello_grid::formats();
    hello_grid::bake()
        .register(&mut formats)
        .unwrap_or_else(|e| panic!("{e}"));
    Source {
        bundle: Bundle::new(vec![pack.into_layer("pack")]),
        formats,
    }
}

/// Checks the row `asset <path>`: the `state_hash` of the `K` at `path`
/// of the game's files, which the pack's must equal.
fn asset<K: AssetKind + Serialize>(golden: &mut Golden, files: &Source, pack: &Source, path: &str) {
    let hash = |source: &Source| {
        let asset = source
            .formats
            .load::<K>(&source.bundle, path)
            .unwrap_or_else(|e| panic!("{path}: {e:?}"));
        state_hash(&asset).unwrap_or_else(|e| panic!("{path}: {e}"))
    };
    let loose = hash(files);
    assert_eq!(hash(pack), loose, "{path} from the pack");
    golden.check(format!("asset {path}"), loose);
}

#[test]
fn the_record_and_the_assets_give_their_golden_values() {
    let mut golden = shuyi_assets::golden!("tests/golden/hello-grid.ron");
    let record = Record {
        file: RECORD.to_owned(),
        text: include_str!("records/seed-11.ron").to_owned(),
    };
    let run = HelloTools.replay(&record).unwrap_or_else(|e| panic!("{e}"));
    for step in &run.steps {
        golden.check(format!("step {:03}", step.step), &step.hash);
    }
    let outcome = run
        .outcome
        .unwrap_or_else(|| panic!("{RECORD}'s play doesn't end"));
    golden.check("outcome", outcome);

    let files = files();
    let pack = packed(&files);
    asset::<English>(&mut golden, &files, &pack, "en.ron");
    asset::<KeymapDef<Action>>(&mut golden, &files, &pack, "keymap.ron");
    asset::<PaletteDef>(&mut golden, &files, &pack, "palette.ron");
    asset::<DialogueFile>(&mut golden, &files, &pack, "intro.dlg");
    asset::<GridMap<TerrainId, Feature>>(&mut golden, &files, &pack, "stage.map");
    asset::<FontAtlasDef>(&mut golden, &files, &pack, FONT_RON);
    asset::<ImageInfo>(&mut golden, &files, &pack, "tiles.png");
    asset::<ImageInfo>(&mut golden, &files, &pack, FONT_PNG);
    // What a record of this build is stamped with (ticket 2111).
    let content = hello_grid::made().content;
    assert_eq!(content, files.bundle.fingerprint());
    golden.check("fingerprint files", content);
    golden.assert();
}
