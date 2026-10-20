---
id: "2403"
title: "Parked: In-house controllers, part 2: Linux through evdev, macOS through the Game Controller framework, and gilrs removed"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2402"]
nick_input: sign-off
completed:
---

# 2403 — In-house controllers: Linux and macOS, and gilrs removed

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

The second half of in-house controllers. 2402 made `shuyi-pads`, the
mapping database and Windows, and the `in-house-pads` feature. This ticket
adds Linux (the Steam Deck included) and macOS, then makes them the only
path and removes gilrs
([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§4).

Written from the Linux kernel's documentation (the input subsystem,
`Documentation/input/gamepad.rst` and `event-codes.rst`, the `EVIOCG*`
ioctls in `linux/input.h`) and Apple's documentation for the Game
Controller framework (`GCController`, `GCExtendedGamepad`). Never from
the source of gilrs, SDL or any other controller library (ADR-0021 §1).
Bindings: `rustix` (ioctls, `inotify`) and `objc2-game-controller`.
Re-check both licenses on the day (§2).

## Nick input

**Sign-off:** play the game on the Steam Deck (its built-in controls, and
a pad plugged in) and on a Mac if one is at hand. Say if a button is wrong
or a pad isn't seen. If no Mac is at hand, say so, and the ticket notes
that macOS was tested only in CI.

## Scope

**In:**
- **Linux**: open `/dev/input/event*` devices that report gamepad buttons
  (`BTN_GAMEPAD`) or joystick axes, read their `input_event`s without
  blocking, and pick up hot-plugging with `inotify` on `/dev/input`.
  The kernel's gamepad mapping (`BTN_SOUTH` and friends) is used when the
  driver gives it. Otherwise the mapping database from 2402, with the GUID
  built from the bus, vendor, product and version (`EVIOCGID`). A device
  that can't be opened (permissions) is skipped with one log line.
  `libudev` isn't used.
- **The Steam Deck's controls** appear as a pad through Steam's virtual
  device. Check it, and note in the completion notes how it showed up.
- **macOS**: `GCController` notifications for connect and disconnect, and
  `GCExtendedGamepad`'s elements read each poll. The framework maps pads
  itself, so the database isn't used there.
- **The switch**: `in-house-pads` becomes the only path:
  - remove `gilrs` from `crates/app/Cargo.toml`;
  - delete the gilrs source in `crates/app/src/pads/native.rs`;
  - remove the feature flag.
- **Check**: `cargo tree --workspace` shows none of `gilrs`, `gilrs-core`
  or `vec_map`.
- **For the game:** `shuyi-app`'s API is unchanged, so the game's next pin
  bump needs no code change. Say so in the PR.

**Out (do not do):**
- Rumble, lights, motion, batteries (as 2402).
- BSDs.
- iOS and Android.

## Implementation steps

1. `crates/pads/src/linux.rs`, `src/linux/sys.rs`, `src/macos.rs`,
   `src/macos/sys.rs`.
2. Linux: finding devices, reading capabilities, events, hot-plugging,
   mapping.
3. macOS: notifications and polling.
4. CI builds `shuyi-pads` for Linux and macOS, and tests the event parser
   on recorded event streams.
5. Make the switch (scope above).
6. Build the game against this working copy (`run-gates`, *The game at the
   next pin*), then test on the Steam Deck or Linux, and on macOS if
   possible.
7. In the completion notes: the documents used, the devices tested, what
   Nick said.

## Acceptance criteria

- [ ] On Linux and macOS the game's pads work as they did with gilrs.
- [ ] On the Steam Deck the built-in controls drive the game.
- [ ] `cargo tree` shows none of the crates listed above.
- [ ] The game builds and plays at this revision with no code change.
- [ ] The PR body says the code was written from the documents listed in
      the completion notes.
- [ ] Nick signs off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: decoding `input_event` records; axis ranges (`EVIOCGABS`) to
  `i16`; hat axes to the d-pad; the GUID built from `EVIOCGID`.
- Property: a recorded stream of random events never panics and leaves
  every value in range.
- Manual: step 6.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
