---
id: "2402"
title: "Parked: Desktop pads through SDL3: its input parts built from source, CMake as a prerequisite, and gilrs removed"
type: feature
milestone: "Engine: in-house platform"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2903", "2414"]
nick_input: sign-off
completed:
---

# 2402 — Desktop pads through SDL3, and gilrs removed

> **Parked** (Nick, 2026-10-19, chose "Park until MVP" for the in-house platform layer, `docs/decisions.md`): after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

**From 2903 (done):** the controller research tried gilrs 0.11.2 and SDL
3.4.20 on Nick's pads (2026-11-24,
[`docs/research/2903-controllers-after-gilrs.md`](../../docs/research/2903-controllers-after-gilrs.md)).
SDL3 read an Xbox pad and a DualSense completely; gilrs missed the Xbox
pad's logo button and didn't list the DualSense. Nick chose "sdl3 with a
fix for switch" (`docs/decisions.md`, 2026-11-24), and
[ADR-0147](../../docs/adr/0147-desktop-pads-through-sdl3-with-our-own-start-up-for-switch-2-pads.md)
says how. This ticket was first "the `shuyi-pads` crate, the mapping
database and Windows", our own code; 2903 rewrote it.

Today `crates/app/src/pads/native.rs` reads pads with gilrs and hands
`shuyi-ui` a list of `(PadId, PadKind, PadState)`. `shuyi-ui`'s `Pads`
(`crates/ui/src/input/pad.rs`) holds every rule, so only the source
changes. The web keeps its own reading (`crates/cli/web/gamepad.js`, or
`web-sys` once 3331, parked, moves it).

The Switch 2 pads' start-up is 2403, after this. The Steam Deck is tried
first by 2414, whose findings this ticket reads.

## Nick input

**Sign-off:** play the game on Windows with the Xbox and PS5 pads, and on
the Steam Deck. Say if a button does the wrong thing or a pad isn't seen.

## Scope

**In:**
- **The source:** `crates/app/src/pads/native.rs` reads pads through
  `sdl3-sys` (Zlib; re-check on the day), built from source and linked
  statically, with only the input parts (ADR-0147 §1): the
  `build-from-source-static` feature and the `no-sdl-*` features for
  audio, video, render, GPU, camera, dialog and tray. Not on `wasm32`.
  - `SDL_SetHint(SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS, "1")`, then
    `SDL_Init(SDL_INIT_GAMEPAD)`; on failure one
    `log::warn!(target: "input", …)` and no pads, as gilrs's source does.
  - Each frame: pump SDL's events and drop them, list the gamepads, open
    the new ones and close the gone ones, and fill each pad's
    `PadState` from `SDL_GetGamepadButton` and `SDL_GetGamepadAxis`.
    The triggers are axes in SDL; a trigger past half counts as held, as
    today. SDL's sticks are down-positive, as the engine's are.
  - `PadId` from SDL's joystick id. SDL gives a pad plugged back a new
    id; 2406's seats expect no more.
  - `PadKind::from_ids` from `SDL_GetGamepadVendor` and
    `SDL_GetGamepadProduct`. An Xbox pad through XInput reports
    `045e:02ff`: still Microsoft. If 3012 is done, the Deck's controls
    are `PadKind::SteamDeck` as 3012 says, read from what 2414 found.
  - Every `unsafe` call in one module, `crates/app/src/pads/sdl.rs`,
    with a `// SAFETY:` comment each (ADR-0021 §6, ADR-0002 §9).
