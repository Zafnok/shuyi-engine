---
id: "3315"
title: "The Playdate reads its pack file by file through the SDK's seek and read, so a pack larger than its heap fits"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2262", "2263"]
nick_input: none
completed:
---

# 3315 — The Playdate reads its pack file by file

## Context

[ADR-0128](../../docs/adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§2 and §6, from ticket 2906's numbers
([`docs/research/2906-asset-memory.md`](../../docs/research/2906-asset-memory.md),
*The Playdate*):

- `shuyi_playdate::pack()` (`crates/playdate/src/device/mod.rs`) calls
  `read_pack` (`crates/playdate/src/backend.rs`), which reads the whole
  pack with `Sdk::read_bundled` and leaks it (3010). A game opens it with
  `Pack::open` and lays it in its bundle.
- Read whole and kept, a pack takes 1.6 times its bytes, so one over about
  6 MB doesn't fit the 10 MB heap; the `playdate` profile allows a 40 MB
  package. Embedding it is no way out: the launcher loads the program
  into the same RAM.
- Read file by file, the 40 MB synthetic pack peaks at 0.6 MB of heap and
  keeps 0.4 MB (its table and the decoded values).
- *Inside Playdate with C*, section 7.4, *File handles*: `open` with
  `kFileRead` opens a file in the `.pdx`; `seek(file, pos, whence)` and
  `read(file, buf, len)` read a byte range; at most 64 files are open at
  once. Section 1.2 warns that disk access is slow on the hardware.
- 2263 gives `pack::Index::read`, `pack::Source` and a layer that reads
  one file per `Bundle::bytes`, handed out as `Cow::Owned` (2262).

## Nick input

None.

## Scope

**In:**

- **The SDK's calls:** `Sdk` (`crates/playdate/src/sdk.rs`) gains
  `open_bundled(path) -> Result<Self::File, String>`, `seek(file, at)`
  and `read(file, into: &mut [u8]) -> Result<usize, String>`, with
  `Self::File: Copy + fmt::Debug`. The device's are the C shim's
  (ADR-0062: a function each in its C file, checked against the SDK's
  header on the day), and the fake's (`crates/playdate/src/fake.rs`) read
  its bundled files' bytes.
- **The source:** a `pack::Source` over one file kept open for the run,
  reading `at..at + len` with a seek and reads until `into` is full. It is
  `Send + Sync` as `pack::Source` asks (the Playdate runs one thread;
  say how in its doc comment).
- **`shuyi_playdate::pack_layer(name, stamp) -> Result<Layer, String>`**:
  reads the pack's header and index (`Index::read`, twice at most: the
  header, then the length it says), checks the stamp, and makes the layer
  over the source. `pack()` stays for a game that wants its pack whole,
  its doc comment saying when (a pack under about 6 MB) and pointing at
  `pack_layer`.
- **The run report** (3090's `RunReport`): the slow reads it already
  lists count the pack's reads too, so a part that reads many small files
  shows.
- **`examples/hello-grid`'s Playdate build** uses `pack_layer`.
- **Docs:** `crates/playdate/README.md` (or its module comment) and the
  book's Playdate chapter: a pack is read file by file; what stays (the
  index, about 52 bytes a file) and what a game keeps (its values).
- **For the game:** additive. Its Playdate front end (game#1170) may move
  from `pack()` to `pack_layer` when its pack passes about 6 MB.

**Out (do not do):**

- measuring on a device (3011). **Either order with 3011:** if 3011 is
  done, add a line to `docs/targets/playdate.md` saying its numbers were
  of a pack read whole, and that a pack read file by file awaits the next
  device run; if not, nothing to do (3011 reports how the pack was
  read);
- the load waiting or showing its count (3316): the Playdate's reads are
  synchronous and never `NotYet`;
- the parked DS's source (3024, 2264).

**Estimated size:** about 700 lines (code 300, tests 300, docs 100).

## Implementation steps

1. Add the SDK's `open_bundled`, `seek` and `read`, to the C shim, the
   device and the fake.
2. Write the source and `pack_layer`.
3. Move Hello, Grid's Playdate build onto it; write the docs.
4. Run the gates.

## Acceptance criteria

- [ ] On the fake SDK, a pack read through `pack_layer` loads Hello,
      Grid equal to the pack read whole (`Pack::open`).
- [ ] On the fake SDK with the heap counter, the `playdate` synthetic
      pack (`examples/hello-grid/examples/asset_memory.rs
      --write-synthetic playdate`) loads Hello, Grid with a peak under
      1 MB of heap, against 68 MB read whole (a test, its number in the
      completion notes).
- [ ] A read that fails in the SDK leaves the file missing and is logged
      once, never a panic.
- [ ] CI's jobs that build the Playdate's device crate (the C shim)
      pass.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the source's seek and read on the fake, with short reads; the
  stamp refused; a read past the end.
- Integration: Hello, Grid on the fake SDK through `pack_layer`, and the
  peak heap of the synthetic pack.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
