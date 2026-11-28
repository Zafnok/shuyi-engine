//! Part sets (the `parts` feature, off by default): pictures made of
//! parts, for the people, creatures or things nobody drew one by one.
//! Hand-drawn pictures stay the engine's way ([`picture_set`] gives each
//! named person theirs); a game opts into this for its crowds.
//!
//! The parts (heads, eyes, hair, hats) are drawn in grey, and each grey
//! that matters is a **key**: this grey is skin, that one skin in shadow,
//! that one hair. A [`PartSet`] says which slots a picture has, in draw
//! order, which parts fit each slot, and which colours each key may take.
//! Each **kind** (a people, a species, a class of item) lists what it
//! rolls, and a kind with a `base` overrides only what it names:
//!
//! ```ron
//! (
//!     size: (32, 32),
//!     // Draw order, first undermost.
//!     slots: ["head", "ears", "eyes", "hair", "helmet"],
//!     // A grey (red = green = blue) and what it stands for.
//!     keys: { 200: "skin", 150: "skin_shadow", 120: "hair" },
//!     // Keys coloured together: one roll picks a row.
//!     colours: {
//!         "skin": (keys: ["skin", "skin_shadow"], rows: [
//!             ["#F1C27D", "#C68642"], ["#8D5524", "#5C3A1A"],
//!         ]),
//!         "hair": (keys: ["hair"], rows: [["#2B1B10"], ["#D8B060"]]),
//!     },
//!     parts: {
//!         "head_round": (slot: "head", image: "faces/heads.png", at: (0, 0)),
//!         "ears_long": (slot: "ears", image: "faces/ears.png", at: (0, 0)),
//!         "hair_short": (slot: "hair", image: "faces/hair.png", at: (0, 0)),
//!         "helm_iron": (slot: "helmet", image: "faces/gear.png", at: (0, 0),
//!             hides: ["hair", "ears"]),
//!     },
//!     kinds: {
//!         "plain": (slots: {
//!             "head": ["head_round"],
//!             "hair": ["hair_short", None],
//!         }),
//!         "elf": (base: Some("plain"), slots: { "ears": ["ears_long"] }),
//!     },
//! )
//! ```
//!
//! Each part is the set's `size`, taken at `at` in its image (relative to
//! the file's folder), and drawn over the whole picture. `None` in a list
//! is "nothing in this slot", and a list may name a part twice to roll it
//! more often. A slot no kind of the chain names is empty. A kind may
//! also replace a colour group's rows (`colours: { "skin": [["#5A8A3A",
//! "#3A6A2A"]] }`), the group's keys staying the set's.
//!
//! The file is a [`PartsDef`], read by the format [`PartsRon`] and
//! checked alone ([`PartsDef::from_source`]); [`PartsDef::resolve`] makes
//! the [`PartSet`] once the image table is known, and
//! [`PartSet::check_pixels`] finds a grey a brush slipped on, once the
//! pixels are.
//!
//! # A look, rolled
//!
//! The rules keep a person's seed and kind (and what they wear) in their
//! state, never pixels or colours. [`PartSet::appearance`] works the look
//! out from those, the same on every platform:
//!
//! 1. each slot rolls one of its kind's list, from a [`SimRng`] of its
//!    own, seeded by the FNV hash of the seed and the slot's name;
//! 2. each colour group rolls a row the same way, from its name;
//! 3. each worn part takes its slot;
//! 4. from the topmost slot down, a part still shown empties the slots
//!    it `hides` (which are all under it), so a hidden part hides nothing.
//!
//! So adding a slot or a colour group to a set changes no other slot's
//! roll: a mod that adds earrings leaves every face of a saved game as
//! it was. The skin paints the [`Appearance`] it gives into a painted
//! picture (ADR-0078) with `shuyi_ui::frame::parts::paint`, behind
//! `shuyi-ui`'s `parts` feature (which turns this one on), and paints it
//! again when the person changes. A headless build rolls without
//! painting.
//!
//! [`picture_set`]: crate::picture_set

use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use shuyi_core::rng::SimRng;

use crate::error::{ContentError, codes};
use crate::fnv::Hasher;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, join, text};
use crate::image::{Image, ImageId, ImageInfo, ImageRect, ImageTable};
use crate::palette::parse_hex;
use crate::ron::parse_ron_implicit_some;

mod check;
#[cfg(test)]
mod tests;

/// The most slots a set may have.
pub const MAX_SLOTS: usize = 64;

