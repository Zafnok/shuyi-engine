---
id: "2402"
title: "Parked: In-house controllers, part 1: the shuyi-pads crate, a standard pad, the mapping database, and Windows"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["2903"]
nick_input: sign-off
completed:
---

# 2402 — In-house controllers: the crate, the mapping database, and Windows

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It also waits for the controller research (2903), which may keep, change or drop it.

## Context

Nick, 2026-10-19 (`docs/decisions.md`): controllers go in-house, replacing
gilrs, after sound. Today `crates/app/src/pads/native.rs` reads pads with
gilrs, and hands `shuyi-ui` a list of `PadState`s (`crates/app/src/pads.rs`).
`shuyi-ui`'s `Pads` holds all the rules: sticks as directions, the
Switch-style swap, several pads as one. The web already reads the
browser's Gamepad API through our own `crates/cli/web/gamepad.js`, so only
native changes.

[ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md):
a crate of its own, `shuyi-pads` (§4), over raw bindings (§3), with
`unsafe` only in its `sys` module (§6). It's written from Microsoft Learn
(XInput: `XInputGetState`, `XInputGetCapabilities`; Windows.Gaming.Input:
`RawGameController`, `Gamepad`) and from the format of SDL's controller
mapping database as its README describes it. Never from the source of
gilrs, SDL or any other controller library (§2). SDL's
`gamecontrollerdb.txt` (Zlib, `github.com/mdqinc/SDL_GameControllerDB`)
is used **as data** (ADR-0021 §2), read by our own parser, with its notice
in `THIRD_PARTY_ASSETS.md`.

The `input-actions` skill's rules hold: the crate reports buttons, never
actions, and the engine maps them.

## Nick input

**Sign-off:** play the game on Windows with an Xbox pad and any other pad
you own (PlayStation, Switch Pro, 8BitDo) with `in-house-pads` turned on.
Say if any button does the wrong thing or a pad isn't seen.

## Scope

**In:**
- **The crate** `crates/pads` (`shuyi-pads`), MIT OR Apache-2.0, no
  `shuyi-*` dependency:
  - `StandardPad`: the buttons of the W3C Gamepad API's standard mapping
    (the face buttons by position, shoulders, triggers, select and start,
    stick presses, the d-pad, home), two sticks as `i16`, and two
    triggers as `u16`;
  - `PadInfo`: a stable id while plugged in, the name, the USB vendor and
    product ids, and `Layout` (Xbox, PlayStation, Nintendo, other), so the
    engine's `PadKind` can be filled. Seats (2406, parked) give a pad to
    a player by this id, so it must not change while the pad stays
    plugged in. A pad unplugged and plugged back may get a new one;
  - `Pads::new() -> Result<Pads, PadsError>`, `Pads::poll(&mut self) ->
    &[(PadInfo, StandardPad)]`, with pads plugged in and removed since
    the last poll picked up;
  - a `null` backend for platforms not done yet and for tests.
- **The mapping database**: `shuyi_pads::mapping`: our parser for SDL's
  mapping-line format (GUID, name, `a:b0,leftx:a0,dpup:h0.1,…`, `+a`/`-a`
  half axes, `~` inverted, `platform:`), and a mapping from a raw
  device's buttons, axes and hats to `StandardPad`. The database file is
  embedded (`include_bytes!`), with a function to add lines at run time
  (players' own pads).
- **Windows**, through the `windows` crate:
  - XInput for Xbox-style pads (four slots, polled), which needs no
    mapping;
  - Windows.Gaming.Input `RawGameController` for every other pad, its raw
    buttons, axes and switches mapped by the database, by the GUID SDL
    builds from the vendor and product ids;
  - a pad seen by both is reported once (skip XInput-capable devices in
    the raw path).
- **The engine side**: `crates/app/src/pads/native.rs` gains a
  `shuyi-pads` source behind a cargo feature `in-house-pads`, off by
  default, producing the same `PadState`s. On Linux and macOS the feature
  uses the `null` backend until 2403.
- **Rumble, either order with 2408 (parked):** if 2408 is done, the
  `in-house-pads` source gives `PadInput`'s `Motors`
  (`shuyi_platform::rumble::Motors`) the same `Source::set_motors` and
  `Source::stop_motors` as the gilrs source. The crate stays free of
  `shuyi-*`, so it gets its own call,
  `Pads::set_motors(id, strong: u16, weak: u16)`: XInput's
  `XInputSetState` for Xbox-style pads, written from Microsoft Learn.
  XInput holds a level until told otherwise, so the source lets go
  itself when `for_ms` runs out. Other pads get no rumble on Windows
  unless the documents show a cheap path (re-check on the day). If 2408
  isn't done, nothing to do (2408 adds it, by its own clause).

**Out (do not do):**
- Linux and macOS (2403), and removing gilrs (2403).
- Lights, motion sensors, touchpads, batteries: write a ticket when a
  game asks.
- Analog input as actions (1121, parked): this crate already reports sticks
  and triggers as values. **Either order with 1121:** if 1121 is done,
  the `in-house-pads` source gives the same axis `RawInputEvent`s as the
  gilrs source; if not, nothing to do (1121 reads the axes from both).
- Steam Input.

## Implementation steps

1. `crates/pads` with `src/lib.rs`, `src/standard.rs`, `src/mapping.rs`,
   `src/null.rs`, `src/windows.rs`, `src/windows/sys.rs`.
2. Take the database file at a pinned commit into
   `crates/pads/data/gamecontrollerdb.txt`, with its license text beside
   it and a `THIRD_PARTY_ASSETS.md` entry (create the file if it doesn't
   exist yet, as `AGENTS.md` rule 7 says).
3. The parser, then the mapping, tested on lines written in the tests.
4. XInput, then `RawGameController`, then the de-duplication.
5. The engine side behind `in-house-pads`, with rumble if 2408 is done.
6. CI: build `shuyi-pads` for Windows, and run its tests with the `null`
   backend everywhere.
7. Play the game on Windows with the pads at hand.
8. In the completion notes: the documents used, the pads tested, what
   Nick said.

## Acceptance criteria

- [ ] With `in-house-pads` on Windows, an Xbox pad and one non-Xbox pad
      drive the game's menus and battles as they do with gilrs.
- [ ] Plugging a pad in or out while the game runs is seen within a
      second.
- [ ] A pad with no mapping is listed with `Layout::Other` and no
      buttons, and logs its GUID once, so a player can add a line.
- [ ] `THIRD_PARTY_ASSETS.md` lists the mapping database with its
      license.
- [ ] The PR body says the code was written from the documents listed in
      the completion notes.
- [ ] If 2408 is done: an Xbox pad rumbles for a cue with
      `in-house-pads` on.
- [ ] Nick signs off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: parsing every element kind of a mapping line; half and inverted
  axes; hats; the GUID built from vendor and product ids; XInput's state
  to `StandardPad`.
- Property: any mapping line our formatter writes parses back equal.
- Fuzz-style: random lines never panic, and the whole embedded database
  parses with no errors.
- Manual: step 7.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
