---
id: "1121"
title: "Parked: Analog sticks as axes and the pointer in a tick game: a dead zone, mouse buttons as bindable inputs, and the pointer in world pixels in TickInput"
type: feature
milestone: "Engine: real time"
model: reasoning
effort: high
status: blocked
blocked_by: ["1119", "1120", "2405"]
nick_input: none
completed:
---

# 1121 — Pointer and analog input

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; 2026-10-20: "unless these real-time work tickets overlap with visions of shuyi they should be parked until visions of shuyi is complete"): after Visions of Shuyi launches, not only after the MVP (`docs/decisions.md`, 2026-10-20). Unparked early only if the game needs it: its ticket 0060 says the army walks around freely (then 1119–1125 become game work), or it asks for a piece of this through its `engine-change` skill, which is then split out and unparked alone. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

ADR-0003 §8. The engine reads keys and controller buttons only. Sticks
are turned into four digital directions (`shuyi-ui::input::pad`, from
`ui/src/input/pad.rs`); 2405 (After MVP), split out of this ticket on
2026-10-20, reads the mouse and touches for menus and grid maps, in
console pixels. Free movement wants analog speed. A tick game that aims
at its world wants the pointer there, in world pixels, in its records.

Read the `input-actions` skill. Every new input stays an action the
player can rebind.

## Nick input

None. Our game doesn't use these unless a later ticket says so.

## Scope

**In:**
- `shuyi-app` reads stick axes (gilrs on native, the web pad plugin) and
  the mouse buttons other than the primary into `RawInputEvent`s; the
  mouse's place, its primary button, the wheel and touches already
  arrive through 2405's events.
- **Either order with 2402:** if 2402 is done, its `in-house-pads`
  source fills `PadState`s too, from `StandardPad`'s `i16` sticks:
  take the axes from the `PadState`s, not from gilrs alone, so both
  native sources give the same `RawInputEvent`s; if not, nothing to do
  (2402 keeps these events unchanged).
- `shuyi-ui::input` gets four things:
  - `Axis` actions (two-axis and one-axis) bound to sticks or to keys
    (keys give full deflection);
  - a dead zone setting;
  - the pointer in world pixels, with the index of the layer it was
    mapped through. It is computed from 2405's `PointerInput::at`
    (console pixels) through the topmost world layer with parallax
    factor 1 on both axes (the world itself, ADR-0003 §5) whose
    viewport (1120) holds the pointer, and through that layer's camera
    as the frame drew it (when 2361 smooths that layer, the blended
    camera, so a click lands where the player saw it). Over no such
    viewport there is no world position. **Either order with 2361:** if
    2361 is done, test a click on a smoothed layer here; if not, 2361
    does. **Either order with 3023 (parked):** if 3023 is done, the
    pointer is mapped only through the world layers of the surface it
    is over, in that surface's pixels (over no surface, no world
    position), with a test per layout; if not, nothing to do (3023 does
    it). **Either order with 3107:** if 3107 is done, the world-pointer
    mapping asks the skin's `WorldPick` hook when one is set, with a
    unit test that uses a stub hook; if not, nothing to do (3107 hooks
    it in);
  - mouse buttons as bindable inputs, so a game binds them to its
    actions (an ADR choice: new `Key`s or a pointer table in the keymap
    file), and `shuyi check --target` treats them as absent on a target
    without a pointer. The primary button comes from 2405's
    `PointerDown` and `PointerUp`, the others from the reader above.
- **The OS cursor follows the pointer bindings.**
  **Either order with 3044:** if 3044 is done, while the top screen
  declares an action bound to a mouse button, `shuyi-app` sets
  `keep_shown` on 3044's `IdleCursor` (`crates/app/src/window.rs`), so
  the cursor stays shown, and otherwise 3044's rule hides it after 2 s
  still and in fullscreen; if not, nothing to do (nothing hides the
  cursor yet, and 3044 sets `keep_shown` from these bindings).
