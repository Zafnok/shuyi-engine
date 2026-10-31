//! The game on the Playdate (the `playdate` feature): how it starts there,
//! for `cargo shuyi export playdate`. Its data comes from the baked pack in
//! the `.pdx`, its console fits the screen, and its frames are painted in
//! the one-bit look ([`Hello::one_bit_ctx`]).

use crate::prelude::*;

use shuyi::assets::ContentError;
use shuyi::platform::{Card, Look, Setup, Start};
use shuyi::ui::{Ctx, Game};

use crate::app_info::APP;
use crate::{Hello, game};

shuyi_playdate::main!(start);

/// What the game's frames are painted with.
fn look(game: &Hello) -> Look<'_> {
    game.look()
}

/// `errors` as one text, to show.
fn lines(errors: &[ContentError]) -> String {
    let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
    lines.join("\n")
}

/// The game on `setup`, from `ctx`, at its title.
pub fn started(setup: Setup, ctx: Ctx<Hello>) -> Game<Hello> {
    let mut ctx = setup.ctx(ctx);
    ctx.game.seed = ctx.music_seed;
    game(ctx)
}

/// The game read from the pack in the `.pdx`.
fn load(setup: Setup) -> Result<Game<Hello>, String> {
    let pack = shuyi_playdate::pack()?;
    let ctx = Hello::one_bit_ctx(pack).map_err(|errors| lines(&errors))?;
    Ok(started(setup, ctx))
}

/// How the game starts on the Playdate.
pub fn start() -> Start<Hello> {
    Start::new(Card::text(APP.name), look, load)
}
