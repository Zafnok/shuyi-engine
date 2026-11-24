---
id: "3233"
title: "The pointer drags, and widgets accept drops, with a pick-up-and-place path for pads"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2409", "3230"]
nick_input: none
completed:
---

# 3233 — The pointer drags, and widgets accept drops, with a pick-up-and-place path for pads

## Context

The pointer came to turn games and the engine's screens in 2405 (done,
[ADR-0134](../../docs/adr/0134-pointing-in-turn-games-and-screens.md)),
and the mouse and touches reach it from `shuyi-app` in 2413 (done).
What a screen sees each frame is `PointerInput`
(`crates/ui/src/input/pointer.rs`): `at`, `moved`, `pressed`,
`released`, `down_at`, `over`, `clicked` and `wheel`. `Pointer::frame`
folds the frame's `RawInputEvent`s into it, looking hit areas up with
`GlyphBuffer::hit_at`. A click is a press and a release on the same
hit (`Hit::same_target`, `crates/ui/src/frame/hit.rs`).

Nothing drags. Every plan leaves it out, and none takes it:

- ADR-0134, line 81: drag and drop, gestures and several touches are
  left out, and it names 2407.
- 2407 (parked) plans swipes as buttons only (`Button::SwipeUp` and
  the rest), from a touch that starts on no control.
- 2409's Out: scroll bars, dragging and drag and drop, "no ticket plans
  them; write one when a game asks".
- 2355's Out: "Drag and drop."
- 2759 (parked) picks inventory items up with a click and drops them
  with a click, "no drag (2405 left drag out)".

`grep -li drag tickets/open` finds no UI drag ticket; the hits are
animation rigs, cameras, the RTS box select (2716) and the Outs above.

Many mouse-first 2D games need it: a card game's hand (play a card onto
a lane), a solitaire, a deck builder, a farming game's chest and
backpack, a puzzle game's pieces, and a slider's thumb or a scroll bar
in any settings screen. Godot (`_get_drag_data` and `_can_drop_data`),
Unity (`IDragHandler` and friends) and egui (`Response::dragged`) ship
it in their core. Later users here: 2759's inventory grid, 2716's box
select, and the editor's tree and graph panel (2507, 2508, all parked).

Found by the engine blind-spot audit of 2026-11-23 (finding 41).

Pointer events are already in session records: they are
`RawInputEvent`s, recorded and replayed like keys
(`crates/platform/tests/it/session.rs`). A drag is folded from them, so
it replays as itself with nothing new recorded.

## Nick input

None.

## Scope

**In:**
- A drag in `PointerInput`: a press on a hit marked draggable that then
  moves at least `DRAG_THRESHOLD_PX` console pixels while held. From
  then on the press is a drag, and its release is no click.
- A hit marked draggable or a drop target (`Hit::draggable`,
  `Hit::drop_target`). Presses on other hits behave exactly as now.
- `DragAndDrop<P>` in `shuyi_ui::input::drag`: what is carried (a
  payload `P` the screen gives), where it came from, and the drop or
  return when it ends. The screen decides what a source offers and
  what a target accepts.
- The pad's path to the same result: Confirm on a focused source picks
  it up, the cursor keys move it between drop targets, Confirm places
  it and Cancel puts it back. The pointer is never required (ADR-0134).
- The Harness: `drag(from, to)` and `drag_text(from_text, to_text)`.
- The first user: the options screen's volume bar. Dragging along it
  sets the volume, as the keys do in steps of `VOLUME_STEP`.
- A new ADR (`write-adr`) that adds drag and drop to ADR-0134's rules,
  and an `Amended by` line in ADR-0134.
- The book's pointer section and the shipped `shuyi-input` skill.

**Public types** (ADR-0039): `Hit` and `PointerInput` are already
`#[non_exhaustive]`; their new fields are additive and start empty.
`Drag` is kind 4 (plain data, `#[non_exhaustive]`, a constructor for
tests). `DragPhase` and `DragEnd` are enums that may grow
(`#[non_exhaustive]`). `DragAndDrop<P>` is built with `new`.

**For the game:** additive. The game marks no hit draggable, so its
screens behave as before. Its PR moving the pin needs no change.

**After 3230:** drag and drop reaches widgets through 3230's `Widget`
contract, as its ADR says a cross-cutting feature must: the frame's
`drag` is a field of `WidgetInput`, and a widget says it is a drag
source or a drop target by a defaulted `Widget` method (none by
default). No new trait. A screen without widgets uses
`Hit::draggable`, `Hit::drop_target` and `DragAndDrop` directly.

