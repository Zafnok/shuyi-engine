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
//! rest), an index (the stamp, the target, and each file's path, kinds,
//! offset and sizes) and each file's bytes, compressed with LZ4 when that
//! makes them smaller.
//!
//! The rules:
//!
//! - **Text stays the source.** A pack is derived by [`Bake::bake`], and
//!   [`Bake::check`] proves every asset read from it equal to the one
//!   read from its source.
//! - **No migration.** A pack of another [`VERSION`], or baked for
//!   another stamp (the game's word for the shapes of its own kinds), is
//!   refused with a message to bake it again.
//! - **Never a panic.** A pack cut short or changed anywhere is refused:
//!   the checksum covers every byte after the header, and every size is
//!   checked before anything is allocated for it.
//!
//! The reader builds without `std`: a Playdate loads only packs
//! (ADR-0008 §5).
use crate::prelude::*;

use alloc::borrow::Cow;
use alloc::collections::BTreeMap;
use core::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::bundle::Layer;
use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format};

mod bake;

pub use bake::{Bake, Bakeable, Baked, BakedFile};

/// A pack's file extension, without the dot.
pub const EXTENSION: &str = "shuyipak";

/// The bytes a pack starts with.
pub const MAGIC: [u8; 8] = *b"SHUYIPAK";

/// The version of the pack format this engine reads and writes: the
/// header, the index, and the binary form of the engine's own kinds. A
/// change to any of them makes it the next number, and a pack of another
/// version is refused, never migrated.
pub const VERSION: u32 = 1;

/// The bytes a baked file in a pack starts with.
pub const BAKED_MAGIC: [u8; 8] = *b"SHUYIBKD";

/// The most files a pack holds.
pub const MAX_FILES: usize = 1 << 16;

/// The most bytes a pack's files add up to, unpacked: 1 GiB.
pub const MAX_UNPACKED_BYTES: u64 = 1 << 30;

/// The most bytes a baked file has: 64 MiB, as the engine's biggest
/// sources (a PNG, a sound).
pub const MAX_BAKED_BYTES: usize = 64 << 20;

/// How many bytes LZ4 can make of one, at most: a file claiming more
/// than this many times its stored bytes is broken.
const MAX_RATIO: u64 = 255;

/// The header: the magic bytes, the version (`u32`), the checksum of
/// everything after the header (`u64`) and the index's length (`u32`).
const HEADER: usize = 24;

/// What a pack says of itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackInfo {
    /// The game's word for the shapes of its own kinds, the ones a
    /// [`VERSION`] doesn't cover (its version, say). A game opens only a
    /// pack baked with its own stamp.
    pub stamp: String,
    /// The target the pack was baked for (`shuyi bake --target`), if any.
    pub target: Option<String>,
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
}

/// The index: what the pack says of itself, and its files in path order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Index {
    info: PackInfo,
    entries: Vec<Entry>,
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

