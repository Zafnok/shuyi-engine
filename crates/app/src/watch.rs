//! The disk layers watched for changes, in a build with debug tools
//! (ADR-0071): [`read_folder`](crate::read_folder) notes each folder beside
//! its layer, and the backend's [`Poller`] compares its files' modification
//! times and sizes once a second, puts what changed in the layer
//! ([`Layer::replace`]) and says which paths changed
//! ([`Platform::changed_files`](shuyi_platform::Platform::changed_files)). A
//! file deleted keeps its last bytes until the game starts again: an editor
//! that saves by deleting and renaming would otherwise flash the embedded
//! file.

use std::collections::BTreeMap;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime};

use macroquad::logging::warn;
use shuyi_assets::bundle::Layer;
use shuyi_platform::Millis;

use crate::folder::{self, FolderError};

/// How often a [`Poller`] looks at its folders.
const EVERY: Duration = Duration::from_secs(1);

/// What says a file changed: its modification time, where the system has
/// one, and its size.
pub(crate) type Stamp = (Option<SystemTime>, u64);

/// A folder read into a layer, and what was last seen of its files, by path
/// in the layer.
#[derive(Debug)]
pub(crate) struct Watched {
    root: PathBuf,
    layer: Layer,
    max_file_bytes: u64,
    seen: BTreeMap<String, Stamp>,
}

impl Watched {
    /// Adds the paths of the files of the folder that changed to `out`, with
    /// their new bytes in the layer. A file that can't be read now (a saving
    /// editor holds it) is tried again next time; one over the limit is said
    /// once.
    fn poll(&mut self, disk: &impl Disk, out: &mut Vec<String>) {
        for (path, stamp) in disk.scan(&self.root) {
            if self.seen.get(&path) == Some(&stamp) {
                continue;
            }
            match disk.read(&self.root, &path, self.max_file_bytes) {
                Ok(bytes) => {
                    if self.layer.replace(&path, bytes) {
                        out.push(path.clone());
                    }
                }
                Err(error @ FolderError::TooBig { .. }) => {
                    warn!("hot reload: {}", error);
                }
                Err(_) => continue,
            }
            self.seen.insert(path, stamp);
        }
    }
}

/// The folders read since a [`Poller`] last took them.
static READ: Mutex<Vec<Watched>> = Mutex::new(Vec::new());

/// Notes the folder at `root`, read into `layer` after its files were seen
/// as `seen`, for the backend's [`Poller`].
pub(crate) fn note_folder(
    root: &Path,
    layer: Layer,
    max_file_bytes: u64,
    seen: BTreeMap<String, Stamp>,
) {
    let watched = Watched {
        root: root.to_path_buf(),
        layer,
        max_file_bytes,
        seen,
    };
    READ.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(watched);
}

/// Where a [`Poller`] looks: the disk, or a test's.
pub(crate) trait Disk {
    /// What is seen of each file under `root`, by path in the layer.
    fn scan(&self, root: &Path) -> BTreeMap<String, Stamp>;

    /// The bytes of the file at `path` under `root`.
    fn read(&self, root: &Path, path: &str, limit: u64) -> Result<Vec<u8>, FolderError>;
}

/// The disk.
pub(crate) struct RealDisk;

impl Disk for RealDisk {
    fn scan(&self, root: &Path) -> BTreeMap<String, Stamp> {
        scan(root)
    }

    fn read(&self, root: &Path, path: &str, limit: u64) -> Result<Vec<u8>, FolderError> {
        folder::read_file(&root.join(path), limit)
    }
}

/// What is seen of each file under `root`, by path in a layer of the folder:
/// as [`read_folder`](crate::read_folder) walks it. What can't be read or
/// named is left out.
pub(crate) fn scan(root: &Path) -> BTreeMap<String, Stamp> {
    let mut seen = BTreeMap::new();
    let mut pending = vec![(root.to_path_buf(), String::new())];
    while let Some((dir, prefix)) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let inner = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                pending.push((entry.path(), inner));
                continue;
            }
            // The listing already holds a file's facts; only a link needs
            // the file it points at looked up, which costs far more.
            let meta = if kind.is_symlink() {
                fs::metadata(entry.path())
            } else {
                entry.metadata()
            };
            if let Ok(meta) = meta
                && meta.is_file()
            {
                seen.insert(inner, stamp_of(&meta));
            }
        }
    }
    seen
}

/// What says the file of `meta` changed.
fn stamp_of(meta: &Metadata) -> Stamp {
    (meta.modified().ok(), meta.len())
}

/// Looks at the folders once a second, while debug tools are on.
#[derive(Debug)]
pub(crate) struct Poller {
    /// Whether it looks at all: never in a build without debug tools.
    on: bool,
    folders: Vec<Watched>,
    /// When it last looked.
    last: Option<Millis>,
}

