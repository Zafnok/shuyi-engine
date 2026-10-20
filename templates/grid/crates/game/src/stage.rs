//! The stage, the game's one screen: the rules ([`Stage`]) played with the
//! engine's grid kit. Each direction is a step of your piece, then the
//! wanderer's; once the stage is over, Confirm starts the next one and
//! Cancel leaves the game.
//!
//! What it shows is a [`MapScene`] that the engine's glyph skin paints
//! ([`GLYPHS`]), and a [`StageView`] of the lines round it. Its look is
//! here too: the terrains' glyphs and colors, and the pieces' letters.

use std::any::Any;

use rules::{Command, Event, Outcome, Stage, TERRAIN_NAMES};
use shuyi_assets::keymap::{ActionSet, UiAction};
use shuyi_core::{Dir, Pos, Sides, TerrainId, TurnRules};
use shuyi_ui::frame::glyph_buffer::{Cell, GlyphBuffer, Rect};
use shuyi_ui::frame::{EngineColor, Palette};
use shuyi_ui::grid::{
    Camera, GlyphSkin, Grid, GridSkin, Looks, MapColors, MapScene, MapUnit, TerrainGlyphs,
    Terrains, UnitPainter, WalkSpeeds, glyph,
};
use shuyi_ui::{Ctx, FrameInput, Screen, Transition};

use crate::{Action, Data};

/// The game's context.
pub type GameCtx = Ctx<Data>;

/// The cells the map is shown in: all but the top and bottom rows.
pub const AREA: Rect = Rect::new(0, 1, 48, 22);

/// Each terrain's two glyphs and colors, by [`TerrainId`]: floor, wall,
/// goal.
pub const TERRAIN_LOOKS: [([char; 2], &str, &str); 3] = [
    (['.', ' '], "floor_fg", "floor_bg"),
    (['▓', '▓'], "wall_fg", "wall_bg"),
    (['[', ']'], "goal_fg", "goal_bg"),
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
        Some(TerrainGlyphs { glyphs, fg, bg })
    }
}

/// The glyph skin: two cells a tile.
pub const GLYPHS: GlyphSkin = GlyphSkin {
    colors: MapColors {
        path: "path",
        cursor: "cursor",
        threat: "threat",
    },
    walk: WalkSpeeds::of(8.0),
    foot_px: 0,
};

/// Which piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Who {
    /// Yours.
    Hero,
    /// The wanderer.
    Wanderer,
}

impl Who {
    /// Its letter, and its color's name in the palette.
    pub const fn look(self) -> (char, &'static str) {
        match self {
            Who::Hero => ('@', "hero"),
            Who::Wanderer => ('&', "wanderer"),
        }
    }
}

/// A piece on the map, as the scene holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    /// Which piece.
    pub who: Who,
    /// The tile it is on.
    pub pos: Pos,
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
        let (letter, _) = self.who.look();
        format!("{letter} ({},{})", self.pos.x, self.pos.y)
    }
}

/// The pieces under the glyph skin: each its letter, in its color, over
/// its tile's left cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Letters<'a> {
    /// The palette the colors are named in.
    pub palette: &'a Palette,
}

impl UnitPainter<Piece> for Letters<'_> {
    fn paint(&self, scene: &MapScene<Piece>, grid: &Grid, buf: &mut GlyphBuffer) {
        for piece in &scene.units {
            let (letter, color) = piece.who.look();
            let fg = self
                .palette
                .lookup(color)
                .unwrap_or_else(|| self.palette.engine(EngineColor::Text));
            if let Some((x, y)) = glyph::cell_of(grid, piece.pos)
                && let Some(&cell) = buf.get(x, y)
            {
                buf.set(
                    x,
                    y,
                    Cell {
                        glyph: letter,
                        fg,
                        ..cell
                    },
                );
            }
        }
    }
}

/// What the lines round the map show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageView {
    /// The turn.
    pub turn: String,
    /// What is happening.
    pub message: String,
    /// The help line.
    pub help: String,
}