- `TickInput` carries axes as `i16`, and the pointer in world pixels
  with the index of the layer it was mapped through (or none), so
  records replay exactly (1119). Each step takes the latest values:
  every step a frame runs sees that frame's axes and pointer. Mouse
  buttons reach a step as actions, so 1119's latch gives each press and
  release to exactly one step.
- Axes and the pointer belong to a seat (2406): a stick's axes to its
  pad's seat, and the mouse and touches to the seat holding the
  keyboard (or its first half). **Either order with 2406:** if 2406 is
  done, `TickInput` carries them per seat, and a Harness test moves two
  dots with scripted sticks on pads 0 and 1 at different deflections; if
  not, everything is seat 0 and 2406 routes them.
- The Harness scripts stick positions (2405 already scripts pointer
  moves and clicks).
- The rebinding screen (1112) lists axes.
- **Either order with 2367:** if 2367 is done, mouse buttons bound to
  actions are split between the running screens like any other action
  (its `FrameInput::split`), so an `Undeclared` overlay that declares
  one takes that click and the tick stage below doesn't see it; the
  pointer's place reaches every running screen, as keys do. A Harness
  test shows the click taken. If not, nothing to do: 2367's Out leaves
  pointer input as it is, and its split takes these actions like any
  other.
- **The crank as an axis.** **From 3002 (done):** `CrankForward` and
  `CrankBack` are `Button`s, a press and a release per detent
  (`RawInputEvent::from_crank`), and they stay; the analog crank is
  this ticket's. The crank's change in a frame also becomes the
  one-axis `Crank` input `targets/playdate.ron` lists, in integer 1/256
  turns, which `Axis` actions bind like a stick's.
  **From 3010 (done):** `shuyi-playdate` reads `getCrankChange` each
  frame (`Sdk::crank_change`, `crates/playdate/src/sdk.rs`), and
  `PadInput` (`input.rs`) turns it into whole 1/256 turns at the
  boundary, but only to make the detents
  (`RawInputEvent::from_crank_steps`). It reports no `Crank` axis: this
  ticket adds it there, from the same steps, with a fake-SDK test.

**Out (do not do):** pinch and other two-finger gestures; swipes and
on-screen touch controls (2407, parked); the pointer in menus and on
grid maps (2405); more than one screen. Devices per player, and which
player a viewport belongs to, are 2406's. The parked 3023 (ADR-0012 §5)
adds a surface id to 2405's pointer events, in that surface's
coordinates. Pointer mapping is one chain: window to console pixels
(2405) on a surface (3023), to a world layer's viewport (here).

## Implementation steps

1. Add the stick axes and the other mouse buttons in `shuyi-app`, on
   native and web.
2. Add the axis actions and the bindings format (saved bindings get a
   new `PLAYER_KEYS_VERSION`, and the old one reads through a step in
   `PlayerKeysFile`'s `Migrate::upgrade` (2110), with a fixture of the
   new version in `crates/ui/tests/saves/keybindings/` beside the old
   ones; if 2110 isn't done, through the hand-written version range in
   `PlayerKeys::from_ron`, and 2110 moves it).
3. Add the pointer in world pixels and in `TickInput`, the cursor hook,
   the Harness's stick scripts and the tests.

## Acceptance criteria

- [ ] A Harness test moves a dot with a scripted stick at half deflection
      and gets half speed.
- [ ] A Harness test aims with the pointer in a tick stage: the world
      pixel in `TickInput` follows the camera (1120).
- [ ] Saved bindings from before load unchanged (fixture).
- [ ] With two layers side by side from `split_viewports(frame, 2)`
      (1120), a Harness pointer in the right half reports the right
      layer's index and the world pixel its camera shows there.
- [ ] `check-keys` passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: dead zone, axis from keys, pointer mapping, also with two
  viewports side by side and an inset over one of them (the inset wins
  where they overlap; outside every viewport there is no world
  position).
- Integration: the Harness tests above.

## Completion notes
