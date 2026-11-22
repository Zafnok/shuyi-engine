//! Several pads as one (ticket 2412, game ADR-0034): each pad's events
//! come with its pad ([`Pads::update`]), and [`Game`] merges them here
//! before its [`InputState`] hears them, so a game sees one controller
//! however many are plugged in.
//!
//! [`Pads::update`]: crate::input::Pads::update
//! [`Game`]: super::Game
//! [`InputState`]: crate::input::InputState

use crate::prelude::*;
use alloc::collections::BTreeMap;

use shuyi_assets::keymap::Button;

use super::RawInputEvent;
use crate::input::pad::{ButtonSet, PadId};

/// The buttons each pad holds, and those the game holds as one.
#[derive(Debug, Clone, Default)]
pub(super) struct PadMerge {
    /// Each pad's buttons held, by binding position.
    held: BTreeMap<PadId, ButtonSet>,
    /// The buttons the game's input was told are down.
    down: ButtonSet,
}

impl PadMerge {
    /// `events` as the game's input hears them, in order. Keys and text
    /// pass as they are. Of the pad events:
    ///
    /// - a press is heard when the first pad presses the button, with that
    ///   pad's kind;
    /// - a release is heard when the last pad holding the button lets go,
    ///   unless another pad presses it later in `events` (the button
    ///   passes from one pad to the other in one poll, and stays held);
    /// - a removed pad's [`PadRemoved`](RawInputEvent::PadRemoved) is
    ///   heard as the releases of what it still held that way, in
    ///   [`Button::ALL`] order, and the pad is forgotten.
    ///
    /// What is heard is what one merged stream of every pad gave before
    /// pads had ids. A release no pad held is heard, as it was.
    pub(super) fn merge(&mut self, events: &[RawInputEvent]) -> Vec<RawInputEvent> {
        let mut heard = Vec::with_capacity(events.len());
        let mut rest = events;
        while let [event, later @ ..] = rest {
            rest = later;
            let event = *event;
            match event {
                RawInputEvent::PadDown(button, _, pad) => {
                    self.held.entry(pad).or_default().insert(button);
                    if !self.down.contains(button) {
                        self.down.insert(button);
                        heard.push(event);
                    }
                }
                RawInputEvent::PadUp(button, pad) => {
                    if let Some(set) = self.held.get_mut(&pad) {
                        set.remove(button);
                    }
                    if self.lets_go(button, pad, later) {
                        heard.push(event);
                    }
                }
                RawInputEvent::PadRemoved(pad) => {
                    let held = self.held.remove(&pad).unwrap_or_default();
                    for button in held.iter() {
                        if self.lets_go(button, pad, later) {
                            heard.push(RawInputEvent::PadUp(button, pad));
                        }
                    }
                }
                RawInputEvent::Down(_)
                | RawInputEvent::Up(_)
                | RawInputEvent::Text(_)
                | RawInputEvent::PointerMoved { .. }
                | RawInputEvent::PointerLeft
                | RawInputEvent::PointerDown
                | RawInputEvent::PointerUp
                | RawInputEvent::Wheel(_) => {
                    heard.push(event);
                }
            }
        }
        heard
    }

    /// Whether `pad` letting go of `button` lets the game's input go of it:
    /// no pad holds it now, and no other pad presses it in `later`. If so,
    /// the game no longer holds it.
    fn lets_go(&mut self, button: Button, pad: PadId, later: &[RawInputEvent]) -> bool {
        let held = self.held.values().any(|set| set.contains(button));
        let handed_on = later.iter().any(|event| {
            matches!(*event, RawInputEvent::PadDown(b, _, other) if b == button && other != pad)
        });
        if held || handed_on {
            return false;
        }
        self.down.remove(button);
        true
    }
}

#[cfg(test)]
mod tests;
