# Roadmap

Plan B, as Nick chose it on 2026-10-14 ([`decisions.md`](decisions.md)):

1. carve the engine out of the game;
2. bring it to parity with the game (the MVP) and publish it with its AI
   tooling;
3. build the rest when the game, or a clear audience, needs it.

Nothing here is ever put in front of the game's Chapter 1 work.

## Phase 0: this repository (done 2026-10-14)

Plans, tickets, skills, licenses and the README (ticket 2001). No code.

## Phase 1: the carve, in the game repository

The game's tickets, worked in `github.com/Zafnok/visions-of-shuyi` after
Nick's Chapter 1 playtest (game ticket 0804):

- **1102**: the `engine/` folder and a gate that keeps game code out of
  it. This one can run before 0804, because it conflicts with no open
  work.
- **1103–1117**: generic parts of the game move into `shuyi-*` crates, one
  layer at a time, with no change in behavior:
  - asset types;
  - input actions as an open set;
  - the frame;
  - screens and `Ctx<G>`;
  - the platform shell;
  - the test harness;
  - generic screens;
  - cinematics;
  - the tactics kit;
  - the tools.
- **1118**: a small example game made from engine crates alone.
- **1131**: the `engine/` folder moves here with its history. The game
  then depends on this repository by a pinned revision.

## Phase 2: the MVP, parity with the game

The engine does everything the game needs today, from this repository,
and agents can drive it. These tickets, in order:

| Ticket | What |
| ------ | ---- |
| 2002 | This repository's gates: CI, the ticket lint, the license check |
| 2201 | Formats are plugins: the registry (ADR-0007) |
| 1129 | The tool contract: `shuyi new`, `check`, `run`, `scene`, `replay`, `bots`, `import`, `doctor` |
| 1133 | An MCP server over the contract |
| 1134 | The engine's skill pack (Agent Skills) |
| 1132 | Community files and a docs site with the examples playable in the browser |

## Phase 3: after the MVP

These are not parked, but they come after the MVP:

- **2202**: dialogue in Yarn Spinner and Ink, beside our `.dlg`.
- **2203**: more image and audio formats as plugins.
- **2204**: an optimized pack format for shipping.
- **2901**: research on better images with machine learning, as a
  separate repository with a loader plugin.

## Parked

Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets
for networking, styles for later". Each one's title starts with
`Parked:`. None is started until Nick unparks it.

| Tickets | What | Unparked early if |
| ------- | ---- | ----------------- |
| 1119–1125 | Real time: fixed-step simulation, cameras, pointer and analog input, collision, animation, Tiled and LDtk, and a walkable town that starts a grid battle | the game's 0060 says the army walks around freely |
| 1126–1128 | Isometric, side view (platformer), optional physics | — |
| 1135 | Research: visual scripting as a view over text | — |
| 1140–1150 | Networking: lockstep, relay, browser, LAN, Steam, sessions, referee server, real time, accounts and ratings, chat, MMO and global chat research | the game's 0053 picks an online mode |
