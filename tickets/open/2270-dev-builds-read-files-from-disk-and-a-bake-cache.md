---
id: "2270"
title: A desktop development build reads the game's files from disk, and shuyi bake skips unchanged files
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2270 — A desktop development build reads the game's files from disk, and shuyi bake skips unchanged files

## Context

A game's files are always embedded in a development build. The template's
`crates/game/src/lib.rs` has `static FILES: Embedded =
shuyi::files!("$SHUYI_FILES")`, a table of `include_bytes!` of every file
(ADR-0138). Its `build.rs` prints `cargo:rerun-if-changed` for the whole
`files/` folder, and `shuyi-embed`'s `Table::write` prints one for every
file it embeds (`crates/embed/src/lib.rs`, `write_and_report`). So
changing one byte of one picture recompiles the game crate, re-embeds
every file and relinks both the window program and the tool program
(`examples/tools.rs`).

`cargo shuyi run --watch` doesn't avoid this. It lays the folder on disk
*over* the embedded files (`crates/cli/src/watch.rs`,
`shuyi_app::watched_layer`, `BasicGame::over` in
`crates/platform/src/basic.rs`), so the embedded copy is still built. Only
the `packed` feature of the template embeds nothing, and it reads the pack
beside the program (`shuyi::app::pack_beside_program`): a release export's
build, not a development one.

This is fine today. The largest game measured is the game's public assets,
813,487 bytes (`docs/formats.md`, *Measurements*). A game with hundreds of
MB of art and sound would wait on that relink at every asset edit: the
loop ticket 2009's compile-time gate protects, which times only a one-line
code change (`crates/xtask/src/compile_times.rs`). Godot, Bevy, Defold and
LÖVE all read loose files from disk in development and pack them only at
export.

The second half is the bake. `shuyi bake`, `shuyi check --baked` and every
export decode and encode every file again on every run
(`Bake::bake` in `crates/assets/src/pack/bake.rs` loops over
`bundle.files_under("")` and calls `kind.encode` for each). The only cache
is the Playdate's sound cache (`audio_cache` in
`crates/cli/src/contract/bake.rs`, keyed by a CRC32 and length of the
source, ADR-0102 §5). A bake at real-game scale, with one-bit pictures to
dither for several targets, turns each export into minutes.

The pieces this builds on exist: `shuyi_app::read_folder` reads a folder as
a `Layer::owned` and watches it in a debug build (2207, 2220);
`shuyi_assets::bundle::recording` and the crate's `reads_of` record every
path a decode found or asked for (ADR-0057, ADR-0071); a pack is proven
equal to its sources (ADR-0023). ADR-0128 §4 keeps decoding on one thread;
this ticket doesn't change that.

From the engine blind-spot audit of 2026-11-23 (assets: "Development
builds embed every file", "`shuyi bake` has no cache").

## Nick input

None.

## Scope

**In:**
- A `disk-files` feature of the template's game crate. With it the build
  embeds none of the game's files nor its private files, and its build
  script watches neither folder. The program reads `files/` (and
  `private/files/` when the checkout has it) from disk at start, watched,
  through `shuyi_app::read_folder`.
- `disk-files` is a debug feature: an export refuses it, as it refuses
  `link` and `debug-tools` (`DEBUG_FEATURES` in `crates/cli/src/export.rs`).
  It is native only: a web build of it fails with a message saying so.
- `cargo shuyi run --watch` builds with `disk-files` when the game's
  package has the feature. The template's `shuyi.ron` names it in
  `game.features`, so the tool program (`run`, `scene`, MCP `start`) reads
  from disk too.
- A measurement: `cargo xtask asset-times`, which makes a game with a
  synthetic folder of about 300 MB (noise PNG and WAV files), and times the
  clean build, the rebuild after one PNG changed, and the start-up, each
  embedded and with `disk-files`, plus the program's peak memory. Its
  numbers go in `docs/formats.md`, *Measurements*. It is run by hand, not
  in CI.
- A bake cache. `Bake` gains a `bake_with(formats, bundle, target, cache)`
  over a `BakeCache` trait in `shuyi-assets` (no I/O there); `bake` is
  `bake_with` and no cache. `shuyi-cli` keeps the cache on disk under
  `target/shuyi/cache/bake-v<pack::VERSION>/`. `shuyi bake` and every
  export use it. `shuyi check --baked` never reads it: it stays the full
  proof.
- A cached file is reused only when nothing it was made from changed: its
  path, its kinds and their formats' ids, `pack::VERSION`, the bake's
  stamp and target, `ENGINE_VERSION`, the tool program's own binary (its
  length and modified time), and the bytes of every path its encode found
  or asked for (an import settings file not found counts as absent). An
  entry is written only for a file whose encode had no error.
