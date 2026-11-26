---
id: "3537"
title: "The bots' settings, the perf rows and five other constructors take no two neighbouring numbers"
type: infra
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["3503"]
nick_input: none
completed: 2026-11-26
---

# 3537 — The bots' settings, the perf rows and five other constructors take no two neighbouring numbers

## Context

3503 made the rule of
[ADR-0151](../../docs/adr/0151-constructors-take-no-two-neighbouring-numbers.md):
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
- Each constructor above changes shape as ADR-0151 says: a named
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

1. Read ADR-0151, then the entries "ticket 3537 changes it" in
   `tools/constructors.ron`; run `cargo xtask api` to see the check pass
   before any change.
2. Change the constructors crate by crate, with their call sites, and
   take each entry out of the file.
3. Write the `Breaking:` lines and the `migration` blocks. Run `cargo
   xtask api` and the gates.

## Acceptance criteria

- [x] No entry of `tools/constructors.ron` says "ticket 3537 changes it".
- [x] `cargo xtask api` passes on the branch.
- [x] For each constructor that changes, a test shows it builds what it
      built before.
- [x] The completion notes have a `Breaking:` line and a `migration` block
      for each constructor that changed.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- For each constructor that changes, a test shows it builds what it built
  before.

## Completion notes

**What was done.**

- Ten of the 19 constructors the file listed "ticket 3537 changes it"
  changed shape as ADR-0151 says, and their entries left
  `tools/constructors.ron`. Each kept `const fn` where it was.
  - **A `DEFAULT` and `with_` methods** where no field must be chosen:
    `PlanSettings::DEFAULT` (a beam 1 wide, 1 candidate, 1 command deep,
    1 copy, temperature 0), `threat::Strike::DEFAULT` (all 0, as its
    `Default`), `PageLimits::DEFAULT` (256 KiB and 1 MiB, as its
    `Default`, which now returns it) and `pcm::Limits::DEFAULT` (the
    `DEFAULT_MAX_*` constants, likewise).
  - **The field that must be chosen, and `with_` methods for the rest:**
    `RolloutSettings::new(horizon)` (1 candidate, 1 copy, 100 commands,
    temperature 0); `BalanceSettings::new(tries)` (quick runs of a fifth
    of the tries, at least 1, and 100 evaluations: what the `autobalance`
    tool already defaulted to); `Tries::new(count)` (seed 0, turn 200:
    the `playtest` tool's `DEFAULT_TRIES`); `Grid::top_down(origin,
    tile)` (corner `(0, 0)`, 1 × 1 tiles, zoom 1); `SfxDef::new(wave, hz)`
    (no length, full volume); `PackSpan::new(folder, start)` (0 bytes).
- The `Replace` of `tools/sound-parity/footprint.ron` that took
  `Limits::new`'s entry out of `tools/constructors.ron` is gone with the
  entry.
- Call sites: the engine's crates and tests, `examples/hello-grid`'s
  tests, and the skill pack's `shuyi-balance-bots` (its example `bots.rs`
  and `SKILL.md`, which also named `Strike::new` and `PlanSettings`). No
  template, tutorial step or site snippet calls any of them (searched
  `templates/`, `evals/`, `site/`).
- Tests that each builds what it built before: the existing literal
  tests of `PlanSettings`, `RolloutSettings`, `BalanceSettings` and
  `Tries` now build through the new shape, with the defaults checked;
  `a_strike_is_built_from_its_default` (`crates/core/tests`); and one each
  for `Grid`, `SfxDef`, `PageLimits`, `pcm::Limits` and `PackSpan`.

**Deviations.**

- Split at the ticket's **Split:** point: with all 19 changed the PR
  counted 1,368 lines, so the perf rows (`HeapCounts`, `HeapDiff`,
  `HeapSnapshot`, `SpanEvent`, `TagDelta`, `TagRow`, `PerfRead::of`,
  `Counts`, `TagCounts`) went to 3539, with the shapes this ticket had
  given them, and stay in `tools/constructors.ron` as "ticket 3539
  changes it". So the first acceptance criterion holds for this ticket's
  id, and 3539 carries the rest.

**Follow-up tickets.** 3539 (the perf rows).

**For the game's pin move.** It calls `Tries::new` in
`crates/tools/src/skirmish.rs` and `crates/tools/src/game_tools.rs`, and
its copy of the skill pack calls `RolloutSettings::new` and names
`Strike::new` (`gh search code --repo Zafnok/visions-of-shuyi`,
2026-11-26); the skill pack's updater brings the new copy. The same
search for `PlanSettings::new`, `BalanceSettings::new`, `top_down`,
`SfxDef::new`, `PageLimits`, `PackSpan` and `pcm::Limits` finds no other
call (`top_down` only in the stage's module name).

Sound parity: none, `decoders.rs` only builds its two test limits through `Limits::DEFAULT`; no gate, budget or reading changed

Book: none, no chapter of `site/content/learn/` or snippet of `site/snippets/` builds any of these constructors (searched)

Breaking: shuyi-core: `bots::plan::PlanSettings::new`: removed; start from `PlanSettings::DEFAULT` and use `with_width`, `with_candidates`, `with_depth`, `with_samples` and `with_temperature`

```migration
since: 0.1.0
what: A planner's settings name each number, from PlanSettings::DEFAULT.
old: PlanSettings::new(u32, u32, u32, u32, u32)
new: PlanSettings::DEFAULT.with_width(u32).with_candidates(u32).with_depth(u32).with_samples(u32).with_temperature(u32)
before: |
  const PLAN: PlanSettings = PlanSettings::new(4, 3, 2, 5, 10);
