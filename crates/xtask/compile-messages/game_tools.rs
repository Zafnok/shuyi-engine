//~ `Tools` is not a game's tools
//~ write `impl GameTools for Tools`, or use the engine's `BasicTools`
//~ only `frame_options`, `frame` and `lang` are required
//~ https://zafnok.github.io/shuyi-engine/learn/book/tools-for-agents/
//! A game's code with its tool program, its `impl GameTools` missing (ticket 2510).

use std::process::ExitCode;

/// The game's tools.
struct Tools;

fn main() -> ExitCode {
    ExitCode::from(shuyi_cli::main_with(std::env::args().skip(1), Some(&Tools)))
}
