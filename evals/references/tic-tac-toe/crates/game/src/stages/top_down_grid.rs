//! The stage: tic-tac-toe ([`Board`]) for two players on one keyboard or
//! pad. The four directions move the cursor, which starts on the middle
//! cell and stops at the edges; Confirm marks the cell under it for the
//! side to move. Once the game is over the board stays, and Confirm starts
//! the next one; Cancel goes back to the title.
//!
//! Its part of the game's data is [`Own`]: the next game's seed, which a
//! record keeps. What it shows is a [`StageView`] (plain data) that
//! [`StageScreen::draw`] paints with glyphs. Its play is a
//! [`StageRecord`], which its tools' `replay` reads and a crash report
//! holds ([`crash_record`]). Its tools ([`Tools`]) add its scene, records
//! and bot run to the engine's.

#[cfg(feature = "tools")]
mod tools;

use std::any::Any;

use rules::{Board, Command, Event, Mark, Outcome, SIZE};
use serde::{Deserialize, Serialize};
use shuyi::core::{Made, Pos, Sides, StateHash, TurnRules, state_hash};
use shuyi::prelude::*;
use shuyi::ui::text::StyledText;

use crate::Data;

#[cfg(feature = "tools")]
pub use tools::{OPTIONS, Tools, playtest_plan, seed_of};

/// A record of a game: its seed and the commands sent to its rules, and
/// what made it. The game's record format for `replay`; `playtest` keeps
/// one of each try, and a crash report holds the open game's
/// ([`crash_record`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StageRecord {
    /// The game's seed.
    pub seed: u64,
    /// The commands, in order.
    pub commands: Vec<Command>,
    /// What made it ([`crate::made`]); a record without one still reads.
    #[serde(default)]
    pub made: Option<Made>,
}

/// The game's record of its play for a crash report: the open game's
/// [`StageRecord`] as RON, or `None` when no game is open.
pub fn crash_record(game: &Game<Data>) -> Option<String> {
    ron::to_string(&game.screen::<StageScreen>()?.record()).ok()
}

/// The stage's part of the game's data.
#[derive(Debug)]
pub struct Own {
    /// The seed of the next game, which its record keeps.
    pub seed: u64,
}

impl Own {
    /// The seed of a new game: each game the next one.
    pub fn next_seed(&mut self) -> u64 {
        let seed = self.seed;
        self.seed = self.seed.wrapping_add(1);
        seed
    }
}

/// The stage's part of the game's data: nothing read from its files.
pub fn own(_bundle: &Bundle) -> Result<Own, Vec<ContentError>> {
    Ok(Own { seed: 0 })
}

/// The stage, as the title starts it.
pub fn first(ctx: &mut Ctx<Data>) -> Box<dyn Screen<Data>> {
    Box::new(StageScreen::new(ctx))
}

/// Every screen the stage can show.
pub fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    vec![first(ctx)]
}

/// A mark's letter on the board.
pub const fn letter(mark: Mark) -> char {
    match mark {
        Mark::X => 'X',
        Mark::O => 'O',
    }
}

/// What the stage shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageView {
    /// The board's rows, a letter or a space a cell.
    pub rows: Vec<String>,
    /// The cell under the cursor.
    pub cursor: Pos,
    /// Whose move it is, or how the game ended.
    pub message: String,
    /// The help line, its keys in key spans.
    pub help: StyledText,
}

/// The stage screen.
#[derive(Debug)]
pub struct StageScreen {
    /// The rules' state.
    pub board: Board,
    /// The cell under the cursor.
    pub cursor: Pos,
    /// Its seed.
    pub seed: u64,
    /// Every command the rules took, in order.
    pub commands: Vec<Command>,
    /// The command the rules are applying, until they return: a crash
    /// inside them is in its [`record`](Self::record).
    pub pending: Option<Command>,
    /// Every event of the rules, in order.
    pub events: Vec<Event>,
}

impl StageScreen {
    /// A new game, its seed the game's next.
    pub fn new(ctx: &mut Ctx<Data>) -> Self {
        Self {
            board: Board::new(),
            cursor: Pos::new(SIZE / 2, SIZE / 2),
            seed: ctx.game.own.next_seed(),
            commands: Vec::new(),
            pending: None,
            events: Vec::new(),
        }
    }

