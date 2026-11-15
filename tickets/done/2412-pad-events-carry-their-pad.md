---
id: "2412"
title: "Before the first release, every pad event carries its pad: PadDown and PadUp with the pad's id, PadRemoved, and the merge of pads moved into Game, split out of parked 2406"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
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

- [x] Every `PadDown`, `PadUp` and `PadRemoved` carries its pad's id, on
      desktop, the web and the Playdate (a test per backend's path from
      its pads to the events).
- [x] A game sees the same actions and held actions as before for any
      sequence of events from up to four pads (the property test below).
- [x] Every record and Harness script in the repository and the
      templates replays to the same hashes.
- [x] The `Breaking:` line names the variants and `kind_holding`.
- [x] 2406 no longer plans the pad ids.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

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

Done. Every pad event names its pad, and the merge of several pads into
one moved from `Pads` into `Game`, so a game sees what it saw.

- `crates/ui/src/game.rs`: `RawInputEvent::PadDown(Button, PadKind,
  PadId)`, `PadUp(Button, PadId)` and `PadRemoved(PadId)`. `from_pads`
  keeps its signature. `from_crank` and `from_crank_steps` take the
  crank's pad after its kind.
- `crates/ui/src/input/pad.rs`: `Pads::update` returns the events
  themselves: every pad's releases (a removed pad's among them), then
  each removed pad's `PadRemoved`, then every pad's presses with its
  kind. `kind_holding` is gone; `down` stays (every pad's buttons).
- `crates/ui/src/game/pads.rs` (new, private): `PadMerge`, which `Game`
  runs over a frame's events before its `InputState` hears them. A
  press is heard when the first pad presses the button, a release when
  the last lets go or is removed.
- Backends: `shuyi-app` passes its pads' ids through `from_pads`
  unchanged (gilrs's and the browser's numbers), and
  `crates/cli/web/gamepad.js` didn't change. The Playdate's one pad is
  pad 0, for its buttons, its crank and its system menu. The Harness
  and a script's `pad` and `crank` steps use pad 0
  (`script::SCRIPT_PAD`).
- Tests:
  - per pad (`pad/tests.rs`): the order, a removed pad's releases then
    its `PadRemoved`, a pad removed holding nothing, two pads pressing
    one button each, and the property test
    `events_track_what_each_pad_holds`;
  - the path from pads to events on each backend:
    `pad_changes_become_events_of_each_pad_with_its_id`
    (`game/tests.rs`, the `Pads` every backend calls),
    `events_carry_their_pads_id_and_an_unplugged_pad_is_removed`
    (`shuyi-app`, desktop and the web), and the Playdate's driver
    tests (pad 0);
  - at `Game` (`game/pads/tests.rs`): the two moved tests, a removal on
    its own, a button handed from pad to pad, and one pad's tap in one
    poll;
  - the property test `a_game_sees_what_the_old_merge_gave`, against a
    copy of the old `Pads::down` union (`OldMerge`): up to four pads of
    any kind, frame times that make held directions repeat, and every
    `FrameInput`, the device and the held actions compared;
  - `from_crank`'s pad, and the events' text
    (`a_pad_events_text_names_its_pad_and_text_without_one_is_pad_0`).

**Old records read as pad 0.** A pad event written without its pad
reads as pad 0 (`#[serde(default)]` on the pad), and a session record
stays at `FORMAT_VERSION` 1 (test
`a_record_from_before_pad_ids_reads_its_pad_events_as_pad_0`). Why, and
not a refusal: an old record's pad events were already every pad merged
into one stream, and that stream on one pad is exactly what the merge
in `Game` gives, so the record plays to the same hashes. ADR-0072 raises
the version when a field's meaning changes, and here old values keep
theirs. A refusal would throw away every record a game holds, for
nothing. A script names no pad, so its pad steps are pad 0 and every
script keeps its meaning. The repository's one record,
`examples/hello-grid/tests/sessions/stage.session.ron`, holds no pad
event and replays unchanged; no template holds a record, or a script
with a pad.

Deviations:

- **Order within each part:** by button (`Button::ALL`), then by pad
  id, not by pad id then button. The property test at `Game` found that
  pad-major order changes what a game sees: two pads' presses in one
  poll came in another order, and so did their actions. Button-major
  order is the old stream's order once merged. 2406's text says so.
- **A button passed from pad to pad in one poll stays held:** a release
  is heard only if no other pad presses that button later in the same
  frame's events. The old merge saw no change there, and the property
  test found it.
- **The kind of a press two pads make in one poll** is the lowest id's
  (the first press in the events). The old merge took the first pad of
  `connected` holding it. gilrs and the browser list pads in id order,
  so it is the same pad, and the property test lists them so.
- `PadId` stays the public `usize` it was (`input::PadId`): a plain
  number, ADR-0039 kind 1. A new type would change every backend's call
  for nothing.
- **Focus loss:** `shuyi-app`'s unfocused filter still hands
  `from_pads` no pads, as ADR-0076 §4 says, so each pad also gives its
  `PadRemoved` after its releases. Handing every pad over holding
  nothing instead changes an accepted ADR. 2406 plans that change (its
  3042 bullet, which now says an ADR amends ADR-0076 §4) and needs it
  there, for seats.
- `Pads::update` returns `Vec<RawInputEvent>`, so `from_pads` only
  calls it.
- No ADR: what a game sees doesn't change. The rule (several pads as
  one) is game ADR-0034's, and only where it is kept moved. 2406's ADR
  replaces it with seats.
