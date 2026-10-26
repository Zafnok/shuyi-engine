//! Formats are plugins (ADR-0007, ADR-0019). An **asset kind** is a plain
//! type ([`AssetKind`]): an image's size, a palette, a dialogue file's
//! scenes. A **format** is a value that reads one kind from the bytes of
//! one file ([`Format`]). [`Formats`] holds the formats of every kind; the
//! engine's standard ones are there from the start
//! ([`Formats::standard`]), and a game registers its own kinds and formats
//! at start-up. Code that uses an asset asks for its kind by id
//! ([`Formats::load`]) and never sees which file it came from.
//!
//! The rules of the registry:
//!
//! - **Lookup.** For each file, the formats of the kind whose
//!   [`sniff`](Format::sniff) says yes are tried first, in the order they
//!   were registered, then the one that claims the file's extension. The
//!   first that decodes the file gives the asset; when none does, the
//!   errors are the first one's.
//! - **Conflicts.** Within one kind, two formats may not have the same id
//!   or claim the same extension: [`Formats::register`] refuses the second
//!   with a message naming both, and [`Formats::replace`] puts it in place
//!   of the first. Different kinds may claim one extension (every RON
//!   file is `.ron`).
//! - **Limits.** A file bigger than its format's
//!   [`max_bytes`](Format::max_bytes), or than the registry's own
//!   [`Limits`], is refused before it is decoded.
//! - **One file.** A format reads one file, and the files it names
//!   through [`DecodeCx`]. A check against another kind of asset (a scene
//!   against the game's characters, a cinematic against its music) runs
//!   after, on the assets loaded.
//!
//! A front end may watch every decode the registry makes
//! ([`watch_decodes`]): its kind, its file, its size and whether it read,
//! told before and after each, on the thread that decodes. A debug build
//! tags what each decode allocates with its kind this way, so the heap
//! counter says what the game holds of each kind of asset.
use crate::prelude::*;

use alloc::collections::BTreeMap;
use core::any::Any;
use core::fmt;

use once_cell::race::OnceBox;

use crate::bundle::Bundle;
use crate::error::{ContentError, codes};

/// A kind of asset: the plain type a [`Format`] reads into. The engine's
/// kinds are its own types (`ImageInfo`, `PaletteDef`, `DialogueFile`…);
/// a game makes one of each of its data tables.
pub trait AssetKind: Sized + 'static {
    /// What messages call the kind, in lowercase words: `"image"`,
    /// `"palette"`.
    const NAME: &'static str;
}

/// The most bytes a format reads unless it says otherwise
/// ([`Format::max_bytes`]): 16 MiB.
pub const DEFAULT_MAX_BYTES: usize = 16 << 20;

/// The most bytes the engine's text formats read: 4 MiB, forty times the
/// biggest text file of the game the engine was carved from.
pub const TEXT_MAX_BYTES: usize = 4 << 20;

/// What a file asked for and missing is told.
pub const NOT_FOUND: &str = "file not found in asset bundle";

/// A file format: reads one [`AssetKind`] from a file's bytes.
pub trait Format: Send + Sync + 'static {
    /// The kind it reads.
    type Asset: AssetKind;

    /// Stable id, e.g. `"png"`, `"tiled-tmx"`, `"yarn"`: one per format
    /// of a kind.
    fn id(&self) -> &'static str;

    /// File extensions this format claims, lowercase, without the dot. A
    /// format that only stands in another's place (a one-bit picture at a
    /// PNG's path) claims none and is found by [`sniff`](Self::sniff).
    fn extensions(&self) -> &'static [&'static str];

    /// True if these bytes look like this format (magic bytes); `false`
    /// when the format has none.
    fn sniff(&self, bytes: &[u8]) -> bool {
        let _ = bytes;
        false
    }

    /// The most bytes it reads: a bigger file is refused before
    /// [`decode`](Self::decode) sees it. [`DEFAULT_MAX_BYTES`] unless the
    /// format says.
    fn max_bytes(&self) -> usize {
        DEFAULT_MAX_BYTES
    }

    /// Every problem found, each with file and place; never a panic.
    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<Self::Asset, Vec<ContentError>>;
}

