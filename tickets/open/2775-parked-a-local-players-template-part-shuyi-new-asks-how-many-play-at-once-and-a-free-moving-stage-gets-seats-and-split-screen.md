---
id: "2775"
title: "Parked: A local-players template part: shuyi new asks how many play at once on one machine, and a free-moving stage gets seats and split screen"
type: feature
milestone: "Engine: real time"
model: implementation
effort: medium
status: blocked
blocked_by: ["2774", "2502", "2503", "1122"]
nick_input: none
completed:
---

# 2775 — A local-players template part

> **Parked** (Nick, 2026-10-24: "These will be parked since they're not needed for Visions of Shuyi"): local split screen waits until Nick unparks it. Visions of Shuyi is turn-based and plays hotseat on one shared view, so nothing here is for the game. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md)):
"add parked tickets for adding local split screen multiplayer, both
vertical and horizontal, and lets say even 3 or 4 player split screen".

ADR-0013: `shuyi new` asks the catalog's concerns (`docs/modules.ron`)
and writes a template's parts whose `when` holds (2501 made the
template data, 2502 the wizard's questions, 2503 a stage per
perspective). The catalog has no concern for how many people play at
once on one machine, so a new game can't start with split screen. 2774
is the example that shows how; this ticket turns it into a part, so an
agent starting a couch game gets seats, views and the options rows
written, not copied by hand from the example.

Kinds of game that use it: any top-down free-moving game for two to
four local players (arena, racing seen from above, co-op shooters), and
platformers through `stage-side-view` (the clause with 1127 below).

## Nick input

None. Unparking is Nick's call, as for every parked ticket.

## Scope

**In:**

- **A concern in the catalog** (`docs/modules.ron` and the catalog's
  `docs/modules.md`, ADR-0009 §2): `local-players`, "Local players",
  `alternatives: true`, with the answers:
  - `one`: one player, `by: "default"`, ready;
  - `shared-screen`: two to four players in one view, `by: "default"`,
    ready (2406 is done before this ticket, through 2774), tickets
    `["2406"]`;
  - `split-screen`: two to four players, a view each, `by: "default"`,
    ready, tickets `["2385", "2386", "2387"]`.
  `cargo xtask check-modules` passes. The wizard asks it as a question
  of its own (2502), with a `--local-players` flag beside
  `--perspective` and `--online`, `one` by default, and in
  `--questions --json`.
- **The part** `local-players-split` in the built-in template
  (`templates/shuyi-default/template.ron`, its files in
  `templates/shuyi-default/local-players/split/`), with `when`
  holding for `split-screen` on a `stage-top-down` stage (1122, 2503):
  - the stage's input read per seat (`FrameInput::seat`), seats under
    `JoinOnPress` with `join` and `leave` actions in the keymap, and a
    `split_keyboard` (WASD and the arrows);
  - the stage's layers through `SplitScreen::layers`, each camera
    following its seat's mover, a HUD line per view, and the listeners
    (2387);
  - the options screen's split layout rows (2385);
  - `GameControls::with_seats(1, 4)` and a `min_view`;
  - a Harness test in the new game: two pads join and each moves its
    own mover in its own view.
- **The part** `local-players-shared` for `shared-screen` (its files in
  `templates/shuyi-default/local-players/shared/`): seats and
  per-seat input as above, one view whose camera follows the middle of
  the seats' movers, no split, and its Harness test.
- **Either order with 1127:** if 1127 is done, the `when` of both parts
  also holds on a `stage-side-view` stage, and a test makes a two-player
  side-view game and runs its Harness test. If not, nothing to do here
  (1127 does it, its clause says so).
- **Docs**: the wizard's page and the templates' page in `docs/` (what
  the `document` skill says), and the line in the `shuyi-stage-field`
  skill's split-screen section that `shuyi new` writes it.

**Out (do not do):**

- the example (2774) and the engine pieces (2385, 2386, 2387, 2406);
- an isometric stage (1126): a follow-up when a game asks;
- grid stages: a grid game for several players at once plays on one
  board, or two through 2385's `split_cells`, which no template asks;
- online play (2551).

## Implementation steps

1. Read 2501's `template.ron` format, 2502's questions, 2503's stage
   parts, 1122's `stage-top-down` part and `examples/couch-gems`
   (2774).
2. Add the concern to the catalog.
3. Write the two parts from the example's code, with their tests.
4. The either-order part for 1127.
5. Docs and the skill line.
6. Run the gates.

## Acceptance criteria

- [ ] `shuyi new demo --perspective top-down --local-players
      split-screen --no-input` writes a game whose `cargo test` passes,
      including its two-pad Harness test.
- [ ] The same with `--local-players shared-screen` passes its own
      test.
- [ ] `--local-players one`, and no flag, write the same game as before
      this ticket (the template's own snapshot test is unchanged).
- [ ] `cargo xtask check-modules` passes.
- [ ] CI's template job passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each part's `when`.
- Snapshot / integration: the new games above built and tested in CI's
  template job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
