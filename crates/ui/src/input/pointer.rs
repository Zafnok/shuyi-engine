//! The pointer per frame (ticket 2405, ADR-0134): a [`Pointer`] folds a
//! frame's pointer events ([`RawInputEvent::PointerMoved`] and the rest)
//! into one [`PointerInput`], which a screen reads beside its actions
//! ([`FrameInput::pointer`](super::FrameInput::pointer)).
//!
//! Hover follows motion only ([`PointerInput::moved`]), a click is a
//! press and a release on the same hit area ([`PointerInput::clicked`]),
//! and a touch is a pointer with no hover: its release comes with
//! [`RawInputEvent::PointerLeft`], and that frame still keeps its click.

use crate::frame::hit::Hit;
use crate::game::RawInputEvent;

/// The pointer in one frame, as a screen sees it: plain data.
///
/// Kind 4 of ADR-0039: a test builds one from [`PointerInput::default`]
/// or [`PointerInput::at`] and assigns the rest.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct PointerInput {
    /// Where it is, in console pixels of the frame last shown.
    pub at: (i32, i32),
    /// Whether it moved this frame (it came into the frame, or to another
    /// pixel): only then does hover move a focus.
    pub moved: bool,
    /// Whether the primary button went down this frame.
    pub pressed: bool,
    /// Whether the primary button went up this frame.
    pub released: bool,
    /// Where the press held now, or released this frame, began.
    pub down_at: Option<(i32, i32)>,
    /// The hit area under [`at`](Self::at) in the frame last shown
    /// ([`GlyphBuffer::hit_at`](crate::frame::GlyphBuffer::hit_at)).
    pub over: Option<Hit>,
    /// On a release, the hit area its press began on, if the release is
    /// over the same widget's same item: a press on one row and a release
    /// on another is no click.
    pub clicked: Option<Hit>,
    /// The wheel's whole notches this frame, positive away from the
    /// player.
    pub wheel: i32,
}

impl PointerInput {
    /// A pointer still at `(x, y)`, over nothing, with nothing pressed.
    pub fn at(x: i32, y: i32) -> Self {
        Self {
            at: (x, y),
            ..Self::default()
        }
    }
}

/// Where the pointer is between frames, and what its press began on: it
/// folds each frame's pointer events into that frame's [`PointerInput`].
/// [`Game`](crate::Game) keeps one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Pointer {
    /// Where it is, or `None` outside the frame (and before it ever came).
    at: Option<(i32, i32)>,
    /// Its last place, kept after it left: where a tap's frame says it was.
    last: (i32, i32),
    /// The press held: where it began, and the hit area there.
    press: Option<((i32, i32), Option<Hit>)>,
}

impl Pointer {
    /// Folds this frame's `events` (the pointer's among them, in order)
    /// into what the frame's screen sees, looking hit areas up by console
    /// pixel with `hit_at`: `None` while the pointer is outside the frame,
    /// unless its button went down or up this frame (a tap, whose release
    /// lifts the finger, keeps its click, over nothing).
    pub fn frame(
        &mut self,
        events: &[RawInputEvent],
        hit_at: impl Fn(i32, i32) -> Option<Hit>,
    ) -> Option<PointerInput> {
        let mut input = PointerInput::default();
        let mut released_from = None;
        for &event in events {
            match event {
                RawInputEvent::PointerMoved { x, y } => {
                    input.moved |= self.at != Some((x, y));
                    self.at = Some((x, y));
                    self.last = (x, y);
                }
                RawInputEvent::PointerLeft => self.at = None,
                RawInputEvent::PointerDown => {
                    // A press with no place, or a second without a
                    // release, is dropped.
                    if let (Some((x, y)), None) = (self.at, self.press) {
                        self.press = Some(((x, y), hit_at(x, y)));
                        input.pressed = true;
                    }
                }
                RawInputEvent::PointerUp => {
                    if let Some((from, hit)) = self.press.take() {
                        input.released = true;
                        released_from = Some(from);
                        let here = self.at.and_then(|(x, y)| hit_at(x, y));
                        input.clicked = hit.filter(|h| here.is_some_and(|o| h.same_target(&o)));
                    }
                }
                RawInputEvent::Wheel(notches) => {
                    input.wheel = input.wheel.saturating_add(notches);
                }
                _ => {}
            }
        }
        if self.at.is_none() && !input.pressed && !input.released {
            return None;
        }
        input.at = self.at.unwrap_or(self.last);
        input.over = self.at.and_then(|(x, y)| hit_at(x, y));
        input.down_at = self.press.map(|(from, _)| from).or(released_from);
        Some(input)
    }