- Size: 1,196 counted lines, past the 900 estimated, mostly tests (807:
  the property test at `Game` and its copy of the old merge are 276).
  It would split at the merge (`game/pads.rs` and its tests), but the
  variants without the merge change what a game sees, so the two parts
  can't land apart.

Docs: rustdoc (`RawInputEvent`, `Pads::update`, `pad.rs`'s module docs,
`PadMerge`), `docs/platform-backends.md` (`poll_input`), and 2406's
text. The skills say nothing of pad events.

For the game's pin move: `gh search code --repo Zafnok/visions-of-shuyi
'PadDown'` (and `PadUp`, `from_crank`, `from_pads`, `kind_holding`)
finds `RawInputEvent::PadDown` and `PadUp` built by hand in
`crates/playdate/src/pads.rs`, `crates/ui/src/game.rs` (its tests) and
`crates/ui/tests/it/playdate.rs`, and `RawInputEvent::from_crank` in
`crates/playdate/src/pd/entry.rs`. Each adds the pad (its Playdate's
`PAD`, 0). `kind_holding` is only in the game's ADR-0036 and ticket
0220, as history.

Breaking: shuyi-ui: `RawInputEvent::PadDown`, `RawInputEvent::PadUp`, `RawInputEvent::PadRemoved`: `PadDown` and `PadUp` carry the pad's id (`PadId`) last, and `PadRemoved(PadId)` is a new variant; a game that builds or matches them adds the pad (0 for a front end with one pad) and an arm for `PadRemoved`.

```migration
since: 0.1.0
what: Pad events carry their pad's id, and an unplugged pad gives PadRemoved.
old: RawInputEvent::PadDown(Button, PadKind), RawInputEvent::PadUp(Button)
new: RawInputEvent::PadDown(Button, PadKind, PadId), RawInputEvent::PadUp(Button, PadId), RawInputEvent::PadRemoved(PadId)
before: |
  game.frame(&[RawInputEvent::PadDown(Button::East, PadKind::Playdate)], 0.0);
  game.frame(&[RawInputEvent::PadUp(Button::East)], 0.0);
after: |
  game.frame(&[RawInputEvent::PadDown(Button::East, PadKind::Playdate, 0)], 0.0);
  game.frame(&[RawInputEvent::PadUp(Button::East, 0)], 0.0);
find: ["PadKind::Playdate)]", "PadUp(Button::East)]"]
```

Breaking: shuyi-ui: `Pads::update`, `Pads::kind_holding`: `update` returns each pad's events (`Vec<RawInputEvent>`, with each pad's id and kind) instead of `(Button, bool)` pairs merged over every pad, and `kind_holding` is gone; a game calls `RawInputEvent::from_pads`, or reads the kind from each `PadDown`.

```migration
since: 0.1.0
what: Pads::update gives each pad's events with its id and kind, and kind_holding is gone.
old: Pads::update(&mut self, &[(PadId, PadKind, PadState)]) -> Vec<(Button, bool)>, Pads::kind_holding
new: Pads::update(&mut self, &[(PadId, PadKind, PadState)]) -> Vec<RawInputEvent>
before: |
  for (button, pressed) in pads.update(&connected) {
      let kind = pads.kind_holding(button, &connected).unwrap_or_default();
  }
after: |
  for event in pads.update(&connected) {
      if let RawInputEvent::PadDown(button, kind, pad) = event {}
  }
find: ["kind_holding("]
```

Breaking: shuyi-ui: `RawInputEvent::from_crank`, `RawInputEvent::from_crank_steps`: each takes the pad the crank is on after its kind; a front end with one pad passes 0.

```migration
since: 0.1.0
what: The crank's events name the pad the crank is on.
old: RawInputEvent::from_crank(&mut Crank, f32, bool, PadKind)
new: RawInputEvent::from_crank(&mut Crank, f32, bool, PadKind, PadId)
before: |
  let events = RawInputEvent::from_crank(&mut crank, degrees, docked, PadKind::Playdate);
after: |
  let events = RawInputEvent::from_crank(&mut crank, degrees, docked, PadKind::Playdate, 0);
find: ["docked, PadKind::Playdate);"]
```

Behaviour: shuyi-ui: `Game` merges pad events by pad: a `PadUp` of a button another pad still holds no longer lets go of it, and a `PadDown` of a button another pad holds is no second press. Events from `from_pads` give a game what they gave; only events a test builds with two pad ids see it.

Behaviour: shuyi-app: a window losing focus gives each pad's `PadRemoved` after its releases (ADR-0076 §4 hands no pads); a game sees the same releases.

Book: none, the book's code builds no pad event (its tests press buttons through the Harness's `pad`, which is pad 0), and what a game sees of its pads doesn't change.

Nick: nothing to decide. One player, any pad, as before.