- **The toggle** (ADR-0147 §4, Nick's decision of 2026-11-24): a default
  `pads` feature on `shuyi-app`. Off, the native source is a null one
  (no pads, one `log::info!(target: "input", …)` line at start), and
  `sdl3-sys` isn't built. `shuyi doctor` checks CMake only for a game
  with it on. The book's quick start says how to turn it off and that it
  saves about 1.8 MB. CI builds `shuyi-app` both ways.
- **No libusb:** `deny.toml` bans `libusb1-sys`; a test reads SDL's
  CMake cache in the build's output and fails if `SDL_HIDAPI_LIBUSB` is
  on.
- **CMake as a prerequisite** (ADR-0147 §2):
  - `shuyi doctor` checks for `cmake` and a C compiler, with the install
    line per system (MSYS2's `mingw-w64-x86_64-cmake`, the Linux
    packages, Homebrew's `cmake`), and a problem code for it;
  - `scripts/setup-linux.sh` (and the templates' copies) installs
    `cmake`;
  - the book's quick start says so.
- **gilrs removed** (ADR-0021 §5): from `crates/app/Cargo.toml`, the
  gilrs source deleted. If nothing else needs libudev (`cargo tree -i
  libudev-sys` empty), its row leaves doctor's Linux table
  (`crates/cli/src/contract/doctor/linux.rs`) and its package leaves the
  `APT`, `DNF` and `PACMAN` lines of every `scripts/setup-linux.sh`.
- **The game's compile:** measure `cargo xtask compile-times` before and
  after. The C build is paid once per target directory. If the
  dependency count or the rebuild ratio passes
  `tools/compile-budget.ron`, that is a finding to settle by ADR-0049's
  rules in this PR, never a silent raise.
- **Rumble, either order with 2408 (parked):** if 2408 is done, the
  source implements its `Motors` with `SDL_RumbleGamepad(strong, weak,
  for_ms)` in place of gilrs's force feedback; `stop` is a rumble of 0.
  If 2408 isn't done, nothing here (2408 adds it, by its own clause).
- **Analog, either order with 1121 (parked):** if 1121 is done, its axis
  events come from this source's `PadState`s unchanged.
- **For the game:** `shuyi-app`'s API is unchanged. Say in the PR that
  the game's next pin bump needs CMake on its machines and CI, and no
  code change.

**Estimated size:** about 1,000 lines (code 400, tests 280, doctor, setup
and docs 320).
**Split:** past 900 lines, split off the `pads` toggle and its doctor
check into a new ticket.

**Out (do not do):**
- Switch 2 pads (2403).
- The logo button, gyro, touchpads, lights, battery: SDL has them; write
  a ticket when a game asks.
- Steam Input's action sets (3012's Out).
- The web (it keeps `gamepad.js`), Android and iOS (3015).
- The safe `sdl3` wrapper, unless its gamepad API covers every call above
  with SDL's other parts left out; say which in the completion notes.

## Implementation steps

1. Add `sdl3-sys` to `crates/app/Cargo.toml` (native targets only), the
   ban in `deny.toml`, and check `cargo deny check`.
2. `crates/app/src/pads/sdl.rs` (the calls) and the source in
   `native.rs`, with the same `Source::new` and `Source::read`.
3. The libusb check and the tests below.
4. `shuyi doctor`'s CMake check, the setup scripts, the book.
5. CI: the native jobs on Windows, Linux and macOS build it (their
   runners have CMake); note the cold build's time.
6. Remove gilrs and, if unused, libudev.
7. Play on Windows with the Xbox and PS5 pads, and on the Steam Deck.
8. Completion notes: the pads tried, the compile-time numbers, what Nick
   said.

## Acceptance criteria

- [ ] On Windows, the Xbox and PS5 pads drive the game's menus and
      battles; every button by position, as 2903's probe found.
- [ ] On the Steam Deck, the built-in controls drive the game.
- [ ] Plugging a pad in or out while the game runs is seen within a
      second.
- [ ] `cargo tree --workspace` shows no `gilrs`, `gilrs-core` or
      `libusb1-sys`.
- [ ] `shuyi doctor` names CMake when it's missing, with its install
      line.
- [ ] With `pads` off, a game builds without CMake, runs, and reads no
      pads on desktop; with it on (the default), it reads them.
- [ ] The web build still reads pads through `gamepad.js`.
- [ ] If 2408 is done: the Xbox and PS5 pads rumble for a cue.
- [ ] Nick signs off.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: SDL's buttons and axes to `PadState` (a table test over every
  button, like today's `every_real_button_is_read_from_its_own_gilrs_button`);
  the trigger threshold; `045e:02ff` is `PadKind::Xbox`.
- The real backend starts (or fails cleanly) and reads three frames
  without panicking, whatever is plugged in (as today's
  `starting_and_reading_never_panics`).
- The libusb check above.
- Manual: step 7.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
