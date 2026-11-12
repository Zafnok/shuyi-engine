//! [`BasicGame`]: a game on the short path (ADR-0036), described once for
//! its binary, its tools and its tests. It holds what the engine can't
//! read from the game's files: who it is ([`AppInfo`]), its files,
//! its first screen and, if it has them, its console, its own part,
//! every screen it can show, the modules it installs and its record of
//! its play for a crash report. From it come
//! its data ([`BasicGame::data`]), its context ([`BasicGame::ctx`]), its
//! [`Game`] and how a backend starts it ([`BasicGame::start`]), and its
//! dialogue preview for a writer ([`BasicGame::preview_start`]).

use crate::alloc_prelude::*;
use alloc::{format, vec};
use core::fmt::{self, Debug};

use shuyi_assets::ContentError;
use shuyi_assets::bundle::{Bundle, Layer};
use shuyi_assets::keymap::ActionSet;
use shuyi_assets::pack::Bake;
use shuyi_core::Made;
use shuyi_ui::basic::{BasicData, Own};
use shuyi_ui::dialogue::preview::{DialogueFolder, DialoguePreview, PreviewGame};
use shuyi_ui::frame::ConsoleSpec;
use shuyi_ui::module::{Module, ModuleError};
use shuyi_ui::{Ctx, Game, GameData, GameHooks, Screen};

use crate::app::{Card, CrashRecord, Start};
use crate::app_info::AppInfo;
use crate::platform::Look;

/// The screen a game on the short path starts on, made on its context.
pub type First<A, T> = fn(&mut Ctx<BasicData<A, T>>) -> Box<dyn Screen<BasicData<A, T>>>;

/// Every screen a game on the short path can show, overlays on their own,
/// for the check that its controls fit each target.
pub type Screens<A, T> = fn(&mut Ctx<BasicData<A, T>>) -> Vec<Box<dyn Screen<BasicData<A, T>>>>;

/// The modules a game on the short path installs, made each time its
/// [`Game`] is: `shuyi::enabled_modules`, for those its features turn on.
pub type Modules<A, T> = fn() -> Vec<Box<dyn Module<BasicData<A, T>>>>;

/// The console of a game that doesn't say: 48 × 24 cells of 8 × 16 pixels.
pub const BASIC_CONSOLE: ConsoleSpec = ConsoleSpec::text(48, 24);

/// A game on the short path: its data is a [`BasicData`] of actions `A`
/// and its own part `T`, read from one layer of files.
pub struct BasicGame<A: ActionSet, T: Debug + 'static = ()> {
    app: AppInfo,
    files: Layer,
    /// Why its files couldn't be read, if they couldn't.
    unreadable: Option<&'static str>,
    /// A layer laid over `files`, if it has one.
    over: Option<Layer>,
    own: Own<T>,
    first: First<A, T>,
    console: ConsoleSpec,
    screens: Option<Screens<A, T>>,
    modules: Option<Modules<A, T>>,
    crash_record: Option<CrashRecord<BasicData<A, T>>>,
    /// Whether a crash report holds its record in a build without debug
    /// tools too.
    record_in_release: bool,
}

/// What a game on the short path is painted with: its font and pictures.
fn look<A: ActionSet, T: Debug + 'static>(data: &BasicData<A, T>) -> Look<'_> {
    Look::new(&data.font, &data.images)
}

impl<A: ActionSet> BasicGame<A> {
    /// The game `app` (its name, and the storage its saves are kept
    /// under), its files in `files`, starting on the screen `first` makes.
    pub fn new(app: AppInfo, files: Layer, first: First<A, ()>) -> Self {
        // No part of its own: there is none to read.
        Self::with_own(app, files, |_| Ok(()), first)
    }
}

