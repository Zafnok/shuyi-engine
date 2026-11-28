//! A tileset's terrain tiles as a file writes them: [`TerrainTilesDef`],
//! checked alone ([`TerrainTilesDef::problems`], against a game's
//! [`TileRules`]) and then resolved into [`TerrainTiles`] against the
//! image table and the game's terrains ([`TerrainTilesDef::resolve`]).
//! As a format (ADR-0007): [`TerrainTilesRon`].
use crate::prelude::*;

use alloc::collections::{BTreeMap, BTreeSet};
use core::ops::RangeInclusive;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use shuyi_core::TerrainId;

use super::{
    CornerLayer, Layer, Look, MIX_IN, MIX_OUT, MIXES, TerrainTiles, TileLayer, mix_name, parse_mix,
};
use crate::error::{ContentError, codes};
use crate::format::{AssetKind, AssetPath, DecodeCx, TextFormat};
use crate::grid_map::DEFAULT_TILES;
use crate::image::{ImageId, ImageInfo, ImageRect, ImageTable};
use crate::ron::parse_ron;

/// Cells by name: a look's tile of each terrain.
type Cells = BTreeMap<String, (u32, u32)>;

/// Pictures a map may place, by name.
type Props = BTreeMap<String, PropAt>;

impl AssetKind for TerrainTilesDef {
    const NAME: &'static str = "terrain tiles";
}

/// A tileset's terrain tiles as written. Every picture is a `(column,
/// row)` cell of `tile_px` cells from the image's top-left. Build it with
/// [`TerrainTilesDef::new`] (ADR-0039), or read it with
/// [`TerrainTilesRon`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct TerrainTilesDef {
    /// The image the tiles are in, by its path in the bundle.
    pub image: String,
    /// A tile's size, across × down, in pixels.
    pub tile_px: (u32, u32),
    /// The own look's tile of each terrain, by the terrain's name.
    pub terrain: BTreeMap<String, (u32, u32)>,
    /// The own look's layers, the first undermost. None if left out.
    #[serde(default)]
    pub layers: Vec<LayerDef>,
    /// The own look's pictures a map may place on its tiles, by the name
    /// the map gives ([`Look::props`]). None if left out.
    #[serde(default)]
    pub props: BTreeMap<String, PropAt>,
    /// The other looks, by the name a map's look gives. None if left out.
    #[serde(default)]
    pub looks: BTreeMap<String, LookDef>,
}

/// A look of [`TerrainTilesDef::looks`] as written. Build it with
/// [`LookDef::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct LookDef {
    /// Each terrain's tile, by the terrain's name.
    pub terrain: BTreeMap<String, (u32, u32)>,
    /// The layers painted over the tiles. None if left out.
    #[serde(default)]
    pub layers: Vec<LayerDef>,
    /// The pictures a map may place on its tiles, by name
    /// ([`Look::props`]): the own look's are not this look's. None if left
    /// out.
    #[serde(default)]
    pub props: BTreeMap<String, PropAt>,
}

/// A picture a map may place, as written: a cell of the image and a size.
/// Build it with [`PropAt::new`] (ADR-0039).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct PropAt {
    /// The picture's cell.
    pub at: (u32, u32),
    /// The picture's size in pixels, from its cell's top-left. Left out,
    /// `tile_px`.
    #[serde(
        default,
        deserialize_with = "written",
        serialize_with = "unwritten",
        skip_serializing_if = "Option::is_none"
    )]
    pub size: Option<(u32, u32)>,
}

impl PropAt {
    /// The picture at cell `at`, a tile big.
    #[must_use]
    pub const fn new(at: (u32, u32)) -> Self {
        Self { at, size: None }
    }

    /// This picture, `size` pixels from its cell's top-left.
    #[must_use]
    pub const fn with_size(self, size: (u32, u32)) -> Self {
        Self {
            size: Some(size),
            ..self
        }
    }
}

