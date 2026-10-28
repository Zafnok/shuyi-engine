//! A fake [`Disk`] that can lose the power, and [`FileStorage`] run
//! through `power_cut_suite` over it (ticket 3045).
//!
//! The folder maps names to inodes. A file's bytes are durable as they are
//! written, as `sync_all` makes them; the folder's names are durable only
//! at its last `sync_dir`. A process crash keeps what was done; a power
//! cut puts the names back as they were at the last `sync_dir`.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use shuyi_ui::storage::{PowerCutRig, Storage, conformance_suite, power_cut_suite};

use super::{Disk, FileStorage};

#[derive(Debug, Default)]
struct Medium {
    /// Every file's bytes, by inode.
    inodes: Vec<Vec<u8>>,
    /// The folder as the system sees it: names to inodes.
    names: BTreeMap<String, usize>,
    /// The folder as at its last sync.
    durable: BTreeMap<String, usize>,
    /// Operations left before the power goes, if a cut is pending.
    ops_left: Option<u32>,
}

impl Medium {
    /// One operation: fails at and after the cut.
    fn op(&mut self) -> io::Result<()> {
        match self.ops_left {
            Some(0) => Err(io::Error::other("the power is off")),
            Some(left) => {
                self.ops_left = Some(left - 1);
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn inode(&self, path: &Path) -> io::Result<usize> {
        self.names
            .get(&name(path))
            .copied()
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A handle on the one fake folder, shared by the rig and its storage.
#[derive(Debug, Clone, Default)]
struct FakeDisk(Arc<Mutex<Medium>>);

impl FakeDisk {
    fn medium(&self) -> MutexGuard<'_, Medium> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl Disk for FakeDisk {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        let mut medium = self.medium();
        medium.op()?;
        let inode = medium.inode(path)?;
        Ok(medium.inodes[inode].clone())
    }

    /// A new name gets a new inode with the bytes: one operation. A name
    /// that is there has its inode emptied, then filled, as `File::create`
    /// truncates in place: two.
    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let mut medium = self.medium();
        medium.op()?;
        if let Ok(inode) = medium.inode(path) {
            medium.inodes[inode].clear();
            medium.op()?;
            medium.inodes[inode] = bytes.to_vec();
        } else {
            medium.inodes.push(bytes.to_vec());
            let inode = medium.inodes.len() - 1;
            medium.names.insert(name(path), inode);
        }
        Ok(())
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut medium = self.medium();
        medium.op()?;
        let inode = medium.inode(from)?;
        medium.names.remove(&name(from));
        medium.names.insert(name(to), inode);
        Ok(())
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        let mut medium = self.medium();
        medium.op()?;
        medium.inode(path)?;
        medium.names.remove(&name(path));
        Ok(())
    }

    fn sync_dir(&self, _dir: &Path) -> io::Result<()> {
        let mut medium = self.medium();
        medium.op()?;
        medium.durable = medium.names.clone();
        Ok(())
    }

    fn names(&self, _dir: &Path) -> io::Result<Vec<String>> {
        let mut medium = self.medium();
        medium.op()?;
        Ok(medium.names.keys().cloned().collect())
    }

    fn create_dir_all(&self, _dir: &Path) -> io::Result<()> {
        self.medium().op()
    }
}

/// A [`FileStorage`] over a [`FakeDisk`].
#[derive(Debug)]
struct Rig {
    disk: FakeDisk,
    storage: FileStorage,
    /// A power cut, or a process crash.
    power_cut: bool,
}

impl Rig {
    fn new(power_cut: bool) -> Self {
        let disk = FakeDisk::default();
        Self {
            storage: open(&disk),
            disk,
            power_cut,
        }
    }
}

/// A new [`FileStorage`] over `disk`, as at the game's start.
fn open(disk: &FakeDisk) -> FileStorage {
    FileStorage::at(PathBuf::from("saves"), Box::new(disk.clone()), |_| {})
}

impl PowerCutRig for Rig {
    fn storage(&mut self) -> &mut dyn Storage {
        &mut self.storage
    }

    fn cut_after(&mut self, ops: u32) {
        self.disk.medium().ops_left = Some(ops);
    }

    fn restart(&mut self) {
        {
            let mut medium = self.disk.medium();
            if self.power_cut {
                medium.names = medium.durable.clone();
            } else {
                medium.durable = medium.names.clone();
            }
            medium.ops_left = None;
        }
        self.storage = open(&self.disk);
    }
}

#[test]
fn passes_the_conformance_suite() {
    conformance_suite(&mut open(&FakeDisk::default()));
}

#[test]
fn survives_a_process_crash_at_every_step() {
    power_cut_suite(&mut Rig::new(false));
}

#[test]
fn survives_a_power_cut_at_every_step() {
    power_cut_suite(&mut Rig::new(true));
}

/// A leftover `.tmp` from a crash is swept by the next storage's first
/// write.
#[test]
fn a_restarted_storage_sweeps_the_tmp_files_again() {
    let mut rig = Rig::new(false);
    rig.storage.write("settings", "{}").unwrap();
    rig.cut_after(2);
    rig.storage.write("save-slot-1", "(gold: 2)").unwrap_err();
    rig.restart();
    let names = |rig: &Rig| -> Vec<String> { rig.disk.medium().names.keys().cloned().collect() };
    assert_eq!(names(&rig), ["save-slot-1.ron.tmp", "settings.ron"]);
    rig.storage.write("settings", "{}").unwrap();
    assert_eq!(names(&rig), ["settings.ron", "settings.ron.bak"]);
}
