//! The pack (`.shuyipak`, ADR-0007 §4, ADR-0023): the optimized format a
//! game may ship in place of its loose files. `shuyi bake` turns the
//! sources into one file, and the game lays it in its bundle like any
//! other layer ([`Pack::into_layer`]), under its loose files or alone.
//!
//! A pack holds every file of the bundle it was baked from, each at its
//! own path:
//!
//! - **a baked file** holds the assets read from its source, already in
//!   their binary form (postcard, one section per kind it was read as),
//!   so loading it parses no text. [`Packed`] is the format that reads
//!   one: a game registers it for every kind it bakes
//!   ([`Bake::register`]);
//! - **every other file** (a picture, a sound) holds its own bytes.
//!
//! The file is a header (magic bytes, the [`VERSION`], a checksum of the
//! header and the index), an index (the stamp, the target, the fingerprint
//! of its sources, and each file's path, kinds, offset, sizes and checksum)
//! and each file's bytes, compressed with LZ4 when that makes them smaller.
//!
//! A pack is read whole ([`Pack::open`], which checks every file) or file by
//! file: [`Index::read`] reads the header and the index alone, and
//! [`Index::into_layer`] makes a layer that reads, checks and unpacks one
//! file each time the bundle asks for it, from a [`Source`] a backend gives
//! (ADR-0128 §2).
//!
//! The index lists the files in path order, and [`write()`] lays their bytes
//! in that order with nothing between them. A game may name folders to lay
//! first ([`Bake::first`], [`write_first`], ADR-0128 §3): their files go at
//! the front of the pack, in the order named, and the index holds each
//! one's span ([`Index::spans`]), so a pack streamed in that order can be
//! read from once a folder has arrived ([`Source::arrived`]). A reader
//! takes the files in any order and with bytes between them, so long as no
//! two overlap, the last ends the pack and each span holds its folder: a
//! closed build's copy is laid out by its recipient's code
//! ([`Raw::write_marked`], ADR-0118), within each span, and [`Raw::mark`]
//! reads the code back. What the game reads is the same either way, and no
//! checksum covers the bytes between files: one changed changes nothing the
//! game reads.
//!
//! A pack stands for its sources: its layer ([`Pack::into_layer`]) gives a
//! bundle the fingerprint of the bundle it was baked from
//! ([`Bundle::fingerprint`](crate::bundle::Bundle::fingerprint)), so a
//! record made on the loose files plays on the pack as made with the same
//! content.
//!
//! The rules:
//!
//! - **Text stays the source.** A pack is derived by [`Bake::bake`], and
//!   [`Bake::check`] proves every asset read from it equal to the one
//!   read from its source.
//! - **No migration.** A pack of another [`VERSION`], or baked for
//!   another stamp (the game's word for the shapes of its own kinds), is
//!   refused with a message to bake it again.
//! - **Never a panic.** A pack cut short or changed is refused: the
//!   header's checksum covers the header and the index, each file's its
//!   own bytes, and every size is checked before anything is allocated for
//!   it. A file changed is refused when it is read, and the others still
//!   read.
//!
//! The reader builds without `std`: a Playdate loads only packs
//! (ADR-0008 §5).
use crate::prelude::*;

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use core::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use shuyi_core::ContentHash;

use crate::bundle::Layer;
use crate::error::ContentError;
use crate::fnv::Hasher;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format};
use crate::recipient::{CODE_BYTES, Code, Stream};

mod bake;
mod source;

pub use bake::{Bake, Bakeable, Baked, BakedFile};
pub use source::{FileError, InMemory, Problem, Source, SourceError, SourceFiles};

/// A pack's file extension, without the dot.
pub const EXTENSION: &str = "shuyipak";

/// The bytes a pack starts with.
pub const MAGIC: [u8; 8] = *b"SHUYIPAK";

/// The version of the pack format this engine reads and writes: the
/// header, the index, and the binary form of the engine's own kinds. A
/// change to any of them makes it the next number, and a pack of another
/// version is refused, never migrated. Version 15 reads files' bytes in any
/// order and with bytes between them (a recipient's layout, ADR-0118);
/// version 16 holds a font atlas's ink (ticket 3215, ADR-0124), version
/// 17 its bold face (ticket 3216), version 18 checks the header and the
/// index apart from each file, which has a checksum of its own (ticket
/// 2263, ADR-0128 §2), version 19's index holds the span of each
/// folder laid first (ticket 2265, ADR-0128 §3), version 20's music cue
/// holds a loop region (ticket 2397), version 21's sound cue holds its
/// rules (ticket 2345, ADR-0139), version 22's target profile its
/// graphics tier (ticket 3347), and version 23's sound cue and `Sound` kind
/// their files' lengths (ticket 2280).
pub const VERSION: u32 = 23;

/// The bytes a baked file in a pack starts with.
pub const BAKED_MAGIC: [u8; 8] = *b"SHUYIBKD";

/// The most files a pack holds.
pub const MAX_FILES: usize = 1 << 16;

/// The most bytes a pack's files add up to, unpacked: 1 GiB.
pub const MAX_UNPACKED_BYTES: u64 = 1 << 30;

/// The most folders a pack lays first ([`write_first`]).
pub const MAX_FIRST: usize = 256;

/// The most bytes a baked file has: 64 MiB, as the engine's biggest
/// sources (a PNG, a sound).
pub const MAX_BAKED_BYTES: usize = 64 << 20;

/// How many bytes LZ4 can make of one, at most: a file claiming more
/// than this many times its stored bytes is broken.
const MAX_RATIO: u64 = 255;

/// The header: the magic bytes, the version (`u32`), the checksum of the
/// header's other bytes and the index (`u64`) and the index's length
/// (`u32`).
const HEADER: usize = 24;

/// Where the header's checksum lies in it.
const SUM_AT: core::ops::Range<usize> = 12..20;

