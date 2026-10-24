# The modules

The default engine is small: `shuyi-core`, `shuyi-assets`, `shuyi-ui`,
`shuyi-platform`, `shuyi-app` and `shuyi-cli`. Everything else is a
module, a crate a game turns on only when it needs it
([ADR-0009](adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)).

A game depends on the `shuyi` crate, which re-exports the default engine
and has one cargo feature per module, all off by default. `shuyi add
<module>` turns one on, and `shuyi modules` lists this catalog with the
modules a game has on. A module turned on joins the game through
`shuyi::enabled_modules`, which the templates already install.

## How to read it

- A **concern** is a question every game answers one way or another. A
  **direction** is the answers a stage picks, one per concern. The
  concerns and their answers are data, in [`modules.ron`](modules.ron).
- Two modules that answer the same concern are **alternatives**: they
  never drive the same thing in one stage. They can still share a game,
  in different stages, or on different things when their READMEs say
  how. *Kind of game* is the exception: its answers can share a stage.
- **Status:** *ready* (built, with an example passing CI), *planned*
  (ticketed, not built yet), or *parked* (not started until the plan
  says, [`ROADMAP.md`](ROADMAP.md)).
- Each module has a README in a fixed order: what it is for, what it is
  not for, what it costs, how to start, and how it sits beside its
  alternatives.

A row's name is its crate. A module's crate also says what it answers in
its `Cargo.toml`, under `[package.metadata.shuyi]`. `cargo xtask
check-modules` checks the two against each other: every crate outside
the default engine has a row here, a README and its metadata; every
answer it names is in `modules.ron`; alternatives name each other; and
every feature of `shuyi` names a row.

## Bodies and motion

The default engine's movers and drawn clips, or one of these.

| Module | Serves | Status | Tickets |
| ------ | ------ | ------ | ------- |
| `shuyi-physics` | rigid-body physics: side view, top-down free | parked | 1128 |
| `shuyi-procanim` | procedural animation: side view, top-down free | parked | 2701–2704 |

## Online play

None by default, or this.

| Module | Serves | Status | Tickets |
| ------ | ------ | ------ | ------- |
| `shuyi-net` | lockstep, rollback and a referee server, for turn and tick games | parked | 1140–1150 |

## Perspective and world drawing

The default kits draw a flat 2D world from a top-down grid, and later
top-down free, isometric, side view and hex grids. These draw it in 3D.

| Module | Serves | Status | Tickets |
| ------ | ------ | ------ | ------- |
| `shuyi-3d` | first person, third person, top-down 3D; full 3D drawing | parked | 3101–3109 |
| `shuyi-app-3d` | the GPU painter of `shuyi-3d`'s frames, over miniquad | parked | 3104 |
| `shuyi-diorama` | diorama (HD-2D): a top-down or isometric stage drawn in 3D | parked | 3107 |

## Kind of game

Kits built for one kind of game. The default engine's is tactics. These
answers can share a stage: a strategy game can tell its story as a
visual novel.

| Module | Serves | Status | Tickets |
| ------ | ------ | ------ | ------- |
| `shuyi-rts` | real-time strategy: top-down free, isometric | parked | 2705–2720, 2733–2734 |
| `shuyi-vn` | visual novels | parked | 2721–2726 |
| `shuyi-mystery` | mysteries: clues, people, places and the case file | parked | 2727–2732 |
| `shuyi-crawl` | dungeon crawlers: a first-person grid | parked | 2741–2744, 2753–2755 |
| `shuyi-rpg` | party RPGs: a party, rows, stats and levels | parked | 2745–2752, 2758, 2759 |
| `shuyi-action` | action RPGs: hits, hurts and loot in real time | parked | 2756, 2757, 2760–2762, 2764 |
| `shuyi-procgen` | roguelikes: noise and generated levels; its field of view is in `shuyi-core` (2120) | parked | 2118, 2119, 2735 |

## Format plugins

Formats are cargo features of `shuyi-assets` (ADR-0007), not modules:
a game turns them on in its own `Cargo.toml`.

| Format | Serves | Status | Tickets |
| ------ | ------ | ------ | ------- |
| `shuyi-assets` dialogue formats | Yarn Spinner and Ink, beside `.dlg` | planned | 2202 |
| `shuyi-assets` image and audio formats | JPEG, WebP, QOI, JPEG XL, FLAC, MP3, each behind a feature | planned | 2203 |
| `shuyi-assets` pack | `.shuyipak`, one indexed file of baked assets (always in) | ready | 2204 |
| `shuyi-assets` `sfx` | `.sfx.ron` sound effects from a few numbers | planned | 2225 |

## Platform backends

Backends are crates of their own (ADR-0008): a game's binary runs on one.

| Backend | Serves | Status | Tickets |
| ------- | ------ | ------ | ------- |
| `shuyi-app` | desktop, the web and the Steam Deck, over macroquad (default) | ready | — |
| `shuyi-playdate` | the Playdate | planned | 3010 |
| `shuyi-gba` | the Game Boy Advance | parked | 3021 |
| `shuyi-nds` | the Nintendo DS | parked | 3024 |
