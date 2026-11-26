---
id: "3537"
title: "The bots' settings, the perf rows and five other constructors take no two neighbouring numbers"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: ["3503"]
nick_input: none
completed:
---

# 3537 — The bots' settings, the perf rows and five other constructors take no two neighbouring numbers

## Context

3503 made the rule of
[ADR-0150](../../docs/adr/0150-constructors-take-no-two-neighbouring-numbers.md):
a constructor of a `#[non_exhaustive]` type takes no two neighbouring
integers, floats or tuples of one type, which a call could swap without a
word. `cargo xtask api` checks it, and `tools/constructors.ron` lists what
may break it. 3503 changed nine constructors and, past its size, split off
these, each listed in the file as "ticket 3537 changes it":

- the bots' settings in `shuyi-core`: `PlanSettings::new(u32, u32, u32,
  u32, u32)`, `RolloutSettings::new(u32, u32, Horizon, u32, u32)`,
  `BalanceSettings::new(Tries, u32, u32)`, `Tries::new(u32, u64, u32)` and
  `threat::Strike::new(i64, i64, i64)`;
- the perf rows: `shuyi-ui`'s `perf::HeapCounts::new`, `HeapDiff::new`,
  `HeapSnapshot::new`, `SpanEvent::new`, `TagDelta::new` and `TagRow::new`,
  `shuyi-cli`'s `contract::perf::PerfRead::of(&PerfRing, u64, u32)`, and
  `shuyi-alloc-count`'s `Counts::new(u64, u64, u32)` and
  `TagCounts::new(u16, u8, &str)`;
- five more: `shuyi-ui`'s `grid::layout::Grid::top_down(Pos, (i32, i32),
  (i32, i32), (i32, i32), i32)`, `shuyi-assets`' `sfx::SfxDef::new(Wave,
  u32, u32, u32, u8)`, `shuyi-app`'s `page::PageLimits::new(u64, u64)`,
  `shuyi-cli`'s `contract::bake::PackSpan::new(_, u64, u64)` and
  `shuyi-sound`'s `pcm::Limits::new(u32, u64)`.

The game calls `RolloutSettings::new` in its copy of the skill pack's
`shuyi-balance-bots` example (`gh search code --repo
Zafnok/visions-of-shuyi`, 2026-11-25).

## Nick input

None.

## Scope

**In:**
- Each constructor above changes shape as ADR-0150 says: a named
  constructor, a `with_<field>` (from a `DEFAULT` where no field must be
  chosen), or a plain value whose fields name them. Keep `const fn` where it
  was. 3503's `PadState`, `StickDef` and `MusicSwitch` are the pattern.
- Each one's entry leaves `tools/constructors.ron`: the check fails it as
  stale once its constructor changes.
- **From 3503 (done):** `tools/sound-parity/footprint.ron` has a
  `Replace` that takes `Limits::new`'s entry out of
  `tools/constructors.ron` when the sound replacement is given up. Remove
  that edit with the entry (`giving_up_covers_all_the_chain_added` fails
  on an edit whose text is gone).
- All their call sites change: in the engine, the templates, the tutorial's
  steps, the site's snippets and the skill pack's examples (`skills/`).
- `Breaking:` lines with `migration` blocks for each.

**Out (do not do):**
- The constructors the file lists for their order (a width then a height).
- The check itself (3503).

**Estimated size:** about 900 lines (code 600, tests 200, docs 100).

**Split:** past 1,000 lines, split off the perf rows into a new ticket of
the same milestone, and leave them in `tools/constructors.ron` until it
lands.

## Implementation steps

1. Read ADR-0150, then the entries "ticket 3537 changes it" in
   `tools/constructors.ron`; run `cargo xtask api` to see the check pass
   before any change.
2. Change the constructors crate by crate, with their call sites, and
   take each entry out of the file.
3. Write the `Breaking:` lines and the `migration` blocks. Run `cargo
   xtask api` and the gates.

## Acceptance criteria

- [ ] No entry of `tools/constructors.ron` says "ticket 3537 changes it".
- [ ] `cargo xtask api` passes on the branch.
- [ ] For each constructor that changes, a test shows it builds what it
      built before.
- [ ] The completion notes have a `Breaking:` line and a `migration` block
      for each constructor that changed.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- For each constructor that changes, a test shows it builds what it built
  before.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
