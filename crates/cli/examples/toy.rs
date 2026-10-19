//! The toy game's tool program: `shuyi` runs it for `frame-png` and
//! `lang-status` on the toy project (`toy/shuyi.ron`). Try, from `toy/`:
//! `cargo shuyi frame-png toy.png --lobby`.

use std::process::ExitCode;

use shuyi_cli::toy::ToyTools;

fn main() -> ExitCode {
    ExitCode::from(shuyi_cli::main_with(
        std::env::args().skip(1),
        Some(&ToyTools),
    ))
}