/// The most parts a set may have.
pub const MAX_PARTS: usize = 1024;

/// The most kinds a set may have.
pub const MAX_KINDS: usize = 256;

/// The most rows a colour group may have.
pub const MAX_ROWS: usize = 64;

/// The longest side a set's pictures may have, in pixels.
pub const MAX_SIDE: u32 = 256;

/// How near a key a grey may be and still be a slip of the brush
/// ([`PartSet::check_pixels`]).
pub const NEAR_KEY: u8 = 4;

/// The alpha from which a part's pixel is drawn; under it, the pixel is
/// left out.
pub const DRAWN_ALPHA: u8 = 128;

/// A part set as written. Read it with [`PartsRon`] or
/// [`PartsDef::from_source`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PartsDef {
    /// Every picture's and every part's width and height, in pixels.
    pub size: (u32, u32),
    /// The slots, in draw order, first undermost.
    pub slots: Vec<String>,
    /// Each key grey and what it stands for, in the file's order.
    #[serde(with = "entries")]
    pub keys: Vec<(u8, String)>,
    /// The colour groups, by name.
    #[serde(default)]
    pub colours: BTreeMap<String, ColourGroupDef>,
    /// The parts, by name.
    pub parts: BTreeMap<String, PartDef>,
    /// The kinds, by name.
    pub kinds: BTreeMap<String, KindDef>,
}

/// Keys coloured together: one roll picks a row, a colour for each key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct ColourGroupDef {
    /// The keys it colours, by what they stand for.
    pub keys: Vec<String>,
    /// The rows, each a `#RRGGBB` colour per key.
    pub rows: Vec<Vec<String>>,
}

/// A part as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PartDef {
    /// The slot it goes in.
    pub slot: String,
    /// Its image, relative to the file's folder.
    pub image: String,
    /// Its top-left corner in the image.
    pub at: (u32, u32),
    /// The slots it empties when shown, all under its own.
    #[serde(default)]
    pub hides: Vec<String>,
}

/// A kind as written: what it rolls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct KindDef {
    /// The kind it takes its lists and colours from, if any.
    #[serde(default)]
    pub base: Option<String>,
    /// Each slot's list (a part, or `None` for nothing), replacing the
    /// base's.
    #[serde(default)]
    pub slots: BTreeMap<String, Vec<Option<String>>>,
    /// Each colour group's rows, replacing the set's or the base's.
    #[serde(default)]
    pub colours: BTreeMap<String, Vec<Vec<String>>>,
}

impl AssetKind for PartsDef {
    const NAME: &'static str = "part set";
}

impl PartsDef {
    /// Parses `source`, the file shown as `file`, and checks it alone.
    /// Reports every problem, each at its line when the file shows it.
    pub fn from_source(file: &str, source: &str) -> Result<Self, Vec<ContentError>> {
        let def: Self = parse_ron_implicit_some(file, source).map_err(|e| vec![e])?;
        let errors = check::problems(&def, file, source);
        if errors.is_empty() {
            Ok(def)
        } else {
            Err(errors)
        }
    }

    /// The set a skin paints from: each part's image, relative to `dir`
    /// (the file's folder), an image of `images`, and its rectangle inside
    /// it. Reports every part that isn't, as a problem of `file`. The
    /// definition must be one [`from_source`](Self::from_source) passed.
    pub fn resolve(
        &self,
        file: &str,
        dir: &str,
        images: &ImageTable,
    ) -> Result<PartSet, Vec<ContentError>> {
        let slot_of = |name: &str| self.slots.iter().position(|s| s == name).unwrap_or(0);
        let (w, h) = self.size;
        let mut errors = Vec::new();
        let mut parts = BTreeMap::new();
        for (name, part) in &self.parts {
            let path = join(dir, &part.image);
            let rect = ImageRect {
                x: part.at.0,
                y: part.at.1,
                w,
                h,
            };
            let found = images.id(&path).and_then(|id| Some((id, images.info(id)?)));
            let problem = match found {
                None => Some((
                    format!("{path} is not an image in the assets"),
                    codes::MISSING_FILE,
                )),
                Some((_, size)) if !size.holds(rect) => Some((
                    format!(
                        "{w}×{h} px at ({}, {}) is outside {path}, which is {}×{} px",
                        rect.x, rect.y, size.width, size.height
                    ),
                    codes::OUT_OF_RANGE,
                )),
                Some((image, _)) => {
                    let hides = part.hides.iter().map(|s| slot_of(s)).collect();
                    parts.insert(
                        name.clone(),
                        Part {
                            slot: slot_of(&part.slot),
                            image,
                            at: part.at,
                            hides,
                        },
                    );
                    None
                }
            };
            if let Some((message, code)) = problem {
                errors.push(
                    ContentError::new(file, format!("part \"{name}\": {message}")).with_code(code),
                );
            }
        }
        if !errors.is_empty() {
            return Err(errors);
        }
        let groups: Vec<ColourGroup> = self
            .colours
            .iter()
            .map(|(name, group)| ColourGroup {
                name: name.clone(),
                keys: group.keys.iter().filter_map(|k| self.key_grey(k)).collect(),
            })
            .collect();
        let kinds = self
            .kinds
            .keys()
            .map(|name| (name.clone(), self.flatten(name)))
            .collect();
        Ok(PartSet {
            size: self.size,
            slots: self.slots.clone(),
            keys: self.keys.iter().cloned().collect(),
            parts,
            groups,
            kinds,
        })
    }

