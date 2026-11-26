//! The "Debug view" screen of the debug menu (ADR-0085): every channel the
//! running screens have offered, each on or off, toggled with Confirm,
//! then slow motion (ADR-0153), whose speed Confirm moves on, under a line
//! saying whether the game is paused. The channels' choice is saved
//! ([`Ctx::set_debug_channel`]); the game shows the channels on over its
//! screens once the menu is closed.

use core::fmt::{self, Write as _};

use shuyi_assets::keymap::{ActionSet, UiAction};

use super::gizmo::DebugView;
use super::glyph;
use crate::ctx::{Ctx, GameData};
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::help_keys::{HelpKeys, cursor_keys_name, help_line, key_name};
use crate::log::{LogCategory, LogLevel};
use crate::prelude::*;
use crate::screen::{FrameInput, Screen, Transition};
use crate::widgets::menu::SuffixKind;
use crate::widgets::{Menu, MenuEvent, MenuItem, MenuView};

/// What the "Debug view" screen shows.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DebugViewView {
    /// Its title.
    pub title: String,
    /// Whether the game is paused: `Game running`, `Game paused`, or
    /// `Game paused, step N` after N steps.
    pub status: String,
    /// The channels, each with on or off after its name, then slow
    /// motion with its speed.
    pub menu: MenuView,
    /// What it says when no screen has offered a channel yet; `None` when
    /// some have.
    pub empty: Option<String>,
    /// The help line.
    pub help: String,
}

/// A row of the "Debug view" screen's menu.
///
/// A set that grows (ADR-0039, kind 6): a `match` on it has a `_` arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ViewRow {
    /// A channel, by its id: Confirm turns it on or off.
    Channel(&'static str),
    /// Slow motion: Confirm moves it on to the next speed
    /// ([`next_slow`]).
    SlowMotion,
}

/// The slow-motion speeds Confirm goes through, as one update in this many
/// frames: 1×, ½×, ¼×, ⅛×, then 1× again.
const SPEEDS: [u8; 4] = [1, 2, 4, 8];

/// The speed after one update in `every` frames, in the order 1, 2, 4, 8:
/// the next one up, or 1 after the slowest.
pub fn next_slow(every: u8) -> u8 {
    SPEEDS.into_iter().find(|&s| s > every).unwrap_or(1)
}

/// Slow motion's speed as the menu says it: `1x`, `1/2x`, `1/4x`…
fn speed_name(every: u8) -> String {
    match every {
        1 => "1x".to_owned(),
        n => format!("1/{n}x"),
    }
}

/// The channels offered, on or off, and slow motion: Confirm turns the
/// focused channel on or off or moves slow motion on, Cancel closes the
/// screen.
pub struct DebugViewScreen {
    /// Each item stands for its row.
    menu: Menu<ViewRow>,
}

impl fmt::Debug for DebugViewScreen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DebugViewScreen")
            .field("menu", &self.menu)
            .finish()
    }
}

