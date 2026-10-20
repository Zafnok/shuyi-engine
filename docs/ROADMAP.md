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

### Requests from the game

The engine parts of the game's open tickets, split off by game ticket
1157 (2026-10-19). Each names its game ticket, and that game ticket
waits for it (`engine#NNNN` in its `blocked_by`). 2304 and 2305 are
small and hold up the game's Chapter 1 playtest fixes, so they come
first, before the MVP's tickets above (they are part of the MVP:
parity with what the game needs). The rest come when the game reaches
them.

| Ticket | What | For |
| ------ | ---- | --- |
| 2304 | The grid scene says where the map goes on past the view; the grid skins let a game paint over the units and under the cursor | game#0449 |
| 2305 | The text box paints narration from the left when the game asks | game#0740 |
| 2301 | Wide glyphs: two cells in the glyph buffer, from a wide font in the atlas | game#0236 |
| 2302 | Line breaking for text without spaces, and a report of translated text too long for its place | game#0237 |
| 2303 | Wide glyph sets per language, widths by language, Chinese line breaking | game#0239 |
| 2207 | Bundle layers from outside the binary: files handed in at run time, a folder on disk | game#1012, game#1014 |
| 2101 | A deterministic, sandboxed script runtime: the `shuyi-script` module, and the ADR choosing its language | game#1017 |
| 2102 | The `shuyi-script` module's example | 2101 |

### Friction from the game

Game ticket 1153 (2026-10-19) measured the game's biggest screens
against what they should take with a good engine (game ADR-0092, its
`docs/engine/friction.md`). What it found the engine missing, ranked
there by ratio × how many screens shrink. None of it holds up the game:
the game ships with its own code meanwhile, and its follow-up ticket
deletes that code once the engine has the piece.

| Ticket | What | For |
| ------ | ---- | --- |
| 2307 | A list cursor with wrap, disabled rows and a scroll window, under the menu | game#1160 |
| 2306 | A screen hands a result to the one below when it pops; covered and uncovered hooks | game#1158, game#1159 |
| 2208 | A folder loader on the bundle | game#1169 |
| 2312 | A frame time you can trust, a clock a held key speeds up, the text box's Confirm step | game#1166 |
| 2311 | Glyph drawing helpers: a titled box, a meter, text centred or cut | game#1165 |
| 2309 | A question widget | game#1163 |
| 2310 | A player profiles screen and a name entry widget | game#1164 |
| 2308 | Menu items carry a value; labels are text keys | game#1162 |
| 2313 | The kit's grid skin: zoom, a held walk speed, a per-frame painter, glyph-skin styles | game#1167 |
| 2314 | Option rows for an enum setting | game#1168 |
| 2209 | Word wrap in the assets crate, and a fits-its-box check | game#1169 |
| 3028 | The Playdate backend runs the game on a stack of its own | game#1170 |
| 3029 | The audio player for every backend, and Playdate audio | game#1170 |

The sweep also added notes to 3003 (a loading phase), 3010 (the
game's front end as working code to lift) and 3011 (the report a
shipped game keeps).

### A frame cap

Nick, 2026-10-19 ([`decisions.md`](decisions.md)): a game runs at most
60 frames a second unless its developer says otherwise, so a simple
game doesn't max out a CPU. Part of the MVP:

| Ticket | What |
| ------ | ---- |
| 2321 | A frame cap: 60 by default, set by the game and the player, working with vsync and VRR, and a request the platform confirms |

### Seeing inside a running game: the debug view

Nick, 2026-10-19 ([`decisions.md`](decisions.md)): see level layouts,
collision bounds and skeletons in real time, in debug mode. The debug
view draws gizmos (outlines, lines, crosses, labels) over the running
game, by channel, from the debug menu. A gizmo is plain data, built
read-only, so a replay with the view on has the same hash as one with
it off. These two come after the MVP and need nothing parked:

| Ticket | What |
| ------ | ---- |
| 2315 | Gizmos over the frame, channels on the debug menu, `frame-png --debug`, and the grid kit's layout as the first channel |
| 2316 | In real time: pause, step one frame, slow motion, and a readout of the top screen |

The channels for collision (2317), level layouts (2318), skeletons
(2319) and 3D (2320) follow the parked work they draw, and are parked
with it (*Parked*, below).

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

