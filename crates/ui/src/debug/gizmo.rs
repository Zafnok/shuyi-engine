//! The debug view's data (ADR-0085): what a screen hands in to be drawn
//! over the running game ([`Gizmo`]s, each in a [`Channel`] with a
//! [`GizmoRole`]), one frame's list of them ([`Gizmos`]), and which
//! channels are on ([`DebugView`], on [`Ctx::debug_view`]).
//!
//! A screen hands its gizmos in from
//! [`Screen::debug_draw`](crate::screen::Screen::debug_draw), from `&self`
//! and the context alone, so looking changes nothing. A channel's helper
//! asks [`Gizmos::channel_on`] first and skips the work when it is off:
//!
//! ```
//! use shuyi_ui::debug::gizmo::{Channel, Gizmo, GizmoRole, Gizmos};
//! use shuyi_ui::frame::Rect;
//!
//! const BOXES: Channel = Channel::new("boxes", "Hit boxes");
//!
//! fn boxes(hits: &[Rect], out: &mut Gizmos) {
//!     if !out.channel_on(BOXES) {
//!         return;
//!     }
//!     for &rect in hits {
//!         out.push(BOXES, GizmoRole::Solid, Gizmo::Rect { rect });
//!     }
//! }
//!
//! let mut out = Gizmos::with_on(["boxes"]);
//! boxes(&[Rect::new(8, 16, 8, 8)], &mut out);
//! assert_eq!(out.len(), 1);
//! assert_eq!(out.offered(), [BOXES]);
//! ```
//!
//! [`Ctx::debug_view`]: crate::Ctx::debug_view

use alloc::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::frame::glyph_buffer::PxRect;
use crate::prelude::*;

pub use crate::frame::gizmo::GizmoRole;

/// A shape the debug view draws over the frame, in console pixels, like
/// the frame's items. Its colour comes from its role
/// ([`GizmoRole::color`]), never from the screen.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Gizmo {
    /// The outline of a rectangle, one pixel wide, inside it.
    Rect {
        /// The rectangle.
        rect: PxRect,
    },
    /// A line one pixel wide, both ends drawn.
    Line {
        /// Its first pixel.
        from: (i32, i32),
        /// Its last pixel.
        to: (i32, i32),
    },
    /// A plus, `size` pixels each way from its centre.
    Cross {
        /// Its centre.
        at: (i32, i32),
        /// How far each arm reaches.
        size: u8,
    },
    /// The outline of a circle, `2 × radius + 1` pixels across.
    Circle {
        /// Its centre.
        center: (i32, i32),
        /// Its radius.
        radius: u16,
    },
    /// Text, ASCII only (debug tools are English only), in the glyph
    /// cells from the one holding pixel `at`, in the frame's font. Up to
    /// [`LABEL_MAX`](crate::frame::gizmo::LABEL_MAX) characters are kept.
    Label {
        /// A pixel of its first cell.
        at: (i32, i32),
        /// The text.
        text: String,
    },
}

/// A kind of thing the debug view shows, turned on and off as one from
/// the debug menu: a stable `snake_case` id (`grid`, `collision`), which
/// is what is saved, and a label for the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Channel {
    id: &'static str,
    label: &'static str,
}

impl Channel {
    /// The channel `id`, called `label` on the menu.
    pub const fn new(id: &'static str, label: &'static str) -> Self {
        Self { id, label }
    }

    /// Its id.
    pub const fn id(self) -> &'static str {
        self.id
    }

    /// Its name on the menu.
    pub const fn label(self) -> &'static str {
        self.label
    }
}

/// A gizmo handed in, with its channel and role.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Drawn {
    /// Its channel.
    pub channel: Channel,
    /// What it shows.
    pub role: GizmoRole,
    /// Its shape.
    pub gizmo: Gizmo,
}

/// What one frame hands in: the gizmos of the channels that are on, in
/// the order they were pushed, and every channel offered, on or off. It
/// knows the frame's pixels ([`frame`](Self::frame)), so a helper can keep
/// its gizmos on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gizmos {
    /// The frame's pixels.
    frame: PxRect,
    /// The ids of the channels that are on.
    on: BTreeSet<String>,
    /// Every channel is on, whatever its id.
    all: bool,
    /// The channels offered, in the order first offered.
    offered: Vec<Channel>,
    /// The gizmos kept.
    drawn: Vec<Drawn>,
}

