---
id: "2149"
title: "The playtest report gets a 95% interval with maybe verdicts, and a check of the gap between two entrants' rates"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-24
---

# 2149 — A 95% interval and a gap between two entrants

## Context

**From 2148 (done):** parts D and E of 2148 (*The bots catch up with the
game*), split off at its size valve ("past 1,100 lines, split off parts
D and E"). 2148 did A–C: the `TacticsOrder` setting
`targets_after_steps`, `Kind::Spare`, and `Targets::kinds` with
`Targets::checks_of`. Nick asked for all of it in one ticket
(2026-11-24: "lets include 0517 and 0523 and open tickets into one which
improves engine bots"); the split only lands it in two PRs.

1. **The game tickets.**
   - game#0532 (*Band reports show a 95% interval and mark only sure
     misses*,
     `github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0532-band-reports-show-a-95-interval-and-mark-only-sure-misses.md`):
     its steps 1–3 are the engine's half, written there, and its own
     half waits on this ticket. On 100 tries a success rate carries about
     ±10 points of noise, while a band is 25 points wide, and the table
     marks 49/100 against a 50–75% band "✗ too hard", as it marks 15/100.
   - game#1276 (*The grinder bot and a shop-only balance check*), step
     6: "the generic gap between two entrants' rates" is this ticket's
     part B. Its turns margin stays the game's.
2. **What is there:** `shuyi_core::bots::playtest` (`crates/core/src/bots/playtest.rs`):
   `Band::verdict` compares the share alone; `RateVerdict` is `Below`,
   `Within`, `Above`; `Found` is `Rate` and `Median`, both exhaustive;
   `Check` is `#[non_exhaustive]` with `Rate` and `Median`.
   `Targets` (`bots/playtest/targets.rs`) has `success`, `bands`,
   `floors`, `medians` and `kinds` (2148), and isn't `#[non_exhaustive]`.
3. **Examples from games that aren't tactics:** any game with bots wants
   both, e.g. a card game whose bot with the best deck should win at most
   15 points more often than its bot with the starter deck.

The core is pure and builds without `std`, with
`deny(clippy::float_arithmetic)`: the interval is computed in integers.

## Nick input

None. Bands, floors and what a miss does stay the game's data (game
`docs/design/playtest-bots.md`).

## Scope

