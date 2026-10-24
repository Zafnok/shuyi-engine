---
id: "2122"
title: "Parked: Curves in fixed point: lines, Bézier and Catmull-Rom baked by length, a curve file, and a follower that stops, loops or ping-pongs"
type: feature
milestone: "Engine: real time"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["1119", "2335"]
nick_input: none
completed:
---

# 2122 — Curves in fixed point and a follower

> **Parked** (Nick, 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): this is real-time work, so it waits until Visions of Shuyi launches, unless the game needs it first (its ticket 0060 says the army walks around freely, which unparks 1119 too, or the game asks through its `engine-change` skill for something that moves along a curved line, such as a marker travelling between two places on a map). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and with GameMaker and Construct 3. Each has a curve a
thing can follow:

- Godot: `Path2D` holds a `Curve2D` (Bézier), and `PathFollow2D` moves
  along it by distance, looping or not
  (<https://docs.godotengine.org/en/stable/classes/class_pathfollow2d.html>);
- Bevy: `bevy_math`'s `cubic_splines` module (Bézier, and
  `CubicCardinalSpline`, which is Catmull-Rom) and its `curve` module
  (<https://docs.rs/bevy_math/latest/bevy_math/cubic_splines/index.html>);
- Unity: the Splines package and its `SplineAnimate` component. Unreal:
  the spline component;
- GameMaker: path assets with a speed per point. Construct 3: the Move
  To behaviour with waypoints.

What Shuyi has today:

- `crates/core/src/path.rs`: one Dijkstra over a square grid
  (`reachable`, `flow_field`), tile to tile, for turn games;
- `crates/ui/src/cinematic.rs`: `Pan` and `ease_in_out`, in `f32`. They
  move a shot's picture in a straight line. They draw, and never drive
  a simulation;
- no fixed-point numbers and no angle (`crates/core/src/lib.rs` has
  neither module). `Fx` and `FxVec2` are 1119's `shuyi_core::fx`. The
  integer `Angle(u8)` and its sine table are 2335's
  `shuyi_core::angle`.

What is planned:

- 2106 (parked): `nav::Path { points: Vec<(Fx, Fx)> }` and
  `Path::follow`, a line of corners that steers a mover round walls;
- 1124 (parked): a map's object layers keep polylines and polygons,
  each as its points in `Fx` pixels, beside boxes and points;
- 2707 (parked): the real-time strategy `Patrol`, which swaps between
  two ends;
- 2701 (parked) plans `Fx::sqrt`, `FxVec2::length` and `Angle::atan2`.
  2106 takes `Fx::sqrt` from 2701 or adds it, either order;
- 2738 (parked) flies its shooter's foes on a `Curve` with
  `CurveFollower` if this ticket is done, and 2362 leaves camera rails
  here.

Nothing holds a curve as data, measures it by length, or moves a thing
along it in a loop or back and forth.

**Why it matters.** Visions of Shuyi is a grid tactics game and doesn't
need this now. Other 2D games do: patrols in a platformer (1127), enemy
flight paths in a shooter (2738), a guard on a fixed beat in a top-down
game, and a rail that 1120's camera follows. For the end goal (Nick,
2026-10-19: an agent makes a game "in not too many LOC"), a curve is a
few RON lines and one call a step, and it replays the same on every
target, because it is all integers.

**Names.** The type is `Curve`, in `shuyi_core::curve`, so it doesn't
clash with 2106's `nav::Path`. 2109's ease curves map time to a
fraction and are named `Ease`; a `Curve` maps a distance to a place.

