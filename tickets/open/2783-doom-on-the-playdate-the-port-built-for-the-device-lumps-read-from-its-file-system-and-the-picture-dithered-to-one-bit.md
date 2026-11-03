---
id: "2783"
title: "DOOM on the Playdate: the port built for the device, its WAD read from the device's files a lump at a time, and its picture dithered to one bit"
type: feature
milestone: "Engine: demos"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2780"]
nick_input: sign-off
completed:
---

# 2783 — DOOM on the Playdate

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*). 2780 runs DOOM on the engine on the desktop,
from `github.com/Zafnok/shuyi-doom` (GPL). The Playdate is the engine's
smallest target (ADR-0008): a 400 × 240 one-bit screen, a 168 MHz
Cortex-M7 and 16 MB of memory. If DOOM runs there through the engine,
the engine's claim to run "anything" on it holds.

What there is:

- the Playdate backend, `shuyi-playdate`, and `shuyi export playdate`
  (3010, done); the game runs on a stack of its own (3028);
- 2399 dithers an indexed picture to one bit through `onebit::BAYER`,
  in integers, at paint time;
- 2244 plays sounds a game adds at run time, on the Playdate too;
- the budgets for the Playdate are provisional (3011 measures them on
  a device).

What's hard:

- **memory**: Freedoom Phase 1 is bigger than the device's memory, and
  DOOM's zone wants several megabytes beside it. So the WAD is read
  **a lump at a time** from the game's folder on the device, through
  the SDK's file API, never whole. DOOM caches lumps in its zone and
  frees the purgeable ones itself; the zone gets a fixed size (start at
  6 MB, tune it);
- **C for the device**: doomgeneric is compiled with the SDK's
  `arm-none-eabi-gcc` for the Cortex-M7 and linked with the port's Rust
  for `thumbv7em-none-eabihf`. It needs a C library. **Either order
  with 2782:** if 2782 is done, try its small libc first; if not, use
  the SDK's newlib. Nothing is left to 2782 either way;
- **speed**: DOOM's renderer at 320 × 200 then a dither of 64,000
  pixels a frame; the target is 30 frames a second on the device
  (the screen's own rate), 20 at worst.

## Nick input

**Sign-off:** play Freedoom's first map in the Playdate simulator, and
on a device if you have one. Comment on how it reads in one bit and
how it plays with the d-pad and crank (if the crank is used, below).

## Scope

**In** (in `Zafnok/shuyi-doom`, unless it says this repository):
- **The build**: `shuyi export playdate` of the port, with the C built
  for the device by the SDK's compiler from `build.rs`. If the engine's
  Playdate export can't take a crate that compiles C, write the engine
  ticket for it (the `write-ticket` skill, in this repository) and
  block on it.
- **The WAD from the device's files**: the port's file layer on the
  Playdate opens the WAD in the game's data folder and reads each lump
  when DOOM asks, through a game-facing file reader of
  `shuyi-playdate`. If the backend has none (it has storage for saves,
  `crates/playdate/src/storage.rs`), write the engine ticket for one:
  open a file of the game's folder, read a range of bytes. The player
  copies `freedoom1.wad` (or their own WAD) into the game's folder over
  USB; the README says how.
- **The picture**: DOOM's 320 × 200 indexed picture placed 1:1 in the
  middle of the 400 × 240 screen, dithered by 2399. The palette's
  flashes dither too. A setting chooses between `BAYER` and a plain
  threshold.
- **Controls**: the d-pad moves and turns, A fires, B uses, A and B
  together open the menu; the crank turns if the player turns it
  (read as the crank's change each frame through the backend's input,
  if `shuyi-playdate` reports it as an analog action; if it doesn't,
  the crank is unused and the completion notes say so). All of it
  through the keymap's `playdate` table.
- **Sound**: DOOM's sound lumps through 2244 on the Playdate.
- **Measuring**: frame times in the simulator through the engine's
  frame records (3063), written in the completion notes; on a device
  too if Nick has one.
- **This repository**: the PR moves the ticket to `done/`, and the
  ROADMAP's demos part says where DOOM runs.

**Out (do not do):**
- A render of DOOM at 400 × 240: the original 320 × 200 only.
- The engine's Playdate budgets (3011).
- Music (2784, parked).

## Implementation steps

1. Read 2780's port, `crates/playdate/` (the backend, its storage, its
   input, its sound), 3010's export, 2399's dither and the SDK's file
   API.
2. Build doomgeneric for the device, and link it.
3. The lump-at-a-time file layer, and the zone size.
4. The picture, the controls, the sound.
5. Play it in the simulator; measure; tune the zone and the dither.
6. The PR here.

## Acceptance criteria

- [ ] `shuyi export playdate` builds the port, and Freedoom Phase 1's
      first map plays in the Playdate simulator: menus, a map played
      through, sound, a save.
- [ ] The WAD is never held whole in memory (the heap counter's peak,
      3064, written in the completion notes).
- [ ] Frame times in the simulator are written in the completion notes,
      and on a device if one was used.
- [ ] Every control is an action through the keymap's `playdate` table.
- [ ] Nick signed off.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass, here and the port's own.

## Tests required

- Unit (in the port): the lump reader over a file in parts; the zone
  size check at start.
- Snapshot / integration (in the port): `frame-png --target playdate`
  of tic 500 of `DEMO1` as a golden PNG; the scene after 1,000 tics
  equal to the desktop's.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
