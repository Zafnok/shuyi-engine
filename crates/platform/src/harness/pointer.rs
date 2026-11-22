//! The Harness's pointer (ticket 2405): a test moves it, clicks, turns the
//! wheel and clicks a menu row by its words, each step a frame of
//! [`FRAME_DT`], as [`Harness::keys`] presses keys.

use crate::alloc_prelude::*;

use shuyi_ui::frame::GlyphBuffer;
use shuyi_ui::{GameData, RawInputEvent};

use super::{FRAME_DT, Harness};

impl<G: GameData> Harness<G> {
    /// Moves the pointer to the console pixel `(x, y)`: a frame with
    /// [`RawInputEvent::PointerMoved`].
    pub fn point(&mut self, x: i32, y: i32) -> &mut Self {
        self.pointer = Some((x, y));
        self.frame(&[RawInputEvent::PointerMoved { x, y }], FRAME_DT)
    }

    /// Clicks the console pixel `(x, y)`: [`point`](Self::point) there
    /// when the pointer is elsewhere, then a frame with the press and one
    /// with the release.
    pub fn click(&mut self, x: i32, y: i32) -> &mut Self {
        if self.pointer != Some((x, y)) {
            self.point(x, y);
        }
        self.frame(&[RawInputEvent::PointerDown], FRAME_DT);
        self.frame(&[RawInputEvent::PointerUp], FRAME_DT)
    }

    /// Turns the wheel `notches` (positive away from the player) in one
    /// frame.
    pub fn wheel(&mut self, notches: i32) -> &mut Self {
        self.frame(&[RawInputEvent::Wheel(notches)], FRAME_DT)
    }

    /// The pointer leaves the frame, in one frame.
    pub fn leave_pointer(&mut self) -> &mut Self {
        self.pointer = None;
        self.frame(&[RawInputEvent::PointerLeft], FRAME_DT)
    }

    /// [`click`](Self::click)s the centre pixel of the first cell of the
    /// first run of `text` in the frame shown ([`Game::buffer`]), top row
    /// first: so a test clicks a menu row by its words.
    ///
    /// # Panics
    ///
    /// If `text` isn't shown.
    ///
    /// [`Game::buffer`]: shuyi_ui::Game::buffer
    pub fn click_text(&mut self, text: &str) -> &mut Self {
        let buffer = self.game().buffer();
        let Some((col, row)) = find_text(buffer, text) else {
            panic!("Harness: {text:?} isn't shown");
        };
        let (cw, ch) = buffer.spec().cell_px();
        self.click(col * cw + cw / 2, row * ch + ch / 2)
    }
}

/// The cell `text` starts in, the first run of it, top row first: a wide
/// character's second cell is no character of its own.
fn find_text(buffer: &GlyphBuffer, text: &str) -> Option<(i32, i32)> {
    let wanted: Vec<char> = text.chars().collect();
    if wanted.is_empty() {
        return None;
    }
    (0..i32::from(buffer.height())).find_map(|y| {
        let cells: Vec<(i32, char)> = (0..i32::from(buffer.width()))
            .filter_map(|x| {
                let cell = buffer.get(x, y)?;
                (!cell.is_continuation()).then_some((x, cell.glyph))
            })
            .collect();
        cells
            .windows(wanted.len())
            .find(|run| run.iter().map(|c| c.1).eq(wanted.iter().copied()))
            .map(|run| (run[0].0, y))
    })
}
