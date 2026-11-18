//! The frame on the Playdate's screen: a game's frame drawn with its
//! one-bit look into a frame of our own (`shuyi_ui::frame::onebit`, redrawing
//! only where it differs from the last), then the rows that differ from the
//! SDK's frame buffer copied into it and marked (`markUpdatedRows`). The
//! loading card and the errors are the SDK's text, in the system's font,
//! and the load's count is a bar under the card's text, drawn into the
//! frame buffer and marked the same way (ADR-0128 §4).

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::string::String;

use shuyi_platform::{Card, Frame, OneBitLook, Show};
use shuyi_ui::frame::Rgb;
use shuyi_ui::frame::glyph_buffer::GlyphBuffer;
use shuyi_ui::frame::onebit::{self, FRAME_BYTES, HEIGHT, ROW_BYTES, RowMask, WIDTH, luminance};

use crate::report::{Measured, Phases};
use crate::sdk::Sdk;

/// What a one-bit look that isn't there says.
// check-text: not player text (a build problem, shown as errors are)
const NO_LOOK: &str = "This game has no one-bit look: its Look has no one_bit part.";

/// Whether `rgb` is nearer white than black.
fn is_light(rgb: Rgb) -> bool {
    luminance(rgb) >= 128
}

/// The loading bar's left edge: it is half the screen wide, centred.
const BAR_LEFT: i32 = WIDTH / 4;
/// The loading bar's width, its outline in.
const BAR_WIDTH: i32 = WIDTH / 2;
/// The loading bar's top row: under the card's text, which is centred.
const BAR_TOP: i32 = HEIGHT - 48;
/// The loading bar's height, its outline in.
const BAR_HEIGHT: i32 = 8;

/// How many pixels of the loading bar's inside, within its outline of one
/// pixel, are filled for `done` units of `of`: none for a load that knows
/// of no unit, all once `done` reaches `of`.
fn bar_fill(done: u32, of: u32) -> i32 {
    let inside = BAR_WIDTH - 2;
    if of == 0 {
        return 0;
    }
    let filled = u64::from(done.min(of)) * u64::from(inside.unsigned_abs()) / u64::from(of);
    i32::try_from(filled).unwrap_or(inside)
}

/// Sets pixel `(x, y)` of `frame`, on the screen, white or black.
#[allow(clippy::cast_sign_loss)] // on the screen
fn set_pixel(frame: &mut onebit::Frame, x: i32, y: i32, white: bool) {
    let byte = &mut frame[y as usize * ROW_BYTES + (x >> 3) as usize];
    let bit = 0x80 >> (x & 7);
    if white {
        *byte |= bit;
    } else {
        *byte &= !bit;
    }
}

