//! [`Module`]: a piece from outside the default engine that joins a
//! running [`Game`](crate::Game) (ADR-0040).
//!
//! A module is a library first: the game's screens call its types. It
//! installs a `Module` only for what must happen on every frame whatever
//! screen is on top, or be reached by screens it didn't make. Everything
//! it does is declared in [`Module::install`]: the [`Hook`]s the game
//! calls, its state on the [`Ctx`] (found by type,
//! [`Ctx::module_state`]) and its entries on the debug menu
//! ([`Install::debug_entry`]). The game lists each module with what it
//! declared ([`Game::modules`](crate::Game::modules)).
//!
//! A hook changes the module's own state and the context's UI side (a
//! sound), never the game's data or a stage's state, which change only
//! through commands or inputs per step. A module takes no actions: one
//! that needs input has a screen, which declares them.

use crate::prelude::*;
use core::any::{Any, TypeId, type_name};
use core::fmt;

use crate::ctx::{Ctx, GameData};
use crate::debug::DebugEntry;
use crate::frame::glyph_buffer::GlyphBuffer;

/// A piece that joins a running game: the game installs it with
/// [`Game::with_module`](crate::Game::with_module), and calls only the
/// hooks its [`install`](Self::install) declares, in install order.
pub trait Module<G: GameData>: 'static {
    /// Its name: `snake_case` ASCII, unique in a game.
    fn name(&self) -> &'static str;

    /// Declares what it does: its hooks and its state on the context. By
    /// default nothing.
    fn install(&mut self, _install: &mut Install<'_, G>) {}

    /// Runs each frame `dt` seconds long, after the input and before the
    /// top screen's update ([`Hook::BeforeUpdate`]).
    fn before_update(&mut self, _ctx: &mut Ctx<G>, _dt: f32) {}

    /// Draws over the screens, after them ([`Hook::DrawOver`]).
    fn draw_over(&self, _ctx: &Ctx<G>, _buf: &mut GlyphBuffer) {}
}

/// A place in the frame loop where the game calls a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Hook {
    /// [`Module::before_update`].
    BeforeUpdate,
    /// [`Module::draw_over`].
    DrawOver,
}

impl Hook {
    /// Its name: the method's.
    pub const fn name(self) -> &'static str {
        match self {
            Self::BeforeUpdate => "before_update",
            Self::DrawOver => "draw_over",
        }
    }
}

/// What a module declared, as [`Game::modules`](crate::Game::modules)
/// lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct InstalledModule {
    /// Its name.
    pub name: &'static str,
    /// The hooks the game calls.
    pub hooks: Vec<Hook>,
    /// The types it put on the context, by [`type_name`].
    pub states: Vec<&'static str>,
    /// Its entries' names on the debug menu, as they read at install, in
    /// the order declared.
    pub debug_entries: Vec<String>,
}

impl InstalledModule {
    /// A module called `name` that declared nothing.
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            hooks: Vec::new(),
            states: Vec::new(),
            debug_entries: Vec::new(),
        }
    }
}

/// Why a module couldn't be installed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ModuleError {
    /// A module of this name is already installed.
    #[error("the module {name} is installed twice")]
    SameName {
        /// The name.
        name: &'static str,
    },
    /// Two modules put a value of the same type on the context.
    #[error("the modules {other} and {module} both put a {state} on the context")]
    SameState {
        /// The type, by [`type_name`].
        state: &'static str,
        /// The module installed second.
        module: &'static str,
        /// The module that put it there first.
        other: &'static str,
    },
}

/// What [`Module::install`] declares with.
pub struct Install<'a, G: GameData> {
    ctx: &'a mut Ctx<G>,
    installed: InstalledModule,
    /// The first mistake, reported when the install ends.
    error: Option<ModuleError>,
}

impl<G: GameData> fmt::Debug for Install<'_, G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Install")
            .field("installed", &self.installed)
            .finish_non_exhaustive()
    }
}