**Either order with 3234:** if 3234 is done, make `Slider`'s thumb
and `ScrollPanel`'s content follow the drag, each with the
pick-up-and-place path for pads (the options screen's volume row wraps
the `Slider`, so step 7 lands there); if not, nothing to do (3234 uses
the drag once this lands), and step 7 drags the options screen's
volume row here.

**Either order with 2355:** if 2355 is done, the pad's carried item
moves between drop targets by `FocusMap` (where they sit), and 2355's
reachability check also checks that every drop target can be reached
while carrying; if not, the cursor keys step through the drop targets
in the order the frame recorded them (Up or Left back, Down or Right
on, no wrap), and 2355 moves that onto `FocusMap`.

**Estimated size:** about 1,400 lines (code 600, tests 550, docs 250).

**Split:** past 1,000 lines, split off step 7 (the volume bar as the
first user) into a new ticket; the fixture screen in the tests stays
the proof.

**Out (do not do):**
- Mouse buttons other than the primary, the pointer as a game action,
  and the pointer in `TickInput`: 1121 (parked).
- Swipes and several touches at once: 2407 (parked). A swipe starts on
  no control and a drag on a draggable hit, so they never meet.
- Drag in the inventory grid: 2759 (parked) keeps its click to pick up
  and click to drop until it starts.
- The RTS box select: 2716 (parked) reads its box from the pointer
  itself.
- The editor's tree reordering and graph wires: 2507 and 2508
  (parked).
- Scroll panels, tab bars and standalone sliders: 3234.
- Dragging between screens, between windows, or files dropped from the
  operating system.
- Dragging with a held key (Shift to copy and the like): a follow-up if
  a game asks.
- Moving the pointer on the platform side: `shuyi-app` already sends a
  touch's motion as `PointerMoved` (`crates/app/src/pointer.rs`).
- Animation of a returned item flying home: the screen paints what it
  likes.

## Implementation steps

1. **Read** `crates/ui/src/input/pointer.rs`, `frame/hit.rs`,
   `frame/glyph_buffer.rs` (`hit`, `hit_cells`, `hit_at`),
   `game.rs` (where `Pointer` is kept and `forget` is called),
   `screens/options.rs` and `options/glyph.rs` (`volume_text`,
   `VOLUME_BAR`), `crates/platform/src/harness/pointer.rs`, ADR-0134,
   and 2409's options rows as they landed.
2. **Mark hits.** In `frame/hit.rs`, add to `Hit` a field saying what
   it takes (a `HitTakes` set: `drag` and `drop`, both off by default),
   with `Hit::draggable(self) -> Self` and `Hit::drop_target(self) ->
   Self`. `same_target` ignores it. Add `GlyphBuffer::push_hit(Hit)` so
   a painter records a marked hit; `hit` and `hit_cells` are unchanged.
3. **Fold a drag** in `Pointer::frame`:
   - `pub const DRAG_THRESHOLD_PX: i32 = 4;`, the larger of the x and
     y distances from `down_at`, in console pixels.
   - A press on a draggable hit that reaches the threshold while held
     begins a drag. `PointerInput` gains `drag: Option<Drag>`, with
     `Drag { source: Hit, from: (i32, i32), at: (i32, i32), phase:
     DragPhase }` and `DragPhase::{Began, Moved, Ended, Cancelled}`.
   - Each later frame while held is `Moved` (also when nothing moved).
     The release frame is `Ended`; `over` is the drop place, and
     `clicked` stays `None`.
   - A press on a draggable hit released before the threshold is a
     click, as now.
   - The pointer leaving the frame keeps the drag; a release outside
     ends it over nothing. `Pointer::forget` (the window lost focus)
     ends a drag as `DragPhase::Cancelled`.
4. **Carry and drop** in a new `crates/ui/src/input/drag.rs`, re-exported
   from `shuyi_ui::input`:
   - `DragAndDrop<P: Clone>`: idle, or carrying `P` from a source `Hit`,
     by the pointer or by the pad.
   - `update(&mut self, pointer: Option<&PointerInput>, actions:
     &[UiAction], offer: impl Fn(Hit) -> Option<P>, accepts: impl Fn(&P,
     Hit) -> bool, targets: &[Hit]) -> Option<DragEnd<P>>`, with
     `DragEnd::Dropped { payload, from, onto }` and `DragEnd::Returned {
     payload, from }`. A drop onto a hit that is no drop target, or one
     `accepts` refuses, is `Returned`.
   - `carried(&self) -> Option<(&P, (i32, i32))>`: what to paint and
     where (the pointer's place, or the focused target's centre for the
     pad). The screen paints it last and records **no hit** for it, so
     `hit_at` under the pointer still finds the target.
   - `hovered_target(&self) -> Option<(Hit, bool)>`: the target under
     the item and whether it accepts, for the screen to highlight.
   - Actions it uses are taken from the frame's actions only while it
     carries by pad, so a screen's own handling is unchanged otherwise.
