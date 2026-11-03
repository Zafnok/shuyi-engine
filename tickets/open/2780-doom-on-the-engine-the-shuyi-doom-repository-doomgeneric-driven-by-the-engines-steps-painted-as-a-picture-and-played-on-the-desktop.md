---
id: "2780"
title: "DOOM on the engine: the shuyi-doom repository (GPL), doomgeneric driven by the engine's steps and actions, painted as a picture, heard through run-time sounds, and played on the desktop with Freedoom"
type: feature
milestone: "Engine: demos"
model: opus-5.5 / gpt-6.1-sol
effort: xhigh
status: todo
blocked_by: ["2399", "2244", "2137"]
nick_input: setup
completed:
---

# 2780 — DOOM on the engine

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*): "anything that can run video game logic must be
able to run DOOM". Asked where the port lives, he chose a new GPL
repository: DOOM's released code is GPL, and the engine ships only
permissive code (`AGENTS.md` rule 7, ADR-0002 §8). So:

- **This is a port of DOOM's own released code, not a remake.**
  id Software released DOOM's source under the GPL in 1999.
  **doomgeneric** (`github.com/ozkl/doomgeneric`) is a port of it,
  built on Chocolate Doom, made to be moved to new platforms: a
  platform gives it a few functions (`DG_Init`, `DG_DrawFrame`,
  `DG_SleepMs`, `DG_GetTicksMs`, `DG_GetKey`, `DG_SetWindowTitle`).
  Nothing of DOOM is rewritten.
- **It lives in `github.com/Zafnok/shuyi-doom`**, public, under the
  GPL. It depends on the engine by a pinned git revision, as the game
  does. The engine is taken under its MIT license. This repository gets
  no GPL code: only the generic pieces DOOM needs (2399, 2244), and
  this ticket, which its PR here moves to `done/` with a link to the
  port.
- **The license version.** Chocolate Doom's file headers say "either
  version 2 of the License, or (at your option) any later version".
  Check doomgeneric's headers and `LICENSE` at the pinned commit. If
  every file allows "or later", the port is **GPL-2.0-or-later**: then
  it may be combined with Apache-2.0-only crates under GPL-3.0 terms
  (the FSF counts Apache-2.0 compatible with GPL-3.0, not with
  GPL-2.0-only). If any file is GPL-2.0-only, the port is too, and its
  `deny.toml` must refuse Apache-2.0-only dependencies. Write which in
  the port's README. This isn't legal advice; flag it for Nick if a
  file is unclear.
- **The game data.** The port ships no id Software data. It plays
  **Freedoom** (`freedoom.github.io`), whose WADs are under a BSD-style
  license (check its `COPYING` at the release taken): Phase 1 for
  DOOM's episodes, Phase 2 for DOOM II's. A player may use their own
  `doom1.wad` (the shareware), `doom.wad` or `doom2.wad`.

How DOOM fits the engine's rules (ADR-0002, ADR-0003 §2):

- **State changes by inputs per step.** DOOM already works this way: a
  `ticcmd_t` per player per tic, 35 tics a second, and its demos
  (`DEMO1`–`DEMO3` lumps) replay from them alone. The engine steps 60
  times a second (`crates/platform/src/tick.rs`). The port tells DOOM
  the time as **steps, never the clock**: `DG_GetTicksMs` returns
  `step × 1000 / 60` from the step count, so DOOM's own `I_GetTime`
  gives 35 tics a second from the steps, the same on any machine and
  frame rate. `DG_SleepMs` does nothing.
- **Steps come from 2137's `TickDriver`**, with its latch, so a press
  between two frames reaches a step, and a session can be recorded as
  a `TickRecord` (2781 replays them).
- **Actions, never keys** (`AGENTS.md` rule 8). The port's actions are
  DOOM's controls: `Forward`, `Back`, `TurnLeft`, `TurnRight`,
  `StrafeLeft`, `StrafeRight`, `Strafe`, `Run`, `Fire`, `Use`,
  `NextWeapon`, `PrevWeapon`, `Weapon1`–`Weapon7`, `Map`, `Menu`,
  `Confirm`, `Cancel`, `Up`, `Down`, `Left`, `Right`. The port turns
  each frame's actions into the DOOM key events `DG_GetKey` hands out;
  its `keymap.ron` holds the defaults, and the player rebinds them on
  the engine's key bindings screen. DOOM's own key settings are
  unused. Its prompts that name keys ("press y or n") are patched in
  the vendored source to name actions through the keymap.
- **Graphics are a skin.** DOOM draws into its 320 × 200 screen of
  palette indices (`I_VideoBuffer`) with its palette (`PLAYPAL`,
  changed for damage and pickup flashes). The skin copies both into a
  2399 indexed `Painted` picture each frame and places it at the
  largest whole scale that fits, 4:3 corrected by drawing it 6 pixels
  tall for every 5 (as DOOM looked on a CRT; a setting turns it off).
- **Sound** through 2244: the port reads each `DS*` lump (DMX: a
  header, then 8-bit unsigned samples) when the game starts, adds it
  as a cue named after the lump, and plays DOOM's `S_StartSound` calls
  as `PlaySound` with DOOM's volume, centered: DOOM's stereo
  separation needs a pan per play, which the engine's requests don't
  have (a follow-up in the port's repository if it's missed).
- **Files.** The WAD is read whole into memory from a bundle layer
  (`shuyi_app::read_folder` of a `wads/` folder beside the program, or
  the path given with `--iwad`). The vendored file layer
  (`w_file_stdc.c`) is replaced by one reading that memory. Saves go
  through the engine's storage, not `fopen`: `M_WriteFile` and
  `M_ReadFile` are patched to call the port's functions.

## Nick input

**Setup:** make the repository, before the work starts.