/// What a pack says of itself (ADR-0039 kind 5): build it with
/// [`PackInfo::new`]. A field it gains moves [`VERSION`], since a baked
/// pack's index is not self-describing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct PackInfo {
    /// The game's word for the shapes of its own kinds, the ones a
    /// [`VERSION`] doesn't cover (its version, say). A game opens only a
    /// pack baked with its own stamp.
    pub stamp: String,
    /// The target the pack was baked for (`shuyi bake --target`), if any.
    pub target: Option<String>,
    /// The fingerprint of the bundle it was baked from
    /// ([`Bundle::fingerprint`](crate::bundle::Bundle::fingerprint)),
    /// which its layer stands for.
    pub content: ContentHash,
}

impl PackInfo {
    /// A pack baked with the game's `stamp`, for `target` if any, from the
    /// bundle whose fingerprint is `content`.
    #[must_use]
    pub fn new(stamp: impl Into<String>, target: Option<String>, content: ContentHash) -> Self {
        Self {
            stamp: stamp.into(),
            target,
            content,
        }
    }
}

/// How a file's bytes are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Codec {
    /// As they are.
    Stored,
    /// Compressed with LZ4 (its block format).
    Lz4,
}

/// One file in the index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    /// Its path in the bundle.
    path: String,
    /// The kinds it was baked as: none for a file kept as its own bytes.
    kinds: Vec<String>,
    /// How its bytes are kept.
    codec: Codec,
    /// Where its bytes start, after the index.
    offset: u64,
    /// How many bytes are kept.
    stored: u64,
    /// How many bytes it has, unpacked.
    size: u64,
    /// The [`checksum`] of its kept bytes.
    sum: u64,
}

impl Entry {
    /// How the file is kept, without its path.
    fn kept(&self) -> Kept {
        Kept {
            codec: self.codec,
            offset: self.offset,
            stored: self.stored,
            size: self.size,
            sum: self.sum,
            baked: !self.kinds.is_empty(),
        }
    }
}

/// How one file is kept, once its index has checked ([`check_listing`]):
/// what reading it needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Kept {
    codec: Codec,
    /// Where its bytes start, after the index.
    offset: u64,
    stored: u64,
    size: u64,
    sum: u64,
    /// Whether it holds baked assets.
    baked: bool,
}

/// A folder laid first ([`write_first`]), and the bytes its files take.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Span {
    /// The folder, with no `/` at its end (`""` for the whole pack).
    dir: String,
    /// Where the bytes start, after the index.
    offset: u64,
    /// How many bytes they are: from the first byte of its files to the
    /// last, whatever lies between.
    length: u64,
}

/// The index as the file holds it: what the pack says of itself, its files
/// in path order, and the folders laid first, in the order named.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Listing {
    info: PackInfo,
    entries: Vec<Entry>,
    spans: Vec<Span>,
}

/// Whether the file at `path` is under folder `dir` (with no `/` at its
/// end): inside it or a folder below it. Every file is under `""`.
pub(crate) fn under(path: &str, dir: &str) -> bool {
    dir.is_empty()
        || path
            .strip_prefix(dir)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Whether the file at `path` is in the group `dir` names (with no `/` at
/// its end) when it is laid first ([`Bake::first`]): under the folder, or
/// the file at that path itself, a group of one.
pub(crate) fn in_group(path: &str, dir: &str) -> bool {
    path == dir || under(path, dir)
}

/// Folder `dir` as a span names it: with no `/` at its end.
pub(crate) fn folder(dir: &str) -> &str {
    dir.trim_end_matches('/')
}

/// The folders of `first`, as spans name them, each once, in the order
/// first named, and no more than [`MAX_FIRST`].
fn folders<S: AsRef<str>>(first: &[S]) -> Vec<String> {
    let mut dirs: Vec<String> = Vec::new();
    for dir in first {
        let dir = folder(dir.as_ref());
        if dirs.len() < MAX_FIRST && !dirs.iter().any(|seen| seen == dir) {
            dirs.push(dir.to_owned());
        }
    }
    dirs
}

/// The order a pack of the files at `paths` (in path order) lays them in
/// with no code, each a place in `paths`, and how many files each group of
/// it holds: the files under each folder of `first`, in its order (a file
/// under two goes with the first), then every other, each group in path
/// order.
fn grouped(paths: &[&str], first: &[String]) -> (Vec<usize>, Vec<usize>) {
    let group_of = |path: &str| {
        first
            .iter()
            .position(|dir| in_group(path, dir))
            .unwrap_or(first.len())
    };
    let groups: Vec<usize> = paths.iter().map(|path| group_of(path)).collect();
    let mut order: Vec<usize> = (0..paths.len()).collect();
    // A stable sort: each group keeps its files in path order.
    order.sort_by_key(|&at| groups[at]);
    let mut sizes = vec![0; first.len() + 1];
    for group in groups {
        sizes[group] += 1;
    }
    (order, sizes)
}

/// A pack's header and index, read and checked alone ([`Index::read`]):
/// what a layer that reads the pack file by file holds
/// ([`Index::into_layer`]), and the check of each file's bytes
/// ([`Index::check_file`]).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Index {
    listing: Listing,
    /// Where the files' bytes start in the pack: the header's and the
    /// index's length.
    data_at: u64,
}

/// Why [`Index::read`] read no index.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum IndexError {
    /// The bytes given end before the index does: the pack's header and
    /// index take `needs` bytes (the header's own, when fewer than it were
    /// given). Read that many from the pack's start, and read again.
    #[error("the pack's header and index take {needs} bytes")]
    Short {
        /// How many bytes the header and the index take.
        needs: u64,
    },
    /// The pack is refused.
    #[error(transparent)]
    Pack(#[from] PackError),
}

/// What goes into a pack at one path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Content {
    /// The file's own bytes.
    Raw(Vec<u8>),
    /// The assets read from the file: each kind's name and its binary form
    /// (postcard).
    Baked(BTreeMap<&'static str, Vec<u8>>),
}

/// Why a pack was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PackError {
    /// It doesn't start with [`MAGIC`].
    #[error("not a Shuyi pack: a .shuyipak file starts with \"SHUYIPAK\"")]
    NotAPack,
    /// It is of another [`VERSION`].
    #[error(
        "the pack is of pack format version {found} and this engine reads version {VERSION}: \
         bake it again with `shuyi bake`"
    )]
    Version {
        /// Its version.
        found: u32,
    },
    /// It was baked for another stamp.
    #[error(
        "the pack was baked for \"{found}\" and this game is \"{expected}\": bake it again with \
         `shuyi bake`"
    )]
    Stamp {
        /// Its stamp.
        found: String,
        /// The game's.
        expected: String,
    },
    /// It is shorter than its header or its index says.
    #[error("the pack is cut short: bake it again with `shuyi bake`")]
    Truncated,
    /// A byte after the header isn't the one baked.
    #[error("the pack was changed after it was baked (its checksum differs): bake it again")]
    Checksum,
    /// Its index or one of its files doesn't hold together.
    #[error("the pack is broken: {0}")]
    Broken(String),
}

