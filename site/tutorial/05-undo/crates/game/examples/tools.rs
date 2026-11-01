//! The game's tool program: `cargo shuyi <command>` runs it (the alias in
//! `.cargo/config.toml`, and `shuyi.ron`'s `game`) with the game's tools:
//! its stages' (`game::stages::tools`), or the engine's for a game like
//! this one. Try `cargo shuyi run --frames 30`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let tools = game::stages::tools();
    ExitCode::from(shuyi_cli::main_with(std::env::args().skip(1), Some(&tools)))
}