/// A layer as written: a [`Layer`] with its terrains by name and its
/// pictures by cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum LayerDef {
    /// A [`CornerLayer`].
    Corners {
        /// The terrains in the layer.
        of: Vec<String>,
        /// The picture of each mix of corners that has one.
        tiles: Vec<CornersAt>,
        /// Each picture's size in pixels, from its cell's top-left. Left
        /// out, `tile_px`.
        #[serde(
            default,
            deserialize_with = "written",
            serialize_with = "unwritten",
            skip_serializing_if = "Option::is_none"
        )]
        size: Option<(u32, u32)>,
        /// [`CornerLayer::offset`]. Left out, `(0, 0)`.
        #[serde(default)]
        offset: (i32, i32),
        /// [`CornerLayer::skip`], by name. Left out, none.
        #[serde(default)]
        skip: Vec<String>,
        /// [`CornerLayer::image`]: the image its pictures are in, by its
        /// path in the bundle (a made corner sheet, `"tiles/water-corners.png"`),
        /// its cells `size` (or `tile_px`) each. Left out, the tileset's.
        #[serde(
            default,
            deserialize_with = "written",
            serialize_with = "unwritten",
            skip_serializing_if = "Option::is_none"
        )]
        image: Option<String>,
    },
    /// A [`TileLayer`].
    Tiles {
        /// The terrains that get the picture.
        of: Vec<String>,
        /// The picture's cell.
        at: (u32, u32),
        /// [`TileLayer::beside`], by name. Left out, none.
        #[serde(default)]
        beside: Vec<String>,
        /// The picture of each mix of sides that has its own.
        #[serde(default)]
        sides: Vec<SidesAt>,
    },
}

/// The picture of one mix of a corner layer, as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CornersAt {
    /// The mix: four of [`MIX_IN`] and [`MIX_OUT`] (`"..##"`).
    pub corners: String,
    /// The picture's cell.
    pub at: (u32, u32),
}

/// The picture of one mix of a tile layer's sides, as written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SidesAt {
    /// The mix: four of [`MIX_IN`] and [`MIX_OUT`] (`".#.#"`).
    pub sides: String,
    /// The picture's cell.
    pub at: (u32, u32),
}

/// A field that may be left out, written bare: `Some` of what is written.
fn written<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}

/// A field [`written`] reads: what it holds, bare.
#[allow(clippy::ref_option)] // serde gives the field as it is
fn unwritten<S: Serializer, T: Serialize>(value: &Option<T>, s: S) -> Result<S::Ok, S::Error> {
    match value {
        Some(value) => value.serialize(s),
        None => s.serialize_none(),
    }
}

/// What a game's terrain tiles may be: the looks its maps may name, and
/// the sides a tile or a picture may have. Build it with
/// [`TileRules::new`] (ADR-0039).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TileRules<'a> {
    /// The looks a map may name ([`MapFormat::looks`](crate::grid_map::MapFormat::looks)).
    pub looks: &'a [&'a str],
    /// Each side of `tile_px` and of a corner layer's `size`, in pixels.
    pub side_px: RangeInclusive<u32>,
}

impl<'a> TileRules<'a> {
    /// Tiles whose maps may name the looks `looks`, each side of a tile or
    /// a picture in `side_px`.
    #[must_use]
    pub const fn new(looks: &'a [&'a str], side_px: RangeInclusive<u32>) -> Self {
        Self { looks, side_px }
    }
}

impl LookDef {
    /// The look of the tiles `terrain`, with no layer and no picture to
    /// place.
    #[must_use]
    pub const fn new(terrain: BTreeMap<String, (u32, u32)>) -> Self {
        Self {
            terrain,
            layers: Vec::new(),
            props: BTreeMap::new(),
        }
    }
}

