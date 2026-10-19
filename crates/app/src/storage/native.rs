//! [`FileStorage`]: one `.ron` file per key in the OS data directory.
//!
//! The data directory is resolved by hand from environment variables rather
//! than with the `directories` crate: it pulls in `dirs-sys`, which
//! unconditionally depends on `option-ext` (MPL-2.0), denied by game ADR-0013.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use shuyi_ui::storage::{Storage, StorageError, is_valid_key};

/// A [`Storage`] backed by one file per key, under the OS's per-app data
/// directory ([`data_dir`]). Writes are atomic: the new content lands in
/// `<key>.ron.tmp`, then renames onto `<key>.ron`, so a crash mid-write
/// never corrupts the previous save.
#[derive(Debug)]
pub struct FileStorage {
    dir: PathBuf,
}

impl FileStorage {
    /// Locates (but does not create) the data directory of the game whose
    /// storage is called `name` ([`data_dir`]). Fails only if the OS has
    /// no usable home directory.
    pub fn new(name: &str) -> Result<Self, StorageError> {
        let dir = data_dir(name).ok_or_else(|| {
            StorageError::Backend("could not determine an OS data directory".to_owned())
        })?;
        Ok(Self { dir })
    }

    fn path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.ron"))
    }

    fn tmp_path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.ron.tmp"))
    }
}

/// The OS's per-app data directory for the game whose storage is called
/// `name`: that folder in `%APPDATA%` on Windows, in
/// `~/Library/Application Support` on macOS, in `$XDG_DATA_HOME` (or
/// `~/.local/share`) elsewhere on Unix. `None` if the relevant environment
/// variable isn't set.
pub fn data_dir(name: &str) -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|dir| PathBuf::from(dir).join(name))
    }
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join("Library/Application Support")
                .join(name)
        })
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg_data_home) = std::env::var_os("XDG_DATA_HOME") {
            Some(PathBuf::from(xdg_data_home).join(name))
        } else {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share").join(name))
        }
    }
}

impl Storage for FileStorage {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        check_key(key)?;
        match fs::read_to_string(self.path(key)) {
            Ok(contents) => Ok(Some(contents)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(backend_error(&self.path(key), &e)),
        }
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        check_key(key)?;
        fs::create_dir_all(&self.dir).map_err(|e| backend_error(&self.dir, &e))?;
        let tmp = self.tmp_path(key);
        fs::write(&tmp, value).map_err(|e| backend_error(&tmp, &e))?;
        let dest = self.path(key);
        fs::rename(&tmp, &dest).map_err(|e| backend_error(&dest, &e))
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        check_key(key)?;
        match fs::remove_file(self.path(key)) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(backend_error(&self.path(key), &e)),
        }
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(backend_error(&self.dir, &e)),
        };
        let mut keys = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| backend_error(&self.dir, &e))?;
            if let Some(key) = entry
                .path()
                .file_stem()
                .and_then(|s| s.to_str())
                .filter(|_| entry.path().extension().is_some_and(|ext| ext == "ron"))
            {
                keys.push(key.to_owned());
            }
        }
        Ok(keys)
    }
}

fn check_key(key: &str) -> Result<(), StorageError> {
    if is_valid_key(key) {
        Ok(())
    } else {
        Err(StorageError::InvalidKey(key.to_owned()))
    }
}

fn backend_error(path: &Path, e: &io::Error) -> StorageError {
    StorageError::Backend(format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shuyi_ui::storage::conformance_suite;

    fn storage() -> (FileStorage, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        (
            FileStorage {
                dir: tmp.path().to_owned(),
            },
            tmp,
        )
    }

    #[test]
    fn passes_the_conformance_suite() {
        let (mut storage, _tmp) = storage();
        conformance_suite(&mut storage);
    }

    #[test]
    fn writes_are_atomic_and_do_not_leave_tmp_files_behind() {
        let (mut storage, tmp) = storage();
        storage.write("save-slot-1", "abc").unwrap();
        let entries: Vec<_> = fs::read_dir(tmp.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(entries, ["save-slot-1.ron"]);
    }

    #[test]
    fn creates_the_directory_on_first_write() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nested").join("data");
        let mut storage = FileStorage { dir: dir.clone() };
        assert!(!dir.exists());
        storage.write("settings", "{}").unwrap();
        assert!(dir.exists());
        assert_eq!(storage.read("settings").unwrap(), Some("{}".to_owned()));
    }

    #[test]
    fn missing_directory_lists_as_empty_not_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage {
            dir: tmp.path().join("never-created"),
        };
        assert_eq!(storage.list().unwrap(), Vec::<String>::new());
    }

    #[test]
    fn the_data_directory_is_the_storage_name_in_the_os_data_folder() {
        let dir = data_dir("some-game");
        assert!(
            dir.as_ref().is_some_and(|d| d.ends_with("some-game")),
            "{dir:?}"
        );
        #[cfg(target_os = "windows")]
        assert_eq!(
            dir,
            std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("some-game"))
        );
    }
}
