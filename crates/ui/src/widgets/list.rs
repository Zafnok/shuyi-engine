//! A cursor over a list: the focus that moves up and down `n` rows, and
//! the window of rows a box shows when the list is longer than the box.
//!
//! The [`Menu`](super::Menu) keeps its focus in one; a screen with its own
//! list (a shop, a level select, a save list) keeps one too, instead of
//! stepping an index by hand. The cursor knows nothing of what the rows
//! are: whether a row can take the focus is a predicate the caller passes.

use core::ops::Range;

/// How the window follows the focus.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Scroll {
    /// The window moves only when the focus would leave it, and then only
    /// as far as it must: by one row when the focus steps past an edge.
    #[default]
    KeepInView,
    /// The focus sits in the middle of the window, except near the ends of
    /// the list, where the window stops at the first or the last row.
    Centred,
}

/// A focus over `len` rows and the window of rows shown around it.
///
/// The window's place lives in the cursor, so it doesn't jump from frame
/// to frame: ask for it with [`window`](Self::window) (rows of one height)
/// or [`window_by_heights`](Self::window_by_heights) each time the list is
/// shown.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ListCursor {
    len: usize,
    focus: usize,
    /// The first row of the window the last time it was asked for.
    first: usize,
    scroll: Scroll,
}

impl ListCursor {
    /// A cursor on the first of `len` rows, keeping the focus in view.
    pub fn new(len: usize) -> Self {
        Self {
            len,
            ..Self::default()
        }
    }

    /// The same cursor, its window following the focus by `scroll`.
    #[must_use]
    pub fn scrolling(mut self, scroll: Scroll) -> Self {
        self.scroll = scroll;
        self
    }

    /// The same cursor on row `index` (the last row, if there is no such
    /// row).
    #[must_use]
    pub fn at(mut self, index: usize) -> Self {
        self.focus = index.min(self.len.saturating_sub(1));
        self
    }

    /// The focused row. With no rows it is 0, which isn't a row.
    pub fn focus(&self) -> usize {
        self.focus
    }

    /// How many rows there are.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no rows.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// How the window follows the focus.
    pub fn scroll(&self) -> Scroll {
        self.scroll
    }

    /// The first row of the window, as the last [`window`](Self::window)
    /// or [`window_by_heights`](Self::window_by_heights) left it.
    pub fn first_shown(&self) -> usize {
        self.first
    }

    /// Moves the focus to the next (`forward`) or previous row that
    /// `enabled` accepts, going round the ends if `wrap`, else stopping at
    /// them. Returns whether the focus moved, so the caller knows to play
    /// its move sound. With no such row it stays put.
    pub fn step(&mut self, forward: bool, wrap: bool, enabled: impl Fn(usize) -> bool) -> bool {
        let (n, at) = (self.len, self.focus);
        let next = if wrap {
            (1..n)
                .map(|k| {
                    if forward {
                        (at + k) % n
                    } else {
                        (at + n - k) % n
                    }
                })
                .find(|&i| enabled(i))
        } else if forward {
            (at + 1..n).find(|&i| enabled(i))
        } else {
            (0..at.min(n)).rev().find(|&i| enabled(i))
        };
        match next {
            Some(i) => {
                self.focus = i;
                true
            }
            None => false,
        }
    }

    /// Moves the focus off a row `enabled` rejects: to the next row it
    /// accepts, else the nearest one before. Returns whether it moved. For
    /// after the rows changed (a row was used up, or removed).
    pub fn settle(&mut self, enabled: impl Fn(usize) -> bool) -> bool {
        if self.is_empty() || enabled(self.focus) {
            return false;
        }
        self.step(true, false, &enabled) || self.step(false, false, &enabled)
    }

    /// Sets the number of rows, after some were added or removed. A focus
    /// past the new end moves to the last row; the window is clamped the
    /// next time it is asked for.
    pub fn set_len(&mut self, len: usize) {
        self.len = len;
        self.focus = self.focus.min(len.saturating_sub(1));
    }

    /// The rows a box of `rows_shown` rows shows: always the focus, placed
    /// by the cursor's [`Scroll`] rule, and never past the end of the list
    /// while there are rows before it to show instead. A box of no rows
    /// still shows the focused one. With no rows it is empty.
    pub fn window(&mut self, rows_shown: usize) -> Range<usize> {
        if self.is_empty() {
            self.first = 0;
            return 0..0;
        }
        let rows = rows_shown.max(1).min(self.len);
        let last_first = self.len - rows;
        let first = match self.scroll {
            // As little as it takes: the focus no higher than the top row
            // and no lower than the bottom one.
            Scroll::KeepInView => self
                .first
                .clamp((self.focus + 1).saturating_sub(rows), self.focus),
            Scroll::Centred => self.focus.saturating_sub((rows - 1) / 2),
        };
        self.first = first.min(last_first);
        self.first..self.first + rows
    }

