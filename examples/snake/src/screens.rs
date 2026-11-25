//! The game's screens: the [`Title`], the board ([`Stage`]), the [`Pause`]
//! menu over it, and the end of a run ([`Over`]). The stage steps the rules
//! once for each of the frame's steps through the engine's [`TickDriver`],
//! which records them; what it shows is a [`MapScene`] of the board, the
//! snake and the food, painted by the grid kit's glyph skin ([`GLYPHS`]).
//! Tests of what happened read the rules' state or the scene.

use std::any::Any;

use shuyi::assets::lighting::Lighting;
use shuyi::core::tick::TickRecord;
use shuyi::core::{Dir, Pos, StateHash, TerrainId, TickSim};
use shuyi::prelude::*;
use shuyi::ui::frame::Palette;
use shuyi::ui::grid::{
    GlyphSkin, Grid, GridSkin, Looks, MapColors, MapScene, MapUnit, TerrainGlyphs, Terrains,
    UnitPainter, WalkSpeeds, glyph,
};
use shuyi::ui::screens::print_centred_styled;
use shuyi::ui::tick::TickDriver;

use crate::Data;
use crate::rules::{Event, Outcome, Snake, TERRAIN_NAMES};

/// The sound the snake makes as it eats.
// check-text: not player text, a sound's id
pub const EAT: &str = "eat";
/// The sound of the end of a run.
// check-text: not player text, a sound's id
pub const DIE: &str = "die";

/// Every screen the game can show, overlays on their own.
pub fn every(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    vec![
        Box::new(Title),
        Box::new(Stage::new(ctx)),
        Box::new(Pause::default()),
        Box::new(Over::new(Outcome::Filled, 0, 0)),
    ]
}

/// The title: the game's name, how to play, and the help.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Title;

impl Screen<Data> for Title {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "title"
    }

    /// Confirm to play, Cancel to leave.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm, UiAction::Cancel])
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if ctx.platform.can_quit() && input.actions.contains(&UiAction::Cancel) {
            return Transition::Quit;
        }
        if input.actions.contains(&UiAction::Confirm) {
            return Transition::Push(Box::new(Stage::new(ctx)));
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |e| ctx.palette.engine(e);
        let bg = c(EngineColor::Background);
        buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
        let title = ctx.text("title.name");
        let width = i32::try_from(title.chars().count()).unwrap_or(0) + 6;
        let x = (i32::from(buf.width()) - width).max(0) / 2;
        buf.draw_box(
            Rect::new(x, 3, width, 3),
            BoxStyle::Double,
            c(EngineColor::Border),
            bg,
        );
        print_centred(buf, 4, title, c(EngineColor::Highlight), bg);
        let subtitle = ctx.text("title.subtitle");
        print_centred(buf, 7, subtitle, c(EngineColor::TextDim), bg);
        let colors = ctx.palette.text_colors_on(c(EngineColor::Text), bg);
        print_centred_styled(buf, 10, &ctx.text_styled("title.play", &[]), colors);
        let dim = || ctx.palette.text_colors_on(c(EngineColor::TextDim), bg);
        print_centred_styled(buf, 12, &ctx.text_styled("title.steer", &[]), dim());
        if ctx.platform.can_quit() {
            let last = i32::from(buf.height()) - 1;
            print_centred_styled(buf, last, &ctx.text_styled("title.help", &[]), dim());
        }
    }
}

/// The cells the board is shown in: all but the score line.
pub const AREA: Rect = Rect::new(0, 1, 50, 14);

/// Each terrain's two glyphs and colors, by [`TerrainId`]: floor, wall.
pub const TERRAIN_LOOKS: [([char; 2], &str, &str); 2] = [
    (['·', ' '], "floor_fg", "floor_bg"),
    (['▓', '▓'], "wall_fg", "wall_bg"),
];

/// The terrains as the skin sees them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Terrain;