/// The frame of a list that wasn't told its own: about all of `i32`.
const ANY_FRAME: PxRect = PxRect::new(-(1 << 30), -(1 << 30), i32::MAX, i32::MAX);

impl Default for Gizmos {
    fn default() -> Self {
        Self {
            frame: ANY_FRAME,
            on: BTreeSet::new(),
            all: false,
            offered: Vec::new(),
            drawn: Vec::new(),
        }
    }
}

impl Gizmos {
    /// A frame's list with no channel on: it only gathers the channels
    /// offered.
    pub fn new() -> Self {
        Self::default()
    }

    /// The same list for a frame of `pixels` (the buffer's
    /// [`pixel_bounds`](crate::frame::GlyphBuffer::pixel_bounds)).
    #[must_use]
    pub fn within(self, pixels: PxRect) -> Self {
        Self {
            frame: pixels,
            ..self
        }
    }

    /// The frame's pixels: what a helper keeps its gizmos inside. Nearly
    /// all of `i32` unless the list was told ([`within`](Self::within));
    /// the game tells it the buffer's.
    pub fn frame(&self) -> PxRect {
        self.frame
    }

    /// A frame's list with the channels of these ids on.
    pub fn with_on<'a>(ids: impl IntoIterator<Item = &'a str>) -> Self {
        Self {
            on: ids.into_iter().map(str::to_owned).collect(),
            ..Self::default()
        }
    }

    /// A frame's list with every channel on.
    pub fn all_on() -> Self {
        Self {
            all: true,
            ..Self::default()
        }
    }

    /// Whether `channel` is on, which also offers it (the debug menu lists
    /// it). A channel's helper asks first and does nothing more when it is
    /// off.
    pub fn channel_on(&mut self, channel: Channel) -> bool {
        if !self.offered.contains(&channel) {
            self.offered.push(channel);
        }
        self.all || self.on.contains(channel.id)
    }

    /// Adds `gizmo` of `channel`, showing `role`, after those already
    /// pushed, if the channel is on; it is dropped if not. Either way the
    /// channel is offered.
    pub fn push(&mut self, channel: Channel, role: GizmoRole, gizmo: Gizmo) {
        if self.channel_on(channel) {
            self.drawn.push(Drawn {
                channel,
                role,
                gizmo,
            });
        }
    }

    /// The gizmos kept, in the order pushed.
    pub fn drawn(&self) -> &[Drawn] {
        &self.drawn
    }

    /// How many gizmos were kept.
    pub fn len(&self) -> usize {
        self.drawn.len()
    }

    /// Whether none was kept.
    pub fn is_empty(&self) -> bool {
        self.drawn.is_empty()
    }

    /// Every channel offered, on or off, in the order first offered.
    pub fn offered(&self) -> &[Channel] {
        &self.offered
    }
}

/// Which channels of the debug view are on, and which the running screens
/// have offered ([`Ctx::debug_view`](crate::Ctx::debug_view)). None is on
/// at first. Turned on from the debug menu, which saves the choice
/// ([`Ctx::set_debug_channel`](crate::Ctx::set_debug_channel)); a shipped
/// build never reads or writes it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DebugView {
    /// The ids of the channels that are on.
    on: BTreeSet<String>,
    /// Every channel is on, offered or not ([`show_all`](Self::show_all)).
    all: bool,
    /// The channels offered since the game started, in the order first
    /// offered: kept when the screens that offer them are hidden (by the
    /// debug menu itself).
    offered: Vec<Channel>,
}

/// The saved form of a [`DebugView`]: the channels on, by id.
#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename = "DebugView")]
struct Saved {
    on: Vec<String>,
}

impl DebugView {
    /// Whether the channel `id` is on.
    pub fn is_on(&self, id: &str) -> bool {
        self.all || self.on.contains(id)
    }

    /// Whether any channel is on.
    pub fn any_on(&self) -> bool {
        self.all || !self.on.is_empty()
    }

    /// The ids of the channels on, in order (not those only
    /// [`show_all`](Self::show_all) turned on).
    pub fn on(&self) -> impl Iterator<Item = &str> {
        self.on.iter().map(String::as_str)
    }

    /// Turns the channel `id` on or off. It need not be offered yet: a
    /// screen may offer it later.
    pub fn set(&mut self, id: &str, on: bool) {
        if on {
            self.on.insert(id.to_owned());
        } else {
            self.on.remove(id);
        }
    }