impl TerrainTilesDef {
    /// The tiles `terrain` of `image`, `tile_px` each, with no layer, no
    /// picture to place and no other look.
    #[must_use]
    pub const fn new(
        image: String,
        tile_px: (u32, u32),
        terrain: BTreeMap<String, (u32, u32)>,
    ) -> Self {
        Self {
            image,
            tile_px,
            terrain,
            layers: Vec::new(),
            props: BTreeMap::new(),
            looks: BTreeMap::new(),
        }
    }

    /// Every look: its name (`None` for the own one), its tiles, its
    /// layers and its pictures to place.
    fn all_looks(&self) -> impl Iterator<Item = (Option<&str>, &Cells, &[LayerDef], &Props)> {
        let own = (None, &self.terrain, self.layers.as_slice(), &self.props);
        let looks = self.looks.iter().map(|(name, look)| {
            let name = Some(name.as_str());
            (name, &look.terrain, &*look.layers, &look.props)
        });
        core::iter::once(own).chain(looks)
    }

    /// What is wrong with the file alone, under `rules`: a side of
    /// `tile_px` or of a corner layer's `size` outside
    /// [`side_px`](TileRules::side_px); a look's name not one of
    /// [`looks`](TileRules::looks), or [`DEFAULT_TILES`] (the own look,
    /// written at the top); a layer whose `of` is empty; a mix not four of
    /// [`MIX_IN`] and [`MIX_OUT`], or listed twice; a corner layer with a
    /// picture for mix 0, or with none; a picture to place with an empty
    /// name, or a side of its `size` 0 or outside `side_px`. Each names its
    /// look and layer or picture (`look "indoor": layer 2: …`,
    /// `look "winter": prop "lamp": …`).
    pub fn problems(&self, rules: &TileRules<'_>) -> Vec<String> {
        let mut problems = Vec::new();
        problems.extend(side_problem("tile_px", self.tile_px, &rules.side_px));
        for name in self.looks.keys() {
            problems.extend(name_problem(name, rules.looks));
        }
        for (name, _, layers, props) in self.all_looks() {
            for (number, layer) in (1..).zip(layers) {
                let what = format!("{}layer {number}", prefix(name));
                layer_problems(&what, layer, &rules.side_px, &mut problems);
            }
            for (prop, at) in props {
                let what = format!("{}prop \"{prop}\"", prefix(name));
                prop_problems(&what, prop, at, &rules.side_px, &mut problems);
            }
        }
        problems
    }

    /// The tiles as a skin paints them, with `file` (as shown) in each
    /// error: the image looked up in `images`, each terrain named looked up
    /// in `terrains`. Every problem: the image, or a corner layer's own
    /// `image`, not in the table; a picture outside its image (a corner
    /// layer's and a picture to place's at its `size`); a terrain name (in
    /// `terrain`, `of`, `beside`, `skip`) not in `terrains`; a terrain of
    /// `terrains` with no tile in a look. Run on a file
    /// [`problems`](Self::problems) passed: a mix it reported is left out.
    pub fn resolve(
        &self,
        file: &str,
        images: &ImageTable,
        terrains: &BTreeMap<String, TerrainId>,
    ) -> Result<TerrainTiles, Vec<ContentError>> {
        let image = images.id(&self.image);
        let mut errors = Vec::new();
        if image.is_none() {
            let message = format!(
                "image \"{}\" is not in the bundle's image table",
                self.image
            );
            errors.push(ContentError::new(file, message).with_code(codes::MISSING_FILE));
        }
        let mut resolving = Resolving {
            file,
            images,
            image: image.and_then(|id| images.info(id)),
            tile_px: self.tile_px,
            terrains,
            errors,
        };
        let look = resolving.look(None, &self.terrain, &self.layers, &self.props);
        let looks: BTreeMap<String, Look> = self
            .looks
            .iter()
            .map(|(name, look)| {
                let resolved = resolving.look(Some(name), &look.terrain, &look.layers, &look.props);
                (name.clone(), resolved)
            })
            .collect();
        for (name, tiles, _, _) in self.all_looks() {
            let own = prefix(name);
            for terrain in terrains.keys().filter(|t| !tiles.contains_key(*t)) {
                let message = format!("{own}terrain \"{terrain}\" has no tile");
                resolving.errors.push(ContentError::new(file, message));
            }
        }
        match image {
            Some(image) if resolving.errors.is_empty() => {
                let mut tiles = TerrainTiles::new(image, self.tile_px, look);
                tiles.looks = looks;
                Ok(tiles)
            }
            _ => Err(resolving.errors),
        }
    }
}

