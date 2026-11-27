//! The game on the Playdate (the `playdate` feature): how it starts there,
//! for `cargo shuyi export playdate`. Its data comes from the baked pack in
//! the `.pdx`, its console fits the screen, and its frames are painted in
//! the one-bit look ([`Hello::one_bit_ctx`]).

use crate::prelude::*;

use shuyi::platform::Progress;
use shuyi::prelude::{Card, ContentError, Ctx, Game, Look, Setup, Start};

use crate::app_info::APP;
use crate::{Hello, game, loading};

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

/// The game read from the pack in the `.pdx`, a file a step: the card
/// counts them, as many a frame as the profile's slice holds.
fn load(setup: Setup) -> impl FnMut() -> Progress<Hello> {
    loading::in_steps(
        || {
            let pack = shuyi_playdate::pack()?;
            Hello::one_bit_files(pack).map_err(|errors| lines(&errors))
        },
        move |data, bundle, formats| {
            let ctx = Hello::one_bit_ctx_of(data, bundle, formats);
            Ok(started(setup, ctx.map_err(|errors| lines(&errors))?))
        },
    )
}

/// How the game starts on the Playdate.
pub fn start() -> Start<Hello> {
    Start::in_steps(Card::text(APP.name), look, load)
}
