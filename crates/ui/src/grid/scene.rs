//! What is on the visible part of a grid map, as plain data (game
//! game ADR-0038): terrain (flashing where it just changed, or marked with what
//! it would turn into), ranges, units, the cursor, the path and those that
//! threaten a tile. No colors, glyphs, cells or pixels: a
//! [`GridSkin`](super::GridSkin) turns a scene into those. The units are
//! the game's own view of them (`U`), which only the game's
//! [`UnitPainter`](super::UnitPainter) paints.

use crate::prelude::*;
use core::fmt::Write as _;

use serde::{Deserialize, Serialize};
use shuyi_assets::grid_map::MapLook;
use shuyi_core::{Pos, TerrainId};

use super::skin::Terrains;

/// The visible part of a grid map, with units of the game's kind `U`.
/// Build it with [`MapScene::new`] and assign the rest: a struct literal
/// doesn't compile outside this crate, so a new field never breaks a game
/// (ADR-0039).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct MapScene<U> {
    /// The map tile at the view's top-left (negative, or past the map, when
    /// the map is smaller than the view).
    pub origin: Pos,
    /// Visible tiles, across × down.
    pub size: (i32, i32),
    /// The sides of the view the map goes on past: for a game's marks at
    /// those edges. None unless set ([`MapEdges::of_view`]).
    pub more: MapEdges,
    /// The visible tiles, row by row: `size.0 × size.1` of them.
    pub tiles: Vec<TileView>,
    /// The ranges shown, in the order laid on: on a tile in two, the
    /// second is laid over the first.
    pub ranges: Vec<RangeLayer>,
    /// The terrain of the ring of tiles just outside the view (the row
    /// above it, then the two ends of each of its rows, then the row
    /// below), for a skin whose picture of a tile depends on its
    /// neighbours. `None` = off the map, or not given. Read it with
    /// [`terrain_near`](Self::terrain_near).
    pub rim: Vec<Option<TerrainId>>,
    /// How the map looks, as far as its file says: which set of pictures
    /// a skin that has pictures paints its terrain with, and which light
    /// it shades them by (the light's name: the skin says how it looks).
    pub look: MapLook,
    /// The units on visible tiles, in the game's order.
    pub units: Vec<U>,
    /// The cursor, if it is shown and on a visible tile.
    pub cursor: Option<CursorView>,
    /// The selected unit's path, its own tile first; empty = none. Its
    /// tiles may lie outside the view.
    pub path: Vec<Pos>,
    /// Those that could attack a tile, if the game shows them; `None` =
    /// nothing to show.
    pub threat: Option<ThreatView>,
    /// The screen's animation clock, in milliseconds: what a skin or a
    /// unit painter moves its marks by. It only ever goes forward.
    pub clock_ms: u64,
}

/// The sides of a view the map goes on past: there is more map to its
/// left, to its right, above it or below it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MapEdges {
    /// Across: more map left of the view (`before`), right of it
    /// (`after`).
    pub x: Ends,
    /// Down: more map above the view (`before`), below it (`after`).
    pub y: Ends,
}

/// Whether the map goes on before and after a view, along one axis.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Ends {
    /// More map before the view's first tile (left, or above).
    pub before: bool,
    /// More map after its last (right, or below).
    pub after: bool,
}

impl Ends {
    /// Neither end.
    pub const NONE: Self = Self {
        before: false,
        after: false,
    };

    /// The ends a view `len` tiles long from `start` leaves map past, on a
    /// map `map_len` tiles long from 0.
    fn of_view(start: i32, len: i32, map_len: i32) -> Self {
        let end = i64::from(start) + i64::from(len.max(0));
        Self {
            before: start > 0,
            after: end < i64::from(map_len),
        }
    }
}

impl MapEdges {
    /// The map goes on past no side: it all fits in the view.
    pub const NONE: Self = Self {
        x: Ends::NONE,
        y: Ends::NONE,
    };

    /// The sides a view of `size` tiles from `origin` leaves map past, on
    /// a map of `map_size` tiles (across × down) from `(0, 0)`: left when
    /// the view starts right of the map's first column, right when it
    /// ends before its last, and the same up and down. A map smaller than
    /// the view (centred in it, from a negative origin) goes on past none.
    pub fn of_view(origin: Pos, size: (i32, i32), map_size: (i32, i32)) -> Self {
        Self {
            x: Ends::of_view(origin.x, size.0, map_size.0),
            y: Ends::of_view(origin.y, size.1, map_size.1),
        }
    }

    /// More map left of the view.
    pub const fn left(self) -> bool {
        self.x.before
    }

    /// More map right of the view.
    pub const fn right(self) -> bool {
        self.x.after
    }

    /// More map above the view.
    pub const fn up(self) -> bool {
        self.y.before
    }

    /// More map below the view.
    pub const fn down(self) -> bool {
        self.y.after
    }

    /// Whether the map goes on past any side.
    pub const fn any(self) -> bool {
        self.left() || self.right() || self.up() || self.down()
    }

    /// The sides set, by name, in the order left, right, up, down.
    pub fn names(self) -> Vec<&'static str> {
        let sides = [
            (self.left(), "left"),
            (self.right(), "right"),
            (self.up(), "up"),
            (self.down(), "down"),
        ];
        sides
            .into_iter()
            .filter_map(|(set, name)| set.then_some(name))
            .collect()
    }
}

