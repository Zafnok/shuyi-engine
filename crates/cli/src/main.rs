//! `shuyi`: Shuyi Engine's tools, run on the project whose `shuyi.ron` is
//! in the working folder or one above it. See the library's docs.

use std::process::ExitCode;

fn main() -> ExitCode {
    ExitCode::from(shuyi_cli::main_with(std::env::args().skip(1), None))
}