/// A format that can also write its kind: what tools and an optimized
/// format derived from a source (ADR-0007 §4) use.
pub trait Encode: Format {
    /// `asset` as this format's bytes. Decoding them gives an asset equal
    /// to `asset`.
    fn encode(&self, asset: &Self::Asset) -> Vec<u8>;
}

/// Where a file is: its path in the bundle (relative to a layer's root,
/// `/`-separated), and the name errors show it by (`base/images/a.png`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AssetPath {
    path: String,
    shown: String,
}

impl AssetPath {
    /// The file at `path`, shown as `path`.
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            shown: path.clone(),
            path,
        }
    }

    /// The file at `path` in `bundle`, shown as the bundle shows it
    /// ([`Bundle::display_path`]).
    pub fn in_bundle(bundle: &Bundle, path: &str) -> Self {
        Self {
            path: path.to_owned(),
            shown: bundle.display_path(path),
        }
    }

    /// This path, shown as `shown`.
    #[must_use]
    pub fn shown_as(self, shown: impl Into<String>) -> Self {
        Self {
            shown: shown.into(),
            ..self
        }
    }

    /// The path in the bundle.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The name errors show it by.
    pub fn shown(&self) -> &str {
        &self.shown
    }

    /// The file's name: what follows the last `/`.
    pub fn name(&self) -> &str {
        self.path
            .rsplit_once('/')
            .map_or(&self.path, |(_, name)| name)
    }

    /// The directory it is in: what comes before the last `/`, or `""` at
    /// the root.
    pub fn dir(&self) -> &str {
        self.path.rsplit_once('/').map_or("", |(dir, _)| dir)
    }

    /// Its extension as written (in any case), without the dot: what
    /// follows the last `.` of its name, when something comes before it.
    pub fn extension(&self) -> Option<&str> {
        let (stem, extension) = self.name().rsplit_once('.')?;
        (!stem.is_empty()).then_some(extension)
    }

    /// Its name without its extension.
    pub fn stem(&self) -> &str {
        let name = self.name();
        match self.extension() {
            Some(extension) => &name[..name.len() - extension.len() - 1],
            None => name,
        }
    }

    /// The path of the file `name` in this one's directory.
    pub fn sibling(&self, name: &str) -> String {
        join(self.dir(), name)
    }

    /// The path of the file beside this one with its stem and the
    /// extension `extension` (`atlas.ron` → `atlas.png`).
    pub fn with_extension(&self, extension: &str) -> String {
        self.sibling(&format!("{}.{extension}", self.stem()))
    }
}

/// `name` inside `dir` (`""` for the root).
pub fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}

/// How much a decoding may read, beyond each format's own
/// [`max_bytes`](Format::max_bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most bytes a file may have: lower it to refuse big files from
    /// mods or downloads. [`usize::MAX`] (no limit but the formats' own)
    /// by default.
    pub max_bytes: usize,
}

impl Limits {
    /// No limit but the formats' own.
    pub const NONE: Self = Self {
        max_bytes: usize::MAX,
    };
}

impl Default for Limits {
    fn default() -> Self {
        Self::NONE
    }
}

/// What a format has besides the file it decodes: the limits, and the
/// bundle the file is in, for the files it names (a sidecar's picture, a
/// manifest's sounds). Without a bundle, every file named is missing.
#[derive(Debug, Clone, Copy, Default)]
pub struct DecodeCx<'a> {
    /// The limits of this decoding.
    pub limits: Limits,
    /// The bundle the file is in, if any.
    pub bundle: Option<&'a Bundle>,
}

impl<'a> DecodeCx<'a> {
    /// No bundle, no limits but the formats' own: for bytes decoded alone.
    pub const fn new() -> Self {
        Self {
            limits: Limits::NONE,
            bundle: None,
        }
    }

    /// Decoding a file of `bundle`.
    pub const fn in_bundle(bundle: &'a Bundle) -> Self {
        Self {
            limits: Limits::NONE,
            bundle: Some(bundle),
        }
    }

    /// This context, with `limits`.
    #[must_use]
    pub const fn with_limits(self, limits: Limits) -> Self {
        Self { limits, ..self }
    }