/// A pack's files, read and unpacked: what a game lays in its bundle. A
/// file the pack keeps as it is borrows the pack's bytes (`'a`); only a
/// compressed one is unpacked into memory of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack<'a> {
    info: PackInfo,
    files: Vec<PackFile<'a>>,
}

/// One file of a [`Pack`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackFile<'a> {
    /// Its path in the bundle.
    pub path: String,
    /// The kinds it was baked as: none for a file kept as its own bytes.
    pub kinds: Vec<String>,
    /// How many bytes the pack keeps of it.
    pub stored: usize,
    /// Its bytes, unpacked: a baked file's ([`Packed`] reads them), or the
    /// file's own.
    pub bytes: Cow<'a, [u8]>,
}

/// The checksum of a pack: the word-wise FNV-1a hash (`crate::fnv`) of
/// its bytes, then their length.
fn checksum(bytes: &[u8]) -> u64 {
    let mut hash = Hasher::new();
    hash.bytes(bytes);
    hash.word(bytes.len() as u64);
    hash.0
}

/// The `N` bytes at `at` in `bytes`, or [`PackError::Truncated`].
fn take<const N: usize>(bytes: &[u8], at: usize) -> Result<[u8; N], PackError> {
    bytes
        .get(at..at + N)
        .and_then(|b| b.try_into().ok())
        .ok_or(PackError::Truncated)
}

/// A broken pack, for `why`.
fn broken(why: impl Into<String>) -> PackError {
    PackError::Broken(why.into())
}

/// The bytes of a baked file of `sections` (kind name → binary form).
fn baked_file(sections: &BTreeMap<&'static str, Vec<u8>>) -> Vec<u8> {
    let list: Vec<(&str, &[u8])> = sections
        .iter()
        .map(|(kind, data)| (*kind, data.as_slice()))
        .collect();
    let mut bytes = BAKED_MAGIC.to_vec();
    // A list of strings and byte strings always serializes.
    bytes.extend(postcard::to_allocvec(&list).unwrap_or_default());
    bytes
}

/// The sections of a baked file (kind name → binary form), if `bytes` is
/// one whole.
fn sections(bytes: &[u8]) -> Option<Vec<(&str, &[u8])>> {
    let rest = bytes.strip_prefix(&BAKED_MAGIC)?;
    let (list, left) = postcard::take_from_bytes::<Vec<(&str, &[u8])>>(rest).ok()?;
    left.is_empty().then_some(list)
}

/// A pack of `files` (path → content), described by `info`. Each file's
/// bytes are compressed when that makes them smaller.
pub fn write(info: &PackInfo, files: &BTreeMap<String, Content>) -> Vec<u8> {
    write_first::<&str>(info, &[], files)
}

/// A pack of `files` (path → content), described by `info`, the files
/// under each folder of `first` laid at its front, in the order of `first`
/// (a file under two goes with the first), then every other file in path
/// order. `a/` is the same folder as `a`, a folder named twice is laid
/// where first named, and past [`MAX_FIRST`] folders the rest are laid in
/// path order. The index holds each folder's span ([`Index::spans`]):
/// [`Bake::first`] names them.
///
/// ```
/// use std::collections::BTreeMap;
/// use shuyi_assets::pack::{self, Content, Index, PackInfo};
/// use shuyi_core::ContentHash;
///
/// let info = PackInfo::new("my-game 1", None, ContentHash(1));
/// let file = |path: &str| (path.to_owned(), Content::Raw(path.as_bytes().to_vec()));
/// let files = BTreeMap::from([file("a.txt"), file("title/logo.txt")]);
/// let bytes = pack::write_first(&info, &["title/"], &files);
/// let index = Index::read(&bytes, "my-game 1").unwrap();
/// let title = index.file_range("title/logo.txt").unwrap();
/// assert!(title.end <= index.file_range("a.txt").unwrap().start);
/// assert_eq!(index.spans().collect::<Vec<_>>(), [("title", title)]);
/// ```
pub fn write_first<S: AsRef<str>>(
    info: &PackInfo,
    first: &[S],
    files: &BTreeMap<String, Content>,
) -> Vec<u8> {
    let files: Vec<RawFile> = files
        .iter()
        .map(|(path, content)| match content {
            Content::Raw(bytes) => RawFile::kept(path.clone(), Vec::new(), Cow::Borrowed(bytes)),
            Content::Baked(sections) => RawFile::kept(
                path.clone(),
                sections.keys().map(|&kind| kind.to_owned()).collect(),
                Cow::Owned(baked_file(sections)),
            ),
        })
        .collect();
    let first = folders(first);
    let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
    let (order, _) = grouped(&paths, &first);
    assemble(info, &files, &first, &order, &[])
}

