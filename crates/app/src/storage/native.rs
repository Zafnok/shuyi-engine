//! [`FileStorage`]: one `.ron` file per key in the OS data directory.
//!
//! The data directory is resolved by hand from environment variables rather
//! than with the `directories` crate: it pulls in `dirs-sys`, which
//! unconditionally depends on `option-ext` (MPL-2.0), denied by game ADR-0013.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use shuyi_ui::storage::{Storage, StorageError, is_valid_key};

#[cfg(test)]
mod fake_disk;

/// A [`Storage`] backed by one file per key, under the OS's per-app data
/// directory ([`data_dir`]).
///
/// The folder holds `<key>.ron`, the value, and `<key>.ron.bak`, the
/// value before it (ticket 3045). A `<key>.ron.tmp` is there only between
/// a crash and the next write, which removes it. A sync of the folder
/// (Steam Auto-Cloud) must carry the `.bak` files with the `.ron` files:
/// a key whose `.ron` is gone reads its good `.bak`, so a save deleted on
/// one machine would come back from its backup on another.
///
/// A write puts the new value in `<key>.ron.tmp` and flushes it to the
/// disk, copies the old `<key>.ron` (when it is a good copy: not empty,
/// not all zero bytes, UTF-8) to `<key>.ron.bak` and flushes that, renames
/// the `.tmp` onto the `.ron`, and syncs the folder. A read of a `.ron`
/// that is missing, empty, all zero bytes or not UTF-8 beside a good
/// `.bak` reads the `.bak`, and logs a warning. What that promises:
///
/// - **a crash of the game**, on every OS: the key reads the old value or
///   the new one;
/// - **a power cut or a crash of the system**, on Linux (the Steam Deck)
///   and macOS: the key reads the old value or the new one, and a write
///   that returned `Ok` reads the new one, since the file and the folder
///   are flushed before it returns (`F_FULLFSYNC` on macOS). A file
///   system that can't sync a folder (it answers `Unsupported`) keeps the
///   rename only as well as it journals it;
/// - **a power cut** on Windows: `std` opens no folder for a sync, so the
///   folder isn't flushed. NTFS journals the rename, so the key reads the
///   old value or the new one, never damage; a write that returned in the
///   moments before the cut may read the old one.
///
/// A key written `""` or only `\0` characters keeps no backup, so it reads
/// back as written and not as damage.
#[derive(Debug)]
pub struct FileStorage {
    dir: PathBuf,
    disk: Box<dyn Disk>,
    /// Where a read that falls back to a backup says so.
    warn: fn(&str),
    /// Whether a write has removed the `.tmp` files a crash left.
    swept: bool,
}

impl FileStorage {
    /// Locates (but does not create) the data directory of the game whose
    /// storage is called `name` ([`data_dir`]). Fails only if the OS has
    /// no usable home directory.
    pub fn new(name: &str) -> Result<Self, StorageError> {
        let dir = data_dir(name).ok_or_else(|| {
            StorageError::Backend("could not determine an OS data directory".to_owned())
        })?;
        Ok(Self::at(dir, Box::new(StdDisk), log_warning))
    }

    /// A storage in `dir`, through `disk`, that warns with `warn`.
    fn at(dir: PathBuf, disk: Box<dyn Disk>, warn: fn(&str)) -> Self {
        Self {
            dir,
            disk,
            warn,
            swept: false,
        }
    }

    fn path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.ron"))
    }

    fn tmp_path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.ron.tmp"))
    }

    fn bak_path(&self, key: &str) -> PathBuf {
        self.dir.join(format!("{key}.ron.bak"))
    }

    /// The backup's text, if it is a good copy.
    fn good_backup(&self, key: &str) -> Option<String> {
        let bytes = self.disk.read(&self.bak_path(key)).ok()?;
        if is_good(&bytes) {
            String::from_utf8(bytes).ok()
        } else {
            None
        }
    }

    /// Removes the `.tmp` files a crash left in the folder.
    fn sweep(&self) -> Result<(), StorageError> {
        let names = self
            .disk
            .names(&self.dir)
            .map_err(|e| backend_error(&self.dir, &e))?;
        for name in names.iter().filter(|name| name.ends_with(".ron.tmp")) {
            remove_if_there(&*self.disk, &self.dir.join(name))?;
        }
        Ok(())
    }
}

