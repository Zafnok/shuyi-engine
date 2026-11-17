//! A pack read file by file (ADR-0128 §2): the [`Source`] a backend gives
//! of a pack's bytes, and the layer over it ([`Index::into_layer`]), which
//! reads, checks and unpacks one file each time the bundle asks for it, and
//! keeps nothing of it.
use crate::prelude::*;

use alloc::borrow::Cow;

use once_cell::race::OnceBox;

use core::panic::RefUnwindSafe;
use core::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

use super::{Codec, Index, Kept, PackError, checked, folder, under};
use crate::bundle::{Files, Layer, Unread};

/// Where a layer read file by file finds a pack's bytes: a backend's file
/// (the Playdate's, read with a seek), or its bytes as they arrive (the
/// web's). `shuyi-assets` does no I/O: it calls this, on the thread that
/// reads the bundle. It is shared as the installed bundle is (`Send`,
/// `Sync`), and a bundle that holds it may be read across a caught panic
/// (`RefUnwindSafe`), as every other bundle may.
pub trait Source: Send + Sync + RefUnwindSafe {
    /// Fills `into` with the pack's bytes from `at`, from its start.
    ///
    /// # Errors
    ///
    /// [`SourceError::NotYet`] when those bytes haven't arrived, and
    /// [`SourceError::Failed`] when they can't be read.
    fn read(&self, at: u64, into: &mut [u8]) -> Result<(), SourceError>;

    /// Whether the `len` bytes of the pack from `at` can be read now: what
    /// [`Bundle::arrived`](crate::bundle::Bundle::arrived) asks of a folder's
    /// span ([`Index::spans`]). A source that has every byte at once (a
    /// file, bytes in memory) keeps this default, `true`; one whose bytes
    /// arrive in time (the web's stream) says whether they have.
    fn arrived(&self, at: u64, len: u64) -> bool {
        let _ = (at, len);
        true
    }
}

/// Why a [`Source`] gave no bytes.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SourceError {
    /// The bytes haven't arrived yet: a file of them reads as missing for
    /// now, and nothing is wrong.
    #[error("the bytes haven't arrived yet")]
    NotYet,
    /// The bytes can't be read, for the reason given.
    #[error("{0}")]
    Failed(String),
}

/// A [`Source`] over bytes the program holds for its whole run: a pack
/// embedded with `include_bytes!`, or a test's.
#[derive(Debug, Clone, Copy)]
pub struct InMemory(&'static [u8]);

impl InMemory {
    /// The source of the pack `bytes`.
    #[must_use]
    pub const fn new(bytes: &'static [u8]) -> Self {
        Self(bytes)
    }
}

impl Source for InMemory {
    fn read(&self, at: u64, into: &mut [u8]) -> Result<(), SourceError> {
        let bytes = usize::try_from(at)
            .ok()
            .and_then(|start| self.0.get(start..start.checked_add(into.len())?))
            .ok_or_else(|| SourceError::Failed("past the end of the pack".to_owned()))?;
        into.copy_from_slice(bytes);
        Ok(())
    }
}

/// Why a file of a layer read file by file read as missing, though its
/// pack has it.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FileError {
    /// Its source failed ([`SourceError::Failed`]).
    #[error("its source failed: {0}")]
    Source(String),
    /// Its bytes are refused: changed since the bake (its checksum differs)
    /// or broken.
    #[error(transparent)]
    Pack(PackError),
}

/// A file a layer read file by file couldn't read ([`SourceFiles::problems`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{path}: {error}")]
#[non_exhaustive]
pub struct Problem {
    /// The file's path in the bundle.
    pub path: &'static str,
    /// What went wrong, the first time.
    pub error: FileError,
}

/// One file of a [`SourceFiles`]: 32 bytes, as a table's `(path, bytes)`
/// is, beside its path's bytes (ADR-0128 §1: about 52 bytes a file).
struct SourceFile {
    /// Where its path starts in [`SourceFiles::paths`]; it ends where the
    /// next file's starts.
    path_at: u32,
    /// Where its bytes start in the pack, from its start (`listing` refuses
    /// a pack whose files reach past 4 GiB).
    at: u32,
    stored: u32,
    size: u32,
    sum: u64,
    codec: Codec,
    baked: bool,
    /// Whether a read of it has failed, and gone into the problems.
    failed: AtomicBool,
}

// The record stays the size of a table's `(path, bytes)`, on every
// target (ADR-0128 §1).
const _: () = assert!(size_of::<SourceFile>() == 32);

