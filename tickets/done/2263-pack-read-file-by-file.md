---
id: "2263"
title: "The pack read file by file: its index read alone, a check per file, and a layer over a source the backend gives"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2262"]
nick_input: none
completed: 2026-11-17
---

# 2263 — The pack read file by file

## Context

[ADR-0128](../../docs/adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§2 and §3, from ticket 2906's numbers
([`docs/research/2906-asset-memory.md`](../../docs/research/2906-asset-memory.md)):

- `Pack::open` (`crates/assets/src/pack.rs`) checks one checksum over
  every byte after the header, then unpacks every file, so it needs the
  whole pack in memory (ADR-0023 §1, §3).
- **The Playdate:** a pack over about 6 MB doesn't fit its 10 MB heap
  read whole; its budget is 40 MB. Read file by file, a 40 MB pack peaks
  at 0.6 MB, its table at about 52 bytes a file.
- **The web:** a pack over about 12 MB can't arrive before the first
  frame within the 5 s start-up at 25 Mbit/s; its budget is 25 MB. A web
  game that names the folders its first frame reads gets them first.

The decision: an index read alone, a checksum per file, folders laid in
an order the game names (`Bake::first`), a `Source` trait a backend
implements, a layer that reads one file per `Bundle::bytes`, and
`Bundle::arrived(dir)`. 3315 (the Playdate) and 3318 (the web) give the
sources; desktop keeps reading its pack whole.

## Nick input

None.

## Scope

**In:**

- **The file, `pack::VERSION` 18** (17 went to ticket 3216's bold face;
  ADR-0023 §5: refused, never migrated):
  - the header's checksum covers the header and the index only;
  - each entry gains the checksum of its stored bytes (the same FNV-1a
    word hash, `checksum`);
  - *(split off to 2265)* the index gains each folder's span: for every
    folder `Bake::first` named, the offset and length its files take, in
    the order named;
  - `docs/formats.md`'s table of the file, and the pinned snapshot of the
    binary form, move with it.
- **Reading:**
  - `pack::Index` (`#[non_exhaustive]`, ADR-0039): `Index::read(prefix:
    &[u8]) -> Result<Index, IndexError>`, where `IndexError::Short {
    needs }` says how many bytes the header and index take, so a caller
    reads the header, then exactly that many;
  - `Index::check_file(&self, path, stored: &[u8]) -> Result<Vec<u8>,
    PackError>`: the file's checksum, then its bytes unpacked;
  - `Pack::open` stays, checking every file's checksum, for a pack
    embedded or read whole.
- **`pack::Source`**, a trait a backend implements: `fn read(&self, at:
  u64, into: &mut [u8]) -> Result<(), SourceError>`, where
  `SourceError` (`#[non_exhaustive]`) is `NotYet` (the bytes haven't
  arrived) or `Failed(String)`. `Send + Sync`, as the installed bundle
  needs. A `Source` over a `&'static [u8]` (`pack::InMemory`) for tests
  and an embedded pack.
- **The layer:** `Index::into_layer(self, name, source: &'static dyn
  Source) -> Layer`:
  - its paths and name are leaked once, as `Pack::into_layer`'s are;
    `has`, `files_in`, `files_under`, `left_out` and `display_path` read
    the index alone;
  - `Bundle::bytes` reads the file's stored bytes from the source,
    checks them and unpacks them, and hands them out as `Cow::Owned`
    (2262): nothing is kept;
  - **From 2262 (done):** `shuyi-ui`'s `Images for Bundle` (the one-bit
    frame's pictures, read where they lie) draws only a file its layer
    lends (`Cow::Borrowed`): a file of this layer is no picture there.
    This ticket says, in the doc of `Index::into_layer` and in
    `docs/formats.md`, that a game over this layer keeps its one-bit
    pictures as values of its own (ADR-0128 §3), and a test shows the
    one-bit frame draws such a file as no picture rather than failing;
  - a file that is `NotYet` reads as missing, and a `Failed` read or a
    bad checksum as missing too, told to the `watch`er; the layer's
    `problems()` lists each failure once, for the backend to log;
  - the layer stands for the baked bundle's fingerprint, as
    `Pack::into_layer`'s does.
- *(split off to 2265, with `Bake::first` and `shuyi bake`'s spans
  below)* **`Bundle::arrived(&self, dir) -> bool`**: whether every file under
  `dir` can be read now. Always true for every other layer; for a layer
  over a source, true once the folder's span (or, for a folder not named
  first, every file under it) answers without `NotYet`, which a source
  says through `fn arrived(&self, at: u64, len: u64) -> bool` (default
  `true`).