/// One visible tile.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TileView {
    /// Its terrain; `None` = off the map.
    pub terrain: Option<TerrainId>,
    /// How strongly it flashes after its terrain changed: `1` at the
    /// change, fading to `0`. One entry per change still flashing; usually
    /// none.
    pub flashes: Vec<f32>,
    /// The terrain it would turn into (a spell being aimed at it): shown
    /// in place of its own, so the tile reads as terrain, not as a target.
    pub becomes: Option<TerrainId>,
}

/// A kind of range a map shows (where a unit can move, what it can
/// attack), by its name: the game's own list, of which the engine knows
/// nothing. A skin tints a range's tiles with the palette's color of that
/// name, and the scene's text marks a tile with its first letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RangeName(pub &'static str);

impl RangeName {
    /// Its letter in [`MapScene::to_text`]: its name's first, `?` for an
    /// empty name.
    pub fn mark(self) -> char {
        self.0.chars().next().unwrap_or('?')
    }
}

/// A range shown on the map: its kind, and the visible tiles in it in the
/// order given.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeLayer {
    /// Which range.
    pub kind: RangeName,
    /// Its visible tiles.
    pub tiles: Vec<Pos>,
}

/// The ranges each visible tile of a scene is in, in the order laid on:
/// what a skin tints a tile by ([`MapScene::tints`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tints {
    /// Tiles across.
    across: i32,
    /// Row by row, as [`MapScene::tiles`].
    tiles: Vec<Vec<RangeName>>,
}

impl Tints {
    /// The ranges the tile `dx` right of and `dy` below the view's first
    /// is in, in the order laid on; none for a tile outside the view.
    pub fn at(&self, dx: i32, dy: i32) -> &[RangeName] {
        let index = dy
            .checked_mul(self.across)
            .and_then(|row| row.checked_add(dx))
            .filter(|_| (0..self.across).contains(&dx))
            .and_then(|i| usize::try_from(i).ok());
        index
            .and_then(|i| self.tiles.get(i))
            .map_or(&[], Vec::as_slice)
    }
}

/// How the cursor is drawn: corner marks by default; bigger corners and
/// the tile glow are for players who need them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CursorStyle {
    /// Corner marks.
    #[default]
    Corners,
    /// Longer corner marks.
    LargeCorners,
    /// The tile's background tinted towards the cursor color.
    TileGlow,
}

impl CursorStyle {
    /// Its name in [`MapScene::to_text`].
    pub const fn name(self) -> &'static str {
        match self {
            CursorStyle::Corners => "corners",
            CursorStyle::LargeCorners => "large-corners",
            CursorStyle::TileGlow => "glow",
        }
    }
}

/// The cursor as shown on the map.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CursorView {
    /// The tile it is on.
    pub pos: Pos,
    /// Where it is in its pulse: `1` = full brightness, down to about half.
    pub brightness: f32,
    /// The player's cursor style.
    pub style: CursorStyle,
}

/// Those that could attack a tile in their next turn: an arrow from each
/// to the tile, and a mark over each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreatView {
    /// The tile they could attack.
    pub tile: Pos,
    /// The tiles they stand on, in the game's order; never empty. They may
    /// lie outside the view.
    pub enemies: Vec<Pos>,
}

/// What a scene needs of a game's view of a unit: which unit it is, the
/// tile it is shown on, and its line in the scene's text.
pub trait MapUnit {
    /// What tells units apart.
    type Id: Copy + PartialEq;

    /// Which unit it is.
    fn id(&self) -> Self::Id;

    /// The tile it is shown on.
    fn pos(&self) -> Pos;

    /// Its line in [`MapScene::to_text`]: everything about it a test or a
    /// bug report needs, the same for the same unit.
    fn text(&self) -> String;
}

impl<U> MapScene<U> {
    /// A view of `size` tiles from `origin` with nothing on it: every tile
    /// off the map. A negative size counts as zero.
    pub fn new(origin: Pos, size: (i32, i32)) -> Self {
        let size = (size.0.max(0), size.1.max(0));
        let count = usize::try_from(size.0).unwrap_or(0) * usize::try_from(size.1).unwrap_or(0);
        Self {
            origin,
            size,
            more: MapEdges::NONE,
            tiles: vec![TileView::default(); count],
            ranges: Vec::new(),
            rim: vec![None; rim_len(size)],
            look: MapLook::default(),
            units: Vec::new(),
            cursor: None,
            path: Vec::new(),
            threat: None,
            clock_ms: 0,
        }
    }

    /// Where `pos` is in the view, as tiles right of and below
    /// [`origin`](Self::origin), or `None` if it isn't visible.
    pub fn offset(&self, pos: Pos) -> Option<(i32, i32)> {
        let dx = pos.x.checked_sub(self.origin.x)?;
        let dy = pos.y.checked_sub(self.origin.y)?;
        let inside = (0..self.size.0).contains(&dx) && (0..self.size.1).contains(&dy);
        inside.then_some((dx, dy))
    }

    /// Whether `pos` is a visible tile.
    pub fn contains(&self, pos: Pos) -> bool {
        self.offset(pos).is_some()
    }