    /// Forgets the press held (its release is then no click) and the
    /// pointer's place: until it moves again there is no pointer. For a
    /// window that lost the focus, which hears no release.
    pub fn forget(&mut self) {
        self.at = None;
        self.press = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::glyph_buffer::Rect;
    use crate::frame::hit::WidgetKey;
    use crate::prelude::*;
    use RawInputEvent::{PointerDown, PointerLeft, PointerUp, Wheel};

    fn moved(x: i32, y: i32) -> RawInputEvent {
        RawInputEvent::PointerMoved { x, y }
    }

    /// Two rows of a menu, 16 pixels each, from (0, 0): row 0, then row 1.
    fn rows(_: i32, y: i32) -> Option<Hit> {
        let row = u32::try_from(y.div_euclid(16)).ok().filter(|&r| r < 2)?;
        let top = i32::try_from(row).ok()? * 16;
        Some(Hit::new(Rect::new(0, top, 80, 16), WidgetKey(0), row))
    }

    fn frames(pointer: &mut Pointer, frames: &[&[RawInputEvent]]) -> Vec<Option<PointerInput>> {
        frames
            .iter()
            .map(|events| pointer.frame(events, rows))
            .collect()
    }

    #[test]
    fn there_is_no_pointer_until_it_comes_and_after_it_leaves() {
        let mut p = Pointer::default();
        assert_eq!(p.frame(&[], rows), None);
        assert_eq!(p.frame(&[Wheel(1)], rows), None, "the wheel outside");
        let got = p.frame(&[moved(3, 20)], rows).unwrap();
        assert_eq!((got.at, got.moved), ((3, 20), true));
        assert_eq!(got.over.and_then(|h| h.item()), Some(1));
        assert_eq!(p.frame(&[PointerLeft], rows), None);
        assert_eq!(p.frame(&[PointerDown], rows), None, "a press with no place");
    }

    #[test]
    fn it_moved_only_when_its_place_changed() {
        let mut p = Pointer::default();
        let got = frames(
            &mut p,
            &[&[moved(1, 1)], &[], &[moved(1, 1)], &[moved(2, 1)]],
        );
        let moves: Vec<_> = got.iter().map(|f| f.unwrap().moved).collect();
        assert_eq!(moves, [true, false, false, true]);
    }

    #[test]
    fn a_press_and_a_release_on_the_same_hit_click_it() {
        let mut p = Pointer::default();
        let got = frames(
            &mut p,
            &[&[moved(5, 2), PointerDown], &[moved(9, 3)], &[PointerUp]],
        );
        let pressed = got[0].unwrap();
        assert!(pressed.pressed && !pressed.released);
        assert_eq!(pressed.down_at, Some((5, 2)));
        assert_eq!(got[1].unwrap().down_at, Some((5, 2)), "held");
        let released = got[2].unwrap();
        assert!(released.released && !released.pressed);
        assert_eq!(released.down_at, Some((5, 2)));
        assert_eq!(released.clicked.and_then(|h| h.item()), Some(0));
        // The next frame, nothing held.
        assert_eq!(p.frame(&[], rows).unwrap().down_at, None);
    }

    #[test]
    fn a_press_here_and_a_release_there_is_no_click() {
        let mut p = Pointer::default();
        let got = frames(
            &mut p,
            &[&[moved(5, 2), PointerDown], &[moved(5, 20), PointerUp]],
        );
        let released = got[1].unwrap();
        assert!(released.released);
        assert_eq!(released.clicked, None);
        assert_eq!(released.over.and_then(|h| h.item()), Some(1));
        // Nor a release over nothing.
        let got = frames(&mut p, &[&[PointerDown], &[moved(5, 40), PointerUp]]);
        assert_eq!(got[1].unwrap().clicked, None);
    }

    #[test]
    fn a_press_and_a_release_in_one_frame_click() {
        let mut p = Pointer::default();
        let got = p
            .frame(&[moved(5, 2), PointerDown, PointerUp], rows)
            .unwrap();
        assert!(got.pressed && got.released);
        assert_eq!(got.clicked.and_then(|h| h.item()), Some(0));
    }

    /// A tap: the finger lifts with its release, and the frame keeps the
    /// click, over nothing; the next frame has no pointer.
    #[test]
    fn a_release_that_leaves_in_the_same_frame_still_clicks() {
        let mut p = Pointer::default();
        let got = p
            .frame(&[moved(5, 20), PointerDown, PointerUp, PointerLeft], rows)
            .unwrap();
        assert_eq!(got.clicked.and_then(|h| h.item()), Some(1));
        assert_eq!((got.at, got.over), ((5, 20), None));
        assert_eq!(p.frame(&[], rows), None);
    }

    #[test]
    fn the_wheels_notches_add_up_in_a_frame() {
        let mut p = Pointer::default();
        let got = p.frame(&[moved(0, 0), Wheel(2), Wheel(-3)], rows).unwrap();
        assert_eq!(got.wheel, -1);
        assert_eq!(p.frame(&[], rows).unwrap().wheel, 0);
    }

    #[test]
    fn a_forgotten_press_clicks_nothing_and_the_place_is_forgotten() {
        let mut p = Pointer::default();
        p.frame(&[moved(5, 2), PointerDown], rows);
        p.forget();
        assert_eq!(p.frame(&[], rows), None);
        let got = p.frame(&[moved(5, 2), PointerUp], rows).unwrap();
        assert!(!got.released);
        assert_eq!(got.clicked, None);
        assert_eq!(PointerInput::at(1, 2).at, (1, 2));
    }
}
