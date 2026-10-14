# ADR-0004: Assets: standard files in, the engine's own data out; a deterministic movement kit, physics optional

- **Status:** Accepted (Nick chose plan B, 2026-10-14; see `docs/decisions.md`). First proposed as the game repository's ADR-0065 (game ticket 1101)
- **Date:** 2026-10-14
- **Related tickets:** 1103–1105 (asset types move), 1122 (collision and
  movers), 1123 (animation and Aseprite), 1124 (Tiled and LDtk), 1128
  (optional physics), 1011 (mods: runtime layers)
- **Extends:** game ADR-0005 (content is data, validated with file and line),
  game ADR-0013 (licenses), game ADR-0026 (audio cues), game ADR-0049, game ADR-0052 and
  game ADR-0056 (pictures from their own files), game ADR-0060 (drawn from the time
  alone), ADR-0003 (fixed-point simulation)

## Context

Nick, 2026-10-14: the engine should give "good support for loading
assets, raw or via normal tileset/sprite files, as well as animations,
cutscenes, audio cues, collision, physics, etc. It should be a serious 2D
game engine."

What exists: PNG pictures named by RON sidecars (portraits, map sprites,
combat pictures, tilesets with autotile layers), a bitmap font atlas made
from BDF, audio cues by id with music beside the game, timed cinematics,
the `.dlg` dialogue format, the `.map` text format, and an `xtask`
importer for Tiled `.tsx` terrain. Everything is embedded at compile time
from fixed folders (the game repository's [boundary map](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/boundary.md), coupling 9). There is no
animation format beyond the 3×4 walk sheet, no collision beyond tile
occupancy, no physics.

What the research found (1101, sources in the game repository's [feasibility report](https://github.com/Zafnok/visions-of-shuyi/blob/main/docs/engine/feasibility.md)):

- `tiled` (MIT) reads TMX/TSX in pure Rust, but only with default
  features off (its default `zstd` builds C code).
- LDtk's own crates are stale (`ldtk_rust` 2022, `ldtk2` 2024). LDtk
  publishes a JSON schema and generated serde types; the editor is MIT.
- `asefile` (MIT, last release 2024) reads `.aseprite` files. Aseprite's
  CLI exports a sheet plus JSON with frame tags, the common route.
- `rapier2d` (Apache-2.0, pure Rust) is bit-for-bit deterministic across
  platforms, WASM included, with its `enhanced-determinism` feature.
- `kira` (MIT/Apache) would be a better mixer, but its default features
  pull `symphonia` (MPL-2.0, denied by game ADR-0013), and its web backend
  likely needs `wasm-bindgen`, which macroquad's web loader doesn't use.

## Decision

### 1. Where files come from

`shuyi-assets`' bundle (1103) is a list of **layers** the game gives at
start-up, searched in order: embedded folders (as today: the game's,
private art, test files) and, new, **folders on disk**. A disk layer
serves mods (1011) and, in debug builds, **hot reload**: `shuyi-app`
watches the folder and the game's `Ctx` reloads changed files and shows
their errors in the debug overlay instead of crashing. A shipped build
embeds everything, as game ADR-0004 rule 4 says.

### 2. Standard files are read directly, into the engine's own types

Importers live in `shuyi-assets` and run at load time, not as a build
step, so modders and hot reload get the same path. Each reports every
error with file, line or JSON path, and a message (game ADR-0005 rule 1).

| What | Files read | Into |
| ---- | ---------- | ---- |
| Pictures | PNG | `ImageId` + size (today) |
| Sprite sheets | our RON sidecars (today); Aseprite JSON export (`json-array` or `json-hash`, with `frameTags`); TexturePacker JSON | `Atlas`: named rectangles in an image |
| Animations | Aseprite frame tags (direction, repeat, per-frame duration); our RON | `AnimationSet` of `AnimationClip`s |
| `.aseprite` itself | via `asefile`, behind an `aseprite` feature, only if its WASM size cost is under 300 KB (1123 measures) | the same `Atlas` + `AnimationSet` |
| Tile maps | Tiled TMX/TSX via `tiled` with `default-features = false`; LDtk `.ldtk` via serde types generated from LDtk's published schema, vendored with its MIT notice; our `.map` (grid kit) | `TileMap`: tile layers (a `Grid` of tile ids per tileset), object layers (named boxes and points with properties), the map's properties |
| Tilesets | our tileset RON (game ADR-0049, game ADR-0052); Tiled TSX | `Tileset` (+ autotile corners where given) |
| Fonts | BDF, through `shuyi-cli font-atlas` | atlas PNG + RON (game ADR-0016) |
| Audio | cue manifest RON; WAV and OGG (and what `shuyi-app` decodes today) | cues by id (game ADR-0026) |
| Text and dialogue | language packs (game ADR-0045); `.dlg` (game ADR-0011, game ADR-0055) | text by key; scenes |
| Cinematics | timeline RON (game ADR-0060); shot kinds registered by the game | timelines |

- Our own text formats stay first-class. They are the ones an agent or
  a person can read and diff, and every importer's output can be printed
  back as them (`shuyi-cli import --print`), so a map drawn in Tiled can
  be checked in as text if the game prefers.
- Ids are strings, as everywhere (game ADR-0005 rule 2). An importer never
  invents ids from file positions; Tiled and LDtk names and custom
  properties become ids.

### 3. Animation is a function of time

`AnimationClip` = frames (atlas rectangle, duration in ms) + direction
(forward, reverse, ping-pong) + repeat (loop, once, n times). The frame
shown is a pure function of the clip and the time since it started, as
game ADR-0060 decided for cinematics. A tick simulation stores `(clip,
start_step)` per thing and nothing else, so replays and rollback are
exact and a test can draw any moment. Easing curves come from the
cinematic module. Particles follow the same rule: an emitter's particles
are a function of its seed and the time, never stored one by one.

### 4. Collision and movement: a small deterministic kit in `shuyi-core`

All in `Fx` fixed-point (ADR-0003 §3), with no floats:

- `TileCollision`: a grid of solid, empty and one-way-up tiles (slopes
  later, when an example needs them), built from a tile map's layer or
  the `.map` terrain.
- `Aabb` boxes, and a uniform-grid broad phase for "who overlaps whom".
- **Movers** step one pixel at a time and stop at what they hit,
  reporting it (`move_x` / `move_y`, as TowerFall and Celeste do): no
  tunnelling, no overlap to resolve afterwards, exact under replay.
- **Triggers**: a box that reports enter and leave as events.
- A top-down character controller (8 directions, analog speed) and a
  platformer controller (gravity, coyote time, jump buffer, variable
  jump height, one-way platforms) as tested, documented examples, data
  for their numbers.

### 5. Physics is optional

`shuyi-physics` (1128) wraps `rapier2d` with `enhanced-determinism` (and
never `simd8`) for games that want rigid bodies (crates to push, ropes,
ragdolls). Positions cross from `Fx` to rapier's floats at its edge.
It isn't in the default engine: most 2D games in Nick's list (tactics,
top-down, platformers) are better with the kit in §4, and rapier adds
size and floats.

### 6. Audio stays as it is

Cues by id, music streamed beside the game, the music clock (game ADR-0026,
0028, 0037), behind `shuyi-app`'s `Backend` trait. Positional sound
(pan and volume from the distance to the camera) is added to the cue
request when a tick example needs it. `kira` is reconsidered only with
default features off (no MPL) and once its web backend is shown to run
under macroquad's loader.

### 7. Every dependency passes game ADR-0013

New dependencies are added with `default-features = false` and only the
features named here, and pass `cargo deny`. Named traps: `tiled`'s
`zstd` (C code), `kira`'s `symphonia` (MPL-2.0), `mlua` (C, emscripten
only on the web).

## Consequences

- A developer with a Tiled or LDtk map and an Aseprite sheet gets a
  walking, animated character on a collidable map without writing a
  loader, and their errors name the file and the place in it.
- Hot reload changes the edit loop for art and maps: save in Aseprite or
  Tiled, see it in the running game.
- Three importers are code we maintain against formats we don't control.
  Each is pinned to a format version, with fixture files from the tool
  in `crates/assets/tests/fixtures/`, and fails with a clear
  message on a newer version.
- The movement kit is opinionated (pixel stepping, integers). Games that
  want floats and impulses use the physics crate, and lose cross-platform
  replay unless they keep rapier's determinism feature.

## Alternatives considered

- **Convert standard files to our own formats in a build step** (as
  `xtask tileset-import` does today) — simpler loaders, but modders and
  hot reload would need the build tool, and a second copy of every map
  drifts from its source. Kept only for bought art, whose originals we
  may not ship (game ADR-0040).
- **`bevy_ecs_ldtk` and Bevy's asset server** — Bevy-only.
- **Our own `.aseprite` parser** — the format spec is public, but
  `asefile` exists and is MIT; the JSON export covers most users anyway.
- **`rapier2d` as the default movement** — floats and rigid bodies make
  platformers feel floaty and need tuning; pixel-stepping movers are the
  genre's norm and replay exactly.
- **`kira` now** — the MPL default and the web backend make it a risk
  for no gain the current audio lacks.