    /// Whether the bundle has the file at `path` ([`Bundle::has`]).
    pub fn has(&self, path: &str) -> bool {
        self.bundle.is_some_and(|bundle| bundle.has(path))
    }

    /// The bytes of the file at `path` (relative to a layer's root), if the
    /// bundle has it.
    pub fn bytes(&self, path: &str) -> Option<&'static [u8]> {
        self.bundle?.bytes(path)
    }

    /// The text of the file at `path`, if the bundle has it and it is valid
    /// UTF-8.
    pub fn text(&self, path: &str) -> Option<&'static str> {
        self.bundle?.file(path)
    }

    /// The name errors show the file at `path` by.
    pub fn shown(&self, path: &str) -> String {
        self.bundle
            .map_or_else(|| path.to_owned(), |bundle| bundle.display_path(path))
    }

    /// Whether the bundle leaves the file at `path` out on purpose
    /// ([`Bundle::left_out`]).
    pub fn left_out(&self, path: &str) -> bool {
        self.bundle.is_some_and(|bundle| bundle.left_out(path))
    }

    /// The paths of the files inside directory `dir` or below it
    /// ([`Bundle::files_under`]); none without a bundle.
    pub fn files_under(&self, dir: &str) -> Vec<&'static str> {
        self.bundle
            .map(|bundle| bundle.files_under(dir))
            .unwrap_or_default()
    }
}

/// The text of a text format's file `bytes`, or the error that it isn't
/// valid UTF-8.
pub fn text<'b>(file: &AssetPath, bytes: &'b [u8]) -> Result<&'b str, Vec<ContentError>> {
    core::str::from_utf8(bytes).map_err(|_| {
        vec![ContentError::new(file.shown(), "file is not valid UTF-8").with_code(codes::NOT_TEXT)]
    })
}

/// `format`'s asset in `bytes` (the file `file`), unless the file is
/// bigger than `format` or `cx` allow.
pub fn decode_with<F: Format + ?Sized>(
    format: &F,
    file: &AssetPath,
    bytes: &[u8],
    cx: &DecodeCx<'_>,
) -> Result<F::Asset, Vec<ContentError>> {
    let max = format.max_bytes().min(cx.limits.max_bytes);
    if bytes.len() > max {
        let message = format!(
            "the file is {} bytes; the \"{}\" {} format reads at most {max}",
            bytes.len(),
            format.id(),
            <F::Asset as AssetKind>::NAME
        );
        return Err(vec![
            ContentError::new(file.shown(), message).with_code(codes::TOO_BIG),
        ]);
    }
    format.decode(file, bytes, cx)
}

/// One decode the registry made ([`Formats::decode`]), as a
/// [`DecodeWatch`] is told it. Build it with [`Decoded::new`] (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Decoded<'a> {
    /// The kind decoded: its [`AssetKind::NAME`], `"image"`, `"palette"`.
    pub kind: &'static str,
    /// The file's path in its bundle ([`AssetPath::path`]).
    pub path: &'a str,
    /// The file's size, in bytes.
    pub bytes: usize,
    /// Whether a format read it.
    pub ok: bool,
}

impl<'a> Decoded<'a> {
    /// A decode of `kind` from `path`, `bytes` long, that read (`ok`) or
    /// didn't.
    pub const fn new(kind: &'static str, path: &'a str, bytes: usize, ok: bool) -> Self {
        Self {
            kind,
            path,
            bytes,
            ok,
        }
    }
}

