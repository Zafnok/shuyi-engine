//! Which folder a desktop keeps the storage in ([`DesktopFolder`], ticket
//! 3310, ADR-0108), and the move of the old storage into a new folder on
//! the first launch that uses it.
//!
//! The move copies every file under the old folder into the new one, each
//! to the same place, never over a file that is there (`create_new`), and
//! flushes each copy. Only when every copy is made are the old files
//! removed. A copy that fails removes the copies this move made and leaves
//! the old folder as it was, and that launch keeps the old folder. A
//! removal that fails leaves an old file beside its copy: the next launch
//! finds keys in the new folder and never moves again.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

use super::DesktopFolder;
use super::native::data_dir;

/// The folder of the game whose storage is called `name`, as `desktop`
/// says, without moving anything: [`data_dir`] for
/// [`DesktopFolder::Data`] and off Windows, `<Documents>\My Games\<folder>`
/// on Windows for [`DesktopFolder::MyGames`] (the system's Documents, or
/// `%USERPROFILE%\Documents` if it won't say). `None` if the system has no
/// such folder.
pub fn desktop_dir(name: &str, desktop: DesktopFolder) -> Option<PathBuf> {
    match desktop {
        DesktopFolder::MyGames(folder) if cfg!(windows) => {
            documents().map(|docs| docs.join("My Games").join(folder))
        }
        _ => data_dir(name),
    }
}

/// The folder of the game whose storage is called `name`, as `desktop`
/// says, with the storage of its [`DesktopFolder::Data`] folder moved
/// there first when that is a different folder ([`settle`]).
pub(super) fn find(name: &str, desktop: DesktopFolder) -> Option<PathBuf> {
    let new = desktop_dir(name, desktop);
    settle(data_dir(name), new, log)
}

/// Logs a line of the move.
fn log(line: &str) {
    // check-text: not player text (a line for the log)
    macroquad::prelude::info!("storage: {}", line);
}

/// The folder to use when the storage was in `old` and belongs in `new`:
/// `new`, once `old`'s files are moved there when `new` holds no key yet
/// ([`holds_keys`]); `old` if the move fails, which `log` says. A `new`
/// that holds keys is used as it is and `old` left alone.
pub(super) fn settle(old: Option<PathBuf>, new: Option<PathBuf>, log: fn(&str)) -> Option<PathBuf> {
    let (old, new) = match (old, new) {
        (Some(old), Some(new)) => (old, new),
        (old, new) => return new.or(old),
    };
    if old == new {
        return Some(new);
    }
    match holds_keys(&new) {
        Ok(true) => return Some(new),
        Ok(false) => {}
        Err(e) => {
            log(&format!(
                "{} can't be read ({e}); keeping {} for now",
                new.display(),
                old.display()
            ));
            return Some(old);
        }
    }
    match move_files(&old, &new) {
        Ok(Moved { files: 0, .. }) => {}
        Ok(Moved { files, left }) => {
            log(&format!(
                "moved {files} files from {} to {}",
                old.display(),
                new.display()
            ));
            for (path, e) in left {
                log(&format!(
                    "{} was copied but not removed: {e}",
                    path.display()
                ));
            }
        }
        Err(e) => {
            log(&format!(
                "the files of {} couldn't move to {} ({e}); keeping {} for now",
                old.display(),
                new.display(),
                old.display()
            ));
            return Some(old);
        }
    }
    Some(new)
}

/// Whether `dir` holds a key of a [`FileStorage`](super::FileStorage): a
/// `.ron` file, or a `.ron.bak`. A folder that isn't there holds none.
fn holds_keys(dir: &Path) -> io::Result<bool> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    for entry in entries {
        let name = entry?.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".ron") || name.ends_with(".ron.bak") {
            return Ok(true);
        }
    }
    Ok(false)
}

/// What a move did.
#[derive(Debug)]
struct Moved {
    /// How many files moved.
    files: usize,
    /// The old files that were copied but couldn't be removed, and why.
    left: Vec<(PathBuf, io::Error)>,
}

