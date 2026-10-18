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

### Modules and directions

Nick, 2026-10-16 ([`decisions.md`](decisions.md)): a developer sets a
direction and adds modules that don't conflict, and it is obvious what
each piece is built for
([ADR-0009](adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)).

| Ticket | What |
| ------ | ---- |
| 2003 | The `shuyi` crate with a feature per module, module metadata, the catalog by concern, `shuyi modules` and `shuyi add` |
| 2004 | Guides: one per direction, one per move between directions |

Both come before the first module (1128, 1140 or 2701) is built, so it
lands in a place that already works.

### Starting a game: the wizard

Nick, 2026-10-17 ([`decisions.md`](decisions.md)): "some create-react-app
type command with templates", extensible, that asks about perspectives
(several at once, with moves between them), physics, music and export
targets. After the MVP
([ADR-0011](adr/0011-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)).

| Ticket | What |
| ------ | ---- |
| 2501 | Templates as data: `template.ron`, parts, a pure composer behind `shuyi new` |
| 2502 | The wizard: one question per concern from the catalog, multi-pick, a flag per answer, the questions as JSON |
| 2503 | Several perspectives in one game: a stage each, moves between them, a test that crosses them |
| 2504 | Targets in the wizard: modules checked against targets, CI per target (after 3001 and 3004) |
| 2505 | Templates from a folder or git at a revision, no code run, `shuyi template check` and `new` |

The perspective tickets (1122, 1126, 1127) and the first move (1125) each
add their part to the built-in template when they land.

### Export targets: the Steam Deck and the Playdate

Nick, 2026-10-15 ([`decisions.md`](decisions.md)): Visions of Shuyi
should run on the Playdate and the Steam Deck. On the Playdate the rules
are 1-bit graphics, a good frame rate, and the d-pad, A, B and the crank
only. Every target has performance budgets
([ADR-0008](adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)).
The tickets, in order:

| Ticket | What |
| ------ | ---- |
| 3001 | Target profiles as data, `shuyi check --target` (screen and text size) |
| 3003 | The platform boundary: `shuyi-platform`, `App::frame`, a headless backend |
| 3002 | Controls fit: each screen's required actions fit the target, or it doesn't ship |
| 3004 | `shuyi export <target>`: Windows, Linux, macOS, the web |
| 3005 | `no_std` core, UI and platform, gated in CI |
| 3008 | Performance budgets: `shuyi bench`, gated in CI |
| 3006 | 1-bit output: patterns per palette role, a check that sides stay distinct |
| 3007 | 1-bit sprites at bake time, hand-drawn overrides per target (after 2204) |
| 3009 | Bots think in slices: the frame rate holds, the decision never changes |
| 3012 | Steam Deck target |
| 3013 | Steam Deck on the device: frame times and battery |
| 3010 | The Playdate backend and `shuyi export playdate` |
| 3011 | Playdate on the device: frame times and memory, budgets calibrated |

3001, 3003 and 3005 are the cheapest early, because each one gets
dearer as code piles up. The game's own Playdate port is still its
ticket 1025, parked until after launch.

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
| 2701–2704 | Procedural animation, a module (ADR-0010): bodies of points in `Fx`, limbs and gaits that find footholds, strips and rotated sprites to draw them, and a side-view cave example | Nick asks (2026-10-16: "this will be very late addition since it won't tie into visions of shuyi game") |
| 2205–2206 | MIDI music, a module (ADR-0012): `.mid` and `.sf2` formats, a synth, cues that play a sequence, an example | Nick asks (2026-10-17: he chose to park it, since the game doesn't need it) |
| 3014–3016 | Export targets that cost money and bring agreements: Nintendo Switch and the eShop, mobile (Android, iOS), research on PlayStation and Xbox | Nick asks (2026-10-15: "eventually nintendo eshop and maybe even mobile and console") |
