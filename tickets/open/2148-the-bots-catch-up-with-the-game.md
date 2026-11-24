---
id: "2148"
title: "The bots catch up with the game: one strike per piece first, steps kept out of the rest, bands by stage kind, a 95% interval and a gap between two entrants"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2148 — The bots catch up with the game

## Context

A request from the game, and several tickets' engine parts in one (Nick,
2026-11-24: "lets include 0517 and 0523 and open tickets into one which
improves engine bots").

1. **The game tickets.** The main one is game#1180, which moves the game's
   persona bots onto 2129–2132
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1180-the-games-playtesting-on-the-engines-playtest-command-targets-weights-threat-areas-and-order.md`).
   The others are game#0532 (a 95% interval on the band report, whose
   engine steps move here) and game#1276 (the grinder margin, whose
   generic "gap between two entrants' rates" its step 6 names as an
   engine candidate). The order rules come from game tickets that are
   done: game#0517 (no wandering once no foe is left) and game#0523
   (a piece's other targets after every piece's moves). The band
   overlays come from game#0510 (skirmish bands) and game#0851 (hold
   bands, and no median check on a hold). None of them is a blocker.
2. **The job in one sentence:** the engine's candidate order, targets
   and checks do what the game's bots now do, so that the game can
   delete its own copies and keep the bots playing exactly as today.
3. **Friction numbers.** There was no friction review: these are missing
   features found while starting game#1180. That ticket must keep seeded
   command traces equal, and `TacticsOrder` was built (2132,
   2026-10-24) from the game's order as it stood then. Since then the
   game's `order` (`crates/bots/src/persona.rs:523-630`, game `86257cd7`,
   engine pin `7d60a21`) gained steps, and two of them can't be said
   with today's `Kind`s:
   - **game#0523** lists each unit's best strike only (one per unit,
     `split_attacks`, `persona.rs:639-658`). The unit's best strike on
     each of its other targets comes after the advances and holds, and
     its other ways of striking come after those. `TacticsOrder`'s step
     1 lists the first per `(piece, target)`, so a piece with three
     targets fills the cap before any piece's advance. Its step 6 mixes
     the other targets and the other ways by value.
   - **game#0517**: when no foe of the player is left, the game drops
     the rest's unit actions (`wanders`, `persona.rs:619-624`) but still
     weighs each unit's advance, its hold and the AI's pick.
     `TacticsOrder` lists every unlisted `Step` in the rest, and a
     `Drop` is neither weighed as an advance nor kept as the pick.

   The rest of the game's later steps fit today's hooks. Escape and
   seize (0517, 0849) are an `Extra` group. A cover in a survive (0526)
   is another `Extra` group. A hold in a survive with no goals (0529)
   is the `distance` hook answering the walk from where the piece
   stands, and only for a piece in reach.

   The targets: game `Targets` (`crates/bots/src/playtest.rs:286-388`)
   has `skirmish_bands` and `hold_bands` over the tier's bands, and no
   median check on a hold (`checks_of`). Engine `Targets<T>` has one
   band map per tier and medians on every stage.
4. **The generic shape:**
   - **Tactics kit:**
     - a `TacticsOrder` setting that lists one strike per piece first
       and the other targets after the steps, before the other ways;
     - a command kind that is weighed as a piece's advance or hold, or
       kept as the game's pick, but never listed in the rest.
   - **Core (`bots::playtest`):**
     - targets that lay a stage kind's bands over the tier's, entrant by
       entrant, and that can leave the medians out for a kind;
     - a 95% interval on a success rate, with "maybe" verdicts;
     - a check that one entrant's success rate is at most some points
       above another's.
5. **Examples from games that aren't tactics:**
   - **Order:** in a puzzle roguelike, once a floor is cleared only the
     steps towards the stairs are worth trying: wandering steps waste
     the planner's cap.
   - **Targets:** a racing game judges its bots on time trials by other
     bands than on races, and doesn't compare two bots' lap medians on a
     time trial.
   - **Interval and gap:** any game with bots wants them, e.g. a card
     game whose bot with the best deck should win at most 15 points more
     often than its bot with the starter deck.

   The order belongs in the tactics kit, next to `TacticsOrder`
   (`crates/core/src/threat/order.rs`). The rest belongs in core
   `bots::playtest`.

## Nick input

None. Nick asked for this ticket (2026-11-24). Bands, floors and what a
miss does stay the game's data (game `docs/design/playtest-bots.md`).

## Scope

**In:**
- **A. One strike per piece first** (`crates/core/src/threat/order.rs`):
  - `TacticsOrder` gains `pub targets_after_steps: bool` (false in
    `new`, so today's order is unchanged) and
    `with_targets_after_steps(self, bool) -> Self`.
    `TacticsOrder` is `#[non_exhaustive]`, so the field is additive.
  - When it is true:
    - step 1 lists each piece's best strike only (over every target
      and way);
    - step 6 lists first each `(piece, target)`'s best strike not
      listed yet, best first, then the other ways, best first.
  - The dedupe per `(piece, target, way)` is unchanged.
