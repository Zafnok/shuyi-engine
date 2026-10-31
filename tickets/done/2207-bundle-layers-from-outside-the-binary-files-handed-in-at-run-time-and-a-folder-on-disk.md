---
id: "2207"
title: "Bundle layers from outside the binary: files handed in at run time, and a folder on disk read by shuyi-app"
type: feature
milestone: After MVP
model: reasoning
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-10-27
---

# 2207 — Bundle layers from outside the binary

## Context

- **The game tickets:** game#1012
  ([`1012-bundle-runtime-mod-layers.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1012-bundle-runtime-mod-layers.md))
  lays mods over the game's files, and game#1014
  ([`1014-mods-folder-manifest-and-load-order.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1014-mods-folder-manifest-and-load-order.md))
  finds and reads them. Where mods live, their manifest, their order and
  their checks are the game's (its mods ADR, game#1011); a layer of
  files that didn't come from the binary is this ticket.
- **The job:** let a player's files (a mod, a level they made) replace
  or add to the files a game ships, without the game's loaders changing.
- **Friction:** no friction review: a missing feature, found when the
  game's tickets were split between the two repositories.
- **The generic shape:** ADR-0004 §1 plans the bundle's layers to come
  from folders on disk as well as from the binary, serving mods and
  hot reload. Today a `Layer` (`crates/assets/src/bundle.rs`) is a
  directory embedded by `include_dir!` (`Files::Dir`) or a sorted
  table of `&'static` paths and bytes (`Files::Table`), and its `name`
  is `&'static str`. This ticket adds a layer of files handed in at run
  time (owned paths and bytes, kept for the life of the program so every
  read stays `&'static`), and `shuyi-app` reading a folder on disk into
  one. Hot reload is 2220's, which waits for this ticket: it watches
  these layers and swaps a changed file's bytes in (`Bundle::replace`).
- **Outside tactics:** mods for a farming game; a platformer's
  player-made levels loaded from a folder; a debug build that reads its
  art from disk so an artist sees a change without a rebuild.

## Nick input

None.

## Scope