/// Draws the loading bar into `frame`: its outline, and `fill` pixels of
/// its inside from the left, in white if `white` and black otherwise; the
/// rest of its inside in the other.
fn draw_bar(frame: &mut onebit::Frame, fill: i32, white: bool) {
    for y in BAR_TOP..BAR_TOP + BAR_HEIGHT {
        for x in BAR_LEFT..BAR_LEFT + BAR_WIDTH {
            let edge = y == BAR_TOP
                || y == BAR_TOP + BAR_HEIGHT - 1
                || x == BAR_LEFT
                || x == BAR_LEFT + BAR_WIDTH - 1;
            let filled = x - BAR_LEFT - 1 < fill;
            set_pixel(frame, x, y, white == (edge || filled));
        }
    }
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
    /// The filled pixels of the loading bar last drawn ([`bar_fill`]),
    /// `None` when the screen shows no bar.
    bar: Option<i32>,
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
            bar: None,
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
            Show::Card(card) => {
                let line = match card {
                    Card::Text { line, .. } => line.as_str(),
                    _ => "",
                };
                self.card(sdk, line, is_light(card.background()), frame.loading);
            }
            Show::Errors(text) => {
                self.text(sdk, text, true, Showing::Errors);
            }
            Show::Game { buffer, look, .. } => match look.and_then(|look| look.one_bit) {
                Some(one_bit) => self.game(sdk, buffer, &one_bit),
                None => {
                    self.text(sdk, NO_LOOK, true, Showing::Errors);
                }
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

    /// Draws `text` on a white screen if `white`, else black, unless it is
    /// there already: whether it drew.
    fn text(&mut self, sdk: &impl Sdk, text: &str, white: bool, showing: Showing) -> bool {
        self.showing = showing;
        if self.text.as_deref() == Some(text) && !self.stale {
            return false;
        }
        sdk.draw_text(text, white);
        self.text = Some(text.to_owned());
        self.bar = None;
        self.shown = None;
        self.stale = false;
        self.changed = true;
        true
    }

    /// The loading card: its `line`, and under it the load's `count` as a
    /// bar in the card's ink (black on a white screen), drawn again only
    /// when its filled part moves or the screen was cleared.
    fn card(&mut self, sdk: &impl Sdk, line: &str, white: bool, count: Option<(u32, u32)>) {
        self.text(sdk, line, white, Showing::Card);
        let Some(fill) = count.map(|(done, of)| bar_fill(done, of)) else {
            return;
        };
        if self.bar == Some(fill) {
            return;
        }
        sdk.with_frame(|frame| draw_bar(frame, fill, !white));
        sdk.mark_rows(BAR_TOP, BAR_TOP + BAR_HEIGHT - 1);
        self.bar = Some(fill);
        self.changed = true;
    }

    fn game<S: Sdk>(&mut self, sdk: &S, buffer: &GlyphBuffer, look: &OneBitLook<'_>) {
        self.text = None;
        self.bar = None;
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
            onebit::render_marked(buffer, font, &look.images, map, &mut self.drawn, mark);
            self.shown = Some(buffer.clone());
            return;
        }
        if let Some(shown) = &mut self.shown {
            onebit::render_changed(shown, buffer, font, &look.images, map, &mut self.drawn);
            shown.clone_from(buffer);
        } else {
            onebit::render(buffer, font, &look.images, map, &mut self.drawn);
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

    use shuyi_platform::Show;
    use shuyi_ui::frame::onebit::is_white;

    use crate::fake::FakeSdk;

    /// The row through the middle of the loading bar.
    const BAR_MIDDLE: i32 = BAR_TOP + BAR_HEIGHT / 2;

    /// The pixels of the bar's inside along its middle row, white or not.
    fn inside(sdk: &FakeSdk) -> Vec<bool> {
        let frame = sdk.frame();
        (BAR_LEFT + 1..BAR_LEFT + BAR_WIDTH - 1)
            .map(|x| is_white(&frame, x, BAR_MIDDLE))
            .collect()
    }

    /// Presents `card` with the load's `count`.
    fn card_with(screen: &mut Screen, sdk: &FakeSdk, card: &Card, count: (u32, u32)) {
        screen.present(sdk, &Frame::new(Show::Card(card)).with_loading(Some(count)));
    }

    #[test]
    fn the_bar_fills_its_inside_by_the_count() {
        assert_eq!(bar_fill(0, 4), 0);
        assert_eq!(bar_fill(1, 4), (BAR_WIDTH - 2) / 4);
        assert_eq!(bar_fill(4, 4), BAR_WIDTH - 2);
        assert_eq!(bar_fill(9, 4), BAR_WIDTH - 2, "never past its end");
        assert_eq!(bar_fill(3, 0), 0, "a load of no units");
    }

    #[test]
    fn a_cards_count_is_a_bar_in_its_ink_under_its_text() {
        let sdk = FakeSdk::default();
        let mut screen = Screen::default();
        let card = Card::text("Toy").colored(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
        card_with(&mut screen, &sdk, &card, (1, 4));
        assert_eq!(sdk.take_texts(), [("Toy".to_owned(), true)]);
        // Black on white: a quarter of its inside, and its outline, black.
        let quarter = usize::try_from(bar_fill(1, 4)).unwrap_or(0);
        let pixels = inside(&sdk);
        assert!(pixels[..quarter].iter().all(|white| !white));
        assert!(pixels[quarter..].iter().all(|white| *white));
        let frame = sdk.frame();
        assert!(!is_white(&frame, BAR_LEFT, BAR_MIDDLE), "its outline");
        assert!(
            is_white(&frame, BAR_LEFT - 1, BAR_MIDDLE),
            "the card around it"
        );
        assert!(
            sdk.take_marked()
                .contains(&(BAR_TOP, BAR_TOP + BAR_HEIGHT - 1))
        );
        assert!(screen.take_changed());
    }

    #[test]
    fn the_bar_is_drawn_again_only_when_it_moves() {
        let sdk = FakeSdk::default();
        let mut screen = Screen::default();
        let card = Card::text("Toy");
        card_with(&mut screen, &sdk, &card, (1, 400));
        let uses = sdk.frame_uses();
        let _ = (screen.take_changed(), sdk.take_marked());
        // One more unit of 400 fills no more of a bar 198 pixels inside.
        card_with(&mut screen, &sdk, &card, (2, 400));
        assert_eq!(sdk.frame_uses(), uses);
        assert!(!screen.take_changed());
        // Half: drawn again, in white on the black card.
        card_with(&mut screen, &sdk, &card, (200, 400));
        assert_eq!(sdk.frame_uses(), uses + 1);
        assert!(screen.take_changed());
        let half = usize::try_from(bar_fill(1, 2)).unwrap_or(0);
        let pixels = inside(&sdk);
        assert!(pixels[..half].iter().all(|white| *white));
        assert!(pixels[half..].iter().all(|white| !white));
        // Cleared by a system menu over it, the card draws it again.
        screen.invalidate();
        card_with(&mut screen, &sdk, &card, (200, 400));
        assert_eq!(sdk.frame_uses(), uses + 2);
        assert_eq!(inside(&sdk), pixels);
    }

    /// Whether each pixel of the screen is black, row by row.
    fn black(sdk: &FakeSdk) -> Vec<bool> {
        let frame = sdk.frame();
        (0..HEIGHT)
            .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| !is_white(&frame, x, y))
            .collect()
    }

    /// The screen with black wherever `inked` says, row by row.
    fn screen_where(inked: impl Fn(i32, i32) -> bool) -> Vec<bool> {
        (0..HEIGHT)
            .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| inked(x, y))
            .collect()
    }

    /// The bar, in figures of its own: columns 100 to 299 and rows 192 to
    /// 199, half the screen wide, centred, under the middle.
    #[test]
    fn the_bar_is_an_outline_half_the_screen_wide_filled_from_the_left() {
        let (left, right, top, bottom) = (100, 299, 192, 199);
        let in_bar = |x: i32, y: i32| (left..=right).contains(&x) && (top..=bottom).contains(&y);
        let on_edge =
            |x: i32, y: i32| in_bar(x, y) && (x == left || x == right || y == top || y == bottom);
        let white_card = Card::text("Toy").colored(Rgb::new(0, 0, 0), Rgb::new(255, 255, 255));
        let sdk = FakeSdk::default();
        let mut screen = Screen::default();
        card_with(&mut screen, &sdk, &white_card, (0, 4));
        assert!(black(&sdk) == screen_where(on_edge), "the outline alone");
        card_with(&mut screen, &sdk, &white_card, (1, 2));
        let half = |x: i32, y: i32| on_edge(x, y) || (in_bar(x, y) && x <= left + 99);
        assert!(black(&sdk) == screen_where(half), "half its inside");
        card_with(&mut screen, &sdk, &white_card, (2, 2));
        assert!(black(&sdk) == screen_where(in_bar), "all of it");
    }

    #[test]
    fn a_card_without_a_count_has_no_bar() {
        let sdk = FakeSdk::default();
        let mut screen = Screen::default();
        let card = Card::text("Toy");
        screen.present(&sdk, &Frame::new(Show::Card(&card)));
        assert_eq!(sdk.frame_uses(), 0);
        assert!(inside(&sdk).iter().all(|white| !white), "the black card");
    }

    #[test]
    fn neighbouring_rows_are_one_run() {
        let got: Vec<_> = runs(&mask(&[3, 4, 5, 9, 239])).collect();
        assert_eq!(got, vec![(3, 5), (9, 9), (239, 239)]);
        assert_eq!(runs(&mask(&[])).count(), 0);
    }
}
