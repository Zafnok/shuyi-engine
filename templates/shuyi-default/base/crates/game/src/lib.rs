//! The game: its files in `files/`, embedded, which the engine reads as
//! its data ([`Data`]): the English text, the keys, the palette, the font
//! and its stages' own part. Its private files, a checkout of its private
//! repository in `private/files/` when the build had one, are laid over
//! them ([`PRIVATE`]). A release export builds it with `packed`: then
//! it embeds none of them, and reads the pack the export bakes of them beside
//! the program in their place. It opens on the title ([`title`]), which
//! starts its first stage ([`stages`]) if it has one. [`game`] says all
//! the engine needs to know: `src/main.rs` runs it, and
//! `examples/tools.rs` hands it to the engine's tools. Who the game is,
//! its name, id and version, is `shuyi.ron`'s `app`, which
//! `cargo shuyi app-info` writes into [`app_info`].

pub mod app_info;
pub mod stages;
pub mod title;

use std::sync::OnceLock;

#[cfg(not(feature = "packed"))]
use include_dir::{Dir, include_dir};
use shuyi::core::Made;
use shuyi::prelude::*;

/// The game's files, embedded: `files/`, or an edition's own folder when
/// `cargo shuyi export --edition` builds one with a content root
/// (`build.rs`). A packed build embeds none ([`files`]).
// check-text: not player text, a folder
#[cfg(not(feature = "packed"))]
static FILES: Dir<'static> = include_dir!("$SHUYI_FILES");

/// The game's private files, embedded (the engine's ADR-0111): the
/// checkout's `private/files/` when the build had one, laid over [`FILES`]
/// as a private layer, so a file there replaces the public one at its path;
/// else none, as for an edition with its own content root (`build.rs`). A
/// packed build embeds none: the pack holds them.
// check-text: not player text, a folder
#[cfg(not(feature = "packed"))]
static PRIVATE: Dir<'static> = include_dir!("$SHUYI_PRIVATE_FILES");

/// The game's files as its layer, and why they couldn't be read if they
/// couldn't: embedded.
#[cfg(not(feature = "packed"))]
fn files() -> (Layer, Option<&'static str>) {
    // check-text: not player text, a folder
    (Layer::dir("files", &FILES), None)
}

/// The game's files as its layer, and why they couldn't be read if they
/// couldn't: in a packed build, the pack beside the program
/// (`<package>.pack`), read once.
#[cfg(feature = "packed")]
fn files() -> (Layer, Option<&'static str>) {
    static PACK: OnceLock<Result<Layer, &'static str>> = OnceLock::new();
    let read = PACK.get_or_init(|| {
        let stamp = app_info::APP.pack_stamp();
        // check-text: not player text, the package's name
        let package = env!("CARGO_PKG_NAME");
        shuyi::app::pack_beside_program(package, &stamp)
            // check-text: not player text, a folder
            .map(|pack| pack.into_layer("files"))
            .map_err(|e| &*Box::leak(e.to_string().into_boxed_str()))
    });
    match read {
        Ok(layer) => (*layer, None),
        // check-text: not player text, a folder
        Err(why) => (Layer::table("files", &[]), Some(*why)),
    }
}

/// The game's data and state, which its screens share: the engine's, read
/// from `files/`, with the six moves of `files/keymap.ron` as its actions,
/// and its stages' own part.
pub type Data = BasicData<UiAction, stages::Own>;

/// The game: who it is ([`app_info::APP`]: its name, and where the
/// player's settings and keys are kept), its files, its private files over
/// them, and its stages' part of them, its first screen, every screen it can show, the engine's modules
/// its features turn on (`Cargo.toml`), and its stages' record of its play
/// for a crash report, if they have one.
pub fn game() -> BasicGame<UiAction, stages::Own> {
    let (files, unreadable) = files();
    let game = BasicGame::with_own(app_info::APP, files, stages::own, |_| {
        Box::new(title::Title)
    })
    .screens(screens)
    .modules(shuyi::enabled_modules);
    // A packed build's pack holds its private files already.
    #[cfg(not(feature = "packed"))]
    // check-text: not player text, a folder
    let game = game.private(Layer::dir("private", &PRIVATE));
    let game = match unreadable {
        Some(why) => game.files_unreadable(why),
        None => game,
    };
    match stages::crash_record() {
        Some(hook) => game.crash_record(hook),
        None => game,
    }
}

/// What made this build, for its records and saves: the game's id and
/// version, and the fingerprint of the files it ships, computed once. A
/// record or a save keeps it in a `made` field, checked on reading.
pub fn made() -> Made {
    static MADE: OnceLock<Made> = OnceLock::new();
    MADE.get_or_init(|| {
        // check-text: not player text, the build's name in its records
        let build = format!("{} {}", app_info::APP.id, app_info::APP.version);
        Made::new(build, game().bundle().fingerprint())
    })
    .clone()
}

/// Every screen the game can show: the title and its stages'.
fn screens(ctx: &mut Ctx<Data>) -> Vec<Box<dyn Screen<Data>>> {
    let mut screens: Vec<Box<dyn Screen<Data>>> = vec![Box::new(title::Title)];
    screens.extend(stages::screens(ctx));
    screens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_load() {
        let data = game().data().unwrap_or_else(|e| panic!("{e:?}"));
        assert!(data.images.images.is_empty(), "{:?}", data.images.images);
        assert!(game().ctx().is_ok());
    }

    /// The checkout of the private files, if the game has one.
    #[cfg(not(feature = "packed"))]
    fn checkout() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../private/files")
    }

    /// Every file under `dir`, by its path from it, `/`-separated.
    #[cfg(not(feature = "packed"))]
    fn files_under(dir: &std::path::Path, prefix: &str, out: &mut Vec<String>) {
        let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        for entry in entries {
            let entry = entry.unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = format!("{prefix}{name}");
            if entry.path().is_dir() {
                files_under(&entry.path(), &format!("{path}/"), out);
            } else {
                out.push(path);
            }
        }
    }

    #[test]
    #[cfg(not(feature = "packed"))]
    fn the_private_files_are_the_checkouts() {
        let bundle = game().bundle();
        let checkout = checkout();
        // An edition with its own content root holds none; nor does a build
        // with no checkout.
        if option_env!("SHUYI_CONTENT").is_some() || !checkout.is_dir() {
            assert_eq!(PRIVATE.entries().len(), 0, "{:?}", PRIVATE.entries());
            return;
        }
        let mut paths = Vec::new();
        files_under(&checkout, "", &mut paths);
        for path in paths {
            let on_disk = std::fs::read(checkout.join(&path)).unwrap_or_default();
            assert!(
                bundle.bytes(&path) == Some(&on_disk[..]),
                "private/files/{path} isn't the game's {path}: the build was made before the \
                 checkout, or before the file changed: run `cargo clean -p {}`",
                env!("CARGO_PKG_NAME")
            );
            assert!(bundle.is_private(&bundle.display_path(&path)), "{path}");
        }
    }

    #[test]
    fn the_build_is_stamped_with_its_files() {
        let made = made();
        assert_eq!(made.content, game().bundle().fingerprint());
        assert_eq!(made, Made::new(made.game.clone(), made.content));
        assert!(made.game.starts_with(app_info::APP.id), "{made:?}");
        assert!(made.game.ends_with(app_info::APP.version), "{made:?}");
    }
}