/// What a look called `name` is called in a problem: nothing for the
/// own one.
fn prefix(name: Option<&str>) -> String {
    name.map_or_else(String::new, |n| format!("look \"{n}\": "))
}

/// The problem with size `(w, h)`, called `name`, if a side isn't in
/// `side_px`.
fn side_problem(name: &str, (w, h): (u32, u32), side_px: &RangeInclusive<u32>) -> Option<String> {
    let (lo, hi) = (side_px.start(), side_px.end());
    (!side_px.contains(&w) || !side_px.contains(&h))
        .then(|| format!("{name} is ({w}, {h}); each side must be {lo} to {hi} px"))
}

/// The problem with calling a look `name`, if any: only a look of `looks`
/// that isn't the own one.
fn name_problem(name: &str, looks: &[&str]) -> Option<String> {
    if name == DEFAULT_TILES {
        return Some(format!(
            "look \"{name}\" is the tileset's own `terrain` and `layers`: write it there"
        ));
    }
    (!looks.contains(&name)).then(|| {
        let known: Vec<String> = looks.iter().map(|l| format!("\"{l}\"")).collect();
        format!(
            "look \"{name}\" is not a look a map can name ({})",
            known.join(", ")
        )
    })
}

/// The problems of `layer` alone, called `what`, pushed onto `problems`.
fn layer_problems(
    what: &str,
    layer: &LayerDef,
    side_px: &RangeInclusive<u32>,
    problems: &mut Vec<String>,
) {
    let (LayerDef::Corners { of, .. } | LayerDef::Tiles { of, .. }) = layer;
    if of.is_empty() {
        problems.push(format!("{what}: `of` names no terrain"));
    }
    match layer {
        LayerDef::Corners { tiles, size, .. } => {
            if let Some(size) = size {
                problems.extend(side_problem(&format!("{what}: size"), *size, side_px));
            }
            let written = tiles.iter().map(|t| t.corners.as_str());
            let mixes = mix_problems(what, "corners", written, problems);
            if mixes.contains(&0) {
                problems.push(format!(
                    "{what}: corners \"{}\" has no corner in the layer, so there is nothing to \
                     draw",
                    mix_name(0)
                ));
            }
            if mixes.is_empty() {
                problems.push(format!("{what}: it has no pictures"));
            }
        }
        LayerDef::Tiles { sides, .. } => {
            let written = sides.iter().map(|s| s.sides.as_str());
            mix_problems(what, "sides", written, problems);
        }
    }
}

/// The problems of the picture to place `at`, called `name` and as a whole
/// `what`, pushed onto `problems`.
fn prop_problems(
    what: &str,
    name: &str,
    at: &PropAt,
    side_px: &RangeInclusive<u32>,
    problems: &mut Vec<String>,
) {
    if name.is_empty() {
        problems.push(format!("{what}: its name is empty"));
    }
    match at.size {
        Some((w, h)) if w == 0 || h == 0 => problems.push(format!(
            "{what}: size is ({w}, {h}); a side of 0 px draws nothing"
        )),
        Some(size) => problems.extend(side_problem(&format!("{what}: size"), size, side_px)),
        None => {}
    }
}