/// The span of each folder of `first` once `files` lie at `offsets`: from
/// the first byte of its files to the last (none for a folder with no file).
fn spans_of(files: &[RawFile<'_>], offsets: &[u64], first: &[String]) -> Vec<Span> {
    first
        .iter()
        .map(|dir| {
            let (mut start, mut end) = (u64::MAX, 0);
            for (file, &offset) in files.iter().zip(offsets) {
                if in_group(&file.path, dir) {
                    start = start.min(offset);
                    end = end.max(offset + file.kept.len() as u64);
                }
            }
            let start = start.min(end);
            Span {
                dir: dir.clone(),
                offset: start,
                length: end - start,
            }
        })
        .collect()
}

/// The pack of `info` and `files` (in path order), their bytes laid in the
/// order `order` gives (each a place in `files`), each after the bytes of
/// the same place in `gaps`, with the span of each folder of `first`.
fn assemble(
    info: &PackInfo,
    files: &[RawFile<'_>],
    first: &[String],
    order: &[usize],
    gaps: &[Vec<u8>],
) -> Vec<u8> {
    let mut offsets = vec![0u64; files.len()];
    let mut data = Vec::new();
    for (place, &at) in order.iter().enumerate() {
        if let Some(gap) = gaps.get(place) {
            data.extend_from_slice(gap);
        }
        offsets[at] = data.len() as u64;
        data.extend_from_slice(&files[at].kept);
    }
    let spans = spans_of(files, &offsets, first);
    let entries = files
        .iter()
        .zip(offsets)
        .map(|(file, offset)| Entry {
            path: file.path.clone(),
            kinds: file.kinds.clone(),
            codec: file.codec,
            offset,
            stored: file.kept.len() as u64,
            size: file.size,
            sum: checksum(&file.kept),
        })
        .collect();
    let listing = Listing {
        info: info.clone(),
        entries,
        spans,
    };
    // Strings, numbers and lists always serialize.
    let index = postcard::to_allocvec(&listing).unwrap_or_default();
    let mut out = Vec::with_capacity(HEADER + index.len() + data.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    // An index over 4 GiB would hold more than `MAX_FILES` files.
    out.extend_from_slice(&u32::try_from(index.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&index);
    let sum = header_sum(&out);
    out[SUM_AT].copy_from_slice(&sum.to_le_bytes());
    out.extend_from_slice(&data);
    out
}

/// The checksum of a pack's header and index, `head` (at least the
/// header), but for the bytes the checksum itself takes.
fn header_sum(head: &[u8]) -> u64 {
    let mut summed = Vec::with_capacity(head.len());
    summed.extend_from_slice(&head[..SUM_AT.start]);
    summed.extend_from_slice(&head[SUM_AT.end..]);
    checksum(&summed)
}

/// The index of the pack at the start of `bytes`, checked, and where its
/// files' bytes start: refused when its magic bytes, version or checksum
/// are wrong, or [`IndexError::Short`] when `bytes` end before the index
/// does.
fn listing(bytes: &[u8]) -> Result<(Listing, usize), IndexError> {
    let seen = &bytes[..bytes.len().min(MAGIC.len())];
    if !MAGIC.starts_with(seen) {
        return Err(PackError::NotAPack.into());
    }
    let short = |needs: usize| IndexError::Short {
        needs: needs as u64,
    };
    if bytes.len() < HEADER {
        if bytes.len() >= 12 {
            check_version(bytes)?;
        }
        return Err(short(HEADER));
    }
    check_version(bytes)?;
    let length = u32::from_le_bytes(take(bytes, 20)?);
    let needs = usize::try_from(length)
        .ok()
        .and_then(|length| HEADER.checked_add(length))
        .ok_or(PackError::Truncated)?;
    let head = bytes.get(..needs).ok_or_else(|| short(needs))?;
    if header_sum(head) != u64::from_le_bytes(take(bytes, SUM_AT.start)?) {
        return Err(PackError::Checksum.into());
    }
    let listing: Listing =
        postcard::from_bytes(&head[HEADER..]).map_err(|_| broken("its index doesn't read"))?;
    check_listing(&listing)?;
    let ends = listing.entries.iter().map(|entry| {
        (needs as u64)
            .saturating_add(entry.offset)
            .saturating_add(entry.stored)
    });
    if ends.max().unwrap_or(0) > u64::from(u32::MAX) {
        return Err(broken("its files' bytes reach past 4 GiB").into());
    }
    Ok((listing, needs))
}

/// Refuses a pack whose version, at bytes 8 to 12, isn't [`VERSION`].
fn check_version(bytes: &[u8]) -> Result<(), PackError> {
    match u32::from_le_bytes(take(bytes, 8)?) {
        VERSION => Ok(()),
        found => Err(PackError::Version { found }),
    }
}

/// Whether `listing` holds together, whatever follows it: no more than
/// [`MAX_FILES`] files, in path order, each of a size it can have and none
/// overlapping another, adding up to no more than [`MAX_UNPACKED_BYTES`].
fn check_listing(listing: &Listing) -> Result<(), PackError> {
    let entries = &listing.entries;
    if entries.len() > MAX_FILES {
        return Err(broken(format!("it has more than {MAX_FILES} files")));
    }
    let mut unpacked: u64 = 0;
    let mut spans = Vec::with_capacity(entries.len());
    for (at, entry) in entries.iter().enumerate() {
        let path = &entry.path;
        if at > 0 && entries[at - 1].path >= *path {
            return Err(broken(format!("{path} is out of path order")));
        }
        unpacked = unpacked.saturating_add(entry.size);
        if unpacked > MAX_UNPACKED_BYTES {
            return Err(broken(format!(
                "its files add up to more than {MAX_UNPACKED_BYTES} bytes"
            )));
        }
        match entry.codec {
            Codec::Stored if entry.stored != entry.size => {
                return Err(broken(format!("{path} is stored with two sizes")));
            }
            Codec::Lz4 if entry.stored >= entry.size => {
                return Err(broken(format!(
                    "{path} is compressed to no fewer bytes than it has"
                )));
            }
            Codec::Lz4 if entry.size > entry.stored.saturating_mul(MAX_RATIO) => {
                return Err(broken(format!(
                    "{path} claims {} bytes from {} stored",
                    entry.size, entry.stored
                )));
            }
            _ => {}
        }
        let end = entry
            .offset
            .checked_add(entry.stored)
            .ok_or_else(|| broken(format!("{path} goes past the end of the pack")))?;
        spans.push((entry.offset, end, at));
    }
    spans.sort_unstable();
    for pair in spans.windows(2) {
        let ((_, first_end, first), (second_start, _, second)) = (pair[0], pair[1]);
        if second_start < first_end {
            return Err(broken(format!(
                "{} overlaps {}",
                entries[second].path, entries[first].path
            )));
        }
    }
    let data_end = spans.iter().map(|&(_, end, _)| end).max().unwrap_or(0);
    check_spans(listing, data_end)
}

/// Whether the spans of `listing` hold together, its files' bytes ending at
/// `data_end`: no more than [`MAX_FIRST`], each folder once and with no `/`
/// at its end, each ending by `data_end` and holding every file under its
/// folder, or the file it names.
fn check_spans(listing: &Listing, data_end: u64) -> Result<(), PackError> {
    let spans = &listing.spans;
    if spans.len() > MAX_FIRST {
        return Err(broken(format!(
            "it lays more than {MAX_FIRST} folders first"
        )));
    }
    let entries = &listing.entries;
    for (at, span) in spans.iter().enumerate() {
        let dir = &span.dir;
        if folder(dir) != dir || spans[..at].iter().any(|seen| seen.dir == *dir) {
            return Err(broken(format!(
                "its span of {dir}/ is named wrong or twice"
            )));
        }
        let end = span
            .offset
            .checked_add(span.length)
            .filter(|&end| end <= data_end)
            .ok_or_else(|| broken(format!("its span of {dir}/ goes past its files")))?;
        // The files under a folder are together in path order, from the
        // first path at or after `dir/`.
        let start = if dir.is_empty() {
            0
        } else {
            let prefix = format!("{dir}/");
            entries.partition_point(|entry| entry.path < prefix)
        };
        // A span may name a file, laid first alone ([`in_group`]).
        let file = entries
            .binary_search_by(|entry| entry.path.as_str().cmp(dir))
            .ok()
            .map(|at| &entries[at]);
        let outside = entries[start..]
            .iter()
            .take_while(|entry| under(&entry.path, dir))
            .chain(file)
            .find(|entry| entry.offset < span.offset || entry.offset + entry.stored > end);
        if let Some(entry) = outside {
            return Err(broken(format!(
                "{} lies outside its span of {dir}/",
                entry.path
            )));
        }
    }
    Ok(())
}

/// How many bytes the pack at the start of `bytes` takes, by its header and
/// its index: for a tool that looks for a pack inside a bigger file, such as
/// a program that embeds one with `include_bytes!`. The header and the index
/// are checked, the files' bytes not: [`Pack::open`] the bytes it counts.
pub fn length(bytes: &[u8]) -> Result<usize, PackError> {
    let (listing, data_at) = listing(bytes).map_err(IndexError::into_pack)?;
    let data = listing
        .entries
        .iter()
        .map(|entry| entry.offset.saturating_add(entry.stored))
        .max()
        .unwrap_or(0);
    usize::try_from(data)
        .ok()
        .and_then(|data| data_at.checked_add(data))
        .filter(|&total| total <= bytes.len())
        .ok_or(PackError::Truncated)
}

/// The index of pack `bytes` and its files' bytes, once its header, version
/// and index are right.
fn index(bytes: &[u8]) -> Result<(Listing, &[u8]), PackError> {
    let (listing, data_at) = listing(bytes).map_err(IndexError::into_pack)?;
    Ok((listing, &bytes[data_at..]))
}

/// The bytes of the file at `path`, kept as `kept` says in `stored`,
/// checked against its checksum and unpacked: borrowed if it is kept as it
/// is.
fn checked<'a>(path: &str, kept: Kept, stored: &'a [u8]) -> Result<Cow<'a, [u8]>, PackError> {
    if stored.len() as u64 != kept.stored {
        return Err(PackError::Truncated);
    }
    if checksum(stored) != kept.sum {
        return Err(PackError::Checksum);
    }
    let bytes = match kept.codec {
        Codec::Stored => Cow::Borrowed(stored),
        Codec::Lz4 => {
            let size =
                usize::try_from(kept.size).map_err(|_| broken(format!("{path} is too big")))?;
            match lz4_flex::block::decompress(stored, size) {
                Ok(bytes) if bytes.len() == size => Cow::Owned(bytes),
                _ => return Err(broken(format!("{path} doesn't unpack"))),
            }
        }
    };
    if kept.baked && sections(&bytes).is_none() {
        return Err(broken(format!("{path} isn't a whole baked file")));
    }
    Ok(bytes)
}

/// The bytes of `entry` in `data`, checked and unpacked.
fn unpack<'a>(entry: &Entry, data: &'a [u8]) -> Result<Cow<'a, [u8]>, PackError> {
    let range = usize::try_from(entry.offset)
        .ok()
        .zip(usize::try_from(entry.offset.saturating_add(entry.stored)).ok())
        .and_then(|(start, end)| data.get(start..end))
        .ok_or_else(|| broken(format!("{} goes past the end of the pack", entry.path)))?;
    checked(&entry.path, entry.kept(), range)
}

