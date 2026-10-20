//! The stage: the rules ([`Stage`]) played with the engine's grid kit.
//! Confirm on your piece picks it up and shows where it can walk; moving
//! the cursor steers its path; Confirm on a tile of its reach sends the
//! command and shows the walk. Then the foe's mind ([`Chase`]) gives the
//! foe's command, and its step is shown. Cancel puts your piece down, or
//! asks whether to leave on a screen of its own ([`Leave`]), which pops
//! with the answer. The stage starts with its scene ([`Talk`]) over it,
//! and ends on the [`End`] screen.
//!
//! What it shows is a [`MapScene`] that the skin of the player's settings
//! paints, and a [`StageView`] of the lines round it.

use std::any::Any;

use shuyi_core::{Bot, Dir, Pos, Reach, Sides, TurnRules};
use shuyi_ui::audio::MenuSound;
use shuyi_ui::frame::glyph_buffer::{GlyphBuffer, Rect};
use shuyi_ui::grid::{Camera, Cursor, CursorStyle, CursorView, GridSkin, MapScene, steer, walk};
use shuyi_ui::{FrameInput, Screen, ScreenResult, Transition};

use shuyi_assets::keymap::{ActionSet, UiAction};

use super::{End, HelloCtx, Leave, Talk};
use crate::map::{self, GLYPHS, Letters, MOVE, Piece, Terrain, Tiles};
use crate::rules::{Chase, Command, Event, MOVES, Side, Stage};
use crate::settings::MapLook;
use crate::skin::{self, StageView};
use crate::{Action, Hello};

/// The cells the map is shown in: all but the top and bottom rows.
pub const AREA: Rect = Rect::new(0, 1, 48, 22);

/// A piece on its way along its path.
#[derive(Debug, Clone, PartialEq)]
pub struct Walk {
    /// Whose piece.
    pub side: Side,
    /// Its path, the tile it left first.
    pub path: Vec<Pos>,
    /// Seconds since it set off.
    pub t: f32,
}

/// The stage screen.
#[derive(Debug)]
pub struct StageScreen {
    /// The rules' state.
    pub stage: Stage,
    /// Every command sent to the rules, in order.
    pub commands: Vec<Command>,
    /// Every event of the rules, in order.
    pub events: Vec<Event>,
    /// The cursor.
    pub cursor: Cursor,
    /// Your piece's reach and path, while it is picked up.
    pub selected: Option<(Reach, Vec<Pos>)>,
    /// A piece being shown walking, if one is.
    pub walk: Option<Walk>,
    /// Whether the foe tripped on its last move.
    pub tripped: bool,
    /// Whether it is asking whether to leave: the [`Leave`] question is
    /// over it.
    pub leaving: bool,
    /// Whether the scene before the stage has been opened.
    briefed: bool,
}

impl StageScreen {
    /// A new stage, its luck from the game's next seed.
    ///
    /// # Panics
    ///
    /// If the game's map makes no stage, which loading it rules out.
    pub fn new(ctx: &mut HelloCtx) -> Self {
        let seed = ctx.game.next_seed();
        let stage = Stage::new(&ctx.game.stage_map, seed).unwrap_or_else(|e| panic!("{e}"));
        Self::of(stage)
    }

    /// The screen of `stage`, already briefed: no scene opens.
    pub fn of(stage: Stage) -> Self {
        Self {
            cursor: Cursor::new(stage.hero()),
            stage,
            commands: Vec::new(),
            events: Vec::new(),
            selected: None,
            walk: None,
            tripped: false,
            leaving: false,
            briefed: false,
        }
    }

    /// The same screen with its scene seen already.
    #[must_use]
    pub fn briefed(mut self) -> Self {
        self.briefed = true;
        self
    }

