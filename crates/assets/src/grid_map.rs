//! The `.map` text format of a grid map: a RON header, a `---` line, then
//! one character per tile, top row first.
//!
//! ```text
//! (
//!     name: "Crossing",
//!     legend: { '.': "plain", 'T': "forest", '~': "water" },
//!     features: { (2, 0): Sign("Turn back") },
//!     look: (tiles: "indoor", light: "dusk"),
//! )
//! ---
//! ..T
//! ~~.
//! ```
//!
//! The header has the map's `name`, its `legend` (tile character →
//! terrain id), its `features` and its `look`; the last two may be left
//! out. A game says what a feature is (the [`MapFeature`] hook: its own
//! serde type, written after the tile's `(x, y)`), what a terrain id
//! resolves to, and which tile looks there are ([`MapFormat`]). Every
//! problem is reported together, each with its line and column.
//!
//! The look is kept beside the tiles, for a skin to paint them by; the
//! rules never need it. As a format (ADR-0007): [`MapText`].

use crate::prelude::*;

use alloc::collections::BTreeMap;
use core::fmt;
use core::marker::PhantomData;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use shuyi_core::{Grid, Pos};

use crate::error::ContentError;
use crate::format::{AssetKind, AssetPath, DecodeCx, Format, TEXT_MAX_BYTES, text};
use crate::ron::parse_ron;

/// The line separating the RON header from the tile rows.
pub const SEPARATOR: &str = "---";
/// The tile look of a map that names none.
pub const DEFAULT_TILES: &str = "outdoor";
/// The light of a map that names none.
pub const DEFAULT_LIGHT: &str = "noon";

/// What a game's maps may be: the tile looks there are, and how big a map
/// may be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapFormat<'a> {
    /// The sets of terrain pictures a map may name in its `look`.
    /// [`DEFAULT_TILES`] should be one of them.
    pub looks: &'a [&'a str],
    /// Largest width and height, in tiles.
    pub max_size: usize,
}

/// A game's tile feature, as written in a map header after the tile's
/// `(x, y)` (a shop, a chest, a sign): the hook through which a game adds
/// its own things to the format.
pub trait MapFeature: Serialize + DeserializeOwned {
    /// A problem visible in the file alone, if the feature has one.
    fn problem(&self) -> Option<String> {
        None
    }
}

/// The feature of a game whose maps have none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoFeature {}

impl MapFeature for NoFeature {}

/// How a map looks, as far as its file says: which pictures a skin that
/// has pictures paints its terrain with, and in which light. A skin
/// without them ignores it. Look data only: the rules never see it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MapLook {
    /// Which set of terrain pictures: one of the game's
    /// [`looks`](MapFormat::looks).
    #[serde(default = "default_tiles")]
    pub tiles: String,
    /// Which light the map is in. [`DEFAULT_LIGHT`] unless the file says.
    #[serde(default = "default_light", skip_serializing_if = "is_default_light")]
    pub light: String,
}

fn default_tiles() -> String {
    DEFAULT_TILES.to_owned()
}

fn default_light() -> String {
    DEFAULT_LIGHT.to_owned()
}

#[allow(clippy::ptr_arg)] // serde gives the field as it is
fn is_default_light(light: &String) -> bool {
    light == DEFAULT_LIGHT
}

impl Default for MapLook {
    fn default() -> Self {
        Self {
            tiles: default_tiles(),
            light: default_light(),
        }
    }
}

impl MapLook {
    /// Whether it is the look of a map that names none: not written when
    /// a map is printed.
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// This look in the light `light`, if one is given: a battle's own
    /// light instead of its map's.
    #[must_use]
    pub fn lit(mut self, light: Option<&str>) -> Self {
        if let Some(light) = light {
            light.clone_into(&mut self.light);
        }
        self
    }
}