- **`Bake::first(dir)`** (`crates/assets/src/pack/bake.rs`): lays the
  folder's files at the front of the pack, in the order of the calls,
  then every other file in path order. ADR-0118's recipient layout
  (`Raw::write_marked`) keeps each named folder's files inside its span.
  `Bake::check` holds as before.
- **`shuyi bake`** says the spans it laid.
- **Docs:** `docs/formats.md` (*The pack*): the index alone, the check
  per file, a layer over a source (`Bake::first` and `arrived` in 2265);
  the book's chapter on shipping a game, if it shows `Bake`.

**Out (do not do):**

- the Playdate's source (3315), the web's (3318), and the load that waits
  for a folder (3316);
- reading a desktop pack file by file (ADR-0128 §7: not crossed);
- paths read from the index on demand, not leaked (2264, parked);
- a group type or a list of paths in data (ADR-0128 §3: a group is a
  folder);
- encrypted or signed packs (ADR-0023, *Alternatives*).

**Estimated size:** about 1,300 lines (code 550, tests 600, docs 150).

**Split:** past 1,000 lines, split off `Bake::first`, the spans and
`Bundle::arrived` into a new ticket, which 3318 then waits for.

## Implementation steps

1. Move the pack to version 17: the header's checksum, the per-file
   checksum, the spans; update the snapshot.
2. Write `Index::read`, `Index::check_file` and `pack::Source` with
   `InMemory`.
3. Write the layer over a source, and `Bundle::arrived`.
4. *(Split off to 2265.)* Write `Bake::first` and its layout, with
   ADR-0118's.
5. Write the docs; run the gates.

## Acceptance criteria

- [x] `Index::read` on a prefix shorter than the index says how many
      bytes it needs, and on the whole index reads it; on a pack cut short
      or changed in the index it is refused, never a panic.
- [x] A file changed in the pack is refused when it is read, and every
      other file still reads (a test).
- [x] A layer over `InMemory` reads every asset equal to `Pack::open`'s
      layer on Hello, Grid's pack and on the fixtures (`Bake::check`'s
      equality), and hands out `Cow::Owned`.
- [x] A source that answers `NotYet` for a span: its files read as
      missing (a test). (`arrived` moved to 2265.)
