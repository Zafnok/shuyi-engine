---
id: "2263"
title: "The pack read file by file: its index read alone, a check per file, folders laid in the order the bake names, and a layer over a source the backend gives"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2262"]
nick_input: none
completed:
---

# 2263 — The pack read file by file

## Context

[ADR-0127](../../docs/adr/0127-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
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

- **The file, `pack::VERSION` 17** (ADR-0023 §5: refused, never
  migrated):
  - the header's checksum covers the header and the index only;
  - each entry gains the checksum of its stored bytes (the same FNV-1a
    word hash, `checksum`);
  - the index gains each folder's span: for every folder `Bake::first`
    named, the offset and length its files take, in the order named;
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
  - a file that is `NotYet` reads as missing, and a `Failed` read or a
    bad checksum as missing too, told to the `watch`er; the layer's
    `problems()` lists each failure once, for the backend to log;
  - the layer stands for the baked bundle's fingerprint, as
    `Pack::into_layer`'s does.
- **`Bundle::arrived(&self, dir) -> bool`**: whether every file under
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
  per file, a layer over a source, `Bake::first`, `arrived`; the book's
  chapter on shipping a game, if it shows `Bake`.

**Out (do not do):**

- the Playdate's source (3315), the web's (3318), and the load that waits
  for a folder (3316);
- reading a desktop pack file by file (ADR-0127 §7: not crossed);
- paths read from the index on demand, not leaked (2264, parked);
- a group type or a list of paths in data (ADR-0127 §3: a group is a
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
4. Write `Bake::first` and its layout, with ADR-0118's.
5. Write the docs; run the gates.

## Acceptance criteria

- [ ] `Index::read` on a prefix shorter than the index says how many
      bytes it needs, and on the whole index reads it; on a pack cut short
      or changed in the index it is refused, never a panic.
- [ ] A file changed in the pack is refused when it is read, and every
      other file still reads (a test).
- [ ] A layer over `InMemory` reads every asset equal to `Pack::open`'s
      layer on Hello, Grid's pack and on the fixtures (`Bake::check`'s
      equality), and hands out `Cow::Owned`.
- [ ] A source that answers `NotYet` for a span: its files read as
      missing, `arrived` is false for their folder and true for the
      others (a test).
- [ ] `Bake::first("a/")` lays `a/`'s files first, and the pack with a
      recipient's layout keeps them in their span.
- [ ] Run again on `examples/hello-grid/examples/asset_memory.rs`, the
      layer over `InMemory` keeps the "on first read" resident of
      `docs/research/2906-asset-memory.md` or less (the program's table
      gains its row, measured).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the index alone; the check per file; `NotYet` and `Failed`;
  `arrived`; `Bake::first`'s order.
- Property: any byte changed in a pack is refused by `Pack::open`, and by
  the layer when that file is read; the layer and `Pack::open` read the
  same bytes for every file.
- Fuzz-style: `Index::read` and the layer never panic on random bytes
  (ADR-0007 §5).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