1. Open `github.com/new`.
2. Owner `Zafnok`, name `shuyi-doom`, description "DOOM on Shuyi
   Engine, through doomgeneric".
3. Choose **Public**. Leave "Add a README", ".gitignore" and "license"
   unticked: the session adds the GPL text and the README.
4. Click **Create repository**, and tell the session it's there.

## Scope

**In** (all in `Zafnok/shuyi-doom`, unless it says this repository):
- **The repository**: `README.md` (what it is, how to build and play,
  the license section above, credits to id Software, doomgeneric,
  Chocolate Doom and Freedoom), `COPYING` (the GPL text), a
  `rust-toolchain.toml` equal to the engine's, CI on GitHub Actions
  (fmt, clippy, the tests, `cargo deny` with a `deny.toml` for the
  license choice above), Dependabot for the crates and the actions.
- **The crates**: `shuyi-doom` (the library: the platform hooks, the
  skin, the actions, the sound, the files) and a `shuyi-doom` binary.
  The engine by git revision, as the game pins it (`shuyi-engine.rev`
  in the root, read by a small `build.rs` check or by Cargo itself).
- **doomgeneric vendored** under `vendor/doomgeneric` at a pinned
  commit, with a `VENDORED.md` naming the commit and every patch made
  (the time, the files, the saves, the prompts). Built with the `cc`
  crate (`MIT OR Apache-2.0`; check) from the crate's `build.rs`. It
  builds with MinGW-w64 GCC on Windows (Nick's machine has no MSVC,
  `AGENTS.md` *Environment*), with GCC or Clang on Linux, and with MSVC
  on CI's Windows runner.
- **The hooks** in Rust (`extern "C"`): `DG_Init`, `DG_DrawFrame` (the
  skin reads the buffer), `DG_GetTicksMs` (from steps), `DG_SleepMs`
  (nothing), `DG_GetKey` (from the frame's actions), `DG_SetWindowTitle`
  (ignored: the window's title comes from `AppInfo`). The main loop
  isn't doomgeneric's: each frame, the port runs `doomgeneric_Tick`
  once after setting the step count, and DOOM runs the tics due.
- **The game**, through the engine's screens: a screen that holds DOOM
  and passes the frame's actions to it; the engine's key bindings
  screen from DOOM's menu (a patched "Controls" entry) or from a pause
  screen of the engine's on `Menu` held for a second; quit through
  `ctx.platform`.
- **Freedoom for CI**: a script fetches Freedoom's release at a pinned
  version and checks its SHA-256. CI plays Phase 1's `DEMO1` headless
  for 1,000 tics as a smoke test (the full replay is 2781).
- **The tools**: a `GameTools` with `session` giving a scene of DOOM's
  state that matters to a test (the map, the tic, the player's
  position, angle, health, armor, ammunition and weapon, the kills),
  so the MCP tools (`start`, `step`, `scene`, `frame_png`) play DOOM.
- **This repository**: the PR here moves the ticket to `done/`, adds
  the port's link to the demos part of `docs/ROADMAP.md`, and names
  every engine ticket the port found it needed (written with the
  `write-ticket` skill, open, in that PR).

**Out (do not do):**
- Rewriting DOOM, in Rust or otherwise, or using its code inside this
  repository.
- Music: the MUS lumps need MIDI (2784, parked).
- The full demo replays and `TickRecord`s (2781), the web (2782), the
  Playdate (2783).
- Mouse look and analog turning: pointer and analog input is 1121
  (parked). Keys and pads turn.
- Multiplayer, mods (DeHackEd, PWADs beyond `-file`), and source ports'
  extra features.
- Shipping any id Software WAD.

## Implementation steps

1. Read ADR-0002, ADR-0003 §2, 2137's `TickDriver`, 2399's ADR, 2244's
   requests, `crates/app/src/folder.rs`, `examples/hello-grid` (its
   short path and tools), and doomgeneric's `doomgeneric.h`,
   `doomgeneric.c`, `i_video.c`, `i_sound.c`, `w_file*.c` and
   `m_misc.c`.
2. Check the licenses (doomgeneric's headers, `cc`, Freedoom's
   `COPYING`); decide GPL-2.0-or-later or GPL-2.0-only as the Context
   says; write `deny.toml`.
3. Set up the repository, CI and the vendored source with
   `VENDORED.md`.
4. The hooks and the loop; the memory file layer; the time from steps.
5. The skin; then the actions and `keymap.ron`; then the sound; then
   the saves.
6. The tools and the CI smoke test.
7. Play Freedoom Phase 1's first map start to end on Windows, and a
   player's `doom1.wad` if you have one.
8. The PR here.

## Acceptance criteria

- [ ] `Zafnok/shuyi-doom` builds on Windows (MinGW-w64), Linux and
      CI's MSVC runner, and its CI passes.
- [ ] Freedoom Phase 1 plays on the desktop: menus, the first map
      played through, sound effects, saving and loading a game.
- [ ] Two runs from the same `TickRecord` reach the same scene, at 30,
      60 and 144 frames a second (DOOM's time is the steps').
- [ ] Every control is an action; the key bindings screen rebinds
      them; nothing names a key.
- [ ] The README states the license choice and why, and credits id
      Software, doomgeneric, Chocolate Doom and Freedoom.
- [ ] No id Software data is in the repository.
- [ ] This repository's PR moves the ticket and links the port.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass, here and the port's own.

## Tests required

- Unit (in the port): ticks from steps (`step × 1000 / 60` gives 35
  tics a second over 60 steps); actions to DOOM key events; the DMX
  lump reader on a hand-made lump; the palette copy.
- Snapshot / integration (in the port): the CI smoke test; the scene
  after 1,000 tics of `DEMO1` as a golden; a `frame_png` of tic 500 as
  a golden PNG.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