impl Terrains for Terrain {
    fn name(&self, id: TerrainId) -> Option<&str> {
        TERRAIN_NAMES.get(usize::from(id.0)).copied()
    }

    fn glyphs(&self, id: TerrainId) -> Option<TerrainGlyphs<'_>> {
        let &(glyphs, fg, bg) = TERRAIN_LOOKS.get(usize::from(id.0))?;
        Some(TerrainGlyphs::new(glyphs, fg, bg))
    }
}

/// The glyph skin: two cells a tile. The board has no paths, cursor or
/// threats, so those colours are never drawn.
// check-text: not player text, the palette's names
pub const GLYPHS: GlyphSkin = GlyphSkin::new(
    MapColors::new("body", "head", "food"),
    WalkSpeeds::of(8.0),
    0,
);

/// What a unit of the board is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The snake's head, heading this way.
    Head(Dir),
    /// A tile of its body, counted from the head.
    Body(u16),
    /// The food.
    Food,
}

impl Part {
    /// Its two glyphs, and its color's name in the palette.
    pub const fn look(self) -> ([char; 2], &'static str) {
        match self {
            Part::Head(Dir::Right) => (['►', '►'], "head"),
            Part::Head(Dir::Left) => (['◄', '◄'], "head"),
            Part::Head(Dir::Up) => (['▲', '▲'], "head"),
            Part::Head(Dir::Down) => (['▼', '▼'], "head"),
            Part::Body(_) => (['(', ')'], "body"),
            Part::Food => (['<', '>'], "food"),
        }
    }
}

/// A unit on the board, as the scene holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// What it is.
    pub part: Part,
    /// Its tile.
    pub pos: Pos,
}

impl MapUnit for Piece {
    type Id = Part;

    fn id(&self) -> Part {
        self.part
    }

    fn pos(&self) -> Pos {
        self.pos
    }

    fn text(&self) -> String {
        let ([a, b], _) = self.part.look();
        format!("{a}{b} ({},{})", self.pos.x, self.pos.y)
    }
}

/// The units under the glyph skin: each its two glyphs, in its color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyphs<'a> {
    /// The palette the colors are named in.
    pub palette: &'a Palette,
}

impl UnitPainter<Piece> for Glyphs<'_> {
    fn paint(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
        for piece in &scene.units {
            let (glyphs, color) = piece.part.look();
            let fg = self
                .palette
                .lookup(color)
                .unwrap_or_else(|| self.palette.engine(EngineColor::Text));
            let Some((x, y)) = glyph::cell_of(grid, piece.pos) else {
                continue;
            };
            for (dx, glyph) in (0..).zip(glyphs) {
                if let Some(&cell) = buf.get(x + dx, y) {
                    buf.set(x + dx, y, Cell { glyph, fg, ..cell });
                }
            }
        }
    }
}

/// What the board shows of `snake`: its tiles, the snake and the food.
pub fn scene(snake: &Snake) -> MapScene<Piece> {
    let board = snake.board();
    let size = (i32::from(board.width()), i32::from(board.height()));
    let mut scene = MapScene::new(Pos::new(0, 0), size);
    scene.set_terrain(|pos| board.get(pos).copied());
    if let Some(food) = snake.food() {
        scene.push_unit(Piece {
            part: Part::Food,
            pos: food,
        });
    }
    let (head, body) = snake.body().split_at(1);
    for (n, &pos) in (1..).zip(body) {
        scene.push_unit(Piece {
            part: Part::Body(n),
            pos,
        });
    }
    scene.push_unit(Piece {
        part: Part::Head(snake.heading()),
        pos: head[0],
    });
    scene
}

/// The board: a run of the rules, stepped by the driver.
#[derive(Debug)]
pub struct Stage {
    /// The rules' state.
    pub snake: Snake,
    /// The seed its luck started from.
    pub seed: u64,
    /// What steps the rules, recording each step.
    pub driver: TickDriver<UiAction>,
    /// Every event of the rules, in order.
    pub events: Vec<Event>,
}

