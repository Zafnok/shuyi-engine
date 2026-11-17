---
id: "2264"
title: "Parked: A pack's paths read from its index when asked, not leaked, for the DS's heap"
type: feature
milestone: "Parked: export targets"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2263"]
nick_input: none
completed:
---

# 2264 — A pack's paths read from its index when asked

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): only the parked `nds` target needs it, so it waits with 3024 until Nick unparks the DS. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0127](../../docs/adr/0127-asset-memory-bytes-handed-out-a-pack-read-file-by-file-folders-as-groups.md)
§1, §6 and §7, from ticket 2906's numbers
([`docs/research/2906-asset-memory.md`](../../docs/research/2906-asset-memory.md)):

- A layer's paths are `&'static str`, leaked once when it is made, so a
  pack read file by file (2263) still keeps its table: about 52 bytes a
  file, 376 KB for the 7,248 files of a 40 MB pack.
- That is 3.6% of the Playdate's 10 MB heap, under ADR-0127's trigger (a
  tenth of the heap). ADR-0012's parked `nds` row allows a 2.5 MB heap
  and a 64 MB ROM: about 11,000 files in the game's proportions, whose
  table takes about 575 KB, 23% of the heap.
- The paths reach far: `files_in`, `files_under`, `ImageId`, `ImageTable`
  and the keys of `Formats::load_under` and `load_each` are `&'static
  str` (ADR-0127, *The call sites, counted*: `ImageId` alone is in 72
  lines of 19 files in the engine and 20 files of the game).

## Nick input

None.

## Scope

**In:**

- **Measure first:** the table of the `nds` profile's synthetic pack,
  with `examples/hello-grid/examples/asset_memory.rs` (its
  `--write-synthetic` and the table column), once `targets/nds.ron`
  exists (3024). If it is under a tenth of the profile's
  `peak_heap_bytes`, write that number in the completion notes, close
  this ticket and change nothing else.
- **Otherwise, the smallest way under it**, decided in an ADR that
  extends ADR-0127 §1, weighing:
  - the index kept as its postcard bytes, with each path read from it
    when asked (a sorted table of offsets and a binary search), and paths
    handed out as `Cow<'static, str>` as 2262 does bytes;
  - paths hashed to 64 bits in the index, with the names only in a debug
    build;
  - a pack per part of the game, each with its own small index.
- The build it decides, with the `Breaking:` lines a path type's change
  needs (this is after the first release: a 0.x break, 2005's policy).
- **Either order with 3024:** if 3024 is done, its source over the
  ROM's file system is the one measured here; if not, nothing to do
  (3024 reads its pack through 2263's `Source`).

**Out (do not do):**

- the Playdate (3.6% of its heap: ADR-0127 §7, not crossed);
- the DS backend itself (3024).

**Estimated size:** about 1,200 lines (code 500, tests 500, docs 200), if
the measure crosses the trigger.

**Split:** past 1,000 lines, split off the change to `ImageId` and
`ImageTable` into a new ticket.

## Implementation steps

1. Measure the `nds` table; stop there if it is under a tenth of the
   heap.
2. Write the ADR.
3. Build it, with its migration blocks.
4. Run the gates.

## Acceptance criteria

- [ ] The `nds` table is measured and written in the completion notes.
- [ ] If it crosses a tenth of the heap: the ADR is written, and the
      table of a 64 MB pack read file by file is under a tenth of the
      `nds` heap, measured.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit and property, as the ADR's build needs: every path the old index
  had is found, and none it lacked.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