- **B. Steps kept out of the rest:** a new `Kind::Spare { step:
  Option<(P, Pos)> }`. It is tried only as the hooks' `pick` or, with
  `step`, as that piece's advance or hold (steps 4 and 5, exactly as a
  `Step`). It is never in step 7. `Kind` is `#[non_exhaustive]`, so the
  variant is additive.
- **C. Bands by stage kind** (`crates/core/src/bots/playtest/targets.rs`):
  - `Targets<T>` gains `#[serde(default)] pub kinds: BTreeMap<String,
    KindTargets<T>>`.
  - `KindTargets<T>` holds:
    - `bands: BTreeMap<T, BTreeMap<String, Band>>`;
    - `medians: bool`, true when the file leaves it out.

    It is a new `#[non_exhaustive]` type with a constructor.
  - `Targets::checks_of(&self, tier: &T, kinds: &[&str], entrants:
    &[&str]) -> Vec<Check>`:
    - It starts from the tier's bands. Each kind in `kinds` order lays
      its tier's bands over them, entrant by entrant (a later kind
      wins).
    - It leaves every median out if any of `kinds` has `medians: false`.
    - A kind not in `kinds` is skipped.
  - `checks(tier, entrants)` stays, as `checks_of(tier, &[], entrants)`.
  - `unknown_entrants` also reads the kinds' bands.
  - `Targets` isn't `#[non_exhaustive]` (`targets.rs:20`), so its new
    fields (`kinds` here, `gaps` in E) are a break, with a `Breaking:`
    line.
