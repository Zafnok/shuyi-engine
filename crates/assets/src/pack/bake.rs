//! Baking: which assets a game bakes ([`Bake`]), the pack made of its
//! bundle ([`Bake::bake`]), and the proof that every asset read from the
//! pack equals the one read from its source ([`Bake::check`]).
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};
use core::any::TypeId;
use core::fmt;
use core::marker::PhantomData;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{Content, PackInfo, Packed, write};
use crate::bundle::Bundle;
use crate::error::{ContentError, codes};
use crate::format::{AssetKind, Formats, RegisterError};

/// A kind that can be baked: it has a binary form (serde, both ways), and
/// two of it can be compared, for [`Bake::check`].
pub trait Bakeable: AssetKind + Serialize + DeserializeOwned + PartialEq + fmt::Debug {}

impl<K: AssetKind + Serialize + DeserializeOwned + PartialEq + fmt::Debug> Bakeable for K {}

/// Which files of a kind a [`Bake`] bakes.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Which {
    /// The asset with this id ([`Formats::find`]).
    Id(String),
    /// Every file under this directory that a format of the kind claims
    /// ([`Formats::load_under`]).
    Under(String),
}

/// A kind of a [`Bake`], whatever its type.
trait BakeKind: Send + Sync {
    /// The kind's name.
    fn name(&self) -> &'static str;

    /// The kind's type.
    fn type_id(&self) -> TypeId;

    /// The paths of the files `which` names in `bundle`.
    fn paths(
        &self,
        formats: &Formats,
        bundle: &Bundle,
        which: &Which,
    ) -> Result<Vec<String>, Vec<ContentError>>;

    /// The binary form of the asset read from the file at `path`.
    fn encode(
        &self,
        formats: &Formats,
        bundle: &Bundle,
        path: &str,
    ) -> Result<Vec<u8>, Vec<ContentError>>;

    /// Whether the asset at `path` reads the same from `pack` as from
    /// `source`; the error that says how it doesn't otherwise.
    fn same(
        &self,
        formats: &Formats,
        source: &Bundle,
        pack: &Bundle,
        path: &str,
    ) -> Result<(), Vec<ContentError>>;

    /// Registers the kind's [`Packed`] format, unless it is there.
    fn register(&self, formats: &mut Formats) -> Result<(), RegisterError>;

    /// Reads the asset at `path` of `bundle`, and drops it.
    fn load(&self, formats: &Formats, bundle: &Bundle, path: &str)
    -> Result<(), Vec<ContentError>>;
}

/// Kind `K`.
struct KindOf<K>(PhantomData<fn() -> K>);

impl<K: Bakeable> BakeKind for KindOf<K> {
    fn name(&self) -> &'static str {
        K::NAME
    }

    fn type_id(&self) -> TypeId {
        TypeId::of::<K>()
    }

    fn paths(
        &self,
        formats: &Formats,
        bundle: &Bundle,
        which: &Which,
    ) -> Result<Vec<String>, Vec<ContentError>> {
        match which {
            Which::Id(id) => formats
                .find::<K>(bundle, id)
                .map(|path| vec![path])
                .map_err(|e| vec![e]),
            Which::Under(dir) => Ok(bundle
                .files_under(dir)
                .into_iter()
                .filter(|path| formats.claims::<K>(path))
                .map(ToOwned::to_owned)
                .collect()),
        }
    }

    fn encode(
        &self,
        formats: &Formats,
        bundle: &Bundle,
        path: &str,
    ) -> Result<Vec<u8>, Vec<ContentError>> {
        let asset = formats.load::<K>(bundle, path)?;
        postcard::to_allocvec(&asset).map_err(|e| {
            let message = format!("its {} has no binary form: {e}", K::NAME);
            vec![ContentError::new(bundle.display_path(path), message)]
        })
    }

    fn same(
        &self,
        formats: &Formats,
        source: &Bundle,
        pack: &Bundle,
        path: &str,
    ) -> Result<(), Vec<ContentError>> {
        let shown = pack.display_path(path);
        if !formats.ids::<K>().contains(&Packed::<K>::ID) {
            let message = format!(
                "no format reads a baked {}: register the pack's formats (Bake::register)",
                K::NAME
            );
            return Err(vec![ContentError::new(shown, message)]);
        }
        let from_source = formats.load::<K>(source, path)?;
        let from_pack = formats.load::<K>(pack, path)?;
        if from_source == from_pack {
            return Ok(());
        }
        let message = format!(
            "the {} read from the pack differs from the one read from its source",
            K::NAME
        );
        Err(vec![
            ContentError::new(shown, message).with_code(codes::PACK_DIFFERS),
        ])
    }

    fn register(&self, formats: &mut Formats) -> Result<(), RegisterError> {
        if formats.ids::<K>().contains(&Packed::<K>::ID) {
            return Ok(());
        }
        formats.register(Packed::<K>::new())
    }

    fn load(
        &self,
        formats: &Formats,
        bundle: &Bundle,
        path: &str,
    ) -> Result<(), Vec<ContentError>> {
        formats.load::<K>(bundle, path).map(drop)
    }
}

