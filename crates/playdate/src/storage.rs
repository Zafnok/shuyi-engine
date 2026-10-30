//! The game's [`Storage`] on the Playdate's files: `<key>.ron` per key in the
//! game's data folder (`Disk/Data/<bundle id>/` in the Simulator), kept as
//! `shuyi-app`'s `FileStorage` keeps them (ticket 3045).
//!
//! A write puts the new value in `<key>.ron.tmp` and flushes it, copies the
//! old `<key>.ron` (when it is a good copy: not empty, not all zero bytes,
//! UTF-8) to `<key>.ron.bak` and flushes that, then renames the `.tmp` onto the
//! `.ron`. A read of a `.ron` that is missing, empty, all zero bytes or not
//! UTF-8 beside a good `.bak` reads the `.bak`. Where the SDK won't move a file
//! onto another, the old `.ron` goes first: its `.bak` holds it meanwhile.

use alloc::borrow::ToOwned;
use alloc::collections::BTreeSet;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use shuyi_ui::storage::{Storage, StorageError, conformance_suite, is_valid_key};

use crate::sdk::{Sdk, backend};

/// A [`Storage`] of one file per key in the game's data folder.
#[derive(Debug)]
pub struct SdkStorage<S> {
    sdk: S,
    /// The folder of the data folder its files are in: `""` for the data
    /// folder itself, else ending in `/`.
    folder: String,
    /// Whether a write has removed the `.tmp` files a crash left.
    swept: bool,
}

impl<S: Sdk> SdkStorage<S> {
    /// The game's storage on `sdk`'s files.
    pub fn new(sdk: S) -> Self {
        Self {
            sdk,
            folder: String::new(),
            swept: false,
        }
    }

    /// The storage in `folder` (a name without `/`) of the game's data
    /// folder, which a write makes.
    pub fn in_folder(sdk: S, folder: &str) -> Self {
        Self {
            sdk,
            folder: format!("{folder}/"),
            swept: false,
        }
    }

    fn path(&self, key: &str) -> String {
        format!("{}{key}.ron", self.folder)
    }

    fn tmp_path(&self, key: &str) -> String {
        format!("{}{key}.ron.tmp", self.folder)
    }

    fn bak_path(&self, key: &str) -> String {
        format!("{}{key}.ron.bak", self.folder)
    }

    /// The names in its folder.
    fn names(&self) -> Result<Vec<String>, StorageError> {
        let folder = self.folder.trim_end_matches('/');
        self.sdk.data_names(folder).map_err(|e| backend(folder, &e))
    }

    /// The backup's text, if it is a good copy.
    fn good_backup(&self, key: &str) -> Option<String> {
        let bak = self.bak_path(key);
        if !self.sdk.is_file(&bak) {
            return None;
        }
        let bytes = self.sdk.read_data(&bak).ok()?;
        if is_good(&bytes) {
            String::from_utf8(bytes).ok()
        } else {
            None
        }
    }

    /// Removes the `.tmp` files a crash left.
    fn sweep(&self) -> Result<(), StorageError> {
        for name in self.names()?.iter().filter(|n| n.ends_with(".ron.tmp")) {
            self.remove_if_there(&format!("{}{name}", self.folder))?;
        }
        Ok(())
    }

    fn remove_if_there(&self, path: &str) -> Result<(), StorageError> {
        if self.sdk.is_file(path) {
            self.sdk.remove_data(path).map_err(|e| backend(path, &e))?;
        }
        Ok(())
    }

    /// Moves the `.tmp` onto the `.ron`, the old one going first where the
    /// SDK won't move a file onto another.
    fn replace(&self, tmp: &str, dest: &str) -> Result<(), StorageError> {
        if self.sdk.rename_data(tmp, dest).is_ok() {
            return Ok(());
        }
        self.remove_if_there(dest)?;
        self.sdk
            .rename_data(tmp, dest)
            .map_err(|e| backend(dest, &e))
    }
}

/// Whether a file's bytes are a good copy of a value: not empty, not all
/// zero bytes (what a power cut can leave), and UTF-8.
fn is_good(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.iter().any(|&b| b != 0) && core::str::from_utf8(bytes).is_ok()
}

fn check_key(key: &str) -> Result<(), StorageError> {
    if is_valid_key(key) {
        Ok(())
    } else {
        Err(StorageError::InvalidKey(key.to_owned()))
    }
}