- `shuyi bake` reports how many files came from the cache: a new field of
  `PackMade` (`#[non_exhaustive]`, so additive), in the contract's schema.
- New public types follow ADR-0039: `BakeCache` is a trait; the entry it
  stores (`CachedFile`) and what `bake_with` reports (`BakeReport`) are
  `#[non_exhaustive]` with constructors or accessors. `Baked` is not
  `#[non_exhaustive]`, so it is left as it is.
- An ADR (the `write-adr` skill): the development build that embeds
  nothing, and the bake cache's key. It amends ADR-0138 and ADR-0023.

**Out (do not do):**
- Decoding on worker threads, or splitting one large file's decode
  (ADR-0128 §4). If the measurement shows a hitch, write a ticket with
  its numbers. 3344's worker pool runs a game's `Think` jobs only, not
  loads or decoding.
- A disk layer over the embedded files for the tool program and the MCP
  session when the game has no `disk-files` feature, and telling the MCP
  session its build is stale. That is 3414's (see the clause below).
- Moving Hello, Grid, the tutorial's crates (`site/tutorial/`) or the
  game to `disk-files`. Each may later, in its own change.
- Caching a step that reads many files apart from its file's entry. A
  file's entry is keyed by every path its encode read, so such a file
  simply misses when any of them changes; nothing more here.
- A cache shared between machines or kept by CI. CI bakes from nothing.
- Pruning old cache entries beyond deleting the folder of an older
  `pack::VERSION`. `cargo clean` clears the rest.

**Either order with 3414:** if 3414 is done, a `disk-files` build lays no
second disk layer on the headless path: its own files layer is already
the folder on disk, and 3414's reload and stale answers read it as they
read the overlay; add a test that a `disk-files` tool program reports an
edited file as reloaded. If not, nothing to do (3414 lays its overlay over
whichever layer the game has, and handles a `disk-files` build there).

**Estimated size:** about 1,800 lines (code 850, tests 700, docs 250).

**Split:** past 900 lines, split off steps 7–11 (the bake cache) into a
new ticket, blocked by nothing.

## Implementation steps

1. `templates/shuyi-default/base/crates/game/Cargo.toml.tmpl`: add
   `disk-files = []` beside `packed`, with a comment in the style of
   `packed`'s. The template is checked by CI's template job, not by
   `cargo fmt --all`.
2. `templates/shuyi-default/base/crates/game/build.rs`: when
   `CARGO_FEATURE_DISK_FILES` is set, still set `SHUYI_FILES` and
   `SHUYI_PRIVATE_FILES`, but print no `rerun-if-changed` for either
   folder (else a file edit reruns the build script, which rebuilds the
   crate). Keep `rerun-if-env-changed=SHUYI_CONTENT`.
3. `templates/shuyi-default/base/crates/game/src/lib.rs`: gate `FILES`
   and `PRIVATE` on `not(any(feature = "packed", feature = "disk-files"))`.
   Add a `disk-files` `files()` that reads `env!("SHUYI_FILES")` with
   `shuyi::app::read_folder` once (a `OnceLock`, as the packed one does)
   and returns why on failure, so `BasicGame::files_unreadable` shows it.
   The private files likewise, laid as `BasicGame::private`. A
   `compile_error!` for `disk-files` on `wasm32`. When `run --watch`
   also lays its watched layer over them, the same folder is read twice;
   make `files()` use `shuyi::app::watched_layer()` when it gives one, so
   the folder is read and watched once.
4. `crates/cli/src/export.rs`: add `("disk-files", "the development
   build that reads the game's files from disk")` to `DEBUG_FEATURES`.
   It has no engine package of that name, so `ENGINE_DEBUG_FEATURES`'
   test is kept equal by leaving it out there; say so in a comment.
5. `crates/cli/src/watch.rs`: when the game package's manifest has a
   `disk-files` feature, add it to the build's features. The template's
   `shuyi.ron`: `game: (…, features: ["tools", "disk-files"])`.
6. `crates/xtask/src/asset_times.rs` (new; registered in `main.rs`): make
   a game with `shuyi new grid` in `target/asset-times/` (as
   `compile_times` does with `make_game` and `Runner`), write the
   synthetic folder of about 300 MB into its `files/` from a fixed seed,
   and time with cargo's `Finished … in` line: the clean build, then the
   rebuild after one PNG changed (three times, the median), each with and
   without `disk-files`; and the tool program's start-up to its first
   answer. Report the peak working set (Windows) or max RSS (Linux).
   `--mb <n>` sets the size. Write its numbers in `docs/formats.md`,
   *Measurements*, with the machine they ran on.
