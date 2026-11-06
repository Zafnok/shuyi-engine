//! The "Debug view" screen of the debug menu (ADR-0085): every channel the
//! running screens have offered, each on or off, toggled with Confirm.
//! The choice is saved ([`Ctx::set_debug_channel`]); the game shows the
//! channels on over its screens once the menu is closed.

use core::fmt;

use shuyi_assets::keymap::{ActionSet, UiAction};

use super::glyph;
use crate::ctx::{Ctx, GameData};
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::help_keys::{cursor_keys_name, help_line, key_name};
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
    /// The channels, each with on or off after its name.
    pub menu: MenuView,
    /// What it says when no screen has offered a channel yet; `None` when
    /// some have.
    pub empty: Option<String>,
    /// The help line.
    pub help: String,
}

/// The channels offered, on or off: Confirm turns the focused one on or
/// off, Cancel closes the screen.
pub struct DebugViewScreen {
    /// Each item stands for its channel's id.
    menu: Menu<&'static str>,
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

    /// The channels `ctx`'s screens have offered, the first focused.
    pub fn new<G: GameData>(ctx: &Ctx<G>) -> Self {
        Self {
            menu: Self::menu_of(ctx),
        }
    }

    /// The menu of the channels offered now, each on or off.
    fn menu_of<G: GameData>(ctx: &Ctx<G>) -> Menu<&'static str> {
        let view = &ctx.debug_view;
        let items = view
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
                    .with_value(channel.id())
            })
            .collect();
        Menu::new(items)
    }

    /// The menu: its items, each standing for its channel's id, and focus.
    pub fn menu(&self) -> &Menu<&'static str> {
        &self.menu
    }

    /// What the screen shows, naming the keys of the device used last.
    pub fn view<G: GameData>(&self, ctx: &Ctx<G>) -> DebugViewView {
        let km = ctx.help_keys();
        let key = |ui| G::Action::for_ui(ui).map(|a| key_name(km, a));
        // check-text: not player text (debug tools are English only)
        let help = help_line(&[
            (Some(cursor_keys_name(km)), "move"),
            (key(UiAction::Confirm), "on/off"),
            (key(UiAction::Cancel), "back"),
        ]);
        let empty = self
            .menu
            .items()
            .is_empty()
            // check-text: not player text (debug tools are English only)
            .then(|| "No screen offers a channel yet".to_owned());
        DebugViewView {
            // check-text: not player text (debug tools are English only)
            title: "Debug view".to_owned(),
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
                Some(MenuEvent::Chosen(id)) => {
                    let on = !ctx.debug_view.is_on(id);
                    if let Err(e) = ctx.set_debug_channel(id, on) {
                        let (level, category) = (LogLevel::Warn, LogCategory::Storage);
                        // check-text: not player text (a warning for the log)
                        ctx.log(level, category, format!("debug view not saved: {e}"));
                    }
                    let focus = self.menu.focus();
                    self.menu = Self::menu_of(ctx).focused(focus);
                }
                None => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        glyph::paint_view(ctx, &self.view(ctx), buf);
    }
}
