//! The frame on the Playdate's screen: a game's frame drawn with its
//! one-bit look into a frame of our own (`shuyi_ui::frame::onebit`, redrawing
//! only where it differs from the last), then the rows that differ from the
//! SDK's frame buffer copied into it and marked (`markUpdatedRows`). The
//! loading card and the errors are the SDK's text, in the system's font.

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;

use shuyi_platform::{Card, Frame, OneBitLook, Show};
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::frame::onebit::{self, FRAME_BYTES, ROW_BYTES, RowMask, luminance};

use crate::report::{Measured, Phases};
use crate::sdk::Sdk;

/// What a one-bit look that isn't there says.
// check-text: not player text (a build problem, shown as errors are)
const NO_LOOK: &str = "This game has no one-bit look: its Look has no one_bit part.";

/// Whether `rgb` is nearer white than black.
fn is_light(rgb: Rgb) -> bool {
    luminance(rgb) >= 128
}

/// The first and last row of each run of neighbouring rows of `rows`.
fn runs(rows: &RowMask) -> impl Iterator<Item = (i32, i32)> + '_ {
    let mut rows = rows.rows().peekable();
    core::iter::from_fn(move || {
        let first = rows.next()?;
        let mut last = first;
        while rows.next_if_eq(&(last + 1)).is_some() {
            last += 1;
        }
        Some((first, last))
    })
}

/// What the screen last showed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Showing {
    /// Nothing yet.
    #[default]
    Nothing,
    /// The loading card.
    Card,
    /// The errors, or a game without a one-bit look.
    Errors,
    /// The game's frame.
    Game,
}

/// The screen, as the backend last drew it.
#[derive(Debug)]
pub struct Screen {
    /// The frame drawn with [`shown`](Self::shown), laid out as the SDK's.
    drawn: Box<onebit::Frame>,
    /// The game's frame last drawn, `None` before the first and after text,
    /// which is what the SDK's frame buffer then holds.
    shown: Option<GlyphBuffer>,
    /// The text last drawn, `None` when the last thing drawn was the game.
    text: Option<String>,
    /// Whether the SDK's frame buffer may differ from [`drawn`](Self::drawn)
    /// though the game's frame didn't change: a system menu's been over it.
    stale: bool,
    /// Whether anything reached the SDK since [`take_changed`](Self::take_changed).
    changed: bool,
    showing: Showing,
    /// What the run report measures of the draws, while it is on.
    measured: Option<Measured>,
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            drawn: Box::new([u8::MAX; FRAME_BYTES]),
            shown: None,
            text: None,
            stale: true,
            changed: false,
            showing: Showing::Nothing,
            measured: None,
        }
    }
}

impl Screen {
    /// Shows `frame`.
    pub fn present(&mut self, sdk: &impl Sdk, frame: &Frame<'_>) {
        self.show(sdk, frame);
    }

    /// [`present`](Self::present), with the SDK's type named for its timer.
    fn show<S: Sdk>(&mut self, sdk: &S, frame: &Frame<'_>) {
        match frame.show {
            Show::Card(Card::Text {
                line, background, ..
            }) => self.text(sdk, line, is_light(*background), Showing::Card),
            Show::Card(card) => self.text(sdk, "", is_light(card.background()), Showing::Card),
            Show::Errors(text) => self.text(sdk, text, true, Showing::Errors),
            Show::Game { buffer, look, .. } => match look.and_then(|look| look.one_bit) {
                Some(one_bit) => self.game(sdk, buffer, &one_bit),
                None => self.text(sdk, NO_LOOK, true, Showing::Errors),
            },
            _ => {}
        }
    }

    /// What the screen last showed.
    pub(crate) fn showing(&self) -> Showing {
        self.showing
    }

    /// Times each draw of the game's frame from now on, and each of its
    /// phases if `phases`.
    pub(crate) fn measure(&mut self, phases: bool) {
        self.measured = Some(Measured {
            phases: phases.then(Phases::default),
            ..Measured::default()
        });
    }

