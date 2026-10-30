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
}

impl Default for Screen {
    fn default() -> Self {
        Self {
            drawn: Box::new([u8::MAX; FRAME_BYTES]),
            shown: None,
            text: None,
            stale: true,
            changed: false,
        }
    }
}

impl Screen {
    /// Shows `frame`.
    pub fn present(&mut self, sdk: &impl Sdk, frame: &Frame<'_>) {
        match frame.show {
            Show::Card(Card::Text {
                line, background, ..
            }) => self.text(sdk, line, is_light(*background)),
            Show::Card(card) => self.text(sdk, "", is_light(card.background())),
            Show::Errors(text) => self.text(sdk, text, true),
            Show::Game { buffer, look, .. } => match look.and_then(|look| look.one_bit) {
                Some(one_bit) => self.game(sdk, buffer, &one_bit),
                None => self.text(sdk, NO_LOOK, true),
            },
            _ => {}
        }
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

    fn text(&mut self, sdk: &impl Sdk, text: &str, white: bool) {
        if self.text.as_deref() == Some(text) && !self.stale {
            return;
        }
        sdk.draw_text(text, white);
        self.text = Some(text.to_owned());
        self.shown = None;
        self.stale = false;
        self.changed = true;
    }

    fn game(&mut self, sdk: &impl Sdk, buffer: &GlyphBuffer, look: &OneBitLook<'_>) {
        self.text = None;
        let (font, map) = (look.font, look.map);
        match &mut self.shown {
            Some(shown) if shown == buffer && !self.stale => return,
            Some(shown) => {
                onebit::render_changed_with(shown, buffer, font, &look.images, map, &mut self.drawn);
                shown.clone_from(buffer);
            }
            None => {
                onebit::render_with(buffer, font, &look.images, map, &mut self.drawn);
                self.shown = Some(buffer.clone());
            }
        }
        self.copy(sdk);
        self.stale = false;
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
