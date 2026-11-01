//! The game's tools: `BasicTools` has no scene, bot or replay, so this
//! `GameTools` adds them and calls `BasicTools` for the rest.
//! `examples/tools.rs` runs them, and the tests use them too.

use serde::{Deserialize, Serialize};
use shuyi::core::bots::{Bot, play};
use shuyi::core::{Dir, Made, TurnRules};
use shuyi::platform::harness::Harness;
use shuyi::prelude::*;
use shuyi::ui::input::fit::Controls;
use shuyi_cli::contract::bots::{BotsInput, BotsOutput, Play, UNFINISHED, tally};
use shuyi_cli::contract::replay::{Record, ReplayRun, turns};
use shuyi_cli::contract::session::{Headless, Scene, Session};
use shuyi_cli::frame_png::{Script, Shot};
use shuyi_cli::serde_json::json;
use shuyi_cli::{BasicTools, GameArg, GameLang, GameOption, GameTools};

use crate::action::Action;
use crate::board::{Board, Terrain};
use crate::rules::{Command, Puzzle};
use crate::stages::Own;
use crate::{Data, made};

// ANCHOR: bot
/// A bot that plays a line kept in the game's files: each level's is in
/// `files/lines/`.
#[derive(Debug, Clone)]
pub struct LineBot {
    /// The steps left, last first.
    steps: Vec<Dir>,
}

impl LineBot {
    /// The bot that plays level `level`'s line, counted from 0.
    pub fn new(bundle: &Bundle, level: usize) -> Result<Self, String> {
        // check-text: not player text, a file name
        let name = format!("lines/{}.ron", level + 1);
        let text = bundle.file(&name).ok_or(format!("no {name}"))?;
        let mut steps: Vec<Dir> = ron::from_str(text).map_err(|e| format!("{name}: {e}"))?;
        steps.reverse();
        Ok(Self { steps })
    }
}

impl Bot<Puzzle> for LineBot {
    fn choose(&mut self, puzzle: &Puzzle) -> Command {
        // A line played out with no win steps wherever it can: the play's
        // turn limit ends it.
        let legal = || puzzle.legal_commands().first().copied();
        self.steps
            .pop()
            .map(Command::Step)
            .or_else(legal)
            .unwrap_or(Command::Step(Dir::Up))
    }
}
// ANCHOR_END: bot

// ANCHOR: record
/// A record of a level: which one and the commands sent to its rules, and
/// what made it. The game's record format for `replay`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LevelRecord {
    /// The level, counted from 0.
    pub level: usize,
    /// The commands, in order.
    pub commands: Vec<Command>,
    /// What made it ([`crate::made`]); a record without one still reads.
    #[serde(default)]
    pub made: Option<Made>,
}
// ANCHOR_END: record

// ANCHOR: scene
/// The board's scene in `h`: its map, and what a program asks of it.
pub fn scene(h: &Harness<Data>) -> Result<Scene, String> {
    let board = h
        .screen::<Board>()
        .ok_or_else(|| format!("no level is open (the top screen is `{}`)", h.top_screen()))?;
    let map = board.scene(Board::view_tiles());
    let mut scene = Scene::new("board", "grid map", map.to_text(&Terrain));
    scene.data = json!({
        "level": board.level,
        "moves": board.history.len(),
        "won": board.puzzle.outcome().is_some(),
    });
    Ok(scene)
}
// ANCHOR_END: scene

/// The game's tools: the engine's, and the board's on top.
#[derive(Debug, Clone, Copy)]
pub struct Tools {
    game: BasicGame<Action, Own>,
}

impl Tools {
    /// The tools of `game`.
    pub fn new(game: BasicGame<Action, Own>) -> Self {
        Self { game }
    }

    /// The engine's tools for the game.
    fn basic(&self) -> BasicTools<Action, Own> {
        BasicTools::new(self.game)
    }

    /// The puzzle of level `level`, counted from 0.
    fn puzzle(&self, level: usize) -> Result<Puzzle, String> {
        let data = self.game.data().map_err(|errors| {
            let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
            lines.join("\n")
        })?;
        let file = data.own.levels.get(level);
        Puzzle::new(file.ok_or(format!("no level {level}"))?)
    }
}

impl GameTools for Tools {
    fn frame(
        &self,
        options: &[GameArg],
        script: &Script,
        scale: Option<u32>,
    ) -> Result<Shot, String> {
        self.basic().frame(options, script, scale)
    }

    fn frame_options(&self) -> &[GameOption] {
        &[]
    }

    fn lang(&self) -> Result<GameLang, String> {
        self.basic().lang()
    }

    fn check(&self) -> Result<Vec<ContentError>, String> {
        self.basic().check()
    }

    fn session(&self, _: &[GameArg]) -> Result<Box<dyn Session>, String> {
        let session = Headless::new(self.game.harness())
            .with_scene(scene)
            .with_picture(|h, scale| BasicTools::<Action, Own>::picture(h, scale.unwrap_or(1)));
        Ok(Box::new(session))
    }

    fn replay(&self, record: &Record) -> Result<ReplayRun, String> {
        let parsed: LevelRecord =
            ron::from_str(&record.text).map_err(|e| format!("{}: {e}", record.file))?;
        let mut run = turns(&record.file, self.puzzle(parsed.level)?, &parsed.commands)?;
        run.made = parsed.made;
        Ok(run)
    }

    /// [`crate::made`]: `replay` refuses a record another build made.
    fn made(&self) -> Option<Made> {
        Some(made())
    }

    // ANCHOR: bots
    /// Each play is level 1's line: the same every time, as the rules have
    /// no luck.
    fn bots(&self, input: &BotsInput) -> Result<BotsOutput, String> {
        let bundle = self.game.bundle();
        let plays = (input.seed..input.seed + u64::from(input.games))
            .map(|seed| {
                let mut puzzle = self.puzzle(0)?;
                let mut bot = LineBot::new(&bundle, 0)?;
                let report = play(&mut puzzle, &mut [((), &mut bot)], input.turns)
                    .map_err(|e| e.to_string())?;
                let outcome = if report.outcome.is_some() {
                    "won"
                } else {
                    UNFINISHED
                };
                let commands = u32::try_from(report.commands.len()).unwrap_or(u32::MAX);
                Ok(Play::new(seed, outcome, report.turns, commands))
            })
            .collect::<Result<_, String>>()?;
        Ok(tally(plays))
    }
    // ANCHOR_END: bots

    /// Its keymap and every screen it can show.
    fn controls(&self) -> Option<Result<Box<dyn Controls>, String>> {
        self.basic().controls()
    }
}
