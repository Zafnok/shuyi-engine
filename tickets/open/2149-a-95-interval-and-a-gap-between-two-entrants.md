---
id: "2149"
title: "The playtest report gets a 95% interval with maybe verdicts, and a check of the gap between two entrants' rates"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] `Band::interval` gives 35–53 for 44/100, a sane interval at 0/100
      and 100/100, and `None` for 0 tries. On band 30–55 at 100 tries:
      20 → `Below`, 29 → `MaybeBelow`, 40 → `Within`, 60 → `MaybeAbove`.
      The floor is still judged on the share. Old history JSON without
      `interval` still reads.
- [ ] A gap of 10 points passes at 60/100 against 50/100, fails at
      61/100, and compares in integers across different try counts. It
      never sets `blocked`.
- [ ] `unknown_entrants` names a misspelt entrant in a gap.
- [ ] Each API break has its `Breaking:` line. `shuyi-balance-bots`
      documents the interval and the gap, and `check-skills` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the interval and each verdict; the gap's integer comparison;
  `checks_of` with gaps.
- Property: the interval contains the share's own percent whenever there
  is a try; `Within` exactly when the share is in the band.
- Snapshot / integration: the report table with an interval, a maybe
  mark and a gap line.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick. Name the game PRs
that move the pin for game#0532 and game#1276.)*