/// A map's legend: tile character → terrain id as written, and what each
/// resolved to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Legend<T> {
    /// Character → terrain id, as written in the file.
    pub names: BTreeMap<char, String>,
    /// Character → resolved terrain.
    pub terrains: BTreeMap<char, T>,
}

impl<T> Default for Legend<T> {
    fn default() -> Self {
        Self {
            names: BTreeMap::new(),
            terrains: BTreeMap::new(),
        }
    }
}

impl<T: Copy + PartialEq> Legend<T> {
    /// Builds a legend from `char → terrain id` pairs, resolving each id
    /// with `terrain`; `None` if any id is unknown.
    pub fn resolve(
        names: BTreeMap<char, String>,
        terrain: impl Fn(&str) -> Option<T>,
    ) -> Option<Self> {
        let terrains = names
            .iter()
            .map(|(&c, id)| Some((c, terrain(id)?)))
            .collect::<Option<_>>()?;
        Some(Self { names, terrains })
    }

    /// The first (lowest) character standing for terrain `t`.
    pub fn char_for(&self, t: T) -> Option<char> {
        self.terrains
            .iter()
            .find(|&(_, &u)| u == t)
            .map(|(&c, _)| c)
    }
}

/// A map read from a `.map` file: its tiles and features, the legend it
/// was written with (for printing it back) and its look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridMap<T, F> {
    /// The map's display name.
    pub name: String,
    /// The terrain of each tile.
    pub tiles: Grid<T>,
    /// The game's features, by tile.
    pub features: BTreeMap<Pos, F>,
    /// The legend from the file.
    pub legend: Legend<T>,
    /// How the map looks.
    pub look: MapLook,
}

impl<T: 'static, F: 'static> AssetKind for GridMap<T, F> {
    const NAME: &'static str = "map";
}

/// What a terrain id resolves to, in a [`MapText`].
type Terrain<T> = Box<dyn Fn(&str) -> Option<T> + Send + Sync>;

/// The `.map` format (`map`, `.map`) of a game's maps, read as [`parse`]
/// says: the game's [`MapFormat`], what a legend's terrain id resolves to,
/// and its features `F`.
pub struct MapText<T, F = NoFeature> {
    format: MapFormat<'static>,
    terrain: Terrain<T>,
    feature: PhantomData<fn() -> F>,
}

impl<T, F> MapText<T, F> {
    /// Maps of `format`, their legends' terrain ids resolved by `terrain`.
    pub fn new(
        format: MapFormat<'static>,
        terrain: impl Fn(&str) -> Option<T> + Send + Sync + 'static,
    ) -> Self {
        Self {
            format,
            terrain: Box::new(terrain),
            feature: PhantomData,
        }
    }
}

impl<T, F> fmt::Debug for MapText<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MapText")
            .field("format", &self.format)
            .finish_non_exhaustive()
    }
}

impl<T: Copy + Default + 'static, F: MapFeature + 'static> Format for MapText<T, F> {
    type Asset = GridMap<T, F>;

    fn id(&self) -> &'static str {
        "map"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["map"]
    }

    fn max_bytes(&self) -> usize {
        TEXT_MAX_BYTES
    }

    fn decode(
        &self,
        file: &AssetPath,
        bytes: &[u8],
        _cx: &DecodeCx<'_>,
    ) -> Result<GridMap<T, F>, Vec<ContentError>> {
        parse(
            file.shown(),
            text(file, bytes)?,
            &self.format,
            &self.terrain,
        )
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(serialize = "F: Serialize", deserialize = "F: DeserializeOwned"))]
struct Header<F> {
    name: String,
    legend: BTreeMap<char, String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    features: BTreeMap<(i32, i32), F>,
    #[serde(default, skip_serializing_if = "MapLook::is_default")]
    look: MapLook,
}

