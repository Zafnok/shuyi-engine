---
id: "2220"
title: "Hot reload of any asset kind: a file on disk changes and the running game sees it, through the registry, with errors shown instead of a crash"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2207", "2358"]
nick_input: none
completed: 2026-11-01
---

# 2220 — Hot reload of any asset kind

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage, "like hot reload". Bevy's homepage: "asset changes
appear in running apps without restarts or recompiles", for scenes,
textures and meshes, and "any asset type can be integrated".

ADR-0004 §1 planned hot reload of disk layers in debug builds, but the
only ticket for it was 1124, which is parked and is mostly about Tiled
and LDtk maps. This ticket takes hot reload out of 1124 and makes it
work for every asset kind the registry knows (ADR-0007, ADR-0019), so
1124's maps get it by being formats.

What is there today:

- `shuyi_assets::bundle` (`crates/assets/src/bundle.rs`) is one bundle
  installed for the process, and every read is `&'static`;
  `bundle::watch` already tells a front end of every file read;
- `shuyi_assets::format` has `AssetKind`, `Format`, `Formats` and
  `decode_with`;
- **From 2207 (done):** `Layer::owned(name, files)` makes a layer of
  files handed in at run time, `Files::Owned(&'static OwnedFiles)`,
  whose sorted table of leaked paths and bytes is private to
  `bundle.rs`; `shuyi_app::read_folder(path, name, max_file_bytes)`
  reads a folder on disk into one (native only). Nothing records which
  folder a layer came from: `shuyi-app` keeps that beside the layers it
  reads, to poll. `Bundle::replace` is this ticket's: it swaps a path's
  bytes in an `OwnedFiles` (give it interior mutability, or a new table
  per change, leaking the old), and an embedded layer's file stays
  as it is;
