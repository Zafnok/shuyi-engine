//! The board: a level on the screen. What it shows is a [`MapScene`] the
//! engine's glyph skin paints, and a [`BoardView`] of the lines round it.

use std::any::Any;

use shuyi::core::History;
use shuyi::core::Pos;
use shuyi::core::{Dir, TerrainId, TurnRules};
use shuyi::prelude::*;
use shuyi::ui::frame::Palette;
use shuyi::ui::grid::{
    Camera, GlyphSkin, Grid, GridSkin, Looks, MapColors, MapScene, MapUnit, TerrainGlyphs,
    Terrains, UnitPainter, WalkSpeeds, glyph,
};

use crate::Data;
use crate::action::Action;
use crate::level::TERRAIN_NAMES;
use crate::rules::{Command, Puzzle};

// ANCHOR: skin
/// The cells a level is shown in: all but the top and bottom rows.
pub const AREA: Rect = Rect::new(0, 1, 48, 22);

/// Each terrain's two glyphs and colors, by [`TerrainId`]: floor, wall,
/// target.
pub const TERRAIN_LOOKS: [([char; 2], &str, &str); 3] = [
    ([' ', ' '], "floor_fg", "floor_bg"),
    (['▓', '▓'], "wall_fg", "wall_bg"),
    (['<', '>'], "target_fg", "floor_bg"),
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

/// The glyph skin: two cells a tile.
// check-text: not player text, the palette's names
pub const GLYPHS: GlyphSkin = GlyphSkin::new(
    MapColors::new("path", "cursor", "threat"),
    WalkSpeeds::of(8.0),
    0,
);

/// Which piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    /// The player.
    Player,
    /// The box with this number.
    Box(usize),
}

/// A piece on the map, as the scene holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// Which piece.
    pub who: Who,
    /// The tile it is on.
    pub pos: Pos,
    /// Whether it is a box on a target.
    pub home: bool,
}

impl MapUnit for Piece {
    type Id = Who;

    fn id(&self) -> Who {
        self.who
    }

    fn pos(&self) -> Pos {
        self.pos
    }

    fn text(&self) -> String {
        let letter = if self.who == Who::Player { '@' } else { '#' };
        format!("{letter} ({},{})", self.pos.x, self.pos.y)
    }
}

/// The pieces under the glyph skin: the player's letter over its tile, and
/// a box over both of its tile's cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pieces<'a> {
    /// The palette the colors are named in.
    pub palette: &'a Palette,
}

impl UnitPainter<Piece> for Pieces<'_> {
    fn paint(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
        for piece in &scene.units {
            let (glyphs, color) = match (piece.who, piece.home) {
                (Who::Player, _) => (['@', ' '], "player"),
                (Who::Box(_), false) => (['[', ']'], "box"),
                (Who::Box(_), true) => (['[', ']'], "box_home"),
            };
            let fg = self
                .palette
                .lookup(color)
                .unwrap_or_else(|| self.palette.engine(EngineColor::Text));
            let Some((x, y)) = glyph::cell_of(grid, piece.pos) else {
                continue;
            };
            for (x, glyph) in (x..).zip(glyphs) {
                if glyph != ' '
                    && let Some(&cell) = buf.get(x, y)
                {
                    buf.set(x, y, Cell { glyph, fg, ..cell });
                }
            }
        }
    }
}
// ANCHOR_END: skin

/// What the lines round the map show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardView {
    /// The level's name.
    pub name: String,
    /// The help line.
    pub help: String,
}

// ANCHOR: board
/// The board screen.
#[derive(Debug)]
pub struct Board {
    /// The level's name.
    name: String,
    /// The rules' state.
    pub puzzle: Puzzle,
    /// The level's first state and every command since.
    pub history: History<Puzzle>,
}
// ANCHOR_END: board

impl Board {
    /// The board on the game's level.
    ///
    /// # Panics
    ///
    /// If the level makes no puzzle, which loading it rules out.
    pub fn new(ctx: &Ctx<Data>) -> Self {
        let file = &ctx.game.own.level;
        let puzzle = Puzzle::new(file).unwrap_or_else(|e| panic!("{e}"));
        Self {
            name: file.name.clone(),
            history: History::new(puzzle.clone()),
            puzzle,
        }
    }