On 2026-10-19 Nick asked for a visual editor like Godot's or Unreal's
Blueprints, and for the debug view's channels
([`decisions.md`](decisions.md)). The editor follows the visual
scripting research (1135), which is parked, so it is parked too. Each
channel is parked with the work it draws. The order, when unparked:

- **The debug view's channels:** collision (2317) after 1122; level
  layouts (2318) after 1120 and 1124; skeletons (2319) after 2318, 2701
  and 2702; 3D (2320) after 3102, 3103 and 3105.
- **The editor:** the research (2506) after 1129 and 2316, then the
  shell (2507), then the graph panel (2508) after 1135.

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

On 2026-10-19 Nick decided macroquad and the libraries around it leave
the engine, after the MVP ([`decisions.md`](decisions.md),
[ADR-0021](adr/0021-the-platform-layer-in-house-clean-room-pieces-in-their-own-crates.md)):
graphics through wgpu and the window through winit, as in Bevy, with our
own code for the rest where research says so. Ours is written from
specifications, never copied. The order, when unparked:

- **Sound:** the research (2902) first. If it keeps our own code: 2210,
  then 2211, 2212, 2213 and 2214, then the mixer (3030), then output on
  Windows and the web (3031), then Linux and macOS, which removes quad-snd
  (3032).
- **Controllers:** the research (2903) first. If it keeps our own code:
  2402, then 2403, which removes gilrs.
- **The two small ones:** web saves (3033) and file embedding (2215).
  Neither needs anything else.
- **macroquad:** the research (3034), which writes the ADR and the tickets
  for winit and our own 2D renderer over wgpu, which draws on Vulkan,
  Metal, Direct3D 12, OpenGL and GLES, WebGL 2 and WebGPU.
- **Repositories of their own** (2008): last, for each piece we wrote, when
  it is a full replacement and Nick says so.

On 2026-10-20 a comparison with Bevy 0.20 and Godot 4.6 found engine
features that no ticket covered (`AGENTS.md` rule 2). The game needs
none of them, so each is parked under Nick's rule of 2026-10-14. Nick
then asked that Steam achievements and cloud saves have tickets too
([`decisions.md`](decisions.md), 2026-10-20), and decided Spine import picks,
when unparked, between the official runtime under a $69 Essential
license and a reader of our own (the same day). The order, when unparked:

- **Animation beyond clips:** particles (2103) and state machines over
  clips (2104) after 1123; skeletal animation (2105) after 2703, then
  DragonBones import (2216) and Spine import (2217).
- **Free movement:** the navigation mesh (2106) after 1122.
- **Accessibility:** hold or toggle and slower timings (2404), the
  accessibility tree (2322) and a text size the player picks (2323,
  after 2209) need nothing parked; the colour-vision check (2324) after
  3006; screen reader output (3035) after 2322 and the winit backend
  that 3034 plans.
- **The debug view:** the frame profiler (2325) after 2315 and 2316.
- **2D effects:** screen effects (2326) on the wgpu renderer that 3034
  plans, then a game's own shaders (2327) and 2D lights (2328, also
  after 1120).
- **Store services:** achievements and stats with no store in them
  (3036), then Steam achievements, stats and cloud saves (3037).

