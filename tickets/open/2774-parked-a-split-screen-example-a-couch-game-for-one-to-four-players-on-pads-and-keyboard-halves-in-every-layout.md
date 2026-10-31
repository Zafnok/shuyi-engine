---
id: "2774"
title: "Parked: A split-screen example: a couch game for one to four players on pads and keyboard halves, in every layout, tested in CI"
type: feature
milestone: "Engine: real time"
model: reasoning
effort: high
status: blocked
blocked_by: ["2386", "2387", "2114", "2115"]
nick_input: none
completed:
---

# 2774 — A split-screen example: a couch game for one to four players

> **Parked** (Nick, 2026-10-24: "These will be parked since they're not needed for Visions of Shuyi"): local split screen waits until Nick unparks it. Visions of Shuyi is turn-based and plays hotseat on one shared view, so nothing here is for the game. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Nick, 2026-10-24 ([`docs/decisions.md`](../../docs/decisions.md)):
"add parked tickets for adding local split screen multiplayer, both
vertical and horizontal, and lets say even 3 or 4 player split screen".

ADR-0003 §1 claims a style only when its example passes CI. Split
screen is built by:

- 2406: seats, joining by a press, keyboard halves;
- 2385: the layouts (two side by side or stacked, the three-player
  shapes, four as a square or strips) and the player's pick;
- 2386: a view per seat, cameras that follow, the spare quarter,
  dividers, a flash in one view, and the check per target;
- 2387: a listener per view.

This ticket is the small game that proves them together, the reference
an agent reads when asked for a split-screen game, and the base of
2775's template part.

The game, **Couch Gems**, is made for this example only (`AGENTS.md`
rule 4): one to four players each steer a dot around a walled field
three times wider and taller than the screen, picking up gems that
appear at seeded places. A round lasts 60 seconds; the most gems wins.
Its look is drawn shapes (2336) and the engine's font; its one pickup
sound is a short WAV made for it and checked in under the engine's
licenses.

## Nick input

None. Unparking is Nick's call, as for every parked ticket.

## Scope

**In:**

- **`examples/couch-gems`**, a crate in the workspace like
  `examples/hello-grid`, built for the desktop and the web as every
  example is:
  - the rules as a `TickSim` (1119) in `Fx`: each seat's dot moves by
    its seat's input (`FrameInput::seat`, 2406) with a speed and a
    stop at the field's walls; gems placed by a named RNG stream
    (2114) from the round's seed; the round's clock an `Every` and a
    `Countdown` (2115), never `dt`. A `TickRecord` of a round replays
    to the same hash;
  - seats under `JoinOnPress` (2406) with a `join` and a `leave`
    action, up to 4, and the keymap's `split_keyboard` (WASD and the
    arrows), so two players can share one keyboard;
  - a title screen that shows who has joined, from `seat_events`, and
    starts the round;
  - the round through `SplitScreen` (2386): a view per seat in the
    player's layout, each camera following its dot, dividers, each
    player's score and the clock in the cells of their view's HUD
    rectangle, `Spare::Overview` with three players, and a flash in a
    player's view when they pick up a gem;
  - the options screen with the split layout rows (2385) for two,
    three and four players;
  - the pickup sound from the gem's place through
    `SplitScreen::listeners(SplitPan::ByScreen)` (2387);
  - `GameControls::with_seats(1, 4)` and `SplitPrefs::min_view` set,
    so `shuyi check --target` reports what fits where (four players on
    `desktop`; one on `playdate`, with a `PlayersCapped` warning).
- **Tests**, in the example's crate:
  - Harness tests (pads by `N:` prefix, 2406): one to four players
    join; each layout for two, three and four players gives the
    snapshot it should; a player leaving mid-round reflows the views
    and keeps the others' cameras; two players on one keyboard move
    their own dots;
  - a property test: any inputs for up to four seats over a round keep
    every dot inside the field and conserve the gems (picked plus left
    equals placed);
  - `frame-png` pictures at two players in both layouts, three in
    `Grid` and four in `Grid`, checked in;
  - a bot (the contract's `bots`, as `hello-grid` has one) that steers
    each seat to its nearest gem, and a `playtest` round with four bots
    that ends with every gem picked or the clock run out.
- **Docs**: the examples list in `docs/` (what the `document` skill
  says), and the `shuyi-stage-field` skill's split-screen section
  (2386) points at this example.

**Out (do not do):**

- the template part (2775);
- online play: Steam Remote Play Together streams this with no change
  (2406's Context);
- collisions between dots, and any wall but the field's edges (1122's
  kit is for a game that needs them);
- art or sounds from any other game, and a story.

## Implementation steps

1. Read 2386's `SplitScreen`, 2387's listeners, 2406's seats and
   `hello-grid` for how an example is laid out, tested and built.
2. The rules as a `TickSim`, with the property test.
3. The title, round and options screens.
4. The Harness tests, the `frame-png` pictures and the bot.
5. Docs and the skill line.
6. Run the gates.

## Acceptance criteria

- [ ] `cargo run -p couch-gems` plays with one to four pads and with two
      players on one keyboard (checked by hand once, noted in the PR).
- [ ] The Harness tests above pass, and the `frame-png` pictures match
      their checked-in files.
- [ ] A recorded four-player round replays to the same hash.
- [ ] `shuyi check --target desktop` passes; `--target playdate` passes
      with a `PlayersCapped` warning.
- [ ] The web build of the example passes in CI.
- [ ] A `playtest` round with four bots ends as above.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: a dot stops at each wall; a gem is picked once; the round ends
  at 60 seconds in steps.
- Property: dots stay in the field; gems are conserved; a replay
  equals its record.
- Snapshot / integration: the Harness tests, the `frame-png`
  pictures, and the bot's `playtest`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