/// Moves every file under `old` to the same place under `new` (module
/// docs): all of them, or none.
fn move_files(old: &Path, new: &Path) -> io::Result<Moved> {
    let mut files = Vec::new();
    match list_files(old, Path::new(""), &mut files) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Ok(Moved {
                files: 0,
                left: Vec::new(),
            });
        }
        result => result?,
    }
    let mut made: Vec<PathBuf> = Vec::new();
    for file in &files {
        let to = new.join(file);
        if let Err(e) = copy_new(&old.join(file), &to) {
            for copy in made.iter().rev() {
                let _ = fs::remove_file(copy);
            }
            return Err(io::Error::new(e.kind(), format!("{}: {e}", to.display())));
        }
        made.push(to);
    }
    let mut left = Vec::new();
    for file in &files {
        let from = old.join(file);
        if let Err(e) = fs::remove_file(&from) {
            left.push((from, e));
        }
    }
    remove_empty_folders(old);
    Ok(Moved {
        files: files.len(),
        left,
    })
}

/// Adds the files under `root.join(dir)` to `files`, by their paths from
/// `root`; links are left where they are.
fn list_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(root.join(dir))? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = dir.join(entry.file_name());
        if kind.is_dir() {
            list_files(root, &path, files)?;
        } else if kind.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

/// Copies `from` to `to`, a file that must not exist yet, and flushes it;
/// a copy that fails part way is removed.
fn copy_new(from: &Path, to: &Path) -> io::Result<()> {
    if let Some(folder) = to.parent() {
        fs::create_dir_all(folder)?;
    }
    let mut source = File::open(from)?;
    let mut copy = OpenOptions::new().write(true).create_new(true).open(to)?;
    let copied = io::copy(&mut source, &mut copy).and_then(|_| copy.sync_all());
    if copied.is_err() {
        drop(copy);
        let _ = fs::remove_file(to);
    }
    copied
}

/// Removes `dir` and the folders under it that are empty, deepest first.
fn remove_empty_folders(dir: &Path) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                remove_empty_folders(&entry.path());
            }
        }
    }
    let _ = fs::remove_dir(dir);
}

/// The user's Documents folder: the system's known folder, wherever it was
/// moved, or `%USERPROFILE%\Documents` if the system won't say.
#[cfg(windows)]
fn documents() -> Option<PathBuf> {
    known_documents().or_else(|| {
        std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("Documents"))
    })
}

/// Off Windows, no Documents folder is used.
#[cfg(not(windows))]
fn documents() -> Option<PathBuf> {
    None
}

