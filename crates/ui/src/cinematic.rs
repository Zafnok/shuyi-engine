//! Plays a cinematic (`shuyi_assets::cinematic`, ticket 1113; the game's
//! ADR-0060): a [`Timeline`] says which shot a time falls in and how far
//! through it, and a [`CinematicPlayer`] has the shot draw itself there.
//! What a shot shows is the game's: its shot kinds implement
//! [`ShotPainter`].
//!
//! **Every frame is drawn from the time alone**: nothing is carried from
//! one frame to the next, and no shot may keep state that an update
//! advances; whatever moves in a shot is worked out from the time. So
//! whoever owns the clock can follow the music, jump or loop, and a test
//! can draw any moment. The player has no clock of its own.
//!
//! The engine also gives what most shots move by: an [`ease_in_out`], and
//! a [`Pan`] over a picture shown as the frame's backdrop (ADR-0048,
//! [`cinema::view`]), its frame ([`PanFrame`]) and its paint
//! ([`paint_pan`]).

use crate::prelude::*;
use alloc::rc::Rc;

use shuyi_assets::cinematic::CinematicDef;

use crate::ctx::{Ctx, GameData};
use crate::frame::cinema;
use crate::frame::color::{EngineColor, Palette};
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Rect};

/// Where a time falls in a [`Timeline`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShotAt {
    /// Which shot, counted from 0.
    pub index: usize,
    /// How far through the shot: 0 at its start, up to (never reaching) 1
    /// at the next shot's start.
    pub progress: f32,
}

/// Everything a shot is told about a moment of its cinematic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moment {
    /// Which shot, counted from 0.
    pub index: usize,
    /// How far through the shot, as [`ShotAt::progress`].
    pub progress: f32,
    /// Milliseconds since the shot started.
    pub into_ms: u32,
    /// How long the shot lasts, in milliseconds ([`Timeline::shot_ms`]).
    pub shot_ms: u32,
    /// Milliseconds since the cinematic started, wrapped at its length.
    pub t_ms: u32,
}

/// When each shot of a cinematic starts, and when the cinematic ends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    /// Each shot's start, in milliseconds, in order.
    starts_ms: Vec<u32>,
    /// The length, in milliseconds; at least 1.
    length_ms: u32,
}

impl Timeline {
    /// A timeline of shots starting at `starts_ms` (in order, the first at
    /// 0, as a loaded cinematic has them) over `length_ms` (0 counts as 1).
    pub fn new(starts_ms: Vec<u32>, length_ms: u32) -> Self {
        Self {
            starts_ms,
            length_ms: length_ms.max(1),
        }
    }

    /// The timeline of `def`'s shots.
    pub fn of<S>(def: &CinematicDef<S>) -> Self {
        Self::new(def.shots.iter().map(|s| s.at_ms).collect(), def.length_ms)
    }

    /// The length, in milliseconds.
    pub fn length_ms(&self) -> u32 {
        self.length_ms
    }

    /// When the last shot starts, in milliseconds (0 with no shots).
    pub fn last_start_ms(&self) -> u32 {
        self.starts_ms.last().copied().unwrap_or(0)
    }

    /// Whether `t_ms` (wrapped at the length) falls in the last shot.
    pub fn in_last_shot(&self, t_ms: u64) -> bool {
        self.wrapped_ms(t_ms) >= self.last_start_ms()
    }

    /// How long shot `index` lasts, in milliseconds: until the next one
    /// starts, the last until the end. 0 for a shot there isn't.
    pub fn shot_ms(&self, index: usize) -> u32 {
        let Some(&start) = self.starts_ms.get(index) else {
            return 0;
        };
        let end = self.starts_ms.get(index + 1).copied();
        end.unwrap_or(self.length_ms).saturating_sub(start)
    }

    /// `t_ms` wrapped at the length: the cinematic loops.
    pub fn wrapped_ms(&self, t_ms: u64) -> u32 {
        u32::try_from(t_ms % u64::from(self.length_ms)).unwrap_or(0)
    }

    /// The shot `t_ms` milliseconds after the start falls in, wrapped at
    /// the length. A shot lasts until the next one starts, the last until
    /// the end; time before the first shot's start is the first shot's.
    pub fn at_ms(&self, t_ms: u64) -> ShotAt {
        let (index, into) = self.into_shot(t_ms);
        let start = self.wrapped_ms(t_ms) - into;
        let end = self.starts_ms.get(index + 1).map_or(self.length_ms, |&s| s);
        let span = end.saturating_sub(start).max(1);
        #[allow(clippy::cast_precision_loss)] // a shot is far below 2^24 ms
        let progress = (into as f32 / span as f32).clamp(0.0, 1.0);
        ShotAt { index, progress }
    }