after: |
  const PLAN: PlanSettings = PlanSettings::DEFAULT
      .with_width(4)
      .with_candidates(3)
      .with_depth(2)
      .with_samples(5)
      .with_temperature(10);
find: ["PlanSettings::new("]
```

Breaking: shuyi-core: `bots::rollout::RolloutSettings::new`: takes only the horizon (1 candidate, 1 copy, 100 commands, temperature 0); set the rest with `with_candidates`, `with_samples`, `with_commands` and `with_temperature`

```migration
since: 0.1.0
what: A rollout bot's settings take the horizon, and name each number.
old: RolloutSettings::new(u32, u32, Horizon, u32, u32)
new: RolloutSettings::new(Horizon).with_candidates(u32).with_samples(u32).with_commands(u32).with_temperature(u32)
before: |
  let settings = RolloutSettings::new(8, 4, Horizon::Turns(3), 12, 0);
after: |
  let settings = RolloutSettings::new(Horizon::Turns(3))
      .with_candidates(8)
      .with_samples(4)
      .with_commands(12);
find: ["RolloutSettings::new(8, 4,"]
```

Breaking: shuyi-core: `bots::balance::BalanceSettings::new`: takes only the tries (quick runs of a fifth of them, at least 1, and 100 evaluations); set the rest with `with_quick` and `with_evaluations`

```migration
since: 0.1.0
what: The balance search's settings take the tries, and name the two numbers.
old: BalanceSettings::new(Tries, u32, u32)
new: BalanceSettings::new(Tries).with_quick(u32).with_evaluations(u32)
before: |
  let settings = BalanceSettings::new(tries, 10, 40);
after: |
  let settings = BalanceSettings::new(tries).with_quick(10).with_evaluations(40);
find: ["BalanceSettings::new(tries, 10, 40)"]
```

Breaking: shuyi-core: `bots::playtest::Tries::new`: takes only the count (from seed 0, stopped at turn 200); set the rest with `with_seed` and `with_turns`

```migration
since: 0.1.0
what: A playtest's tries take their count, and name the seed and the turn.
old: Tries::new(u32, u64, u32)
new: Tries::new(u32).with_seed(u64).with_turns(u32)
before: |
  let tries = Tries::new(50, 7, 120);
after: |
  let tries = Tries::new(50).with_seed(7).with_turns(120);