/// What is told of every decode the registry makes, once set
/// ([`watch_decodes`]): two plain functions, called on the thread that
/// decodes. Build it with [`DecodeWatch::new`] (ADR-0039).
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct DecodeWatch {
    /// Told the kind as a decode starts; what it gives back, a token, goes
    /// to `after`.
    pub before: fn(kind: &'static str) -> u32,
    /// Told the decode once it is decided, with `before`'s token.
    pub after: fn(token: u32, decoded: &Decoded<'_>),
}

impl DecodeWatch {
    /// A watch that calls `before` as each decode starts and `after` once
    /// it is decided.
    pub const fn new(
        before: fn(kind: &'static str) -> u32,
        after: fn(token: u32, decoded: &Decoded<'_>),
    ) -> Self {
        Self { before, after }
    }
}

/// The watch of every decode, once one is set.
static DECODE_WATCH: OnceBox<DecodeWatch> = OnceBox::new();

/// Tells `watch` of every decode any registry makes from now on
/// ([`Formats::decode`], so [`Formats::load`] and the rest): `before` as
/// it starts, with its kind, and `after` once it is decided, with
/// `before`'s token, the file and whether it read. For a front end that
/// tags, logs or tallies what it loads. Set once: a second call gives its
/// watch back and changes nothing.
pub fn watch_decodes(watch: DecodeWatch) -> Result<(), DecodeWatch> {
    DECODE_WATCH.set(Box::new(watch)).map_err(|watch| *watch)
}

/// Why a format could not be registered.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegisterError {
    /// It claims an extension an earlier format of its kind claims.
    #[error(
        "{kind} format \"{format}\" claims the extension \"{extension}\", which {kind} format \
         \"{earlier}\" claims already; replace \"{earlier}\" to read .{extension} files with \
         \"{format}\""
    )]
    Extension {
        /// The kind.
        kind: &'static str,
        /// The format refused.
        format: &'static str,
        /// The format that claims the extension.
        earlier: &'static str,
        /// The extension.
        extension: &'static str,
    },
    /// An earlier format of its kind has its id.
    #[error("{kind} format \"{format}\" is registered already; replace it to use the new one")]
    Id {
        /// The kind.
        kind: &'static str,
        /// The id.
        format: &'static str,
    },
    /// One of its extensions isn't lowercase, has a dot, or is empty.
    #[error(
        "{kind} format \"{format}\" claims the extension \"{extension}\"; an extension is \
         lowercase, without the dot, and not empty"
    )]
    BadExtension {
        /// The kind.
        kind: &'static str,
        /// The format refused.
        format: &'static str,
        /// The extension.
        extension: &'static str,
    },
}

/// A kind's formats, in the order they were registered.
struct KindList<K> {
    formats: Vec<Box<dyn Format<Asset = K>>>,
}

/// A [`KindList`] of any kind: what [`Formats`] keeps.
trait Kind: Any + Send + Sync {
    /// The kind's name.
    fn name(&self) -> &'static str;

    /// Its formats' ids, in order.
    fn ids(&self) -> Vec<&'static str>;
}

impl<K: AssetKind> Kind for KindList<K> {
    fn name(&self) -> &'static str {
        K::NAME
    }

    fn ids(&self) -> Vec<&'static str> {
        self.formats.iter().map(|format| format.id()).collect()
    }
}

/// Whether `format` claims the extension `extension` (any case).
fn claims<K: AssetKind>(format: &dyn Format<Asset = K>, extension: &str) -> bool {
    format
        .extensions()
        .iter()
        .any(|claimed| claimed.eq_ignore_ascii_case(extension))
}

/// The first of `earlier` that has `format`'s id or one of its extensions,
/// as the error that refuses `format`.
fn conflict<F: Format>(
    earlier: &[Box<dyn Format<Asset = F::Asset>>],
    format: &F,
) -> Option<RegisterError> {
    let kind = <F::Asset as AssetKind>::NAME;
    earlier.iter().find_map(|old| {
        if old.id() == format.id() {
            return Some(RegisterError::Id {
                kind,
                format: format.id(),
            });
        }
        let extension = format
            .extensions()
            .iter()
            .find(|&&extension| claims(&**old, extension))?;
        Some(RegisterError::Extension {
            kind,
            format: format.id(),
            earlier: old.id(),
            extension,
        })
    })
}

/// The error for the first of `format`'s extensions that isn't lowercase
/// and without a dot, if one isn't.
fn bad_extension<F: Format>(format: &F) -> Option<RegisterError> {
    let extension = format.extensions().iter().find(|extension| {
        extension.is_empty() || extension.contains('.') || extension.chars().any(char::is_uppercase)
    })?;
    Some(RegisterError::BadExtension {
        kind: <F::Asset as AssetKind>::NAME,
        format: format.id(),
        extension,
    })
}