impl<A: ActionSet, T: Debug + 'static> BasicGame<A, T> {
    /// As [`BasicGame::new`], for a game with a part of its own, which
    /// `own` reads from its files ([`BasicData::own`]).
    pub fn with_own(app: AppInfo, files: Layer, own: Own<T>, first: First<A, T>) -> Self {
        Self {
            app,
            files,
            unreadable: None,
            over: None,
            own,
            first,
            console: BASIC_CONSOLE,
            screens: None,
            modules: None,
            crash_record: None,
            record_in_release: false,
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

    /// The same game, with the modules `modules` makes installed in its
    /// [`Game`] (none until said).
    #[must_use]
    pub fn modules(mut self, modules: Modules<A, T>) -> Self {
        self.modules = Some(modules);
        self
    }

    /// The same game, with `hook` its record of its play in a crash
    /// report ([`Start::crash_record`]; none until said).
    #[must_use]
    pub fn crash_record(mut self, hook: CrashRecord<BasicData<A, T>>) -> Self {
        self.crash_record = Some(hook);
        self
    }

    /// The same game, with its record of its play in a crash report in a
    /// build without debug tools too, where the panic at a frame named by
    /// `SHUYI_PANIC_AT_FRAME` is honoured as well. Until said, only a build
    /// with debug tools adds the record: together they write the game's
    /// state on demand, which a shipped game gives anyone who runs it.
    #[must_use]
    pub fn crash_record_in_release(mut self) -> Self {
        self.record_in_release = true;
        self
    }

    /// Whether its crash reports hold its record in a build without debug
    /// tools too ([`crash_record_in_release`](Self::crash_record_in_release)).
    pub fn records_play_in_release(&self) -> bool {
        self.record_in_release
    }

    /// Who it is.
    pub fn app(&self) -> AppInfo {
        self.app
    }

    /// Its name: the window's title and the loading card's line.
    pub fn name(&self) -> &'static str {
        self.app.name
    }

    /// The name its saves are kept under.
    pub fn storage_name(&self) -> &'static str {
        self.app.storage
    }

    /// The console it starts on.
    pub fn console_spec(&self) -> ConsoleSpec {
        self.console
    }

    /// The same game, whose files couldn't be read, for `why`: its data is
    /// that one problem, which the platform shows in place of the game. A
    /// packed build whose pack beside the program is missing says so here
    /// (`shuyi_app::pack_beside_program`).
    #[must_use]
    pub fn files_unreadable(mut self, why: &'static str) -> Self {
        self.unreadable = Some(why);
        self
    }

    /// The same game with `layer` laid over its files: a folder on disk
    /// (`shuyi_app::watched_layer`, `cargo shuyi run --watch`), or a mod.
    #[must_use]
    pub fn over(mut self, layer: Layer) -> Self {
        self.over = Some(layer);
        self
    }

    /// Its files, as a bundle of their one layer, under the one laid
    /// [`over`](Self::over) them if it has one.
    pub fn bundle(&self) -> Bundle {
        Bundle::new(self.over.into_iter().chain([self.files]).collect())
    }

    /// Its data from its files, or every problem of every file; or why its
    /// files couldn't be read at all
    /// ([`files_unreadable`](Self::files_unreadable)).
    pub fn data(&self) -> Result<BasicData<A, T>, Vec<ContentError>> {
        if let Some(why) = self.unreadable {
            return Err(vec![ContentError::new(self.files.name, why)]);
        }
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

    /// The game on `ctx`, at its first screen, with its
    /// [`modules`](Self::modules) installed; or why one could not be.
    pub fn game(
        &self,
        mut ctx: Ctx<BasicData<A, T>>,
    ) -> Result<Game<BasicData<A, T>>, ModuleError> {
        let first = (self.first)(&mut ctx);
        let game = Game::new(ctx, first, GameHooks::default());
        match self.modules {
            Some(modules) => game.with_modules(modules()),
            None => Ok(game),
        }
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

    /// What made this build, for its records: its id and version as the
    /// game's word, and the fingerprint of its files.
    pub fn made(&self) -> Made {
        Made::new(self.pack_stamp(), self.bundle().fingerprint())
    }

    /// The stamp of a pack of its files (ADR-0023): its id and version,
    /// since the shapes of its own kinds change only with it. A pack baked
    /// with another stamp is refused, with "bake it again".
    pub fn pack_stamp(&self) -> String {
        self.app.pack_stamp()
    }

    /// What a pack of its files bakes ([`BasicData::bake`]), stamped
    /// [`pack_stamp`](Self::pack_stamp): what `cargo shuyi bake` and a
    /// release export write, and what its data reads back
    /// ([`BasicData::formats`]).
    pub fn bake(&self) -> Bake {
        BasicData::<A, T>::bake(self.pack_stamp())
    }

    /// How a backend starts it: a card with its name, then its context on
    /// the platform and its first screen. A file that doesn't load, or a
    /// module that can't be installed, is the load's error, which the
    /// platform shows in place of the game. A session record of it carries
    /// its [`made`](Self::made), and each frame's hash the state hash of its
    /// screens ([`Screen::state_hash`]). A crash report carries its
    /// [`crash_record`](Self::crash_record), if it has one.
    pub fn start(self) -> Start<BasicData<A, T>> {
        let made = self.made();
        let crash_record = self.crash_record;
        let start = Start::new(Card::text(self.app.name), look::<A, T>, move |setup| {
            let ctx = self.ctx()?;
            self.game(setup.ctx(ctx)).map_err(|e| e.to_string())
        })
        .made(made)
        .state_hash(Game::state_hash);
        match crash_record {
            Some(hook) => start.crash_record(hook),
            None => start,
        }
    }

    /// Its dialogue preview on `ctx` ([`DialoguePreview`]): the scenes of
    /// the `.dlg` files of `folder`, played as `preview` says, read again
    /// when one changes ([`DialogueFolder`] over `folder`). Its
    /// [`modules`](Self::modules) are installed, and no debug screen: it is
    /// a writer's program. Or why a module could not be installed.
    pub fn preview_game<P: PreviewGame<BasicData<A, T>>>(
        &self,
        ctx: Ctx<BasicData<A, T>>,
        folder: Layer,
        preview: P,
    ) -> Result<Game<BasicData<A, T>>, ModuleError> {
        let root = DialoguePreview::new(&ctx, preview);
        let game = Game::new(ctx, Box::new(root), GameHooks::default()).with_debug_screens(false);
        let game = match self.modules {
            Some(modules) => game.with_modules(modules())?,
            None => game,
        };
        game.with_module(DialogueFolder::new(folder))
    }

    /// How a backend starts its dialogue preview
    /// ([`preview_game`](Self::preview_game)) over `folder`, as
    /// [`start`](Self::start) starts the game: its card, then the preview.
    /// A folder that couldn't be read (its `Err`, what went wrong), a file
    /// of the game's that doesn't load, or a module that can't be
    /// installed, is the load's error, which the platform shows in place of
    /// the preview. A `.dlg` file dropped on the window joins the folder
    /// ([`drop_scenes`](shuyi_ui::dialogue::preview::drop_scenes)): on the
    /// web, which has no folders, `folder` is an empty layer the writer
    /// drops their scenes in.
    pub fn preview_start<P: PreviewGame<BasicData<A, T>>>(
        self,
        folder: Result<Layer, String>,
        preview: P,
    ) -> Start<BasicData<A, T>> {
        let made = self.made();
        let start = Start::new(Card::text(self.app.name), look::<A, T>, move |setup| {
            let folder = folder?;
            let ctx = self.ctx()?;
            self.preview_game(setup.ctx(ctx), folder, preview)
                .map_err(|e| e.to_string())
        })
        .made(made)
        .state_hash(Game::state_hash);
        #[cfg(feature = "std")]
        let start = start.on_system(dropped_scenes::<A, T>);
        start
    }

    /// Its dialogue preview ([`preview_game`](Self::preview_game)) over
    /// `folder` in the test Harness.
    ///
    /// # Panics
    ///
    /// As [`harness`](Self::harness).
    #[cfg(any(test, feature = "harness"))]
    pub fn preview_harness<P: PreviewGame<BasicData<A, T>>>(
        &self,
        folder: Layer,
        preview: P,
    ) -> crate::harness::Harness<BasicData<A, T>> {
        let ctx = self
            .ctx()
            .unwrap_or_else(|errors| panic!("the game's files:\n{errors}"));
        let game = self
            .preview_game(ctx, folder, preview)
            .unwrap_or_else(|e| panic!("the game's modules: {e}"));
        let harness = crate::harness::Harness::new(game);
        #[cfg(feature = "std")]
        let harness = harness.with_system_hook(dropped_scenes::<A, T>);
        harness
    }

    /// The game in the test Harness, at its first screen.
    ///
    /// # Panics
    ///
    /// If its files don't load, or a module can't be installed: the panic
    /// gives every problem.
    #[cfg(any(test, feature = "harness"))]
    pub fn harness(&self) -> crate::harness::Harness<BasicData<A, T>> {
        let ctx = self
            .ctx()
            .unwrap_or_else(|errors| panic!("the game's files:\n{errors}"));
        let game = self
            .game(ctx)
            .unwrap_or_else(|e| panic!("the game's modules: {e}"));
        crate::harness::Harness::new(game)
    }
}

/// The dialogue preview's hook for the system's events: the `.dlg` files
/// of a drop join its folder ([`drop_scenes`](shuyi_ui::dialogue::preview::drop_scenes)).
#[cfg(feature = "std")]
fn dropped_scenes<A: ActionSet, T: Debug + 'static>(
    game: &mut Game<BasicData<A, T>>,
    event: crate::SystemEvent,
) {
    if let crate::SystemEvent::FilesDropped(files) = event {
        let files = files
            .into_iter()
            .map(|file| (file.name().to_owned(), file.into_bytes()));
        shuyi_ui::dialogue::preview::drop_scenes(game, files);
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
            .field("app", &self.app)
            .field("files", &self.files.name)
            .field("console", &self.console)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