7. `crates/assets/src/pack/bake.rs`: split the body of `Bake::bake`'s
   loop into a function that makes one file's `Content` and returns the
   paths it found and asked for (`reads_of` around the encode).
8. Add `pub trait BakeCache { fn get(&mut self, path: &str) ->
   Option<CachedFile>; fn put(&mut self, path: &str, file: CachedFile); }`
   and `CachedFile`: the key's fixed part (a `ContentHash` of the kinds,
   format ids, `VERSION`, stamp and target), each read path with the
   `ContentHash` of its bytes or absent, and the `Content`; `to_bytes`
   and `from_bytes`, refusing bytes of another `VERSION`. `bake_with` asks
   the cache for each file, reuses an entry whose fixed part and every
   read path's hash match the bundle now, else encodes and puts. Return
   `(Baked, BakeReport)` with how many files were reused.
9. `crates/cli/src/contract/bake_cache.rs` (new): a `BakeCache` on disk,
   one file per path (its name a hash of the path) under
   `target/shuyi/cache/bake-v<VERSION>/`, with a manifest holding
   `ENGINE_VERSION` and the tool program's length and modified time
   (`std::env::current_exe`); a different manifest empties the folder.
   A file that can't be read or parsed is a miss, never an error.
   Delete `bake-v*` folders of other versions.
10. Use it in `contract::bake::bake` and in the exports' bakes
    (`crates/cli/src/export.rs` `bake_pack`, `export/desktop.rs`). Not in
    `check_file` or `shuyi check --baked`. Put the count in `PackMade`
    and regenerate the schema.
11. Docs: `docs/formats.md` (*Hot reload*: a `disk-files` build; *The
    pack*: the cache, and that `check --baked` never reads it),
    `docs/contract.md` (the new `PackMade` field), the new ADR, and the
    book's `site/content/learn/book/assets-and-formats.md`.

## Acceptance criteria

- [ ] A game made by `shuyi new grid`, built with `disk-files`, has none of
      its `files/` in its binary: the binary of a game with a 1 MB file of
      noise added is less than 1 MB larger than without it.
- [ ] In that build, a PNG changed in `files/` and a second `cargo build`
      compile nothing (`Fresh` for the game's package), and the running
      program shows the new picture under `run --watch`.
- [ ] A `disk-files` program whose `files/` can't be read shows the
      unreadable-files problem naming the folder, and doesn't panic.
- [ ] `cargo shuyi export` of a package built with `disk-files` is refused,
      naming the feature; a web build of it fails at compile time with the
      message from step 3.
- [ ] `cargo xtask asset-times` runs at about 300 MB, and
      `docs/formats.md` holds its numbers for both modes.
- [ ] A second `shuyi bake` with no file changed reuses every file: the
      count in `PackMade` equals the number of files, and the pack's bytes
      equal the first bake's.
- [ ] After one PNG changes, `shuyi bake` encodes exactly that file and
      the files whose encode read it; after a `.meta` beside a picture is
      added, that picture is encoded again; after the stamp or the target
      changes, every file is.
- [ ] A cached pack and a pack baked from nothing are byte-equal for the
      template's game and for Hello, Grid.
- [ ] A cache file cut short or of another `pack::VERSION` is a miss, and
      the bake still succeeds.
- [ ] `shuyi check --baked` reads no cache entry (a test with a cache
      holding wrong bytes still proves the pack equal or not as before).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `CachedFile` round-trips through `to_bytes`/`from_bytes`, and
  refuses another `VERSION` and cut bytes. `bake_with` with an in-memory
  cache: a hit, a miss after a read path changed, a miss after an absent
  import settings file appears, and a miss after the stamp changes.
  `DEBUG_FEATURES` refuses `disk-files`. `watch.rs` adds the feature only
  when the package has it (with the faked cargo the tests already use).
  `asset_times` with the faked `Runner`, as `compile_times`' tests do.
- Property: for random small bundles and random edits, `bake_with` over
  a cache filled by the bake before gives bytes equal to `bake` from
  nothing.
- Snapshot / integration: CI's template job builds a new game with
  `disk-files` and runs its tool program once; a `shuyi bake` run twice
  over a fixture project reports the reused count; the contract schema
  snapshot holds the new `PackMade` field.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
