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
  then depends on this repository by a pinned revision. Done 2026-10-19,
  with this repository's code gates in CI and Pages for the examples
  ([`carve.md`](carve.md)).

## Phase 2: the MVP, parity with the game

The engine does everything the game needs today, from this repository,
and agents can drive it. These tickets, in order:

| Ticket | What |
| ------ | ---- |
| 2002 | This repository's gates: the ticket lint, typos and links, in the CI 1131 started with the code gates |
| 2007 | Coverage and SonarCloud in CI, and the Sonar debt queue as batched GitHub issues, copied from the game |
| 2201 | Formats are plugins: the registry (ADR-0007) |
| 1129 | The tool contract: `shuyi new`, `check`, `run`, `scene`, `replay`, `bots`, `import`, `doctor` |
| 1133 | An MCP server over the contract |
| 1134 | The engine's skill pack (Agent Skills) |
| 1132 | Community files and a docs site with the examples playable in the browser |
| 2006 | `check-keys` and `check-text` on the engine's own code |
| 2005 | The first release on crates.io, and the game on versions |

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
([ADR-0013](adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)).

| Ticket | What |
| ------ | ---- |
| 2501 | Templates as data: `template.ron`, parts, a pure composer behind `shuyi new` |
| 2502 | The wizard: one question per concern from the catalog, multi-pick, a flag per answer, the questions as JSON |
| 2503 | Several perspectives in one game: a stage each, moves between them, a test that crosses them |
| 2504 | Targets in the wizard: the picked modules' needs checked against each target (with 3017), CI per target (after 3001 and 3004) |
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

On 2026-10-17 Nick added 3D, HD-2D, and GBA and DS ports as engine
features ([`decisions.md`](decisions.md)). Nothing in the game needs
them, so they are parked like the rest. The order inside each, when
unparked:

- **3D:** 3101, then 3102 and 3105, then 3103, 3104 and 3106, then
  3107, then the examples 3108 and 3109.
- **The GBA and the DS:** 3017 and 3018 first, then 3019, 3020 and
  3023, then the GBA (3021, then 3027 and 3022), then the DS (3024,
  3025, 3026). DS 3D (3025) also needs 3103 and 3107.

On 2026-10-18 Nick added real-time strategy with a thousand units or
more and the AI to run it, visual novels, and detective games
([`decisions.md`](decisions.md)). These wait longer than the rest:
**until after Visions of Shuyi launches**, not just the MVP. The order
inside each, when unparked:

- **Real-time strategy** (ADR-0015): 2705 (measure first) and 2706, then
  2707, then 2708, 2709 and 2713, then 2710, 2711 and 2712, then the
  economy (2733), the AI (2714, 2715) and the view (2716, then 2734),
  then 2717, 2718 and the example (2719). Multiplayer (2720) needs 2706 and the networking ticket 1140.
- **Visual novels** (ADR-0016): 2721, 2722, then 2723 and 2725, then
  2724, then the example (2726).
- **Detective games** (ADR-0016): after 2722, 2727, then 2728, 2729 and
  2730, then the fair-play check (2731, after 2724), then the example
  (2732, after 2726).

| Tickets | What | Unparked early if |
| ------- | ---- | ----------------- |
| 1119–1125 | Real time: fixed-step simulation, cameras, pointer and analog input, collision, animation, Tiled and LDtk, and a walkable town that starts a grid battle | the game's 0060 says the army walks around freely |
| 1126–1128 | Isometric, side view (platformer), optional physics | — |
| 1135 | Research: visual scripting as a view over text | — |
| 1140–1150 | Networking: lockstep, relay, browser, LAN, Steam, sessions, referee server, real time, accounts and ratings, chat, MMO and global chat research | the game's 0053 picks an online mode |
| 2701–2704 | Procedural animation, a module (ADR-0010): bodies of points in `Fx`, limbs and gaits that find footholds, strips and rotated sprites to draw them, and a side-view cave example | Nick asks (2026-10-16: "this will be very late addition since it won't tie into visions of shuyi game") |
| 2205–2206 | MIDI music, a module (ADR-0014): `.mid` and `.sf2` formats, a synth, cues that play a sequence, an example | Nick asks (2026-10-17: he chose to park it, since the game doesn't need it) |
| 3014–3016 | Export targets that cost money and bring agreements: Nintendo Switch and the eShop, mobile (Android, iOS), research on PlayStation and Xbox | Nick asks (2026-10-15: "eventually nintendo eshop and maybe even mobile and console") |
| 3017–3027 | The GBA and the DS (ADR-0012): target capabilities and the check that a game fits its target's hardware, toolchain research and the selling question, indexed colour, old ARM builds in CI, the GBA backend, its audio and device, two screens, the DS backend, DS hardware 3D, the DS device | Nick asks (2026-10-17: "I want GBA, and DS ports") |
| 3101–3109 | 3D and HD-2D, as modules (ADR-0011): the 20.12 fixed-point space, glTF meshes, the 3D view and a software rasterizer, the GPU painter, 3D collision, cameras, the HD-2D diorama skin, and four templates with their examples (first person, third person, top-down 3D, HD-2D) | Nick asks (2026-10-17: "add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D") |
| 2705–2720, 2733–2734 | Real-time strategy at scale, a module (ADR-0015): measuring units at scale, command sims, units in columns, a spatial grid, sectors and flow fields shared by armies, steering and formations, combat, fog of war, the economy (resources, workers, construction, production), AI for units, squads and commanders, the view and selection, parallel steps on desktops, budgets of a thousand units, an example, and lockstep multiplayer | Nick asks, after the game launches (2026-10-18: "make tickets parked for after visions of shuyi launches") |
| 2721–2726 | Visual novels, a module (ADR-0016): a story as turn rules, stage directions, characters as data, the scene and skin, text speed, auto, skip, the backlog, going back, saves anywhere, the gallery, story checks and a route chart, voices, an example | the same |
| 2727–2732 | Detective games, a module on top of visual novels (ADR-0016): clues and the case file, investigation, deductions, testimony, a fair-play check that every case can be solved, an example | the same |
