# ADR-0146: Desktop pads through SDL3, with our own start-up for Switch 2 pads

- **Status:** Accepted
- **Date:** 2026-11-24
- **Related tickets:** 2903 (the research), 2402, 2403, 2414, 2408, 2008
- **Resolves:** ADR-0021 §1's controllers choice. **Narrows** ADR-0021 §4's
  `shuyi-pads` and §6's platform crates; its other decisions stand.

## Context

ADR-0021 §1 left controllers to research: gilrs leaves, and 2903 decides
whether our own code (2402, 2403 as first written) or a library replaces
it. The [research](../research/2903-controllers-after-gilrs.md) ran gilrs
0.11.2 and SDL 3.4.20 as black boxes on Nick's pads on 2026-11-24:

- SDL3 read an Xbox pad and a DualSense completely (the logo button,
  the DualSense's gyro, touchpad and light), and rumbled both;
- gilrs missed the Xbox pad's logo button and didn't list the DualSense;
- neither read a Switch 2 Pro pad (`057e:2069`). SDL3 reads it only when
  built with libusb, which is LGPL-2.1 and so not shippable here
  (ADR-0002 §8);
- switchbrew.org's protocol page for that pad documents one
  "Initialize" command, sent over USB interface 1's bulk OUT endpoint.
  Sent by a scratch program written from that page, it made the pad
  report all 17 buttons, its sticks and its triggers through gilrs.

Our own code written only from what the makers publish would reach about
what gilrs reaches on Windows: XInput has no documented logo button, and
Sony and Nintendo publish nothing about their pads' reports.

Nick (2026-11-24, `docs/decisions.md`): "either sdl3 with a fix for
switch, or write our own. if no fix for switch can exist with sdl3, then
write our own. make sure it is maintainable and not a clone of existing
stuff." The fix exists, so the first branch holds.

## Decision

### 1. SDL3 reads pads on Windows, Linux and macOS

`shuyi-app`'s native pad source (`crates/app/src/pads/native.rs`) reads
pads through SDL3's gamepad API, through `sdl3-sys` (Zlib, bindings
generated from SDL's headers), with SDL built from source and linked
statically (`build-from-source-static`):

- **Only its input parts:** the joystick, gamepad, HIDAPI, haptic and
  sensor subsystems; audio, video, render, GPU, camera, dialog and tray
  off (the `no-sdl-*` features). macroquad keeps the window, and winit
  later (3034); SDL opens none.
- **SDL's hint for background events on**
  (`SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS`), since the window isn't
  SDL's.
- **No libusb, ever** (LGPL-2.1). `cargo deny` keeps it out; a check that
  SDL's CMake found no libusb keeps it out of the C build.
- **The seam stays:** the source fills the same `PadState`s, and
  `shuyi-ui`'s `Pads` keeps every rule. `PadKind` still comes from the
  USB ids, which SDL reports.
- **The web keeps `gamepad.js`.** SDL on the web needs Emscripten.
- **gilrs leaves** in the same PR, with `libudev` from the Linux setup
  if nothing else needs it (ADR-0021 §5).

`unsafe` calls into SDL live in `shuyi-app`, which already has
ADR-0002 §9's exception, in one module under the rules of ADR-0021 §6.

### 2. Building SDL needs CMake

SDL's C source is built by CMake with the C compiler a game's machine
already has (MSYS2's on Windows). So CMake joins the prerequisites:
`shuyi doctor` checks for it and names the install line per system;
`scripts/setup-linux.sh` installs it; CI's runners have it. The C build
runs once per target directory (measured cold: 132 s on Nick's machine,
24 cores, with only the input parts), then cargo caches it. A game grows
by about 1.8 MB, measured stripped.

A system SDL (`use-pkg-config`) is not used: it would put a DLL beside
every game and tie the engine to each system's version.

### 3. Switch 2 pads: a start-up of our own, in `shuyi-pads`

ADR-0021 §4's `shuyi-pads` becomes a small crate for what SDL can't
ship: today, starting Switch 2 pads over USB.

- It finds a Switch 2 pad by its USB ids, claims interface 1 and sends
  the documented "Initialize" command over its bulk OUT endpoint, through
  `nusb` (Apache-2.0 OR MIT; Windows, Linux, macOS; no C). After that the
  pad is a plain HID gamepad.
- It then reads the pad through SDL3 if SDL takes it once started; if
  SDL won't on a platform, the crate reads that pad's HID reports itself,
  laid out by the pad's own HID report descriptor (2403 finds which).
- It depends on no `shuyi-*` crate and has no `unsafe` (`nusb` is safe),
  so it keeps `unsafe_code = "forbid"`, narrowing ADR-0021 §6. It can
  move to its own repository later (2008).
- **Nothing cloned.** It is written from switchbrew's protocol
  description and from what the pad does when tried; never from SDL's,
  gilrs's or another driver's source (ADR-0021 §2). Rumble, lights and
  the pad's other commands come only when a document describes them.

### 4. What stays out

- Our own XInput, WGI, evdev and Game Controller code (2402 and 2403 as
  first written): SDL covers them.
- libusb, and SDL's Switch 2 driver that needs it.
- Steam Input's action sets (3012's Out holds).

## Consequences

- Pads work as the tests saw: Xbox and Sony pads completely on Windows,
  and SDL's own drivers for the rest; Switch 2 pads once 2403 lands.
  Android and iOS pads come with SDL if 3015 is unparked.
- The engine carries a C library (SDL, Zlib) and needs CMake to build.
  Its releases come monthly; the engine pins `sdl3-sys` and moves on
  purpose, like wgpu and winit.
- The logo button, gyro, touchpads, lights and battery are available
  when a game asks (each a ticket then).
- Rumble (2408) goes through `SDL_RumbleGamepad` on all three systems,
  macOS included where SDL supports the pad.
- Hardware bugs in SDL's drivers are worked around in the engine, never
  reported upstream (`AGENTS.md` rule 12).
- The Steam Deck isn't tried yet: 2414 does it before 2402 lands.

## Alternatives considered

- **Keep gilrs.** Lost: on Nick's pads it didn't list the DualSense or
  read the Xbox logo button; one maintainer, and crash fixes unreleased
  since May 2026.
- **Our own code everywhere (2402, 2403 as first written).** Lost: from
  published documents alone it reaches about gilrs's coverage, for the
  most work; Nick chose it only if no Switch fix existed.
- **SDL3 with libusb**, for SDL's own Switch 2 driver. Lost: LGPL-2.1.
- **The safe `sdl3` wrapper** (MIT) over `sdl3-sys`. Lost for now: it
  brings SDL's other subsystems' API for the ten-odd gamepad calls we
  make; 2402 may take it if its gamepad API covers the needs.
- **A system SDL.** Lost: a DLL beside each game, and its version
  differs per machine.