**One follower.** `CurveFollower` moves by distance along a curve, and
nothing stops it: a patrol on a beat, a foe's flight, a camera rail.
2106's `Path::follow` is a different job. It steers a mover from where
collision left it toward the next corner, so it takes the mover's real
position. The two share one kind of data: 2106's route becomes a
polyline `Curve` (below). A thing that nothing should stop uses
`CurveFollower`, never a second follower.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::curve`** (`crates/core/src/curve.rs`, tests in
  `crates/core/src/curve/tests.rs`, as `path` keeps its own), pure,
  `no_std`, integers only:
  - `Curve`, built by one of:
    - `Curve::lines(points: &[FxVec2], closed: bool)`: straight pieces
      through every point;
    - `Curve::bezier(points, closed)`: cubic Bézier pieces, written
      anchor, handle, handle, anchor, and so on. An open curve has
      `3n + 1` points (`n >= 1`). A closed one has `3n` (`n >= 2`), its
      last two handles leading back to the first anchor. The anchors
      are `points[0]`, `points[3]`, `points[6]` and so on;
    - `Curve::catmull_rom(points, closed)`: through every point,
      turned into cubic Bézier pieces. It is uniform Catmull-Rom: the
      handles of the piece from `p1` to `p2` are `p1 + (p2 - p0) / 6`
      and `p2 - (p3 - p1) / 6`, divided as `Fx` divides (toward
      negative infinity). An open curve repeats its end points.
  - Each returns `Result<Curve, CurveError>`. `CurveError` says which
    point (by its index in `points`) is wrong:
    - too few points (fewer than 2 for lines and Catmull-Rom, 3 when
      closed) or too many (more than 1024);
    - a Bézier count that isn't `3n + 1` (open) or `3n` (closed), as
      above;
    - two anchors in a row at one place (a piece with no length). A
      closed curve whose last anchor repeats its first is this error
      too, because closing adds that piece;
    - a point more than 2^20 px from the origin on either axis, so
      every difference fits 2^21 px and every square sum fits `i64`;
    - a whole length over what `Fx` holds.
  - No quadratic piece. A cubic holds one, and one kind less is less
    for an agent to learn.
  - **Baked by length** when it is built: a table of points and the
    distance from the start to each.
    - A straight piece is one entry.
    - A cubic is cut into `n` pieces, `n` being its control polygon's
      length in whole pixels divided by 8, at least 4 and at most 64.
      Each point is evaluated exactly at `t = k / n` in `i64` (the
      Bernstein form times `n³`, divided once at the end with
      `div_euclid`, so it rounds toward negative infinity, as `Fx`
      does).
    - A table piece of length 0 (a cusp can make one) is dropped.
    - Each piece's length is the floor of the root of `dx² + dy²` in
      raw `Fx` units, worked in `i64` with `u64::isqrt`, so it never
      overflows for the differences the point limit allows. If the
      `FxVec2::length` this ticket finds or adds squares through `Fx`
      and can overflow there, the table uses a private `i64` function
      instead.
  - `Curve::length() -> Fx`.
  - `Curve::sample(distance: Fx) -> CurvePoint { pos: FxVec2, angle:
    Angle }`: a binary search in the table, then a straight step
    between the two table points around `distance`, in `i64`, rounded
    toward negative infinity. At a table point it is that point
    exactly. The angle is the direction of the table piece that starts
    at or before `distance` (the last piece at the end of an open
    curve), through `Angle::atan2`. An open curve clamps the distance
    to `0..=length`; a closed one wraps it (`rem_euclid`).
  - `Curve::point_distance(i) -> Fx`: how far along the curve authored
    point `i` is. `i` counts anchors: every point of a lines or
    Catmull-Rom curve, and every third point of a Bézier (its handles
    are not points).
  - `Curve::point_count() -> usize`, the number of authored points, and
    `Curve::pieces()`: the baked points in order, for drawing.
  - `Curve`, `CurveFollower` and `CurvePoint` are re-exported from the
    crate root, beside `Reach` and the rest.
- **`CurveFollower`**, in the same module: `{ distance: Fx, speed: Fx,
  end: FollowEnd }`, with `FollowEnd` one of `Once`, `Loop` and
  `PingPong`, and `CurveFollower::new(speed, end)` starting at 0. It
  holds no curve: the game keeps the curve (an asset, or built in code)
  and passes it to each call, so a state that holds followers stays
  small.
  - `step(&mut self, curve: &Curve) -> Vec<FollowEvent>` moves `speed`
    along the curve. A negative speed goes back. A tick game calls it
    once a step. Events come in the order they happen.
    - `Once`: it stops at the end it reaches, its speed becomes 0, and
      it reports `Ended`. On a closed curve the end is one lap, at
      `length`.
    - `Loop`: what is left of the step past an end carries over from
      the other end, and it reports `Wrapped`. On an open curve it
      jumps to the other end, and the point it lands on is reported
      after `Wrapped`. A closed one goes on round.
    - `PingPong`: it turns at either end and moves the rest of the step
      the other way. Its speed's sign flips, and it reports `Turned`.
    - `Passed(i)` for every authored point reached or crossed in the
      step. The interval is half-open: where the step started is left
      out and where it ends is in, so a point is never reported twice
      in a row. On a closed curve, point 0 is passed at `length`.
    - A step moves at most the curve's length less 1/256 px (a larger
      `|speed|` is clamped for that step), so one step meets at most
      one end.
  - `point(&self, curve: &Curve) -> CurvePoint`.
  - `CurveFollower`, `FollowEnd` and `FollowEvent` derive `Clone`,
    `Copy`, `PartialEq`, `Eq`, `Debug`, `Hash`, `Serialize` and
    `Deserialize`. `Passed` holds a `u16` (a curve has at most 1024
    points), so nothing depends on the width of `usize`. A `TickSim`
    state that holds a follower hashes it (`shuyi_core::state_hash`).
    If 1119's `Fx` lacks one of these derives, add it there.
  - A view that draws from the time alone (ADR-0004 §3) needs no
    follower: it calls `sample` with the speed times the steps since
    it started. The module docs say so.
- **Public types (ADR-0039):** the new public types follow it
  (`#[non_exhaustive]` and constructors); `CurveError`, `FollowEnd`,
  `FollowEvent` and `CurveKind` are `#[non_exhaustive]`.
