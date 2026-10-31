---
id: "2359"
title: "Parked: Animated tiles and unit frames as data in the grid kit: terrain pictures that move by the screen's clock, and idle and walk clips"
type: feature
milestone: "Engine: real time"
model: sonnet-5 / gpt-6.1-terra
effort: high
status: blocked
blocked_by: ["2231"]
nick_input: none
completed:
---

# 2359 — Animated tiles and unit frames as data in the grid kit

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first (a game ticket asks, through the game's `engine-change` skill, for animated terrain tiles or for unit idle and walk frames as data; then this ticket is unparked with 2231). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engine GameMaker. Animated tiles are a
built-in of the tile maps they draw:

- Godot's `TileSetAtlasSource` gives a tile frames, a duration each and
  a speed, and a mode with random start times so tiles don't move in
  step
  (https://docs.godotengine.org/en/stable/classes/class_tilesetatlassource.html).
- Unity's 2D Tilemap Extras has `AnimatedTile`. GameMaker's tile sets
  animate. Tiled writes an `<animation>` in a TSX tileset.

Shuyi today:

- `crates/assets/src/tileset.rs`: `TerrainTiles` gives each terrain
  (`Look::tiles`), each corner mix (`CornerLayer::tiles`) and each tile
  layer (`TileLayer::tile`, `TileLayer::sides`) one `ImageRect`. It has
  no serde and no format of the engine's: a game's own format reads
  into it (the comment on its `AssetKind`). A new field would break that
  reader's struct literals.
- `crates/ui/src/grid/sprite/ground.rs` paints those rectangles as
  sprite items: `paint_tiles` (through `Ground::own`), `paint_corners`,
  `paint_singles`, and the tint wash in `paint_tints`.
- `MapScene::clock_ms` (`crates/ui/src/grid/scene.rs`) is the screen's
  animation clock, "what a skin or a unit painter moves its marks by".
  No skin reads it yet.
- `crates/ui/src/grid/walk.rs` hard-codes a unit's frames: `IDLE_FRAMES`
  at `IDLE_STEP_MS`, `WALK_FRAMES` at `WALK_FRAME_S`. Nothing in the
  engine calls `walk::idle_frame(clock_ms)` or `walk::gait(path, tiles,
  pace)`: a game's `UnitPainter` (`crates/ui/src/grid/skin.rs`) paints
  its units and picks their frames with them.
- `Ctx::idle_motion` (`crates/ui/src/ctx.rs`) is `Off` in tests, "so a
  screen left alone in a test looks the same a moment later", and the
  platform turns it `On`. The toy field (`Field::update` in
  `crates/ui/src/toy/grid.rs`) moves its clock either way.
- 2231 gives `AnimationClip`, `frame_at` and the `AnimationSet` kind.

Why it matters:

- **The game first.** Visions of Shuyi's maps are the grid kit's. No
  game ticket asks for moving tiles today, so this waits with 2231. It
  is what the game uses the day it wants moving water or its units'
  frames as data, and it changes nothing for the game until then: no
  API breaks and no snapshot changes.
- **Other 2D games.** The grid kit is the engine's only world kit
  before real time. A tactics game, a puzzle or a farming game on a
  grid animates water, torches and units waiting for their turn, and
  gets none of it today.
- **The end goal** (Nick, 2026-10-19): a game in few lines. Moving
  water is one RON file of clips (2231), one `TileMotion::from_set` and
  one builder call.

## Nick input

None.

## Scope

**In:**
- **`ImageRect`** (`crates/assets/src/image.rs`) derives `PartialOrd`
  and `Ord`, to be a map key.
- **`shuyi_ui::grid::motion`** (`crates/ui/src/grid/motion.rs`, tests
  in `crates/ui/src/grid/motion/tests.rs`, `pub mod motion;` in
  `crates/ui/src/grid.rs`), `no_std` with `alloc` like the rest of the
  crate:
  - `TilePhase` (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`,
    `Default`): `Together`, the default (every tile of a clip shows the
    same frame), or `ByTile` (each tile starts its clip at an offset of
    its own, as Godot's random start times, but the same on every run);
  - `TileMotion` (`Debug`, `Clone`, `PartialEq`, `Eq`, `Default`): a
    `BTreeMap` from a still picture's rectangle to a clip and its
    phase, with `pub const fn new() -> Self` (empty);
  - `TileMotion::insert(&mut self, still: ImageRect, clip:
    AnimationClip, phase: TilePhase)`; a later insert for the same
    rectangle wins;
  - `TileMotion::from_set(set: &AnimationSet, dir: &str, image:
    ImageId, phase: TilePhase) -> Result<Self, String>`: each clip of
    the set moves the picture that is its first frame as written.
    `dir` is the set file's folder (`AssetPath::dir`, `"anim"` for
    `anim/water.ron`): a set names its picture from its own folder
    (2231). It refuses, naming them, a set whose `set.image_path(dir)`
    isn't `image.path()` (its frames would be rectangles of another
    picture), a clip with no frame (the format refuses one, but a set
    built in code may have it) and two clips with the same first frame;
  - `TileMotion::picture(&self, still: ImageRect, at: Pos, clock_ms:
    u64) -> ImageRect`: `still` when it has no clip; otherwise
    `frame_at(clip, t)`, where `t` is `clock_ms` for `Together`, and
    `clock_ms` plus an offset for `ByTile`. The offset is
    `shuyi_core::state_hash(&at)` (`crates/core/src/hash.rs`: FNV-1a
    over a fixed form, the same on every platform; it returns a
    `Result`, which can't fail for a `Pos`, so an error counts as 0)
    modulo the clip's `cycle_ms()`, 0 for a 0 ms cycle; the sum
    saturates.
- **`SpriteSkin::with_motion(self, motion: TileMotion) -> Self`**
  (`crates/ui/src/grid/sprite.rs`), a `#[must_use]` builder like
  `with_zooms`, kept in a new private field. `SpriteSkin::new` keeps its
  signature and stays a `const fn` (the field starts as
  `TileMotion::new()`). `zoomed` and `at_zoom` clone the skin, so they
  keep the motion. Without terrain tiles (the glyph skin's ground) the
  motion does nothing.
- **The ground moves** (`crates/ui/src/grid/sprite/ground.rs`):
  `ground::paint` takes the skin's `&TileMotion`, `Ground` holds it and
  reads the clock from `scene.clock_ms`, and every rectangle it paints
  goes through `TileMotion::picture`:
  - a terrain's own picture in `Ground::own`, at its tile's map
    position (`scene.origin` plus `(dx, dy)`). The tint wash in
    `paint_tints` also uses `own`, so the wash has the shape of the
    frame shown;
  - a corner layer's picture in `paint_corners`, at the map position of
    the tile right of and below its corner point (`scene.origin` plus
    `(i, j)`);
  - a tile layer's picture in `paint_singles`, at its tile's map
    position (`scene.origin` plus `(dx, dy)`).
- **The clock while nothing should move.** The skin can't see `Ctx`, so
  the rule is the screen's: it leaves `MapScene::clock_ms` where it is
  while `ctx.idle_motion` is `Off`. Say so in the doc comment of
  `MapScene::clock_ms`, and do it in the toy field's `update`
  (`Field::update` in `crates/ui/src/toy/grid.rs`, which today adds the
  frame's milliseconds whatever `ctx` says; its `ctx` argument is
  unused now). The walk still moves: it goes by `Walk::t`, not the
  clock. A screen left alone in a test then paints the same tiles a
  moment later. **Either order with 2340:** if 2340 is done, the toy
  field and the doc comment read `Ctx::shows_idle_motion()` (2340),
  which is false while idle motion is off or the player's motion setting
  is `Reduced`, and never the setting itself; if not, they read
  `ctx.idle_motion`, and 2340 moves them onto `shows_idle_motion`.
- **Unit frames as data** (`crates/ui/src/grid/walk.rs`):
  - `UnitClips { pub idle: AnimationClip<u8>, pub walk:
    AnimationClip<u8> }`, deriving `Debug`, `Clone`, `PartialEq`, `Eq`,
    `Serialize` and `Deserialize`, with `#[serde(default)]` on the
    struct so a file that leaves a clip out gets the default's. A game
    puts it in a kind of its own, read with
    `shuyi_assets::ron::Ron<T>` (`crates/assets/src/ron.rs`);
  - `impl Default for UnitClips`, written by hand: the idle clip is
    `IDLE_FRAMES` at `IDLE_STEP_MS` each, the walk clip is `WALK_FRAMES`
    at 100 ms each (`WALK_FRAME_S`), both `Forward` and `Loop`;
  - `UnitClips::idle_frame(&self, clock_ms: u64) -> u8`:
    `frame_at(&self.idle, clock_ms)`, or `STANDING_FRAME` for an empty
    clip;
  - `UnitClips::gait(&self, path: &[Pos], tiles: f32, pace: f32) ->
    Option<Gait>`: as `gait`, with the frame from the walk clip
    (`STANDING_FRAME` for an empty clip):
    - when every frame of the clip lasts the same `ms`, count the frames
      shown as `gait` does today, with that frame time in seconds
      (`ms / 1000`, which for 100 ms is exactly today's `0.1_f32`), and
      ask the clip at `shown × ms` (saturating). Convert `ms` without a
      lossy cast (`u16::try_from`, then `f32::from`); a frame over
      65 535 ms takes the next path;
    - otherwise ask the clip at the walk's time: `tiles / pace × 1000`,
      rounded down to whole ms, with `tiles` cleaned as `gait` cleans it
      (0 for a negative or non-finite distance) and 0 ms for a pace that
      isn't a positive finite number;
  - the free `idle_frame` and `gait` keep their arithmetic (a private
    helper may take the frame time as a parameter), so a game that calls
    them sees no change, and `walk.rs`'s existing tests pass unchanged;
  - the module docs of `skin.rs`, where "What the painter needs is kept
    beside the skin", name `UnitClips` as one such thing.
- **Docs:** `crates/ui/src/grid.rs`'s module docs mention moving tiles
  and unit clips.
- **For the game:** no API break. Everything is new except the derives
  on `ImageRect`. `TerrainTiles`, `Look` and `SpriteSkin::new` don't
  change, and the game's PR that moves the pin needs no edit for this.
  The PR says so.
- **Size.** If the PR passes about 600 changed lines (not counting
  fixtures and snapshots), `UnitClips` goes to a follow-up ticket,
  parked like this one, and this PR keeps the moving tiles and the
  clock rule.

**Out (do not do):**
- adding fields to `TerrainTiles` or `Look`;
- the clip type, `frame_at` and the `AnimationSet` format (2231); the
  Aseprite and TexturePacker importers (1123); particles (2103);
- the glyph skin cycling a terrain's glyphs (a follow-up if a game
  asks);
- moving tiles in a `MapPicture` (`crates/ui/src/grid/picture.rs`): it
  paints the map once, at its scene's clock, and stays a still picture;
- Tiled's TSX `<animation>` read into clips (1124);
- the tile-layer painter of real-time maps and its animated tiles
  (2360);
- a clip per facing, and which clip plays when (2104);
- the game's own unit painter: it moves to `UnitClips` in a game PR of
  its own, if it wants to.

## Implementation steps

1. Read 2231's `anim` module, `crates/ui/src/grid/sprite.rs`,
   `sprite/ground.rs` and its tests, `sprite/tests.rs`, `walk.rs`, the
   toy field (`crates/ui/src/toy/grid.rs`) and its Harness tests
   (`crates/ui/tests/it/grid.rs`).
2. Write `motion.rs` with its unit and property tests.
3. Add `with_motion`, and pass every rectangle in `ground.rs` through
   the motion. Add the ground tests to the `tests` module of
   `ground.rs`, with the toy's skin (`sprite(Some(layered(..)))` from
   `crate::toy::grid`) given a motion by `with_motion`.
4. Write `UnitClips` and its tests in `walk.rs`'s `tests` module.
5. Hold the toy field's clock while idle motion is off (with 2340, while
   `Ctx::shows_idle_motion()` is false), and write the docs.
6. Run the gates.

## Acceptance criteria

- [ ] With no motion, every sprite skin test and every `frame-png`
      snapshot is unchanged.
- [ ] With `TilePhase::Together`, a terrain picture, a corner layer
      picture and a tile layer picture, each with a two-frame clip of
      200 ms frames, paint the first frame's `src` at clocks 0 and 199
      and the second's at 200 and 399 (sprite items).
- [ ] With `TilePhase::ByTile`, two tiles of the test scene show
      different frames at one clock, and the values are pinned in the
      test. Panning the view (another `scene.origin`) keeps each map
      tile's frame.
- [ ] The tint wash over a moving tile has the `src` of the frame shown.
- [ ] `TileMotion::from_set` takes a set read from `anim/water.ron`
      whose `image` is `water.png`, with `dir` `"anim"` and the
      `ImageId` of `anim/water.png`. It refuses a set of another image,
      a clip with no frame, and two clips with one first frame, naming
      them.
- [ ] `UnitClips::default()` gives exactly what `idle_frame` and `gait`
      give, for clocks from 0 to 10 000 ms and for random paths,
      distances and paces (property test).
- [ ] A walk clip of frame 0 for 100 ms then frame 2 for 300 ms, on a
      path of 4 tiles in a row at a pace of 5 tiles a second, shows
      frame 0 at 0.25 tiles, frame 2 at 1.0 tile and frame 0 at 2.25
      tiles.
- [ ] A `UnitClips` read from RON with only `idle` given has the
      default walk clip.
- [ ] In the Harness, the toy field's `clock_ms` stays 0 after
      `h.wait(1.0)` with `toy::ctx()` (idle motion off), and is at
      least 1000 with `ctx.idle_motion = IdleMotion::On` (each frame of
      `FRAME_DT` adds its rounded 17 ms). If 2340 is done, it also stays
      0 with idle motion on and the player's motion setting `Reduced`.
- [ ] CI's `no_std` build of `shuyi-ui` for `thumbv7em-none-eabihf`
      passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `TileMotion::picture` with no clip, `Together` and `ByTile`;
  `from_set`'s refusals; the three kinds of picture in the ground; the
  tint wash; `UnitClips` with uneven frames and an empty clip; its RON
  defaults.
- Property: `TileMotion::picture` is the same twice for any position and
  clock, and always one of the clip's frames or the still picture;
  `UnitClips::default()` against the free functions.
- Snapshot / integration: the existing sprite skin tests and `frame-png`
  snapshots, unchanged; the toy field's clock in
  `crates/ui/tests/it/grid.rs`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
