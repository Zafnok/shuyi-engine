---
id: "2540"
title: "frame-png scripts wind the crank: a `--crank DEGREES` step beside `--keys`, `--pad` and `--wait`"
type: feature
milestone: MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2540 — frame-png scripts wind the crank

## Context

1. **The game ticket:** `game#1039`
   ([`tickets/done/1039-playdate-forecast-unit-info-level-up-and-rewind.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/1039-playdate-forecast-unit-info-level-up-and-rewind.md)),
   the Playdate's forecast, unit info, level-up and rewind screens.
2. **The job:** take a picture of a screen that only the Playdate's crank
   opens (the game's rewind screen, wound back a notch), with
   `shuyi frame-png`.
3. **The friction numbers** (the game's `docs/engine/friction.md`,
   2026-10-22, 1039): estimate 450, measured 776 (1.7×), under 2×; the
   signal was a workaround: outcome 1, engine feature. The engine's
   `frame_png::Script` has `--keys`, `--pad` and `--wait` steps, and the
   Harness it drives already has `crank(degrees)` (ticket 1035's notches),
   but a script can't wind it. The game added two options of its own to
   its `GameTools` (`--crank DEGREES`, `--then-pad BUTTONS`), played after
   the script's steps, marked `// engine-friction: engine#2540`.
4. **The generic shape:** a script step that winds the crank `DEGREES` in
   one frame (backwards negative), as a Playdate front end reports it:
   `Step::Crank(f32)`, played with `Harness::crank`, in order with the
   other steps.
5. **A game that isn't tactics:** any crank game (a fishing game reeling
   in a line, a music toy, a platformer whose crank turns a gear) wants a
   picture of what the crank does, from the same tool.

## Nick input

None.

## Scope

**In:**
- `--crank DEGREES` as a `frame_png::Script` step (`Step::Crank`),
  parsed by `Script::parse_step` (an error for a value that isn't a
  number), played by `Script::play` with the Harness's `crank`, in order
  with `--keys`, `--pad` and `--wait`; in `frame-png --help`.
- `Step::check` refuses a crank that isn't finite (`--crank NaN`), as
  it refuses such a wait. `contract/fake.rs`'s `FakeSession` counts one
  frame for a `Crank` step, as `Harness::crank` runs.
- `Step` is a set that will grow, so it becomes `#[non_exhaustive]`
  here if it isn't yet (ADR-0039 §6, and its *When*: the PR that next
  adds to an existing type).
- `Step` is in the `run`, `scene` and `session.step` input schemas:
  regenerate the schemas, raise `contract::VERSION` by one, and add its
  line to `crates/cli/schemas/versions.json` (`docs/contract.md`, *The
  schemas*).
- Tests: parsing, the error, and a step played in order.

**Out (do not do):**
- Anything in the game: its PR deletes its own `--crank` and
  `--then-pad` options and moves the pin after this one (game ticket
  1182, blocked by this).
- Docking or undocking the crank, or its absolute angle.

**For the game:** an API break in `shuyi-cli`, which CI's `api` job
reports: a variant added to `frame_png::Step`, and `#[non_exhaustive]`
on it. The completion notes carry its `Breaking:` line (ADR-0038 §6).
A game that matches `Step` exhaustively adds a `_` arm; the game
doesn't match on it. The game's PR that moves the pin follows this one.

## Implementation steps

1. Add `Step::Crank(f32)` and its parsing in `crates/cli/src/frame_png.rs`.
2. Play it with the Harness's `crank` in `Script::play`.
3. The help text's steps list, and tests.
4. `#[non_exhaustive]` on `Step`; regenerate the schemas and raise the
   contract's version.

## Acceptance criteria

- [ ] `shuyi frame-png out.png --pad "…" --crank -30 --pad South` winds
      the crank between the two button steps.
- [ ] `--crank far` is an error naming the value.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: parsing (a number, a negative number, not a number); a script
  with a crank step between pad steps plays them in order.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