/// What a game bakes into a pack: which assets of which kinds, the same
/// way its loaders ask for them ([`asset`](Self::asset) as
/// [`Formats::load`], [`under`](Self::under) as [`Formats::load_under`]),
/// and its stamp. Every other file of the bundle goes into the pack as its
/// own bytes.
///
/// A file whose bytes something other than its kind's format reads (a
/// picture the platform draws, a sound it plays, a file another format
/// reads beside its own) stays as it is: bake only kinds whose files
/// nothing else reads.
pub struct Bake {
    stamp: String,
    plan: Vec<(Box<dyn BakeKind>, Which)>,
}

impl fmt::Debug for Bake {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let plan: Vec<(&str, &Which)> = self
            .plan
            .iter()
            .map(|(kind, which)| (kind.name(), which))
            .collect();
        f.debug_struct("Bake")
            .field("stamp", &self.stamp)
            .field("plan", &plan)
            .finish()
    }
}

/// What [`Bake::bake`] made: the pack, and what went into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Baked {
    /// The pack's bytes.
    pub bytes: Vec<u8>,
    /// Every file in it, in path order.
    pub files: Vec<BakedFile>,
}

/// One file of a [`Baked`] pack.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BakedFile {
    /// Its path.
    pub path: String,
    /// The kinds it was baked as; none if it went in as its own bytes.
    pub kinds: Vec<&'static str>,
    /// Its source's bytes.
    pub source: usize,
}

impl Baked {
    /// The bytes of every file's source, added up.
    pub fn source_bytes(&self) -> usize {
        self.files.iter().map(|file| file.source).sum()
    }
}

impl Bake {
    /// A bake of nothing yet, with `stamp`: the game's word for the shapes
    /// of its own kinds (its version, say), which [`Pack::open`] then asks
    /// for.
    ///
    /// [`Pack::open`]: super::Pack::open
    pub fn new(stamp: impl Into<String>) -> Self {
        Self {
            stamp: stamp.into(),
            plan: Vec::new(),
        }
    }

    /// The stamp.
    pub fn stamp(&self) -> &str {
        &self.stamp
    }

    /// This bake, baking the asset of kind `K` with id `id` (`data/palette`
    /// or `data/palette.ron`).
    #[must_use]
    pub fn asset<K: Bakeable>(mut self, id: &str) -> Self {
        self.plan
            .push((Box::new(KindOf::<K>(PhantomData)), Which::Id(id.to_owned())));
        self
    }

    /// This bake, baking every file of kind `K` under directory `dir` (`""`
    /// for the whole bundle) whose extension a format of the kind claims.
    #[must_use]
    pub fn under<K: Bakeable>(mut self, dir: &str) -> Self {
        self.plan.push((
            Box::new(KindOf::<K>(PhantomData)),
            Which::Under(dir.to_owned()),
        ));
        self
    }

