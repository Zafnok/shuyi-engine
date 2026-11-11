//! Moon Garden's tool program, as `crates/game/examples/tools.rs` is a new
//! game's: `cargo xtask tutorial-tries` draws the book's pictures with it.
//! It is not named `tools`: `hello-grid` has an example of that name, and
//! two in one workspace write one file (ticket 2056).

use std::process::ExitCode;

fn main() -> ExitCode {
    book_snippets::quick_start::tool_program()
}