/// What pack `bytes` says of itself, once its header, version and index
/// are right: for a tool that looks at a pack without opening it.
pub fn info(bytes: &[u8]) -> Result<PackInfo, PackError> {
    index(bytes).map(|(listing, _)| listing.info)
}

/// The order `entries` (checked, [`check_listing`]) lay their bytes in
/// `data_len` bytes of data (each a place in `entries`), once none goes past
/// the end and the last ends the data.
fn layout(entries: &[Entry], data_len: usize) -> Result<Vec<usize>, PackError> {
    let mut spans = Vec::with_capacity(entries.len());
    for (at, entry) in entries.iter().enumerate() {
        let span = usize::try_from(entry.offset)
            .ok()
            .zip(usize::try_from(entry.offset.saturating_add(entry.stored)).ok())
            .filter(|&(_, end)| end <= data_len)
            .ok_or_else(|| broken(format!("{} goes past the end of the pack", entry.path)))?;
        spans.push((span, at));
    }
    spans.sort_unstable();
    let end = spans.iter().map(|&((_, end), _)| end).max().unwrap_or(0);
    if end != data_len {
        return Err(broken("bytes follow its last file"));
    }
    Ok(spans.into_iter().map(|(_, at)| at).collect())
}

impl IndexError {
    /// The error of a pack read whole: one cut short before its index ends
    /// is [`PackError::Truncated`].
    fn into_pack(self) -> PackError {
        match self {
            Self::Short { .. } => PackError::Truncated,
            Self::Pack(error) => error,
        }
    }
}

