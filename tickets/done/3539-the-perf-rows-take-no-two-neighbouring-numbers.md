---
id: "3539"
title: "The perf rows take no two neighbouring numbers"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["3537"]
nick_input: none
completed: 2026-11-26
---

# 3539 — The perf rows take no two neighbouring numbers

## Context

[ADR-0151](../../docs/adr/0151-constructors-take-no-two-neighbouring-numbers.md)
says a constructor of a `#[non_exhaustive]` type takes no two neighbouring
integers, floats or tuples of one type, and `cargo xtask api` checks it
against `tools/constructors.ron`. 3537 changed the bots' settings and five
other constructors, and split these off at its **Split:** point (1,000
lines). Each is listed in the file as "ticket 3539 changes it":

- `shuyi-ui`'s `perf::HeapCounts::new(u64, u64, u32)`,
  `HeapDiff::new(u32, u32)`, `HeapSnapshot::new(u64, u64, Vec<TagRow>)`,
  `SpanEvent::new(name, u32, u32, u8)`, `TagDelta::new(class, name, i64,
  i64)` and `TagRow::new(class, name, u64, u32)`;
- `shuyi-cli`'s `contract::perf::PerfRead::of(&PerfRing, u64, u32)`;
- `shuyi-alloc-count`'s `Counts::new(u64, u64, u32)` and
  `TagCounts::new(u16, u8, &str)`.

3537 had them changed once (about 550 counted lines) in these shapes, which
read well:

- `HeapCounts::new(live_bytes)` and `Counts::new(live_bytes)`, the peak
  the live bytes and no call, with `with_peak_bytes` and `with_allocs`;
- `HeapSnapshot::new(live_bytes, tags)`, its peak the live bytes, with
  `with_peak_bytes`;
- `TagRow::new(class, name, live_bytes)` and `TagDelta::new(class, name,
  live_bytes)`, 0 blocks, with `with_live_blocks`;
- `SpanEvent::new(name, us)`, at the frame's start and depth 0, with
  `with_start_us` and `with_depth`;
- `HeapDiff::until(to)` from frame 0, with `with_from`;
- `PerfRead::of(ring, since)` reading every kept record from `since`, and
  `.first(max)` keeping the first `max` and lowering `next` to match;
- `TagCounts::new(tag, name)` of `tags::OTHER_CLASS`, with `with_class`.

No call in the game (`gh search code --repo Zafnok/visions-of-shuyi`,
2026-11-26).

## Nick input

None.

## Scope

**In:**
- Each constructor above changes shape as ADR-0151 says (the shapes above,
  unless one reads better), keeping `const fn` where it was.
- Each one's entry leaves `tools/constructors.ron`.
- All their call sites change: the engine's crates and tests (`shuyi-ui`,
  `shuyi-cli`, `shuyi-platform`, `shuyi-app`, `shuyi-playdate`,
  `shuyi-alloc-count`) and `docs/platform-backends.md`'s `memory` section.
- The contract's schemas, if a description names a constructor
  (`HeapDiff`'s does): regenerated, with their `versions.json` line.
- `Breaking:` lines with `migration` blocks for each.

**Out (do not do):**
- The constructors 3537 changed, and those the file lists for their order.

**Estimated size:** about 550 lines (code 300, tests 200, docs 50).

## Implementation steps

1. Read ADR-0151 and 3537's completion notes; run `cargo xtask api` to see
   the check pass before any change.
2. Change the constructors crate by crate, with their call sites, and take
   each entry out of the file.
3. Regenerate the schemas (`cargo shuyi schema --out crates/cli/schemas`)
   and add the line their test asks for.
4. Write the `Breaking:` lines and the `migration` blocks. Run `cargo
   xtask api` and the gates.

## Acceptance criteria

- [x] No entry of `tools/constructors.ron` says "ticket 3539 changes it".
- [x] `cargo xtask api` passes on the branch.
- [x] For each constructor that changes, a test shows it builds what a
      literal built.
- [x] The completion notes have a `Breaking:` line and a `migration` block
      for each constructor that changed.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- For each constructor that changes, a test shows it builds what a literal
  built.

## Completion notes

**What was done.**

- The nine constructors the file listed "ticket 3539 changes it" changed
  shape as ADR-0151 says, in the shapes 3537 gave them, and their entries
  left `tools/constructors.ron`. Each kept `const fn` where it was, and
  every new `with_` method is a `#[must_use] const fn` (but
  `PerfRead::first`, which truncates a `Vec`).
  - `shuyi-ui`'s `perf`: `HeapCounts::new(live_bytes)` (peak the live
    bytes, no call) with `with_peak_bytes` and `with_allocs`;
    `HeapSnapshot::new(live_bytes, tags)` (peak the live bytes) with
    `with_peak_bytes`; `TagRow::new(class, name, live_bytes)` and
    `TagDelta::new(class, name, live_bytes)` (0 blocks) with
    `with_live_blocks`; `SpanEvent::new(name, us)` (at the frame's start,
    depth 0) with `with_start_us` and `with_depth`; `HeapDiff::new`
    became `HeapDiff::until(to)` (from frame 0) with `with_from`.
  - `shuyi-cli`'s `PerfRead::of(ring, since)` reads every kept record
    from `since`, and `PerfRead::first(max)` keeps the first `max`,
    lowering `next` to the one after the last kept.
  - `shuyi-alloc-count`: `Counts::new(live_bytes)` with
    `with_peak_bytes` and `with_allocs`; `TagCounts::new(tag, name)` of
    `tags::OTHER_CLASS` with `with_class`.
