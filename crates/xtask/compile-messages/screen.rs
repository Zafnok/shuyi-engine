//~ is not a screen of `
//~ write `impl Screen<
//~ goes on the stack boxed
//~ https://zafnok.github.io/shuyi-engine/learn/book/games-and-screens/
//! A game's code with its title screen, its `impl Screen` missing (ticket 2510).

use shuyi::prelude::*;

type Data = BasicData<UiAction>;

/// The title screen.
struct Title;

fn first() -> Box<dyn Screen<Data>> {
    Box::new(Title)
}

fn main() {
    let _ = first();
}