    /// Index in [`tiles`](Self::tiles) of the tile `dx` right of and `dy`
    /// below the origin.
    fn index(&self, dx: i32, dy: i32) -> Option<usize> {
        if !(0..self.size.0).contains(&dx) || !(0..self.size.1).contains(&dy) {
            return None;
        }
        let (dx, dy) = (usize::try_from(dx).ok()?, usize::try_from(dy).ok()?);
        Some(dy * usize::try_from(self.size.0).ok()? + dx)
    }

    /// The tile `dx` right of and `dy` below the origin, if in the view.
    pub fn tile_at(&self, dx: i32, dy: i32) -> Option<&TileView> {
        self.tiles.get(self.index(dx, dy)?)
    }

    /// The tile at map position `pos`, if visible.
    pub fn tile(&self, pos: Pos) -> Option<&TileView> {
        let (dx, dy) = self.offset(pos)?;
        self.tile_at(dx, dy)
    }

    /// Mutable [`tile`](Self::tile).
    pub fn tile_mut(&mut self, pos: Pos) -> Option<&mut TileView> {
        let (dx, dy) = self.offset(pos)?;
        let index = self.index(dx, dy)?;
        self.tiles.get_mut(index)
    }

    /// Index in [`rim`](Self::rim) of the tile `dx` right of and `dy` below
    /// the origin, if it is in the ring just outside the view.
    fn rim_index(&self, dx: i32, dy: i32) -> Option<usize> {
        let (w, h) = self.size;
        if !(-1..=w).contains(&dx) || !(-1..=h).contains(&dy) {
            return None;
        }
        let across = w + 2;
        let index = if dy == -1 {
            dx + 1
        } else if dy == h {
            across + 2 * h + dx + 1
        } else if dx == -1 {
            across + 2 * dy
        } else if dx == w {
            across + 2 * dy + 1
        } else {
            return None;
        };
        usize::try_from(index).ok()
    }

    /// The terrain of the tile `dx` right of and `dy` below the origin: a
    /// tile of the view, or of the ring just outside it. `None` off the
    /// map, further out, or where the ring wasn't given.
    pub fn terrain_near(&self, dx: i32, dy: i32) -> Option<TerrainId> {
        match self.tile_at(dx, dy) {
            Some(tile) => tile.terrain,
            None => self.rim.get(self.rim_index(dx, dy)?).copied().flatten(),
        }
    }

    /// Sets the terrain of every tile of the view, and of the ring just
    /// outside it, to what `terrain` gives for its map position.
    pub fn set_terrain(&mut self, terrain: impl Fn(Pos) -> Option<TerrainId>) {
        let (w, h) = self.size;
        let origin = self.origin;
        for (dx, dy) in (-1..=h).flat_map(|dy| (-1..=w).map(move |dx| (dx, dy))) {
            let id = terrain(Pos::new(origin.x + dx, origin.y + dy));
            let tile = self.index(dx, dy).and_then(|i| self.tiles.get_mut(i));
            if let Some(tile) = tile {
                tile.terrain = id;
            } else if let Some(slot) = self.rim_index(dx, dy).and_then(|i| self.rim.get_mut(i)) {
                *slot = id;
            }
        }
    }

    /// Lays the range `kind` on each of `tiles` that is visible: a new
    /// layer over those laid before, if any of them is.
    pub fn tint(&mut self, tiles: impl IntoIterator<Item = Pos>, kind: impl Into<RangeName>) {
        let tiles: Vec<Pos> = tiles
            .into_iter()
            .filter(|&pos| self.tile(pos).is_some())
            .collect();
        if !tiles.is_empty() {
            let kind = kind.into();
            self.ranges.push(RangeLayer { kind, tiles });
        }
    }

    /// The ranges every visible tile is in, in the order laid on (a tile
    /// listed twice in a layer is in it twice).
    pub fn tints(&self) -> Tints {
        let mut tiles = vec![Vec::new(); self.tiles.len()];
        for layer in &self.ranges {
            for &pos in &layer.tiles {
                let at = self.offset(pos).and_then(|(dx, dy)| self.index(dx, dy));
                if let Some(list) = at.and_then(|i| tiles.get_mut(i)) {
                    list.push(layer.kind);
                }
            }
        }
        Tints {
            across: self.size.0,
            tiles,
        }
    }

    /// The visible tiles in the range `kind`, row by row, each once.
    pub fn tinted(&self, kind: impl Into<RangeName>) -> Vec<Pos> {
        let kind = kind.into();
        let tints = self.tints();
        let (w, h) = self.size;
        let offsets = (0..h).flat_map(|dy| (0..w).map(move |dx| (dx, dy)));
        offsets
            .filter(|&(dx, dy)| tints.at(dx, dy).contains(&kind))
            .map(|(dx, dy)| Pos::new(self.origin.x + dx, self.origin.y + dy))
            .collect()
    }

    /// The ranges `pos` is in, in the order laid on; none if it isn't
    /// visible.
    pub fn tints_at(&self, pos: Pos) -> Vec<RangeName> {
        match self.offset(pos) {
            Some((dx, dy)) => self.tints().at(dx, dy).to_vec(),
            None => Vec::new(),
        }
    }

    /// The terrain shown on `pos`, if it is a visible map tile.
    pub fn terrain_at(&self, pos: Pos) -> Option<TerrainId> {
        self.tile(pos).and_then(|t| t.terrain)
    }