- **The `fx` functions this needs. Either order with 2701 and 2106:**
  `Fx::sqrt`, `FxVec2::length` and `Angle::atan2`. If 2701 or 2106 is
  done, use what it added. If not, add the missing ones here as 2701
  specifies (the floor of the exact root, through `i64`; `atan2` to
  2335's `Angle`), with 2701's property tests and its checked-in table
  compared on native and `wasm32`, and change 2701's and 2106's text to
  say they exist.
- **A curve file**, `shuyi_assets::curve` (`crates/assets/src/curve.rs`,
  tests in `crates/assets/src/curve/tests.rs`), as ADR-0007 and
  ADR-0019 say:
  - the kind `CurveFile { kind: CurveKind, closed: bool, points:
    Vec<(i32, i32)> }`, `AssetKind::NAME` `"curve"`, with points in whole
    world pixels and `CurveKind` one of `Lines`, `Bezier` and
    `CatmullRom`. `CurveFile::curve() -> Result<Curve, CurveError>`
    builds it, through `Fx::from_px`;
  - the format `CurveRon`: id `curve`, extension `ron`, limit
    `TEXT_MAX_BYTES`. It needs nothing of the game, so it goes in
    `Formats::register_standard` (`crates/assets/src/format.rs`). Files
    are named `<name>.curve.ron` by habit; the kind asked for picks the
    format, as for every RON kind (ADR-0019 §2);
  - `decode` reads the RON with `crate::ron::parse_ron`, then builds
    the `Curve` once, and turns each `CurveError` into a `ContentError`
    naming the file and the point by its index. Its line is the line
    of that point's `(` after `points:` in the source, found by a small
    private helper (`error::line_of` finds only the first match of a
    text, and two points may be written alike). When the helper finds
    none, the error has no line;
  - `CurveFile` derives `Serialize`, `Deserialize`, `PartialEq` and
    `Debug`, so a game can bake it into a pack (ADR-0023 §4). In
    `crates/assets/src/pack/tests.rs`, add a fixture to `sources()`,
    the kind to `every_kind()`, and a line to
    `the_binary_form_of_every_engine_kind_is_pinned_by_the_pack_version`.
    A new kind changes no existing kind's binary form, so
    `pack::VERSION` stays; the PR says so;
  - `crates/assets/src/format/builtin_tests.rs`: `named(&CurveRon,
    "curve", &["ron"], TEXT_MAX_BYTES)`, and `CurveRon` in the
    property test that random bytes and edited fixtures never panic a
    format;
  - `Importers::standard()` (`crates/cli/src/contract/import.rs`, line
    170) gains the kind, with a summary such as `catmull-rom, open, 4
    points`, so `shuyi import` reads a curve file;
  - `docs/formats.md`: a row in the table of standard formats, and
    `curve::CurveFile` among "the engine's other kinds". The crate's
    module docs (`crates/assets/src/lib.rs`) name the module;
  - a game whose `GameTools::check` loads its curves gets their errors
    in `shuyi check --json` with nothing more.
  - **The golden row:** 2116 is done before this ticket starts (1119
    is blocked by it), so add the row `curve: curve` to
    `crates/assets/tests/golden/formats.ron`, as its guard asks of a
    new standard format.
  - **Either order with 2515:** if 2515 is done, `CurveFile` derives
    `JsonSchema` behind the `schema` feature, `CurveRon` implements
    `TextSchema` with an example in `crates/assets/kinds/curve.ron`, and
    `Importers::standard()` describes it. If not, change 2515's text to
    list `CurveRon` among the standard RON formats it describes.

  A file looks like this:

  ```ron
  (
      kind: CatmullRom,
      closed: false,
      points: [(0, 0), (48, -24), (96, 0), (144, -24)],
  )
  ```
- **Either order with 2106:** if 2106 isn't done, change 2106's text in
  this PR: its `nav::Path` holds a polyline `Curve`
  (`Curve::lines(points, false)`) instead of its own `points`, and
  `Path::follow` reads its corners from it. If 2106 is done, write a
  follow-up ticket (the `write-ticket` skill, parked with real time)
  that makes that change with 2106's tests unchanged.
- **Either order with 1124:** 1124's object layers keep polylines and
  polygons. If 1124 is done, add one function beside its `TileMap`
  that turns a polyline object into `Curve::lines(points, false)` and a
  polygon into `Curve::lines(points, true)`, tested on its fixture made
  in Tiled. If not, nothing to do: 1124 adds it (its clause with this
  ticket says so).
- **The debug view, if 2315 is done:**
  `shuyi_ui::debug::curve::gizmos(&Curve, Option<&CurveFollower>,
  to_px, &mut Gizmos)` pushes, on a channel `curve`:
  - a `Line` per baked piece (`GizmoRole::Path`);
  - a `Cross` on each authored point (`Note`);
  - a `Cross` where the follower is (`Note`).

  If 2318 is done, it pushes through `Gizmos::push_world`. If not, the
  caller gives an offset, as 2317 does. If 2315 isn't done, write a
  follow-up ticket (parked with real time) for the helper.
- **The example:** in 1119's tick example (a dot moved by input; the
  dot's room if 1122 is done), a second dot patrols a `CatmullRom`
  curve with `PingPong`. The curve comes from a `.curve.ron` fixture
  read through `Formats::standard()`. If 1119's example lives in
  `shuyi-core`, which can't reach `shuyi-assets`, the example builds the
  same curve with `Curve::catmull_rom`, and a test in `shuyi-assets`
  checks the fixture reads to an equal curve. A test dumps the patrol's
  place every 10 steps for 120 steps as a snapshot, and the example's
  replay hash covers it.
- **The module docs** say what needs no code here:
  - a camera rail is a follower that 1120's camera follows;
  - a sprite turned along the curve takes `CurvePoint::angle` as
    2335's sprite angle;
  - a skin draws a curve as lines over `pieces()` (2336's
    `Shape::line`, if 2336 is done).

**Size.** About 600 changed lines, not counting fixtures, tables and
snapshots, when 2701 or 2106 has added the `fx` functions. If this PR
adds them and the diff passes about 700, move the debug helper, then
the example, to follow-up tickets (the `write-ticket` skill, parked
with real time) and say so in the Completion notes.

**Out (do not do):**
- a speed per point (GameMaker's model). A game changes `speed` when it
  hears `Passed(i)`. Add it when an example needs it;
- moving 2738's shooter onto `Curve`s when the shooter lands first:
  its foes keep their straight paths, and nothing is left here;
- quadratic pieces and B-splines; curves in 3D (no 3D ticket has them:
  a follow-up on 3101's `FxVec3` if a 3D game asks);
- moving platforms that carry what stands on them (a moving solid that
  pushes and carries 1122's movers). Neither 1122 nor 1127 has it; it
  is a ticket of its own, parked with 1127, when a level needs one;
- routes round walls, and steering a mover that collision can stop
  (2106);
- reading polylines and polygons from Tiled and LDtk (1124's, as
  above);
- camera feel (1120, 2362), rotated sprites (2335), drawn shapes
  (2336), keyframed tracks in cinematics (2364), ease curves over time
  (2109), and game speed and hit-stop (2123: a follower steps once per
  simulation step, whatever the speed);
- the shared determinism job and its golden tables (2116);
- an editor for curves (none is planned; the parked editor is
  2506–2508).

## Implementation steps

1. Read 1119's `shuyi_core::fx`, 2335's `shuyi_core::angle`, 2701's
   text on `Fx::sqrt` and `atan2` (and its code, if done), 2106's `nav`
   (if done), `crates/core/src/path.rs` (module and test layout),
   `crates/assets/src/names.rs` (a small RON format),
   `crates/assets/src/format/builtin_tests.rs`,
   `crates/assets/src/pack/tests.rs` and
   `crates/cli/src/contract/import.rs`.
2. Reuse or add the `fx` functions.
3. Write `Curve`: `CurveError`, the three builders, the table,
   `sample` and `point_distance`, with unit tests on hand-made curves.
4. Write `CurveFollower` and its events.
5. Write `CurveFile` and `CurveRon`, register it, and add the error
   fixtures, the random-bytes test, the pack pin, the importer's
   summary and the rows in `docs/formats.md`. Add 2116's golden row,
   and do the 2515 part if it is done.
6. Add the example, the debug helper if 2315 is done, and the 1124
   function if 1124 is done.
7. Change 2106's, 2701's and 2515's texts, or write the
   follow-up tickets, as the either-order clauses above say.
8. Run the gates, including the `no_std` builds.

## Acceptance criteria

- [ ] On a curve of one straight piece, `sample(d)` is within 2/256 px
      on each axis of the point at distance `d` along it, computed in
      `f64` in the test (property test over random pieces).
- [ ] For any curve of any kind and any `d1 <= d2`, the straight
      distance between `sample(d1)` and `sample(d2)` is at most
      `d2 - d1`, plus 1/256 px for each table piece the span touches,
      plus 2/256 px (property test).
- [ ] `sample(0)` is the first point, and `sample(length)` is the last
      point (open) or the first (closed), exactly.
- [ ] For every kind, `sample(point_distance(i))` is authored point `i`
      exactly.
- [ ] For `0 < |s| < length`: after `k` steps, a `Loop` follower on a
      closed curve at speed `s` is at `(k × s).rem_euclid(length)`; a
      `PingPong` follower never leaves `0..=length` and reports at most
      one `Turned` a step; a `Once` follower stops exactly on its end
      and reports `Ended` once (property tests).
- [ ] Between two `Wrapped`, a `Loop` follower reports every authored
      point by `Passed` exactly once, in order, on open and closed
      curves (property test).
- [ ] Each bad curve fixture (too few points, a wrong Bézier count, two
      anchors at one place, a point too far) gives an error naming the
      file, the point's index and its line; a RON syntax error names
      the line.
- [ ] Random bytes, and the fixtures with a few characters changed,
      never make `CurveRon` panic.
- [ ] A checked-in table of `sample` results on three fixture curves is
      equal on native and `wasm32`: a golden table through 2116's
      `shuyi_assets::golden` (under `crates/core/tests/golden/`, as
      1119's `fx.ron` is), checked by CI's `test` and `determinism`
      jobs.
- [ ] `shuyi import` of a curve fixture answers its summary.
- [ ] The pack's pinned binary form lists `CurveFile`, and
      `pack::VERSION` is unchanged.
- [ ] `shuyi-core` and `shuyi-assets` still build for
      `thumbv7em-none-eabihf`.
- [ ] The example's replay hash is the same on every run, and its
      patrol snapshot is committed.
- [ ] 2106's text, or its follow-up ticket, says what the either-order
      clause asks.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each builder on a hand-made curve; each `CurveError`; a cusp
  dropping a table piece; `Passed`, `Wrapped`, `Turned` and `Ended` on
  a square of four lines; an open `Loop` jumping back to its start; a
  closed `Once` stopping after one lap; the line of a bad point when
  two points are written alike.
- Property: the criteria above; the same points always give the same
  table.
- Snapshot / integration: the patrol's places; the `sample` table on
  native and `wasm32`; the pack's pinned binary form; the importer's
  summary; the debug helper's gizmo list, if 2315 is done.

## Completion notes

*(Filled in by the session that completes the ticket.)*