/// The checksum of a pack: FNV-1a's constants over 64-bit words (the last
/// few bytes one at a time), each step followed by a shift that folds the
/// high bits into the low ones, then the length. Any one byte changed
/// changes it, since each step is a bijection of the hash so far.
fn checksum(bytes: &[u8]) -> u64 {
    const PRIME: u64 = 0x0100_0000_01b3;
    let step = |hash: u64, word: u64| {
        let hash = (hash ^ word).wrapping_mul(PRIME);
        hash ^ (hash >> 32)
    };
    let (words, rest) = bytes.as_chunks::<8>();
    let mut hash = 0xcbf2_9ce4_8422_2325;
    for word in words {
        hash = step(hash, u64::from_le_bytes(*word));
    }
    for &b in rest {
        hash = step(hash, u64::from(b));
    }
    step(hash, bytes.len() as u64)
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
    let mut entries = Vec::with_capacity(files.len());
    let mut data = Vec::new();
    for (path, content) in files {
        let (kinds, bytes) = match content {
            Content::Raw(bytes) => (Vec::new(), Cow::Borrowed(bytes.as_slice())),
            Content::Baked(sections) => (
                sections.keys().map(|&kind| kind.to_owned()).collect(),
                Cow::Owned(baked_file(sections)),
            ),
        };
        let packed = lz4_flex::block::compress(&bytes);
        let (codec, kept) = if packed.len() < bytes.len() {
            (Codec::Lz4, packed.as_slice())
        } else {
            (Codec::Stored, &*bytes)
        };
        entries.push(Entry {
            path: path.clone(),
            kinds,
            codec,
            offset: data.len() as u64,
            stored: kept.len() as u64,
            size: bytes.len() as u64,
        });
        data.extend_from_slice(kept);
    }
    let index = Index {
        info: info.clone(),
        entries,
    };
    // Strings, numbers and lists always serialize.
    let index = postcard::to_allocvec(&index).unwrap_or_default();
    let mut body = Vec::with_capacity(index.len() + data.len());
    body.extend_from_slice(&index);
    body.extend_from_slice(&data);
    let mut out = Vec::with_capacity(HEADER + body.len());
    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&checksum(&body).to_le_bytes());
    // An index over 4 GiB would hold more than `MAX_FILES` files.
    out.extend_from_slice(&u32::try_from(index.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

/// The index of pack `bytes` and where its files' bytes start, once its
/// header, version and checksum are right.
fn index(bytes: &[u8]) -> Result<(Index, &[u8]), PackError> {
    if !bytes.starts_with(&MAGIC) {
        return Err(if MAGIC.starts_with(bytes) {
            PackError::Truncated
        } else {
            PackError::NotAPack
        });
    }
    let found = u32::from_le_bytes(take(bytes, 8)?);
    if found != VERSION {
        return Err(PackError::Version { found });
    }
    let sum = u64::from_le_bytes(take(bytes, 12)?);
    let length = u32::from_le_bytes(take(bytes, 20)?);
    let body = bytes.get(HEADER..).ok_or(PackError::Truncated)?;
    let length = usize::try_from(length).map_err(|_| PackError::Truncated)?;
    if body.len() < length {
        return Err(PackError::Truncated);
    }
    if checksum(body) != sum {
        return Err(PackError::Checksum);
    }
    let (index, data) = body.split_at(length);
    let index: Index = postcard::from_bytes(index).map_err(|_| broken("its index doesn't read"))?;
    Ok((index, data))
}

/// The bytes of `entry` in `data`, unpacked, after the files before it
/// (`end` bytes of `data`) and `unpacked` bytes unpacked so far.
fn unpack<'a>(
    entry: &Entry,
    data: &'a [u8],
    end: u64,
    unpacked: u64,
) -> Result<Cow<'a, [u8]>, PackError> {
    let path = &entry.path;
    if entry.offset != end {
        return Err(broken(format!(
            "{path} starts at {} and the file before it ends at {end}",
            entry.offset
        )));
    }
    if unpacked.saturating_add(entry.size) > MAX_UNPACKED_BYTES {
        return Err(broken(format!(
            "its files add up to more than {MAX_UNPACKED_BYTES} bytes"
        )));
    }
    let size = usize::try_from(entry.size).map_err(|_| broken(format!("{path} is too big")))?;
    let range = usize::try_from(entry.offset)
        .ok()
        .zip(usize::try_from(entry.offset.saturating_add(entry.stored)).ok())
        .and_then(|(start, end)| data.get(start..end))
        .ok_or_else(|| broken(format!("{path} goes past the end of the pack")))?;
    match entry.codec {
        Codec::Stored if entry.stored == entry.size => Ok(Cow::Borrowed(range)),
        Codec::Stored => Err(broken(format!("{path} is stored with two sizes"))),
        Codec::Lz4 if entry.size > entry.stored.saturating_mul(MAX_RATIO) => Err(broken(format!(
            "{path} claims {size} bytes from {} stored",
            entry.stored
        ))),
        Codec::Lz4 => match lz4_flex::block::decompress(range, size) {
            Ok(bytes) if bytes.len() == size => Ok(Cow::Owned(bytes)),
            _ => Err(broken(format!("{path} doesn't unpack"))),
        },
    }
}

/// What pack `bytes` says of itself, once its header, version and
/// checksum are right: for a tool that looks at a pack without opening
/// it.
pub fn info(bytes: &[u8]) -> Result<PackInfo, PackError> {
    index(bytes).map(|(index, _)| index.info)
}

impl<'a> Pack<'a> {
    /// The pack `bytes`, every file unpacked, if it is of this engine's
    /// [`VERSION`], was baked with `stamp`, and holds together.
    pub fn open(bytes: &'a [u8], stamp: &str) -> Result<Self, PackError> {
        let (index, data) = index(bytes)?;
        if index.info.stamp != stamp {
            return Err(PackError::Stamp {
                found: index.info.stamp,
                expected: stamp.to_owned(),
            });
        }
        if index.entries.len() > MAX_FILES {
            return Err(broken(format!("it has more than {MAX_FILES} files")));
        }
        let mut files: Vec<PackFile<'a>> = Vec::with_capacity(index.entries.len());
        let (mut end, mut unpacked) = (0, 0);
        for entry in index.entries {
            if files.last().is_some_and(|last| last.path >= entry.path) {
                return Err(broken(format!("{} is out of path order", entry.path)));
            }
            let bytes = unpack(&entry, data, end, unpacked)?;
            if !entry.kinds.is_empty() && sections(&bytes).is_none() {
                return Err(broken(format!("{} isn't a whole baked file", entry.path)));
            }
            end = entry.offset.saturating_add(entry.stored);
            unpacked = unpacked.saturating_add(entry.size);
            files.push(PackFile {
                path: entry.path,
                kinds: entry.kinds,
                stored: usize::try_from(entry.stored).unwrap_or(usize::MAX),
                bytes,
            });
        }
        if usize::try_from(end).ok() != Some(data.len()) {
            return Err(broken("bytes follow its last file"));
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
    /// A bundle layer of the pack's files, shown as `name`. A game opens
    /// its pack once, at start-up (most simply one it embeds with
    /// `include_bytes!`): the files it unpacked are kept for the rest of
    /// the run, as the files a game embeds are.
    pub fn into_layer(self, name: &'static str) -> Layer {
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
        Layer::table(name, Box::leak(table.into_boxed_slice()))
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
mod tests;
