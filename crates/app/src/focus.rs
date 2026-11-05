//! Whether the window has the player's focus (ticket 3042, ADR-0076):
//! the system's [`SystemEvent::FocusLost`] and
//! [`FocusGained`](SystemEvent::FocusGained), from what miniquad reports.
//!
//! miniquad 0.4 reports a focus change as a minimize and a restore
//! (`window_minimized_event`, `window_restored_event`), on Windows, macOS,
//! X11, Wayland and the web. macroquad queues them, with the keys and the
//! mouse, for an input subscriber, which [`WindowFocus`] replays each frame
//! into what it records ([`WindowHeard`], in `keys.rs`, the one file that
//! reads the keyboard). [`Focus`] then turns what was heard
//! into the events, and smooths two quirks: Windows says minimized twice
//! on a minimize, and macOS says minimized when the window is moved, with
//! no restore after. A key or a mouse button pressed reaches only a window
//! with focus, so it says focus is back.

use macroquad::input::utils::{register_input_subscriber, repeat_all_miniquad_input};
use shuyi_platform::SystemEvent;

use crate::keys::WindowHeard;

/// What the window told miniquad, of what [`Focus`] reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Heard {
    /// It lost focus, or was minimized (or, on macOS, moved).
    Minimized,
    /// It got focus back, or was restored.
    Restored,
    /// A key or a mouse button went down in it.
    Pressed,
}

/// Whether the window has focus, from what it heard. It starts focused, as
/// `App` does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Focus {
    focused: bool,
}

impl Default for Focus {
    fn default() -> Self {
        Self { focused: true }
    }
}

impl Focus {
    /// Whether the window has focus.
    pub(crate) fn is_focused(self) -> bool {
        self.focused
    }

    /// What `heard` means to the game: a change of focus, or nothing when
    /// it changes nothing (a second minimize, a restore or a press while
    /// focused).
    pub(crate) fn hear(&mut self, heard: Heard) -> Option<SystemEvent> {
        let focused = !matches!(heard, Heard::Minimized);
        if focused == self.focused {
            return None;
        }
        self.focused = focused;
        Some(if focused {
            SystemEvent::FocusGained
        } else {
            SystemEvent::FocusLost
        })
    }
}

/// The window's focus over macroquad's input queue: its subscriber, made
/// on the first frame read (so a run that never reads the window, a
/// capture's, queues nothing), what it heard and the focus.
#[derive(Debug, Default)]
pub(crate) struct WindowFocus {
    subscriber: Option<usize>,
    heard: WindowHeard,
    focus: Focus,
}

impl WindowFocus {
    /// The focus changes since the last call, in order, into `out`.
    pub(crate) fn events(&mut self, out: &mut Vec<SystemEvent>) {
        let subscriber = *self
            .subscriber
            .get_or_insert_with(register_input_subscriber);
        repeat_all_miniquad_input(&mut self.heard, subscriber);
        let focus = &mut self.focus;
        out.extend(self.heard.0.drain(..).filter_map(|heard| focus.hear(heard)));
    }

    /// Whether the window has focus, as of the last [`events`](Self::events).
    pub(crate) fn is_focused(&self) -> bool {
        self.focus.is_focused()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What `focus` makes of `heard`, in order.
    fn hear_all(focus: &mut Focus, heard: &[Heard]) -> Vec<SystemEvent> {
        heard.iter().filter_map(|&h| focus.hear(h)).collect()
    }

    #[test]
    fn it_starts_focused() {
        assert!(Focus::default().is_focused());
    }

    #[test]
    fn two_minimizes_in_a_row_lose_focus_once() {
        let mut focus = Focus::default();
        let events = hear_all(&mut focus, &[Heard::Minimized, Heard::Minimized]);
        assert_eq!(events, [SystemEvent::FocusLost]);
        assert!(!focus.is_focused());
    }

    #[test]
    fn a_restore_gains_focus_only_while_unfocused() {
        let mut focus = Focus::default();
        assert_eq!(hear_all(&mut focus, &[Heard::Restored]), []);
        let events = hear_all(&mut focus, &[Heard::Minimized, Heard::Restored]);
        assert_eq!(events, [SystemEvent::FocusLost, SystemEvent::FocusGained]);
        assert!(focus.is_focused());
    }

    /// macOS's move: a minimize with no restore after; the next press says
    /// the window has focus.
    #[test]
    fn a_press_gains_focus_only_while_unfocused() {
        let mut focus = Focus::default();
        assert_eq!(hear_all(&mut focus, &[Heard::Pressed]), []);
        let events = hear_all(
            &mut focus,
            &[Heard::Minimized, Heard::Pressed, Heard::Pressed],
        );
        assert_eq!(events, [SystemEvent::FocusLost, SystemEvent::FocusGained]);
        assert!(focus.is_focused());
    }
}
