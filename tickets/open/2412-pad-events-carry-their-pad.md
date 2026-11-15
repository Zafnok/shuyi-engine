---
id: "2412"
title: "Before the first release, every pad event carries its pad: PadDown and PadUp with the pad's id, PadRemoved, and the merge of pads moved into Game, split out of parked 2406"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2412 — Every pad event carries its pad, before the first release

## Context

Nick, 2026-11-14 (`docs/decisions.md`): no run of breaking versions after
the MVP, and the controller backend worked out before the release (2082
has his words). The audit found that replacing gilrs (2903, 2402, 2403)
stays internal once 2083 opens the input types. One change no attribute
softens: the pad events carry no pad.

- `RawInputEvent::PadDown(Button, PadKind)` and `PadUp(Button)`
  (`crates/ui/src/game.rs:43`).
- `Pads::update` (`crates/ui/src/input/pad.rs:410`) merges every pad's
  buttons into one set. `PadUp` comes when the last pad lets go, and
  `Pads::kind_holding` (`pad.rs:433`) finds the kind afterwards.

Local multiplayer (2406, parked) needs each event's pad. Adding the pad
to these variants' payload after the release breaks every game that
matches them, and the records that hold them
(`RawInputEvent` is `Serialize`).

2406 already specifies this part (its Scope's "Pad ids on the raw
events" and step 3). This ticket does that part alone and keeps today's
behaviour: one player, any pad drives the game. 2406 then builds seats
on it.

## Nick input

None.

## Scope

**In:**
- `RawInputEvent::PadDown(Button, PadKind, PadId)`,
  `PadUp(Button, PadId)`, and a new `PadRemoved(PadId)`. `PadId` is the
  platform's own number for a connected pad (`pad.rs`), made public as
  ADR-0039 kind 1.
- `Pads::update` reports each pad's own changes and no longer merges:
  - every pad's releases (by pad id, then in `Button::ALL` order);
  - then each removed pad's `PadRemoved`;
  - then every pad's presses.

  `kind_holding` goes. `RawInputEvent::from_pads` keeps its signature
  and passes each pad's id and kind through, so
  `crates/app/src/pads.rs` and `crates/cli/web/gamepad.js` keep what
  they send.
- `RawInputEvent::from_crank` takes the `PadId` of the pad with the
  crank; a front end with one pad passes pad 0.
- **The merge moves into `Game`:** `Game` keeps the buttons each pad
  holds. Its one `InputState` hears a press when the first pad presses
  a button, and a release when the last lets go or is removed. So a
  game sees exactly what it sees today.
- The records and Harness scripts that hold raw events read and write
  the new variants. A script's pad step with no pad means pad 0, so
  every existing script keeps its meaning. An old record's
  `PadDown`/`PadUp` without a pad reads as pad 0, or is refused with
  its problem code and a line saying how to record it again. Choose
  one, with why, in the notes.
- A `Breaking:` line in the completion notes (ADR-0038). The game's PR
  that moves the pin follows; search the game first
  (`gh search code --repo Zafnok/visions-of-shuyi 'PadDown'`, and the
  game's Playdate front end, which builds `PadDown` itself in its
  `crates/playdate/src/pads.rs`).
- 2406's text: its pad-id part points here, done.

**Out (do not do):**
- Seats, join and leave, the split keyboard, keymaps per seat, the `N:`
  prefix in scripts, and `Ctx::seats` (2406).
- Making `RawInputEvent` `#[non_exhaustive]` (2083).
- Pointer, touch and analog events (2405, 1121).

**Estimated size:** about 900 lines (code 400, tests 350, docs 150).

## Implementation steps

1. Read 2406's Scope and step 3, `pad.rs`, `game.rs`, `input/state.rs`,
   the Harness's script reader and the record formats that hold
   `RawInputEvent`.
2. The variants and `PadId`; `Pads::update` per pad; `PadRemoved`.
3. The merge in `Game`. Move `two_pads_holding_one_button_are_one_press`
   and `a_removed_pad_releases_what_it_held` (`pad/tests.rs`) to the
   level of `Game`. Delete
   `the_pad_holding_a_button_is_the_first_connected_that_does` with
   `kind_holding`, and make the property test `events_track_what_is_down`
   track each pad on its own.
4. `from_crank`'s pad; the Playdate backend.
5. Records and scripts: the default pad, and old records.
6. 2406's text; the `document` skill's places for the input events.

## Acceptance criteria

- [ ] Every `PadDown`, `PadUp` and `PadRemoved` carries its pad's id, on
      desktop, the web and the Playdate (a test per backend's path from
      its pads to the events).
- [ ] A game sees the same actions and held actions as before for any
      sequence of events from up to four pads (the property test below).
- [ ] Every record and Harness script in the repository and the
      templates replays to the same hashes.
- [ ] The `Breaking:` line names the variants and `kind_holding`.
- [ ] 2406 no longer plans the pad ids.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Pads::update`'s order (releases, removals, presses); a removed
  pad's `PadRemoved` and its releases; `from_crank`'s pad.
- Unit, at `Game`: two pads holding one button are one press; a removed
  pad releases what it held.
- Property: any sequence of pad events from up to four pads gives the
  same `InputState` as today's merge (a copy of the old `Pads::down`
  union in the test).
- The existing replays and golden hashes, unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
