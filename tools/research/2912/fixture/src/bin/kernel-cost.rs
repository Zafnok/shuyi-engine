//! In-process cost of the same Workshop replay in each kernel build.

use std::hint::black_box;
use std::io::Write as _;
use std::time::Instant;

use cloning_fixture::workshop::{Action, Outcome, Workshop};
use shuyi_core::TurnRules;

const ACTIONS: [Action; 11] = [
    Action::Gather,
    Action::Gather,
    Action::Refine,
    Action::Craft,
    Action::Deliver,
    Action::Gather,
    Action::Rest,
    Action::Gather,
    Action::Refine,
    Action::Craft,
    Action::Deliver,
];
const WARMUP_REPLAYS: usize = 1_000;
const SAMPLES: usize = 20_000;

fn replay(seed: u64) -> Workshop {
    let mut game = Workshop::new(seed);
    for action in ACTIONS {
        let _ = game
            .apply(&action)
            .unwrap_or_else(|_| unreachable!("fixed winning replay is legal"));
    }
    game
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(replay(11).outcome(), Some(Outcome::Won));
    for index in 0..WARMUP_REPLAYS {
        let seed = u64::try_from(index).unwrap_or_else(|_| unreachable!("small sample"));
        black_box(replay(black_box(seed)));
    }

    let mut nanos = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        let seed =
            u64::try_from(sample + WARMUP_REPLAYS).unwrap_or_else(|_| unreachable!("small sample"));
        let start = Instant::now();
        black_box(replay(black_box(seed)));
        nanos.push(start.elapsed().as_nanos());
    }
    nanos.sort_unstable();
    let median = u128::midpoint(nanos[SAMPLES / 2 - 1], nanos[SAMPLES / 2]);
    let p99 = nanos[(SAMPLES * 99).div_ceil(100) - 1];
    let mut output = std::io::stdout().lock();
    serde_json::to_writer(
        &mut output,
        &serde_json::json!({
            "median_ns_per_replay": median,
            "p99_ns_per_replay": p99,
            "actions_per_replay": ACTIONS.len(),
            "samples": SAMPLES,
            "warmup_replays": WARMUP_REPLAYS,
            "includes": "Workshop construction, 11 accepted actions and per-replay timer overhead; excludes process startup and rendering",
        }),
    )?;
    writeln!(output)?;
    Ok(())
}