/// A problem, and the next one: a list that only grows, with no lock.
struct Failure {
    problem: Problem,
    next: OnceBox<Failure>,
}

/// A folder a pack lays first, and where its files lie in the pack, from its
/// start ([`Index::spans`]).
struct SourceSpan {
    dir: Box<str>,
    at: u64,
    length: u64,
}

/// The files of a pack read file by file ([`Index::into_layer`]): its
/// paths, leaked once, and where each file lies, kept for the life of the
/// program; every file's bytes are read from the source when asked for,
/// and handed out owned.
pub struct SourceFiles {
    /// Its files, sorted by path.
    files: Box<[SourceFile]>,
    /// Every file's path, one after the other.
    paths: &'static str,
    /// The folders laid first.
    spans: Box<[SourceSpan]>,
    source: &'static dyn Source,
    /// The files that couldn't be read, in the order they failed.
    failures: OnceBox<Failure>,
}

impl core::fmt::Debug for SourceFiles {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SourceFiles")
            .field("files", &self.files.len())
            .finish_non_exhaustive()
    }
}

impl SourceFiles {
    /// The path of the file at `at`.
    fn path(&self, at: usize) -> &'static str {
        let start = self.files.get(at).map_or(0, |file| file.path_at as usize);
        let end = self
            .files
            .get(at + 1)
            .map_or(self.paths.len(), |file| file.path_at as usize);
        self.paths.get(start..end).unwrap_or_default()
    }

    /// The place of the file at `path`. A file's path ends where the next
    /// one's starts, so the search takes each file's place from where it
    /// lies in the list.
    fn find(&self, path: &str) -> Option<usize> {
        let first = self.files.as_ptr().addr();
        let place = |file: &SourceFile| {
            (core::ptr::from_ref(file).addr() - first) / size_of::<SourceFile>()
        };
        self.files
            .binary_search_by(|file| self.path(place(file)).cmp(path))
            .ok()
    }

    /// Whether the pack has a file at `path`: read from the index alone.
    pub(crate) fn has(&self, path: &str) -> bool {
        self.find(path).is_some()
    }

    /// The paths of the files, sorted: read from the index alone.
    pub(crate) fn paths(&self) -> impl Iterator<Item = &'static str> + '_ {
        (0..self.files.len()).map(|at| self.path(at))
    }

    /// Whether every file under folder `dir` can be read from the source
    /// now: its span, if the pack lays it first, else each file under it.
    /// A file that can't be read for another reason (its read failed, or its
    /// bytes are refused) has arrived all the same.
    pub(crate) fn arrived(&self, dir: &str) -> bool {
        let dir = folder(dir);
        if let Some(span) = self.spans.iter().find(|span| &*span.dir == dir) {
            return self.source.arrived(span.at, span.length);
        }
        (0..self.files.len())
            .filter(|&at| under(self.path(at), dir))
            .all(|at| {
                let file = &self.files[at];
                self.source
                    .arrived(u64::from(file.at), u64::from(file.stored))
            })
    }

    /// The bytes of the file at `path`, read from the source, checked and
    /// unpacked: [`Unread`] when it can't be read now (its bytes haven't
    /// arrived, or the read or the check failed, which
    /// [`problems`](Self::problems) then lists), `None` when the pack has
    /// no such file.
    pub(crate) fn read(&self, path: &str) -> Option<Result<Cow<'static, [u8]>, Unread>> {
        let at = self.find(path)?;
        let file = &self.files[at];
        Some(self.fetch(file, path).map(Cow::Owned).map_err(|error| {
            // Only a file's first failure goes in.
            if let Some(error) = error
                && !file.failed.swap(true, AtomicOrdering::Relaxed)
            {
                let path = self.path(at);
                self.fail(Problem { path, error });
            }
            Unread
        }))
    }

    /// The bytes of `file`, at `path`, or what went wrong: `None` when they
    /// only haven't arrived.
    fn fetch(&self, file: &SourceFile, path: &str) -> Result<Vec<u8>, Option<FileError>> {
        let mut stored = vec![0; file.stored as usize];
        self.source
            .read(u64::from(file.at), &mut stored)
            .map_err(|error| match error {
                SourceError::Failed(why) => Some(FileError::Source(why)),
                SourceError::NotYet => None,
            })?;
        let kept = Kept {
            codec: file.codec,
            offset: u64::from(file.at),
            stored: u64::from(file.stored),
            size: u64::from(file.size),
            sum: file.sum,
            baked: file.baked,
        };
        checked(path, kept, &stored)
            .map(Cow::into_owned)
            .map_err(|error| Some(FileError::Pack(error)))
    }

    /// Adds `problem` to the end of the list.
    fn fail(&self, problem: Problem) {
        let mut slot = &self.failures;
        let mut node = Box::new(Failure {
            problem,
            next: OnceBox::new(),
        });
        while let Err(back) = slot.set(node) {
            node = back;
            let Some(taken) = slot.get() else { return };
            slot = &taken.next;
        }
    }

    /// Every file that couldn't be read, once each, with what went wrong
    /// the first time, in the order they failed: for the backend to log. A
    /// file whose bytes only haven't arrived isn't one.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        let mut next = self.failures.get();
        while let Some(failure) = next {
            problems.push(failure.problem.clone());
            next = failure.next.get();
        }
        problems
    }
}