- 2228 (either order) adds `Bundle::versions` and
  `Formats::load_versions`: every layer's copy of a path, for loaders
  that merge layers (mods adding entries to a table). It tells of the
  path once, through the one helper `Bundle::bytes` uses (the `watch`er,
  and 2516's `bundle::recording` once it is in), so read tracking covers
  a value merged from every copy: a change to any layer's copy is a
  change at that path;
- a game keeps what it loaded in its own data (`GameData`), not in a
  store the engine owns.

So reload needs: a way to see that a file changed (the platform's job),
a new copy of it in the bundle, and each loaded value told to decode
again.

## Nick input

None.

## Scope

**In:**
- **The platform sees changes** (`AGENTS.md` rule 5: only backends
  touch files and the clock):
  - `Platform::changed_files(&mut self, out: &mut Vec<String>)`
    (`crates/platform/src/platform.rs`), with a default that adds
    nothing;
  - `shuyi-app`, in debug builds with debug tools on, checks the
    modification times of its disk layers' files once a second, with no
    new dependency (a `notify`-style crate is a follow-up only if
    polling costs too much on a large folder: measure 5 000 files);
  - the headless backend takes a script of changes, for tests.
- **The bundle takes the new bytes:** with `std`, a disk layer's file can
  be replaced (`Bundle::replace(path, bytes)`). The old bytes are
  leaked, as 2207's layers are, so every `&'static` read stays valid;
  this happens only in debug builds, and the docs say so.
- **Values that reload:** `shuyi_assets::hot::Hot<T: AssetKind>`:
  - made by decoding a path through the registry; it records every file
    the decode read through `bundle::recording` (the per-thread read log
    2516 adds; never `bundle::watch`, which a process sets once). Either
    order with 2516: if 2516 isn't done, add `bundle::recording` here as
    2516 describes it, and 2516 reuses it. So a change to any of them
    (an atlas's image, a level's tileset) reloads it;
  - `Hot::refresh(&mut self, changed: &[String]) -> Reload` where
    `Reload` is `Unchanged`, `Reloaded`, or `Failed(Vec<ContentError>)`;
    on failure the old value stays;
  - `Deref<Target = T>`, so code reads it like the value.
- **The game hears of it, through a module hook** (2358's ADR: a new
  frame-loop hook goes on `Module`, and `GameHooks` gains no field,
  because a field breaks every game that builds `GameHooks` as a struct
  literal): `Hook::Reloaded` and `Module::reloaded(&mut self, ctx: &mut
  Ctx<G>, changed: &[String])`, defaulted. `App` hands the paths from
  `Platform::changed_files` to `Game::reloaded(&mut self, changed:
  &[String])`, which calls each module that declared the hook, in
  install order, before the frame's update. A module refreshes its own
  `Hot` settings there. A game refreshes its own `Hot` values from a
  small module of its own that it installs with `Game::with_module`
  (Hello, Grid's palette shows it). What the hook changes stays inside
  2358's ADR point 3 (the module's own state and the context's UI
  side), so a game's module keeps the `Hot` values it refreshes as its
  own state, not in `ctx.game`. A screen can rebuild its view after it,
  and `run --json` lists the hook.
- **Either order with 3104 (the GPU 3D painter, parked):** if 3104 is
  done, `shuyi-app-3d` uploads its meshes and textures once only; add
  the upload again on reload, from `Module::reloaded`, for the meshes
  and textures read from a changed file. If not, nothing to do (3104
  does it).
- **Errors show, never crash:** a `Failed` reload's errors (file, line
  and message, as every loader gives them) show as a line at the top of
  the frame, drawn by `shuyi-app`'s debug overlay, until the next good
  reload. They show with every debug view channel off, so they are not
  gizmos.
- **Determinism:** a reload is not an input, and the files' new bytes
  are in no record. **Either order with 2518 (session records):** if
  2518 is done, `App` records the first frame whose
  `Platform::changed_files` named files as `SessionRecord::reloaded_at:
  Option<u32>`, and 2518's `Played::limits` warns that frames after it
  may not match. If 2518 isn't done, 2518 does it (its own clause says
  so).
- **`shuyi run --watch`** (`crates/cli`): runs the game with its
  `assets/` as a disk layer over the embedded files, and debug tools on.
- **Either order with 2506 (the editor research, parked):** if 2506 is
  done, read its ADR's live edits (its point 4) and say in the
  completion notes whether what reloads here matches it; if not,
  nothing to do (2506's spike uses this reload, its clause says so).
- **Either order with 1124:** if 1124 is done, a harness test shows a
  changed `.tmx`, and a changed TSX tileset it reads, reloading through
  `Hot<TileMap>` (the headless backend's script); if not, nothing to do
  (1124's maps are formats, so they reload with no extra code, and 1124
  adds that test when it lands).
- **Either order with 2318:** if 2318 is done, add to its `layout`
  channel a `Note` label "reloaded <file>", shown for two seconds of
  frames when a `Hot<TileMap>` reloads; if not, nothing to do (2318
  adds it).
- **From 2229 (done, import settings):** add a test that a `Hot` value
  of the txtimg toy kind (`crates/assets/tests/it/txtimg.rs`, whose
  `TxtImg` reads the `txtimg` section through `cx.import_settings`)
  reloads when its `<file>.meta` changes, and when the `_folder.meta` it
  reads changes. The settings files are read through the bundle
  (`DecodeCx::bytes`), so the reads `Hot` tracks hold them.
- **From 2802 (done):** the book's chapter 5, *Assets and formats*
  (`site/content/learn/book/assets-and-formats.md`), gets a section on
  hot reload (`Hot<T>` and `shuyi run --watch`), its code a snippet in
  `site/snippets/`.

**Out (do not do):**
- reloading code (2905);
- hot reload on the web (2528's `shuyi web --serve` rebuilds and
  reloads the whole page instead) or on a console (no files to watch);
- scene files (2221 uses this).

## Implementation steps

1. Read 2207's layers, 2358's ADR and `crates/ui/src/module.rs`,
   `bundle.rs`, `format.rs`, and the `Platform` trait. Check whether
   2228, 2516, 2518, 1124 and 2318 are in `tickets/done/`.
2. Add `changed_files` to the trait and both backends, with the poller in
   `shuyi-app`.
3. Add `Bundle::replace` and `Hot<T>` with its read tracking on
   `bundle::recording` (added here if 2516 isn't done).
4. Add `Hook::Reloaded`, `Module::reloaded` and `Game::reloaded`, the
   error display, and `reloaded_at` on 2518's record, if 2518 is done.
5. Add `shuyi run --watch`, the map reload test if 1124 is done and
   2318's note if 2318 is done, and run the gates.

## Acceptance criteria

- [x] With the headless backend's script, a changed palette file is seen
      by the game on the next frame (a harness test).
- [x] A changed image that a sprite sheet reads reloads the sheet.
- [x] ~~If 1124 is done, a changed map and a changed TSX tileset it reads
      reload through `Hot<TileMap>` (a harness test).~~ (not done: 1124 is parked, and its clause now adds the test)
- [x] ~~If 2318 is done, a reloaded map shows its note in the `layout`
      channel (a test).~~ (not done: 2318 is parked, and its clause now adds the note)
- [x] A broken file keeps the old value, and its error names the file
      and line (a test).
- [x] A shipped build (debug tools off) never polls (a test with a
      counting fake clock).
- [x] `shuyi run --watch` on Hello, Grid shows a changed palette within
      two seconds (checked by hand; say so in the completion notes).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Hot::refresh` for each outcome; read tracking across two files;
  the poller with a fake clock and file times; if 2228 is done, read
  tracking records a path read through `Bundle::versions`. If not,
  nothing to do: 2228 moves the recording into its helper and tests it
  (its clause says so).
- Integration: the harness test through the headless backend's script.

## Completion notes

What was done, all as the scope says and recorded in ADR-0071:

- `Platform::changed_files` (defaulted), asked by `App` once a frame before the
  update, only with debug tools on. `shuyi-app`'s `Poller`
  (`crates/app/src/watch.rs`) looks at the folders `read_folder` read once a
  second. The headless backend takes a script (`change_file`).
- `Layer::replace` and `Bundle::replace`; `bundle::recording` (2516 reuses it);
  `shuyi_assets::hot::{Hot, Reload}`.
- `Hook::Reloaded`, `Module::reloaded`, `Game::reloaded`.
- `Ctx::note_reload` and `reload_errors`, `Frame::reload_errors`, drawn by
  `shuyi-app` over the top of the frame.
- `shuyi run --watch`, and `shuyi.ron`'s new `assets` part; `shuyi_app::watched_layer`,
  `BasicGame::over`. Hello, Grid's palette shows the module
  (`examples/hello-grid/src/hot_palette.rs`).
- The book's chapter 5 has its section, its code a snippet.

**Polling, measured:** scanning 5 000 files in 50 folders takes about 8 ms on
Nick's machine in a debug build (Windows 11, NTFS). A first version looked each
file up and took about 100 ms, a visible hitch once a second, so it now reads
what the directory listing holds. No `notify`-style crate is needed.

**Checked by hand**, on Hello, Grid in its window: with the folder of a copy of
its files watched (`SHUYI_WATCH`, in a build with the `link` feature so a
screenshot could be taken), a changed colour showed within two seconds, a broken
palette kept its colours and showed `watched/palette.ron:8: …` at the top of the
window, and a good save cleared it. That check found the error line running off
the window; it now wraps to the window's width, checked the same way. `shuyi run --watch` itself was run on Hello, Grid and
opened its window; a change wasn't made through that command, which sets the
same variable.

Deviations and what the ticket didn't say:

- `Hot` tracks the paths **asked for**, found or not, where `recording` gives the
  found ones (as 2516 describes it): a `.meta` file that doesn't exist yet must
  reload a value when it appears. Both read one per-thread log.
- A deleted file keeps its last bytes until the game restarts (ADR-0071 §1).
- The ticket says a game's `assets/`; Hello, Grid's are in `files/` and a new
  game's in `crates/game/files`, so `shuyi.ron` gained `assets`, `assets` by
  default.
- A short-path game under `--watch` starts from its files on disk, but sees later
  changes only through a module of its own.
- `HeadlessPlatform::reload_errors` holds the frame's errors: `Presented::Game`
  is an exhaustive variant, and a new field would have broken games' patterns.

The either-order clauses: 2228, 2516, 2518, 1124, 2318, 2506 and 3104 aren't done.
Each clause naming this ticket now says what holds. **Follow-up:** 2242, the
pictures a backend uploaded (a changed PNG isn't redrawn: the renderer's textures
are made once).

No break.
