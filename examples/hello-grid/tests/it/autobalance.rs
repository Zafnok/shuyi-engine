//! `autobalance` through the tool contract (`shuyi_cli::contract::autobalance`):
//! the stage `balance.ron` tunes, whose foe never trips, searched on
//! threads until the walker sees it stumble as often as `balance.ron`'s
//! band wants, as the core's search finds it on one thread; the time
//! limit, and `--apply` writing the tuning found.

use std::path::PathBuf;

use hello_grid::Hello;
use hello_grid::rules::{Stage, Tuning};
use hello_grid::tools::{
    BALANCE_FILE, ENTRANTS, HelloTools, autobalance_plan, balance, knobs, playtest_checks,
};
use shuyi_cli::contract::autobalance::{AutobalanceInput, AutobalanceOutput, autobalance};
use shuyi_cli::frame_png::{Script, Shot};
use shuyi_cli::{AutobalancePlan, GameArg, GameLang, GameOption, GameTools};
use shuyi_core::bots::balance::Change;
use shuyi_core::bots::playtest::{Entrant, Tries};
use shuyi_core::{BalanceSettings, Stop};

/// Seeds 0 to 49 decide, quick runs of 10, at most 40 runs: the search of
/// `balance.rs`.
fn input(more: impl FnOnce(&mut AutobalanceInput)) -> AutobalanceInput {
    let mut input = AutobalanceInput {
        tries: Some(50),
        quick: Some(10),
        budget: Some(40),
        ..AutobalanceInput::default()
    };
    more(&mut input);
    input
}

/// What `autobalance` found on `game` with `input`, which ran, its time
/// left out.
fn found(game: &dyn GameTools, input: &AutobalanceInput) -> AutobalanceOutput {
    let answer = autobalance(game, input);
    assert!(answer.error.is_none(), "{:?}", answer.error);
    let mut output = answer.result.unwrap_or_else(|| panic!("no result"));
    output.seconds = 0.0;
    output
}

#[test]
fn balance_ron_starts_from_a_foe_that_never_trips() {
    let file = balance().unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        file.tuning,
        Tuning {
            trip_percent: 0,
            foe_waits: 0
        }
    );
    assert_eq!(file.checks.len(), 1);
}

#[test]
fn the_tool_contract_finds_what_the_core_finds_on_one_thread() {
    let output = found(&HelloTools, &input(|i| i.threads = Some(4)));
    assert!(output.in_bands);
    assert_eq!(output.stop, Stop::InBands);
    assert_eq!(
        output.changes,
        [Change::new(
            "trip",
            4,
            "the foe trips 20% of the time, not 0%"
        )]
    );
    assert_eq!(output.written, None);
    assert_eq!(
        output.title,
        "the stage (files/stage.map), tuned as balance.ron says"
    );
    // The core's search over the same stage, knobs, heroes and checks.
    let game = Hello::load().unwrap_or_else(|e| panic!("{e:?}"));
    let file = balance().unwrap_or_else(|e| panic!("{e}"));
    let stage =
        Stage::with_tuning(&game.stage_map, 0, file.tuning).unwrap_or_else(|e| panic!("{e}"));
    let (trip, head_start) = knobs(file.tuning);
    let entrants: Vec<&dyn Entrant<Stage, Stage>> = ENTRANTS
        .iter()
        .map(|e| e as &dyn Entrant<Stage, Stage>)
        .collect();
    let mut checks = playtest_checks().unwrap_or_else(|e| panic!("{e}"));
    checks.extend(file.checks);
    let settings = BalanceSettings::new(Tries::new(50, 0, 200), 10, 40);
    assert_eq!(output.settings, settings);
    let core =
        shuyi_core::autobalance(&stage, &[&trip, &head_start], &entrants, &checks, &settings)
            .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(output.changes, core.changes);
    assert_eq!((output.before, output.after), (core.before, core.after));
    assert_eq!(
        (output.stop, output.evaluations),
        (core.stop, core.evaluations)
    );
}

#[test]
fn what_it_finds_is_the_same_on_one_thread_or_four() {
    let one = found(&HelloTools, &input(|i| i.threads = Some(1)));
    let four = found(&HelloTools, &input(|i| i.threads = Some(4)));
    assert_eq!(one, four);
}

#[test]
fn a_time_of_0_keeps_the_stage_as_authored_and_the_command_fails() {
    let answer = autobalance(&HelloTools, &input(|i| i.time = Some(0)));
    assert_eq!(answer.exit_code(), 1);
    let output = answer.result.unwrap_or_else(|| panic!("no result"));
    assert_eq!((output.stop, output.evaluations), (Stop::Stopped, 1));
    assert!(output.changes.is_empty(), "{:?}", output.changes);
    assert!(!output.in_bands);
    assert_eq!(answer.limits.len(), 1);
}

#[test]
fn the_command_exits_0_in_every_band_and_1_out_of_them() {
    let code = |args: &[&str]| {
        let args = ["autobalance", "--tries", "50", "--quick", "10"]
            .iter()
            .chain(args)
            .map(|a| (*a).to_owned());
        shuyi_cli::main_with(args, Some(&HelloTools))
    };
    assert_eq!(code(&["--budget", "40", "--threads", "4"]), 0);
    assert_eq!(code(&["--budget", "40", "--json"]), 0);
    assert_eq!(code(&["--budget", "3"]), 1);
    assert_eq!(code(&["--time", "0"]), 1);
    assert_eq!(code(&["--quick", "0"]), 2);
}

#[test]
fn autobalance_takes_no_start_option() {
    let with_option = input(|i| {
        i.start.push(GameArg {
            flag: "--stage".to_owned(),
            value: None,
        });
    });
    let error = autobalance(&HelloTools, &with_option).error;
    assert_eq!(
        error.as_deref(),
        Some("autobalance takes no start option, not --stage")
    );
}

/// Hello, Grid's tools, its autobalance writing `balance.ron` to a file of
/// the test's own.
struct WritesTo(PathBuf);

impl GameTools for WritesTo {
    fn frame_options(&self) -> &[GameOption] {
        HelloTools.frame_options()
    }

    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        HelloTools.frame(options, script, scale)
    }

    fn lang(&self) -> Result<GameLang, String> {
        HelloTools.lang()
    }

    fn autobalance_plan(
        &self,
        _stage: Option<&str>,
        _input: &AutobalanceInput,
    ) -> Result<AutobalancePlan, String> {
        autobalance_plan(self.0.clone())
    }
}

#[test]
fn apply_writes_the_tuning_found_to_balance_ron() {
    let dir = std::env::temp_dir().join(format!("hello-grid-balance-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join(BALANCE_FILE);
    let output = found(&WritesTo(file.clone()), &input(|i| i.apply = true));
    assert_eq!(output.written, Some(vec![BALANCE_FILE.to_owned()]));
    let written = std::fs::read_to_string(&file).unwrap();
    let read: hello_grid::tools::Balance = ron::from_str(&written).unwrap();
    assert_eq!(
        read.tuning,
        Tuning {
            trip_percent: 20,
            foe_waits: 0
        }
    );
    assert_eq!(
        read.checks,
        balance().unwrap_or_else(|e| panic!("{e}")).checks
    );
    assert_eq!(
        written,
        hello_grid::tools::balance_text(&read).unwrap_or_else(|e| panic!("{e}"))
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