    /// Turns every channel on, offered now or later (`frame-png --debug
    /// all`). Not saved.
    pub fn show_all(&mut self) {
        self.all = true;
    }

    /// The channels the running screens have offered, in the order first
    /// offered.
    pub fn offered(&self) -> &[Channel] {
        &self.offered
    }

    /// Keeps `channels` among those offered, after those already there.
    pub fn offer(&mut self, channels: &[Channel]) {
        for &channel in channels {
            if !self.offered.contains(&channel) {
                self.offered.push(channel);
            }
        }
    }

    /// A frame's empty list, with this view's channels on.
    pub fn gizmos(&self) -> Gizmos {
        Gizmos {
            on: self.on.clone(),
            all: self.all,
            ..Gizmos::default()
        }
    }

    /// The saved form: the ids of the channels on (RON).
    pub fn to_ron(&self) -> String {
        let saved = Saved {
            on: self.on.iter().cloned().collect(),
        };
        ron::to_string(&saved).unwrap_or_default()
    }

    /// The channels on, read from the saved form; what was offered is
    /// kept.
    ///
    /// # Errors
    ///
    /// Why the text can't be read.
    // check-text: not player text (warnings for the log)
    pub fn read_ron(&mut self, text: &str) -> Result<(), String> {
        let saved: Saved = ron::from_str(text).map_err(|e| format!("unreadable: {e}"))?;
        self.on = saved.on.into_iter().collect();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::glyph_buffer::Rect;

    const A: Channel = Channel::new("a", "Channel A");
    const B: Channel = Channel::new("b", "Channel B");

    fn rect(x: i32) -> Gizmo {
        Gizmo::Rect {
            rect: Rect::new(x, 0, 1, 1),
        }
    }

    #[test]
    fn push_keeps_the_order_filled_and_skips_a_channel_that_is_off() {
        let mut out = Gizmos::with_on(["a"]);
        out.push(A, GizmoRole::Bounds, rect(0));
        out.push(B, GizmoRole::Note, rect(1));
        out.push(A, GizmoRole::Note, rect(2));
        let kept: Vec<_> = out.drawn().iter().map(|d| (d.channel, d.role)).collect();
        assert_eq!(kept, [(A, GizmoRole::Bounds), (A, GizmoRole::Note)]);
        assert_eq!(out.drawn()[1].gizmo, rect(2));
        assert_eq!(out.len(), 2);
        // Both were offered, in the order first offered.
        assert_eq!(out.offered(), [A, B]);
    }

    #[test]
    fn channel_on_offers_the_channel_and_says_whether_it_is_on() {
        let mut none = Gizmos::new();
        assert!(!none.channel_on(B));
        assert!(!none.channel_on(B));
        assert!(none.is_empty());
        assert_eq!(none.offered(), [B]);
        let mut all = Gizmos::all_on();
        assert!(all.channel_on(A) && all.channel_on(B));
        assert_eq!((A.id(), A.label()), ("a", "Channel A"));
    }

    #[test]
    fn a_view_turns_channels_on_keeps_what_was_offered_and_saves_the_ones_on() {
        let mut view = DebugView::default();
        assert!(!view.any_on() && !view.is_on("a"));
        view.set("b", true);
        view.set("a", true);
        view.set("a", false);
        assert!(view.is_on("b") && !view.is_on("a") && view.any_on());
        view.offer(&[B, A]);
        view.offer(&[A]);
        assert_eq!(view.offered(), [B, A]);
        let mut gizmos = view.gizmos();
        assert!(gizmos.channel_on(B) && !gizmos.channel_on(A));

        let saved = view.to_ron();
        assert_eq!(saved, r#"(on:["b"])"#);
        let mut back = DebugView::default();
        back.read_ron(&saved).unwrap();
        assert_eq!(back.on().collect::<Vec<_>>(), ["b"]);
        assert!(back.read_ron("(on: 3)").is_err());
        // What is missing reads as nothing on.
        back.read_ron("()").unwrap();
        assert!(!back.any_on());

        view.show_all();
        assert!(view.is_on("anything") && view.gizmos().channel_on(A));
        // Showing all isn't saved.
        assert_eq!(view.to_ron(), r#"(on:["b"])"#);
    }
}