/// The formats of every kind ([module docs](self)).
#[derive(Default)]
pub struct Formats {
    /// One [`KindList`] per kind that has a format.
    kinds: Vec<Box<dyn Kind>>,
    /// The limits [`Formats::load`] and [`Formats::load_under`] decode with.
    pub limits: Limits,
}

impl fmt::Debug for Formats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let kinds: Vec<(&str, Vec<&str>)> = self
            .kinds
            .iter()
            .map(|kind| (kind.name(), kind.ids()))
            .collect();
        f.debug_struct("Formats")
            .field("kinds", &kinds)
            .field("limits", &self.limits)
            .finish()
    }
}

impl Formats {
    /// A registry with no format at all.
    pub fn new() -> Self {
        Self::default()
    }

    /// A registry with the engine's standard formats, those that need
    /// nothing of the game ([`Formats::register_standard`]).
    pub fn standard() -> Self {
        let mut formats = Self::new();
        // On an empty registry none of them is refused: they claim no id
        // or extension twice in a kind, which a test checks.
        let _ = formats.register_standard();
        formats
    }

    /// Registers the engine's standard formats after those already here:
    /// every format of the engine that needs nothing of the game. The
    /// formats that read a game's own types (its map features, dialogue
    /// directives, actions, shots) or follow its rules (the glyphs it
    /// needs, its language rules) are the game's to register, with them.
    pub fn register_standard(&mut self) -> Result<(), RegisterError> {
        use crate::{
            audio, bitmap, credits, dialogue, font, image, lighting, names, onebit, palette, target,
        };
        self.register(image::PngInfo)?;
        self.register(bitmap::OneBitInfo)?;
        #[cfg(feature = "png")]
        self.register(image::Png)?;
        self.register(audio::Wav)?;
        self.register(audio::OggVorbis)?;
        self.register(audio::OggVorbisTrack)?;
        self.register(audio::AudioManifestRon)?;
        self.register(credits::CreditsRon::default())?;
        self.register(font::FontAtlasRon::default())?;
        self.register(palette::PaletteRon::default())?;
        self.register(onebit::PatternMapRon)?;
        self.register(lighting::LightingRon::default())?;
        self.register(names::NamesRon)?;
        self.register(dialogue::Dlg::<dialogue::NoDirective>::new())?;
        self.register(target::TargetRon)?;
        Ok(())
    }

    /// The list of kind `K`, if it has one.
    fn list<K: AssetKind>(&self) -> &[Box<dyn Format<Asset = K>>] {
        self.kinds
            .iter()
            .find_map(|kind| (&**kind as &dyn Any).downcast_ref::<KindList<K>>())
            .map_or(&[], |list| &list.formats)
    }

    /// The list of kind `K`, if it has one, to change.
    fn list_mut<K: AssetKind>(&mut self) -> Option<&mut KindList<K>> {
        self.kinds
            .iter_mut()
            .find_map(|kind| (&mut **kind as &mut dyn Any).downcast_mut::<KindList<K>>())
    }

    /// Adds `format` after the formats of its kind, unless one of them has
    /// its id or claims one of its extensions.
    pub fn register<F: Format>(&mut self, format: F) -> Result<(), RegisterError> {
        if let Some(error) = bad_extension(&format).or_else(|| conflict(self.list(), &format)) {
            return Err(error);
        }
        self.insert(None, Box::new(format));
        Ok(())
    }