impl Index {
    /// The header and the index of the pack whose first bytes are
    /// `prefix`, checked, if it is of this engine's [`VERSION`] and was
    /// baked with `stamp`. When `prefix` ends before the index does, it is
    /// [`IndexError::Short`], saying how many bytes to read: a backend reads
    /// the header's, then that many.
    ///
    /// ```
    /// use std::collections::BTreeMap;
    /// use shuyi_assets::pack::{self, Content, Index, IndexError, PackInfo};
    /// use shuyi_core::ContentHash;
    ///
    /// let info = PackInfo::new("my-game 1", None, ContentHash(1));
    /// let files = BTreeMap::from([("a.txt".to_owned(), Content::Raw(b"a".to_vec()))]);
    /// let pack = pack::write(&info, &files);
    /// // The header is 24 bytes, and says how long the index is.
    /// let Err(IndexError::Short { needs }) = Index::read(&pack[..24], "my-game 1") else {
    ///     panic!("the header holds no index");
    /// };
    /// let index = Index::read(&pack[..needs as usize], "my-game 1").unwrap();
    /// assert_eq!(index.info(), &info);
    /// ```
    pub fn read(prefix: &[u8], stamp: &str) -> Result<Self, IndexError> {
        let (listing, data_at) = listing(prefix)?;
        if listing.info.stamp != stamp {
            return Err(PackError::Stamp {
                found: listing.info.stamp,
                expected: stamp.to_owned(),
            }
            .into());
        }
        Ok(Self {
            listing,
            data_at: data_at as u64,
        })
    }

    /// What the pack says of itself.
    pub fn info(&self) -> &PackInfo {
        &self.listing.info
    }

    /// The paths of its files, in path order.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.listing.entries.iter().map(|entry| entry.path.as_str())
    }

    /// Where the bytes the pack keeps of the file at `path` lie in the pack,
    /// from its start, if it has the file: what a backend reads to hand
    /// them to [`check_file`](Self::check_file).
    pub fn file_range(&self, path: &str) -> Option<core::ops::Range<u64>> {
        let entry = self.entry(path)?;
        let start = self.data_at.saturating_add(entry.offset);
        Some(start..start.saturating_add(entry.stored))
    }

    /// Each folder laid first ([`write_first`]), in the order named, with
    /// no `/` at its end, and where its files' bytes lie in the pack, from
    /// its start: from the first byte of its files to the last, whatever
    /// lies between (empty for a folder with no file). A page that streams
    /// the pack fetches it in this order, and a folder can be read once its
    /// span has arrived.
    pub fn spans(&self) -> impl Iterator<Item = (&str, core::ops::Range<u64>)> {
        self.listing.spans.iter().map(|span| {
            let start = self.data_at.saturating_add(span.offset);
            (span.dir.as_str(), start..start.saturating_add(span.length))
        })
    }

    /// The bytes of the file at `path`, from `stored`, the bytes the pack
    /// keeps of it ([`file_range`](Self::file_range)): checked against the
    /// file's checksum, then unpacked. A file changed in the pack is refused
    /// here, when it is read, and every other file still reads.
    pub fn check_file(&self, path: &str, stored: &[u8]) -> Result<Vec<u8>, PackError> {
        let entry = self
            .entry(path)
            .ok_or_else(|| broken(format!("it has no file {path}")))?;
        checked(path, entry.kept(), stored).map(Cow::into_owned)
    }

    /// The entry of the file at `path`.
    fn entry(&self, path: &str) -> Option<&Entry> {
        let entries = &self.listing.entries;
        let at = entries
            .binary_search_by(|entry| entry.path.as_str().cmp(path))
            .ok()?;
        entries.get(at)
    }
}

/// One file of a pack, as the pack keeps it: a stored file's bytes are
/// borrowed from the caller's while [`write_first`] assembles the pack, so
/// pictures and sounds are not copied twice.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RawFile<'a> {
    path: String,
    kinds: Vec<String>,
    codec: Codec,
    /// Its bytes unpacked.
    size: u64,
    /// Its bytes as kept.
    kept: Cow<'a, [u8]>,
}

impl<'a> RawFile<'a> {
    /// The file at `path` of `kinds` whose bytes are `bytes`, compressed when
    /// that makes them smaller, and kept as they come (borrowed or owned)
    /// when not.
    fn kept(path: String, kinds: Vec<String>, bytes: Cow<'a, [u8]>) -> Self {
        let packed = lz4_flex::block::compress(&bytes);
        let size = bytes.len() as u64;
        let (codec, kept) = if packed.len() < bytes.len() {
            (Codec::Lz4, Cow::Owned(packed))
        } else {
            (Codec::Stored, bytes)
        };
        Self {
            path,
            kinds,
            codec,
            size,
            kept,
        }
    }
}

/// What a pack's layout holds ([`Raw::mark`]).
///
/// The engine's own, for `shuyi export --recipient` (ADR-0118): not part
/// of the API, which semver doesn't cover (ADR-0119).
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackMark {
    /// No mark: its files are in path order with nothing between them, as
    /// [`write()`] lays them (a public build's pack).
    None,
    /// A recipient's code, held twice between its files, and whether the
    /// order of its files is the one the code gives (a second sign).
    Code {
        /// The code.
        code: Code,
        /// Whether its files are in the order the code gives.
        order_agrees: bool,
    },
    /// Bytes between its files, or an order, that hold no code.
    Broken,
}

/// The bytes a code lays between a pack's files: the code twice.
const GAP_BYTES: usize = 2 * CODE_BYTES;

/// The seed of the stream the bytes between files are kept under.
const GAP_SEED: u64 = 0x5348_5559_4741_5053;

/// The seed a code's order of files is drawn from, with the code's bits.
const ORDER_SEED: u64 = 0x5348_5559_4f52_4452;

/// The order `code` lays `count` files in: each data place's file.
#[cfg(test)]
fn order_of(code: Code, count: usize) -> Vec<usize> {
    order_in(code, &[count])
}

/// The order `code` lays files in groups of `sizes` files in, each group
/// after the one before: each data place's place in the order with no code.
/// Each group's files are shuffled among its own places, from one stream
/// drawn group after group, so a pack with no folder laid first is laid
/// as before there were groups (the order `a_codes_layout_is_pinned` pins).
fn order_in(code: Code, sizes: &[usize]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..sizes.iter().sum()).collect();
    let mut stream = Stream::new(ORDER_SEED ^ u64::from(code.bits()));
    let mut start = 0;
    for &size in sizes {
        for at in (1..size).rev() {
            order.swap(start + at, start + stream.below(at + 1));
        }
        start += size;
    }
    order
}

