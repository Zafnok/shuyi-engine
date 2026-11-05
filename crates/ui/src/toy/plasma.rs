//! The toy game's plasma stage (ticket 2399, ADR-0078): three pictures a
//! skin paints at run time from the step count, each 64 × 48 and drawn
//! twice its size: a plasma in RGBA, the same plasma's levels through a
//! palette that turns with the steps (indexed), and its bright half in
//! black and white (one-bit). The plasma moves every [`STEPS_A_PHASE`]
//! steps, so in the steps between, no picture changes and none is
//! uploaded again. Integers only, so its pictures are the same on every
//! platform.

use core::any::Any;

use super::{Toy, ToyCtx};
use crate::frame::color::{EngineColor, Rgb};
use crate::frame::glyph_buffer::{Cell, GlyphBuffer, Layer, Rect};
use crate::frame::painted::{Painted, PaintedId, PaintedSprite};
use crate::game::{Game, GameHooks};
use crate::prelude::*;
use crate::screen::{FrameInput, Screen, Transition};

/// The RGBA plasma.
pub const RGBA: PaintedId = PaintedId::new("plasma/rgba");
/// The indexed plasma.
pub const INDEXED: PaintedId = PaintedId::new("plasma/indexed");
/// The one-bit plasma.
pub const ONE_BIT: PaintedId = PaintedId::new("plasma/one-bit");

/// The pictures' size.
pub const SIZE: (u16, u16) = (64, 48);

/// How many steps the plasma stays still.
pub const STEPS_A_PHASE: u64 = 4;

/// The toy game on `ctx`, showing the plasma stage alone.
pub fn game_on(ctx: ToyCtx) -> Game<Toy> {
    Game::new(ctx, Box::new(PlasmaStage::default()), GameHooks::default())
}

/// The stage's view: how many steps have run. The skin paints from it.
#[derive(Debug, Default)]
pub struct PlasmaStage {
    /// Steps run, one a frame.
    pub step: u64,
}

impl PlasmaStage {
    /// The plasma's phase: what its pictures show (it stops after
    /// 2³² phases, years of frames).
    pub fn phase(&self) -> u32 {
        u32::try_from(self.step / STEPS_A_PHASE).unwrap_or(u32::MAX)
    }
}

/// A triangle wave of `v`, `period` long: 0 up to 255 and back.
fn wave(v: u32, period: u32) -> u8 {
    let (at, half) = (v % period, period / 2);
    let rise = if at < half { at } else { period - at };
    u8::try_from(rise * 255 / half).unwrap_or(u8::MAX)
}

/// The plasma's level at `(x, y)` in phase `phase`: three waves summed.
fn level(x: u32, y: u32, phase: u32) -> u8 {
    let sum = u32::from(wave(x + phase, 32))
        + u32::from(wave(y + 2 * phase, 24))
        + u32::from(wave(x + y + 3 * phase, 40));
    u8::try_from(sum / 3).unwrap_or(u8::MAX)
}

/// Each pixel of a picture the plasma's size, with its `(x, y)`, row
/// after row.
fn pixels() -> impl Iterator<Item = (u32, u32)> {
    let (w, h) = (u32::from(SIZE.0), u32::from(SIZE.1));
    (0..h).flat_map(move |y| (0..w).map(move |x| (x, y)))
}

/// The plasma in RGBA in `phase`: its level in red, the level a third of
/// the way round in green, and a slower wave in blue.
pub fn rgba(phase: u32) -> Painted {
    let mut picture = Painted::rgba(SIZE.0, SIZE.1);
    let bytes = picture.pixels_mut().as_chunks_mut::<4>().0;
    for ((x, y), pixel) in pixels().zip(bytes) {
        let l = level(x, y, phase);
        *pixel = [l, l.wrapping_add(85), wave(x + y + phase, 96), u8::MAX];
    }
    picture
}

/// The plasma's levels through a palette of 256 colours turned by
/// `phase`: the palette moves, the levels stay.
pub fn indexed(phase: u32) -> Painted {
    let palette: Vec<Rgb> = (0..=255u32)
        .map(|i| {
            let i = i + phase * 8;
            Rgb::new(wave(i, 256), wave(i + 85, 256), wave(i + 170, 256))
        })
        .collect();
    let mut picture = Painted::indexed(SIZE.0, SIZE.1, &palette);
    for ((x, y), pixel) in pixels().zip(picture.pixels_mut()) {
        *pixel = level(x, y, 0);
    }
    picture
}

/// The plasma's bright half in `phase`: white from level 128.
pub fn one_bit(phase: u32) -> Painted {
    let mut picture = Painted::one_bit(SIZE.0, SIZE.1);
    let stride = picture.stride();
    let bytes = picture.pixels_mut();
    for (x, y) in pixels() {
        if level(x, y, phase) >= 128 {
            let (x, y) = (x as usize, y as usize);
            bytes[y * stride + x / 8] |= 0x80 >> (x % 8);
        }
    }
    picture
}

impl Screen<Toy> for PlasmaStage {
    fn name(&self) -> &'static str {
        "plasma"
    }

    fn update(&mut self, _: &mut ToyCtx, _: &FrameInput<super::Action>) -> Transition<Toy> {
        self.step += 1;
        Transition::None
    }

    /// The three pictures, twice their size: RGBA top left, indexed top
    /// right, one-bit below, each made afresh and put in only if it
    /// changed; and the step.
    fn draw(&self, ctx: &ToyCtx, buf: &mut GlyphBuffer) {
        let paper = ctx.palette.engine(EngineColor::Panel);
        // The toy's engine colours are all near the same red: white shows.
        let ink = Rgb::new(255, 255, 255);
        buf.fill_rect(buf.bounds(), Cell::new(' ', ink, paper));
        let phase = self.phase();
        let pictures = buf.pictures_mut();
        pictures.update(RGBA, rgba(phase));
        pictures.update(INDEXED, indexed(phase));
        pictures.update(ONE_BIT, one_bit(phase));
        let whole = Rect::new(0, 0, SIZE.0.into(), SIZE.1.into());
        let twice = |x: i32, y: i32| Rect::new(x, y, i32::from(SIZE.0) * 2, i32::from(SIZE.1) * 2);
        for (id, dest) in [
            (RGBA, twice(8, 16)),
            (INDEXED, twice(152, 16)),
            (ONE_BIT, twice(8, 128)),
        ] {
            buf.add_painted(PaintedSprite::new(id, whole, dest, Layer::Under));
        }
        buf.print(19, 9, &format!("step {}", self.step), ink, paper);
    }

    fn as_any(&self) -> Option<&dyn Any> {
        Some(self)
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        Some(self)
    }
}