impl<S: Sdk> Storage for SdkStorage<S> {
    fn read(&self, key: &str) -> Result<Option<String>, StorageError> {
        check_key(key)?;
        let path = self.path(key);
        let main = if self.sdk.is_file(&path) {
            Some(self.sdk.read_data(&path).map_err(|e| backend(&path, &e))?)
        } else {
            None
        };
        if !main.as_deref().is_some_and(is_good)
            && let Some(backup) = self.good_backup(key)
        {
            return Ok(Some(backup));
        }
        main.map(String::from_utf8)
            .transpose()
            .map_err(|e| backend(&path, &format!("{e}")))
    }

    fn write(&mut self, key: &str, value: &str) -> Result<(), StorageError> {
        check_key(key)?;
        if !self.swept {
            let folder = self.folder.trim_end_matches('/');
            if !folder.is_empty() && self.sdk.data_names(folder).is_err() {
                self.sdk
                    .make_data_folder(folder)
                    .map_err(|e| backend(folder, &e))?;
            }
            self.sweep()?;
            self.swept = true;
        }
        let tmp = self.tmp_path(key);
        self.sdk
            .write_data(&tmp, value.as_bytes())
            .map_err(|e| backend(&tmp, &e))?;
        let dest = self.path(key);
        let bak = self.bak_path(key);
        if is_good(value.as_bytes()) {
            let old = if self.sdk.is_file(&dest) {
                Some(self.sdk.read_data(&dest).map_err(|e| backend(&dest, &e))?)
            } else {
                None
            };
            if let Some(old) = old.filter(|old| is_good(old)) {
                self.sdk
                    .write_data(&bak, &old)
                    .map_err(|e| backend(&bak, &e))?;
            }
        } else {
            self.remove_if_there(&bak)?;
        }
        self.replace(&tmp, &dest)
    }

    fn delete(&mut self, key: &str) -> Result<(), StorageError> {
        check_key(key)?;
        self.remove_if_there(&self.bak_path(key))?;
        self.remove_if_there(&self.path(key))
    }

    fn list(&self) -> Result<Vec<String>, StorageError> {
        // A folder not made yet holds no keys.
        let names = if self.swept || self.folder.is_empty() {
            self.names()?
        } else {
            self.names().unwrap_or_default()
        };
        let mut keys = BTreeSet::new();
        for name in &names {
            if let Some(key) = name.strip_suffix(".ron") {
                if is_valid_key(key) {
                    keys.insert(key.to_owned());
                }
            } else if let Some(key) = name.strip_suffix(".ron.bak")
                && is_valid_key(key)
                && !names.contains(&format!("{key}.ron"))
                && self.good_backup(key).is_some()
            {
                keys.insert(key.to_owned());
            }
        }
        Ok(keys.into_iter().collect())
    }
}

/// The folder of the data folder [`check`] runs in, away from the game's
/// own keys.
pub const CHECK_FOLDER: &str = "storage-check";

