//! The player's files on a desktop: offered files written into
//! `<data folder>/exports/`, and folders opened in the system's file
//! manager through `std::process::Command`. A desktop has no picker: an ask
//! for a file is refused, as `Platform::ask_for_file` does by default, and
//! the player drops the file on the window.

use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;

use shuyi_platform::Offered;
use shuyi_platform::player_files::{EXPORTS_FOLDER, numbered_name};
use shuyi_ui::player_files::Folder;

use crate::storage;

/// The most files of one name the exports folder numbers
/// ([`write_export`]): past it, an offer fails.
const MOST_NUMBERED: u32 = 10_000;

/// The player's files of the game whose storage is in `dir`.
#[derive(Debug)]
pub(crate) struct PlayerFiles {
    /// The game's data folder; `None` when the system has none.
    dir: Option<PathBuf>,
}

impl PlayerFiles {
    /// The player's files of the game whose storage is called `name`.
    pub(crate) fn new(name: &str) -> Self {
        Self {
            dir: storage::data_dir(name),
        }
    }

    /// Writes `text` as `name` in the exports folder, never over a file:
    /// where it went, or why it didn't.
    pub(crate) fn offer(&self, name: &str, text: &str) -> Offered {
        let Some(dir) = &self.dir else {
            return Offered::Failed(NO_DATA_FOLDER.to_owned());
        };
        match write_export(&dir.join(EXPORTS_FOLDER), name, text) {
            Ok(path) => Offered::Written(path.display().to_string()),
            Err(e) => Offered::Failed(format!("{name} wasn't written: {e}")),
        }
    }

    /// Opens `folder` in the system's file manager, making it first.
    pub(crate) fn open(&self, folder: Folder) -> Result<(), String> {
        let dir = self.dir.as_ref().ok_or(NO_DATA_FOLDER)?;
        let path = folder_path(dir, folder);
        fs::create_dir_all(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut child = Command::new(OPENER)
            .arg(&path)
            .spawn()
            .map_err(|e| format!("{OPENER} {}: {e}", path.display()))?;
        // Reaped on a thread of its own, so the frame doesn't wait for it.
        std::thread::spawn(move || child.wait());
        Ok(())
    }
}

/// Why nothing is written where the system has no data folder.
const NO_DATA_FOLDER: &str = "the system has no data folder for the game";

/// The program that opens a folder in the system's file manager.
const OPENER: &str = if cfg!(target_os = "windows") {
    "explorer"
} else if cfg!(target_os = "macos") {
    "open"
} else {
    "xdg-open"
};

/// `folder` of the game whose data folder is `dir`.
fn folder_path(dir: &Path, folder: Folder) -> PathBuf {
    match folder {
        Folder::Exports => dir.join(EXPORTS_FOLDER),
        _ => dir.to_path_buf(),
    }
}

/// Writes `text` in the folder `dir` (made if missing) as `name`, or the
/// first of `name (2)`, `name (3)` … that isn't there: never over a file,
/// even one made meanwhile. The path it wrote.
///
/// # Errors
///
/// The folder or the file couldn't be made or written, or
/// [`MOST_NUMBERED`] files of the name are there.
pub(crate) fn write_export(dir: &Path, name: &str, text: &str) -> io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    for n in 1..=MOST_NUMBERED {
        let path = dir.join(numbered_name(name, n));
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        return Ok(path);
    }
    Err(io::Error::other(format!(
        "{MOST_NUMBERED} files called {name} are there already"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder under the target directory, for one test.
    fn temp_dir(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("shuyi-exports-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn two_offers_of_a_name_write_it_and_a_numbered_one() {
        let data = temp_dir("two");
        let files = PlayerFiles {
            dir: Some(data.clone()),
        };
        let exports = data.join(EXPORTS_FOLDER);
        let first = files.offer("a.ron", "one");
        let second = files.offer("a.ron", "two");
        let path = |name: &str| exports.join(name).display().to_string();
        assert_eq!(first, Offered::Written(path("a.ron")));
        assert_eq!(second, Offered::Written(path("a (2).ron")));
        let read = |name: &str| fs::read_to_string(exports.join(name)).ok();
        assert_eq!(read("a.ron").as_deref(), Some("one"));
        assert_eq!(read("a (2).ron").as_deref(), Some("two"));
        let _ = fs::remove_dir_all(&data);
    }

    #[test]
    fn with_no_data_folder_an_offer_fails_and_nothing_opens() {
        let files = PlayerFiles { dir: None };
        assert_eq!(
            files.offer("a.ron", "x"),
            Offered::Failed(NO_DATA_FOLDER.to_owned())
        );
        assert_eq!(files.open(Folder::Storage), Err(NO_DATA_FOLDER.to_owned()));
    }

    #[test]
    fn a_folder_that_cant_be_made_fails_the_offer_saying_why() {
        let data = temp_dir("blocked");
        // A file where the folder would go.
        let _ = fs::write(&data, "in the way");
        let files = PlayerFiles {
            dir: Some(data.clone()),
        };
        let offered = files.offer("a.ron", "x");
        assert!(
            matches!(&offered, Offered::Failed(why) if why.starts_with("a.ron wasn't written")),
            "{offered:?}"
        );
        let _ = fs::remove_file(&data);
    }

    #[test]
    fn the_exports_folder_is_in_the_storage_folder() {
        let dir = Path::new("data");
        assert_eq!(folder_path(dir, Folder::Storage), dir);
        assert_eq!(folder_path(dir, Folder::Exports), dir.join("exports"));
        let opener = OPENER;
        assert!(["explorer", "open", "xdg-open"].contains(&opener));
    }
}
