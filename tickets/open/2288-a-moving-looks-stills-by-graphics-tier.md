---
id: "2288"
title: "A moving look's stills by graphics tier: the lowest tier it moves at, a still for each tier below, and a check of every look"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2288 — A moving look's stills by graphics tier

## Context

A request from the game.

1. **The game ticket:** game#1407, the game's moving looks on these
   stills
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1407-moving-looks-stills-from-the-engine.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** say, in a data file beside a look that
   moves by itself (a tile that loops through frames, drifting
   particles), the lowest graphics tier it moves at and what it shows
   standing on each tier below, and have a check report a tier with no
   still.
3. **The friction numbers:** the game's third engine-friction sweep
   (game#1397, 2026-11-27): `crates/content/src/motion.rs` keeps about
   105 production lines of `Motion`, `Still`, `Show`, `Drawn`,
   `Motion::drawn` and `check`, all over the engine's `GraphicsTier`
   and `GraphicsTier::plays` (`crates/assets/src/target.rs`). The game
   would keep about 10 (its looks' fields naming a `Motion`): about
   10×. `GraphicsTier`'s own doc already says a look "shows the still
   its game names for it", but the engine has no shape for that data.
   The game built it in game#1314 (its ADR-0131).
4. **The generic shape**, in `shuyi_assets::target` beside
   `GraphicsTier`:
   - `Motion { lowest: GraphicsTier, stills: BTreeMap<GraphicsTier,
     Still> }`, read from RON (`lowest` defaults to `Full`);
   - `Still { show: Show, todo: Option<String> }`, where `Show` is
     `FirstFrame` or `Nothing` (`#[non_exhaustive]`, so a later
     `Frame(n)` or a named picture is additive);
   - `Motion::drawn(tier) -> Drawn` (`Moving` or `Still(Show)`): moving
     at `lowest` and above, the still below, the first frame if a still
     is missing;
   - `check(looks: impl IntoIterator<Item = (String, &Motion)>) ->
     StillsReport { problems, todo }`: a tier below `lowest` with no
     still, or a still on a tier where the look moves, is a problem; a
     still's `todo` text is listed, never an error.
5. **Examples from games that aren't tactics:** a platformer's looping
   waterfall and a farming game's swaying wheat, which hold their first
   frame on a handheld target and drop a falling-leaves effect
   altogether.

## Nick input

None.

## Scope

**In:**
- The types, `drawn`, `check` and their docs, with a RON example in the
  module doc.
- `Motion` and `Still` are plain data that may grow:
  `#[non_exhaustive]` with constructors (ADR-0039, ADR-0151).
- A line in `GraphicsTier`'s doc pointing to `Motion`.

**Out (do not do):**
- Playing animations (frame clips and moving tiles): parked 2231 and
  2359. This only says what a look shows on a tier; whoever plays it
  asks `drawn`.
- Wiring `check` into a game's content checks: the game calls it with
  its own looks.
- The game's use (game#1407).

## Implementation steps

1. **Read** `crates/assets/src/target.rs` (`GraphicsTier`, `ALL`,
   `plays`) and the game's `crates/content/src/motion.rs` at the game's
   pin (its doc and tests show the RON form and the messages).
2. The types and `drawn`, with the RON form.
3. `check` and `StillsReport`, its messages naming the look, the tier
   and the lowest tier.
4. Docs.

## Acceptance criteria

- [ ] `drawn` gives `Moving` at and above `lowest`, the still below, and
      `Still(FirstFrame)` for a missing still (unit tests over every
      tier of `GraphicsTier::ALL`).
- [ ] `check` reports a missing still, a still where the look moves, and
      lists a `todo` without an error (unit tests).
- [ ] A RON round trip: `motion: (stills: { Reduced: (show: FirstFrame)
      })` reads with `lowest: Full`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `drawn`, `check`, the RON form.
- Property: for every `lowest`, `drawn` at `GraphicsTier::Full` is
  `Moving` exactly when `Full` plays `lowest`.
- Snapshot / integration: none.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