/// Logs a warning of [`FileStorage`]'s.
fn log_warning(message: &str) {
    // check-text: not player text (a warning for the log)
    macroquad::prelude::warn!("storage: {}", message);
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
        let path = self.path(key);
        let main = match self.disk.read(&path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(backend_error(&path, &e)),
        };
        if !main.as_deref().is_some_and(is_good)
            && let Some(backup) = self.good_backup(key)
        {
            // check-text: not player text (a warning for the log)
            (self.warn)(&format!(
                "{key}: {}, read its backup instead",
                damage(main.as_deref())
            ));
            return Ok(Some(backup));
        }
        main.map(String::from_utf8)
            .transpose()
            .map_err(|e| StorageError::Backend(format!("{}: {e}", path.display())))
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        check_key(key)?;
        self.disk
            .create_dir_all(&self.dir)
            .map_err(|e| backend_error(&self.dir, &e))?;
        if !self.swept {
            self.sweep()?;
            self.swept = true;
        }
        let tmp = self.tmp_path(key);
        self.disk
            .write_synced(&tmp, value.as_bytes())
            .map_err(|e| backend_error(&tmp, &e))?;
        let dest = self.path(key);
        let bak = self.bak_path(key);
        if is_good(value.as_bytes()) {
            let old = match self.disk.read(&dest) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == io::ErrorKind::NotFound => None,
                Err(e) => return Err(backend_error(&dest, &e)),
            };
            if let Some(old) = old.filter(|old| is_good(old)) {
                self.disk
                    .write_synced(&bak, &old)
                    .map_err(|e| backend_error(&bak, &e))?;
            }
        } else {
            remove_if_there(&*self.disk, &bak)?;
        }
        self.disk
            .rename(&tmp, &dest)
            .map_err(|e| backend_error(&dest, &e))?;
        self.disk
            .sync_dir(&self.dir)
            .map_err(|e| backend_error(&self.dir, &e))
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        check_key(key)?;
        remove_if_there(&*self.disk, &self.bak_path(key))?;
        remove_if_there(&*self.disk, &self.path(key))?;
        match self.disk.sync_dir(&self.dir) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(backend_error(&self.dir, &e)),
            _ => Ok(()),
        }
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        let names = match self.disk.names(&self.dir) {
            Ok(names) => names,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(backend_error(&self.dir, &e)),
        };
        let mut keys = BTreeSet::new();
        for name in &names {
            if let Some(key) = name.strip_suffix(".ron") {
                keys.insert(key.to_owned());
            } else if let Some(key) = name.strip_suffix(".ron.bak")
                && !names.contains(&format!("{key}.ron"))
                && self.good_backup(key).is_some()
            {
                keys.insert(key.to_owned());
            }
        }
        Ok(keys.into_iter().collect())
    }
}

/// Whether a file's bytes are a good copy of a value: not empty, not all
/// zero bytes (what a power cut can leave), and UTF-8.
fn is_good(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().any(|&b| b != 0) && std::str::from_utf8(bytes).is_ok()
}

/// Why a main file that is not a good copy ([`is_good`]) isn't, for the
/// warning.
// check-text: not player text (a warning for the log)
fn damage(main: Option<&[u8]>) -> &'static str {
    match main {
        None => "the file is missing",
        Some([]) => "the file is empty",
        Some(bytes) if bytes.iter().all(|&b| b == 0) => "the file is all zero bytes",
        Some(_) => "the file is not UTF-8",
    }
}

