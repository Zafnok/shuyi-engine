//! **Snake**: Shuyi Engine's grid game in real time (ADR-0148), on the
//! short path. The snake moves a tile each time its timer fires, turned by
//! the direction keys; it grows when it eats the food and the run ends on a
//! wall or its own body. A title, the board ([`screens::Stage`]), a pause
//! menu and a screen that says how the run ended.
//!
//! Its rules ([`rules::Snake`]) are a `TickSim`: pure, stepped 60 times a
//! second by the engine's `TickDriver`, so the snake moves the same at 30
//! and 60 frames a second and a record of its steps replays the same
//! anywhere. Its files are in `files/`, embedded: the engine's (the text,
//! the keys, the palette, the font and the sounds) and its own part
//! ([`Own`]): the board and the run's numbers. [`game`] says all the engine
//! needs to know: `src/main.rs` runs it, and `examples/tools.rs` hands it to
//! the engine's tools (`snake::tools`, with the `tools` feature).

pub mod app_info;
pub mod rules;
pub mod screens;
#[cfg(feature = "tools")]
pub mod tools;

use std::sync::OnceLock;

use shuyi::assets::format::Formats;
use shuyi::assets::grid_map::MapText;
use shuyi::assets::ron::Ron;
use shuyi::core::{Made, TerrainId};
use shuyi::prelude::*;

use rules::{Board, Feature, Snake, Tuning};

/// The game's files, embedded.
// check-text: not player text, a folder
static FILES: Embedded = shuyi::files!("$CARGO_MANIFEST_DIR/files");

/// The board, in the game's files.
// check-text: not player text, a file name
pub const BOARD: &str = "board.map";
/// The run's numbers, in the game's files.
// check-text: not player text, a file name
pub const TUNING: &str = "tuning.ron";

/// The game's console: 50 × 15 cells of 8 × 16 pixels, 400 × 240, which
/// the Playdate's screen fits at scale 1: the board's 25 × 14 tiles, two
/// cells each, under a line for the score.
pub const CONSOLE: ConsoleSpec = ConsoleSpec::text(50, 15);

/// The game's own part of its data: the board, the run's numbers and the
/// next run's seed.
#[derive(Debug)]
pub struct Own {
    /// The board.
    pub board: Board,
    /// The run's numbers.
    pub tuning: Tuning,
    /// The seed of the next run's luck.
    pub seed: u64,
}

impl Own {
    /// The seed of a new run's luck: each run the next one.
    pub fn next_seed(&mut self) -> u64 {
        let seed = self.seed;
        self.seed = self.seed.wrapping_add(1);
        seed
    }

    /// A new run, its luck from the next seed.
    pub fn next_run(&mut self) -> (u64, Snake) {
        let seed = self.next_seed();
        // `own` made a run of this board and tuning, so this does too.
        let snake =
            Snake::new(&self.board, self.tuning, seed).unwrap_or_else(|e| panic!("{BOARD}: {e}"));
        (seed, snake)
    }
}

/// The game's own part, read from its files: a board that makes a run with
/// its numbers. The first run's seed is 1.
pub fn own(bundle: &Bundle) -> Result<Own, Vec<ContentError>> {
    let mut formats = Formats::standard();
    // The standard registry has neither kind: none is refused.
    let _ = formats.register(MapText::<TerrainId, Feature>::new(
        rules::FORMAT,
        rules::terrain_named,
    ));
    let _ = formats.register(Ron::<Tuning>::new());
    let board = formats.load::<Board>(bundle, BOARD);
    let tuning = formats.load::<Tuning>(bundle, TUNING);
    let (board, tuning) = match (board, tuning) {
        (Ok(board), Ok(tuning)) => (board, tuning),
        (board, tuning) => {
            return Err([board.err(), tuning.err()]
                .into_iter()
                .flatten()
                .flatten()
                .collect());
        }
    };
    Snake::new(&board, tuning, 0).map_err(|e| vec![ContentError::new(BOARD, e)])?;
    Ok(Own {
        board,
        tuning,
        seed: 1,
    })
}

/// The game's data and state, which its screens share: the engine's, read
/// from `files/`, with the six moves of `files/keymap.ron` as its actions,
/// and its own part.
pub type Data = BasicData<UiAction, Own>;

/// The game: who it is ([`app_info::APP`]), its files and its part of them,
/// its console, its title first, and every screen it can show.
pub fn game() -> BasicGame<UiAction, Own> {
    // check-text: not player text, a folder
    BasicGame::with_own(app_info::APP, Layer::dir("files", FILES), own, |_| {
        Box::new(screens::Title)
    })
    .console(CONSOLE)
    .screens(screens::every)
    .modules(shuyi::enabled_modules)
}

/// What made this build, for its records: the game's id and version and
/// the fingerprint of the files it ships, computed once. A record keeps it,
/// and `replay` refuses one another build made.
pub fn made() -> Made {
    static MADE: OnceLock<Made> = OnceLock::new();
    MADE.get_or_init(|| game().made()).clone()
}