    /// The grey of the key that stands for `name`.
    fn key_grey(&self, name: &str) -> Option<u8> {
        self.keys
            .iter()
            .find(|(_, k)| k == name)
            .map(|&(grey, _)| grey)
    }

    /// Kind `name` with its chain's lists and rows: each slot's and each
    /// group's from the nearest kind that names it, else empty or the
    /// set's. [`check`] refuses a cycle; one read without it stops at the
    /// first kind seen twice.
    fn flatten(&self, name: &str) -> Kind {
        let mut chain = Vec::new();
        let mut seen = BTreeSet::new();
        let mut at = Some(name);
        while let Some(kind) = at
            .filter(|k| seen.insert(*k))
            .and_then(|k| self.kinds.get(k))
        {
            chain.push(kind);
            at = kind.base.as_deref();
        }
        let slots = self
            .slots
            .iter()
            .map(|slot| {
                chain
                    .iter()
                    .find_map(|k| k.slots.get(slot))
                    .cloned()
                    .unwrap_or_default()
            })
            .collect();
        let rows = self
            .colours
            .iter()
            .map(|(group, def)| {
                let rows = chain
                    .iter()
                    .find_map(|k| k.colours.get(group))
                    .unwrap_or(&def.rows);
                rows.iter()
                    .map(|row| {
                        row.iter()
                            .map(|c| parse_hex(c).unwrap_or_default())
                            .collect()
                    })
                    .collect()
            })
            .collect();
        Kind { slots, rows }
    }
}

/// A part set file (`parts`, `.ron`), checked alone as
/// [`PartsDef::from_source`] says. Not in
/// [`Formats::standard`](crate::format::Formats::standard): a game that
/// uses part sets registers it, and resolves what it reads
/// ([`PartsDef::resolve`]) once its images are known.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PartsRon;

impl Format for PartsRon {
    type Asset = PartsDef;

    fn id(&self) -> &'static str {
        "parts"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ron"]
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<PartsDef, Vec<ContentError>> {
        let source = text(file, bytes)?;
        PartsDef::from_source(file.shown(), source)
    }
}

/// A part set a skin paints from, made by [`PartsDef::resolve`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PartSet {
    /// Every picture's width and height, in pixels.
    pub size: (u32, u32),
    /// The slots, in draw order, first undermost.
    pub slots: Vec<String>,
    /// What each key grey stands for.
    pub keys: BTreeMap<u8, String>,
    /// The parts, by name.
    pub parts: BTreeMap<String, Part>,
    /// The colour groups, in name order.
    groups: Vec<ColourGroup>,
    /// Each kind with its chain's lists and rows.
    kinds: BTreeMap<String, Kind>,
}

/// One part of a [`PartSet`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Part {
    /// Its slot, an index into [`PartSet::slots`].
    pub slot: usize,
    /// Its image.
    pub image: ImageId,
    /// Its top-left corner in the image; it is the set's size.
    pub at: (u32, u32),
    /// The slots it empties when shown, indexes into [`PartSet::slots`].
    pub hides: Vec<usize>,
}

/// A colour group, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ColourGroup {
    name: String,
    /// Its keys' greys, in the group's order.
    keys: Vec<u8>,
}

/// A kind with its chain's lists and rows.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Kind {
    /// Each slot's list, by slot; empty for a slot no kind names.
    slots: Vec<Vec<Option<String>>>,
    /// Each colour group's rows, by group.
    rows: Vec<Vec<Vec<[u8; 3]>>>,
}