    /// What the map shows, in a view of `view` tiles.
    pub fn scene(&self, view: (i32, i32)) -> MapScene<Piece> {
        let map = self.stage.map();
        let camera = Camera::centred_on(self.stage.hero(), map.width(), map.height(), view);
        let mut scene = MapScene::new(camera.origin, view);
        scene.set_terrain(|pos| map.get(pos).copied());
        if let Some((reach, path)) = &self.selected {
            scene.tint(reach.tiles().iter(), MOVE);
            scene.path.clone_from(path);
        }
        for (side, pos) in [
            (Side::Hero, self.stage.hero()),
            (Side::Foe, self.stage.foe()),
        ] {
            let pos = match &self.walk {
                Some(w) if w.side == side => {
                    let at = walk::steps(w.path.len(), w.t * map::WALK.normal);
                    w.path.get(at).copied().unwrap_or(pos)
                }
                _ => pos,
            };
            scene.push_unit(Piece { side, pos });
        }
        if self.walk.is_none() && !self.leaving {
            scene.cursor = Some(CursorView {
                pos: self.cursor.pos,
                brightness: self.cursor.brightness(),
                style: CursorStyle::Corners,
            });
        }
        scene
    }

    /// What the lines round the map show.
    pub fn view(&self, ctx: &HelloCtx) -> StageView {
        let help = if self.selected.is_some() {
            "stage.help.picked"
        } else {
            "stage.help"
        };
        let message = if self.tripped {
            "stage.tripped"
        } else {
            "stage.your_move"
        };
        StageView {
            turn: ctx.text_with("stage.turn", &[("turn", &self.stage.turn())]),
            message: ctx.text(message).to_owned(),
            help: if self.walk.is_some() {
                String::new()
            } else {
                ctx.text_with(help, &[])
            },
        }
    }

    /// Sends `command` to the rules and keeps it, if they take it.
    fn send(&mut self, command: Command) -> Option<Vec<Event>> {
        let events = self.stage.apply(&command).ok()?;
        self.commands.push(command);
        self.events.extend_from_slice(&events);
        Some(events)
    }

    /// Moves the cursor `dir`, steering the path while your piece is
    /// picked up.
    fn step(&mut self, ctx: &mut HelloCtx, dir: Dir) {
        let map = self.stage.map();
        if !self.cursor.step(dir, map.width(), map.height()) {
            return;
        }
        ctx.audio.cursor_tick();
        let to = self.cursor.pos;
        if let Some((reach, path)) = &mut self.selected {
            let most = usize::try_from(MOVES).unwrap_or(usize::MAX) + 1;
            let legal = |path: &[Pos]| {
                path.windows(2).all(|w| Pos::manhattan(w[0], w[1]) == 1)
                    && path.iter().all(|&p| reach.contains(p))
                    && path.len() <= most
            };
            *path = steer(path, to, |to| reach.path_to(to), legal);
        }
    }

    /// Confirm: picks your piece up, or sends it walking to the cursor.
    fn confirm(&mut self, ctx: &mut HelloCtx) {
        let at = self.cursor.pos;
        match self.selected.take() {
            None if at == self.stage.hero() => {
                ctx.audio.menu(MenuSound::Select);
                self.selected = Some((self.stage.reach(), vec![at]));
            }
            None => ctx.audio.menu(MenuSound::Denied),
            Some((_, path)) if path.last() == Some(&at) => {
                if self.send(Command::Move(at)).is_some() {
                    ctx.audio.menu(MenuSound::Select);
                    self.tripped = false;
                    self.walk = Some(Walk {
                        side: Side::Hero,
                        path,
                        t: 0.0,
                    });
                }
            }
            Some(kept) => {
                ctx.audio.menu(MenuSound::Denied);
                self.selected = Some(kept);
            }
        }
    }

    /// The foe's move: its mind's command, and its step shown.
    fn foe_moves(&mut self) {
        let command = Chase.choose(&self.stage);
        for event in self.send(command).unwrap_or_default() {
            match event {
                Event::Stepped { from, to } => {
                    self.walk = Some(Walk {
                        side: Side::Foe,
                        path: vec![from, to],
                        t: 0.0,
                    });
                }
                Event::Tripped => self.tripped = true,
                _ => {}
            }
        }
    }

