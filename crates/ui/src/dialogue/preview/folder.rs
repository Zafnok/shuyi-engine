//! The files a dialogue preview reads: the `.dlg` files of a folder, kept
//! on the context ([`DialogueFiles`]) by a module ([`DialogueFolder`]) that
//! reads them again when one changes on disk.

use crate::prelude::*;

use shuyi_assets::bundle::{Bundle, Layer};

use crate::ctx::{Ctx, GameData};
use crate::module::{Hook, Install, Module};

/// The extension of the files read.
const DLG: &str = ".dlg";

/// The `.dlg` files of the folder, as last read: on the context, where the
/// preview finds them ([`Ctx::module_state`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogueFiles {
    /// Each file's path in the folder and bytes, by path.
    files: Vec<(&'static str, &'static [u8])>,
    /// How many times they were read again.
    version: u64,
}

impl DialogueFiles {
    /// The `.dlg` files of `bundle`, read for the `version`-th time.
    fn read(bundle: &Bundle, version: u64) -> Self {
        let paths = bundle.files_under("");
        let files = paths
            .into_iter()
            .filter(|path| path.ends_with(DLG))
            .filter_map(|path| Some((path, bundle.bytes(path)?)))
            .collect();
        Self { files, version }
    }

    /// Each file's path in the folder and bytes, by path.
    pub fn files(&self) -> impl Iterator<Item = (&'static str, &'static [u8])> + '_ {
        self.files.iter().copied()
    }

    /// How many times the files were read again: it moves on each time.
    pub fn version(&self) -> u64 {
        self.version
    }
}

/// The module that keeps the [`DialogueFiles`] of a folder on the context,
/// reading them again when a `.dlg` file of it changes
/// ([`Module::reloaded`]). A preview program reads its folder with
/// `shuyi_app::read_watched_folder`, so the files are watched in a build
/// without debug tools (ADR-0095), and installs this module over the layer
/// it gives.
#[derive(Debug, Clone)]
pub struct DialogueFolder {
    /// The folder.
    bundle: Bundle,
}

impl DialogueFolder {
    /// The module over the folder read as `layer`.
    pub fn new(layer: Layer) -> Self {
        Self {
            bundle: Bundle::new(vec![layer]),
        }
    }
}

impl<G: GameData> Module<G> for DialogueFolder {
    fn name(&self) -> &'static str {
        "dialogue_folder"
    }

    fn install(&mut self, install: &mut Install<'_, G>) {
        install.hook(Hook::Reloaded);
        install.state(DialogueFiles::read(&self.bundle, 0));
    }

    fn reloaded(&mut self, ctx: &mut Ctx<G>, changed: &[String]) {
        if !changed.iter().any(|path| path.ends_with(DLG)) {
            return;
        }
        if let Some(files) = ctx.module_state_mut::<DialogueFiles>() {
            *files = DialogueFiles::read(&self.bundle, files.version + 1);
        }
    }
}