    /// The shot `t_ms` falls in ([`at_ms`](Self::at_ms)), counted from 0,
    /// and how many milliseconds into it that is.
    pub fn into_shot(&self, t_ms: u64) -> (usize, u32) {
        let t = self.wrapped_ms(t_ms);
        let index = self.starts_ms.iter().rposition(|&s| s <= t).unwrap_or(0);
        let start = self.starts_ms.get(index).map_or(0, |&s| s.min(t));
        (index, t - start)
    }

    /// [`at_ms`](Self::at_ms) for `t` seconds ([`millis`]).
    pub fn at(&self, t: f32) -> ShotAt {
        self.at_ms(millis(t))
    }

    /// Everything about the moment `t_ms` milliseconds after the start
    /// (wrapped at the length) that a shot is told.
    pub fn moment(&self, t_ms: u64) -> Moment {
        let ShotAt { index, progress } = self.at_ms(t_ms);
        Moment {
            index,
            progress,
            into_ms: self.into_shot(t_ms).1,
            shot_ms: self.shot_ms(index),
            t_ms: self.wrapped_ms(t_ms),
        }
    }
}

/// `t` seconds in whole milliseconds. A time below 0, or one that isn't a
/// number, counts as 0.
pub fn millis(t: f32) -> u64 {
    let ms = (f64::from(t) * 1000.0).round();
    // The cast stops at 0 and at the biggest number by itself.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    if ms.is_finite() { ms as u64 } else { 0 }
}

/// `p` (0 to 1; clamped, not a number counts as 0) eased in and out: slow
/// at both ends, fastest in the middle.
pub fn ease_in_out(p: f32) -> f32 {
    let p = if p.is_finite() {
        p.clamp(0.0, 1.0)
    } else {
        0.0
    };
    p * p * (3.0 - 2.0 * p)
}

/// A pan over a picture shown as the frame's backdrop (ADR-0048): the
/// picture's pixel `from` in the middle of the screen when the shot
/// starts, `to` when it ends, eased in and out ([`ease_in_out`]), at one
/// whole zoom for the whole shot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pan {
    /// The picture's pixel in the middle of the screen at the start.
    pub from: (f32, f32),
    /// The picture's pixel in the middle of the screen at the end.
    pub to: (f32, f32),
    /// Screen pixels per picture pixel.
    pub zoom: u8,
}

impl Pan {
    /// The picture's pixel in the middle of the screen `progress` (0 to 1)
    /// through the shot.
    pub fn centre(&self, progress: f32) -> (f32, f32) {
        let eased = ease_in_out(progress);
        let (from, to) = (self.from, self.to);
        (
            from.0 + (to.0 - from.0) * eased,
            from.1 + (to.1 - from.1) * eased,
        )
    }

    /// The picture's pixel at the top-left of a screen of `screen`'s cells
    /// of `console` ([`cinema::view`]), `progress` through the shot, over a
    /// picture `picture_px` big: the backdrop's `origin_px`.
    pub fn origin_px(
        &self,
        console: ConsoleSpec,
        picture_px: (u32, u32),
        screen: Rect,
        progress: f32,
    ) -> (f32, f32) {
        let px = |cells: i32, cell: u16| u32::try_from(cells).unwrap_or(0) * u32::from(cell);
        let clip = (px(screen.w, console.cell_w), px(screen.h, console.cell_h));
        cinema::view(picture_px, clip, self.centre(progress), self.zoom)
    }

    /// What the pan shows `progress` through the shot on a screen of
    /// `screen`'s cells of `console`, over a picture `picture_px` big
    /// ([`origin_px`](Self::origin_px)).
    pub fn frame(
        &self,
        console: ConsoleSpec,
        picture_px: (u32, u32),
        screen: Rect,
        progress: f32,
    ) -> PanFrame {
        PanFrame {
            origin_px: self.origin_px(console, picture_px, screen, progress),
            zoom: self.zoom,
        }
    }
}

/// A frame of a [`Pan`]: where its picture is behind the screen, as plain
/// data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanFrame {
    /// The picture's pixel at the screen's top-left ([`Pan::origin_px`]).
    pub origin_px: (f32, f32),
    /// Screen pixels per picture pixel.
    pub zoom: u8,
}