impl Index {
    /// A bundle layer of the pack's files, shown as `name`, that reads each
    /// file from `source` when the bundle asks for it: it reads the bytes
    /// the pack keeps of it, checks them against its checksum, unpacks them,
    /// and hands them out owned (`Cow::Owned`), keeping nothing. Its paths
    /// are leaked once, as [`Pack::into_layer`](super::Pack::into_layer)'s
    /// are, and `has`, the listings, `left_out` and `display_path` read the
    /// index alone. It stands for the bundle the pack was baked from
    /// ([`Layer::standing_for`]).
    ///
    /// A file whose bytes haven't arrived ([`SourceError::NotYet`]) reads as
    /// missing for now. One whose read failed, or whose bytes are refused,
    /// reads as missing too (the bundle's `watch`er hears of it as of any
    /// missing file), and [`Layer::problems`] lists it once, for the backend
    /// to log.
    ///
    /// A game over this layer keeps its one-bit pictures as values of its
    /// own (ADR-0128 §3): the one-bit frame reads a picture where the
    /// bundle lends it, and this layer lends nothing, so its files are no
    /// pictures there.
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use shuyi_assets::bundle::Bundle;
    /// use shuyi_assets::pack::{self, Content, InMemory, Index, PackInfo};
    /// use shuyi_core::ContentHash;
    ///
    /// let info = PackInfo::new("my-game 1", None, ContentHash(1));
    /// let files = BTreeMap::from([("a.txt".to_owned(), Content::Raw(b"a".to_vec()))]);
    /// let pack: &'static [u8] = pack::write(&info, &files).leak();
    /// let index = Index::read(pack, "my-game 1").unwrap();
    /// let source: &'static InMemory = Box::leak(Box::new(InMemory::new(pack)));
    /// let bundle = Bundle::new(vec![index.into_layer("pack", source)]);
    /// assert_eq!(bundle.file("a.txt").as_deref(), Some("a"));
    /// ```
    pub fn into_layer(self, name: &'static str, source: &'static dyn Source) -> Layer {
        let content = self.listing.info.content;
        let spans = self
            .spans()
            .map(|(dir, range)| SourceSpan {
                dir: dir.into(),
                at: range.start,
                length: range.end - range.start,
            })
            .collect();
        let entries = self.listing.entries;
        let mut paths = String::with_capacity(entries.iter().map(|e| e.path.len()).sum());
        // `listing` checked every file's bytes end below 4 GiB from the
        // pack's start and every size is below 1 GiB, so each fits a `u32`;
        // so do the paths, read from an index of no more than 4 GiB.
        let narrow = |n: u64| u32::try_from(n).unwrap_or(u32::MAX);
        let files = entries
            .iter()
            .map(|entry| {
                let path_at = narrow(paths.len() as u64);
                paths.push_str(&entry.path);
                SourceFile {
                    path_at,
                    at: narrow(self.data_at.saturating_add(entry.offset)),
                    stored: narrow(entry.stored),
                    size: narrow(entry.size),
                    sum: entry.sum,
                    codec: entry.codec,
                    baked: !entry.kinds.is_empty(),
                    failed: AtomicBool::new(false),
                }
            })
            .collect();
        let files: &'static SourceFiles = Box::leak(Box::new(SourceFiles {
            files,
            paths: paths.leak(),
            spans,
            source,
            failures: OnceBox::new(),
        }));
        Layer::of(name, Files::Source(files)).standing_for(content)
    }
}
