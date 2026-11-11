---
id: "2781"
title: "DOOM replays its own demos: Freedoom's demo lumps played headless to their end, and a played session's TickRecord replayed equal"
type: feature
milestone: "Engine: demos"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2780"]
nick_input: none
completed:
---

# 2781 — DOOM replays its own demos

## Context

Nick, 2026-11-03 ([`docs/decisions.md`](../../docs/decisions.md),
*Bad Apple and DOOM*). 2780 runs DOOM's own code on the engine, in
`github.com/Zafnok/shuyi-doom` (GPL). This ticket turns DOOM into a
determinism test of the engine's path for real-time games, the claim
ADR-0003 §2 makes: a tick game is its inputs per step, and replaying
them gives the same game.

DOOM was built the same way 30 years earlier. A demo lump (`DEMO1`
and on in a WAD) is a header and a `ticcmd_t` for each tic; the game
replays it, and a single difference in the simulation shows as a
**desync**: the player walks into a wall the recording didn't. Source
ports keep vanilla demos playing as their strictest test, and
doomgeneric comes from Chocolate Doom, which keeps them exact.

So two checks:

- **DOOM's demos through the engine**: Freedoom's demos (Phase 1 and
  Phase 2, at the release 2780 pins) played headless, through the
  engine's steps, to their end. They were recorded against vanilla
  behaviour, so they play to their end only if the port changed
  nothing of the simulation: time from steps (2780), the memory file
  layer and the patched saves included;
- **the engine's records through DOOM**: a session played on the
  desktop, recorded by the engine as a `TickRecord` (2137) and as a
  debug build's session record (2519), replayed headless to the same
  scene and the same DOOM consistency checksum at every tic.

## Nick input

None.

## Scope

**In** (in `Zafnok/shuyi-doom`, unless it says this repository):
- **A headless run**: the port on the engine's headless platform
  (`shuyi_platform::headless`), no window, no sound device, a WAD from
  a path. `cargo run -- --headless --iwad freedoom1.wad --playdemo
  demo1` plays a demo and prints, at its end, the tic count and a hash
  of the game state: DOOM's own consistency value (`consistancy` in
  `d_net`/`g_game`, the player's position and angles) folded with the
  map's thinkers into a 64-bit FNV hash, in the engine's
  `StateHash` print.
- **The demo test**: every demo lump of Freedoom Phase 1 and Phase 2
  played to its end in CI, each with its tic count and final hash kept
  in a golden file (`tests/demos.ron`). A demo that desyncs (it ends
  early, or the player dies where the recording didn't) is a failing
  test, and is fixed in the port, never by editing the demo or the
  golden. If one desyncs in Chocolate Doom as well (check with its
  `-playdemo` at the same Freedoom release), it's marked as such in
  the golden with that reason.
- **The record test**: a 60-second scripted session (an engine input
  script, by actions: walk, turn, fire, open a door) played through the
  harness, recorded as a `TickRecord`, replayed headless, and equal at
  every step; the same with a session record saved by a debug build
  (2519's debug menu entry), played by `shuyi run`.
- **wasm32**: **Either order with 2782:** if 2782 is done (its C
  builds for the web), the demo test also runs as wasm32 in CI and
  gives the same hashes; if not, native only, and 2782 adds the wasm32
  run.
- **This repository**: the PR moves the ticket to `done/`; the ROADMAP's
  demos part says DOOM's demos replay through the engine.

**Out (do not do):**
- Demos of DOOM's commercial WADs in CI: the repository has none. A
  player may run `--playdemo` on theirs.
- Multiplayer demos.
- Recording DOOM demo lumps from the engine (`-record`): the engine
  records its own `TickRecord`.

## Implementation steps

1. Read 2780's port, DOOM's `g_game.c` (`G_DoPlayDemo`,
   `G_ReadDemoTiccmd`, the consistency checks), 2137's `TickRecord`
   and `replay`, 2519's session records, and the engine's headless
   platform.
2. The headless run and the state hash.
3. The demo test and its golden; check any desync against Chocolate
   Doom before calling it the port's bug.
4. The record test.
5. CI; the wasm32 run if 2782 is done.
6. The PR here.

## Acceptance criteria

- [ ] Every demo lump of Freedoom Phase 1 and Phase 2 plays headless
      through the engine's steps to its end, in CI, with its tic count
      and hash in the golden (or a recorded reason it desyncs in
      Chocolate Doom too).
- [ ] A recorded `TickRecord` and a debug build's session record each
      replay to the same hash at every step.
- [ ] The wasm32 run gives the same hashes, if 2782 is done (else the
      completion notes say 2782 adds it).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass, here and the port's own.

## Tests required

- Unit: the state hash over a hand-built state.
- Snapshot / integration: the demo test; the record test; the session
  record played by `shuyi run`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
