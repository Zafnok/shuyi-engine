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

fn sample_seed(sample: usize) -> u64 {
    u64::try_from(sample + WARMUP_REPLAYS).unwrap_or_else(|_| unreachable!("small sample"))
}

fn replay_percentiles(nanos: &mut [u128]) -> (u128, u128) {
    assert_eq!(nanos.len(), SAMPLES);
    nanos.sort_unstable();
    let median = u128::midpoint(nanos[SAMPLES / 2 - 1], nanos[SAMPLES / 2]);
    let p99 = nanos[(SAMPLES * 99).div_ceil(100) - 1];
    (median, p99)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(replay(11).outcome(), Some(Outcome::Won));
    for index in 0..WARMUP_REPLAYS {
        let seed = u64::try_from(index).unwrap_or_else(|_| unreachable!("small sample"));
        black_box(replay(black_box(seed)));
    }

    let mut nanos = Vec::with_capacity(SAMPLES);
    for sample in 0..SAMPLES {
        let seed = sample_seed(sample);
        let start = Instant::now();
        black_box(replay(black_box(seed)));
        nanos.push(start.elapsed().as_nanos());
    }
    let (median, p99) = replay_percentiles(&mut nanos);
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

#[cfg(test)]
mod tests {
    use super::{SAMPLES, replay_percentiles, sample_seed};

    #[test]
    fn measured_seeds_follow_the_warmups() {
        assert_eq!(sample_seed(0), 1_000);
        assert_eq!(sample_seed(7), 1_007);
    }

    #[test]
    fn sorted_replays_report_the_middle_and_nearest_rank_p99() {
        let mut nanos: Vec<u128> = (0..SAMPLES as u128).rev().collect();
        assert_eq!(replay_percentiles(&mut nanos), (9_999, 19_799));
        assert_eq!(nanos[0], 0);
        assert_eq!(nanos[SAMPLES - 1], 19_999);
    }
}