    /// The rows a box `room` tall shows when the rows have different
    /// heights (`heights[i]` is row `i`'s, in the box's units: a backlog of
    /// wrapped lines). As [`window`](Self::window): always the focus, even
    /// a focused row taller than the box, placed by the [`Scroll`] rule,
    /// and as many rows as fit. `heights` has one entry per row.
    pub fn window_by_heights(&mut self, heights: &[usize], room: usize) -> Range<usize> {
        if self.is_empty() {
            self.first = 0;
            return 0..0;
        }
        let focus = self.focus;
        let height = |r: Range<usize>| -> usize { heights[r].iter().sum() };
        let mut first = match self.scroll {
            Scroll::KeepInView => self.first.min(focus),
            Scroll::Centred => {
                let half = room.saturating_sub(heights[focus]) / 2;
                let mut first = focus;
                while first > 0 && height(first - 1..focus) <= half {
                    first -= 1;
                }
                first
            }
        };
        // The focus in view: drop rows from the top until it fits.
        while first < focus && height(first..focus + 1) > room {
            first += 1;
        }
        // No empty room at the bottom while there are rows above to show.
        while first > 0 && height(first - 1..self.len) <= room {
            first -= 1;
        }
        let mut end = focus + 1;
        while end < self.len && height(first..end + 1) <= room {
            end += 1;
        }
        self.first = first;
        first..end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn all(_: usize) -> bool {
        true
    }

    #[test]
    fn stepping_wraps_or_stops_at_the_ends() {
        let mut c = ListCursor::new(3);
        assert!(c.step(false, true, all));
        assert_eq!(c.focus(), 2, "wrapped to the end");
        assert!(c.step(true, true, all));
        assert_eq!(c.focus(), 0, "wrapped to the start");
        assert!(!c.step(false, false, all), "stops at the start");
        assert_eq!(c.focus(), 0);
        let mut c = c.at(2);
        assert!(!c.step(true, false, all), "stops at the end");
        assert_eq!(c.focus(), 2);
        assert!(c.step(false, false, all));
        assert_eq!(c.focus(), 1);
    }

    #[test]
    fn disabled_rows_are_skipped() {
        let on = [true, false, false, true, false];
        let enabled = |i: usize| on[i];
        let mut c = ListCursor::new(5);
        assert!(c.step(true, true, enabled));
        assert_eq!(c.focus(), 3);
        assert!(c.step(true, true, enabled));
        assert_eq!(c.focus(), 0);
        // Without wrap, nothing enabled after 3: it stays.
        let mut c = c.at(3);
        assert!(!c.step(true, false, enabled));
        assert_eq!(c.focus(), 3);
    }

    #[test]
    fn every_row_disabled_but_one_keeps_the_focus() {
        let enabled = |i: usize| i == 2;
        let mut c = ListCursor::new(5).at(2);
        for (forward, wrap) in [(true, true), (false, true), (true, false), (false, false)] {
            assert!(!c.step(forward, wrap, enabled));
            assert_eq!(c.focus(), 2);
        }
        // From elsewhere, both ways find it.
        let mut c = ListCursor::new(5).at(4);
        assert!(c.step(true, true, enabled));
        assert_eq!(c.focus(), 2);
    }

    #[test]
    fn an_empty_list_never_moves() {
        let mut c = ListCursor::new(0);
        assert!(!c.step(true, true, all));
        assert!(!c.step(false, false, all));
        assert!(!c.settle(all));
        assert_eq!(c.focus(), 0);
        assert_eq!(c.window(4), 0..0);
        assert_eq!(c.window_by_heights(&[], 4), 0..0);
        assert!(c.is_empty());
    }

    #[test]
    fn a_removal_at_the_end_clamps_the_focus() {
        let mut c = ListCursor::new(6).at(5);
        assert_eq!(c.window(3), 3..6);
        c.set_len(4);
        assert_eq!((c.focus(), c.len()), (3, 4));
        assert_eq!(c.window(3), 1..4);
        c.set_len(0);
        assert_eq!(c.focus(), 0);
        assert_eq!(c.window(3), 0..0);
    }

    #[test]
    fn settle_leaves_a_disabled_row() {
        let on = [true, false, true, false];
        let mut c = ListCursor::new(4).at(1);
        assert!(c.settle(|i| on[i]));
        assert_eq!(c.focus(), 2, "the next enabled row first");
        let mut c = c.at(3);
        assert!(c.settle(|i| on[i]));
        assert_eq!(c.focus(), 2, "else the one before");
        assert!(!c.settle(|i| on[i]), "already on one");
        assert!(!c.settle(|_| false), "nowhere to go");
    }

    #[test]
    fn keep_in_view_moves_the_window_by_one_at_its_edge() {
        let mut c = ListCursor::new(10);
        assert_eq!(c.window(4), 0..4);
        for _ in 0..3 {
            c.step(true, true, all);
            assert_eq!(c.window(4), 0..4, "inside: the window stays");
        }
        c.step(true, true, all);
        assert_eq!(c.window(4), 1..5, "past the bottom edge: by one");
        c.step(true, true, all);
        assert_eq!(c.window(4), 2..6);
        // Back up inside the window: it stays, until the top edge.
        for _ in 0..3 {
            c.step(false, true, all);
            assert_eq!(c.window(4), 2..6);
        }
        c.step(false, true, all);
        assert_eq!((c.focus(), c.window(4)), (1, 1..5));
        assert_eq!(c.first_shown(), 1);
        // Wrapping round jumps to the other end.
        c.step(false, true, all);
        c.step(false, true, all);
        assert_eq!((c.focus(), c.window(4)), (9, 6..10));
    }

    #[test]
    fn centred_keeps_the_focus_in_the_middle() {
        let mut c = ListCursor::new(20).scrolling(Scroll::Centred);
        assert_eq!(c.scroll(), Scroll::Centred);
        assert_eq!(c.window(5), 0..5, "the top stops at the first row");
        let mut c = c.at(10);
        assert_eq!(c.window(5), 8..13);
        c.step(true, true, all);
        assert_eq!(c.window(5), 9..14);
        let mut c = c.at(19);
        assert_eq!(c.window(5), 15..20, "the bottom stops at the last row");
        // An even box puts the focus just above the middle.
        let mut c = c.at(10);
        assert_eq!(c.window(6), 8..14);
    }

    #[test]
    fn a_short_list_shows_every_row() {
        let mut c = ListCursor::new(3).at(2);
        assert_eq!(c.window(6), 0..3);
        assert_eq!(c.window(0), 2..3, "a box of no rows still shows the focus");
    }

    #[test]
    fn rows_of_different_heights() {
        let heights = [1, 3, 2, 1, 4, 1];
        let mut c = ListCursor::new(6);
        assert_eq!(c.window_by_heights(&heights, 6), 0..3);
        c = c.at(3);
        assert_eq!(c.window_by_heights(&heights, 6), 1..4, "drops the top row");
        c = c.at(4);
        assert_eq!(c.window_by_heights(&heights, 6), 3..6);
        // Back up inside: the window stays.
        c = c.at(3);
        assert_eq!(c.window_by_heights(&heights, 6), 3..6);
        c = c.at(2);
        assert_eq!(c.window_by_heights(&heights, 6), 2..4);
        // A row taller than the box is still shown, alone.
        c = c.at(4);
        assert_eq!(c.window_by_heights(&heights, 3), 4..5);
        // The whole list fits a tall box.
        assert_eq!(c.window_by_heights(&heights, 20), 0..6);
        // A box that grows pulls the window back up, to an exact fit.
        let mut c = ListCursor::new(3).at(2);
        assert_eq!(c.window_by_heights(&[1, 1, 1], 1), 2..3);
        assert_eq!(c.window_by_heights(&[1, 1, 1], 3), 0..3);
    }

    #[test]
    fn rows_of_different_heights_centred() {
        let heights = [1, 1, 1, 1, 1, 3, 1, 1, 1, 1, 1];
        let mut c = ListCursor::new(11).scrolling(Scroll::Centred).at(5);
        // Room 7, the focus 3 tall: 2 above, 2 below.
        assert_eq!(c.window_by_heights(&heights, 7), 3..8);
        let mut c = c.at(0);
        assert_eq!(c.window_by_heights(&heights, 7), 0..5);
    }

    proptest! {
        #[test]
        fn the_focus_is_in_the_window_and_on_an_enabled_row(
            on in prop::collection::vec(any::<bool>(), 0..30),
            heights in prop::collection::vec(1usize..5, 30),
            moves in prop::collection::vec((any::<bool>(), any::<bool>()), 0..40),
            rows in 0usize..10,
            room in 0usize..15,
            centred in any::<bool>(),
            removed in 0usize..30,
        ) {
            let enabled = |i: usize| on[i];
            let scroll = if centred { Scroll::Centred } else { Scroll::KeepInView };
            let mut c = ListCursor::new(on.len()).scrolling(scroll);
            c.settle(enabled);
            let check = |c: &mut ListCursor| -> Result<(), TestCaseError> {
                if c.is_empty() {
                    return Ok(());
                }
                prop_assert!(c.focus() < c.len());
                let w = c.window(rows);
                prop_assert!(w.contains(&c.focus()));
                prop_assert!(w.len() <= rows.max(1));
                let w = c.window_by_heights(&heights[..c.len()], room);
                prop_assert!(w.contains(&c.focus()));
                if on[..c.len()].iter().any(|&e| e) {
                    prop_assert!(on[c.focus()]);
                }
                Ok(())
            };
            check(&mut c)?;
            for (forward, wrap) in moves {
                c.step(forward, wrap, enabled);
                check(&mut c)?;
            }
            c.set_len(on.len().saturating_sub(removed));
            c.settle(enabled);
            check(&mut c)?;
        }
    }
}