    /// Adds `format` in place of the formats of its kind that have its id or
    /// claim one of its extensions (after them all if none does), and
    /// returns their ids.
    pub fn replace<F: Format>(&mut self, format: F) -> Result<Vec<&'static str>, RegisterError> {
        if let Some(error) = bad_extension(&format) {
            return Err(error);
        }
        let mut replaced = Vec::new();
        let mut at = None;
        if let Some(list) = self.list_mut::<F::Asset>() {
            let clash = |old: &dyn Format<Asset = F::Asset>| {
                old.id() == format.id()
                    || format
                        .extensions()
                        .iter()
                        .any(|&extension| claims(old, extension))
            };
            for old in core::mem::take(&mut list.formats) {
                if clash(&*old) {
                    // The place of the first replaced: after the formats
                    // kept before it.
                    at.get_or_insert(list.formats.len());
                    replaced.push(old.id());
                } else {
                    list.formats.push(old);
                }
            }
        }
        self.insert(at, Box::new(format));
        Ok(replaced)
    }

    /// Puts `format` at `at` among its kind's formats (last if `None`).
    fn insert<K: AssetKind>(&mut self, at: Option<usize>, format: Box<dyn Format<Asset = K>>) {
        match self.list_mut::<K>() {
            Some(list) => list
                .formats
                .insert(at.unwrap_or(list.formats.len()), format),
            None => self.kinds.push(Box::new(KindList {
                formats: vec![format],
            })),
        }
    }

    /// The ids of kind `K`'s formats, in the order they are tried.
    pub fn ids<K: AssetKind>(&self) -> Vec<&'static str> {
        self.list::<K>().iter().map(|format| format.id()).collect()
    }

    /// The extensions kind `K`'s formats claim, in their order.
    pub fn extensions<K: AssetKind>(&self) -> Vec<&'static str> {
        self.list::<K>()
            .iter()
            .flat_map(|format| format.extensions().iter().copied())
            .collect()
    }

    /// Whether a format of kind `K` claims the extension of the file at
    /// `path`.
    pub fn claims<K: AssetKind>(&self, path: &str) -> bool {
        let file = AssetPath::new(path);
        let Some(extension) = file.extension() else {
            return false;
        };
        self.list::<K>()
            .iter()
            .any(|format| claims(&**format, extension))
    }

    /// The asset of kind `K` in `bytes`, the contents of `file`, by the
    /// lookup rule: the formats that sniff `bytes` in order, then the one
    /// that claims `file`'s extension. The first that decodes it gives the
    /// asset; when none does, the errors are the first one's. A
    /// [`watch_decodes`] watch is told before and after.
    pub fn decode<K: AssetKind>(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<K, Vec<ContentError>> {
        let Some(watch) = DECODE_WATCH.get() else {
            return self.decode_unwatched(file, bytes, cx);
        };
        let token = (watch.before)(K::NAME);
        let decoded = self.decode_unwatched(file, bytes, cx);
        let told = Decoded::new(K::NAME, file.path(), bytes.len(), decoded.is_ok());
        (watch.after)(token, &told);
        decoded
    }

    /// [`decode`](Self::decode), with no watch told.
    fn decode_unwatched<K: AssetKind>(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        cx: &DecodeCx<'_>,
    ) -> Result<K, Vec<ContentError>> {
        let formats = self.list::<K>();
        let sniffed: Vec<bool> = formats.iter().map(|format| format.sniff(bytes)).collect();
        let extension = file.extension();
        let by_sniff = formats.iter().zip(&sniffed).filter(|&(_, &yes)| yes);
        let by_extension = formats.iter().zip(&sniffed).filter(|&(format, &yes)| {
            !yes && extension.is_some_and(|extension| claims(&**format, extension))
        });
        let mut first = None;
        for (format, _) in by_sniff.chain(by_extension) {
            match decode_with(&**format, file, bytes, cx) {
                Ok(asset) => return Ok(asset),
                Err(errors) => {
                    first.get_or_insert(errors);
                }
            }
        }
        Err(first.unwrap_or_else(|| vec![self.unread::<K>(file)]))
    }

    /// The error for `file`, which no format of kind `K` sniffs or claims.
    fn unread<K: AssetKind>(&self, file: &AssetPath) -> ContentError {
        let kind = K::NAME;
        let why = match file.extension() {
            Some(extension) => {
                format!("no {kind} format knows its bytes or claims its extension \"{extension}\"")
            }
            None => format!("no {kind} format knows its bytes, and it has no extension"),
        };
        let ids = self.ids::<K>();
        let known = if ids.is_empty() {
            format!("no {kind} format is registered")
        } else {
            format!("the {kind} formats are {}", ids.join(", "))
        };
        ContentError::new(file.shown(), format!("{why}; {known}")).with_code(codes::NO_FORMAT)
    }

    /// The path of the asset of kind `K` with id `id`: the file at `id`
    /// itself if `bundle` has one, else the one file at `id` and an
    /// extension kind `K`'s formats claim (`images/card` is
    /// `images/card.png`, or `images/card.txtimg` once a format claims
    /// that).
    pub fn find<K: AssetKind>(&self, bundle: &Bundle, id: &str) -> Result<String, ContentError> {
        if bundle.has(id) {
            return Ok(id.to_owned());
        }
        let tried: Vec<String> = self
            .extensions::<K>()
            .iter()
            .map(|extension| format!("{id}.{extension}"))
            .collect();
        let found: Vec<&String> = tried.iter().filter(|path| bundle.has(path)).collect();
        let (code, message) = match found.as_slice() {
            [path] => return Ok((*path).clone()),
            [] if tried.is_empty() => (codes::MISSING_FILE, NOT_FOUND.to_owned()),
            [] => (
                codes::MISSING_FILE,
                format!("{NOT_FOUND}, nor {}", tried.join(" nor ")),
            ),
            many => {
                let shown: Vec<String> =
                    many.iter().map(|path| bundle.display_path(path)).collect();
                let message = format!(
                    "{} files could be this {}: {}; name the one meant with its extension",
                    shown.len(),
                    K::NAME,
                    shown.join(", ")
                );
                (codes::AMBIGUOUS_FILE, message)
            }
        };
        Err(ContentError::new(bundle.display_path(id), message).with_code(code))
    }

    /// The asset of kind `K` with id `id` in `bundle` ([`Formats::find`]),
    /// decoded with the registry's [`limits`](Self::limits).
    pub fn load<K: AssetKind>(&self, bundle: &Bundle, id: &str) -> Result<K, Vec<ContentError>> {
        let path = self.find::<K>(bundle, id).map_err(|e| vec![e])?;
        self.read(bundle, &path)
    }

    /// The file at `path` of `bundle`, decoded as kind `K`.
    fn read<K: AssetKind>(&self, bundle: &Bundle, path: &str) -> Result<K, Vec<ContentError>> {
        let bytes = bundle.bytes(path).unwrap_or_default();
        let cx = DecodeCx::in_bundle(bundle).with_limits(self.limits);
        self.decode(&AssetPath::in_bundle(bundle, path), bytes, &cx)
    }

    /// Every asset of kind `K` in `bundle` under directory `dir` (`""` for
    /// the whole bundle): each file there whose extension a format of the
    /// kind claims, by path. Reports every problem of every file.
    ///
    /// All or nothing over a tree: the assets come only when every file
    /// reads, and folders below `dir` are read too. A loader that checks
    /// each file after reading wants [`Formats::load_each`] instead.
    pub fn load_under<K: AssetKind>(
        &self,
        bundle: &Bundle,
        dir: &str,
    ) -> Result<BTreeMap<&'static str, K>, Vec<ContentError>> {
        let mut assets = BTreeMap::new();
        let mut errors = Vec::new();
        for path in bundle.files_under(dir) {
            if !self.claims::<K>(path) {
                continue;
            }
            match self.read(bundle, path) {
                Ok(asset) => {
                    assets.insert(path, asset);
                }
                Err(e) => errors.extend(e),
            }
        }
        if errors.is_empty() {
            Ok(assets)
        } else {
            Err(errors)
        }
    }

    /// Each asset of kind `K` directly inside directory `dir` of `bundle`
    /// (`""` for the root), by path: each file there whose extension a
    /// format of the kind claims, with its asset or its errors.
    ///
    /// File by file in one folder: a broken file leaves the others read,
    /// so checks that run after still see every good file, and folders
    /// below `dir` (which may hold another kind with the same extension)
    /// are not read. A missing folder gives an empty map.
    pub fn load_each<K: AssetKind>(
        &self,
        bundle: &Bundle,
        dir: &str,
    ) -> BTreeMap<&'static str, Result<K, Vec<ContentError>>> {
        bundle
            .files_in(dir)
            .into_iter()
            .filter(|path| self.claims::<K>(path))
            .map(|path| (path, self.read(bundle, path)))
            .collect()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) mod builtin_tests;
