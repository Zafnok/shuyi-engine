---
id: "2148"
title: "The bots catch up with the game: one strike per piece first, steps kept out of the rest, and bands by stage kind"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-24
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
    field `kinds` is a break, with a `Breaking:` line.
- **D and E, moved to 2149** at the split (below): a 95% interval on a
  success rate with "maybe" verdicts (game#0532's engine steps), and
  `Check::Gap` with `Targets::gaps` (game#1276's generic part).
- **Docs:**
  - the module docs of `threat/order.rs` (the steps with the new setting
    and kind) and of `bots/playtest` and `targets.rs`;
  - a new ADR beside ADR-0044 for A and B (this repository never edits
    an accepted ADR);
  - `skills/shuyi-balance-bots/SKILL.md` (the order's new setting and
    kind, kinds of stage), with `check-skills` passing;
  - the book's chapter, if one covers bots.

**Out (do not do):**
- The game's use of it: game#1180 for A–C. It is a game PR moving the
  pin after this one.
- D and E: 2149, with game#0532 and game#1276.
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
5. **Docs** and the new ADR. Run `cargo xtask api` and write each
   `Breaking:` line.

## Acceptance criteria

- [x] With `targets_after_steps` false, every existing `TacticsOrder`
      test passes unchanged.
- [x] A fixture with two pieces, one of them with two targets and two
      ways, gives this order with the setting on: each piece's best
      strike, the pick, the extras, the advances, the holds, the second
      target's best strike, the other ways, the rest, the end.
- [x] A `Spare` with a step can be a piece's advance or hold and the
      pick. It is never listed in the rest. A `Spare` without a step is
      listed only as the pick.
- [x] `checks_of` overlays a kind's bands entrant by entrant (a later
      kind wins) and leaves the medians out when a kind says so.
      `checks` still gives today's checks. `unknown_entrants` names a
      misspelt entrant in a kind's bands.
- [x] Each API break has its `Breaking:` line. The new ADR is accepted.
      `shuyi-balance-bots` documents the setting, the kind and stage
      kinds, and `check-skills` passes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the order fixtures above (setting off and on; `Spare` as
  advance, hold, pick and never rest); `checks_of` overlays and medians.
- Property: the order is a permutation of a subset of the commands
  given, never longer than `cap`, with the first `End` last when there
  is one, for either setting.

## Completion notes

**What was done (A–C):**

- **A.** `TacticsOrder` has `targets_after_steps` (false in `new`) and
  `with_targets_after_steps`. With it on, `strikes` lists each piece's
  first strike in step 1, then in step 6 the firsts per `(piece,
  target)` not yet listed, then the other ways, each best first
  (`crates/core/src/threat/order.rs`). The module docs' step list says
  so.
- **B.** `Kind::Spare { step }`: `advances_and_holds` takes a spare with
  a step as a `Step`, the rest never takes one, and the pick filter keeps
  it.
- **C.** `KindTargets<T>` (`#[non_exhaustive]`, `KindTargets::new`,
  `medians` true by default), `Targets::kinds`, `Targets::checks_of`
  (`checks` calls it with no kinds), and `unknown_entrants` reads the
  kinds' bands.
- Tests: a toy `Table` game in `threat/order/tests.rs` tells the order
  each command's kind. It covers the acceptance fixture (two pieces, one
  with two targets and two ways, on and off, uncapped and at 4), `Spare`
  as advance, hold and pick and never in the rest, and a property over
  any kinds, either setting and any cap. The setting only reorders. The
  skirmish property runs with either setting. Every old order test
  passes unchanged. `playtest/tests.rs` covers the overlays (the last kind
  wins), medians left out or kept, an unnamed kind, a misspelt entrant in
  a kind, and `checks_of` in the targets property.
- Docs: ADR-0145 (extends ADR-0044 §2, whose status line and index row
  say so), the module docs of `threat/order.rs`, `bots/playtest` (its
  example file has a `kinds`) and `targets.rs`, and
  `skills/shuyi-balance-bots/SKILL.md` (the setting, `Spare`, kinds of
  stage). `cargo shuyi check-skills skills` passes.

**Deviations:**

- **The split.** D and E (the 95% interval and the gap) are ticket
  2149, as the ticket's `**Split:**` line says. A–C came to 602 counted
  lines before docs, and D and E were estimated at about 900 more (the
  ticket put the whole at 1,800), so the PR would have passed the
  valve's 1,100. 2149 carries D and E's scope, steps, acceptance and
  handoffs to game#0532 and game#1276. This ticket's D and E sections
  now point at it, and its title no longer names them.
- **Different from the game's own `checks_of`:** the game's skirmish
  bands *replace* the tier's (`own.or_else(..)`), and its hold bands lie
  over them only when there are no skirmish bands. `checks_of` lays
  every kind over the tier's entrant by entrant, as the ticket says. The
  game gets its own results by passing `["hold", "skirmish"]` if its
  `skirmish_bands` lists every persona its `bands` list for that tier.
  game#1180 should check that when it moves `targets.ron`.

**For the game (handoffs this PR can't write):** game#0532 is
`blocked_by: ["engine#2148"]`, and game#1276's step 6 says "if the pin is
past engine#2148". Both mean the interval and the gap, which are now
engine#2149. Both game tickets want `engine#2148` read as `engine#2149`
(a ticket edit in the game repository).

Game: game#1180 moves the pin to this change and adopts A–C. The
interval and the gap are 2149's (game#0532, game#1276).

Breaking: shuyi-core: `bots::playtest::Targets` has a new public field `kinds`: a game that builds `Targets { .. }` as a literal adds `kinds: BTreeMap::new()` (a game reading the file changes nothing: `#[serde(default)]`).

```migration
since: 0.1.0
what: Targets has a new field, kinds (each stage kind's bands and whether its medians are checked).
old: Targets { success, bands, floors, medians }
new: Targets { success, bands, floors, medians, kinds }
before: |
  let targets = Targets { success, bands, floors, medians };
after: |
  let targets = Targets { success, bands, floors, medians, kinds: BTreeMap::new() };
find: ["floors, medians }"]
```

Behaviour: shuyi-core: a targets file with a `kinds` field reads now (it was refused as an unknown field). Nothing else a game sees changes: `TacticsOrder::new` orders as before, and `Targets::checks` gives the same checks.

Book: none, no chapter covers the tactics order or playtest targets (`site/content/learn/book/testing.md` names `cargo shuyi playtest` only). The `shuyi-balance-bots` skill and ADR-0145 carry them.

Searched the game for what this touches:
`gh search code --repo Zafnok/visions-of-shuyi 'TacticsOrder'`,
`'playtest::Targets'` and `'Kind::Drop'`. The hits are its copy of the
skill pack and game#1180. The game's code builds neither, so nothing in
it breaks at the pin move.