/// Removes `path`; not an error if it isn't there.
fn remove_if_there(disk: &dyn Disk, path: &Path) -> Result<(), StorageError> {
    match disk.remove(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(backend_error(path, &e)),
        _ => Ok(()),
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

/// The file calls [`FileStorage`] makes: [`StdDisk`] in the game, a fake
/// that can lose the power in the tests. `Send` and `Sync`, as
/// [`FileStorage`] has always been.
trait Disk: fmt::Debug + Send + Sync {
    /// A file's bytes.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;
    /// Creates (or empties) a file, writes `bytes` and flushes them to the
    /// disk.
    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn remove(&self, path: &Path) -> io::Result<()>;
    /// Flushes a folder's names to the disk, so a rename survives the
    /// power.
    fn sync_dir(&self, dir: &Path) -> io::Result<()>;
    /// The file names in a folder.
    fn names(&self, dir: &Path) -> io::Result<Vec<String>>;
    fn create_dir_all(&self, dir: &Path) -> io::Result<()>;
}

/// The [`Disk`] of `std::fs`.
#[derive(Debug)]
struct StdDisk;

impl Disk for StdDisk {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut file = fs::File::create(path)?;
        file.write_all(bytes)?;
        file.sync_all()
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    /// On Unix, opens the folder and syncs it; a file system that doesn't
    /// sync folders (`Unsupported`, `InvalidInput`) is not an error.
    #[cfg(unix)]
    fn sync_dir(&self, dir: &Path) -> io::Result<()> {
        match fs::File::open(dir).and_then(|folder| folder.sync_all()) {
            Err(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::Unsupported | io::ErrorKind::InvalidInput
                ) =>
            {
                Ok(())
            }
            result => result,
        }
    }

    /// On Windows, nothing: `std` opens no folder for a sync, and NTFS
    /// journals the rename.
    #[cfg(not(unix))]
    fn sync_dir(&self, _dir: &Path) -> io::Result<()> {
        Ok(())
    }

    fn names(&self, dir: &Path) -> io::Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in fs::read_dir(dir)? {
            if let Ok(name) = entry?.file_name().into_string() {
                names.push(name);
            }
        }
        Ok(names)
    }

    fn create_dir_all(&self, dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shuyi_ui::storage::conformance_suite;
    use std::cell::RefCell;

    thread_local! {
        static WARNINGS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    /// The tests' `warn`: keeps the warning for [`warnings`].
    fn catch(message: &str) {
        WARNINGS.with(|warnings| warnings.borrow_mut().push(message.to_owned()));
    }

    /// The warnings this test's storage gave since the last call.
    fn warnings() -> Vec<String> {
        WARNINGS.with(RefCell::take)
    }

    fn storage_in(dir: PathBuf) -> FileStorage {
        FileStorage::at(dir, Box::new(StdDisk), catch)
    }

    fn storage() -> (FileStorage, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        (storage_in(tmp.path().to_owned()), tmp)
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn passes_the_conformance_suite() {
        let (mut storage, _tmp) = storage();
        conformance_suite(&mut storage);
    }

    #[test]
    fn the_storage_stays_send_and_sync() {
        fn send_and_sync<T: Send + Sync>() {}
        send_and_sync::<FileStorage>();
    }

    #[test]
    fn writes_are_atomic_and_do_not_leave_tmp_files_behind() {
        let (mut storage, tmp) = storage();
        storage.write("save-slot-1", "abc").unwrap();
        assert_eq!(files(tmp.path()), ["save-slot-1.ron"]);
    }

    #[test]
    fn a_second_write_keeps_the_first_value_as_the_backup() {
        let (mut storage, tmp) = storage();
        storage.write("save-slot-1", "first").unwrap();
        storage.write("save-slot-1", "second").unwrap();
        assert_eq!(
            files(tmp.path()),
            ["save-slot-1.ron", "save-slot-1.ron.bak"]
        );
        let bak = fs::read_to_string(tmp.path().join("save-slot-1.ron.bak")).unwrap();
        assert_eq!(bak, "first");
        assert_eq!(
            storage.read("save-slot-1").unwrap(),
            Some("second".to_owned())
        );
    }

    #[test]
    fn a_damaged_main_file_reads_its_backup_and_warns_why() {
        let damages: [(&str, Option<&[u8]>); 4] = [
            ("the file is missing", None),
            ("the file is empty", Some(b"")),
            ("the file is all zero bytes", Some(b"\0\0\0\0")),
            ("the file is not UTF-8", Some(b"\xff\xfe(gold")),
        ];
        for (why, bytes) in damages {
            let (mut storage, tmp) = storage();
            storage.write("save-slot-1", "first").unwrap();
            storage.write("save-slot-1", "second").unwrap();
            let main = tmp.path().join("save-slot-1.ron");
            match bytes {
                Some(bytes) => fs::write(&main, bytes).unwrap(),
                None => fs::remove_file(&main).unwrap(),
            }
            assert_eq!(
                storage.read("save-slot-1").unwrap(),
                Some("first".to_owned()),
                "{why}"
            );
            assert_eq!(
                warnings(),
                [format!("save-slot-1: {why}, read its backup instead")]
            );
        }
    }

    #[test]
    fn without_a_backup_a_damaged_main_file_reads_as_it_is() {
        let (storage, tmp) = storage();
        let main = tmp.path().join("save-slot-1.ron");
        assert_eq!(storage.read("save-slot-1").unwrap(), None);
        fs::write(&main, b"").unwrap();
        assert_eq!(storage.read("save-slot-1").unwrap(), Some(String::new()));
        fs::write(&main, b"\0\0").unwrap();
        assert_eq!(
            storage.read("save-slot-1").unwrap(),
            Some("\0\0".to_owned())
        );
        fs::write(&main, b"\xff\xfe").unwrap();
        assert!(matches!(
            storage.read("save-slot-1"),
            Err(StorageError::Backend(_))
        ));
        assert_eq!(warnings(), Vec::<String>::new());
    }

    #[test]
    fn a_damaged_backup_is_not_read() {
        let (storage, tmp) = storage();
        fs::write(tmp.path().join("save-slot-1.ron"), b"").unwrap();
        fs::write(tmp.path().join("save-slot-1.ron.bak"), b"\0\0").unwrap();
        assert_eq!(storage.read("save-slot-1").unwrap(), Some(String::new()));
        assert_eq!(warnings(), Vec::<String>::new());
    }

    #[test]
    fn writing_an_empty_or_zero_value_removes_the_backup_and_reads_back() {
        for value in ["", "\0\0"] {
            let (mut storage, tmp) = storage();
            storage.write("save-slot-1", "first").unwrap();
            storage.write("save-slot-1", "second").unwrap();
            storage.write("save-slot-1", value).unwrap();
            assert_eq!(files(tmp.path()), ["save-slot-1.ron"], "{value:?}");
            assert_eq!(storage.read("save-slot-1").unwrap(), Some(value.to_owned()));
            assert_eq!(warnings(), Vec::<String>::new());
        }
    }

    #[test]
    fn a_tmp_file_a_crash_left_is_ignored_and_gone_after_the_next_write() {
        let (mut storage, tmp) = storage();
        fs::write(tmp.path().join("save-slot-1.ron.tmp"), "(gold: 2").unwrap();
        assert_eq!(storage.read("save-slot-1").unwrap(), None);
        assert_eq!(storage.list().unwrap(), Vec::<String>::new());
        storage.write("settings", "{}").unwrap();
        assert_eq!(files(tmp.path()), ["settings.ron"]);
    }

    #[test]
    fn delete_removes_the_backup_too() {
        let (mut storage, tmp) = storage();
        storage.write("save-slot-1", "first").unwrap();
        storage.write("save-slot-1", "second").unwrap();
        storage.delete("save-slot-1").unwrap();
        assert_eq!(files(tmp.path()), Vec::<String>::new());
        assert_eq!(storage.read("save-slot-1").unwrap(), None);
    }

    #[test]
    fn list_names_keys_and_good_lone_backups_but_never_bak_or_tmp_files() {
        let (storage, tmp) = storage();
        for (name, bytes) in [
            ("main.ron", &b"1"[..]),
            ("main.ron.bak", b"0"),
            ("lone-backup.ron.bak", b"1"),
            ("empty-backup.ron.bak", b""),
            ("left.ron.tmp", b"1"),
        ] {
            fs::write(tmp.path().join(name), bytes).unwrap();
        }
        assert_eq!(storage.list().unwrap(), ["lone-backup", "main"]);
    }

    #[test]
    fn creates_the_directory_on_first_write() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nested").join("data");
        let mut storage = storage_in(dir.clone());
        assert!(!dir.exists());
        storage.write("settings", "{}").unwrap();
        assert!(dir.exists());
        assert_eq!(storage.read("settings").unwrap(), Some("{}".to_owned()));
    }

    #[test]
    fn missing_directory_lists_as_empty_not_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let mut storage = storage_in(tmp.path().join("never-created"));
        assert_eq!(storage.list().unwrap(), Vec::<String>::new());
        storage.delete("settings").unwrap();
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
