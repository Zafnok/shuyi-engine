//! The game's tool program: `cargo shuyi <command>` runs it (the alias in
//! `.cargo/config.toml`, and `shuyi.ron`'s `game`) with the engine's tools
//! for a game like this one. Try `cargo shuyi run --frames 30`.

use std::process::ExitCode;

use shuyi_cli::BasicTools;

fn main() -> ExitCode {
    let tools = BasicTools::new(game::game());
    ExitCode::from(shuyi_cli::main_with(std::env::args().skip(1), Some(&tools)))
}
