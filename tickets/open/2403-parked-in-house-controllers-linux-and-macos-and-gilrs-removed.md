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

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title. It also waits for the controller research (2903), which may keep, change or drop it.

## Context

The second half of in-house controllers. 2402 made `shuyi-pads`, the
mapping database and Windows, and the `in-house-pads` feature. This ticket
adds Linux (the Steam Deck included) and macOS, then makes them the only
path and removes gilrs
([ADR-0021](../../docs/adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)
§5).

Written from the Linux kernel's documentation (the input subsystem,
`Documentation/input/gamepad.rst` and `event-codes.rst`, the `EVIOCG*`
ioctls in `linux/input.h`) and Apple's documentation for the Game
Controller framework (`GCController`, `GCExtendedGamepad`). Never from
the source of gilrs, SDL or any other controller library (ADR-0021 §2).
Bindings: `rustix` (ioctls, `inotify`) and `objc2-game-controller`.
Re-check both licenses on the day (§3).

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
  that can't be opened (permissions) is skipped with one
  `log::warn!(target: "input", …)` line, which 3041's sinks write.
  `libudev` isn't used.
- **The Steam Deck's controls** appear as a pad through Steam's virtual
  device. Check it, and note in the completion notes how it showed up.
- **macOS**: `GCController` notifications for connect and disconnect, and
  `GCExtendedGamepad`'s elements read each poll. The framework maps pads
  itself, so the database isn't used there.
- **Rumble, either order with 2408 (parked):** if 2408 is done, Linux
  carries it over through the kernel's force feedback interface, written
  from `Documentation/input/ff.rst`: upload an `FF_RUMBLE` effect with
  `EVIOCSFF`, play it by writing an `EV_FF` event, and remove it with
  `EVIOCRMFF`. A pad needs its device opened read-write for this. Opened
  read-only, it gets no rumble and one log line. So removing gilrs keeps
  rumble on Linux and the Steam Deck. macOS stays without rumble, as
  with gilrs. If 2408 isn't done, nothing here: 2408 adds the Linux
  rumble to `shuyi-pads` when it lands (its clause on in-house pads).
- **The switch**: `in-house-pads` becomes the only path:
  - remove `gilrs` from `crates/app/Cargo.toml`;
  - delete the gilrs source in `crates/app/src/pads/native.rs`;
  - remove the feature flag.
- **Check**: `cargo tree --workspace` shows none of `gilrs`, `gilrs-core`
  or `vec_map`.
- **libudev, either order with 2513:** with gilrs gone, if nothing else
  needs libudev (`cargo tree -i libudev-sys` is empty) and 2513 is
  done, remove the `lib-udev` row from doctor's Linux table
  (`crates/cli/src/contract/doctor/linux.rs`) and libudev's package
  from the `APT`, `DNF` and `PACMAN` lines of every
  `scripts/setup-linux.sh`. If 2513 isn't done, nothing here: it leaves
  libudev out when it lands (its clause).
- **For the game:** `shuyi-app`'s API is unchanged, so the game's next pin
  bump needs no code change. Say so in the PR.

**Out (do not do):**
- Lights, motion, batteries (as 2402).
- BSDs.
- iOS and Android.

## Implementation steps

1. `crates/pads/src/linux.rs`, `src/linux/sys.rs`, `src/macos.rs`,
   `src/macos/sys.rs`.
2. Linux: finding devices, reading capabilities, events, hot-plugging,
   mapping, and rumble if 2408 is done.
3. macOS: notifications and polling.
4. CI builds `shuyi-pads` for Linux and macOS, and tests the event parser
   on recorded event streams.
5. Make the switch and drop libudev if nothing needs it (scope above).
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
- [ ] If 2408 is done: a pad on Linux or the Steam Deck rumbles for a
      cue after gilrs is removed.
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
