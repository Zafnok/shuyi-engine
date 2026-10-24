//! The developer's debug screens: a menu of the game's tools
//! ([`DebugMenuScreen`], with the game's and its modules'
//! [`DebugEntry`]s), the glyph
//! sampler that shows every glyph of the font and every color of the
//! palette ([`GlyphSamplerScreen`]), the scene camera over a picture of
//! the game's ([`SceneCameraScreen`]), and the hint that names the Debug
//! key ([`hint`]). Debug tools are in English only.
//!
//! **The look is a skin**: the menu says what it shows as a
//! [`DebugMenuView`] and [`glyph::paint`] draws it; the scene camera's
//! [`SceneCameraView`](camera::SceneCameraView), [`glyph::paint_camera`].

pub mod camera;
pub mod glyph;

pub use camera::{CameraLayout, SceneCameraScreen};

use crate::prelude::*;
use core::fmt;

use shuyi_assets::keymap::{ActionSet, UiAction};

use crate::audio::MenuSound;
use crate::ctx::{Ctx, GameData};
use crate::frame::color::Palette;
use crate::frame::console::ConsoleSpec;
use crate::frame::glyph_buffer::GlyphBuffer;
use crate::input::help_keys::{HelpKeys, cursor_keys_name, help_line, key_name};
use crate::screen::{FrameInput, Screen, Transition};
use crate::widgets::{Menu, MenuEvent, MenuItem, MenuView};

/// The debug-menu hint, e.g. `F2 debug`: `None` unless debug tools are on
/// and the game's Debug action has a key (it isn't rebindable, so no
/// `! not mapped` hint for it).
pub fn hint<G: GameData>(ctx: &Ctx<G>) -> Option<String> {
    let debug = G::Action::for_ui(UiAction::Debug)?;
    let bound = ctx.keymap.primary(debug).is_some();
    // The debug key is keyboard-only, so it is named on a controller too.
    let keys = HelpKeys::keyboard(&ctx.keymap);
    // check-text: not player text (debug tools are English only)
    (ctx.debug_tools && bound).then(|| format!("{} debug", key_name(keys, debug)))
}

/// What choosing a debug tool does, given the context. Its own state (how
/// many times it ran, say) lives in the closure.
pub type Run<G> = Box<dyn FnMut(&mut Ctx<G>) -> Transition<G>>;

/// What a debug tool is called on the menu, read from the context.
pub type Label<G> = Box<dyn Fn(&Ctx<G>) -> String>;

/// One tool on the debug menu: what the menu calls it (which may name
/// something in `ctx`, such as a look in use) and what choosing it does.
pub struct DebugEntry<G: GameData> {
    /// Its name on the menu.
    label: Label<G>,
    /// What choosing it does.
    run: Run<G>,
}

impl<G: GameData> DebugEntry<G> {
    /// A tool called `label`.
    pub fn new(
        label: &'static str,
        run: impl FnMut(&mut Ctx<G>) -> Transition<G> + 'static,
    ) -> Self {
        Self::named(move |_| label.to_owned(), run)
    }

    /// A tool whose name `label` reads from the context each time the menu
    /// is drawn after it runs.
    pub fn named(
        label: impl Fn(&Ctx<G>) -> String + 'static,
        run: impl FnMut(&mut Ctx<G>) -> Transition<G> + 'static,
    ) -> Self {
        Self {
            label: Box::new(label),
            run: Box::new(run),
        }
    }

    /// Its name on the menu now.
    pub fn label(&self, ctx: &Ctx<G>) -> String {
        (self.label)(ctx)
    }

    /// Runs it.
    pub fn run(&mut self, ctx: &mut Ctx<G>) -> Transition<G> {
        (self.run)(ctx)
    }
}

impl<G: GameData> fmt::Debug for DebugEntry<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DebugEntry").finish_non_exhaustive()
    }
}

/// What the debug menu shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugMenuView {
    /// Its title.
    pub title: String,
    /// The tools.
    pub menu: MenuView,
    /// The help line.
    pub help: String,
}

/// The debug menu the Debug key opens: the game's tools, in its order,
/// then each installed module's ([`Install::debug_entry`](crate::module::Install::debug_entry)),
/// in install order; Cancel closes it. A tool that opens nothing leaves the menu on it, its
/// names read again.
pub struct DebugMenuScreen<G: GameData> {
    /// Each item stands for its entry's index in `entries`.
    menu: Menu<usize>,
    entries: Vec<DebugEntry<G>>,
}