/// Parses map `source` (errors attributed to `file`), resolving legend
/// terrain ids with `terrain`. Reports every problem with its line and
/// column.
pub fn parse<T, F>(
    file: &str,
    source: &str,
    format: &MapFormat<'_>,
    terrain: impl Fn(&str) -> Option<T>,
) -> Result<GridMap<T, F>, Vec<ContentError>>
where
    T: Copy + Default,
    F: MapFeature,
{
    let lines: Vec<&str> = source
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    let Some(sep) = lines.iter().position(|&l| l == SEPARATOR) else {
        return Err(vec![ContentError::new(
            file,
            format!("missing the \"{SEPARATOR}\" line between the header and the tiles"),
        )]);
    };
    let header_text = lines[..sep].join("\n");
    let header: Header<F> = parse_ron(file, &header_text).map_err(|e| vec![e])?;

    let mut errors = Vec::new();
    let mut terrains = BTreeMap::new();
    for (&c, id) in &header.legend {
        if let Some(t) = terrain(id) {
            terrains.insert(c, t);
        } else {
            let (line, col) = legend_position(&lines[..sep], c, id);
            errors.push(
                ContentError::new(file, format!("legend '{c}': unknown terrain \"{id}\""))
                    .at(line, Some(col)),
            );
        }
    }

    let tiles_look = header.look.tiles.as_str();
    if !format.looks.contains(&tiles_look) {
        let known: Vec<String> = format.looks.iter().map(|l| format!("\"{l}\"")).collect();
        let message = format!(
            "look: tiles \"{tiles_look}\" is not a look; the looks are {}",
            known.join(", ")
        );
        let (line, col) =
            header_position(&lines[..sep], &format!("\"{tiles_look}\"")).unwrap_or((1, 1));
        errors.push(ContentError::new(file, message).at(line, Some(col)));
    }

    let rows = Rows {
        file,
        legend: &header.legend,
        terrains: &terrains,
        max_size: format.max_size,
    };
    let tiles = rows.parse(&lines[sep + 1..], sep + 1, &mut errors);
    let mut features = BTreeMap::new();
    for ((x, y), feature) in header.features {
        let pos = Pos::new(x, y);
        let at = || format!("feature at ({x}, {y})");
        if tiles.as_ref().is_some_and(|t| t.get(pos).is_none()) {
            errors.push(ContentError::new(
                file,
                format!("{}: outside the map", at()),
            ));
        }
        if let Some(problem) = feature.problem() {
            errors.push(ContentError::new(file, format!("{}: {problem}", at())));
        }
        features.insert(pos, feature);
    }
    match tiles {
        Some(tiles) if errors.is_empty() => Ok(GridMap {
            name: header.name,
            tiles,
            features,
            legend: Legend {
                names: header.legend,
                terrains,
            },
            look: header.look,
        }),
        _ => Err(errors),
    }
}

/// What the tile rows are read against.
struct Rows<'a, T> {
    file: &'a str,
    legend: &'a BTreeMap<char, String>,
    terrains: &'a BTreeMap<char, T>,
    max_size: usize,
}

