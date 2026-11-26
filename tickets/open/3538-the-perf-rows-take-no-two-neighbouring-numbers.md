---
id: "3538"
title: "The perf rows take no two neighbouring numbers"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3537"]
nick_input: none
completed:
---

# 3538 — The perf rows take no two neighbouring numbers

## Context

[ADR-0151](../../docs/adr/0151-constructors-take-no-two-neighbouring-numbers.md)
says a constructor of a `#[non_exhaustive]` type takes no two neighbouring
integers, floats or tuples of one type, and `cargo xtask api` checks it
against `tools/constructors.ron`. 3537 changed the bots' settings and five
other constructors, and split these off at its **Split:** point (1,000
lines). Each is listed in the file as "ticket 3538 changes it":

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

- [ ] No entry of `tools/constructors.ron` says "ticket 3538 changes it".
- [ ] `cargo xtask api` passes on the branch.
- [ ] For each constructor that changes, a test shows it builds what a
      literal built.
- [ ] The completion notes have a `Breaking:` line and a `migration` block
      for each constructor that changed.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- For each constructor that changes, a test shows it builds what a literal
  built.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