/// The mixes `written` (each called a `kind`) that are mixes, pushing a
/// problem for each that isn't or is listed twice.
fn mix_problems<'w>(
    what: &str,
    kind: &str,
    written: impl Iterator<Item = &'w str>,
    problems: &mut Vec<String>,
) -> BTreeSet<u8> {
    let mut mixes = BTreeSet::new();
    for text in written {
        match parse_mix(text) {
            None => problems.push(format!(
                "{what}: {kind} \"{text}\" must be four of `{MIX_IN}` (in) and `{MIX_OUT}` (out)"
            )),
            Some(m) if !mixes.insert(m) => {
                problems.push(format!("{what}: {kind} \"{text}\" is listed twice"));
            }
            Some(_) => {}
        }
    }
    mixes
}

/// A [`TerrainTilesDef`] being resolved.
struct Resolving<'a> {
    file: &'a str,
    images: &'a ImageTable,
    /// The image's size, if the table has it.
    image: Option<ImageInfo>,
    tile_px: (u32, u32),
    terrains: &'a BTreeMap<String, TerrainId>,
    errors: Vec<ContentError>,
}

impl Resolving<'_> {
    /// The `size` pixels from the top-left of cell `at`, reported (as
    /// `what`) if they lie outside the tileset's image.
    fn rect(&mut self, what: &str, at: (u32, u32), size: (u32, u32)) -> ImageRect {
        self.rect_in(self.image, what, at, size)
    }

    /// The `size` pixels from the top-left of cell `at`, reported (as
    /// `what`) if they lie outside the image of size `image`, if known.
    fn rect_in(
        &mut self,
        image: Option<ImageInfo>,
        what: &str,
        at: (u32, u32),
        (w, h): (u32, u32),
    ) -> ImageRect {
        let rect = ImageRect {
            w,
            h,
            ..ImageRect::cell(at, self.tile_px, (0, 0))
        };
        if let Some(info) = image
            && !info.holds(rect)
        {
            let ImageRect { x, y, .. } = rect;
            let (iw, ih) = (info.width, info.height);
            let message =
                format!("{what}: its {w}×{h} px at ({x}, {y}) lies outside the {iw}×{ih} px image");
            let error = ContentError::new(self.file, message).with_code(codes::OUT_OF_RANGE);
            self.errors.push(error);
        }
        rect
    }

    /// The image at `path` of a layer called `what`, reported if the table
    /// lacks it.
    fn layer_image(&mut self, what: &str, path: &str) -> Option<ImageId> {
        let id = self.images.id(path);
        if id.is_none() {
            let message = format!("{what}: image \"{path}\" is not in the bundle's image table");
            let error = ContentError::new(self.file, message).with_code(codes::MISSING_FILE);
            self.errors.push(error);
        }
        id
    }

    /// The terrain called `name`, reported (as `what`) if the game has
    /// none.
    fn terrain(&mut self, what: &str, name: &str) -> Option<TerrainId> {
        let id = self.terrains.get(name).copied();
        if id.is_none() {
            let message = format!("{what}terrain \"{name}\" is not one of the game's terrains");
            let error = ContentError::new(self.file, message).with_code(codes::UNKNOWN_NAME);
            self.errors.push(error);
        }
        id
    }

    /// The terrains called `names`, each the game lacks reported as
    /// `what`.
    fn terrains(&mut self, what: &str, names: &[String]) -> BTreeSet<TerrainId> {
        let what = format!("{what}: ");
        names
            .iter()
            .filter_map(|name| self.terrain(&what, name))
            .collect()
    }

    /// The pictures of the mixes `entries` (each a mix as written, called
    /// a `kind`, and its cell), each `size` pixels of the image of size
    /// `image`, by mix.
    fn mixes<'e>(
        &mut self,
        image: Option<ImageInfo>,
        what: &str,
        kind: &str,
        size: (u32, u32),
        entries: impl Iterator<Item = (&'e str, (u32, u32))>,
    ) -> [Option<ImageRect>; MIXES] {
        let mut tiles = [None; MIXES];
        for (text, at) in entries {
            if let Some(m) = parse_mix(text) {
                let rect = self.rect_in(image, &format!("{what}, {kind} \"{text}\""), at, size);
                tiles[usize::from(m)] = Some(rect);
            }
        }
        tiles
    }

    /// Layer `layer`, called `what`.
    fn layer(&mut self, what: &str, layer: &LayerDef) -> Layer {
        match layer {
            LayerDef::Corners {
                of,
                tiles,
                size,
                offset,
                skip,
                image,
            } => {
                let of = self.terrains(what, of);
                let entries = tiles.iter().map(|t| (t.corners.as_str(), t.at));
                let size = size.unwrap_or(self.tile_px);
                let own = image.as_deref().map(|path| self.layer_image(what, path));
                let info = match own {
                    Some(id) => id.and_then(|id| self.images.info(id)),
                    None => self.image,
                };
                let tiles = self.mixes(info, what, "corners", size, entries);
                let mut layer = CornerLayer::new(of, tiles);
                layer.skip = self.terrains(&format!("{what}: skip"), skip);
                layer.offset = *offset;
                layer.image = own.flatten();
                Layer::Corners(layer)
            }
            LayerDef::Tiles {
                of,
                at,
                beside,
                sides,
            } => {
                let of = self.terrains(what, of);
                let tile = self.rect(what, *at, self.tile_px);
                let entries = sides.iter().map(|s| (s.sides.as_str(), s.at));
                let sides = self.mixes(self.image, what, "sides", self.tile_px, entries);
                let beside = self.terrains(what, beside);
                Layer::Tiles(TileLayer {
                    of,
                    tile,
                    beside,
                    sides,
                })
            }
        }
    }

    /// The look called `name` (`None`: the own one) of the tiles `terrain`,
    /// the layers `layers` and the pictures to place `props`.
    fn look(
        &mut self,
        name: Option<&str>,
        terrain: &Cells,
        layers: &[LayerDef],
        props: &Props,
    ) -> Look {
        let own = prefix(name);
        let mut tiles = BTreeMap::new();
        for (terrain, &at) in terrain {
            let rect = self.rect(&format!("{own}terrain \"{terrain}\""), at, self.tile_px);
            if let Some(id) = self.terrain(&own, terrain) {
                tiles.insert(id, rect);
            }
        }
        let layers = (1..)
            .zip(layers)
            .map(|(number, layer)| self.layer(&format!("{own}layer {number}"), layer))
            .collect();
        let mut look = Look::new(tiles, layers);
        for (prop, at) in props {
            let size = at.size.unwrap_or(self.tile_px);
            let rect = self.rect(&format!("{own}prop \"{prop}\""), at.at, size);
            look.props.insert(prop.clone(), rect);
        }
        look
    }
}

