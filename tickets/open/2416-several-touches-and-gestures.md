---
id: "2416"
title: "Several touches and touch gestures as pointer data: pinch, two-finger pan, long press and double tap, read by shuyi-app and scripted in the Harness"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2409"]
nick_input: none
completed:
---

# 2416 — Several touches and gestures

## Context

`shuyi-app` treats a touch as the primary mouse button
(`crates/app/src/pointer.rs`, through macroquad's touch-as-mouse), so
one finger at a time reaches a screen. 1121, 2407, 2761 and 3023 each
leave out pinch, long press and other gestures, and no ticket plans
them. The Bevy coverage audit of 2026-11-28 (`docs/ROADMAP.md`,
*Bevy's game coverage*) found the turn games that need them on
touchscreen laptops, tablets and phones' browsers: map strategy with
pinch to zoom and a two-finger pan, and puzzle and board games with a
long press for information. Bevy keeps every touch by id and reads
pinch, rotation, pan and double-tap gestures
(`crates/bevy_input/src/touch.rs`, `gestures.rs`;
`examples/input/touch_input.rs`).

Rule 8 holds: a screen maps a gesture to its own action (pinch to the
map's zoom), and the rules see only actions and commands.

## Nick input

None.

## Scope

**In:**
- **Touches:** `PointerInput` gains a fixed-size list of touches (id,
  place in console pixels, phase: began, moved, ended, cancelled). The
  first touch still drives the pointer as today, so every screen keeps
  working.
- **Gestures,** computed in integers in `shuyi-ui` from the touches over
  frames: `Pinch { centre, scale }` (the ratio of the fingers' distance
  in whole pixels to its start), `Pan { delta }` with two fingers,
  `LongPress { at }` after a time held still, and `DoubleTap { at }`.
  The thresholds (time, distance) are data in the target profile.
- **Backends:** `shuyi-app` reads raw touches on desktop touchscreens
  and on the web (the web plugin's touch events) instead of
  touch-as-mouse.
- **The Harness** scripts touches by id, so a test reads the gesture
  and the screen's response, never cells.
- **A widget path:** the grid map's camera may bind pinch and pan to its
  zoom and scroll actions, as a worked example.
- New types follow ADR-0039.

**Out (do not do):**
- swipes and on-screen controls (2407, parked);
- rotation gestures;
- mobile builds (3015, parked), which reuse this;
- touches in `TickInput` (1121, parked).

**Estimated size:** about 1,100 lines (code 500, tests 450, docs 150).

**Split:** past 900 lines, split off the gestures (the second bullet)
into a new ticket, leaving several touches here.

## Implementation steps

1. Read `crates/app/src/pointer.rs`, `crates/ui/src/input/pointer.rs`,
   ADR-0134, the web plugin's input code and 3053's target facts.
2. Read raw touches by id in `shuyi-app` (native and web).
3. Add the touch list to `PointerInput`, keeping the first touch as the
   pointer.
4. Write the gesture recogniser with its thresholds from the profile.
5. Extend the Harness; wire the grid map's example.
6. Write the docs page.

## Acceptance criteria

- [ ] Two scripted fingers moving apart from 40 to 80 pixels give
  `Pinch` with a scale of two, centred between them.
- [ ] A touch held still past the profile's time gives one `LongPress`
  and no click.
- [ ] Two taps within the profile's time and distance give `DoubleTap`;
  farther apart, two clicks.
- [ ] Every existing pointer test passes unchanged with one finger.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each gesture's thresholds and edge cases.
- Property: any one-finger script gives the same pointer events as
  today.
- Snapshot / integration: Harness runs of each gesture on the grid map.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