- Call sites in `shuyi-ui`, `shuyi-cli`, `shuyi-platform`, `shuyi-app`,
  `shuyi-playdate` and `shuyi-alloc-count`, and `docs/platform-backends.md`'s
  `memory` section. No template, example, eval, skill or site page builds
  any of them (searched `templates/`, `examples/`, `evals/`, `skills/`,
  `site/`).
- The schemas: `HeapDiff`'s description names `HeapDiff::until`; four
  output schemas regenerated, and `versions.json` has the new fingerprint
  at version 62 (a description changes nothing a client reads).
- Tests that each builds what a literal built:
  `the_heaps_counts_build_what_a_literal_built`,
  `a_span_builds_what_a_literal_built`,
  `a_tags_row_and_change_build_what_a_literal_built` and
  `a_snapshot_and_a_diff_build_what_a_literal_built` (`shuyi-ui`),
  `counts_build_what_a_literal_built` and
  `a_tags_counts_build_what_a_literal_built` (`shuyi-alloc-count`), and
  `a_read_of_its_first_records_builds_what_a_literal_built` (`shuyi-cli`),
  which also checks `first` past the records read and `first(0)`.

**Deviations.** None.

**Follow-up tickets.** 3540, which 2243 (#611) filed first: the local
`site` gate lacks CI's copies of the examples' shots and previews, so `zola
check` failed here until `bash site/copy-shots.sh` was run by hand. None of
this ticket's work is left.

**For Nick.** The API check ran out of disk on D: (1.6 GB free); `cargo
xtask trim-targets` (the default trim a passing gate run does) freed
22.6 GB of idle worktrees' incremental caches.

Game: nothing to adopt, the game calls none of these (`gh search code --repo Zafnok/visions-of-shuyi` for `HeapCounts`, `HeapSnapshot`, `TagRow`, `TagDelta`, `HeapDiff`, `SpanEvent`, `PerfRead`, `Counts` and `TagCounts`, 2026-11-26: no hit but unrelated `counts` functions)

Book: none, no chapter of `site/content/learn/` or snippet of `site/snippets/` builds any of these constructors (searched)

Skills: none, the schemas changed only a description naming a constructor, and no skill builds or names these types

Contract: none, only `HeapDiff`'s description changed (it names `HeapDiff::until`); no operation or field did, and `docs/contract.md` names no constructor

Playdate: none, the backend only builds its heap's counts through the new `HeapCounts::new` shape; nothing it does or reads changed

Breaking: shuyi-ui: `perf::HeapCounts::new`: takes only the live bytes (peak the same, no call); set the rest with `with_peak_bytes` and `with_allocs`

```migration
since: 0.1.0
what: The heap's counts take the live bytes, and name the peak and the calls.
old: HeapCounts::new(u64, u64, u32)
new: HeapCounts::new(u64).with_peak_bytes(u64).with_allocs(u32)
before: |
  let heap = HeapCounts::new(counts.live_bytes, counts.peak_bytes, counts.allocs);
after: |
  let heap = HeapCounts::new(counts.live_bytes)
      .with_peak_bytes(counts.peak_bytes)
      .with_allocs(counts.allocs);
find: ["HeapCounts::new(counts.live_bytes, counts.peak_bytes"]
```

Breaking: shuyi-ui: `perf::HeapSnapshot::new`: takes the live bytes and the rows (peak the live bytes); set the peak with `with_peak_bytes`

```migration
since: 0.1.0
what: A heap snapshot takes its live bytes and rows, and names its peak.
old: HeapSnapshot::new(u64, u64, Vec<TagRow>)
new: HeapSnapshot::new(u64, Vec<TagRow>).with_peak_bytes(u64)
before: |
  let snapshot = HeapSnapshot::new(4096, 8192, rows);
after: |
  let snapshot = HeapSnapshot::new(4096, rows).with_peak_bytes(8192);
find: ["HeapSnapshot::new(4096, 8192"]
```

Breaking: shuyi-ui: `perf::TagRow::new`: takes the class, the name and the live bytes (0 blocks); set the blocks with `with_live_blocks`

```migration
since: 0.1.0
what: A tag's row takes its live bytes, and names its blocks.
old: TagRow::new(TagClass, impl Into<Cow<'static, str>>, u64, u32)
new: TagRow::new(TagClass, impl Into<Cow<'static, str>>, u64).with_live_blocks(u32)
before: |
  let row = TagRow::new(TagClass::Phase, "update", 1024, 3);
after: |
  let row = TagRow::new(TagClass::Phase, "update", 1024).with_live_blocks(3);
find: ["TagRow::new(TagClass::Phase, \"update\", 1024, 3)"]
```

Breaking: shuyi-ui: `perf::TagDelta::new`: takes the class, the name and the change in live bytes (no change in blocks); set the blocks with `with_live_blocks`

```migration
since: 0.1.0
what: A tag's change takes its bytes, and names its blocks.
old: TagDelta::new(TagClass, impl Into<Cow<'static, str>>, i64, i64)
new: TagDelta::new(TagClass, impl Into<Cow<'static, str>>, i64).with_live_blocks(i64)
before: |
  let delta = TagDelta::new(TagClass::Span, "ai", -512, -2);
after: |
  let delta = TagDelta::new(TagClass::Span, "ai", -512).with_live_blocks(-2);
find: ["TagDelta::new(TagClass::Span, \"ai\", -512, -2)"]
```

Breaking: shuyi-ui: `perf::SpanEvent::new`: takes the name and its microseconds (at the frame's start, inside no span); set the rest with `with_start_us` and `with_depth`

```migration
since: 0.1.0
what: A span takes its name and length, and names its start and depth.
old: SpanEvent::new(impl Into<Cow<'static, str>>, u32, u32, u8)
new: SpanEvent::new(impl Into<Cow<'static, str>>, u32).with_start_us(u32).with_depth(u8)
before: |
  let span = SpanEvent::new("ai", 40, 250, 1);
after: |
  let span = SpanEvent::new("ai", 250).with_start_us(40).with_depth(1);
find: ["SpanEvent::new(\"ai\", 40, 250"]
```

Breaking: shuyi-ui: `perf::HeapDiff::new`: removed; use `HeapDiff::until(to)` (from frame 0) and `with_from`

```migration
since: 0.1.0
what: An empty heap diff names the frame it runs until, and the one it runs from.
old: HeapDiff::new(u32, u32)
new: HeapDiff::until(u32).with_from(u32)
before: |
  let diff = HeapDiff::new(1, 600);
after: |
  let diff = HeapDiff::until(600).with_from(1);
find: ["HeapDiff::new("]
```

Breaking: shuyi-cli: `contract::perf::PerfRead::of`: takes the ring and `since`, reading every kept record from it; keep the first ones with `first`

```migration
since: 0.1.0
what: A perf read takes the ring and where to read from, and names how many to keep.
old: PerfRead::of(&PerfRing, u64, u32)
new: PerfRead::of(&PerfRing, u64).first(u32)
before: |
  let read = PerfRead::of(&ring, since, max);
after: |
  let read = PerfRead::of(&ring, since).first(max);
find: ["PerfRead::of(&ring, since, max)"]
```

Breaking: shuyi-alloc-count: `Counts::new`: takes only the live bytes (peak the same, no call); set the rest with `with_peak_bytes` and `with_allocs`

```migration
since: 0.1.0
what: A counter's counts take the live bytes, and name the peak and the calls.
old: Counts::new(u64, u64, u32)
new: Counts::new(u64).with_peak_bytes(u64).with_allocs(u32)
before: |
  let counts = Counts::new(60, 100, 2);
after: |
  let counts = Counts::new(60).with_peak_bytes(100).with_allocs(2);
find: ["Counts::new(60, 100"]
```

Breaking: shuyi-alloc-count: `TagCounts::new`: takes the tag and its name (of `tags::OTHER_CLASS`); set the class with `with_class`

```migration
since: 0.1.0
what: A tag's counts take the tag and its name, and name its class.
old: TagCounts::new(u16, u8, &'static str)
new: TagCounts::new(u16, &'static str).with_class(u8)
before: |
  let row = TagCounts::new(tag, class, name);
after: |
  let row = TagCounts::new(tag, name).with_class(class);
find: ["TagCounts::new(tag, class, name)"]
```
