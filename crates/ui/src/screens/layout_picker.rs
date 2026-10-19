//! The first-launch "Pick your layout" screen: one panel per [`Layout`]
//! the keymap file lists, with a small keyboard diagram and a legend, both
//! read from that layout's bindings. A game's options open it again to
//! switch layout ([`LayoutPickerScreen::change`]). What the legend lists
//! is the game's ([`Legend`]).
//!
//! **The look is a skin.** This module decides what the screen shows and
//! does, and says it as plain data: a [`LayoutPickerView`]
//! ([`LayoutPickerScreen::view`]), with each layout's legend and what each
//! key of its keyboard does. [`glyph::paint`] draws that view as glyphs and
//! decides where the keys sit. Tests of what happened read the view; only
//! [`glyph`]'s tests read cells.

pub mod glyph;
pub mod view;

use crate::prelude::*;
use core::fmt;
pub use view::{KeyCapView, KeyRole, LayoutPickerView, LayoutView, LegendRow};

use shuyi_assets::keymap::{ActionSet, Chord, Key, Layout, UiAction};

use crate::ctx::{Ctx, GameData};
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::help_keys::{all_key_names, cursor_keys_name, key_name};
use crate::input::keymap::Keymap;
use crate::screen::{FrameInput, Screen, Transition};
use crate::widgets::{Menu, MenuEvent, MenuItem};

/// The key of the title in the language files ([`Ctx::text`]).
pub const TITLE: &str = "layout_picker.title";
/// The text key of the help line the first time, before any layout is
/// chosen, with `{up}`, `{down}` and `{confirm}`: every key of each.
pub const HELP_FIRST: &str = "layout_picker.help_first";
/// The text key of the help line when the picker is opened to switch
/// layout (it names the Cancel key too).
pub const HELP_CHANGE: &str = "layout_picker.help_change";

/// Which keys a line of the legend names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegendKeys<A> {
    /// The cursor keys: the line that is flagged as movement.
    Cursor,
    /// Every key of the action.
    All(A),
    /// The action's first key.
    First(A),
}

/// The game's legend, top to bottom: which keys each line names, and the
/// text key of what they do.
pub type Legend<A> = [(LegendKeys<A>, &'static str)];

/// The name shown for a layout, in `ctx`'s language: the text key the
/// keymap file gives it (empty for a layout the keymap doesn't have, which
/// is never in use).
pub fn label<G: GameData>(ctx: &Ctx<G>, layout: Layout) -> &str {
    ctx.game
        .keymap_def()
        .layout_text(layout)
        .map_or("", |key| ctx.text(key))
}

/// The layouts the picker shows, in the keymap file's order.
fn layouts<G: GameData>(ctx: &Ctx<G>) -> Vec<Layout> {
    ctx.game.keymap_def().layout_ids().collect()
}

/// Shows every layout and saves the one picked. Asked for by a key press
/// while no layout is chosen, it can't be cancelled with a key (the
/// keyboard needs a layout), but any controller button closes it without
/// choosing: that player is on the controller. Pops itself once a layout
/// is picked.
pub struct LayoutPickerScreen<G: GameData> {
    menu: Menu,
    /// Whether Cancel backs out without changing anything (opened from
    /// the options).
    cancellable: bool,
    /// What the legend lists.
    legend: &'static Legend<G::Action>,
}

impl<G: GameData> fmt::Debug for LayoutPickerScreen<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LayoutPickerScreen")
            .field("menu", &self.menu)
            .field("cancellable", &self.cancellable)
            .field("legend", &self.legend)
            .finish()
    }
}

impl<G: GameData> Clone for LayoutPickerScreen<G> {
    fn clone(&self) -> Self {
        Self {
            menu: self.menu.clone(),
            cancellable: self.cancellable,
            legend: self.legend,
        }
    }
}

impl<G: GameData> LayoutPickerScreen<G> {
    /// Its [`Screen::name`].
    pub const NAME: &'static str = "layout_picker";

    /// The picker with the first layout focused, its legend listing
    /// `legend`.
    pub fn new(ctx: &Ctx<G>, legend: &'static Legend<G::Action>) -> Self {
        Self {
            // A layout must be picked.
            menu: Self::menu(ctx).without_cancel(),
            cancellable: false,
            legend,
        }
    }

    /// The picker as the options open it, to switch layout: the layout in
    /// use focused, and Cancel backs out without changing anything.
    pub fn change(ctx: &Ctx<G>, legend: &'static Legend<G::Action>) -> Self {
        let current = layouts(ctx).iter().position(|&l| Some(l) == ctx.layout());
        Self {
            menu: Self::menu(ctx).focused(current.unwrap_or(0)),
            cancellable: true,
            legend,
        }
    }