/// Runs the engine's storage conformance suite on `sdk`'s files, in
/// [`CHECK_FOLDER`], and leaves the folder empty; panics if it fails. A debug
/// build on the device runs it at start, on the real files.
pub fn check<S: Sdk>(sdk: S) {
    let mut storage = SdkStorage::in_folder(sdk, CHECK_FOLDER);
    let clear = |storage: &mut SdkStorage<S>| {
        let keys = storage.list().unwrap_or_else(|e| panic!("{e}"));
        for key in keys {
            storage.delete(&key).unwrap_or_else(|e| panic!("{e}"));
        }
    };
    clear(&mut storage);
    conformance_suite(&mut storage);
    clear(&mut storage);
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use shuyi_ui::storage::{PowerCutRig, power_cut_suite};

    use super::*;
    use crate::fake::{FakeSdk, Rename};

    /// A storage over the fake SDK's files, which can lose the power.
    #[derive(Debug)]
    struct Rig {
        sdk: FakeSdk,
        storage: SdkStorage<FakeSdk>,
    }

    impl Rig {
        fn new(rename: Rename) -> Self {
            let sdk = FakeSdk::default();
            sdk.set_rename(rename);
            Self {
                storage: SdkStorage::new(sdk.clone()),
                sdk,
            }
        }
    }

    impl PowerCutRig for Rig {
        fn storage(&mut self) -> &mut dyn Storage {
            &mut self.storage
        }

        fn cut_after(&mut self, ops: u32) {
            self.sdk.cut_power_after(ops);
        }

        fn restart(&mut self) {
            self.sdk.restore_power();
            self.storage = SdkStorage::new(self.sdk.clone());
        }
    }

    #[test]
    fn it_passes_the_conformance_suite() {
        conformance_suite(&mut SdkStorage::new(FakeSdk::default()));
    }

    #[test]
    fn it_passes_where_a_file_cannot_be_moved_onto_another() {
        let sdk = FakeSdk::default();
        sdk.set_rename(Rename::Refuses);
        conformance_suite(&mut SdkStorage::new(sdk));
    }

    #[test]
    fn it_passes_in_a_folder_and_the_check_leaves_nothing() {
        conformance_suite(&mut SdkStorage::in_folder(FakeSdk::default(), "saves"));
        let sdk = FakeSdk::default();
        sdk.put_file("settings.ron", b"(a)");
        check(sdk.clone());
        check(sdk.clone());
        assert_eq!(
            sdk.data_names("").unwrap_or_default(),
            ["settings.ron".to_string(), "storage-check/".to_string()]
        );
        assert_eq!(
            sdk.data_names(CHECK_FOLDER).unwrap_or_default(),
            [] as [String; 0]
        );
    }

    #[test]
    fn it_passes_the_power_cut_suite() {
        power_cut_suite(&mut Rig::new(Rename::Replaces));
    }

    #[test]
    fn it_passes_the_power_cut_suite_where_a_file_cannot_be_moved_onto_another() {
        power_cut_suite(&mut Rig::new(Rename::Refuses));
    }

    #[test]
    fn a_write_goes_tmp_then_bak_then_rename_each_flushed() {
        let sdk = FakeSdk::default();
        let mut storage = SdkStorage::new(sdk.clone());
        storage
            .write("slot", "(a)")
            .unwrap_or_else(|e| panic!("{e}"));
        sdk.take_file_calls();
        storage
            .write("slot", "(b)")
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            sdk.take_file_calls(),
            [
                "write slot.ron.tmp",
                "write slot.ron.bak",
                "rename slot.ron.tmp slot.ron",
            ]
        );
        assert_eq!(sdk.file("slot.ron.bak").as_deref(), Some(&b"(a)"[..]));
        assert_eq!(
            storage.read("slot").unwrap_or_default().as_deref(),
            Some("(b)")
        );
    }

    #[test]
    fn damage_reads_the_backup() {
        let sdk = FakeSdk::default();
        let mut storage = SdkStorage::new(sdk.clone());
        storage
            .write("slot", "(a)")
            .unwrap_or_else(|e| panic!("{e}"));
        storage
            .write("slot", "(b)")
            .unwrap_or_else(|e| panic!("{e}"));
        sdk.put_file("slot.ron", &[0, 0, 0]);
        assert_eq!(
            storage.read("slot").unwrap_or_default().as_deref(),
            Some("(a)")
        );
        sdk.put_file("slot.ron", &[0xff]);
        sdk.put_file("slot.ron.bak", &[0xff]);
        assert!(matches!(
            storage.read("slot"),
            Err(StorageError::Backend(_))
        ));
    }

    #[test]
    fn only_keys_are_listed() {
        let sdk = FakeSdk::default();
        let mut storage = SdkStorage::new(sdk.clone());
        storage
            .write("settings", "()")
            .unwrap_or_else(|e| panic!("{e}"));
        sdk.put_file("load-report.txt", b"");
        sdk.put_file("Bad Name.ron", b"");
        sdk.put_file("slot.ron.tmp", b"");
        let keys = storage.list().unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(keys, ["settings".to_string()]);
    }

    #[test]
    fn a_bad_key_is_an_error() {
        let mut storage = SdkStorage::new(FakeSdk::default());
        assert!(matches!(
            storage.read("a/b"),
            Err(StorageError::InvalidKey(_))
        ));
        assert!(matches!(
            storage.write("a/b", "x"),
            Err(StorageError::InvalidKey(_))
        ));
        assert!(matches!(
            storage.delete("a/b"),
            Err(StorageError::InvalidKey(_))
        ));
    }
}