5. **The pad's path** in the same type: Confirm on a focused hit that
   `offer` gives a payload for picks it up; while carrying, the cursor
   actions move between `targets` (the clause with 2355 says how);
   Confirm places, and Cancel returns. No new `UiAction`; `controls_fit`
   and the targets' fit tests are unchanged.
6. **The Harness** (`crates/platform/src/harness/pointer.rs`):
   `drag(from: (i32, i32), to: (i32, i32))` points at `from`, presses,
   moves one frame to `from` plus the threshold toward `to`, one frame
   to `to`, and releases, each a frame of `FRAME_DT`.
   `drag_text(from_text, to_text)` finds both as `click_text` does.
7. **The volume bar.** In `options/glyph.rs`, record each volume row's
   bar (its `VOLUME_BAR` cells) as a draggable hit of its own item.
   In `options.rs`, a drag on it, or a click on it, sets the level from
   the pointer's x: `(x - left) * MAX_VOLUME / width`, rounded to the
   nearest `VOLUME_STEP`, clamped to `0..=MAX_VOLUME`, with integer
   arithmetic only. It plays the sound a key's change plays, at most
   once a frame, and only when the level changed. Leaving the screen
   asks to keep a change, as the keys' changes do. (See the clause with
   3234.)
8. **ADR and docs.** Write the ADR (`write-adr`): drag only from a
   draggable hit, the threshold, a drag is no click, the dragged image
   records no hit, the screen judges the drop, the pad's path, and a
   drop enters a command only as a click would (the screen sends it).
   Add the `Amended by` line to ADR-0134. Update the pointer section of
   `site/content/learn/book/actions.md` and
   `skills/shuyi-input/SKILL.md` (the `document` skill).

## Acceptance criteria

- [ ] A press on a draggable hit, moved 3 console pixels and released
      on the same hit, gives `clicked` and no `drag`.
- [ ] Moved 4 pixels (x or y), it gives `DragPhase::Began` that frame,
      `Moved` each frame held after, and `Ended` on release with
      `clicked` `None` and `over` the hit under the release.
- [ ] A press on a hit not marked draggable never gives a `drag`,
      however far it moves; every existing pointer test passes
      unchanged.
- [ ] A press, a move past the threshold and a release in one frame
      give one `drag` of phase `Ended`.
- [ ] A drag whose pointer leaves the frame and is released there ends
      over nothing; `Pointer::forget` mid-drag gives `Cancelled`, and
      the next release is no click.
- [ ] On a fixture screen with two sources and two targets, one
      refusing the payload: a drop onto the accepting target is
      `Dropped` with that `onto`; onto the refusing one, onto a hit
      that is no target, or onto nothing, it is `Returned`.
- [ ] While carrying, `hit_at` at the pointer finds the target under
      the carried item, not the item.
- [ ] The same fixture with the pad: Confirm on a source, the cursor
      to the accepting target and Confirm gives the same `Dropped` as
      the pointer; Cancel gives `Returned`.
- [ ] Through the Harness in the toy's options screen: a drag on the
      music volume's bar to its left end sets 0, to its right end sets
      100 (`MAX_VOLUME`), and to its middle sets 50. A click on its
      middle sets 50 too. Leaving and Keep keeps it.
- [ ] A recorded session with a drag replays to the same state hash.
- [ ] `controls_fit` and every target's fit test are unchanged.
- [ ] The ADR is written and ADR-0134 names it.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Pointer::frame`'s drag cases above, in `input/pointer.rs`'s
  tests; `DragAndDrop` in `input/drag.rs`'s tests, by pointer and by
  pad; the volume's pixel-to-level rule at both ends, the middle and
  past each end.
- Property: for any sequence of pointer events, a frame never has both
  `clicked` and a `drag`, and every drag that begins ends exactly once
  (`Ended` or `Cancelled`).
- Integration: `crates/ui/tests/it/pointer.rs` for the fixture screen
  through the Harness's `drag` and `drag_text`; the options screen's
  volume bar in the toy; a session record with a drag replayed in
  `crates/platform/tests/it/session.rs`.
- Snapshot: the options screen while dragging the volume shows the bar
  as the keys' value would; no hit shows in a snapshot.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