impl DebugViewScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "debug_view";

    /// The channels `ctx`'s screens have offered and slow motion, the
    /// first focused.
    pub fn new<G: GameData>(ctx: &Ctx<G>) -> Self {
        Self {
            menu: Self::menu_of(&ctx.debug_view),
        }
    }

    /// The menu of the channels offered now, each on or off, then slow
    /// motion at its speed.
    fn menu_of(view: &DebugView) -> Menu<ViewRow> {
        let mut items: Vec<MenuItem<ViewRow>> = view
            .offered()
            .iter()
            .map(|channel| {
                // check-text: not player text (debug tools are English only)
                let state = if view.is_on(channel.id()) {
                    "  on"
                } else {
                    "  off"
                };
                MenuItem::new(channel.label())
                    .with_suffix(state, SuffixKind::Aside)
                    .with_value(ViewRow::Channel(channel.id()))
            })
            .collect();
        let speed = format!("  {}", speed_name(view.slow()));
        // check-text: not player text (debug tools are English only)
        let slow = MenuItem::new("Slow motion")
            .with_suffix(speed, SuffixKind::Aside)
            .with_value(ViewRow::SlowMotion);
        items.push(slow);
        Menu::new(items)
    }

    /// The menu: its items, each standing for its row, and focus.
    pub fn menu(&self) -> &Menu<ViewRow> {
        &self.menu
    }

    /// What the screen shows, naming the keys of the device used last.
    pub fn view<G: GameData>(&self, ctx: &Ctx<G>) -> DebugViewView {
        let km = ctx.help_keys();
        let key = |ui| G::Action::for_ui(ui).map(|a| key_name(km, a));
        // check-text: not player text (debug tools are English only)
        let help = help_line(&[
            (Some(cursor_keys_name(km)), "move"),
            (key(UiAction::Confirm), "change"),
            (key(UiAction::Cancel), "back"),
        ]);
        let empty = ctx
            .debug_view
            .offered()
            .is_empty()
            // check-text: not player text (debug tools are English only)
            .then(|| "No screen offers a channel yet".to_owned());
        // The pause's keys are keyboard only, so named on a controller too,
        // and only when bound.
        let keyboard = HelpKeys::keyboard(&ctx.keymap);
        let pace_key = |ui| {
            G::Action::for_ui(ui)
                .filter(|&a| ctx.keymap.primary(a).is_some())
                .map(|a| key_name(keyboard, a))
        };
        // check-text: not player text (debug tools are English only)
        let (mut status, keys) = match ctx.debug_view.pause_line() {
            Some(paused) => (
                format!("Game {paused}"),
                vec![
                    (pace_key(UiAction::DebugStep), "step"),
                    (pace_key(UiAction::DebugPause), "run"),
                ],
            ),
            None => (
                "Game running".to_owned(),
                vec![(pace_key(UiAction::DebugPause), "pause")],
            ),
        };
        for (key, what) in keys.into_iter().filter_map(|(k, w)| Some((k?, w))) {
            let _ = write!(status, " · {key} {what}");
        }
        DebugViewView {
            // check-text: not player text (debug tools are English only)
            title: "Debug view".to_owned(),
            status,
            menu: self.menu.view(),
            empty,
            help,
        }
    }
}

impl<G: GameData> Screen<G> for DebugViewScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    /// Its menu's.
    fn actions(&self) -> Option<Vec<G::Action>> {
        Some(crate::input::ui_actions(Menu::<()>::ACTIONS))
    }

    fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        for &action in &input.actions {
            match self.menu.handle_with_sound(action, &mut ctx.audio) {
                Some(MenuEvent::Cancelled) => return Transition::Pop,
                Some(MenuEvent::Chosen(ViewRow::Channel(id))) => {
                    let on = !ctx.debug_view.is_on(id);
                    if let Err(e) = ctx.set_debug_channel(id, on) {
                        let (level, category) = (LogLevel::Warn, LogCategory::Storage);
                        // check-text: not player text (a warning for the log)
                        ctx.log(level, category, format!("debug view not saved: {e}"));
                    }
                }
                Some(MenuEvent::Chosen(ViewRow::SlowMotion)) => {
                    let next = next_slow(ctx.debug_view.slow());
                    ctx.debug_view.set_slow(next);
                }
                None => continue,
            }
            let focus = self.menu.focus();
            self.menu = Self::menu_of(&ctx.debug_view).focused(focus);
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        glyph::paint_view(ctx, &self.view(ctx), buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slow_motion_goes_through_its_four_speeds_and_names_each() {
        assert_eq!([1, 2, 4, 8].map(next_slow), [2, 4, 8, 1]);
        // A speed set from elsewhere goes on to the next one up.
        assert_eq!([3, 5, 7].map(next_slow), [4, 8, 8]);
        let names = [1, 2, 4, 8, 3].map(speed_name);
        assert_eq!(names, ["1x", "1/2x", "1/4x", "1/8x", "1/3x"]);
    }
}
