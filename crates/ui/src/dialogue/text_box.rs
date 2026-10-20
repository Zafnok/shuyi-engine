//! A dialogue's text box: its text word-wrapped into pages, typed out at
//! the player's text speed and faster while the key that goes on is held
//! ([`TextBox`]); what it shows ([`TextView`]: the page as revealed, the
//! replies of a choice, the question before skipping a scene, the marker
//! that waits for a key); and its glyph look ([`glyph::paint`]).

pub mod glyph;

use crate::prelude::*;

use crate::hurry::{HURRY_DELAY, HURRY_FACTOR, HurryClock};
use crate::widgets::menu::MenuView;
use crate::widgets::word_wrap;

/// Reveal speed multiplier while the key that goes on is held (the
/// [`HurryClock`]'s default factor).
pub const FAST_FORWARD: f32 = HURRY_FACTOR;
/// Seconds that key must be held on a page before it reveals faster, so
/// the tap that turned the page doesn't speed the next one up (the
/// [`HurryClock`]'s default delay).
pub const FAST_FORWARD_DELAY: f32 = HURRY_DELAY;
/// The marker that says "press to go on" is lit for this long, then out
/// for as long, while a page waits.
pub const BLINK_S: f32 = 0.5;

/// The text box's state: the text, wrapped, the page on screen and how
/// much of it is revealed.
#[derive(Debug, Clone, PartialEq)]
pub struct TextBox {
    /// The text, word-wrapped.
    lines: Vec<String>,
    /// Lines per page; longer text pages.
    per_page: usize,
    /// Which page of `lines` is shown.
    page: usize,
    /// Characters of the page revealed so far (fractional between frames).
    shown: f32,
    /// How long the key that goes on has been held on this page, for the
    /// fast-forward ([`FAST_FORWARD_DELAY`]).
    hurry: HurryClock,
    /// Seconds since the page was fully revealed, for the blinking marker.
    waiting: f32,
}

/// Characters on a page.
fn page_len(lines: &[String]) -> f32 {
    let n: usize = lines.iter().map(|l| l.chars().count()).sum();
    // Pages are a few hundred characters at most, so this is exact.
    #[expect(clippy::cast_precision_loss, reason = "at most a few hundred")]
    let n = n as f32;
    n
}

impl TextBox {
    /// `text` wrapped at `width` characters, `per_page` lines a page, on
    /// its first page with nothing revealed yet.
    pub fn new(text: &str, width: usize, per_page: usize) -> Self {
        Self {
            lines: word_wrap(text, width),
            per_page: per_page.max(1),
            page: 0,
            shown: 0.0,
            hurry: HurryClock::default(),
            waiting: 0.0,
        }
    }

    /// The lines of the page on screen, in full.
    pub fn page_lines(&self) -> &[String] {
        let start = (self.page * self.per_page).min(self.lines.len());
        let end = (start + self.per_page).min(self.lines.len());
        &self.lines[start..end]
    }

    /// How many of the page's characters are revealed, with the part of
    /// the next one typed so far.
    pub fn progress(&self) -> f32 {
        self.shown
    }

    /// How many of the page's characters are revealed.
    pub fn shown(&self) -> usize {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "shown is between 0 and the page length"
        )]
        let shown = self.shown as usize;
        shown
    }

    /// Whether the whole page is revealed.
    pub fn is_revealed(&self) -> bool {
        self.shown >= page_len(self.page_lines())
    }

    /// Reveals the whole page at once.
    pub fn reveal(&mut self) {
        self.shown = page_len(self.page_lines());
    }

    /// Turns to the next page, if there is one: returns whether it did.
    pub fn next_page(&mut self) -> bool {
        if (self.page + 1) * self.per_page >= self.lines.len() {
            return false;
        }
        self.page += 1;
        self.start_page();
        true
    }

    /// What the key that goes on does: reveals the page being typed
    /// ([`Step::Revealed`]), else turns to the next page
    /// ([`Step::NextPage`]), else says the box is [`Step::Done`] and leaves
    /// it as it is (its owner moves on to the next line, or closes it).
    pub fn confirm(&mut self) -> Step {
        if !self.is_revealed() {
            self.reveal();
            Step::Revealed
        } else if self.next_page() {
            Step::NextPage
        } else {
            Step::Done
        }
    }

    /// Shows the last page, all of it revealed (the line a choice answers).
    pub fn to_last_page(&mut self) {
        self.page = self.lines.len().saturating_sub(1) / self.per_page;
        self.reveal();
    }

    /// The page starts over: nothing revealed, no key held, not waiting.
    fn start_page(&mut self) {
        self.shown = 0.0;
        self.hurry.reset();
        self.waiting = 0.0;
    }

    /// The key that goes on is no longer counted as held (the box waits on
    /// something else).
    pub fn release(&mut self) {
        self.hurry.release();
    }

    /// `dt` seconds on, with the key that goes on `held` or not.
    pub fn hold(&mut self, held: bool, dt: f32) {
        self.hurry.hold(held, dt);
    }

    /// `dt` seconds of typing at `chars_per_s` (`None`: the whole page at
    /// once), faster for the part of them past [`FAST_FORWARD_DELAY`] of
    /// holding; once the page is revealed, the time it has waited.
    pub fn type_on(&mut self, dt: f32, chars_per_s: Option<f32>) {
        if self.is_revealed() {
            self.waiting += dt;
            return;
        }
        // The part of this frame past the delay goes faster.
        let dt = self.hurry.passing(dt);
        let len = page_len(self.page_lines());
        self.shown = match chars_per_s {
            Some(speed) => (self.shown + dt * speed).min(len),
            None => len,
        };
    }

    /// Whether the marker that says "press to go on" is lit: it blinks
    /// while the page waits.
    pub fn marker_lit(&self) -> bool {
        self.waiting % (2.0 * BLINK_S) < BLINK_S
    }
}

/// What the key that goes on did to a text box ([`TextBox::confirm`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The page was being typed: now it is all revealed.
    Revealed,
    /// The page was revealed: the next one is on screen, typing.
    NextPage,
    /// The last page was revealed: the box is done.
    Done,
}

/// What the text box shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextView {
    /// Whether the text is narration (nobody says it).
    pub narration: bool,
    /// The page on screen, word-wrapped, in full. At a choice, the last
    /// page of the line being answered.
    pub lines: Vec<String>,
    /// How many of the page's characters are revealed, from its start.
    pub shown: usize,
    /// The replies, while a choice is open.
    pub choices: Option<MenuView>,
    /// The "Skip scene?" question, while it is open: shown instead of the
    /// text.
    pub skip: Option<SkipView>,
    /// The page, all shown, waiting for the key that goes on.
    pub waiting: Option<WaitView>,
}

impl TextView {
    /// The revealed part of each line of the page.
    pub fn revealed(&self) -> Vec<String> {
        let mut budget = self.shown;
        let mut revealed = Vec::new();
        for line in &self.lines {
            revealed.push(line.chars().take(budget).collect());
            budget = budget.saturating_sub(line.chars().count());
        }
        revealed
    }
}

/// The question asked before skipping the scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipView {
    /// The question.
    pub question: String,
    /// How to answer, with the keys.
    pub answers: String,
}

/// What shows while a page waits for the key that goes on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitView {
    /// The name of the key that goes on.
    pub key: String,
    /// How to open the log, if its action has a key.
    pub log_hint: Option<String>,
    /// Whether the marker that says "press to go on" is lit: it blinks.
    pub marker: bool,
}

#[cfg(test)]
mod tests;