impl<'a, G: GameData> Install<'a, G> {
    /// The install of the module `name` on `ctx`.
    pub(crate) fn new(ctx: &'a mut Ctx<G>, name: &'static str) -> Self {
        Self {
            ctx,
            installed: InstalledModule::new(name),
            error: None,
        }
    }

    /// Declares `hook`: the game calls it.
    pub fn hook(&mut self, hook: Hook) {
        if !self.installed.hooks.contains(&hook) {
            self.installed.hooks.push(hook);
        }
    }

    /// Puts `value` on the context, where screens and the module's hooks
    /// find it by its type ([`Ctx::module_state`]). One value per type in
    /// a game.
    pub fn state<T: Any>(&mut self, value: T) {
        let module = self.installed.name;
        match self.ctx.modules.put(module, value) {
            Ok(()) => self.installed.states.push(type_name::<T>()),
            Err(other) => {
                let state = type_name::<T>();
                self.error.get_or_insert(ModuleError::SameState {
                    state,
                    module,
                    other,
                });
            }
        }
    }

    /// Adds an entry to the debug menu, after the game's and the entries
    /// of the modules installed before it: `make` makes it each time the
    /// menu opens ([`DebugMenuScreen::new`](crate::debug::DebugMenuScreen::new)).
    pub fn debug_entry(&mut self, make: fn() -> DebugEntry<G>) {
        self.installed.debug_entries.push(make().label(self.ctx));
        self.ctx.modules.makers.push(make);
    }

    /// What it declared, or its first mistake.
    pub(crate) fn finish(self) -> Result<InstalledModule, ModuleError> {
        match self.error {
            Some(error) => Err(error),
            None => Ok(self.installed),
        }
    }
}

/// Whether `name` is a module's name: `snake_case` ASCII.
pub(crate) fn is_snake_case(name: &str) -> bool {
    name.starts_with(|c: char| c.is_ascii_lowercase())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

/// One module's state on the context.
struct State {
    id: TypeId,
    type_name: &'static str,
    /// The module that put it there.
    module: &'static str,
    value: Box<dyn Any>,
}

/// The modules' part of the [`Ctx`]: their states, one per type, and
/// the makers of their debug menu entries, in install order.
pub(crate) struct States<G: GameData> {
    states: Vec<State>,
    makers: Vec<fn() -> DebugEntry<G>>,
}

impl<G: GameData> Default for States<G> {
    fn default() -> Self {
        Self {
            states: Vec::new(),
            makers: Vec::new(),
        }
    }
}

impl<G: GameData> fmt::Debug for States<G> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list()
            .entries(self.states.iter().map(|s| s.type_name))
            .finish()
    }
}

impl<G: GameData> States<G> {
    /// Puts `module`'s `value` here, or names the module whose value of
    /// its type is here already.
    fn put<T: Any>(&mut self, module: &'static str, value: T) -> Result<(), &'static str> {
        if let Some(other) = self.find(TypeId::of::<T>()) {
            return Err(other.module);
        }
        self.states.push(State {
            id: TypeId::of::<T>(),
            type_name: type_name::<T>(),
            module,
            value: Box::new(value),
        });
        Ok(())
    }

    fn find(&self, id: TypeId) -> Option<&State> {
        self.states.iter().find(|s| s.id == id)
    }

    /// The value of type `T`, if a module put one here.
    pub(crate) fn get<T: Any>(&self) -> Option<&T> {
        self.find(TypeId::of::<T>())?.value.downcast_ref()
    }

    /// [`get`](Self::get), mutable.
    pub(crate) fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        let state = self.states.iter_mut().find(|s| s.id == TypeId::of::<T>())?;
        state.value.downcast_mut()
    }

    /// The modules' debug menu entries, made now, in install order.
    pub(crate) fn debug_entries(&self) -> impl Iterator<Item = DebugEntry<G>> + '_ {
        self.makers.iter().map(|make| make())
    }
}

#[cfg(test)]
mod tests;