/// What a person looks like: a part for each slot and a colour for each
/// key, made by [`PartSet::appearance`] and painted by the skin. Plain
/// data: a game works it out from the seed when it paints, and need not
/// keep it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Appearance {
    /// The kind it was rolled for.
    pub kind: String,
    /// The part in each slot, by slot, `None` for nothing.
    pub parts: Vec<Option<String>>,
    /// The colour of each key, by key grey. A key no group colours has
    /// none, and keeps its grey.
    pub colours: BTreeMap<u8, [u8; 3]>,
}

impl Appearance {
    /// The look of `kind` with `parts` (by slot) and `colours` (by key
    /// grey).
    #[must_use]
    pub const fn new(
        kind: String,
        parts: Vec<Option<String>>,
        colours: BTreeMap<u8, [u8; 3]>,
    ) -> Self {
        Self {
            kind,
            parts,
            colours,
        }
    }

    /// The FNV hash ([`crate::fnv`]) of its kind, parts and colours.
    pub fn fingerprint(&self) -> u64 {
        let mut hash = Hasher::new();
        hash.sized(self.kind.as_bytes());
        hash.word(self.parts.len() as u64);
        for part in &self.parts {
            match part {
                Some(name) => hash.sized(name.as_bytes()),
                None => hash.word(u64::MAX),
            }
        }
        for (&key, &[r, g, b]) in &self.colours {
            hash.bytes(&[key, r, g, b]);
        }
        hash.finish()
    }
}

/// Why a look can't be rolled ([`PartSet::appearance`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PartsError {
    /// The set has no kind of this name.
    #[error("the part set has no kind \"{0}\"")]
    UnknownKind(String),
    /// The set has no part of this name.
    #[error("the part set has no part \"{0}\"")]
    UnknownPart(String),
    /// Two worn parts go in one slot.
    #[error("\"{first}\" and \"{second}\" are both worn in slot \"{slot}\"")]
    TwoInOneSlot {
        /// The slot.
        slot: String,
        /// The part worn first.
        first: String,
        /// The other.
        second: String,
    },
}

/// The generator of `what`'s roll (`"slot"` or `"colour"`) called `name`
/// for `seed`: seeded by the FNV hash of the seed, then `what`'s and
/// `name`'s bytes, so each slot and group rolls on its own.
pub fn slot_rng(seed: u64, what: &str, name: &str) -> SimRng {
    let mut hash = Hasher::new();
    hash.word(seed);
    hash.bytes(what.as_bytes());
    hash.bytes(name.as_bytes());
    SimRng::new(hash.finish())
}

/// One of `list`, rolled with `rng`.
fn roll<'a, T>(rng: &mut SimRng, list: &'a [T]) -> Option<&'a T> {
    let n = u32::try_from(list.len()).unwrap_or(u32::MAX);
    list.get(usize::try_from(rng.roll_below(n)).unwrap_or(0))
}