    /// A walk has been shown: the end shows if the stage is over, else
    /// the foe moves if it is its move (after yours).
    fn walked(&mut self) -> Transition<Hello> {
        if let Some(outcome) = self.stage.outcome() {
            return Transition::Replace(Box::new(End::new(outcome)));
        }
        if self.stage.side_to_move() == Side::Foe {
            self.foe_moves();
            if self.walk.is_none() {
                return self.walked();
            }
        }
        Transition::None
    }

    /// The skin of the player's look, with the painter of its pieces, at
    /// work on `paint`.
    fn with_skin<R>(
        ctx: &HelloCtx,
        paint: impl FnOnce(&dyn GridSkin<Piece>, &dyn shuyi_ui::grid::UnitPainter<Piece>) -> R,
    ) -> R {
        match ctx.settings().game.look {
            MapLook::Glyphs => paint(
                &GLYPHS,
                &Letters {
                    palette: &ctx.palette,
                },
            ),
            MapLook::Tiles => paint(
                &ctx.game.sprite_skin(),
                &Tiles {
                    image: ctx.game.tiles(),
                },
            ),
        }
    }

    /// The tiles the player's skin fits in [`AREA`].
    pub fn view_tiles(ctx: &HelloCtx) -> (i32, i32) {
        Self::with_skin(ctx, |skin, _| skin.view_tiles(AREA))
    }
}

impl Screen<Hello> for StageScreen {
    fn name(&self) -> &'static str {
        "stage"
    }

    fn update(&mut self, ctx: &mut HelloCtx, input: &FrameInput<Action>) -> Transition<Hello> {
        if !self.briefed {
            self.briefed = true;
            return Transition::Push(Box::new(Talk::new(ctx, "intro")));
        }
        let dt = input.dt();
        self.cursor.tick(dt);
        if let Some(w) = &mut self.walk {
            w.t += dt;
            let shown = walk::steps(w.path.len(), w.t * map::WALK.normal);
            if shown + 1 < w.path.len() {
                return Transition::None;
            }
            self.walk = None;
            return self.walked();
        }
        for &action in &input.actions {
            match action.ui() {
                Some(UiAction::Up) => self.step(ctx, Dir::Up),
                Some(UiAction::Down) => self.step(ctx, Dir::Down),
                Some(UiAction::Left) => self.step(ctx, Dir::Left),
                Some(UiAction::Right) => self.step(ctx, Dir::Right),
                Some(UiAction::Confirm) => self.confirm(ctx),
                Some(UiAction::Cancel) if self.selected.is_some() => {
                    ctx.audio.menu(MenuSound::Cancel);
                    self.selected = None;
                }
                Some(UiAction::Cancel) => {
                    self.leaving = true;
                    return Transition::Push(Box::new(Leave));
                }
                Some(UiAction::Debug) | None => {}
            }
            if self.walk.is_some() {
                break;
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &HelloCtx, buf: &mut GlyphBuffer) {
        let looks = shuyi_ui::grid::Looks {
            palette: &ctx.palette,
            terrains: &Terrain,
            lights: &ctx.game.lights,
        };
        Self::with_skin(ctx, |skin, painter| {
            let scene = self.scene(skin.view_tiles(AREA));
            skin::paint_stage(&ctx.palette, &self.view(ctx), buf);
            skin.paint(&looks, &scene, AREA, painter, buf);
        });
    }

    /// Back from the scene, or from the question: on a yes, it leaves too.
    fn uncovered(&mut self, _: &mut HelloCtx, result: Option<ScreenResult>) -> Transition<Hello> {
        self.leaving = false;
        if result.and_then(ScreenResult::take::<bool>) == Some(true) {
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }
}
