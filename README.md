# Shuyi Engine (being carved out)

This folder holds the code of **Shuyi Engine**, the open-source 2D engine
that is being carved out of the game in this repository (ADR-0063 §1, §3).
Code moves here one layer at a time (tickets 1103–1117), with no change in
behavior; new generic work after a layer has moved is written here
directly. Once the carve is done and an example game runs from engine
crates alone (1118), ticket 1131 moves this folder to the engine's own
repository with its history.

The engine knows nothing about the game: no game crate, no game asset
folder, no class, character, story or Combat Art. `cargo xtask
check-engine-boundary` fails CI when something here does (ADR-0063 §4).

## Plans, ADRs and tickets

They live in the engine's repository,
[Zafnok/shuyi-engine](https://github.com/Zafnok/shuyi-engine): its ADRs
("engine ADR-NNNN"), its engine-only tickets, its roadmap and product
decisions. Only the carve's tickets (1102–1118, 1131) stay in this
repository, because they change this repository's code.

## The crates

Each goes in `engine/crates/<name>/` and sits under the game crate of the
same layer (ADR-0063 §2). ADR-0004's rules hold for each: the core is pure
and deterministic, only the app touches the platform, time is an argument,
and no `unsafe` outside the app. An engine crate never depends on a game
crate.

| Crate | Folder | Owns |
| ----- | ------ | ---- |
| `shuyi-core` | `core` | The seeded RNG, the simulation traits, and the tactics kit: grid, paths, turns. No I/O. |
| `shuyi-assets` | `assets` | Bundle layers, images, fonts, palette, audio cues, languages, keymaps, tilesets, importers, error reports. |
| `shuyi-ui` | `ui` | The frame, screens, input actions, widgets, skins, cinematics and the test Harness. |
| `shuyi-app` | `app` | The window, blitting, input devices, the audio backend, storage and the run loop. |
| `shuyi-cli` | `cli` | Tools: `frame-png`, `font-atlas`, `check-keys`, `check-text`, `web`, and the rest. |

## Third-party crates the engine may use

An engine crate may depend on another engine crate and on the crates
below, and nothing else. `cargo xtask check-engine-boundary` reads this
block. Adding one is a change to this list in the same PR, after checking
its license against ADR-0013.

```engine-deps
serde
ron
thiserror
include_dir
macroquad
quad-snd
gilrs
quad-storage
proptest
insta
```

## License

While they are in this repository, the files in this folder are under this
repository's `LICENSE`, like the rest of it. When 1131 moves them to the
engine's repository they are published under MIT OR Apache-2.0, the
license Nick chose for the engine.