    /// One line per layout. The menu only keeps the focus: it is never
    /// drawn, so its lines have no text.
    fn menu(ctx: &Ctx<G>) -> Menu {
        let items = layouts(ctx)
            .into_iter()
            .map(|_| MenuItem::new(String::new()));
        Menu::new(items.collect())
    }

    /// The focused layout (`None` only for a keymap with no layouts, which
    /// the loader refuses).
    pub fn focused(&self, ctx: &Ctx<G>) -> Option<Layout> {
        layouts(ctx).get(self.menu.focus()).copied()
    }

    /// The bottom help line: every Cursor up and Cursor down key, then every
    /// Confirm key, from the active keymap (before any layout is chosen,
    /// the layout picker's own keys).
    pub fn help(ctx: &Ctx<G>) -> String {
        Self::help_with(ctx, false)
    }

    /// [`help`](Self::help); when the picker can be backed out of
    /// (`cancellable`, from Options), the line that also names the Cancel
    /// key.
    fn help_with(ctx: &Ctx<G>, cancellable: bool) -> String {
        if cancellable {
            return ctx.text_with(HELP_CHANGE, &[]);
        }
        let km = ctx.help_keys();
        let names = |ui| G::Action::for_ui(ui).map_or_else(String::new, |a| all_key_names(km, a));
        let up = names(UiAction::Up);
        let down = names(UiAction::Down);
        let confirm = names(UiAction::Confirm);
        ctx.text_with(
            HELP_FIRST,
            &[("up", &up), ("down", &down), ("confirm", &confirm)],
        )
    }

    /// The legend for `km`, in `ctx`'s language: the keys and what they
    /// do; an action with no key shows `! not mapped`. Always its keyboard
    /// keys: the screen is about the keyboard's layouts.
    pub fn legend(&self, ctx: &Ctx<G>, km: &Keymap<G::Action>) -> Vec<LegendRow> {
        let km = ctx.help_keys().of(km).on_keyboard();
        let row = |&(keys, what): &(LegendKeys<G::Action>, &str)| LegendRow {
            keys: match keys {
                LegendKeys::Cursor => cursor_keys_name(km),
                LegendKeys::All(action) => all_key_names(km, action),
                LegendKeys::First(action) => key_name(km, action),
            },
            what: ctx.text(what).to_owned(),
            movement: keys == LegendKeys::Cursor,
        };
        self.legend.iter().map(row).collect()
    }

    /// The screen as it is now, as plain data for a skin to paint
    /// ([`glyph::paint`]): each layout with its legend and what every key
    /// of its keyboard does, the one in focus, and the help line.
    pub fn view(&self, ctx: &Ctx<G>) -> LayoutPickerView {
        let focused = self.focused(ctx);
        let layouts = layouts(ctx).into_iter().map(|layout| {
            let km = ctx.keymap_for(layout);
            LayoutView {
                layout,
                name: label(ctx, layout).to_owned(),
                focused: Some(layout) == focused,
                legend: self.legend(ctx, &km),
                keys: key_caps(&km),
            }
        });
        LayoutPickerView {
            title: ctx.text(TITLE).to_owned(),
            layouts: layouts.collect(),
            help: Self::help_with(ctx, self.cancellable),
        }
    }
}

/// What `key` does in `km`: moves the cursor (a key that repeats), does
/// something else, or nothing.
// check-keys: keyboard picture
fn key_role<A: ActionSet>(km: &Keymap<A>, key: Key) -> KeyRole {
    match km.action(Chord::plain(key)) {
        Some(action) if action.is_repeatable() => KeyRole::Movement,
        Some(_) => KeyRole::Bound,
        None => KeyRole::Unbound,
    }
}

/// Every key of the keyboard with its role in `km`; a skin draws the ones
/// its picture has.
// check-keys: keyboard picture
fn key_caps<A: ActionSet>(km: &Keymap<A>) -> Vec<KeyCapView> {
    Key::ALL
        .iter()
        .map(|&key| KeyCapView {
            key,
            role: key_role(km, key),
        })
        .collect()
}

impl<G: GameData> Screen<G> for LayoutPickerScreen<G> {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        // Bound or not; the button does nothing else.
        if !self.cancellable && input.pad_pressed() {
            return Transition::Pop;
        }
        for &action in &input.actions {
            match self.menu.handle_with_sound(action, &mut ctx.audio) {
                Some(MenuEvent::Chosen(i)) => {
                    if let Some(&layout) = layouts(ctx).get(i) {
                        // If saving fails the layout is still used this
                        // session; the player is just asked again next
                        // launch.
                        let _ = ctx.choose_layout(layout);
                    }
                    return Transition::Pop;
                }
                Some(MenuEvent::Cancelled) => return Transition::Pop,
                None => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        glyph::paint(ctx, &self.view(ctx), buf);
    }

    fn as_any(&self) -> Option<&dyn core::any::Any> {
        Some(self)
    }
}

#[cfg(test)]
mod tests;
