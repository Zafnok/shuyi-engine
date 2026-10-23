//! [`BasicGame`]: a game on the short path (ADR-0036), described once for
//! its binary, its tools and its tests. It holds what the engine can't
//! read from the game's files: its name, its storage's name, its files,
//! its first screen and, if it has them, its console, its own part and
//! every screen it can show. From it come its data
//! ([`BasicGame::data`]), its context ([`BasicGame::ctx`]), its
//! [`Game`] and how a backend starts it ([`BasicGame::start`]).

use crate::alloc_prelude::*;
use alloc::{format, vec};
use core::fmt::{self, Debug};

use shuyi_assets::ContentError;
use shuyi_assets::bundle::{Bundle, Layer};
use shuyi_assets::keymap::ActionSet;
use shuyi_ui::basic::{BasicData, Own};
use shuyi_ui::frame::ConsoleSpec;
use shuyi_ui::{Ctx, Game, GameData, GameHooks, Screen};

use crate::app::{Card, Start};
use crate::platform::Look;

/// The screen a game on the short path starts on, made on its context.
pub type First<A, T> = fn(&mut Ctx<BasicData<A, T>>) -> Box<dyn Screen<BasicData<A, T>>>;

/// Every screen a game on the short path can show, overlays on their own,
/// for the check that its controls fit each target.
pub type Screens<A, T> = fn(&mut Ctx<BasicData<A, T>>) -> Vec<Box<dyn Screen<BasicData<A, T>>>>;

/// The console of a game that doesn't say: 48 × 24 cells of 8 × 16 pixels.
pub const BASIC_CONSOLE: ConsoleSpec = ConsoleSpec::text(48, 24);

/// A game on the short path: its data is a [`BasicData`] of actions `A`
/// and its own part `T`, read from one layer of files.
pub struct BasicGame<A: ActionSet, T: Debug + 'static = ()> {
    name: &'static str,
    storage: &'static str,
    files: Layer,
    own: Own<T>,
    first: First<A, T>,
    console: ConsoleSpec,
    screens: Option<Screens<A, T>>,
}

/// What a game on the short path is painted with: its font and pictures.
fn look<A: ActionSet, T: Debug + 'static>(data: &BasicData<A, T>) -> Look<'_> {
    Look {
        font: &data.font,
        images: &data.images,
    }
}

impl<A: ActionSet> BasicGame<A> {
    /// The game called `name`, its saves under `storage` (never change
    /// it, or every player's are lost), its files in `files`, starting on
    /// the screen `first` makes.
    pub fn new(
        name: &'static str,
        storage: &'static str,
        files: Layer,
        first: First<A, ()>,
    ) -> Self {
        // No part of its own: there is none to read.
        Self::with_own(name, storage, files, |_| Ok(()), first)
    }
}

impl<A: ActionSet, T: Debug + 'static> BasicGame<A, T> {
    /// As [`BasicGame::new`], for a game with a part of its own, which
    /// `own` reads from its files ([`BasicData::own`]).
    pub fn with_own(
        name: &'static str,
        storage: &'static str,
        files: Layer,
        own: Own<T>,
        first: First<A, T>,
    ) -> Self {
        Self {
            name,
            storage,
            files,
            own,
            first,
            console: BASIC_CONSOLE,
            screens: None,
        }
    }

    /// The same game on `console` ([`BASIC_CONSOLE`] until said).
    #[must_use]
    pub fn console(mut self, console: ConsoleSpec) -> Self {
        self.console = console;
        self
    }

    /// The same game, with `screens` every screen it can show (its first
    /// alone until said).
    #[must_use]
    pub fn screens(mut self, screens: Screens<A, T>) -> Self {
        self.screens = Some(screens);
        self
    }

    /// Its name: the window's title and the loading card's line.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// The name its saves are kept under.
    pub fn storage_name(&self) -> &'static str {
        self.storage
    }

    /// The console it starts on.
    pub fn console_spec(&self) -> ConsoleSpec {
        self.console
    }

    /// Its files, as a bundle of their one layer.
    pub fn bundle(&self) -> Bundle {
        Bundle::new(vec![self.files])
    }

    /// Its data from its files, or every problem of every file.
    pub fn data(&self) -> Result<BasicData<A, T>, Vec<ContentError>> {
        BasicData::load(self.bundle(), self.console, self.own)
    }

    /// Its context, in its keymap file's first layout; or every problem
    /// of its files, one per line.
    pub fn ctx(&self) -> Result<Ctx<BasicData<A, T>>, String> {
        let data = self.data().map_err(|errors| {
            let lines: Vec<String> = errors.iter().map(ToString::to_string).collect();
            lines.join("\n")
        })?;
        let mut ctx = Ctx::new(data)
            .map_err(|missing| format!("the palette lacks colours: {}", missing.join(", ")))?;
        if let Some(layout) = ctx.game.keymap_def().first_layout() {
            ctx.use_layout(layout);
        }
        Ok(ctx)
    }

    /// The game on `ctx`, at its first screen.
    pub fn game(&self, mut ctx: Ctx<BasicData<A, T>>) -> Game<BasicData<A, T>> {
        let first = (self.first)(&mut ctx);
        Game::new(ctx, first, GameHooks::default())
    }

    /// Every screen it can show ([`screens`](Self::screens)), made on
    /// `ctx`.
    pub fn every_screen(
        &self,
        ctx: &mut Ctx<BasicData<A, T>>,
    ) -> Vec<Box<dyn Screen<BasicData<A, T>>>> {
        match self.screens {
            Some(screens) => screens(ctx),
            None => vec![(self.first)(ctx)],
        }
    }

    /// How a backend starts it: a card with its name, then its context on
    /// the platform and its first screen. A file that doesn't load is
    /// the load's error, which the platform shows in place of the game.
    pub fn start(self) -> Start<BasicData<A, T>> {
        Start::new(Card::text(self.name), look::<A, T>, move |setup| {
            let ctx = self.ctx()?;
            Ok(self.game(setup.ctx(ctx)))
        })
    }

    /// The game in the test Harness, at its first screen.
    ///
    /// # Panics
    ///
    /// If its files don't load: the panic gives every problem.
    #[cfg(any(test, feature = "harness"))]
    pub fn harness(&self) -> crate::harness::Harness<BasicData<A, T>> {
        let ctx = self
            .ctx()
            .unwrap_or_else(|errors| panic!("the game's files:\n{errors}"));
        crate::harness::Harness::new(self.game(ctx))
    }
}

// By hand: a derive would ask `A` and `T` to be `Clone` and `Copy`, and
// every field already is.
impl<A: ActionSet, T: Debug + 'static> Clone for BasicGame<A, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A: ActionSet, T: Debug + 'static> Copy for BasicGame<A, T> {}

impl<A: ActionSet, T: Debug + 'static> Debug for BasicGame<A, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicGame")
            .field("name", &self.name)
            .field("storage", &self.storage)
            .field("files", &self.files.name)
            .field("console", &self.console)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
