---
id: "2267"
title: "A file read from a pack file by file is copied once, a source that holds the whole pack can lend it, and a folder's files are found by search"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2267 — Pack reads copied once, sources that lend, folders found by search

## Context

[ADR-0128](../../docs/adr/0128-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§2 and §3; 2263 (a pack read file by file, #479) and 2265 (folders laid
first, `arrived`), both done. Found by the retro of PRs 379–484 (its
item T21). Checked in the code on 2026-11-17:

- **Two buffers a stored file.** `SourceFiles::fetch`
  (`crates/assets/src/pack/source.rs`, about lines 239–258) reads the
  stored bytes into a `Vec`, `checked` (`crates/assets/src/pack.rs`,
  about line 753) hands back `Cow::Borrowed(stored)` for
  `Codec::Stored`, and `.map(Cow::into_owned)` copies it into a second
  `Vec` before the first is dropped. Each read of a stored file holds
  twice its size at once. (An LZ4 file needs its stored and its unpacked
  bytes at once: that stays.)
- **Every read copies, even from bytes the program holds.**
  `pack::Source` has one way to give bytes, `read(at, into: &mut [u8])`
  (`source.rs`, about line 30): it can only copy out. `InMemory` holds
  the whole pack for the run and copies anyway. 3318's web source
  (`Stream`, `crates/app/src/pack_file/stream.rs`) is a buffer of the
  whole pack leaked for the run, so every read of the web is a copy and a
  checksum of a file already in memory.
- **A folder not laid first is found by a scan.** `SourceFiles::arrived`
  (`source.rs`, about lines 203–215) walks every file of the pack and
  keeps those under `dir` when the folder has no span, though `files` is
  sorted by path. `Bundle::arrived` (`crates/assets/src/bundle.rs`, about
  lines 701–720) is for a screen to ask until it is true, frame after
  frame: 7,248 files (the `playdate` synthetic pack) each frame.
- **The index copied to skip 8 bytes.** `header_sum` (`pack.rs`, about
  lines 563–567) copies the header and the whole index into a new `Vec`
  to leave out the checksum's own 8 bytes (`SUM_AT`, 12..20), on every
  `Index::read` and `Pack::open`. `fnv::Hasher::bytes` takes 8 bytes a
  word (`crates/assets/src/fnv.rs`), so hashing the two runs one after
  the other gives another value: the words must run across the gap.
- 2906's measure of 2263's layer
  ([`docs/research/2906-asset-memory.md`](../../docs/research/2906-asset-memory.md),
  *On first read, measured*): a peak of 1,156,323 bytes on the
  `playdate` pack, against 588,016 computed, set by opening (the index's
  entries and the layer's table live at once). This ticket records the
  peak before and after; it doesn't promise to close that gap.

## Nick input

None.

## Scope

**In:**

- **One buffer a stored read:** `fetch` hands out the `Vec` it read into
  when `checked` gives `Borrowed`, and the unpacked `Vec` when it gives
  `Owned`, never a copy.
- **`Source::lend(&self, at: u64, len: u64) -> Option<&'static [u8]>`**,
  a provided method that gives `None` (additive: a trait method with a
  default, ADR-0039). Its contract, in its doc comment: it lends only
  bytes that have arrived and never change again for the run, and `None`
  says "use `read`" (not that they are missing). `InMemory` lends.
- **`SourceFiles::read` borrows where the source lends:** a stored file
  is handed out `Cow::Borrowed` from the lent bytes, an LZ4 file is
  unpacked from them, with no copy of the stored bytes. A lent file's
  checksum is checked on its first read only: `SourceFile` gains a flag
  (an `AtomicBool`, as `failed` is) and stays 32 bytes (the `const`
  assert stays). A file whose check fails is refused as today, and every
  read after is refused too.
- **`SourceFiles::arrived` by search:** a folder with no span finds the
  first path under it by binary search (`partition_point` over the sorted
  files) and walks only the files under it.
- **`header_sum` without a copy:** the same value, from the bytes before
  and after `SUM_AT` taken as one run of 8-byte words (a helper on
  `fnv::Hasher`, or in `pack.rs`). `pack::VERSION` and every pack's bytes
  stay as they are.
- **A heap count test:** `crates/assets/tests/pack_alloc.rs`, its own
  test binary with `shuyi-alloc-count` (`features = ["global"]`) as a
  dev-dependency of `shuyi-assets` (dev-dependencies are free in
  `tools/layers.ron`), as `crates/sound/tests/mix_alloc.rs` counts.
- **Docs:** the doc comments of `Source`, `lend`, `InMemory`,
  `Index::into_layer` and `Bundle::bytes` (a layer over a source that
  lends lends its stored files); a line in ADR-0128 §2 naming `lend`.
  Completion notes: a `Behaviour:` line (`shuyi-assets`: a layer over
  `InMemory` hands out a stored file `Cow::Borrowed`, not `Owned`).
- **From 3318 (done):** its web source (`Stream`,
  `crates/app/src/pack_file/stream.rs`) implements `lend` for the bytes
  that have arrived, with a unit test there that a read of an arrived
  stored file copies nothing. Its buffer sits behind a `Mutex` today, and
  bytes before `filled` are never written again: lending them as
  `&'static [u8]` needs the buffer held so that the arrived part can be
  borrowed while the rest is filled (say the leaked buffer as a raw
  `&'static` slice split at `filled`), with no `unsafe` beyond what
  `shuyi-app` already allows for its plugins.
- **Either order with 3220:** if 3220 is done, `Bundle::lent` lends a
  stored file of a layer whose source lends it (through the same checked
  borrow as `read`), with a test; if not, nothing to do (3220 uses
  `Source::lend` if this ticket is done first).

**Out (do not do):**

- making `Pack::open` + `Pack::into_layer` a layer over `InMemory`:
  ADR-0128 §2 keeps a pack read whole checking every file at start, and
  unpacking its LZ4 files once for the run; over a lending source an LZ4
  file is unpacked at each read. Nothing measured calls for merging them;
- the Playdate's source (3315): it reads a file, and has nothing to lend;
- the opening peak itself (the index's entries beside the table): no
  profile is near its heap with it (1.2 MB of the Playdate's 10 MB);
- paths read on demand (2264, parked).

**Estimated size:** about 700 lines (code 250, tests 400, docs 50).

## Implementation steps

1. Write the heap count test first, over a pack with one stored file of
   64 KiB read through `Index::into_layer` and a source that doesn't
   lend; see it fail (two buffers).
2. Fix `fetch`.
3. Add `Source::lend`, `InMemory`'s, the checked flag and the borrowed
   read.
4. `arrived` by search; `header_sum` without the copy.
5. Measure `examples/hello-grid/examples/asset_memory.rs`'s rows (as
   2906 did) before and after; write the docs; run the gates.

## Acceptance criteria

- [ ] Reading a 64 KiB stored file through a source that doesn't lend
      allocates its bytes once: the counter's peak rises by less than
      twice the file's size, and one allocation of at least its size is
      made (`pack_alloc.rs`).
- [ ] Over `InMemory`, the same read allocates no buffer of the file's
      size and hands out `Cow::Borrowed`; an LZ4 file is unpacked into
      one buffer of its size.
- [ ] A lent file changed after the bake is refused on its first read and
      on every read after; a lent file read twice is checksummed once (a
      test that counts, or reads the flag).
- [ ] `Bundle::arrived` over a folder with no span answers as before
      (the proptests of `source_tests.rs` and `first_tests.rs` pass
      unchanged), and a test over a source that counts `arrived` calls
      shows only the folder's files are asked about.
- [ ] `header_sum` gives the value the old one did, for any head of at
      least 20 bytes (a proptest against the old function, kept in the
      test module); the golden packs are unchanged.
- [ ] The completion notes give `asset_memory`'s peak and resident for
      Hello, Grid, `web` and `playdate`, before and after.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `fetch` with each codec; `lend`'s borrowed read, its first-read
  check and a refused lent file; `arrived` by search.
- Property: `header_sum` against the old copy; the existing proptests of
  `source_tests.rs` and `first_tests.rs`.
- Integration: `crates/assets/tests/pack_alloc.rs`, the heap count.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