- **D. A 95% interval** (game#0532's engine steps, as written there):
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
- **E. A gap between two entrants** (game#1276's generic part):
  - `Check::Gap { entrant, than, success: Vec<Condition>, most: u32 }`:
    `entrant`'s share of tries meeting `success` is at most `most`
    percentage points above `than`'s, compared in integers (`e_s × 100 ×
    t_n − t_s × 100 × e_n ≤ most × e_n × t_n`). It never blocks a run:
    a miss is ✗ in the report only.
  - `Found::Gap` carries both counts and a `GapVerdict { Within, Over }`.
    The table has one line per gap.
  - `Targets` gains `#[serde(default)] gaps: Vec<GapTarget>` (like
    `medians`), checked wherever both entrants play.
  - `Check` is `#[non_exhaustive]`, so `Check::Gap` is additive. A new
    `Found` variant is a break.
- **Docs:**
  - the module docs of `threat/order.rs` (the steps with the new setting
    and kind) and of `bots/playtest` and `targets.rs`;
  - a new ADR beside ADR-0044 for A and B (this repository never edits
    an accepted ADR);
  - `skills/shuyi-balance-bots/SKILL.md` (the order's new setting and
    kind, kinds of stage, the interval, the gap), with `check-skills`
    passing;
  - the book's chapter, if one covers bots.

**Out (do not do):**
- The game's use of it: game#1180 for A–C, game#0532's game steps for D,
  game#1276 for E. Each is a game PR moving the pin after this one.
- game#0536's newcomer bot: its knobs wait on the game's decision
  game#0098. Today's `PlanSettings` (`temperature`) and weights may be
  enough; that ticket judges it.
- A median *gap* (game#1276's turns margin "if 0087 has it"): it waits
  on the game's decision game#0087.
- game#1095 (units hostile to the player, not one faction), game#0334
  (the mount through one accessor), game#0528 (`Hold` like `Guard`) and
  game#1098 (`Carve` ignored). All four are the game's hooks
  (`prepare`, `classify`) over today's `Threats`, `GoalDistance` and
  `Kind::Drop`, so there is no engine change.
- Autobalance's distance to the bands (`bots/balance.rs`): it still aims
  at the share itself (game#0532, Out).
- The `shuyi playtest` command's own flags and history beyond printing
  the new verdicts and lines.

**Estimated size:** about 1,800 lines (code 550, tests 850, docs 400).
That is over one PR's worth, kept as one ticket at Nick's request: use
the valve.

**Split:** past 1,100 lines, split off parts D and E (the interval and
the gap) into a new ticket that names game#0532 and game#1276, and move
their handoffs with them.

## Implementation steps

1. **Read** `crates/core/src/threat/order.rs` and its tests, ADR-0044,
   `crates/core/src/bots/playtest.rs` (`Band`, `RateVerdict`, `Check`,
   `Found`, `judge`, the table) and `bots/playtest/targets.rs`. Read
   the game's `crates/bots/src/persona.rs` `order` (its doc comment
   lists the eleven steps) and `crates/bots/src/playtest.rs`
   `checks_of`, at the game's `main`: that is the behaviour A–C must
   be able to reproduce.
2. **A.** Add the field and builder. In `strikes`, when it is set, key
   the step-1 firsts by piece. Split the held ones into the firsts per
   `(piece, target)` and the others, each best first, and list them in
   that order in step 6. Note it in the module docs' step list.
3. **B.** Add `Kind::Spare`:
   - In `advances_and_holds`, treat `Spare { step: Some((piece, dest)) }`
     as `Step { piece, dest }`.
   - In `steps`, never put a `Spare` in `rest`.
   - The pick filter keeps it (it is neither `End` nor `Drop`).
4. **C.** Add `KindTargets`, `Targets::kinds` and `checks_of`. Make
   `checks` call it. Extend `unknown_entrants`.
5. **D.** Do game#0532's steps 1–3 (the interval by the score test
   `(100·s − k·n)² · 10_000 ≤ 38_416 · n · k · (100 − k)` in `u128`;
   44/100 → 35–53). Then add the verdicts, the field and the table text.
6. **E.** Add `Check::Gap` (a constructor, `entrants()`), `Found::Gap`,
   `GapVerdict`, its judging in `judge`, its table line, `GapTarget` and
   `Targets::gaps` (in `checks_of`, after the medians).
7. **Docs** and the new ADR. Run `cargo xtask api` and write each
   `Breaking:` line.

## Acceptance criteria

- [ ] With `targets_after_steps` false, every existing `TacticsOrder`
      test passes unchanged.
- [ ] A fixture with two pieces, one of them with two targets and two
      ways, gives this order with the setting on: each piece's best
      strike, the pick, the extras, the advances, the holds, the second
      target's best strike, the other ways, the rest, the end.
- [ ] A `Spare` with a step can be a piece's advance or hold and the
      pick. It is never listed in the rest. A `Spare` without a step is
      listed only as the pick.
- [ ] `checks_of` overlays a kind's bands entrant by entrant (a later
      kind wins) and leaves the medians out when a kind says so.
      `checks` still gives today's checks. `unknown_entrants` names a
      misspelt entrant in a kind's bands.
- [ ] `Band::interval` gives 35–53 for 44/100, a sane interval at 0/100
      and 100/100, and `None` for 0 tries. On band 30–55 at 100 tries:
      20 → `Below`, 29 → `MaybeBelow`, 40 → `Within`, 60 → `MaybeAbove`.
      The floor is still judged on the share. Old history JSON without
      `interval` still reads.
- [ ] A gap of 10 points passes at 60/100 against 50/100, fails at
      61/100, and compares in integers across different try counts. It
      never sets `blocked`.
- [ ] Each API break has its `Breaking:` line. The new ADR is accepted.
      `shuyi-balance-bots` documents the setting, the kind, stage kinds,
      the interval and the gap, and `check-skills` passes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the order fixtures above (setting off and on; `Spare` as
  advance, hold, pick and never rest); `checks_of` overlays and medians;
  the interval and each verdict; the gap's integer comparison.
- Property: the order is a permutation of a subset of the commands
  given, never longer than `cap`, with the first `End` last when there
  is one, for either setting. The interval contains the share's own
  percent whenever there is a try.
- Snapshot / integration: the report table with an interval, a maybe
  mark and a gap line.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick. Name the game PRs
that move the pin for game#1180, game#0532 and game#1276.)*