    /// The tile the cursor is on, if it is shown.
    pub fn cursor_tile(&self) -> Option<Pos> {
        self.cursor.map(|c| c.pos)
    }

    /// Row `dy` of the view in [`to_text`](Self::to_text): its tiles' names,
    /// equal neighbours counted.
    fn row_text(&self, terrains: &dyn Terrains, tints: &Tints, dy: i32) -> String {
        let mut runs: Vec<(String, usize)> = Vec::new();
        for dx in 0..self.size.0 {
            let name = self.tile_at(dx, dy).map_or_else(
                || "!".to_owned(),
                |t| tile_text(terrains, t, tints.at(dx, dy)),
            );
            match runs.last_mut() {
                Some((last, n)) if *last == name => *n += 1,
                _ => runs.push((name, 1)),
            }
        }
        let words: Vec<String> = runs
            .into_iter()
            .map(|(name, n)| if n > 1 { format!("{name}*{n}") } else { name })
            .collect();
        words.join(" ")
    }
}

impl<U: MapUnit> MapScene<U> {
    /// Adds `unit` if it is on a visible tile.
    pub fn push_unit(&mut self, unit: U) {
        if self.contains(unit.pos()) {
            self.units.push(unit);
        }
    }

    /// The unit `id`, if it is shown.
    pub fn unit(&self, id: U::Id) -> Option<&U> {
        self.units.iter().find(|u| u.id() == id)
    }

    /// The unit shown on `pos`, if any (the last one, drawn on top, if two
    /// share it).
    pub fn unit_at(&self, pos: Pos) -> Option<&U> {
        self.units.iter().rev().find(|u| u.pos() == pos)
    }

    /// The scene as text, for tests and bug reports, with the terrains'
    /// names from `terrains`. The same scene always gives the same text.
    /// The clock isn't in it (nothing happens by it), nor the ring of
    /// terrain round the view (it is the map's, one tile further out). A
    /// look other than a map's usual one ends the first line: `look
    /// indoor`; so does a light other than the usual: `light dusk`. The
    /// sides the map goes on past follow it in a line of their own, only
    /// when there are some: `more right down`.
    ///
    /// ```text
    /// origin (-10,-11) size 35x30
    ///  -11: -*35
    ///    0: -*10 water*2 grass+m*3 sand+mm grass! grass>water -*18
    /// units 1
    ///   #4 (7,3)
    /// cursor (3,5) corners 1.00
    /// path (3,5) (4,5)
    /// ```
    ///
    /// One line per row of tiles, headed by its map row: terrain names
    /// (`-` = off the map, `?n` for an id the game lacks), `+` and a letter
    /// per range on the tile in the order laid on (each range's
    /// [mark](RangeName::mark)), and `*n` for `n` such tiles in a row. `!`
    /// marks a tile flashing after its terrain changed, and `>` is
    /// followed by the terrain it would turn into. Each unit's line is its
    /// own ([`MapUnit::text`]). The cursor's number is its brightness. A
    /// last line, `threat (5,3) from (6,3) (9,1)`, is there only when a
    /// tile is threatened: the tile, then where each threat stands.
    // check-text: not player text (the scene as text, for tests)
    pub fn to_text(&self, terrains: &dyn Terrains) -> String {
        let mut out = String::new();
        let Pos { x, y } = self.origin;
        let (w, h) = self.size;
        // Writing to a `String` can't fail.
        let _ = write!(out, "origin ({x},{y}) size {w}x{h}");
        let usual = MapLook::default();
        if self.look.tiles != usual.tiles {
            let _ = write!(out, " look {}", self.look.tiles);
        }
        if self.look.light != usual.light {
            let _ = write!(out, " light {}", self.look.light);
        }
        out.push('\n');
        if self.more.any() {
            let _ = writeln!(out, "more {}", self.more.names().join(" "));
        }
        let tints = self.tints();
        for dy in 0..h {
            let row = self.row_text(terrains, &tints, dy);
            let _ = writeln!(out, "{:>4}: {row}", i64::from(y) + i64::from(dy));
        }
        let _ = writeln!(out, "units {}", self.units.len());
        for u in &self.units {
            let _ = writeln!(out, "  {}", u.text());
        }
        match &self.cursor {
            Some(c) => {
                let Pos { x, y } = c.pos;
                let (style, bright) = (c.style.name(), c.brightness);
                let _ = writeln!(out, "cursor ({x},{y}) {style} {bright:.2}");
            }
            None => out.push_str("cursor -\n"),
        }
        out.push_str("path");
        if self.path.is_empty() {
            out.push_str(" -");
        }
        for p in &self.path {
            let _ = write!(out, " ({},{})", p.x, p.y);
        }
        out.push('\n');
        if let Some(threat) = &self.threat {
            let Pos { x, y } = threat.tile;
            let _ = write!(out, "threat ({x},{y}) from");
            for p in &threat.enemies {
                let _ = write!(out, " ({},{})", p.x, p.y);
            }
            out.push('\n');
        }
        out
    }
}

/// How many tiles the ring just outside a view of `size` tiles has.
fn rim_len(size: (i32, i32)) -> usize {
    let count = |tiles: i32| usize::try_from(tiles).unwrap_or(0);
    2 * (count(size.0) + 2) + 2 * count(size.1)
}