impl Stage {
    /// A new run, its luck from the game's next seed.
    pub fn new(ctx: &mut Ctx<Data>) -> Self {
        let (seed, snake) = ctx.game.own.next_run();
        Self {
            snake,
            seed,
            driver: TickDriver::new().recording(TickRecord::new(crate::made(), seed)),
            events: Vec::new(),
        }
    }

    /// What each step saw so far.
    pub fn record(&self) -> &TickRecord {
        // Made recording, and never taken.
        self.driver
            .record()
            .unwrap_or_else(|| panic!("the stage's driver records"))
    }
}

impl Screen<Data> for Stage {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "stage"
    }

    /// The four turns, and Cancel for the pause menu.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![
            UiAction::Up,
            UiAction::Down,
            UiAction::Left,
            UiAction::Right,
            UiAction::Cancel,
        ])
    }

    fn update(&mut self, ctx: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        for (_, event) in self.driver.update(&mut self.snake, input) {
            match event {
                Event::Ate { .. } => ctx.audio.play_sound(EAT),
                Event::Died => ctx.audio.play_sound(DIE),
                Event::Turned { .. } | Event::Moved { .. } | Event::Filled => {}
            }
            self.events.push(event);
        }
        if let Some(outcome) = self.snake.outcome() {
            let over = Over::new(outcome, self.snake.score(), self.snake.length());
            return Transition::Push(Box::new(over));
        }
        // Back from a suspend, the run waits in the menu.
        if input.resumed() || input.actions.contains(&UiAction::Cancel) {
            return Transition::Push(Box::new(Pause::default()));
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let palette = &ctx.palette;
        let c = |e| palette.engine(e);
        let bg = c(EngineColor::Panel);
        buf.fill_rect(
            Rect::new(0, 0, i32::from(buf.width()), 1),
            Cell::new(' ', bg, bg),
        );
        let score = self.snake.score();
        let score = ctx.text_with("stage.score", &[("score", &score)]);
        buf.print(1, 0, &score, c(EngineColor::Highlight), bg);
        let length = self.snake.length();
        let length = ctx.text_with("stage.length", &[("length", &length)]);
        let width = i32::try_from(length.chars().count()).unwrap_or(0);
        let x = i32::from(buf.width()) - width - 1;
        buf.print(x, 0, &length, c(EngineColor::Text), bg);
        let lights = Lighting::default();
        let looks = Looks::new(palette, &Terrain, &lights);
        GLYPHS.paint(&looks, &scene(&self.snake), AREA, &Glyphs { palette }, buf);
    }

    /// The rules' state, in each frame's hash in a session record.
    fn state_hash(&self) -> Option<StateHash> {
        Some(StateHash(self.snake.hash()))
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }

    /// A turn pressed for a step that hasn't run never fires after the
    /// menu.
    fn covered(&mut self, _: &mut Ctx<Data>) {
        self.driver.clear();
    }

    /// What the menu or the end of the run chose: a new run, the title, or
    /// back to this one.
    fn uncovered(&mut self, ctx: &mut Ctx<Data>, result: Option<ScreenResult>) -> Transition<Data> {
        match result.and_then(ScreenResult::take::<Then>) {
            Some(Then::Again) => Transition::Replace(Box::new(Stage::new(ctx))),
            Some(Then::Title) => Transition::Pop,
            Some(Then::Resume) | None => Transition::None,
        }
    }
}

/// What a screen over the board chose, which the stage reads as it is
/// uncovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    /// Back to the run.
    Resume,
    /// A new run.
    Again,
    /// Back to the title.
    Title,
}