impl Poller {
    /// A poller with no folders yet, on if `on`.
    pub(crate) const fn new(on: bool) -> Self {
        Self {
            on,
            folders: Vec::new(),
            last: None,
        }
    }

    /// Takes the folders read since the last call.
    pub(crate) fn take_read(&mut self) {
        if self.on {
            let mut read = READ.lock().unwrap_or_else(PoisonError::into_inner);
            self.folders.append(&mut read);
        }
    }

    /// Adds the paths of the files that changed to `out`, if a second has
    /// passed on `clock` since it last looked. Off, or with no folders, it
    /// never reads the clock.
    pub(crate) fn poll(
        &mut self,
        clock: impl FnOnce() -> Millis,
        disk: &impl Disk,
        out: &mut Vec<String>,
    ) {
        if !self.on || self.folders.is_empty() {
            return;
        }
        let now = clock();
        if self.last.is_some_and(|last| now.since(last) < EVERY) {
            return;
        }
        self.last = Some(now);
        for folder in &mut self.folders {
            folder.poll(disk, out);
        }
    }

    /// Adds a folder, for a test.
    #[cfg(test)]
    fn watch(&mut self, watched: Watched) {
        self.folders.push(watched);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::time::UNIX_EPOCH;

    use shuyi_assets::bundle::Bundle;

    use super::*;

    /// A disk of files at fake times: a path's time, size and bytes.
    #[derive(Default)]
    struct FakeDisk {
        files: RefCell<BTreeMap<String, (u64, Vec<u8>)>>,
        scans: Cell<u32>,
        locked: Cell<bool>,
    }

    impl FakeDisk {
        fn write(&self, path: &str, time: u64, bytes: &[u8]) {
            self.files
                .borrow_mut()
                .insert(path.to_owned(), (time, bytes.to_vec()));
        }
    }

    impl Disk for FakeDisk {
        fn scan(&self, _: &Path) -> BTreeMap<String, Stamp> {
            self.scans.set(self.scans.get() + 1);
            let files = self.files.borrow();
            let stamp = |(time, bytes): &(u64, Vec<u8>)| {
                let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
                (Some(UNIX_EPOCH + Duration::from_secs(*time)), len)
            };
            files
                .iter()
                .map(|(path, file)| (path.clone(), stamp(file)))
                .collect()
        }

        fn read(&self, root: &Path, path: &str, limit: u64) -> Result<Vec<u8>, FolderError> {
            if self.locked.get() {
                let error = std::io::Error::other("locked");
                let path = root.join(path);
                return Err(FolderError::Read { path, error });
            }
            let bytes = self.files.borrow()[path].1.clone();
            if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limit {
                let path = root.join(path);
                return Err(FolderError::TooBig { path, limit });
            }
            Ok(bytes)
        }
    }

    /// A poller watching a layer that holds what `disk` holds, and a bundle
    /// of the layer.
    fn watching(disk: &FakeDisk) -> (Poller, Bundle) {
        let files = disk
            .files
            .borrow()
            .iter()
            .map(|(path, (_, bytes))| (path.clone(), bytes.clone()))
            .collect();
        let layer = Layer::owned("disk".to_owned(), files).unwrap_or_else(|e| panic!("{e:?}"));
        let mut poller = Poller::new(true);
        poller.watch(Watched {
            root: PathBuf::from("root"),
            layer,
            max_file_bytes: 8,
            seen: disk.scan(Path::new("root")),
        });
        (poller, Bundle::new(vec![layer]))
    }

    /// What `poller` says changed at `ms`.
    fn poll_at(poller: &mut Poller, disk: &FakeDisk, ms: u64) -> Vec<String> {
        let mut out = Vec::new();
        poller.poll(|| Millis(ms), disk, &mut out);
        out
    }

    #[test]
    fn a_changed_file_is_told_once_with_its_new_bytes_in_the_layer() {
        let disk = FakeDisk::default();
        disk.write("a.txt", 1, b"old");
        disk.write("data/b.txt", 1, b"same");
        let (mut poller, bundle) = watching(&disk);
        assert_eq!(poll_at(&mut poller, &disk, 0), Vec::<String>::new());
        disk.write("a.txt", 2, b"new");
        assert_eq!(poll_at(&mut poller, &disk, 1000), ["a.txt"]);
        assert_eq!(bundle.file("a.txt"), Some("new"));
        assert_eq!(poll_at(&mut poller, &disk, 2000), Vec::<String>::new());
        // The same time with another size is a change too.
        disk.write("a.txt", 2, b"newer");
        assert_eq!(poll_at(&mut poller, &disk, 3000), ["a.txt"]);
        // A new file is added.
        disk.write("c.txt", 2, b"c");
        assert_eq!(poll_at(&mut poller, &disk, 4000), ["c.txt"]);
        assert_eq!(bundle.file("c.txt"), Some("c"));
    }

    #[test]
    fn it_looks_once_a_second_of_the_clock() {
        let disk = FakeDisk::default();
        disk.write("a.txt", 1, b"old");
        let (mut poller, _) = watching(&disk);
        let scans = disk.scans.get();
        let _ = poll_at(&mut poller, &disk, 100);
        assert_eq!(disk.scans.get(), scans + 1);
        disk.write("a.txt", 2, b"new");
        for ms in [101, 500, 1099] {
            assert_eq!(poll_at(&mut poller, &disk, ms), Vec::<String>::new());
        }
        assert_eq!(disk.scans.get(), scans + 1);
        assert_eq!(poll_at(&mut poller, &disk, 1100), ["a.txt"]);
        assert_eq!(disk.scans.get(), scans + 2);
    }

    #[test]
    fn with_debug_tools_off_it_never_reads_the_clock_or_the_disk() {
        let disk = FakeDisk::default();
        disk.write("a.txt", 1, b"old");
        let reads = Cell::new(0);
        let clock = || {
            reads.set(reads.get() + 1);
            Millis(0)
        };
        let mut out = Vec::new();
        let mut off = Poller::new(false);
        off.take_read();
        off.poll(clock, &disk, &mut out);
        assert_eq!((reads.get(), disk.scans.get()), (0, 0));
        assert_eq!(out, Vec::<String>::new());
        // On, with no folder, nothing to look at either.
        Poller::new(true).poll(clock, &disk, &mut out);
        assert_eq!((reads.get(), disk.scans.get()), (0, 0));
    }

    #[test]
    fn a_file_held_by_its_editor_is_tried_again() {
        let disk = FakeDisk::default();
        disk.write("a.txt", 1, b"old");
        let (mut poller, bundle) = watching(&disk);
        disk.write("a.txt", 2, b"new");
        disk.locked.set(true);
        assert_eq!(poll_at(&mut poller, &disk, 0), Vec::<String>::new());
        assert_eq!(bundle.file("a.txt"), Some("old"));
        disk.locked.set(false);
        assert_eq!(poll_at(&mut poller, &disk, 1000), ["a.txt"]);
        assert_eq!(bundle.file("a.txt"), Some("new"));
    }

    #[test]
    fn a_file_over_the_limit_keeps_its_bytes_and_is_passed_over_until_it_changes() {
        let disk = FakeDisk::default();
        disk.write("a.txt", 1, b"old");
        let (mut poller, bundle) = watching(&disk);
        disk.write("a.txt", 2, b"far too long a file");
        assert_eq!(poll_at(&mut poller, &disk, 0), Vec::<String>::new());
        assert_eq!(bundle.file("a.txt"), Some("old"));
        disk.write("a.txt", 3, b"short");
        assert_eq!(poll_at(&mut poller, &disk, 1000), ["a.txt"]);
    }

    #[test]
    fn a_deleted_file_keeps_its_last_bytes() {
        let disk = FakeDisk::default();
        disk.write("a.txt", 1, b"old");
        let (mut poller, bundle) = watching(&disk);
        disk.files.borrow_mut().clear();
        assert_eq!(poll_at(&mut poller, &disk, 0), Vec::<String>::new());
        assert_eq!(bundle.file("a.txt"), Some("old"));
    }

    #[test]
    fn the_real_disk_sees_a_folder_as_a_layer_of_it_does() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("data")).unwrap();
        fs::write(dir.path().join("a.txt"), b"aa").unwrap();
        fs::write(dir.path().join("data/b.txt"), b"bbb").unwrap();
        let seen = scan(dir.path());
        let sizes: Vec<_> = seen.iter().map(|(path, s)| (path.as_str(), s.1)).collect();
        assert_eq!(sizes, [("a.txt", 2), ("data/b.txt", 3)]);
        assert_eq!(
            RealDisk.read(dir.path(), "data/b.txt", 8).unwrap(),
            b"bbb".to_vec()
        );
        assert!(scan(&dir.path().join("nope")).is_empty());
    }

    /// `read_folder` hands its folder over in a build with debug tools.
    #[test]
    fn a_folder_read_is_taken_by_the_poller() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"old").unwrap();
        let layer = crate::read_folder(dir.path(), "disk", 16).unwrap();
        let mut poller = Poller::new(true);
        poller.take_read();
        // Other tests read folders too: this one is among them.
        let found = poller.folders.iter().any(|f| f.root == dir.path());
        assert_eq!(found, shuyi_ui::DEBUG_TOOLS);
        poller.folders.retain(|f| f.root == dir.path());
        fs::write(dir.path().join("a.txt"), b"new!").unwrap();
        let mut out = Vec::new();
        poller.poll(|| Millis(0), &RealDisk, &mut out);
        assert_eq!(out, ["a.txt"]);
        assert_eq!(Bundle::new(vec![layer]).file("a.txt"), Some("new!"));
    }
}