/// Paints `frame` over the whole of `buf`: `picture` (a whole map, say:
/// [`MapPicture`](crate::grid::MapPicture)) as its backdrop, behind cells
/// that are all holes, in `palette`'s text color.
pub fn paint_pan(
    palette: &Palette,
    picture: Rc<GlyphBuffer>,
    frame: PanFrame,
    buf: &mut GlyphBuffer,
) {
    let hole = Cell::see_through(' ', palette.engine(EngineColor::Text));
    buf.fill_rect(buf.bounds(), hole);
    buf.set_backdrop(picture, buf.bounds(), frame.origin_px, frame.zoom);
}

/// A game's shot, built once to be drawn at any moment: one of its shot
/// kinds with whatever it needs ready (a picture painted, a text laid
/// out). Never changed by drawing.
pub trait ShotPainter {
    /// The game it is a shot of.
    type Game: GameData;
    /// What every shot of a player is drawn with, from whoever plays it
    /// (`()` for nothing): the backdrop a title picked, say.
    type Shared;
    /// What a frame of the shot shows, as plain data: for tests of what
    /// plays, and for [`paint`](Self::paint).
    type Frame;

    /// What the shot shows at `moment`, on a screen of `screen` cells.
    fn frame(
        &self,
        ctx: &Ctx<Self::Game>,
        shared: &Self::Shared,
        moment: Moment,
        screen: Rect,
    ) -> Self::Frame;

    /// Paints `frame` (the shot's frame at `moment`) over the whole of
    /// `buf`, which is cleared to the background.
    fn paint(
        &self,
        ctx: &Ctx<Self::Game>,
        shared: &Self::Shared,
        moment: Moment,
        frame: Self::Frame,
        buf: &mut GlyphBuffer,
    );
}

/// Draws a cinematic of shots `S` at any time.
#[derive(Debug, Clone, PartialEq)]
pub struct CinematicPlayer<S> {
    timeline: Timeline,
    shots: Vec<S>,
}

impl<S> CinematicPlayer<S> {
    /// A player of `shots` (one for each start of `timeline`).
    pub fn new(timeline: Timeline, shots: Vec<S>) -> Self {
        Self { timeline, shots }
    }

    /// A player of the cinematic `def`, each shot built once by `build`,
    /// in order.
    pub fn of<'d, D>(def: &'d CinematicDef<D>, build: impl FnMut(&'d D) -> S) -> Self {
        let shots = def.shots.iter().map(|s| &s.shot).map(build).collect();
        Self::new(Timeline::of(def), shots)
    }

    /// When its shots start and how long it is.
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }

    /// Shot `index`, counted from 0.
    pub fn shot(&self, index: usize) -> Option<&S> {
        self.shots.get(index)
    }

    /// The moment `t` seconds after the start (wrapped at the length).
    pub fn moment(&self, t: f32) -> Moment {
        self.timeline.moment(millis(t))
    }

    /// The shot shown `t` seconds after the start (wrapped at the length),
    /// and the moment of it; `None` with no shots.
    pub fn at(&self, t: f32) -> Option<(&S, Moment)> {
        let moment = self.moment(t);
        self.shots.get(moment.index).map(|shot| (shot, moment))
    }
}

impl<S: ShotPainter> CinematicPlayer<S> {
    /// What is shown `t` seconds after the start (wrapped at the length),
    /// on a screen of `screen` cells; `None` with no shots.
    pub fn frame(
        &self,
        ctx: &Ctx<S::Game>,
        shared: &S::Shared,
        t: f32,
        screen: Rect,
    ) -> Option<S::Frame> {
        let (shot, moment) = self.at(t)?;
        Some(shot.frame(ctx, shared, moment, screen))
    }

    /// Draws the cinematic as it is `t` seconds after its start (wrapped
    /// at the length) over the whole of `buf`, at `brightness` (1 = as it
    /// is, 0 = black: [`GlyphBuffer::fade`]). The same `t` always draws
    /// the same frame.
    pub fn draw(
        &self,
        ctx: &Ctx<S::Game>,
        shared: &S::Shared,
        t: f32,
        brightness: f32,
        buf: &mut GlyphBuffer,
    ) {
        let c = |e| ctx.palette.engine(e);
        let blank = Cell::new(' ', c(EngineColor::Text), c(EngineColor::Background));
        buf.fill_rect(buf.bounds(), blank);
        if let Some((shot, moment)) = self.at(t) {
            let frame = shot.frame(ctx, shared, moment, buf.bounds());
            shot.paint(ctx, shared, moment, frame, buf);
        }
        buf.fade(brightness);
    }
}

#[cfg(test)]
mod tests;