/// A tileset's terrain tiles file (`terrain-tiles`, `.ron`): a
/// [`TerrainTilesDef`] in RON, checked by
/// [`problems`](TerrainTilesDef::problems) against the game's
/// [`TileRules`]. Not in [`Formats::standard`](crate::format::Formats::standard):
/// a game registers it with its rules, and resolves what it reads
/// ([`TerrainTilesDef::resolve`]) once its images and terrains are known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainTilesRon {
    rules: TileRules<'static>,
}

impl TerrainTilesRon {
    /// Terrain tiles files checked against `rules`.
    #[must_use]
    pub const fn new(rules: TileRules<'static>) -> Self {
        Self { rules }
    }
}

impl TextFormat for TerrainTilesRon {
    type Asset = TerrainTilesDef;

    fn id(&self) -> &'static str {
        "terrain-tiles"
    }

    fn read(
        &self,
        file: &AssetPath,
        text: &str,
        _cx: &DecodeCx<'_>,
    ) -> Result<TerrainTilesDef, Vec<ContentError>> {
        let shown = file.shown();
        let def: TerrainTilesDef = parse_ron(shown, text).map_err(|e| vec![e])?;
        let problems = def.problems(&self.rules);
        if problems.is_empty() {
            Ok(def)
        } else {
            Err(problems
                .into_iter()
                .map(|message| ContentError::new(shown, message))
                .collect())
        }
    }
}

#[cfg(test)]
mod tests;
