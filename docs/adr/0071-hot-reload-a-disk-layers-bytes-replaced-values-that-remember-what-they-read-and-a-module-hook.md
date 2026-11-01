# ADR-0071: Hot reload: a disk layer's bytes replaced, values that remember what they read, and a module hook

- **Status:** Accepted
- **Date:** 2026-11-01
- **Related tickets:** 2220 (this), 2207 (disk layers), 2358 (modules), 2516
  (`bundle::recording`), 2518 (session records), 2221, 2327 and 2330 (use it)
- **Extends:** ADR-0004 §1 (hot reload of disk layers in debug builds),
  ADR-0040 §2 (a new hook goes on `Module`), ADR-0040 §7 (`Platform`
  grows by defaulted methods)

## Context

- Every read of the bundle is `&'static` (`crates/assets/src/bundle.rs`), and
  a game keeps what it loaded in its own data, not in a store the engine
  owns. No engine code can swap a loaded value behind a game's back.
- `shuyi-assets` does no I/O, and only backends touch files and the clock
  (`AGENTS.md` rule 5).
- A format reads one file, and the files it names through `DecodeCx`, so what
  a value depends on is only known by decoding it.
- Bevy's asset server reloads a handle's asset and sends an event
  (<https://bevy.org>, "asset changes appear in running apps"). It has a store
  of handles, which the engine doesn't.

## Decision

### 1. The backend sees changes and replaces the bytes

- `Platform::changed_files(&mut self, out)`, defaulted to nothing. The paths
  are paths in the bundle, and the new bytes are already in their layers when
  they are told.
- `App` asks once a frame the game runs, before its update, and only with
  `DEBUG_TOOLS` and `ctx.debug_tools` on. A build that ships never asks.
- `shuyi-app`: `read_folder` notes its folder beside its layer in a build with
  debug tools. A `Poller` compares each file's modification time and size once
  a second, and reads what changed. No new dependency: 5 000 files cost
  what the ticket's completion notes measure.
- A file that can't be read now is tried again next time. A deleted file keeps
  its last bytes until the game restarts: an editor that saves by deleting and
  renaming would otherwise flash the embedded file for a second.
- The headless backend takes a script (`HeadlessPlatform::change_file`,
  `Harness::change_file`).

### 2. The bundle takes the new bytes

`Layer::replace` and `Bundle::replace`, with `std`: only a layer handed in at
run time changes. `OwnedFiles` holds its table in a `RwLock`; the old bytes
are leaked, as every file of the layer is, so a `&'static` read stays valid.
This leak is why it is for debug builds. A layer is `Copy` and points at the
one table, so every bundle holding the layer sees the change.

### 3. A `Hot<T>` remembers what its decode asked for

`shuyi_assets::hot::Hot<T>` holds the value, an `Arc<Formats>`, a clone of the
bundle, its id, and the paths its last decode asked `Bundle::bytes` for.

- It uses a per-thread read log, `bundle::recording`'s, never `bundle::watch`,
  which a process sets once.
- It keeps the paths asked for, **found or not**, where `recording` gives only
  the found: a `.meta` file that doesn't exist yet reloads the value when it
  appears (ADR-0057 reads the whole chain of settings files).
- `refresh` answers `Unchanged`, `Reloaded` or `Failed(errors)`. On a failure
  the old value stays, and the failed decode's paths are added to the old ones,
  so a missing file the broken one names mends it when it appears.

### 4. The game hears of it through a module

`Hook::Reloaded` and `Module::reloaded`, called in install order by
`Game::reloaded`. What the hook changes stays inside ADR-0040 §3: the module's
own state and the context's UI side (the palette). A module keeps its `Hot`
values as its own state. `run --json` lists the hook with the others.

### 5. Errors show over the frame

`Ctx::note_reload(id, &reload)` keeps a failure's errors until `id`'s next good
reload, as plain lines (`Ctx::reload_errors`). `App` puts them in
`Frame::reload_errors`, and `shuyi-app` draws them at the top of the window in
its own font, whatever the debug view's channels are. A module reports for
itself: there is no list of `Hot` values the engine could read.

### 6. `shuyi run --watch`

It starts the game's program with `cargo run` (a debug build, so debug tools
on) and `SHUYI_WATCH` (`shuyi_platform::WATCH_VAR`) naming the folder of the
game's files, `shuyi.ron`'s `assets`. `shuyi_app::watched_layer` reads that
folder; `run_game` lays it over a short-path game's files, and a game with its
own `main` does the same. It isn't an operation of the contract: it answers
nothing and runs until the window closes. The variable is ignored without
debug tools.

### 7. Determinism

A reload is not an input, and the new bytes are in no record. 2518 records the
first frame that named files, and warns of the frames after it.

## Consequences

- Any kind the registry reads reloads with no code of its own, and so does a
  new format.
- A game writes a small module for what it wants reloaded. A short-path game
  under `--watch` starts from its files on disk, and sees later changes only
  through a module of its own.
- What a backend made once from a file (a texture from a look's pictures) isn't
  a `Hot` value and doesn't follow it: 2242.
- A debug build leaks each version of a changed file. A session edits a file a
  few hundred times at most.
- The bundle's owned reads take a read lock, in shipped builds too. It is
  uncontended, and a read is a load, not a frame's work.

## Alternatives considered

- **A store of handles in the engine, as Bevy has.** Every game keeps its data
  in its own types (ADR-0003 §4), and a store would be a second way of holding
  it.
- **A `notify`-style crate.** A dependency per platform, for a debug tool whose
  polling costs little. It stays a follow-up if a game's folder outgrows it.
- **`Hot` taking the bundle and formats on each refresh.** Then every caller
  must keep them, and could pass another bundle than it loaded from.
- **A new table per change, leaking the old.** Simple, but it leaks the whole
  table on each change; a lock leaks only the file.
- **`Hot` reading the `.meta` chain by rule, on `recording`'s found paths.** It
  would know one format's rule, and miss any other file a format looks for.
- **A `Failed` reload shown by a gizmo.** Off with the debug view's channels,
  which is when it matters most.
