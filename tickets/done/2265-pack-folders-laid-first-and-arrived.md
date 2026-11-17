---
id: "2265"
title: "The pack lays folders in the order the bake names: Bake::first, each folder's span in the index, and Bundle::arrived over a source"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2263"]
nick_input: none
completed: 2026-11-17
---

# 2265 — The pack lays folders in the order the bake names

## Context

[ADR-0128](../../docs/adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§3: a group is a folder. A web game names the folders its first frame
reads (`Bake::first("title/")`), the bake lays them at the front of the
pack, and the page streams the pack in that order (3318), so the first
frame waits only for those folders. A screen that opens a later part asks
`Bundle::arrived(dir)` first and shows its own wait.

Split off from 2263 at its **Split:** point (past 1,000 lines).

**From 2263 (done):** the pack is version 18: the header's checksum
covers the header and the index, and each entry has the checksum of its
kept bytes. `pack::Index::read(prefix, stamp)` reads the index alone,
`Index::check_file` checks one file, and `Index::into_layer(name,
source)` makes a layer over a `pack::Source` (`read(at, into)`, with
`SourceError::NotYet`): `bundle::Files::Source(&'static SourceFiles)`,
whose files are a 32-byte record each, sorted by path. A file whose bytes
are `NotYet` reads as missing. `Index::file_range(path)` gives where a
file's bytes lie in the pack. `pack::write` lays the files in path order,
and `Raw::write_marked` lays them in an order a recipient's code shuffles
(`order_of(code, count)`, its values pinned by `a_codes_layout_is_pinned`).

## Nick input

None.

## Scope

**In:**

- **The file, `pack::VERSION` 19:** the index gains each folder's span:
  for every folder `Bake::first` named, in the order named, its path and
  the offset and length its files take. `Index::read` refuses a span that
  doesn't hold every file under its folder. `docs/formats.md` and the
  pinned snapshot move with it.
- **`Bake::first(dir)`** (`crates/assets/src/pack/bake.rs`): lays the
  folder's files at the front of the pack, in the order of the calls (a
  file under two named folders goes with the first), then every other
  file in path order. A trailing `/` is the same folder. `Bake::check`
  holds as before.
- **The recipient's layout** (ADR-0118, `Raw::write_marked`): shuffles
  each named folder's files within its span, and the rest within theirs,
  so each span still holds its folder. With no folder named, the order
  is the one `a_codes_layout_is_pinned` pins: a copy made before reads
  the same. `Raw::mark` reads a pack with spans.
- **`Source::arrived(&self, at: u64, len: u64) -> bool`**, default
  `true`: whether those bytes can be read now.
- **`Bundle::arrived(&self, dir) -> bool`**: whether every file under
  `dir` can be read now. True for every layer read whole; for a layer
  over a source, true once the folder's span (or, for a folder not named
  first, every file under it) has arrived. `Index::spans()` gives each
  span's folder and its byte range in the pack, for 3318's page.