    /// What was measured since the last take, which starts again; nothing
    /// unless [`measure`](Self::measure) was called.
    pub(crate) fn take_measured(&mut self) -> Measured {
        let Some(measured) = &mut self.measured else {
            return Measured::default();
        };
        let phases = measured.phases.map(|_| Phases::default());
        core::mem::replace(
            measured,
            Measured {
                phases,
                ..Measured::default()
            },
        )
    }

    /// The next frame must reach the SDK's frame buffer whatever it shows:
    /// something else drew over it.
    pub fn invalidate(&mut self) {
        self.stale = true;
    }

    /// Whether anything reached the SDK since the last call: the update
    /// callback returns it.
    pub fn take_changed(&mut self) -> bool {
        core::mem::take(&mut self.changed)
    }

    fn text(&mut self, sdk: &impl Sdk, text: &str, white: bool, showing: Showing) {
        self.showing = showing;
        if self.text.as_deref() == Some(text) && !self.stale {
            return;
        }
        sdk.draw_text(text, white);
        self.text = Some(text.to_owned());
        self.shown = None;
        self.stale = false;
        self.changed = true;
    }

    fn game<S: Sdk>(&mut self, sdk: &S, buffer: &GlyphBuffer, look: &OneBitLook<'_>) {
        self.text = None;
        self.showing = Showing::Game;
        if self.stale || self.shown.as_ref() != Some(buffer) {
            let start = self.measured.is_some().then(S::clock_us);
            self.draw::<S>(buffer, look);
            self.copy(sdk);
            if let (Some(start), Some(measured)) = (start, &mut self.measured) {
                measured.draw.add(S::clock_us().saturating_sub(start));
            }
        }
        self.stale = false;
    }

    /// Draws `buffer` into [`drawn`](Self::drawn): where it differs from the
    /// last frame, or whole with each phase timed when phases are measured.
    fn draw<S: Sdk>(&mut self, buffer: &GlyphBuffer, look: &OneBitLook<'_>) {
        let (font, map) = (look.font, look.map);
        if let Some(phases) = self.measured.as_mut().and_then(|m| m.phases.as_mut()) {
            let mut last = S::clock_us();
            let mark = &mut |phase| {
                let now = S::clock_us();
                phases.add(phase, now.saturating_sub(last));
                last = now;
            };
            onebit::render_marked_with(buffer, font, &look.images, map, &mut self.drawn, mark);
            self.shown = Some(buffer.clone());
            return;
        }
        if let Some(shown) = &mut self.shown {
            onebit::render_changed_with(shown, buffer, font, &look.images, map, &mut self.drawn);
            shown.clone_from(buffer);
        } else {
            onebit::render_with(buffer, font, &look.images, map, &mut self.drawn);
            self.shown = Some(buffer.clone());
        }
    }

    /// Copies the rows that differ into the SDK's frame buffer, and marks
    /// each run of them.
    fn copy(&mut self, sdk: &impl Sdk) {
        let mut changed = RowMask::default();
        let drawn = &self.drawn;
        sdk.with_frame(|screen| {
            changed = onebit::changed_rows(screen, drawn);
            for y in changed.rows() {
                let start = usize::try_from(y).unwrap_or(0) * ROW_BYTES;
                let row = start..start + ROW_BYTES;
                screen[row.clone()].copy_from_slice(&drawn[row]);
            }
        });
        for (first, last) in runs(&changed) {
            sdk.mark_rows(first, last);
        }
        self.changed |= !changed.is_empty();
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;

    use super::*;

    fn mask(rows: &[i32]) -> RowMask {
        let mut mask = RowMask::default();
        for &y in rows {
            mask.insert(y);
        }
        mask
    }

    #[test]
    fn neighbouring_rows_are_one_run() {
        let got: Vec<_> = runs(&mask(&[3, 4, 5, 9, 239])).collect();
        assert_eq!(got, vec![(3, 5), (9, 9), (239, 239)]);
        assert_eq!(runs(&mask(&[])).count(), 0);
    }
}
