//! Files the player keeps: what a screen asks the platform for
//! ([`FileRequest`]), queued on the [`Ctx`](crate::Ctx) and carried out by
//! the platform after the frame, as the frame cap is
//! ([`Ctx::offer_file`](crate::Ctx::offer_file),
//! [`Ctx::ask_for_file`](crate::Ctx::ask_for_file),
//! [`Ctx::open_storage_folder`](crate::Ctx::open_storage_folder)).
//!
//! The answers come back as the system's events in a later frame
//! (`shuyi_platform::SystemEvent::FileOffered` and `FileChosen`), which the
//! game's system hook hears. Where the game keeps its data is a fact of the
//! platform ([`PlatformFacts::storage_place`](crate::PlatformFacts::storage_place)),
//! and whether it can open a picker another
//! ([`PlatformFacts::file_picker`](crate::PlatformFacts::file_picker)).

use crate::prelude::*;

/// A request about the player's files a screen made in a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FileRequest {
    /// Hand the player a file called `name` holding `text`: a download on
    /// the web, a file in the exports folder on a desktop.
    #[non_exhaustive]
    Offer {
        /// The file's name, never a path (`farm.ron`).
        name: String,
        /// What it holds.
        text: String,
    },
    /// Ask the player to pick a text file, of one of the extensions in
    /// `accept` (`.ron`), or of any when it is empty.
    #[non_exhaustive]
    Ask {
        /// The extensions offered, each with its dot.
        accept: Vec<String>,
    },
    /// Open one of the game's folders in the system's file manager.
    OpenFolder(Folder),
}

/// A folder of the game's that the player may look in
/// ([`Ctx::open_storage_folder`](crate::Ctx::open_storage_folder)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Folder {
    /// Where the game keeps its saves and settings
    /// ([`PlatformFacts::storage_place`](crate::PlatformFacts::storage_place)).
    Storage,
    /// Where the files the game offered are written: `exports/` in the
    /// storage folder.
    Exports,
}