**In:**
- `Layer::files(name: String, files: BTreeMap<String, Vec<u8>>) ->
  Result<Layer, LayerError>` (or a name that fits beside `dir` and
  `table`): the name and the files are kept for the program's life
  (leaked once), so `Bundle`'s `&'static` reads don't change.
- Path checks when the layer is made: no absolute path, no `..`, no
  backslash, no empty path. The error names the layer and the path.
- Reads, listings, `display_path` and the `private`, `test` and
  `leaving_out` flags work on such a layer as on the others: a file
  there replaces the file at the same path in a layer after it, and a
  file only there is added.
- Either order with 2228: if 2228 is done, `Bundle::versions` reads the
  new variant through the same private walk, and a unit test checks
  that a run-time layer's copy of a path comes before an embedded
  layer's copy of it; if not, 2228 adds that test.
- In `shuyi-app`, native only: `read_folder(path, name, max_file_bytes)
  -> Result<Layer, ...>`, every file under the folder as one such layer,
  `/`-separated paths relative to it; a file over the limit, or one that
  can't be read, is an error naming it. Nothing on the web.

- `shuyi import` (`crates/cli/src/contract/import.rs`, ADR-0022 §6)
  reads the files a file names from a bundle of its folder, which it
  leaks once per call: make that folder bundle with this layer.

**Out (do not do):**
- Where a game's mods live, their manifest, order or checks (the game's).
- Merging data tables entry by entry (the game's loaders, over 2228's
  `Bundle::versions` and `Formats::load_versions`, which read every
  layer's copy of a file; the merge rules stay the game's).
- Zip files.
- Files fetched over the network as a layer: 2601 (parked) gives HTTP
  requests; a layer of fetched files is a later ticket, and Nick's call.
- Watching folders and reloading: 2220, which waits for this ticket
  and replaces a disk layer's files (`Bundle::replace`). 1124's maps
  reload through 2220 as formats and need nothing here, whichever
  lands first.
- Fuzzing the formats a layer's files reach: 2020. It should be done
  before game#1014 ships mods to players, but this ticket doesn't wait
  for it.

**For the game:** additive. The game's PRs that use it (game#1012,
game#1014) move its pin after this one.

## Implementation steps

1. Read ADR-0004 §1, `crates/assets/src/bundle.rs` and its tests.
   Check whether 2228 is in `tickets/done/`.
2. Add the run-time variant of `Files` and the constructor with its
   checks. Keep the existing constructors `const` and unchanged.
3. Make `find` and `list` (and everything built on them) read the new
   variant; a listing of all layers stays sorted and de-duplicated.
4. `shuyi-app`: `read_folder`, with a test on a temporary folder.

## Acceptance criteria

- [x] A run-time layer's file replaces an embedded file at the same
      path, and a file only there is added (unit tests on reads and
      listings).
- [x] Of two run-time layers, the first wins for a path both have
      (unit test).
- [x] `display_path` shows the layer's name and the path for a file read
      from it (unit test).
- [x] Bad paths are refused with the layer's name and the path in the
      message (unit tests).
- [x] ~~If 2228 is done: `Bundle::versions` gives a run-time layer's copy
      of a path before an embedded layer's copy of it (unit test).~~
      (not done: 2228 is open, so it adds the test; its clause says so)
- [x] `read_folder` reads a temporary folder into a layer, and refuses a
      file over the limit (test).
- [x] The WASM build passes; nothing reads files on the web.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: reads and listings over embedded, table and run-time layers;
  path checks.
- Property: for any set of layers, a listing is sorted and de-duplicated
  and a read agrees with it.
- Integration: `read_folder` on a temporary folder.

## Completion notes

- **`shuyi-assets`:** `Layer::owned(name: String, files: BTreeMap<String,
  Vec<u8>>) -> Result<Layer, LayerError>` makes a layer of files handed in
  at run time. The name, paths and bytes are leaked once, so every read
  stays `&'static`. The new variant is `Files::Owned(&'static
  OwnedFiles)`: `OwnedFiles` keeps its sorted table private, so 2220's
  `Bundle::replace` can change how it holds its files without a break.
  `Files::Table` and `Files::Owned` share the table lookup and listing
  (`table_get`, `table_list`), so reads, listings, `display_path`,
  `has`, `left_out`, `test_part` and the `private`, `test` and
  `leaving_out` flags work on it unchanged. The existing constructors are
  unchanged and still `const`.
- **Path checks:** `LayerError { layer, path, problem: BadPath }` (both
  `#[non_exhaustive]`, ADR-0039) refuses an empty path, an absolute one
  (`/x`, or a drive like `C:/x`), a backslash, a `..` part, and also an
  empty or `.` part (`a//b`, `./a`, `a/`), which no read names. The
  message names the layer and the path.
- **`shuyi-app`:** `read_folder(path: &Path, name: &str, max_file_bytes:
  u64) -> Result<Layer, FolderError>`, native only. It reads at most one
  byte past the limit, also when a file grows while it is read. A link to
  a file is read and a link to a folder isn't followed (no loops).
  `FolderError` is `#[non_exhaustive]`: `Read`, `TooBig`, `NotUtf8` (a
  name no read can name) and `Layer`.
- **`shuyi import`:** its folder bundle is one `Layer::owned` now, in
  place of its own leaked table.
- **Tests:** unit tests over embedded, table and run-time layers (reads,
  listings, the first of two run-time layers wins, `display_path`, the
  flags, a run-time layer answers as the same table does), the path
  checks, and a property test: for any up to four table and run-time
  layers, some marked test, a listing is sorted and once each, holds
  every layer's files in the folder, and each read agrees with the first
  layer that has the path. `read_folder` has tests on temporary folders:
  a folder laid over a base layer, the limit, a missing folder, an empty
  folder, and a backslash in a name (Unix only).
- **Deviation:** 2228 isn't done, so `Bundle::versions`' test of a
  run-time layer is 2228's. Its clause now says so.
- **Handover:** 2220's context says what `Bundle::replace` builds on
  ("From 2207 (done)"). 2228, 2215 and 1124's clauses that waited on this
  ticket now say what holds.
- **Docs:** rustdoc (the `bundle` module comment, `Layer::owned` with an
  example, `read_folder`, `shuyi-app`'s crate doc) and one bullet in
  `docs/formats.md`, *The parts*.
- **For the game:** additive. The game's PRs for game#1012 and game#1014
  move the pin after this one, and no PR exists yet. The engine's PR
  merges first.

Breaking: shuyi-assets: `bundle::Files`: it is `#[non_exhaustive]` now and has a new variant, `Owned`; a game that matches on `Files` adds a `_ =>` arm.