impl<T: Copy + Default> Rows<'_, T> {
    /// Parses the tile `rows` (the first is on 0-based line index `first`)
    /// into a grid, pushing every problem onto `errors`. `None` if the
    /// tiles can't form a grid; a returned grid is only valid if no error
    /// was pushed.
    fn parse(
        &self,
        rows: &[&str],
        first: usize,
        errors: &mut Vec<ContentError>,
    ) -> Option<Grid<T>> {
        let file = self.file;
        let max = self.max_size;
        let mut rows: Vec<(u32, Vec<char>)> = rows
            .iter()
            .enumerate()
            .map(|(i, l)| (line_number(first + i), l.chars().collect()))
            .collect();
        // Blank lines at the end are ignored.
        while rows.last().is_some_and(|(_, r)| r.is_empty()) {
            rows.pop();
        }
        let first_line = line_number(first);
        let Some((_, first_row)) = rows.first() else {
            errors.push(ContentError::new(file, "map has no tiles").at(first_line, Some(1)));
            return None;
        };
        let width = first_row.len();
        if width == 0 {
            errors.push(ContentError::new(file, "first tile row is empty").at(first_line, Some(1)));
            return None;
        }
        if width > max {
            errors.push(
                ContentError::new(
                    file,
                    format!("map is {width} tiles wide; the maximum is {max}"),
                )
                .at(first_line, Some(column_number(max))),
            );
        }
        if rows.len() > max {
            errors.push(
                ContentError::new(
                    file,
                    format!("map is {} tiles tall; the maximum is {max}", rows.len()),
                )
                .at(rows[max].0, Some(1)),
            );
        }
        let mut cells = Vec::with_capacity(width * rows.len());
        for (line, row) in &rows {
            if row.len() != width {
                errors.push(
                    ContentError::new(
                        file,
                        format!(
                            "row is {} tiles wide; expected {width} like the first row",
                            row.len()
                        ),
                    )
                    .at(*line, Some(column_number(row.len().min(width)))),
                );
            }
            for (i, &c) in row.iter().enumerate() {
                match self.terrains.get(&c) {
                    Some(&t) => cells.push(t),
                    // Its unknown terrain id was already reported.
                    None if self.legend.contains_key(&c) => cells.push(T::default()),
                    None => errors.push(
                        ContentError::new(file, format!("'{c}' is not in the legend"))
                            .at(*line, Some(column_number(i))),
                    ),
                }
            }
        }
        let w = u16::try_from(width).ok()?;
        let h = u16::try_from(rows.len()).ok()?;
        Grid::from_cells(w, h, cells)
    }
}

/// Prints `map` in `.map` format: its look only if it isn't the look of a
/// map that names none. `None` if a tile's terrain has no character in the
/// legend. Parsing the text gives `map` back.
pub fn print<T, F>(map: &GridMap<T, F>) -> Option<String>
where
    T: Copy + PartialEq,
    F: MapFeature + Clone,
{
    let header = Header {
        name: map.name.clone(),
        legend: map.legend.names.clone(),
        features: map
            .features
            .iter()
            .map(|(p, f)| ((p.x, p.y), f.clone()))
            .collect(),
        look: map.look.clone(),
    };
    let mut out = ::ron::to_string(&header).ok()?;
    out.push('\n');
    out.push_str(SEPARATOR);
    out.push('\n');
    let width = usize::from(map.tiles.width());
    for row in map.tiles.cells().chunks(width.max(1)) {
        for &t in row {
            out.push(map.legend.char_for(t)?);
        }
        out.push('\n');
    }
    Some(out)
}

/// The header lines of map `source`: every line before the separator (all
/// of them if it has none).
pub fn header_lines(source: &str) -> Vec<&str> {
    let lines: Vec<&str> = source.lines().collect();
    let header = lines.iter().position(|&l| l == SEPARATOR);
    lines[..header.unwrap_or(lines.len())].to_vec()
}

/// 1-based line and column of legend entry `c` in the header lines: the
/// quoted character if found, else the terrain id string, else the start.
pub fn legend_position(header: &[&str], c: char, id: &str) -> (u32, u32) {
    let needles = [format!("'{c}'"), format!("\"{id}\"")];
    let found = needles.iter().find_map(|n| header_position(header, n));
    found.unwrap_or((1, 1))
}

/// 1-based line and column of the first `needle` in the header lines.
pub fn header_position(header: &[&str], needle: &str) -> Option<(u32, u32)> {
    header.iter().enumerate().find_map(|(i, line)| {
        let byte = line.find(needle)?;
        Some((line_number(i), column_number(line[..byte].chars().count())))
    })
}

/// 1-based line number of 0-based line index `i`.
pub fn line_number(i: usize) -> u32 {
    u32::try_from(i + 1).unwrap_or(u32::MAX)
}

/// 1-based column number of 0-based char index `i`.
pub fn column_number(i: usize) -> u32 {
    u32::try_from(i + 1).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;