impl PartSet {
    /// The names of its kinds.
    pub fn kinds(&self) -> impl Iterator<Item = &str> + '_ {
        self.kinds.keys().map(String::as_str)
    }

    /// The look of a `kind` rolled from `seed`, wearing `worn`: as the
    /// [module docs](self) say, in integers, the same on every platform.
    pub fn appearance(
        &self,
        kind: &str,
        seed: u64,
        worn: &[&str],
    ) -> Result<Appearance, PartsError> {
        let rolls = self
            .kinds
            .get(kind)
            .ok_or_else(|| PartsError::UnknownKind(kind.to_owned()))?;
        // 1. Each slot rolls one of its list.
        let mut parts: Vec<Option<String>> = self
            .slots
            .iter()
            .zip(&rolls.slots)
            .map(|(slot, list)| {
                roll(&mut slot_rng(seed, "slot", slot), list)
                    .cloned()
                    .flatten()
            })
            .collect();
        // 2. Each colour group rolls a row.
        let mut colours = BTreeMap::new();
        for (group, rows) in self.groups.iter().zip(&rolls.rows) {
            if let Some(row) = roll(&mut slot_rng(seed, "colour", &group.name), rows) {
                colours.extend(group.keys.iter().copied().zip(row.iter().copied()));
            }
        }
        // 3. Each worn part takes its slot.
        let mut taken: BTreeMap<usize, &str> = BTreeMap::new();
        for &name in worn {
            let part = self
                .parts
                .get(name)
                .ok_or_else(|| PartsError::UnknownPart(name.to_owned()))?;
            if let Some(&first) = taken.get(&part.slot)
                && first != name
            {
                return Err(PartsError::TwoInOneSlot {
                    slot: self.slots[part.slot].clone(),
                    first: first.to_owned(),
                    second: name.to_owned(),
                });
            }
            taken.insert(part.slot, name);
            parts[part.slot] = Some(name.to_owned());
        }
        // 4. From the top down, a part still shown empties what it hides.
        for slot in (0..parts.len()).rev() {
            let Some(part) = parts[slot].as_deref().and_then(|p| self.parts.get(p)) else {
                continue;
            };
            for &hidden in &part.hides {
                parts[hidden] = None;
            }
        }
        Ok(Appearance::new(kind.to_owned(), parts, colours))
    }

    /// What is wrong with its parts' pixels, `pixels` giving an image's:
    /// a part whose image has no pixels, or fewer than its rectangle
    /// needs; and a drawn grey within [`NEAR_KEY`] of a key but not on it
    /// (a slip of the brush that would show grey in the game), naming the
    /// image and the pixel, once each.
    pub fn check_pixels<'a>(
        &self,
        pixels: impl Fn(ImageId) -> Option<&'a Image>,
    ) -> Vec<ContentError> {
        let mut errors = Vec::new();
        let mut seen = BTreeSet::new();
        let (w, h) = self.size;
        for (name, part) in &self.parts {
            let file = part.image.path();
            let rect = ImageRect {
                x: part.at.0,
                y: part.at.1,
                w,
                h,
            };
            let Some(image) = pixels(part.image).filter(|i| i.info().holds(rect) && valid(i))
            else {
                errors.push(
                    ContentError::new(
                        file,
                        format!("part \"{name}\": the image's pixels aren't there"),
                    )
                    .with_code(codes::MISSING_FILE),
                );
                continue;
            };
            for y in rect.y..rect.y + h {
                for x in rect.x..rect.x + w {
                    let Some((grey, key, stands)) = image.pixel(x, y).and_then(|p| self.slip(p))
                    else {
                        continue;
                    };
                    if seen.insert((file, x, y)) {
                        errors.push(
                            ContentError::new(
                                file,
                                format!(
                                    "pixel ({x}, {y}): grey {grey} is {} from key {key} (\"{stands}\") but not on it",
                                    key.abs_diff(grey)
                                ),
                            )
                            .with_code(codes::OUT_OF_RANGE),
                        );
                    }
                }
            }
        }
        errors
    }
}

impl PartSet {
    /// If `pixel` is a drawn grey near a key but not on it: its grey, the
    /// nearest key's, and what that key stands for.
    fn slip(&self, [red, green, blue, alpha]: [u8; 4]) -> Option<(u8, u8, &str)> {
        if alpha < DRAWN_ALPHA || red != green || green != blue || self.keys.contains_key(&red) {
            return None;
        }
        self.keys
            .iter()
            .filter(|&(&k, _)| k.abs_diff(red) <= NEAR_KEY)
            .min_by_key(|&(&k, _)| k.abs_diff(red))
            .map(|(&key, stands)| (red, key, stands.as_str()))
    }
}

/// Whether `image` has as many bytes as its size says.
fn valid(image: &Image) -> bool {
    let ImageInfo { width, height } = image.info();
    u64::try_from(image.rgba.len()).is_ok_and(|n| n == u64::from(width) * u64::from(height) * 4)
}

/// A map read into, and written from, its entries in order, so a key
/// written twice is seen ([`check`]).
mod entries {
    use crate::prelude::*;

    use core::fmt;
    use core::marker::PhantomData;

    use serde::de::{MapAccess, Visitor};
    use serde::ser::SerializeMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<K: Serialize, V: Serialize, S: Serializer>(
        entries: &[(K, V)],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(entries.len()))?;
        for (k, v) in entries {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }

    pub fn deserialize<'de, K: Deserialize<'de>, V: Deserialize<'de>, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<(K, V)>, D::Error> {
        struct Entries<K, V>(PhantomData<(K, V)>);
        impl<'de, K: Deserialize<'de>, V: Deserialize<'de>> Visitor<'de> for Entries<K, V> {
            type Value = Vec<(K, V)>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a map")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry()? {
                    out.push(entry);
                }
                Ok(out)
            }
        }
        deserializer.deserialize_map(Entries(PhantomData))
    }
}