    /// What the stage shows.
    pub fn view(&self, ctx: &Ctx<Data>) -> StageView {
        let rows = (0..SIZE)
            .map(|y| {
                (0..SIZE)
                    .map(|x| self.board.at(Pos::new(x, y)).map_or(' ', letter))
                    .collect()
            })
            .collect();
        let side = |mark: Mark| letter(mark).to_string();
        let (message, help) = match self.board.outcome() {
            None => (
                ctx.text_with(
                    "stage.to_move",
                    &[("mark", &side(self.board.side_to_move()))],
                ),
                "stage.help",
            ),
            Some(Outcome::Won { mark, .. }) => (
                ctx.text_with("stage.won", &[("mark", &side(mark))]),
                "stage.help.over",
            ),
            Some(Outcome::Draw { .. }) => (ctx.text("stage.draw").to_owned(), "stage.help.over"),
        };
        StageView {
            rows,
            cursor: self.cursor,
            message,
            help: ctx.text_styled(help, &[]),
        }
    }

    /// Its play so far: the seed, the commands taken and the one pending,
    /// stamped with what made it.
    pub fn record(&self) -> StageRecord {
        StageRecord {
            seed: self.seed,
            commands: self.commands.iter().chain(&self.pending).copied().collect(),
            made: Some(crate::made()),
        }
    }

    /// Sends `command` to the rules and keeps it, if they take it.
    fn send(&mut self, command: Command) {
        self.pending = Some(command);
        let applied = self.board.apply(&command);
        self.pending = None;
        if let Ok(events) = applied {
            self.commands.push(command);
            self.events.extend(events);
        }
    }

    /// Moves the cursor by `(dx, dy)`, stopping at the edges.
    fn step(&mut self, dx: i32, dy: i32) {
        let at = |v: i32| v.clamp(0, SIZE - 1);
        self.cursor = Pos::new(at(self.cursor.x + dx), at(self.cursor.y + dy));
    }
}

impl Screen<Data> for StageScreen {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "stage"
    }

    /// The cursor's four moves, Confirm to mark and Cancel to go back.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(UiAction::ALL.to_vec())
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        for &action in &input.actions {
            match action {
                UiAction::Up => self.step(0, -1),
                UiAction::Down => self.step(0, 1),
                UiAction::Left => self.step(-1, 0),
                UiAction::Right => self.step(1, 0),
                UiAction::Confirm if self.board.outcome().is_some() => {
                    return Transition::Replace(Box::new(StageScreen::new(ctx)));
                }
                UiAction::Confirm => self.send(Command::Place(self.cursor)),
                UiAction::Cancel => return Transition::Pop,
                _ => {}
            }
        }
        Transition::None
    }

    /// The message on the top row, the board of three cells by three in
    /// the middle, the cursor's cell highlighted, and the help on the last
    /// row.
    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |e| ctx.palette.engine(e);
        let bg = c(EngineColor::Background);
        buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
        let view = self.view(ctx);
        buf.print(1, 0, &view.message, c(EngineColor::Highlight), bg);
        // Each cell three wide, with a line between cells.
        let left = (i32::from(buf.width()) - 11) / 2;
        let top = (i32::from(buf.height()) - 5) / 2;
        for (y, row) in (0..).zip(&view.rows) {
            for (x, mark) in (0..).zip(row.chars()) {
                let on = Pos::new(x, y) == view.cursor;
                let cell_bg = if on { c(EngineColor::Focus) } else { bg };
                let text = format!(" {mark} ");
                buf.print(
                    left + x * 4,
                    top + y * 2,
                    &text,
                    c(EngineColor::Bright),
                    cell_bg,
                );
                if x < SIZE - 1 {
                    buf.print(
                        left + x * 4 + 3,
                        top + y * 2,
                        "│",
                        c(EngineColor::Border),
                        bg,
                    );
                }
            }
            if y < SIZE - 1 {
                // check-text: not player text, the board's lines
                let line = "───┼───┼───";
                buf.print(left, top + y * 2 + 1, line, c(EngineColor::Border), bg);
            }
        }
        let last = i32::from(buf.height()) - 1;
        let colors = ctx.palette.text_colors_on(c(EngineColor::TextDim), bg);
        buf.print_styled(1, last, &view.help, colors);
    }

    /// The rules' state, in each frame's hash in a session record.
    fn state_hash(&self) -> Option<StateHash> {
        state_hash(&self.board).ok()
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}
