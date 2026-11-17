---
id: "3219"
title: "Plain text fills without parsing a message: the cost 3218 added to every frame's hints"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3219 — Plain text fills without parsing a message

## Context

A request from the game.

1. **The game ticket:** game#1285, which moves the game's pin past this
   one and checks its Playdate bench is under budget again
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1285-the-playdate-bench-under-budget-after-the-text-fill-fix.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** filling a screen's text (a help bar, a
   label) costs what it cost before messages when the text has no
   plural, ordinal or select in it.
3. **The friction numbers:** no friction review: a performance
   regression. The game's pin move across engine PRs #460–#479
   (game PR #732, 2026-11-17) made every frame of its `cargo shuyi
   bench` scenarios dearer on every target: an idle battle frame from
   1,882,790 to 1,962,514 instructions on the Playdate (+80k, +4%) and
   from 1,159,227 to 1,248,598 on desktop and web; the Playdate's cursor
   sweep from 1,982,686 to 2,050,631, over its 2,000,000 budget. Most
   of it is #476 (3218): `text::fill_styled_with` now runs
   `Message::parse` on every text it fills, before the plain path it
   took before, and `Ctx::fill_styled` copies its `args` into a new
   `Vec` with `device` and `target` on every call. A game builds many
   such texts each frame (its help bars, labels and panels). Measured on
   the desktop, release build: `Message::parse` of a 110-character help
   line takes about 200 ns of the 1.5 µs `fill` takes.
4. **The generic shape:** `shuyi_ui::text::fill_styled_with` (and so
   `fill`, `fill_values`, `fill_styled` and `Ctx`'s text) does no
   message work for a text that has none: a fast check of the text
   (no `,` inside a pair of braces, no `'` next to a brace or another
   `'`) picks the old plain path, which fills each `{name}` as before.
   `Ctx::fill_styled` hands `device` and `target` to the fill without
   allocating a new list per call (a lookup that tries `args` and then
   the two built-ins, say). Same output for every text, byte for byte.
5. **An example from a game that isn't tactics:** a platformer's pause
   menu and its button hints, filled each frame with the player's keys,
   pay the same cost on a handheld target.

## Nick input

None.

## Scope

**In:**
- The fast path above in `crates/ui/src/text.rs`, with a test that it
  and the message path give the same `StyledText` for every text of the
  toy's and Hello, Grid's languages and of the text tests.
- `Ctx::fill_styled` without the per-call `Vec` of `args`.
- A number: the instructions per fill before and after, by the engine's
  own bench or a micro-benchmark, in the PR.

**Out (do not do):**
- Caching parsed messages per key (a later ticket if the fast path
  isn't enough).
- Any change in what a text fills to.

No API break: a `Behaviour:` line only if an output changes, and none
should. The game's PR that moves the pin follows this one (game#1285).

Estimate: about 60 production lines.

## Implementation steps

1. Read `text::fill_styled_with`, `split_tokens` and `Message::parse`.
2. Add the check and route plain text to the old path; keep messages on
   `Message::parse`.
3. Drop the `Vec` in `Ctx::fill_styled`.
4. Tests, the number, the gates.

## Acceptance criteria

- A text with no message syntax is never given to `Message::parse` (a
  test with a counting wrapper or the path's own test hook).
- Every existing text test passes unchanged.
- The PR states the instructions per fill before and after.

## Tests required

The equivalence test above; a property test over random texts of
braces, commas, apostrophes and words that the fast path and the
message path agree whenever the check picks the fast one.

## Completion notes

*(Filled in by the session that completes the ticket.)*