| Tickets | What | Unparked early if |
| ------- | ---- | ----------------- |
| 1119–1125 | Real time: fixed-step simulation, cameras, pointer and analog input, collision, animation, Tiled and LDtk, and a walkable town that starts a grid battle | the game's 0060 says the army walks around freely |
| 1126–1128 | Isometric, side view (platformer), optional physics | — |
| 1135 | Research: visual scripting as a view over text | — |
| 2506–2508 | The editor (Godot- or Unreal-style): research on where it runs and how it talks to a running game through the tool contract, the shell with a live view, a tree and an inspector over settings files, and the graph panel over `*.graph.ron` | Nick asks (2026-10-19: "UI layer for visual code -- like Godot editor or Unreal Blueprints") |
| 2317–2320 | The debug view's channels: collision bounds, level layouts, skeletons of procedural bodies, and 3D bones and colliders | the work each one draws is unparked (1122; 1120 and 1124; 2701 and 2702; 3102, 3103 and 3105) |
| 1140–1150 | Networking: lockstep, relay, browser, LAN, Steam, sessions, referee server, real time, accounts and ratings, chat, MMO and global chat research | the game's 0053 picks an online mode |
| 2701–2704 | Procedural animation, a module (ADR-0010): bodies of points in `Fx`, limbs and gaits that find footholds, strips and rotated sprites to draw them, and a side-view cave example | Nick asks (2026-10-16: "this will be very late addition since it won't tie into visions of shuyi game") |
| 2205–2206 | MIDI music, a module (ADR-0014): `.mid` and `.sf2` formats, a synth, cues that play a sequence, an example | Nick asks (2026-10-17: he chose to park it, since the game doesn't need it) |
| 3014–3016 | Export targets that cost money and bring agreements: Nintendo Switch and the eShop, mobile (Android, iOS), research on PlayStation and Xbox | Nick asks (2026-10-15: "eventually nintendo eshop and maybe even mobile and console") |
| 3017–3027 | The GBA and the DS (ADR-0012): target capabilities and the check that a game fits its target's hardware, toolchain research and the selling question, indexed colour, old ARM builds in CI, the GBA backend, its audio and device, two screens, the DS backend, DS hardware 3D, the DS device | Nick asks (2026-10-17: "I want GBA, and DS ports") |
| 3101–3109 | 3D and HD-2D, as modules (ADR-0011): the 20.12 fixed-point space, glTF meshes, the 3D view and a software rasterizer, the GPU painter, 3D collision, cameras, the HD-2D diorama skin, and four templates with their examples (first person, third person, top-down 3D, HD-2D) | Nick asks (2026-10-17: "add true 3D support (all kinds, first person, third, top down with 3D models), and HD-2D") |
| 2705–2720, 2733–2734 | Real-time strategy at scale, a module (ADR-0015): measuring units at scale, command sims, units in columns, a spatial grid, sectors and flow fields shared by armies, steering and formations, combat, fog of war, the economy (resources, workers, construction, production), AI for units, squads and commanders, the view and selection, parallel steps on desktops, budgets of a thousand units, an example, and lockstep multiplayer | Nick asks, after the game launches (2026-10-18: "make tickets parked for after visions of shuyi launches") |
| 2721–2726 | Visual novels, a module (ADR-0016): a story as turn rules, stage directions, characters as data, the scene and skin, text speed, auto, skip, the backlog, going back, saves anywhere, the gallery, story checks and a route chart, voices, an example | the same |
| 2727–2732 | Detective games, a module on top of visual novels (ADR-0016): clues and the case file, investigation, deductions, testimony, a fair-play check that every case can be solved, an example | the same |
| 2902–2903, 2210–2215, 2402–2403, 3030–3034, 2008 | The platform layer (ADR-0021): research on sound and on controllers, then our own WAV, Ogg and Vorbis readers, a mixer and sound output, and our own controllers, if the research keeps them; web saves; file embedding; research on replacing macroquad with winit and a 2D renderer over wgpu; and each piece we wrote in a repository of its own | Nick asks (2026-10-19: "All parked until MVP") |
| 2103–2105, 2216–2217 | Animation beyond clips: particles drawn from a seed and the time, state machines over clips with direction sets and frame events, 2D skeletal animation in `Fx` (a `shuyi-bones` module), and DragonBones and Spine import (Spine through the official runtime under an Essential license, or a reader of our own: Nick picks when unparked) | Nick asks |
| 2106 | Paths for free movement: a navigation mesh built from tile collision, routes with clearance, and a path a mover follows | 1122 is unparked |
| 2322–2324, 2404, 3035 | Accessibility: an accessibility tree from the engine's views, a text size the player picks, a colour-vision and text-contrast check, hold or toggle and slower timings, and screen reader output through AccessKit | Nick asks |
| 2325 | A frame profiler in the debug view: phase times, spans, a graph, and a trace file | Nick asks |
| 3036–3037 | Store services: achievements and stats as data with a local backend, an unlock notice and an achievements screen, then Steam achievements, stats and Steam Cloud saves behind the `steam` feature | Nick asks (2026-10-20: "make sure steam achievements and cloud saves are covered by some ticket so it's not lost") |
| 2326–2328 | 2D effects on the wgpu renderer: screen passes (CRT, bloom, palette remap, vignette, flash), a game's own WGSL passes and sprite materials, and 2D lights with normal maps | 3034 is unparked |