impl<G: GameData> fmt::Debug for DebugMenuScreen<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DebugMenuScreen")
            .field("menu", &self.menu)
            .field("entries", &self.entries.len())
            .finish()
    }
}

impl<G: GameData> DebugMenuScreen<G> {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "debug_menu";

    /// The menu of the game's `entries`, then the entries of the modules
    /// installed on `ctx`, made now, its first tool focused.
    pub fn new(ctx: &Ctx<G>, mut entries: Vec<DebugEntry<G>>) -> Self {
        entries.extend(ctx.modules.debug_entries());
        let menu = Self::menu_of(ctx, &entries);
        Self { menu, entries }
    }

    /// The menu of `entries`' names now.
    fn menu_of(ctx: &Ctx<G>, entries: &[DebugEntry<G>]) -> Menu<usize> {
        Menu::new(
            entries
                .iter()
                .enumerate()
                .map(|(i, e)| MenuItem::new(e.label(ctx)).with_value(i))
                .collect(),
        )
    }

    /// The menu: its items, each standing for its tool's index, and focus.
    pub fn menu(&self) -> &Menu<usize> {
        &self.menu
    }

    /// The same menu focused on tool `index`.
    #[must_use]
    pub fn focused(mut self, index: usize) -> Self {
        self.menu = self.menu.focused(index);
        self
    }

    /// What the menu shows: its title, the tools and the help line, naming
    /// the keys of the device used last.
    pub fn view(&self, ctx: &Ctx<G>) -> DebugMenuView {
        let km = ctx.help_keys();
        let key = |ui| G::Action::for_ui(ui).map(|a| key_name(km, a));
        // check-text: not player text (debug tools are English only)
        let help = help_line(&[
            (Some(cursor_keys_name(km)), "move"),
            (key(UiAction::Confirm), "choose"),
            (key(UiAction::Cancel), "back"),
        ]);
        DebugMenuView {
            // check-text: not player text (debug tools are English only)
            title: "Debug tools".to_owned(),
            menu: self.menu.view(),
            help,
        }
    }
}

impl<G: GameData> Screen<G> for DebugMenuScreen<G> {
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
                Some(MenuEvent::Chosen(i)) => {
                    let Some(entry) = self.entries.get_mut(i) else {
                        continue;
                    };
                    let transition = entry.run(ctx);
                    if !matches!(transition, Transition::None) {
                        return transition;
                    }
                    // Its name may have changed (the look in use, say).
                    self.menu = Self::menu_of(ctx, &self.entries).focused(i);
                }
                None => {}
            }
        }
        Transition::None
    }

    fn draw(&self, ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        glyph::paint(ctx, &self.view(ctx), buf);
    }
}

/// What the glyph sampler adds under its palette: the game's sample
/// panels from row `top` down, in its colors.
pub type SamplePanels = fn(&mut GlyphBuffer, &Palette, i32);

/// The [`glyph_sampler`](glyph::sampler) as a screen; Cancel closes it.
#[derive(Debug, Clone)]
pub struct GlyphSamplerScreen {
    /// Drawn once on creation; the sampler never changes.
    sampler: GlyphBuffer,
}

impl GlyphSamplerScreen {
    /// Name reported by [`Screen::name`].
    pub const NAME: &'static str = "glyph_sampler";

    /// The sampler of `glyphs` (the font's), `ctx`'s palette and the game's
    /// `panels`, on a console of `spec`.
    pub fn new<G: GameData>(
        ctx: &Ctx<G>,
        spec: ConsoleSpec,
        glyphs: &[char],
        panels: SamplePanels,
    ) -> Self {
        Self {
            sampler: glyph::sampler(spec, &ctx.palette, glyphs, panels),
        }
    }
}

impl<G: GameData> Screen<G> for GlyphSamplerScreen {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    /// Cancel closes it.
    fn actions(&self) -> Option<Vec<G::Action>> {
        Some(crate::input::ui_actions(&[UiAction::Cancel]))
    }

    fn update(&mut self, ctx: &mut Ctx<G>, input: &FrameInput<G::Action>) -> Transition<G> {
        if input
            .actions
            .iter()
            .any(|a| a.ui() == Some(UiAction::Cancel))
        {
            ctx.audio.menu(MenuSound::Cancel);
            Transition::Pop
        } else {
            Transition::None
        }
    }

    fn draw(&self, _ctx: &Ctx<G>, buf: &mut GlyphBuffer) {
        buf.blit(&self.sampler, 0, 0);
    }
}

#[cfg(test)]
mod tests;
