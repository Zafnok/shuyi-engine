---
id: "2265"
title: "The pack lays folders in the order the bake names: Bake::first, each folder's span in the index, and Bundle::arrived over a source"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2263"]
nick_input: none
completed:
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

- [ ] `Bake::first("a/")` lays `a/`'s files first, and the pack with a
      recipient's layout keeps them in their span; with no folder named,
      the recipient's layout is the one pinned before.
- [ ] A source that answers `NotYet` for a span: `arrived` is false for
      that folder and true for the others (a test).
- [ ] `Bundle::arrived` is true for every folder of a bundle with no layer
      over a source.
- [ ] `shuyi bake` names each span it laid.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Bake::first`'s order (nested folders, a folder named twice, an
  empty one); the spans' check; `arrived` over a source.
- Property: any pack with folders named reads the same files as without,
  and with any recipient's code each span holds its folder.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