- **`shuyi bake`** says the spans it laid (`PackMade` gains them: the
  contract's version and schemas move).
- **Docs:** `docs/formats.md` (*The pack*): `Bake::first`, the spans,
  `arrived`.

**Out (do not do):**

- the web's source and the load that waits (3318, 3316);
- a group type or a list of paths in data (ADR-0128 §3).

**Estimated size:** about 550 lines (code 250, tests 250, docs 50).

## Implementation steps

1. Add the spans to the index, with their check; move `VERSION` to 19
   and the snapshot.
2. Write `Bake::first` and the layout it lays, then the recipient's
   layout within spans.
3. Write `Source::arrived`, `Bundle::arrived` and `Index::spans`.
4. Make `shuyi bake` say the spans; move the contract's version.
5. Write the docs; run the gates.

## Acceptance criteria

- [x] `Bake::first("a/")` lays `a/`'s files first, and the pack with a
      recipient's layout keeps them in their span; with no folder named,
      the recipient's layout is the one pinned before.
- [x] A source that answers `NotYet` for a span: `arrived` is false for
      that folder and true for the others (a test).
- [x] `Bundle::arrived` is true for every folder of a bundle with no layer
      over a source.
- [x] `shuyi bake` names each span it laid.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Bake::first`'s order (nested folders, a folder named twice, an
  empty one); the spans' check; `arrived` over a source.
- Property: any pack with folders named reads the same files as without,
  and with any recipient's code each span holds its folder.

## Completion notes

**What was done:**

- **The file is `pack::VERSION` 19** (`crates/assets/src/pack.rs`): the
  index ends with the spans, each a folder (no `/` at its end), and the
  offset and length from the first byte of its files to the last.
  `check_spans` (run by `Index::read`, `Pack::open` and `Raw::read`)
  refuses more than `pack::MAX_FIRST` (256) spans, a folder named twice
  or with a `/` at its end, a span past the files' end, and a span that
  doesn't hold every file under its folder. The files under a folder are
  found by a binary search for `dir/` in path order, so the check costs
  each span its own files.
- **The layout:** `pack::write_first(info, first, files)` (and `write`,
  which is it with no folder) lays the files of each named folder, in the
  order named and each group in path order, then the rest in path order.
  `Bake::first(dir)` names them (`Bake::firsts()` reads them back), and
  `Bake::bake` refuses more than 256 distinct folders.
- **The recipient's layout** shuffles within each group from one stream
  drawn group after group (`order_in`), so with no folder named it is
  exactly `order_of`, which `a_codes_layout_is_pinned` pins; `Raw` keeps
  the folders read from the spans, and `Raw::write`, `write_marked` and
  `mark` use the groups.
- **Arrived:** `Source::arrived(at, len)` (default `true`),
  `Index::spans()` (folder, byte range from the pack's start), and
  `Bundle::arrived(dir)`: every layer this thread reads must have the
  folder, which a layer over a source answers from the folder's span, or
  from each file under it for a folder not laid first.
- **`shuyi bake`** answers `PackMade::spans` (`PackSpan { folder, start,
  bytes }`, `#[non_exhaustive]` with `PackSpan::new`) and prints a
  `laid first:` line; the contract is version 53. The fake game of the
  CLI's tests lays `images/` first.
- **Tests:** `crates/assets/src/pack/first_tests.rs` (the order, nested
  folders either way round, a folder named twice, an empty one, the whole
  pack; a bake with folders laid first passes `Bake::check`; the spans'
  check at each edge; the recipient's layout within spans; the pinned
  order with no folder; `arrived` over a streaming source with one span
  missing or everything past the title missing; `arrived` everywhere over
  layers read whole and `InMemory`), and the property that any folders
  named change no file read and any code keeps each span holding its
  folder. `crates/cli/src/contract/bake/tests.rs` checks the answer's span
  against the pack's index and the text.
- **Docs:** `docs/formats.md` (*The pack*: folders laid first, whether a
  folder has arrived), `docs/contract.md` (`bake`'s `spans`), the book's
  *The pack* (`site/content/learn/book/assets-and-formats.md`).

**Deviations:**

- **A span is the hull of its folder's files,** from the first byte to the
  last, computed after the layout. When folders nest, a span may hold
  more than its folder (a folder named after one it holds spans both
  groups; one named after its parent lies inside the parent's span), and
  `arrived` then waits for those bytes too. It always holds every file
  under its folder, which is all the check asks.
- **A cap of 256 folders laid first** (`pack::MAX_FIRST`), so a broken or
  hostile index can't make the spans' check cost spans × files; the
  ticket named none.
- **`Bundle::arrived` asks every layer**, not only the one each file is
  read from: a folder under a loose layer laid over a streamed pack waits
  for the pack's span too. Exact per-file answers would cost a search of
  the layers for every file on every frame; a game streams one pack.
- **About 1,070 counted lines** against 550 estimated, most of it tests
  (the spans' check at each edge, the streaming source, the property). It
  would have split cleanly at `shuyi bake`'s spans (the CLI and the
  contract), which need nothing but `Index::spans`.

**Follow-up tickets:** none; 3318 builds the web's source and its
`arrived`, and its context now says what this ticket gives.

Breaking: shuyi-assets: pack::VERSION: 19 (the index holds the spans of the folders laid first); a pack baked before is refused, so a game bakes again with `shuyi bake`

```migration
since: 0.1.0
what: The pack format is version 19; a pack of version 18 is refused.
old: "pack::VERSION == 18"
new: "pack::VERSION == 19"
before: |
  cargo shuyi bake   # with the engine before 2265
after: |
  cargo shuyi bake   # again, with this engine
```

Behaviour: shuyi-cli: `shuyi bake`'s answer gains `spans` and its text a `laid first:` line when the game's `Bake` names folders first; the contract is version 53

Game: nothing to adopt, the game names no folder first, so its pack lays out as before; it bakes its pack in a test and opens it (`Pack::open`), which reads version 19 as it bakes it. `gh search code --repo Zafnok/visions-of-shuyi` for `pack::VERSION`, `write_marked`, `PackMade` and `impl Source for` found only the pins log.

**Nick:** nothing changes in the game. This is for a web game whose pack
is too big to arrive before its first frame (3318 streams it).
