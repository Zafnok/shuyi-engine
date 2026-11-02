//! Moon Garden's tool program, as `crates/game/examples/tools.rs` is a new
//! game's: `cargo xtask tutorial-tries` draws the book's pictures with it.

use std::process::ExitCode;

fn main() -> ExitCode {
    book_snippets::quick_start::tool_program()
}