/// A box in the middle of the console, `rows` tall inside its border,
/// cleared.
fn dialog(ctx: &Ctx<Data>, buf: &mut GlyphBuffer, rows: i32) -> i32 {
    let c = |e| ctx.palette.engine(e);
    let bg = c(EngineColor::Panel);
    let (width, height) = (36, rows + 2);
    let x = (i32::from(buf.width()) - width) / 2;
    let y = (i32::from(buf.height()) - height) / 2;
    let area = Rect::new(x, y, width, height);
    buf.fill_rect(area, Cell::new(' ', bg, bg));
    buf.draw_box(area, BoxStyle::Single, c(EngineColor::Border), bg);
    y + 1
}

/// The pause menu over the board: Resume or back to the title.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pause {
    /// The item in focus: 0 Resume, 1 back to the title.
    pub focus: usize,
}

/// The pause menu's items, by key.
const PAUSE_ITEMS: [&str; 2] = ["pause.resume", "pause.quit"];

impl Screen<Data> for Pause {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "pause"
    }

    fn is_overlay(&self) -> bool {
        true
    }

    /// Up and Down between the items, Confirm to choose, Cancel to resume.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![
            UiAction::Up,
            UiAction::Down,
            UiAction::Confirm,
            UiAction::Cancel,
        ])
    }

    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        for action in &input.actions {
            match action {
                UiAction::Up | UiAction::Down => self.focus = 1 - self.focus,
                UiAction::Confirm if self.focus == 1 => return Transition::pop_with(Then::Title),
                UiAction::Confirm | UiAction::Cancel => return Transition::pop_with(Then::Resume),
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |e| ctx.palette.engine(e);
        let bg = c(EngineColor::Panel);
        let top = dialog(ctx, buf, 5);
        print_centred(
            buf,
            top,
            ctx.text("pause.title"),
            c(EngineColor::Highlight),
            bg,
        );
        for (row, (i, key)) in (top + 2..).zip(PAUSE_ITEMS.iter().enumerate()) {
            let (fg, back) = if i == self.focus {
                (bg, c(EngineColor::Focus))
            } else {
                (c(EngineColor::Text), bg)
            };
            print_centred(buf, row, ctx.text(key), fg, back);
        }
        let dim = ctx.palette.text_colors_on(c(EngineColor::TextDim), bg);
        print_centred_styled(buf, top + 4, &ctx.text_styled("pause.help", &[]), dim);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}

/// The end of a run, over the board: how it ended and the score, Confirm
/// to play again and Cancel for the title.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Over {
    /// How it ended.
    pub outcome: Outcome,
    /// The food eaten.
    pub score: u32,
    /// The snake's tiles.
    pub length: u32,
}

impl Over {
    /// The end of a run that ended so, with that score and length.
    pub fn new(outcome: Outcome, score: u32, length: u32) -> Self {
        Self {
            outcome,
            score,
            length,
        }
    }
}

impl Screen<Data> for Over {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "over"
    }

    fn is_overlay(&self) -> bool {
        true
    }

    /// Confirm to play again, Cancel for the title.
    fn actions(&self) -> Option<Vec<UiAction>> {
        Some(vec![UiAction::Confirm, UiAction::Cancel])
    }

    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<UiAction>) -> Transition<Data> {
        if input.actions.contains(&UiAction::Confirm) {
            return Transition::pop_with(Then::Again);
        }
        if input.actions.contains(&UiAction::Cancel) {
            return Transition::pop_with(Then::Title);
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let c = |e| ctx.palette.engine(e);
        let bg = c(EngineColor::Panel);
        let top = dialog(ctx, buf, 5);
        let title = match self.outcome {
            Outcome::Died { .. } => "over.died",
            Outcome::Filled => "over.filled",
        };
        print_centred(buf, top, ctx.text(title), c(EngineColor::Highlight), bg);
        let (score, length) = (self.score, self.length);
        let line = ctx.text_with("over.score", &[("score", &score), ("length", &length)]);
        print_centred(buf, top + 2, &line, c(EngineColor::Text), bg);
        let dim = ctx.palette.text_colors_on(c(EngineColor::TextDim), bg);
        print_centred_styled(buf, top + 4, &ctx.text_styled("over.help", &[]), dim);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
