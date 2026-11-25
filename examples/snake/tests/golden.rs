//! Snake's golden table (ticket 2116's form), `tests/golden/snake.ron`: a
//! checked-in record's state hash at each step and its outcome, replayed as
//! `shuyi replay` replays it. CI checks it natively on x64 and ARM64
//! (`test`) and on wasm32 under wasmtime (`determinism`), so a run recorded
//! on one replays the same on the others.
//!
//! The record, `tests/records/seed-1.ron`, is the playthrough's
//! (`tests/it/main.rs`): seed 1 from the title, steered to its first food,
//! then into a wall. `cargo test -p snake --test it -- --ignored
//! write_the_golden_record` writes it again. It and the table are read
//! with `include_str!`.

use shuyi_cli::GameTools;
use shuyi_cli::contract::replay::Record;
use snake::tools::Tools;

/// The checked-in record.
const RECORD: &str = "tests/records/seed-1.ron";

#[test]
fn the_records_steps_give_their_golden_hashes() {
    let mut golden = shuyi_assets::golden!("tests/golden/snake.ron");
    let record = Record {
        file: RECORD.to_owned(),
        text: include_str!("records/seed-1.ron").to_owned(),
    };
    let run = Tools::new(snake::game())
        .replay(&record)
        .unwrap_or_else(|e| panic!("{e}"));
    for step in &run.steps {
        golden.check(format!("step {:03}", step.step), &step.hash);
    }
    let outcome = run
        .outcome
        .unwrap_or_else(|| panic!("{RECORD}'s run doesn't end"));
    golden.check("outcome", outcome);
    golden.assert();
}