/// The stream the bytes between `count` files are kept under.
fn gap_key(count: usize) -> [u8; GAP_BYTES] {
    let mut stream = Stream::new(GAP_SEED ^ count as u64);
    let mut key = [0u8; GAP_BYTES];
    for byte in &mut key {
        *byte = stream.next().to_le_bytes()[0];
    }
    key
}

/// The bytes `code` lays between `count` files, before each data place.
fn gaps_of(code: Code, count: usize) -> Vec<Vec<u8>> {
    let bytes = code.to_bytes();
    let key = gap_key(count);
    let all: Vec<u8> = bytes
        .iter()
        .chain(&bytes)
        .zip(key)
        .map(|(byte, key)| byte ^ key)
        .collect();
    let each = GAP_BYTES.div_ceil(count.max(1));
    let mut gaps: Vec<Vec<u8>> = all.chunks(each).map(<[u8]>::to_vec).collect();
    gaps.resize(count, Vec::new());
    gaps
}

/// A pack read as it keeps its files, without its stamp: what a tool that
/// changes a pack after it was baked works on. A closed build's overlay
/// replaces its picture and lays it out by a recipient's code (ADR-0118);
/// the detector reads the code back.
///
/// The engine's own, for `shuyi export --recipient` (ADR-0118): not part
/// of the API, which semver doesn't cover (ADR-0119).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw {
    info: PackInfo,
    /// Its files in path order.
    files: Vec<RawFile<'static>>,
    /// The folders laid first, in the order named, each once.
    first: Vec<String>,
    /// Each data place's file, as read; empty for a pack made here.
    order: Vec<usize>,
    /// The bytes before each data place, as read.
    gaps: Vec<Vec<u8>>,
}

impl Raw {
    /// The pack `bytes`, if it is of this engine's [`VERSION`] and holds
    /// together as [`Pack::open`] says, whatever its stamp.
    pub fn read(bytes: &[u8]) -> Result<Self, PackError> {
        let (index, data) = index(bytes)?;
        let order = layout(&index.entries, data.len())?;
        for entry in &index.entries {
            unpack(entry, data)?;
        }
        let span = |entry: &Entry| {
            // `layout` checked every span is in the data.
            let start = usize::try_from(entry.offset).unwrap_or(0);
            let end = usize::try_from(entry.offset.saturating_add(entry.stored)).unwrap_or(0);
            (start, end)
        };
        let mut gaps = Vec::with_capacity(order.len());
        let mut end = 0;
        for &at in &order {
            let (start, stop) = span(&index.entries[at]);
            gaps.push(data[end..start].to_vec());
            end = stop;
        }
        let files = index
            .entries
            .iter()
            .map(|entry| {
                let (start, end) = span(entry);
                RawFile {
                    path: entry.path.clone(),
                    kinds: entry.kinds.clone(),
                    codec: entry.codec,
                    size: entry.size,
                    kept: Cow::Owned(data[start..end].to_vec()),
                }
            })
            .collect();
        Ok(Self {
            info: index.info,
            files,
            first: index.spans.into_iter().map(|span| span.dir).collect(),
            order,
            gaps,
        })
    }

    /// The order its files are laid in with no code ([`write`](Self::write)),
    /// each a place in its files, and how many each group holds.
    fn grouped(&self) -> (Vec<usize>, Vec<usize>) {
        let paths: Vec<&str> = self.paths().collect();
        grouped(&paths, &self.first)
    }

    /// The order `code` lays its files in: within each group, so each span
    /// still holds its folder.
    fn coded(&self, code: Code) -> Vec<usize> {
        let (base, sizes) = self.grouped();
        order_in(code, &sizes)
            .into_iter()
            .map(|place| base[place])
            .collect()
    }

    /// What the pack says of itself.
    pub fn info(&self) -> &PackInfo {
        &self.info
    }

    /// Its files' paths, in path order.
    pub fn paths(&self) -> impl Iterator<Item = &str> {
        self.files.iter().map(|file| file.path.as_str())
    }

    /// The bytes of the file at `path`, unpacked, if the pack has it and it
    /// is kept as its own bytes (not baked).
    pub fn file(&self, path: &str) -> Option<Vec<u8>> {
        let file = self
            .files
            .iter()
            .find(|file| file.path == path && file.kinds.is_empty())?;
        match file.codec {
            Codec::Stored => Some(file.kept.to_vec()),
            Codec::Lz4 => {
                let size = usize::try_from(file.size).ok()?;
                lz4_flex::block::decompress(&file.kept, size).ok()
            }
        }
    }

    /// Puts `bytes` in place of the file at `path`, kept as its own bytes:
    /// whether the pack has such a file.
    pub fn replace(&mut self, path: &str, bytes: &[u8]) -> bool {
        let Some(file) = self
            .files
            .iter_mut()
            .find(|file| file.path == path && file.kinds.is_empty())
        else {
            return false;
        };
        *file = RawFile::kept(path.to_owned(), Vec::new(), Cow::Owned(bytes.to_vec()));
        true
    }

    /// The pack's bytes with nothing between its files, as [`write()`] and
    /// [`write_first`] lay them: the folders laid first, then every other
    /// file, each group in path order.
    pub fn write(&self) -> Vec<u8> {
        let (order, _) = self.grouped();
        assemble(&self.info, &self.files, &self.first, &order, &[])
    }

    /// The pack's bytes laid out by `code`: its files in the order the code
    /// gives, and the code twice in bytes between the first of them. The
    /// code shuffles each folder laid first among its own places, and every
    /// other file among theirs, so each span still holds its folder. The
    /// index is the same but for its offsets and spans, and the game reads
    /// the same files. None for a pack with no file.
    pub fn write_marked(&self, code: Code) -> Option<Vec<u8>> {
        let count = self.files.len();
        if count == 0 {
            return None;
        }
        let order = self.coded(code);
        let gaps = gaps_of(code, count);
        Some(assemble(
            &self.info,
            &self.files,
            &self.first,
            &order,
            &gaps,
        ))
    }

