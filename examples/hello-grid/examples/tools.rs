//! Hello, Grid's tool program: `cargo shuyi` runs it for `frame-png` and
//! `lang-status` on this project (`shuyi.ron`). Try, from this folder:
//! `cargo shuyi frame-png stage.png --stage --keys "z Right"`.

use std::process::ExitCode;

use hello_grid::tools::HelloTools;

fn main() -> ExitCode {
    ExitCode::from(shuyi_cli::main_with(
        std::env::args().skip(1),
        Some(&HelloTools),
    ))
}