    /// The names of the kinds it bakes, each once, in the order first
    /// named.
    pub fn kinds(&self) -> Vec<&'static str> {
        let mut kinds = Vec::new();
        for (kind, _) in &self.plan {
            if !kinds.contains(&kind.name()) {
                kinds.push(kind.name());
            }
        }
        kinds
    }

    /// Registers in `formats` the format of a baked file of every kind it
    /// bakes ([`Packed`]), where it isn't already: what a game that loads
    /// a pack does at start-up.
    pub fn register(&self, formats: &mut Formats) -> Result<(), RegisterError> {
        self.plan
            .iter()
            .try_for_each(|(kind, _)| kind.register(formats))
    }

    /// The baked files: each path with the kinds it is baked as (in the
    /// order named), and every problem finding them.
    fn planned(
        &self,
        formats: &Formats,
        bundle: &Bundle,
    ) -> (BTreeMap<String, Vec<&dyn BakeKind>>, Vec<ContentError>) {
        let mut planned: BTreeMap<String, Vec<&dyn BakeKind>> = BTreeMap::new();
        let mut errors = Vec::new();
        let mut types: BTreeMap<&str, TypeId> = BTreeMap::new();
        for (kind, which) in &self.plan {
            let first = *types.entry(kind.name()).or_insert(kind.type_id());
            if first != kind.type_id() {
                let message = format!(
                    "the bake has two kinds called \"{}\": a pack tells kinds by name",
                    kind.name()
                );
                errors.push(ContentError::new("bake", message));
                continue;
            }
            match kind.paths(formats, bundle, which) {
                Ok(paths) => {
                    for path in paths {
                        let kinds = planned.entry(path).or_default();
                        if !kinds.iter().any(|k| k.type_id() == kind.type_id()) {
                            kinds.push(&**kind);
                        }
                    }
                }
                Err(e) => errors.extend(e),
            }
        }
        (planned, errors)
    }

    /// The pack of `bundle`, baked for `target`: each file this bake names
    /// holds its assets read with `formats`, and every other file its own
    /// bytes, and its index holds `bundle`'s fingerprint, which the pack
    /// then stands for. Refuses a source with errors: every one of them is
    /// returned.
    pub fn bake(
        &self,
        formats: &Formats,
        bundle: &Bundle,
        target: Option<&str>,
    ) -> Result<Baked, Vec<ContentError>> {
        let (planned, mut errors) = self.planned(formats, bundle);
        let mut contents = BTreeMap::new();
        let mut files = Vec::new();
        for path in bundle.files_under("") {
            let source = bundle.bytes(path).unwrap_or_default();
            let kinds = planned.get(path).map(Vec::as_slice).unwrap_or_default();
            let content = if kinds.is_empty() {
                Content::Raw(source.to_vec())
            } else {
                let mut sections = BTreeMap::new();
                for kind in kinds {
                    match kind.encode(formats, bundle, path) {
                        Ok(data) => {
                            sections.insert(kind.name(), data);
                        }
                        Err(e) => errors.extend(e),
                    }
                }
                Content::Baked(sections)
            };
            files.push(BakedFile {
                path: path.to_owned(),
                kinds: kinds.iter().map(|kind| kind.name()).collect(),
                source: source.len(),
            });
            contents.insert(path.to_owned(), content);
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let info = PackInfo {
            stamp: self.stamp.clone(),
            target: target.map(ToOwned::to_owned),
            content: bundle.fingerprint(),
        };
        Ok(Baked {
            bytes: write(&info, &contents),
            files,
        })
    }

    /// Reads every asset this bake names from `bundle` with `formats`, as
    /// the game's loaders would, and drops them: what `shuyi bake` times,
    /// from the sources and from the pack. Every error found.
    pub fn load(&self, formats: &Formats, bundle: &Bundle) -> Vec<ContentError> {
        let (planned, mut errors) = self.planned(formats, bundle);
        for (path, kinds) in &planned {
            for kind in kinds {
                if let Err(e) = kind.load(formats, bundle, path) {
                    errors.extend(e);
                }
            }
        }
        errors
    }

    /// Every way `pack` (a bundle of a baked pack) differs from `source`,
    /// the bundle it was baked from: an asset this bake names that reads
    /// differently from the two with `formats` (which must have the pack's
    /// formats: [`register`](Self::register)), a file the pack lacks or
    /// adds, or one kept as its own bytes that changed. None if the pack
    /// is true to its sources.
    pub fn check(&self, formats: &Formats, source: &Bundle, pack: &Bundle) -> Vec<ContentError> {
        let (planned, mut errors) = self.planned(formats, source);
        let sources = source.files_under("");
        for path in &sources {
            let shown = pack.display_path(path);
            let Some(bytes) = pack.bytes(path) else {
                errors.push(
                    ContentError::new(shown, "the pack lacks this file")
                        .with_code(codes::PACK_DIFFERS),
                );
                continue;
            };
            match planned.get(*path) {
                Some(kinds) => {
                    for kind in kinds {
                        if let Err(e) = kind.same(formats, source, pack, path) {
                            errors.extend(e);
                        }
                    }
                }
                None if Some(bytes) != source.bytes(path) => {
                    errors.push(
                        ContentError::new(shown, "the pack's copy differs from the file")
                            .with_code(codes::PACK_DIFFERS),
                    );
                }
                None => {}
            }
        }
        let sources: BTreeSet<&str> = sources.into_iter().collect();
        for path in pack.files_under("") {
            if !sources.contains(path) {
                let message = "the pack has this file and the sources don't";
                errors.push(
                    ContentError::new(pack.display_path(path), message)
                        .with_code(codes::PACK_DIFFERS),
                );
            }
        }
        errors
    }
}