find: ["Tries::new(50, 7, 120)"]
```

Breaking: shuyi-core: `threat::Strike::new`: removed; start from `Strike::DEFAULT` and use `with_expected`, `with_all_hit` and `with_struck_back`

```migration
since: 0.1.0
what: A strike's forecast names each number, from Strike::DEFAULT.
old: Strike::new(i64, i64, i64)
new: Strike::DEFAULT.with_expected(i64).with_all_hit(i64).with_struck_back(i64)
before: |
  Some(Strike::new(450, 6, 120))
after: |
  Some(Strike::DEFAULT.with_expected(450).with_all_hit(6).with_struck_back(120))
find: ["Strike::new("]
```

Breaking: shuyi-ui: `grid::layout::Grid::top_down`: takes the origin and the tile's size (corner `(0, 0)`, one tile, zoom 1); set the rest with `with_corner`, `with_tiles` and `with_scale`

```migration
since: 0.1.0
what: A top-down grid takes its origin and tile, and names its corner, tiles and zoom.
old: Grid::top_down(Pos, (i32, i32), (i32, i32), (i32, i32), i32)
new: Grid::top_down(Pos, (i32, i32)).with_corner((i32, i32)).with_tiles((i32, i32)).with_scale(i32)
before: |
  let grid = Grid::top_down(origin, (8, 32), (16, 16), (10, 8), 2);
after: |
  let grid = Grid::top_down(origin, (16, 16))
      .with_corner((8, 32))
      .with_tiles((10, 8))
      .with_scale(2);
find: ["Grid::top_down(origin, (8, 32),"]
```

Breaking: shuyi-assets: `sfx::SfxDef::new`: takes the wave and its pitch (no length, full volume); set the rest with `with_hold_ms`, `with_fade_ms` and `with_volume`

```migration
since: 0.1.0
what: A sound effect takes its wave and pitch, and names its lengths and volume.
old: SfxDef::new(Wave, u32, u32, u32, u8)
new: SfxDef::new(Wave, u32).with_hold_ms(u32).with_fade_ms(u32).with_volume(u8)
before: |
  let def = SfxDef::new(Wave::Sine, 440, 100, 50, 80);
after: |
  let def = SfxDef::new(Wave::Sine, 440)
      .with_hold_ms(100)
      .with_fade_ms(50)
      .with_volume(80);
find: ["SfxDef::new(Wave::Sine, 440, 100"]
```

Breaking: shuyi-app: `page::PageLimits::new`: removed; start from `PageLimits::DEFAULT` and use `with_per_file` and `with_in_all`

```migration
since: 0.1.0
what: The page's file limits name each number, from PageLimits::DEFAULT.
old: PageLimits::new(u64, u64)
new: PageLimits::DEFAULT.with_per_file(u64).with_in_all(u64)
before: |
  let limits = PageLimits::new(64 * 1024, 512 * 1024);
after: |
  let limits = PageLimits::DEFAULT.with_per_file(64 * 1024).with_in_all(512 * 1024);
find: ["PageLimits::new("]
```

Breaking: shuyi-cli: `contract::bake::PackSpan::new`: takes the folder and its start (0 bytes); set the length with `with_bytes`

```migration
since: 0.1.0
what: A pack's span takes its folder and start, and names its length.
old: PackSpan::new(impl Into<String>, u64, u64)
new: PackSpan::new(impl Into<String>, u64).with_bytes(u64)
before: |
  let span = PackSpan::new("title", 120, 4_096);
after: |
  let span = PackSpan::new("title", 120).with_bytes(4_096);
find: ["PackSpan::new(\"title\", 120, 4_096)"]
```

Breaking: shuyi-sound: `pcm::Limits::new`: removed; start from `Limits::DEFAULT` and use `with_max_rate` and `with_max_frames`

```migration
since: 0.1.0
what: A decoder's limits name each number, from Limits::DEFAULT.
old: Limits::new(u32, u64)
new: Limits::DEFAULT.with_max_rate(u32).with_max_frames(u64)
before: |
  let limits = Limits::new(48_000, 1 << 20);
after: |
  let limits = Limits::DEFAULT.with_max_rate(48_000).with_max_frames(1 << 20);
find: ["Limits::new(48_000, 1 << 20)"]
```
