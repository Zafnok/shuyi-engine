//! The game's tool program: `cargo shuyi <command>` runs it (`shuyi.ron`'s
//! `game`) with the game's tools (`snake::tools::Tools`). Try `cargo shuyi
//! run --frames 30`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let tools = snake::tools::Tools::new(snake::game());
    ExitCode::from(shuyi_cli::main_with(std::env::args().skip(1), Some(&tools)))
}