    /// What the map shows, in a view of `view` tiles round the player.
    pub fn scene(&self, view: (i32, i32)) -> MapScene<Piece> {
        let tiles = self.puzzle.tiles();
        let player = self.puzzle.player();
        let camera = Camera::centred_on(player, tiles.width(), tiles.height(), view);
        let mut scene = MapScene::new(camera.origin, view);
        scene.set_terrain(|pos| tiles.get(pos).copied());
        let targets = self.puzzle.targets();
        for (n, &pos) in self.puzzle.boxes().iter().enumerate() {
            let home = targets.contains(&pos);
            let who = Who::Box(n);
            scene.push_unit(Piece { who, pos, home });
        }
        let (who, pos, home) = (Who::Player, player, false);
        scene.push_unit(Piece { who, pos, home });
        scene
    }

    /// What the lines round the map show.
    pub fn view(&self, ctx: &Ctx<Data>) -> BoardView {
        // ANCHOR: won_view
        let help = if self.puzzle.outcome().is_some() {
            "board.help.won"
        } else {
            "board.help"
        };
        // ANCHOR_END: won_view
        BoardView {
            name: self.name.clone(),
            help: ctx.text_with(help, &[]),
        }
    }

    // ANCHOR: send
    /// Sends `command` to the rules. A refused one changes nothing.
    fn send(&mut self, command: Command) {
        if self.puzzle.apply(&command).is_ok() {
            self.history.push(command);
        }
    }
    // ANCHOR_END: send

    // ANCHOR: undo
    /// Takes back the last move, if there is one.
    fn undo(&mut self) {
        if let Some(last) = self.history.len().checked_sub(1)
            && let Ok(puzzle) = self.history.rewind_to(last)
        {
            self.puzzle = puzzle;
        }
    }
    // ANCHOR_END: undo

    /// The tiles the glyph skin fits in [`AREA`].
    pub fn view_tiles() -> (i32, i32) {
        GridSkin::<Piece>::view_tiles(&GLYPHS, AREA)
    }
}

impl Screen<Data> for Board {
    // check-text: not player text, the screen's id
    fn name(&self) -> &'static str {
        "board"
    }

    // ANCHOR: actions
    /// Every action of the game.
    fn actions(&self) -> Option<Vec<Action>> {
        Some(Action::ALL.to_vec())
    }
    // ANCHOR_END: actions

    // ANCHOR: update
    fn update(&mut self, _: &mut Ctx<Data>, input: &FrameInput<Action>) -> Transition<Data> {
        for &action in &input.actions {
            match action {
                Action::Up => self.send(Command::Step(Dir::Up)),
                Action::Down => self.send(Command::Step(Dir::Down)),
                Action::Left => self.send(Command::Step(Dir::Left)),
                Action::Right => self.send(Command::Step(Dir::Right)),
                Action::Undo => self.undo(),
                Action::Cancel => return Transition::Pop,
                Action::Confirm => {}
            }
        }
        Transition::None
    }
    // ANCHOR_END: update

    // ANCHOR: draw
    fn draw(&self, ctx: &Ctx<Data>, buf: &mut GlyphBuffer) {
        let palette = &ctx.palette;
        let c = |e| palette.engine(e);
        let bg = c(EngineColor::Panel);
        buf.fill_rect(buf.bounds(), Cell::new(' ', bg, bg));
        let view = self.view(ctx);
        buf.print(1, 0, &view.name, c(EngineColor::Highlight), bg);
        let last = i32::from(buf.height()) - 1;
        buf.print(1, last, &view.help, c(EngineColor::TextDim), bg);
        let looks = Looks::new(palette, &Terrain, &ctx.game.own.lights);
        let scene = self.scene(Self::view_tiles());
        GLYPHS.paint(&looks, &scene, AREA, &Pieces { palette }, buf);
    }
    // ANCHOR_END: draw

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
