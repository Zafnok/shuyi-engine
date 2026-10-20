//! The game's tool program: `cargo shuyi <command>` runs it (the alias in
//! `.cargo/config.toml`, and `shuyi.ron`'s `game`). Try `cargo shuyi run
//! --frames 30`.

use std::process::ExitCode;

use game::tools::Tools;

fn main() -> ExitCode {
    ExitCode::from(shuyi_cli::main_with(std::env::args().skip(1), Some(&Tools)))
}