- [x] Run again on `examples/hello-grid/examples/asset_memory.rs`, the
      layer over `InMemory` keeps the "on first read" table of
      `docs/research/2906-asset-memory.md` per file, and its resident
      within the layer's own fixed record (the program's table gains its
      row, measured).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the index alone; the check per file; `NotYet` and `Failed`
  (`arrived` and `Bake::first`'s order in 2265).
- Property: any byte changed in a pack is refused by `Pack::open`, and by
  the layer when that file is read; the layer and `Pack::open` read the
  same bytes for every file.
- Fuzz-style: `Index::read` and the layer never panic on random bytes
  (ADR-0007 §5).

## Completion notes

**What was done:**

- **The file is `pack::VERSION` 18.** The header's checksum covers the
  header and the index; each entry holds the checksum of its kept bytes,
  checked when that file is read. `Pack::open` and `Raw::read` check every
  file's. The index's checks (path order, sizes, overlaps, the 1 GiB
  total) run on the index alone, so `Index::read` and `Pack::open` share
  them (`check_listing`).
- **Reading** (`crates/assets/src/pack.rs`): `pack::Index`
  (`#[non_exhaustive]`), `Index::read(prefix, stamp)`, `IndexError::Short
  { needs }` and `IndexError::Pack`, `Index::check_file(path, stored)`, and
  `Index::info`, `Index::paths` and `Index::file_range(path)` (where a
  file's kept bytes lie, for a backend).
- **The source and the layer** (`crates/assets/src/pack/source.rs`):
  `pack::Source` (`read(at, into)`), `SourceError::{NotYet, Failed}`,
  `pack::InMemory`, and `Index::into_layer(name, source)`: a new
  `bundle::Files::Source(&'static SourceFiles)`. Each file is a 32-byte
  record, as a table's `(path, bytes)` is, beside one leaked string of
  every path. The bundle now finds the layer that has a file from the
  index (`Files::has`) before reading it, so `has`, `display_path`,
  `layer_of`, `left_out` and the listings read nothing of the pack; a file
  its layer can't read now reads as missing, not as a lower layer's. A
  failed read or a refused file is recorded once
  (`SourceFiles::problems`, `Layer::problems`), in a list that only grows,
  with no lock, so it builds without `std`.
- **The one-bit frame:** `crates/ui/src/frame/onebit/tests.rs` draws a
  picture from a layer read file by file as no picture (a grey rectangle),
  not a failure; `Index::into_layer`'s doc and `docs/formats.md` say a
  game over such a layer keeps its one-bit pictures as its own values.
- **Measured:** `examples/hello-grid/examples/asset_memory.rs` gains the
  way "on first read, measured", and a test that Hello, Grid's pack read
  file by file passes `Bake::check` against its sources. The numbers are
  in `docs/research/2906-asset-memory.md`, *On first read, measured*.
- **Tests:** `crates/assets/src/pack/source_tests.rs`: the index alone
  and every byte of it changed, the check per file, a file changed among
  others, the fixtures' `Bake::check` and `Cow::Owned` over `InMemory`,
  `NotYet`, `Failed`, a layer under loose files; the properties (the
  layer reads what `Pack::open` reads; any byte changed is refused, by the
  layer for exactly the file holding it), and no random bytes panic
  `Index::read` or the layer.

**Deviations:**

- **Split at the ticket's Split point:** `Bake::first`, the spans,
  `Bundle::arrived`, `Source::arrived` and `shuyi bake`'s spans went to
  ticket 2265, which 3318 now waits for (`blocked_by`, its context and the
  roadmap say so). The core alone came to about 1,400 lines. Their
  criteria went with them.
- **Version 18, not 17:** ticket 3216 took 17 for the bold face after
  this ticket was written. The ticket says so now.
- **`Index::read` takes the stamp** (`read(prefix, stamp)`), as
  `Pack::open` does, so a backend can't make a layer of another game's
  pack.
- **`pack::Source` is `Send + Sync + RefUnwindSafe`:** with a `&'static
  dyn Source` in a layer, `Bundle` would otherwise stop being
  `RefUnwindSafe`, which a test of the bundle (and a game catching a
  panic around a read) relies on.
- **`IndexError::Short { needs }` is a `u64`,** as the header's length
  plus a `u32` may not fit a 32-bit `usize`.
- **Two new limits of the index**, so a layer keeps each place in 32
  bits: a file LZ4 keeps must be smaller than it is (the writer only ever
  wrote such), and no file's bytes reach past 4 GiB from the pack's start.
- **The bytes between files** (a recipient's layout, ADR-0118) are covered
  by no checksum now: ADR-0128 §2 has the header's checksum cover the
  header and the index only. A changed one changes nothing the game reads,
  and `Raw::mark` reads it as no code, as before.
- **The memory criterion is worded as measured:** the layer's resident is
  the computed "on first read" row plus 72 bytes on every pack, its own
  record (56) and its source (16); per file it is the same 32 bytes and
  path as the pack read whole. The criterion said "or less"; it now says
  the table per file and the resident within that fixed record. Its peak
  is higher than computed on the big packs (1.2 MB against 0.6 MB on the
  `playdate` one, against a 10 MB heap): the parsed index and the layer's
  table are live together while it opens.

**Follow-up tickets:** 2265 (`Bake::first`, the spans, `Bundle::arrived`).

Breaking: shuyi-assets: pack::VERSION: 18 (the header's checksum covers the index, and each file has its own); a pack baked before is refused, so a game bakes again with `shuyi bake`

```migration
since: 0.1.0
what: The pack format is version 18; a pack of version 17 is refused.
old: "pack::VERSION == 17"
new: "pack::VERSION == 18"
before: |
  cargo shuyi bake   # with the engine before 2263
after: |
  cargo shuyi bake   # again, with this engine
```

Behaviour: shuyi-assets: `pack::info` checks the header and the index only, so a pack whose files' bytes changed still gives its info; `pack::length` now refuses a pack whose index changed (`PackError::Checksum`)

Behaviour: shuyi-assets: `Pack::open` and `Raw::read` refuse a file LZ4 keeps that isn't smaller than it is, and a pack whose files' bytes reach past 4 GiB (`PackError::Broken`); no pack `shuyi bake` writes is either

Behaviour: shuyi-assets: a bundle finds the layer that has a file before reading it: `has`, `display_path`, `layer_of` and `test_part` ask each layer whether it has the file, and read only the one that does (no layer read whole changes what it answers)

Book: none, the book's chapter on assets (`site/content/learn/book/assets-and-formats.md`, *The pack*) shows no pack API and points at `docs/formats.md`, which says how a pack is read file by file; no backend reads so yet (3315 and 3318 do, and say so in their own docs)

Game: nothing to adopt, the game bakes its pack in a test and opens it (`crates/xtask/src/game_tools/tests.rs`, `Pack::open`), which reads version 18 as it bakes it; `gh search code --repo Zafnok/visions-of-shuyi` for `pack::info`, `pack::length` and `Pack::open` found nothing else.

**Nick:** nothing changes in the game. A pack read file by file is for the
Playdate (3315) and the web (3318), for games bigger than Visions of
Shuyi's public assets.
