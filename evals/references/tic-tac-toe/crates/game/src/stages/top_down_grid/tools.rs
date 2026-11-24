//! The game's tools with its stage, for the engine's `shuyi` (`cargo
//! shuyi <command>`): the engine's for a game on the short path
//! (`BasicTools`: `frame-png`, `lang-status`, `check` with the font atlas
//! its text is checked against, and the controls
//! `check --target` checks), with a `--seed` for the first game, and the
//! stage's own: `run` and `scene` with its board and state, `replay`
//! (a [`StageRecord`], stamped with what made it: [`crate::made`]),
//! `playtest` (the engine's random bot on both sides, each try recorded:
//! [`playtest_plan`]), `import` (the engine's kinds) and `bake`, the
//! engine's. `examples/tools.rs` is the tool program that runs them; the
//! tests use them too.

use rules::{Board, Mark, Outcome};
use serde::Serialize;
use shuyi::assets::font::FontAtlasDef;
use shuyi::core::bots::playtest::{Measures, PlayedTry, Tried, bot_seed};
use shuyi::core::{Bot, Made, Pos, RandomBot, Sides, TurnRules, play};
use shuyi::platform::harness::Harness;
use shuyi::platform::session::{SessionRecord, Stamps};
use shuyi::prelude::*;
use shuyi::ui::input::fit::Controls;
use shuyi_cli::contract::import::Importers;
use shuyi_cli::contract::playtest::PlaytestInput;
use shuyi_cli::contract::replay::{Record, ReplayRun, turns};
use shuyi_cli::contract::session::{Headless, Scene, Session};
use shuyi_cli::frame_png::{Script, Shot};
use shuyi_cli::{BasicTools, GameArg, GameBake, GameLang, GameOption, GameTools, PlaytestPlan};

use super::{Own, StageRecord, StageScreen};
use crate::Data;

/// The game's own options, for `frame-png`, `run` and `scene`.
pub const OPTIONS: [GameOption; 1] =
    [
        GameOption::new("--seed", "seed the first game with N (0 by default)")
            .with_value(Some("N")),
    ];

/// The first game's seed `options` ask for.
pub fn seed_of(options: &[GameArg]) -> Result<u64, String> {
    let mut seed = 0;
    for option in options {
        match (option.flag.as_str(), option.value.as_deref()) {
            ("--seed", Some(n)) => {
                seed = n.parse().map_err(|_| format!("--seed {n}: not a number"))?;
            }
            (flag, _) => return Err(format!("unknown option {flag}")),
        }
    }
    Ok(seed)
}

/// A mark's name in the scene's data.
// check-text: not player text, the scene's data
fn named(mark: Mark) -> &'static str {
    match mark {
        Mark::X => "x",
        Mark::O => "o",
    }
}

/// The data of the stage's scene.
#[derive(Serialize, schemars::JsonSchema)]
pub struct StageData {
    /// The side to move: `x` or `o`.
    pub turn: &'static str,
    /// Empty until the game ends, then `x`, `o` or `draw`.
    pub winner: &'static str,
    /// The board's rows, top to bottom: `x`, `o` or `.` a cell.
    pub board: Vec<String>,
    /// The cell under the cursor.
    pub cursor: Pos,
    /// The seed the game's record keeps.
    pub seed: u64,
    /// How many commands the game has taken.
    pub commands: usize,
}

/// The stage's scene in `h`: its board and the rules' state
/// ([`StageData`]).
pub fn scene(h: &Harness<Data>) -> Result<Scene, String> {
    let screen = h
        .screen::<StageScreen>()
        .ok_or_else(|| format!("no stage is open (the top screen is `{}`)", h.top_screen()))?;
    let board = screen.board.clone();
    let rows: Vec<String> = (0..rules::SIZE)
        .map(|y| {
            (0..rules::SIZE)
                .map(|x| board.at(Pos::new(x, y)).map_or(".", named))
                .collect()
        })
        .collect();
    let winner = match board.outcome() {
        None => "",
        Some(Outcome::Won { mark, .. }) => named(mark),
        // check-text: not player text, the scene's data
        Some(Outcome::Draw { .. }) => "draw",
    };
    let data = StageData {
        turn: named(board.side_to_move()),
        winner,
        board: rows.clone(),
        cursor: screen.cursor,
        seed: screen.seed,
        commands: screen.commands.len(),
    };
    Scene::new("stage", "board", rows.join("\n")).with_data(&data)
}

/// What happened in the rules of the stage in `h`.
pub fn events(h: &Harness<Data>) -> Vec<String> {
    h.screen::<StageScreen>()
        .map_or_default(|screen| screen.events.iter().map(|e| format!("{e:?}")).collect())
}

/// The stage's bot run for `playtest` (`cargo shuyi playtest --tries 20`):
/// one entrant, `random`, playing both sides, and no checks, so it judges
/// nothing and passes when every try played. Each try is [`bot_try`]'s.
pub fn playtest_plan() -> PlaytestPlan {
    PlaytestPlan::new(
        "a game, played by a random bot on both sides",
        vec![("random".to_owned(), None)],
        Box::new(|_| Vec::new()),
        Box::new(|_, seed, turns| bot_try(seed, turns)),
    )
}

