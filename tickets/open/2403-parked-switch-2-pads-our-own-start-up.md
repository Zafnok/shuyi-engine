---
id: "2403"
title: "Parked: Switch 2 pads over USB: our own start-up command in shuyi-pads, then the pad read through SDL3 or by the crate"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2402"]
nick_input: sign-off
completed:
---

# 2403 — Switch 2 pads over USB: our own start-up

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

**From 2903 (done):** neither gilrs nor SDL3 read a Switch 2 Pro pad
(`057e:2069`) on Nick's machine. SDL3 reads it only when built with
libusb, which is LGPL-2.1 and so can't ship (ADR-0002 §8). Nick chose
"sdl3 with a fix for switch", and "make sure it is maintainable and not a
clone of existing stuff. maybe needs its own repo" (`docs/decisions.md`,
2026-11-24).

The fix was tried the same day
([`docs/research/2903-controllers-after-gilrs.md`](../../docs/research/2903-controllers-after-gilrs.md),
*The answer*): switchbrew.org's
[Switch 2: Pro Controller](https://switchbrew.org/wiki/Switch_2:_Pro_Controller)
page documents an "Initialize" command, 16 bytes starting `0x03 0x91`,
sent over USB interface 1's bulk OUT endpoint. A scratch program written
from that page sent it through `nusb`, and gilrs then read all 17 buttons,
the sticks and the triggers. SDL3 still didn't list the pad with
`SDL_JOYSTICK_HIDAPI_SWITCH2=0`.

[ADR-0147](../../docs/adr/0147-desktop-pads-through-sdl3-with-our-own-start-up-for-switch-2-pads.md)
§3 says how: a small `shuyi-pads` crate. This ticket was first "Linux and
macOS through our own code, and gilrs removed"; 2903 rewrote it, and
2402 now removes gilrs.

## Nick input

**Sign-off:** play the game on Windows with the Switch 2 Pro pad by USB,
and say if a button does the wrong thing, the pad isn't seen, or it stops
working after being unplugged and plugged back.

## Scope

**In:**
- **The crate** `crates/pads` (`shuyi-pads`), MIT OR Apache-2.0, no
  `shuyi-*` dependency, `unsafe_code = "forbid"` (ADR-0147 §3), a line in
  `tools/layers.ron`:
  - `nusb` (Apache-2.0 OR MIT; re-check on the day), default features
    off;
  - `Starter::new()`, then `Starter::poll(&mut self)` each frame: it
    watches USB for Switch 2 pads (`057e:2069`, the ids a table holds so
    more can join), and sends each new one the start-up command once,
    keeping the claimed interface open while the pad stays plugged in;
  - a pad it can't open (another program holds it, or Linux's
    permissions) is skipped with one `log::warn!(target: "input", …)`
    line;
  - written from switchbrew's page and from what the pad does; the
    command bytes are a named constant with the page's name and date
    beside it.
- **Read the started pad**, the first of these that works on each of
  Windows, Linux and macOS:
  1. through SDL3 as it stands (2402's source), perhaps with a hint
     (`SDL_HINT_JOYSTICK_HIDAPI_SWITCH2` off, or another SDL documents);
  2. if SDL won't list it there: `shuyi-pads` reads the pad's HID input
     itself and the engine's source adds it as one more pad. Its layout
     comes from the pad's own HID report descriptor, read by our parser
     written from the USB HID 1.11 specification (usb.org), or from
     the operating system's documented HID calls. If that needs `unsafe`
     or a crate beyond `nusb`, the crate takes ADR-0021 §6's rules and
     the ticket says why in an ADR line.
  Note in the completion notes which path each platform took.
- **The sticks:** switchbrew says the vertical axes are inverted, and
  the probe saw them reach about 0.85; flip them, and scale by the range
  a test pad reaches, noted in the completion notes.
- **The engine side:** `shuyi-app`'s native source runs a `Starter`
  beside SDL. The pad is `PadKind::Nintendo` (its vendor id), so the
  Switch-style swap holds.

**Estimated size:** about 1,400 lines (code 700, tests 500, docs 200).
**Split:** past 900 lines, split off the crate's own HID reader (the
second way of reading, with its descriptor parser) into a new ticket.

**Out (do not do):**
- Rumble, lights, the gyro and the pad's other commands: switchbrew
  marks them unknown. A ticket when a document describes them.
- Bluetooth; the Joy-Con 2 and other Switch 2 pads beyond the table's
  first entry (a ticket each, when a player has one to test).
- libusb, and reading SDL's, gilrs's or any driver's source for this
  (ADR-0021 §2).

## Implementation steps

1. `crates/pads` with `src/lib.rs`, `src/start.rs` (the command and the
   table), `src/watch.rs` (hot-plugging through `nusb`).
2. Its tests (below), and CI building it on Windows, Linux and macOS.
3. The engine side in `crates/app/src/pads/native.rs`.
4. Find the reading path per platform, as the Scope says; build the
   crate's own reader only if one needs it.
5. Play on Windows with the Switch 2 Pro pad; on Linux (the Steam Deck)
   if one is at hand.
6. Completion notes: the documents used, the path each platform took,
   the pads tried, what Nick said.

## Acceptance criteria

- [ ] On Windows, a Switch 2 Pro pad by USB drives the game's menus with
      every button by position, and the Switch-style swap.
- [ ] Unplugged and plugged back while the game runs, it works again
      within two seconds.
- [ ] A pad the crate can't open logs one line and the game goes on.
- [ ] `cargo tree` shows no `libusb1-sys`.
- [ ] The PR body says the code was written from the documents listed in
      the completion notes.
- [ ] Nick signs off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the start-up command's bytes against the page's table; the table
  of ids; a pad seen twice is started once; a pad gone and back is
  started again.
- If the crate reads HID itself: the descriptor parser on descriptors
  written in the tests, every item kind; random descriptors and reports
  never panic.
- Manual: step 5.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