    /// What its layout holds: no mark, a code, or bytes that hold none.
    pub fn mark(&self) -> PackMark {
        let count = self.files.len();
        let between: Vec<u8> = self.gaps.iter().flatten().copied().collect();
        let in_order = self.order == self.grouped().0;
        if between.is_empty() {
            return if in_order {
                PackMark::None
            } else {
                PackMark::Broken
            };
        }
        if between.len() != GAP_BYTES {
            return PackMark::Broken;
        }
        let meant: Vec<u8> = between
            .iter()
            .zip(gap_key(count))
            .map(|(byte, key)| byte ^ key)
            .collect();
        let (first, second) = meant.split_at(CODE_BYTES);
        let mut bytes = [0u8; CODE_BYTES];
        bytes.copy_from_slice(first);
        match Code::from_bytes(bytes) {
            Some(code) if first == second => PackMark::Code {
                code,
                order_agrees: self.order == self.coded(code),
            },
            _ => PackMark::Broken,
        }
    }
}

impl<'a> Pack<'a> {
    /// The pack `bytes`, every file checked and unpacked, if it is of this
    /// engine's [`VERSION`], was baked with `stamp`, and holds together: for
    /// a pack embedded or read whole.
    pub fn open(bytes: &'a [u8], stamp: &str) -> Result<Self, PackError> {
        let (index, data) = index(bytes)?;
        if index.info.stamp != stamp {
            return Err(PackError::Stamp {
                found: index.info.stamp,
                expected: stamp.to_owned(),
            });
        }
        layout(&index.entries, data.len())?;
        let mut files: Vec<PackFile<'a>> = Vec::with_capacity(index.entries.len());
        for entry in index.entries {
            let bytes = unpack(&entry, data)?;
            files.push(PackFile {
                path: entry.path,
                kinds: entry.kinds,
                stored: usize::try_from(entry.stored).unwrap_or(usize::MAX),
                bytes,
            });
        }
        Ok(Self {
            info: index.info,
            files,
        })
    }

    /// What the pack says of itself.
    pub fn info(&self) -> &PackInfo {
        &self.info
    }

    /// Its files, in path order.
    pub fn files(&self) -> &[PackFile<'a>] {
        &self.files
    }

    /// The bytes of the file at `path`, unpacked, if the pack has it.
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        let at = self
            .files
            .binary_search_by(|file| file.path.as_str().cmp(path))
            .ok()?;
        Some(&self.files[at].bytes)
    }

    /// The pack, every file in memory of its own: for a pack read at run
    /// time, whose bytes the game doesn't keep.
    pub fn into_owned(self) -> Pack<'static> {
        let files = self
            .files
            .into_iter()
            .map(|file| PackFile {
                bytes: Cow::Owned(file.bytes.into_owned()),
                path: file.path,
                kinds: file.kinds,
                stored: file.stored,
            })
            .collect();
        Pack {
            info: self.info,
            files,
        }
    }
}

impl Pack<'static> {
    /// A bundle layer of the pack's files, shown as `name`, standing for
    /// the bundle it was baked from ([`Layer::standing_for`]). A game opens
    /// its pack once, at start-up (most simply one it embeds with
    /// `include_bytes!`): the files it unpacked are kept for the rest of
    /// the run, as the files a game embeds are.
    pub fn into_layer(self, name: &'static str) -> Layer {
        let content = self.info.content;
        let table: Vec<(&'static str, &'static [u8])> = self
            .files
            .into_iter()
            .map(|file| {
                let path: &'static str = Box::leak(file.path.into_boxed_str());
                let bytes: &'static [u8] = match file.bytes {
                    Cow::Borrowed(bytes) => bytes,
                    Cow::Owned(bytes) => Box::leak(bytes.into_boxed_slice()),
                };
                (path, bytes)
            })
            .collect();
        Layer::table(name, Box::leak(table.into_boxed_slice())).standing_for(content)
    }
}

/// The format of a baked file of kind `K` in a pack: it reads the asset
/// baked from the source, with no parsing of text. It claims no
/// extension, and knows a baked file by its magic bytes, so the file keeps
/// its source's path (`data/palette.ron`) and code that asks for the asset
/// doesn't change. [`Bake::register`] registers it for every kind a game
/// bakes.
pub struct Packed<K>(PhantomData<fn() -> K>);

impl<K> Packed<K> {
    /// The id of every kind's baked format.
    pub const ID: &'static str = "baked";

    /// The format.
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<K> Default for Packed<K> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K> core::fmt::Debug for Packed<K> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("Packed")
    }
}

impl<K: AssetKind + DeserializeOwned> Format for Packed<K> {
    type Asset = K;

    fn id(&self) -> &'static str {
        Self::ID
    }

    fn extensions(&self) -> &'static [&'static str] {
        &[]
    }

    fn sniff(&self, bytes: &[u8]) -> bool {
        bytes.starts_with(&BAKED_MAGIC)
    }

    fn max_bytes(&self) -> usize {
        MAX_BAKED_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<K, Vec<ContentError>> {
        let error = |message: String| vec![ContentError::new(file.shown(), message)];
        let sections =
            sections(bytes).ok_or_else(|| error("a baked file that is broken".to_owned()))?;
        let Some(&(_, data)) = sections.iter().find(|(kind, _)| *kind == K::NAME) else {
            let kinds: Vec<&str> = sections.iter().map(|&(kind, _)| kind).collect();
            return Err(error(format!(
                "baked as {} and not as a {}: bake the {} kind too",
                kinds.join(", "),
                K::NAME,
                K::NAME
            )));
        };
        match postcard::take_from_bytes::<K>(data) {
            Ok((asset, [])) => Ok(asset),
            _ => Err(error(format!(
                "its baked {} doesn't read as this game's: bake it again with `shuyi bake`",
                K::NAME
            ))),
        }
    }
}

#[cfg(test)]
mod first_tests;
#[cfg(test)]
mod one_bit_tests;
#[cfg(test)]
mod source_tests;
#[cfg(test)]
mod tests;