/// The stage screen.
#[derive(Debug)]
pub struct StageScreen {
    /// The rules' state.
    pub stage: Stage,
    /// Its seed.
    pub seed: u64,
    /// Every command sent to the rules, in order.
    pub commands: Vec<Command>,
    /// Every event of the rules, in order.
    pub events: Vec<Event>,
}

impl StageScreen {
    /// A new stage, its luck from the game's next seed.
    ///
    /// # Panics
    ///
    /// If the game's map makes no stage, which loading it rules out.
    pub fn new(ctx: &mut GameCtx) -> Self {
        let seed = ctx.game.next_seed();
        let stage = Stage::new(&ctx.game.stage_map, seed).unwrap_or_else(|e| panic!("{e}"));
        Self {
            stage,
            seed,
            commands: Vec::new(),
            events: Vec::new(),
        }
    }

    /// What the map shows, in a view of `view` tiles round your piece.
    pub fn scene(&self, view: (i32, i32)) -> MapScene<Piece> {
        let map = self.stage.map();
        let camera = Camera::centred_on(self.stage.hero(), map.width(), map.height(), view);
        let mut scene = MapScene::new(camera.origin, view);
        scene.set_terrain(|pos| map.get(pos).copied());
        for (who, pos) in [
            (Who::Hero, self.stage.hero()),
            (Who::Wanderer, self.stage.wanderer()),
        ] {
            scene.push_unit(Piece { who, pos });
        }
        scene
    }

    /// What the lines round the map show.
    pub fn view(&self, ctx: &GameCtx) -> StageView {
        let turn = self.stage.turn();
        let (message, help) = match self.stage.outcome() {
            None => ("stage.your_move", "stage.help"),
            Some(Outcome::Won { .. }) => ("stage.won", "stage.help.over"),
            Some(Outcome::Caught { .. }) => ("stage.caught", "stage.help.over"),
        };
        StageView {
            turn: ctx.text_with("stage.turn", &[("turn", &turn)]),
            message: ctx.text_with(message, &[("turn", &turn)]),
            help: ctx.text_with(help, &[]),
        }
    }

    /// Sends `command` to the rules and keeps it, if they take it.
    fn send(&mut self, command: Command) {
        if let Ok(events) = self.stage.apply(&command) {
            self.commands.push(command);
            self.events.extend(events);
        }
    }

    /// The tiles the glyph skin fits in [`AREA`].
    pub fn view_tiles() -> (i32, i32) {
        GridSkin::<Piece>::view_tiles(&GLYPHS, AREA)
    }
}

impl Screen<Data> for StageScreen {
    fn name(&self) -> &'static str {
        "stage"
    }

    fn update(&mut self, ctx: &mut GameCtx, input: &FrameInput<Action>) -> Transition<Data> {
        for &action in &input.actions {
            let over = self.stage.outcome().is_some();
            match action.ui() {
                Some(UiAction::Up) => self.send(Command::Step(Dir::Up)),
                Some(UiAction::Down) => self.send(Command::Step(Dir::Down)),
                Some(UiAction::Left) => self.send(Command::Step(Dir::Left)),
                Some(UiAction::Right) => self.send(Command::Step(Dir::Right)),
                Some(UiAction::Confirm) if over => {
                    return Transition::Replace(Box::new(StageScreen::new(ctx)));
                }
                Some(UiAction::Cancel) => return Transition::Quit,
                _ => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &GameCtx, buf: &mut GlyphBuffer) {
        let palette = &ctx.palette;
        let c = |e| palette.engine(e);
        let bg = c(EngineColor::Panel);
        let view = self.view(ctx);
        let shown = buf.print(1, 0, &view.turn, c(EngineColor::Highlight), bg);
        buf.print(
            1 + i32::from(shown) + 3,
            0,
            &view.message,
            c(EngineColor::Text),
            bg,
        );
        let last = i32::from(buf.height()) - 1;
        buf.print(1, last, &view.help, c(EngineColor::TextDim), bg);
        let looks = Looks {
            palette,
            terrains: &Terrain,
            lights: &ctx.game.lights,
        };
        let scene = self.scene(Self::view_tiles());
        GLYPHS.paint(&looks, &scene, AREA, &Letters { palette }, buf);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
