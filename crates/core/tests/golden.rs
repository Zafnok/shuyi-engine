//! The toy dot's golden table (ticket 2137), `tests/golden/tick.ron`: a
//! record of the dot, held, tapped and turned, written as a game writes
//! one, and the state's hash after each of its steps through
//! `tick::replay`. CI checks it natively on x64 and ARM64 (`test`) and on
//! wasm32 under wasmtime (`determinism`): a record made on one replays the
//! same on the others.
//!
//! The record's stamp is a fixed one, so the table moves only when the
//! dot's rules, the record's text or the hash do, never at a release.

use shuyi_core::made::Made;
use shuyi_core::tick::{self, ActionId, SeatInput, TickInput, TickRecord, TickSim};
use shuyi_core::toy::{DOWN, Dot, LEFT, RIGHT, UP};

/// The record's stamp, of no real build.
const STAMP: &str = r#"(engine:"golden",game:"dot",content:"0000000000000000")"#;

/// One step's input of seat 0.
fn step(pressed: &[ActionId], held: &[ActionId], released: &[ActionId]) -> TickInput {
    TickInput::one(SeatInput::new(
        pressed.to_vec(),
        held.to_vec(),
        released.to_vec(),
    ))
}

/// The record: Right held 20 steps, an Up tapped, Down held and Left
/// pressed over it, then let go one at a time.
fn record(made: Made) -> TickRecord {
    let mut record = TickRecord::new(made, 1);
    let mut steps = |n: usize, input: TickInput| {
        for _ in 0..n {
            record.push(input.clone());
        }
    };
    steps(1, step(&[RIGHT], &[RIGHT], &[]));
    steps(19, step(&[], &[RIGHT], &[]));
    steps(1, step(&[], &[], &[RIGHT]));
    steps(1, step(&[UP], &[], &[]));
    steps(1, step(&[], &[], &[UP]));
    steps(1, step(&[DOWN], &[DOWN], &[]));
    steps(6, step(&[], &[DOWN], &[]));
    steps(1, step(&[LEFT], &[LEFT, DOWN], &[]));
    steps(4, step(&[], &[LEFT, DOWN], &[]));
    steps(1, step(&[], &[DOWN], &[LEFT]));
    steps(4, step(&[], &[DOWN], &[]));
    steps(1, step(&[], &[], &[DOWN]));
    steps(3, step(&[], &[], &[]));
    record
}

#[test]
fn the_dots_record_replays_to_its_golden_hashes() {
    let made: Made = ron::from_str(STAMP).unwrap_or_else(|e| panic!("{e}"));
    let record = record(made.clone());
    let mut golden = shuyi_assets::golden!("tests/golden/tick.ron");
    let text = ron::to_string(&record).unwrap_or_else(|e| panic!("{e}"));
    golden.check("record", text);
    let mut dot = Dot::default();
    let hashes = tick::replay(&mut dot, &record, &made).unwrap_or_else(|e| panic!("{e}"));
    golden.check("step 000", format_args!("{:016x}", Dot::default().hash()));
    for (i, hash) in hashes.iter().enumerate() {
        golden.check(format!("step {:03}", i + 1), format_args!("{hash:016x}"));
    }
    golden.check("end", &dot);
    golden.assert();
}