/// The Documents known folder (`SHGetKnownFolderPath`).
#[cfg(windows)]
#[allow(unsafe_code)] // The one Win32 call that names the folder (ADR-0108).
fn known_documents() -> Option<PathBuf> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt as _;

    use winapi::shared::winerror::S_OK;
    use winapi::um::combaseapi::CoTaskMemFree;
    use winapi::um::knownfolders::FOLDERID_Documents;
    use winapi::um::shlobj::SHGetKnownFolderPath;

    let mut path: *mut u16 = std::ptr::null_mut();
    // SAFETY: the folder id is a valid GUID, a null token is the current
    // user's, and `path` is a valid place for the answer.
    let result = unsafe {
        SHGetKnownFolderPath(&FOLDERID_Documents, 0, std::ptr::null_mut(), &raw mut path)
    };
    let found = if result == S_OK && !path.is_null() {
        // SAFETY: on `S_OK` `path` is a NUL-terminated wide string that
        // stays valid until it is freed below.
        let wide = unsafe {
            let mut len = 0;
            while *path.add(len) != 0 {
                len += 1;
            }
            std::slice::from_raw_parts(path, len)
        };
        Some(PathBuf::from(OsString::from_wide(wide)))
    } else {
        None
    };
    // SAFETY: the system allocated `path` (or left it null, which is
    // allowed), and it is freed once, whether the call succeeded or not.
    unsafe { CoTaskMemFree(path.cast()) };
    found.filter(|dir| !dir.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    thread_local! {
        static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    fn catch(line: &str) {
        LOG.with(|log| log.borrow_mut().push(line.to_owned()));
    }

    fn logged() -> Vec<String> {
        LOG.with(RefCell::take)
    }

    /// The files under `dir` and their text, by their paths from `dir`
    /// with `/` between folders.
    fn tree(dir: &Path) -> Vec<(String, String)> {
        let mut files = Vec::new();
        if dir.exists() {
            list_files(dir, Path::new(""), &mut files).unwrap();
        }
        let mut tree: Vec<_> = files
            .iter()
            .map(|file| {
                let text = fs::read_to_string(dir.join(file)).unwrap();
                (file.to_string_lossy().replace('\\', "/"), text)
            })
            .collect();
        tree.sort();
        tree
    }

    fn write(dir: &Path, file: &str, text: &str) {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// An old folder with keys, backups and an export, and a new one that
    /// isn't there yet.
    fn places() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("AppData").join("a-game");
        let new = tmp.path().join("Documents").join("My Games").join("A Game");
        write(&old, "settings.ron", "(volume: 3)");
        write(&old, "slot-1.ron", "(gold: 2)");
        write(&old, "slot-1.ron.bak", "(gold: 1)");
        write(&old, "exports/slot-1.ron", "(gold: 2)");
        (tmp, old, new)
    }

    fn the_old_files() -> Vec<(String, String)> {
        [
            ("exports/slot-1.ron", "(gold: 2)"),
            ("settings.ron", "(volume: 3)"),
            ("slot-1.ron", "(gold: 2)"),
            ("slot-1.ron.bak", "(gold: 1)"),
        ]
        .map(|(file, text)| (file.to_owned(), text.to_owned()))
        .to_vec()
    }

    #[test]
    fn the_first_launch_moves_every_file_and_the_second_finds_them_there() {
        let (_tmp, old, new) = places();
        let found = settle(Some(old.clone()), Some(new.clone()), catch);
        assert_eq!(found.as_deref(), Some(new.as_path()));
        assert_eq!(tree(&new), the_old_files());
        assert!(!old.exists(), "the emptied old folder is removed");
        assert_eq!(logged().len(), 1);
        let again = settle(Some(old.clone()), Some(new.clone()), catch);
        assert_eq!(again.as_deref(), Some(new.as_path()));
        assert_eq!(tree(&new), the_old_files());
        assert_eq!(logged(), Vec::<String>::new());
    }

    #[test]
    fn a_new_folder_with_keys_is_used_as_it_is_and_the_old_left_alone() {
        let (_tmp, old, new) = places();
        write(&new, "slot-1.ron.bak", "(gold: 9)");
        let found = settle(Some(old.clone()), Some(new.clone()), catch);
        assert_eq!(found.as_deref(), Some(new.as_path()));
        assert_eq!(tree(&old), the_old_files());
        assert_eq!(
            tree(&new),
            [("slot-1.ron.bak".to_owned(), "(gold: 9)".to_owned())]
        );
        assert_eq!(logged(), Vec::<String>::new());
    }

    #[test]
    fn a_move_that_fails_keeps_the_old_folder_whole_and_writes_over_nothing() {
        let (_tmp, old, new) = places();
        // A file in the new folder, but no key: the move goes ahead and
        // stops at the export of the same name.
        write(&new, "exports/slot-1.ron", "(gold: 7)");
        let found = settle(Some(old.clone()), Some(new.clone()), catch);
        assert_eq!(found.as_deref(), Some(old.as_path()));
        assert_eq!(tree(&old), the_old_files());
        assert_eq!(
            tree(&new),
            [("exports/slot-1.ron".to_owned(), "(gold: 7)".to_owned())]
        );
        let log = logged();
        assert_eq!(log.len(), 1);
        assert!(log[0].contains("couldn't move"), "{log:?}");
    }

    #[test]
    fn with_nothing_to_move_the_new_folder_is_used() {
        let tmp = tempfile::tempdir().unwrap();
        let old = tmp.path().join("never-made");
        let new = tmp.path().join("new");
        let found = settle(Some(old), Some(new.clone()), catch);
        assert_eq!(found, Some(new.clone()));
        assert!(!new.exists(), "nothing is made before the first write");
        assert_eq!(logged(), Vec::<String>::new());
    }

    #[test]
    fn a_missing_folder_falls_back_to_the_other() {
        let a = PathBuf::from("a");
        assert_eq!(settle(Some(a.clone()), None, catch), Some(a.clone()));
        assert_eq!(settle(None, Some(a.clone()), catch), Some(a.clone()));
        assert_eq!(settle(Some(a.clone()), Some(a.clone()), catch), Some(a));
        assert_eq!(settle(None, None, catch), None);
    }

    #[test]
    fn the_data_folder_is_the_data_dir_and_my_games_is_in_documents_on_windows() {
        assert_eq!(
            desktop_dir("a-game", DesktopFolder::Data),
            data_dir("a-game")
        );
        let my_games = desktop_dir("a-game", DesktopFolder::MyGames("A Game"));
        if cfg!(windows) {
            let dir = my_games.unwrap();
            assert!(
                dir.ends_with(Path::new("My Games").join("A Game")),
                "{dir:?}"
            );
            let documents = dir.parent().and_then(Path::parent).unwrap();
            assert!(documents.is_dir(), "{documents:?}");
        } else {
            assert_eq!(my_games, data_dir("a-game"));
        }
    }
}