/// A tile in [`MapScene::to_text`]: its terrain's name (`-` off the map,
/// `?n` for an id the game lacks), `!` if it flashes, `+` and its ranges'
/// letters, then `>` and the terrain it would become.
fn tile_text(terrains: &dyn Terrains, tile: &TileView, ranges: &[RangeName]) -> String {
    let name = |id: TerrainId| match terrains.name(id) {
        Some(name) => name.to_owned(),
        None => format!("?{}", id.0),
    };
    let mut text = tile.terrain.map_or_else(|| "-".to_owned(), name);
    if !tile.flashes.is_empty() {
        text.push('!');
    }
    if !ranges.is_empty() {
        text.push('+');
        text.extend(ranges.iter().map(|k| k.mark()));
    }
    if let Some(id) = tile.becomes {
        text.push('>');
        text.push_str(&name(id));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toy::grid::{GRASS, Piece, SAND, Terrain, WATER};

    fn p(x: i32, y: i32) -> Pos {
        Pos::new(x, y)
    }

    fn piece(id: u32, pos: Pos) -> Piece {
        Piece::new(id, 'k', pos)
    }

    const MOVE: RangeName = RangeName("move");
    const DANGER: RangeName = RangeName("danger");
    const ATTACK: RangeName = RangeName("attack");
    const HEAL: RangeName = RangeName("heal");

    #[test]
    fn a_new_scene_is_empty_tiles_off_the_map() {
        let s = MapScene::<Piece>::new(p(-2, 3), (4, 2));
        assert_eq!((s.origin, s.size), (p(-2, 3), (4, 2)));
        assert_eq!(s.tiles, vec![TileView::default(); 8]);
        assert_eq!(s.tiles[0].terrain, None);
        assert!(s.units.is_empty() && s.path.is_empty() && s.cursor.is_none());
        assert!(s.ranges.is_empty());
        assert_eq!(s.threat, None);
        assert_eq!(s.clock_ms, 0);
        // The ring round a 4 × 2 view: 6 above, 6 below, 2 at each side.
        assert_eq!(s.rim, vec![None; 16]);
        assert_eq!(s.look, MapLook::default());
        assert_eq!(s.more, MapEdges::NONE);
        // A negative size is an empty view.
        let none = MapScene::<Piece>::new(p(0, 0), (-3, 5));
        assert_eq!((none.size, none.tiles.len()), ((0, 5), 0));
        let none = MapScene::<Piece>::new(p(0, 0), (3, -5));
        assert_eq!((none.size, none.tiles.len()), ((3, 0), 0));
        assert!(!none.contains(p(0, 0)));
    }

    #[test]
    fn tiles_are_found_by_map_position_inside_the_view_only() {
        let mut s = MapScene::<Piece>::new(p(-2, 3), (4, 2));
        for (i, t) in (0..).zip(&mut s.tiles) {
            t.terrain = Some(TerrainId(i));
        }
        let id = |pos| s.tile(pos).and_then(|t| t.terrain).map(|t| t.0);
        let last_row = (id(p(-2, 4)), id(p(1, 4)));
        assert_eq!(id(p(-2, 3)), Some(0));
        assert_eq!(id(p(1, 3)), Some(3));
        assert_eq!(last_row, (Some(4), Some(7)));
        for out in [p(-3, 3), p(2, 3), p(-2, 2), p(-2, 5), p(2, 4)] {
            assert_eq!(s.tile(out), None, "{out:?}");
            assert!(!s.contains(out), "{out:?}");
        }
        assert_eq!(s.offset(p(1, 4)), Some((3, 1)));
        assert_eq!(s.offset(p(i32::MIN, 3)), None);
        assert_eq!(s.offset(p(0, i32::MAX)), None);
        assert_eq!(s.tile_at(3, 1).and_then(|t| t.terrain), Some(TerrainId(7)));
        for (dx, dy) in [(4, 0), (-1, 0), (0, 2), (0, -1)] {
            assert_eq!(s.tile_at(dx, dy), None, "({dx}, {dy})");
        }
        // A scene short of tiles (not built by `new`) has none there.
        s.tiles.truncate(5);
        assert_eq!(s.tile(p(-2, 4)).and_then(|t| t.terrain), Some(TerrainId(4)));
        assert_eq!(s.tile(p(-1, 4)), None);
        s.tile_mut(p(-2, 4)).unwrap().terrain = None;
        assert_eq!(s.terrain_at(p(-2, 4)), None);
        assert!(s.tile_mut(p(-1, 4)).is_none());
    }

    #[test]
    fn the_ring_round_the_view_holds_the_terrain_one_tile_out() {
        let mut s = MapScene::<Piece>::new(p(5, -2), (3, 2));
        assert_eq!((rim_len((3, 2)), rim_len((0, 0))), (14, 4));
        assert_eq!((rim_len((-3, 2)), rim_len((3, -2))), (8, 10));
        // Row above, the ends of each row, row below: each place once.
        let ring = [
            (-1, -1),
            (0, -1),
            (1, -1),
            (2, -1),
            (3, -1),
            (-1, 0),
            (3, 0),
            (-1, 1),
            (3, 1),
            (-1, 2),
            (0, 2),
            (1, 2),
            (2, 2),
            (3, 2),
        ];
        for (i, (dx, dy)) in ring.into_iter().enumerate() {
            assert_eq!(s.rim_index(dx, dy), Some(i), "({dx}, {dy})");
        }
        // Inside the view, or further out: not in the ring. Further out
        // along one of the ring's own rows or columns too.
        let outside = [
            (0, 0),
            (2, 1),
            (-2, 0),
            (4, 0),
            (0, -2),
            (0, 3),
            (-2, -1),
            (4, -1),
            (4, 2),
            (-1, 3),
            (3, -2),
        ];
        for (dx, dy) in outside {
            assert_eq!(s.rim_index(dx, dy), None, "({dx}, {dy})");
            assert_eq!(s.terrain_near(dx, dy), None, "({dx}, {dy})");
        }
        // The terrain of each map position: its column and row in one id.
        let id = |pos: Pos| u16::try_from(10 * pos.x + pos.y + 2).ok().map(TerrainId);
        s.set_terrain(|pos| id(pos).filter(|_| pos != p(6, -1) && pos != p(8, 0)));
        assert_eq!(s.terrain_at(p(5, -2)), Some(TerrainId(50)));
        assert_eq!(s.terrain_at(p(7, -1)), Some(TerrainId(71)));
        // Off the map, in the view and in the ring.
        assert_eq!(s.terrain_at(p(6, -1)), None);
        assert_eq!(s.terrain_near(1, 1), None);
        assert_eq!(s.terrain_near(3, 2), None);
        for (dx, dy) in ring {
            let pos = p(5 + dx, -2 + dy);
            let expect = id(pos).filter(|_| pos != p(8, 0));
            assert_eq!(s.terrain_near(dx, dy), expect, "({dx}, {dy})");
        }
        assert_eq!(s.terrain_near(0, 0), Some(TerrainId(50)));
        assert_eq!(s.terrain_near(2, 1), Some(TerrainId(71)));
        assert_eq!(s.terrain_near(-1, -1), Some(TerrainId(39)));
        assert_eq!(s.terrain_near(3, 1), Some(TerrainId(81)));
        // A ring short of tiles (not built by `new`) has none there.
        s.rim.truncate(3);
        assert_eq!(s.terrain_near(1, -1), Some(TerrainId(59)));
        assert_eq!(s.terrain_near(2, -1), None);
        s.set_terrain(|_| Some(TerrainId(1)));
        assert_eq!(s.rim, vec![Some(TerrainId(1)); 3]);
    }

    #[test]
    fn the_map_goes_on_past_the_sides_the_view_leaves_map_beyond() {
        let edges = |left, right, up, down| MapEdges {
            x: Ends {
                before: left,
                after: right,
            },
            y: Ends {
                before: up,
                after: down,
            },
        };
        // A map 20 × 4 seen 8 × 4 at a time: at its left edge, in the
        // middle and at its right edge.
        let wide = |x| MapEdges::of_view(p(x, 0), (8, 4), (20, 4));
        assert_eq!(wide(0), edges(false, true, false, false));
        assert_eq!(wide(6), edges(true, true, false, false));
        assert_eq!(wide(12), edges(true, false, false, false));
        // The same down a map 5 × 30 seen 5 × 10 at a time.
        let tall = |y| MapEdges::of_view(p(0, y), (5, 10), (5, 30));
        assert_eq!(tall(0), edges(false, false, false, true));
        assert_eq!(tall(10), edges(false, false, true, true));
        assert_eq!(tall(20), edges(false, false, true, false));
        // Both ways at once.
        let both = MapEdges::of_view(p(3, 2), (4, 4), (10, 10));
        assert_eq!(both, edges(true, true, true, true));
        // A map smaller than the view, centred in it: none.
        let small = MapEdges::of_view(p(-3, -2), (12, 8), (6, 4));
        assert_eq!(small, MapEdges::NONE);
        assert!(!small.any());
        assert_eq!(MapEdges::default(), MapEdges::NONE);
        // Far out, nothing overflows; a negative size is an empty view.
        let far = MapEdges::of_view(p(i32::MAX, i32::MAX), (i32::MAX, i32::MAX), (1, 1));
        assert_eq!(far, edges(true, false, true, false));
        let empty = MapEdges::of_view(p(0, 0), (-4, -4), (1, 1));
        assert_eq!(empty, edges(false, true, false, true));
        // Each side counts towards any, and is named.
        for one in [
            edges(true, false, false, false),
            edges(false, true, false, false),
            edges(false, false, true, false),
            edges(false, false, false, true),
        ] {
            assert!(one.any(), "{one:?}");
            assert_eq!(one.names().len(), 1, "{one:?}");
        }
        let all = edges(true, true, true, true).names();
        assert_eq!(all, ["left", "right", "up", "down"]);
    }

    #[test]
    fn the_text_lists_the_sides_the_map_goes_on_past_only_when_there_are_some() {
        let mut s = MapScene::<Piece>::new(p(0, -1), (2, 1));
        let plain = s.to_text(&Terrain);
        assert!(!plain.contains("more"), "{plain}");
        // A map that fits: no line.
        s.more = MapEdges::of_view(p(0, 0), s.size, (2, 1));
        assert_eq!(s.to_text(&Terrain), plain);
        let after = Ends {
            before: false,
            after: true,
        };
        s.more = MapEdges { x: after, y: after };
        assert_eq!(
            s.to_text(&Terrain),
            "origin (0,-1) size 2x1\nmore right down\n  -1: -*2\nunits 0\ncursor -\npath -\n"
        );
    }

    #[test]
    fn the_text_names_a_look_other_than_the_usual_one() {
        let mut s = MapScene::<Piece>::new(p(0, -1), (2, 1));
        let text = s.to_text(&Terrain);
        assert!(text.starts_with("origin (0,-1) size 2x1\n"), "{text}");
        s.look.tiles = "indoor".to_owned();
        let text = s.to_text(&Terrain);
        let first = "origin (0,-1) size 2x1 look indoor\n";
        assert!(text.starts_with(first), "{text}");
        // A light other than noon, after it or alone.
        s.look.light = "dusk".to_owned();
        let text = s.to_text(&Terrain);
        let first = "origin (0,-1) size 2x1 look indoor light dusk\n";
        assert!(text.starts_with(first), "{text}");
        s.look.tiles = "outdoor".to_owned();
        let text = s.to_text(&Terrain);
        assert!(
            text.starts_with("origin (0,-1) size 2x1 light dusk\n"),
            "{text}"
        );
    }

    #[test]
    fn tints_go_on_visible_tiles_in_the_order_laid_on() {
        let mut s = MapScene::<Piece>::new(p(1, 1), (3, 2));
        s.tint([p(1, 1), p(3, 2), p(0, 0), p(4, 1)], DANGER);
        s.tint([p(3, 2), p(2, 1)], MOVE);
        assert_eq!(s.tints_at(p(1, 1)), [DANGER]);
        assert_eq!(s.tints_at(p(3, 2)), [DANGER, MOVE]);
        assert!(s.tints_at(p(2, 2)).is_empty());
        assert!(s.tints_at(p(0, 0)).is_empty());
        assert_eq!(s.tinted(DANGER), [p(1, 1), p(3, 2)]);
        assert_eq!(s.tinted(MOVE), [p(2, 1), p(3, 2)]);
        assert!(s.tinted(ATTACK).is_empty());
        // Each layer holds only its visible tiles, as given.
        let layer = |kind, tiles: &[Pos]| RangeLayer {
            kind,
            tiles: tiles.to_vec(),
        };
        assert_eq!(
            s.ranges,
            [
                layer(DANGER, &[p(1, 1), p(3, 2)]),
                layer(MOVE, &[p(3, 2), p(2, 1)]),
            ]
        );
        // A range with no visible tile adds nothing.
        let before = s.clone();
        s.tint([p(0, 0), p(9, 9)], HEAL);
        s.tint([], HEAL);
        assert_eq!(s, before);
        // A tile given twice is in the range twice; a range laid twice
        // tints twice.
        s.tint([p(2, 2), p(2, 2)], HEAL);
        s.tint([p(2, 2)], DANGER);
        assert_eq!(s.tints_at(p(2, 2)), [HEAL, HEAL, DANGER]);
        assert_eq!(s.tinted(HEAL), [p(2, 2)]);
        // Row by row, whatever the order laid on.
        assert_eq!(s.tinted(DANGER), [p(1, 1), p(2, 2), p(3, 2)]);
    }

    #[test]
    fn tints_are_read_by_the_tiles_place_in_the_view() {
        let mut s = MapScene::<Piece>::new(p(1, 1), (3, 2));
        s.tint([p(3, 1), p(1, 2)], MOVE);
        s.tint([p(1, 2)], HEAL);
        let tints = s.tints();
        assert_eq!(tints.at(2, 0), [MOVE]);
        assert_eq!(tints.at(0, 1), [MOVE, HEAL]);
        assert!(tints.at(0, 0).is_empty());
        // Outside the view: none, never the next row's.
        let far = [
            (3, 0),
            (-1, 1),
            (0, 2),
            (0, -1),
            (i32::MAX, 0),
            (0, i32::MAX),
            (0, i32::MIN),
        ];
        for (dx, dy) in far {
            assert!(tints.at(dx, dy).is_empty(), "({dx}, {dy})");
        }
        // A layer naming a tile outside the view (not laid by `tint`):
        // not on any tile.
        s.ranges.push(RangeLayer {
            kind: DANGER,
            tiles: vec![p(0, 1), p(4, 1)],
        });
        assert_eq!(s.tints(), tints);
        // A scene short of tiles: those it lacks are in no range.
        s.tiles.truncate(3);
        let short = s.tints();
        assert_eq!(short.at(2, 0), [MOVE]);
        assert!(short.at(0, 1).is_empty());
        s.tint([p(1, 2)], DANGER);
        assert!(s.tints_at(p(1, 2)).is_empty());
    }

    #[test]
    fn a_ranges_mark_is_its_names_first_letter() {
        assert_eq!(MOVE.mark(), 'm');
        assert_eq!(RangeName("Zone").mark(), 'Z');
        assert_eq!(RangeName("").mark(), '?');
    }

    #[test]
    fn only_units_on_visible_tiles_are_kept() {
        let mut s = MapScene::new(p(1, 1), (3, 2));
        s.push_unit(piece(4, p(3, 2)));
        s.push_unit(piece(5, p(4, 2)));
        s.push_unit(piece(6, p(1, 1)));
        let ids: Vec<u32> = s.units.iter().map(|u| u.id).collect();
        assert_eq!(ids, [4, 6]);
        assert_eq!(s.unit(6).map(|u| u.pos), Some(p(1, 1)));
        assert_eq!(s.unit(5), None);
    }

    #[test]
    fn tiles_answer_what_is_on_them() {
        let mut s = MapScene::new(p(1, 1), (3, 2));
        s.tile_mut(p(2, 1)).unwrap().terrain = Some(TerrainId(4));
        s.tint([p(2, 1)], DANGER);
        s.tint([p(2, 1)], MOVE);
        s.push_unit(piece(4, p(2, 1)));
        s.push_unit(piece(6, p(3, 2)));
        s.push_unit(piece(7, p(3, 2)));
        assert_eq!(s.unit_at(p(2, 1)).map(|u| u.id), Some(4));
        assert_eq!(s.unit_at(p(3, 2)).map(|u| u.id), Some(7));
        assert_eq!(s.unit_at(p(1, 1)), None);
        assert_eq!(s.tints_at(p(2, 1)), [DANGER, MOVE]);
        assert!(s.tints_at(p(1, 1)).is_empty());
        assert!(s.tints_at(p(0, 0)).is_empty());
        assert_eq!(s.terrain_at(p(2, 1)), Some(TerrainId(4)));
        assert_eq!(s.terrain_at(p(1, 1)), None);
        assert_eq!(s.terrain_at(p(9, 9)), None);
        assert_eq!(s.cursor_tile(), None);
        s.cursor = Some(CursorView {
            pos: p(3, 2),
            brightness: 1.0,
            style: CursorStyle::Corners,
        });
        assert_eq!(s.cursor_tile(), Some(p(3, 2)));
    }

    #[test]
    fn to_text_lists_tiles_units_cursor_and_path() {
        let mut s = MapScene::new(p(-1, 2), (5, 3));
        for pos in [p(0, 2), p(1, 2), p(2, 2), p(0, 3), p(1, 3)] {
            s.tile_mut(pos).unwrap().terrain = Some(GRASS);
        }
        s.tile_mut(p(3, 2)).unwrap().terrain = Some(SAND);
        s.tile_mut(p(2, 3)).unwrap().terrain = Some(TerrainId(999));
        s.tint([p(1, 2), p(2, 2)], MOVE);
        s.tint([p(3, 2)], DANGER);
        s.tint([p(3, 2)], ATTACK);
        s.tint([p(0, 3)], HEAL);
        s.tile_mut(p(1, 3)).unwrap().flashes.push(0.5);
        s.tile_mut(p(0, 2)).unwrap().becomes = Some(WATER);
        s.tile_mut(p(2, 2)).unwrap().becomes = Some(TerrainId(998));
        s.push_unit(piece(4, p(0, 2)));
        s.push_unit(Piece::new(1, 'a', p(1, 3)));
        s.cursor = Some(CursorView {
            pos: p(1, 3),
            brightness: 0.756,
            style: CursorStyle::Corners,
        });
        s.path = vec![p(1, 3), p(1, 2)];
        assert_eq!(
            s.to_text(&Terrain),
            "origin (-1,2) size 5x3\n\
             \x20  2: - grass>water grass+m grass+m>?998 sand+da\n\
             \x20  3: - grass+h grass! ?999 -\n\
             \x20  4: -*5\n\
             units 2\n\
             \x20 #4 k (0,2)\n\
             \x20 #1 a (1,3)\n\
             cursor (1,3) corners 0.76\n\
             path (1,3) (1,2)\n"
        );
        // A threatened tile adds a line: the tile, then the threats'.
        s.threat = Some(ThreatView {
            tile: p(1, 2),
            enemies: vec![p(0, 2), p(9, -4)],
        });
        let text = s.to_text(&Terrain);
        let end = "path (1,3) (1,2)\nthreat (1,2) from (0,2) (9,-4)\n";
        assert!(text.ends_with(end), "{text}");
    }

    #[test]
    fn to_text_of_an_empty_scene_and_every_cursor_style() {
        let mut s = MapScene::<Piece>::new(p(0, -1), (2, 1));
        assert_eq!(
            s.to_text(&Terrain),
            "origin (0,-1) size 2x1\n  -1: -*2\nunits 0\ncursor -\npath -\n"
        );
        // A scene short of tiles marks them.
        s.tiles.truncate(1);
        assert!(s.to_text(&Terrain).contains("  -1: - !\n"));
        let styles = [
            (CursorStyle::Corners, "corners"),
            (CursorStyle::LargeCorners, "large-corners"),
            (CursorStyle::TileGlow, "glow"),
        ];
        for (style, name) in styles {
            assert_eq!(style.name(), name);
            s.cursor = Some(CursorView {
                pos: p(1, -1),
                brightness: 1.0,
                style,
            });
            let line = format!("cursor (1,-1) {name} 1.00\n");
            assert!(s.to_text(&Terrain).contains(&line), "{name}");
        }
        assert_eq!(CursorStyle::default(), CursorStyle::Corners);
    }
}