/// A try of a game by two random bots of the try's seed `seed`, to its
/// end or turn `limit`: the empty cells left, and its record, stamped
/// with this build.
fn bot_try(seed: u64, limit: u32) -> Result<PlayedTry, String> {
    let mut board = Board::new();
    let mut x = RandomBot::new(bot_seed(seed));
    let mut o = RandomBot::new(bot_seed(seed.wrapping_add(1)));
    let mut bots: [(Mark, &mut dyn Bot<Board>); 2] = [(Mark::X, &mut x), (Mark::O, &mut o)];
    let report = play(&mut board, &mut bots, limit).map_err(|e| format!("seed {seed}: {e}"))?;
    let empty = i64::try_from(board.legal_commands().len()).unwrap_or(i64::MAX);
    let tried = Tried {
        seed,
        outcome: report.outcome.map(|o| format!("{o:?}")),
        turns: report.turns,
        measures: Measures::from([("empty cells".to_owned(), empty)]),
    };
    let commands = u32::try_from(report.commands.len()).unwrap_or(u32::MAX);
    let record = StageRecord {
        seed,
        commands: report.commands,
        made: Some(crate::made()),
    };
    let mut one = PlayedTry::new(tried, commands);
    one.record = ron::to_string(&record).ok();
    Ok(one)
}

/// A session of `harness`, with the stage's scene and events and the game's
/// picture.
fn session_of(harness: Harness<Data>) -> Box<dyn Session> {
    let session = Headless::new(harness)
        .with_scene(scene)
        .with_events(events)
        .with_picture(|h, scale| BasicTools::<UiAction, Own>::picture(h, scale.unwrap_or(1)));
    Box::new(session)
}

/// The game's tools: the engine's, and the stage's on top.
#[derive(Debug, Clone, Copy)]
pub struct Tools {
    game: BasicGame<UiAction, Own>,
}

impl Tools {
    /// The tools of `game`.
    pub fn new(game: BasicGame<UiAction, Own>) -> Self {
        Self { game }
    }

    /// The engine's tools for the game.
    fn basic(&self) -> BasicTools<UiAction, Own> {
        BasicTools::new(self.game)
    }

    /// The game in the Harness, at its title, its first game seeded
    /// `seed`.
    pub fn harness(&self, seed: u64) -> Result<Harness<Data>, String> {
        let mut ctx = self.game.ctx()?;
        ctx.game.own.seed = seed;
        let game = self.game.game(ctx).map_err(|e| e.to_string())?;
        Ok(Harness::new(game))
    }
}

impl GameTools for Tools {
    fn frame_options(&self) -> &[GameOption] {
        &OPTIONS
    }

    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        let mut h = self.harness(seed_of(options)?)?;
        script.play(&mut h);
        let image = BasicTools::<UiAction, Own>::picture(&h, scale.unwrap_or(1))?;
        Ok(Shot::new(image, h.top_screen()))
    }

    fn lang(&self) -> Result<GameLang, String> {
        self.basic().lang()
    }

    fn check(&self) -> Result<Vec<ContentError>, String> {
        self.basic().check()
    }

    fn session(&self, options: &[GameArg]) -> Result<Box<dyn Session>, String> {
        Ok(session_of(self.harness(seed_of(options)?)?))
    }

    /// The record played from the game's real start, as `src/main.rs`
    /// runs it (`BasicGame::start`).
    fn session_from(
        &self,
        record: &SessionRecord,
        stamps: Stamps,
    ) -> Result<Box<dyn Session>, String> {
        let harness =
            Harness::from_session(self.game.start(), record, stamps).map_err(|e| e.to_string())?;
        Ok(session_of(harness))
    }

    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        let parsed: StageRecord =
            ron::from_str(&record.text).map_err(|e| format!("{}: {e}", record.file))?;
        let mut run = turns(&record.file, Board::new(), &parsed.commands)?;
        run.made = parsed.made;
        Ok(run)
    }

    /// [`crate::made`]: `replay` refuses a record another build made.
    fn made(&self) -> Option<Made> {
        Some(crate::made())
    }

    /// The stage's bot run ([`playtest_plan`]); the game's options don't
    /// change it.
    fn playtest_plan(
        &self,
        _stage: Option<&str>,
        _input: &PlaytestInput,
    ) -> Result<PlaytestPlan, String> {
        Ok(playtest_plan())
    }

    fn importers(&self) -> Importers {
        Importers::standard()
    }

    /// Its keymap and every screen it can show (`game::game`'s screens).
    fn font_atlas(&self) -> Option<Result<FontAtlasDef, String>> {
        self.basic().font_atlas()
    }

    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        self.basic().controls()
    }

    /// The engine's bake of the game's files: what a release export ships
    /// beside the program.
    fn bake(&self, target: Option<&str>) -> Result<GameBake, String> {
        self.basic().bake(target)
    }
}