**In:**
- **A. A 95% interval** (game#0532's engine steps, as written there):
  - `Band::interval(successes, count) -> Option<(u32, u32)>`: a Wilson
    score interval in whole percents, in integers.
  - `RateVerdict` gains `MaybeBelow` and `MaybeAbove`:
    - `Below` only when the whole interval is under the band;
    - `Above` only when it is all over the band;
    - `Within` when the share is in the band;
    - otherwise the `Maybe` ones.
  - `Found::Rate` gains `#[serde(default)] interval`. The table shows
    `band 50–75% · 95% 35–53%` and a `~ maybe too hard` / `~ maybe too
    easy` mark.
  - The floor (`blocked`) is still judged on the share itself.
  - Both are breaks: `RateVerdict` and `Found` are exhaustive. Give each
    its `Breaking:` line.
- **B. A gap between two entrants** (game#1276's generic part):
  - `Check::Gap { entrant, than, success: Vec<Condition>, most: u32 }`:
    `entrant`'s share of tries meeting `success` is at most `most`
    percentage points above `than`'s, compared in integers (`e_s × 100 ×
    t_n − t_s × 100 × e_n ≤ most × e_n × t_n`). It never blocks a run:
    a miss is ✗ in the report only.
  - `Found::Gap` carries both counts and a `GapVerdict { Within, Over }`.
    The table has one line per gap.
  - `Targets` gains `#[serde(default)] gaps: Vec<GapTarget>` (like
    `medians`), checked wherever both entrants play, in `checks_of`
    after the medians. A kind's `medians: false` doesn't leave a gap
    out.
  - `Check` is `#[non_exhaustive]`, so `Check::Gap` is additive. A new
    `Found` variant is a break, and so is the new `Targets` field.
- **Docs:** the module docs of `bots/playtest` and `targets.rs`;
  `skills/shuyi-balance-bots/SKILL.md` (the interval, the maybe marks,
  the gap), with `check-skills` passing; the book's chapter, if one
  covers playtest reports.

**Out (do not do):**
- The game's use of it: game#0532's game steps for A, game#1276 for B.
  Each is a game PR moving the pin after this one.
- A median *gap* (game#1276's turns margin "if 0087 has it"): it waits
  on the game's decision game#0087.
- Autobalance's distance to the bands (`bots/balance.rs`): it still aims
  at the share itself (game#0532, Out).
- The `shuyi playtest` command's own flags and history beyond printing
  the new verdicts and lines.

**Estimated size:** about 900 lines (code 250, tests 450, docs 200).

## Implementation steps

1. **Read** `crates/core/src/bots/playtest.rs` (`Band`, `RateVerdict`,
   `Check`, `Found`, `judge`, the table), `bots/playtest/targets.rs` and
   their tests, and game#0532's steps 1–3.
2. **A.** `Band::interval`: the lowest and highest whole percent `k` in
   `0..=100` that passes the score test `(100·s − k·n)² · 10_000 ≤
   38_416 · n · k · (100 − k)` (z² = 3.8416), in `u128`. The passing `k`
   form one run, so a scan is enough. `None` for 0 tries. If no whole
   `k` passes (only with thousands of tries), the share's own percent.
   44/100 gives 35–53. Then the verdicts, the field and the table text.
3. **B.** `Check::Gap` (a constructor, `entrants()`), `Found::Gap`,
   `GapVerdict`, its judging in `judge`, its table line (and the table's
   entrant width), `GapTarget` (`#[non_exhaustive]`, with a constructor)
   and `Targets::gaps`, read by `checks_of` and `unknown_entrants`.
4. **Docs.** Run `cargo xtask api` and write each `Breaking:` line.

## Acceptance criteria

- [x] `Band::interval` gives 35–53 for 44/100, a sane interval at 0/100
      and 100/100, and `None` for 0 tries. On band 30–55 at 100 tries:
      20 → `Below`, 29 → `MaybeBelow`, 40 → `Within`, 60 → `MaybeAbove`.
      The floor is still judged on the share. Old history JSON without
      `interval` still reads.
- [x] A gap of 10 points passes at 60/100 against 50/100, fails at
      61/100, and compares in integers across different try counts. It
      never sets `blocked`.
- [x] `unknown_entrants` names a misspelt entrant in a gap.
- [x] Each API break has its `Breaking:` line. `shuyi-balance-bots`
      documents the interval and the gap, and `check-skills` passes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the interval and each verdict; the gap's integer comparison;
  `checks_of` with gaps.
- Property: the interval contains the share's own percent whenever there
  is a try; `Within` exactly when the share is in the band.
- Snapshot / integration: the report table with an interval, a maybe
  mark and a gap line.

## Completion notes

**What was done:**

- **A.** `Band::interval(successes, count)`: the lowest and highest
  whole percent passing the score test in `u128`, scanned over
  `0..=100`; `None` for 0 tries, the share's own percent (rounded down)
  when no whole percent passes, and more successes than tries count as
  all of them. `RateVerdict` gains `MaybeBelow` and `MaybeAbove`, and
  `Band::verdict` gives `Below`/`Above` only when the interval's top is
  under `min` / its bottom over `max`. `Found::Rate` gains
  `#[serde(default)] interval`. The table's rate line reads `band
  30–55% · 95% 22–38%   ~ maybe too hard`; with no try it shows no
  interval. The floor (`blocked`) is still judged on the share.
- **B.** `Check::Gap { entrant, than, success, most }` (`#[non_exhaustive]`,
  `Check::gap`, `entrants()` names both), `GapVerdict { Within, Over }`,
  `Found::Gap` with both entrants' counts, judged in `judge` by
  `s·100·t_n ≤ t_s·100·n + most·n·t_n` in `u128`. A gap never sets
  `blocked`. Its table line: `best · 61/100 succeed (starter 50/100)
  at most 10 points over ✗`. `GapTarget` (`#[non_exhaustive]`,
  `GapTarget::new`) and `Targets::gaps`: `checks_of` gives them after
  the medians, wherever both entrants play, whatever the kinds say of
  the medians; `unknown_entrants` reads both names.
- Tests (`bots/playtest/tests.rs`): the intervals (44/100, 0/100,
  100/100, 1/1, 0 tries, a million tries with no whole percent inside,
  `u32::MAX` tries), each verdict on band 30–55 and at the interval's
  own ends, the floor on a maybe, a rate kept without `interval`, the
  gap at 60 and 61 of 100, across counts 10, 100, 200, 3 and 4, with no
  try and at `u32::MAX`, `checks_of` with gaps (tiers, an absent
  entrant, a kind with `medians: false`), a misspelt entrant in a gap,
  the constructors, and the table with an interval, a maybe mark and a
  gap line. Properties: each verdict from the share and the interval,
  the interval holding the share's percent (rounded down and up) for 1
  to `u32::MAX` tries, and a gap's verdict against the comparison in
  `i64`. The CLI reads a history line kept without `interval`, and its
  `was` still works. Expected numbers were checked against the float
  Wilson formula outside the repository.
- The contract's schemas changed (`found.Rate.interval`, the new
  verdicts, `Check::Gap` and `Found::Gap` in `playtest` and
  `autobalance` answers): contract version 57.
- Docs: the module docs of `bots/playtest` (the interval, the gap, the
  example file's `gaps`) and `targets.rs`;
  `skills/shuyi-balance-bots/SKILL.md` (the interval, the maybe marks,
  the floor on the share, `Gap`, `Targets::gaps`, the JSON's new
  verdicts) and its worked example, which now reads the interval
  (`1–23%` for 1 win in 20); `docs/contract.md`'s `playtest` row; the
  book's *Testing* chapter.

**Deviations:**

- **ADR-0039 on the types this adds to.** `RateVerdict` and `Found` are
  now `#[non_exhaustive]` (§6, a set that grows: they just did), so the
  next verdict or kind of check is additive. Their variants stay
  exhaustive, so a game's tests still build `Found::Rate { .. }` by
  literal. `Targets` is `#[non_exhaustive]` with `Targets::new(success)`
  (§2/§5: data read from files, built in code by `new` and assigning),
  so its next field is additive too. Nothing outside the engine's core
  builds `Targets` today (`gh search code --repo Zafnok/visions-of-shuyi
  'Targets {'`: the game's hits are its own `Targets` type). `GapVerdict`
  stays exhaustive, like `MedianVerdict`.
- **A gap with no try on a side** is `Within`: the comparison is 0 ≤ 0.
  The ticket's `GapVerdict` has no `Unknown`; the table line shows the
  `0/0`.

Game: game#0532 (the interval) and game#1276 (the gap) each move the
pin past this change; neither has a PR yet.

Breaking: shuyi-core: `bots::playtest::RateVerdict` has two new variants, `MaybeBelow` and `MaybeAbove`, and is `#[non_exhaustive]`: a game's `match` on it adds their arms (`~ maybe too hard`, `~ maybe too easy`) and a `_` arm.

```migration
since: 0.1.0
what: RateVerdict has two new verdicts, MaybeBelow and MaybeAbove, for a share out of its band whose 95% interval still reaches it, and may grow.
old: enum RateVerdict { Below, Within, Above }
new: "#[non_exhaustive] enum RateVerdict { Below, Within, Above, MaybeBelow, MaybeAbove }"
before: |
  let mark = match verdict {
      RateVerdict::Within => "✓",
      RateVerdict::Below => "✗ too hard",
      RateVerdict::Above => "⚠ too easy",
  };
after: |
  let mark = match verdict {
      RateVerdict::Within => "✓",
      RateVerdict::Below => "✗ too hard",
      RateVerdict::Above => "⚠ too easy",
      RateVerdict::MaybeBelow => "~ maybe too hard",
      RateVerdict::MaybeAbove => "~ maybe too easy",
      _ => "?",
  };
```

Breaking: shuyi-core: `bots::playtest::Found` is `#[non_exhaustive]`, has a new variant `Gap`, and `Found::Rate` has a new field `interval`: a game that builds `Found::Rate { .. }` by literal adds `interval` (`Band::interval(successes, count)`), and a `match` on `Found` adds a `_` arm.

```migration
since: 0.1.0
what: Found::Rate carries its 95% interval, Found has a Gap variant for Check::Gap, and Found may grow.
old: "Found::Rate { successes, count, verdict, blocked }; match found { Found::Rate { .. } => .., Found::Median { .. } => .. }"
new: "Found::Rate { successes, count, verdict, blocked, interval }; match found { Found::Rate { .. } => .., _ => .. }"
before: |
  let found = Found::Rate { successes: 6, count: 50, verdict: RateVerdict::Within, blocked: false };
  match report.checks[0].found {
      Found::Rate { successes, .. } => successes,
      Found::Median { .. } => panic!("{report:?}"),
  }
after: |
  let found = Found::Rate {
      successes: 6,
      count: 50,
      verdict: RateVerdict::Within,
      blocked: false,
      interval: Band::interval(6, 50),
  };
  match report.checks[0].found {
      Found::Rate { successes, .. } => successes,
      _ => panic!("{report:?}"),
  }
find: ["blocked: false }", "Found::Median { .. } =>"]
```

Breaking: shuyi-core: `bots::playtest::Targets` is `#[non_exhaustive]` and has a new field `gaps`: a game that builds `Targets { .. }` by literal builds it with `Targets::new(success)` and assigns the rest (a game reading the file changes nothing: `#[serde(default)]`).

```migration
since: 0.1.0
what: Targets has a new field, gaps (the gaps between two entrants' shares), and is built with Targets::new so later fields don't break.
old: Targets { success, bands, floors, medians, kinds }
new: "Targets::new(success), then targets.bands = ..; and the others"
before: |
  let targets = Targets { success, bands, floors, medians, kinds: BTreeMap::new() };
after: |
  let mut targets = Targets::new(success);
  targets.bands = bands;
  targets.floors = floors;
  targets.medians = medians;
find: ["Targets { success"]
```

Behaviour: shuyi-core, shuyi-cli: a rate out of its band whose 95% interval still reaches the band is `MaybeBelow` or `MaybeAbove` (`~ maybe too hard`, `~ maybe too easy`), no longer `Below` or `Above`: 59/100 against a band from 60 was ✗ too hard and is now a maybe. The table's rate line reads `band 60–85% · 95% 31–95%   ✓` (no interval with 0 tries). `blocked`, and so whether `playtest` fails, is unchanged: the floor is judged on the share.

Behaviour: shuyi-cli: the contract is version 57: `playtest` and `autobalance` answers carry `found.Rate.interval` (`[lo, hi]`), the two new verdicts, and a plan's `Gap` checks with `found.Gap`. A history line kept before reads as before, with no interval.

Behaviour: shuyi-core: a targets file with a `gaps` field reads now (it was refused as an unknown field). `Targets::checks` gives the same checks for a file without one.

Searched the game for what this touches: `gh search code --repo
Zafnok/visions-of-shuyi 'RateVerdict'`, `'Found::Rate'` and `'Targets {'`.
`RateVerdict` is matched in `crates/bots/src/playtest/tests.rs`
(`band_check_flags_too_easy_and_too_hard`, which game#0532 updates: 29
and 60 become maybes), and the same file matches `Found` with no `_`
arm twice (`Found::Median { .. } => None` and `=> unreachable!()`):
each gains a `_` arm at the pin move, as the second migration says. `Found::Rate` is read with `..` and a `_` arm in
`crates/tools/src/playtest.rs` (`was`), which still compiles; its table
test `render_matches_the_layout` reads the engine's table and changes
with game#0532. The `Targets {` hits are the game's own `Targets` type,
not the engine's.

Book: `site/content/learn/book/testing.md` says how the interval marks a
bot; the gap stays in the `shuyi-balance-bots` skill, as no chapter
covers playtest checks.
