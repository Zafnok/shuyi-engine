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
and agents can drive it. On 2026-10-20 Nick added protections for the
code to the MVP, before the first release (the second comparison, in
phase 3 below): 2010–2017, 2509, 2511, 2513, 2808, 3039 and 3040.
On 2026-10-21 he asked for the docs to start early, so that our own
sessions search less: the docs map (2810) comes first, and the MCP
server (1133), the skill pack (1134), the book (2802) and the guides
(2004) write into it. These tickets, in order:

| Ticket | What |
| ------ | ---- |
| 2810 | The docs map: one home for each kind of fact, by who reads it, a `document` skill, and `check-docs` |
| 2002 | This repository's gates: the ticket lint, typos and links, in the CI 1131 started with the code gates |
| 2007 | Coverage and SonarCloud in CI, and the Sonar debt queue as batched GitHub issues, copied from the game |
| 2016 | Governance and continuity: `GOVERNANCE.md`, a backup owner, the accounts and secrets, `main` protected by its required checks |
| 2017 | The backlog and the docs kept true as tickets land: a lint warning on clauses about done tickets, `contract.md` tested |
| 2011 | Dependabot for the crates and the pinned actions, a weekly security workflow, a Dependabot PR as its own ticket |
| 2012 | The pinned Rust in step: `cargo xtask check-toolchain`, a reminder of a new stable, a `bump-rust` skill |
| 2010 | Every PR shows its public API change: `cargo-semver-checks` against the base, and a `Breaking:` line when it breaks |
| 2013 | Public types that can grow without breaking a game: `#[non_exhaustive]` and constructors |
| 2014 | Visions of Shuyi built against each engine PR and nightly `main`, report-only |
| 2015 | `check-layers` over cargo metadata, a float ban in `shuyi-core`, and a new game's rules crate kept pure |
| 2201 | Formats are plugins: the registry (ADR-0007) |
| 1129 | The tool contract: `shuyi new`, `check`, `run`, `scene`, `replay`, `bots`, `import`, `doctor` |
| 2509 | Problem codes: a code and a help line on every error the contract gives, `shuyi explain`, an exit code for "fix the setup" |
| 2530 | Problem codes for the validators: unknown names, duplicates and values out of range in the keymap, the language packs and the maps |
| 3040 | One log for the engine and the game, kept by `App` and carried in the contract's answers |
| 2513 | Zero setup on Linux: `shuyi doctor` checks the system libraries, a setup script, a dev container |
| 2511 | The short path: engine defaults for every `GameData` part, a prelude, `run_game`, and a minimal template |
| 1133 | An MCP server over the contract |
| 1134 | The engine's skill pack (Agent Skills) |
| 1132 | Community files and the contribution policy |
| 3039 | Web builds carry the notices they owe: a `NOTICES.txt` with the crates, the vendored scripts and the fonts |
| 2801 | The website: a Zola site on Pages with Home, Learn, News, Examples, Community and Sponsor |
| 2802 | The book: a quick start, a chapter per concept whose code CI compiles, and the API docs |
| 2803 | The tutorial: a box-pushing puzzle built in ten steps, each tested and playable |
| 2808 | Docs agents can read: `llms.txt`, a Markdown twin of every Learn page, the contract's schemas |
| 2804 | News with an automatic changelog from the tickets done, and a post per release |
| 2805 | The blog: Nick's posts in News, by hand or drafted by Claude |
| 2806 | Sponsors: GitHub Sponsors and Ko-fi, a sponsor page and a strip on the home page |
| 2006 | `check-keys` and `check-text` on the engine's own code |
| 2005 | The first release on crates.io, and the game on versions |

## Phase 3: after the MVP

### Bevy's homepage

Nick, 2026-10-20 ([`decisions.md`](decisions.md)): "every feature outlined
on Bevy's homepage should be available in Shuyi engine at some point in
the not so distant future", with an ECS that may be optional. The new
tickets come after the MVP and are not parked. Bevy features that were
already parked stay parked until he unparks them. The website he asked
for at the same time is in the MVP above (2801–2806, and 2807 parked).

Each feature on bevy.org (read 2026-10-20), and where it lives:

| Bevy's homepage | Here | State |
| --------------- | ---- | ----- |
| Data driven: an ECS, components as structs, systems as functions, queries, resources, change detection, a parallel scheduler | 2904 (research), 2107 (`shuyi-ecs`), 2108 (in a game, parallel with the same hashes, an example). Opt-in; the arena stays the default (ADR-0003 §4) | after MVP |
| 2D renderer: sprite sheets, dynamic texture atlases, cameras, textures, materials | 1123 and 2231 (sprite sheets and clips), 2222 (atlases), 2335–2338 (rotation, shapes, tints and blends, nine-slice), 1120 (cameras), 2327 (materials and a game's shaders) | 2222 and 2335–2338 after MVP; the rest parked |
| 3D renderer: lights, shadows, cameras, meshes, materials, glTF | 3101–3109 | parked |
| Render graph | 3034 (wgpu, not tied to one API), 2331 (the graph) | parked |
| Animation: skeletal rigs, blending, morph targets, glTF | 1123 and 2231 (clips), 2104 (state machines), 2105 (2D skeletons), 3102 (glTF), 3110 (3D blending and morph targets) | parked |
| Cross platform: Windows, macOS, Linux, the web, iOS, Android | 3004 (desktop and the web), 3015 (mobile) | 3004 with the export targets; 3015 parked |
| UI: layout in the style of flexbox, declared in code or a file | 2329 (layout), 2355 (focus between widgets), 2330 (`.ui.ron`), 2224 and 2334 (fonts and text off the cell grid) | after MVP |
| Scenes: save and load worlds keeping ids, instances, live reload | 2221 | after MVP |
| Sound | `shuyi-app` today (cues, variants, fades, streamed music), 3029 for every backend; 2341–2345 and 2225 (crossfades, groups and ducking, handles, cue rules, made sounds); 2363 (positional) and 2365 (a beat clock) | partial: the rest after MVP, 2363 and 2365 parked |
| Hot reload of assets | 2220 (every asset kind), 1124's maps through it | after MVP |
| Hot reload of code (not on Bevy's homepage; Bevy has it experimentally) | 2905 (research) | after MVP |
| Fast compiles | 2009 (measured, gated, a fast setup in the templates) | after MVP |
| Free and open source, MIT or Apache 2.0 | `LICENSE-MIT`, `LICENSE-APACHE` | done |

The order, after the MVP: 2009 and 2220 first (each makes every later
ticket nicer to work on), then 2329 and 2222, then 2355, then 2904,
2107, 2108 and 2221, then 2330 and 2905.

These are not parked, but they come after the MVP:

- **2202**: dialogue in Yarn Spinner and Ink, beside our `.dlg`.
- **2203**: more image and audio formats as plugins.
- **2204**: an optimized pack format for shipping.
- **2901**: research on better images with machine learning, as a
  separate repository with a loader plugin.

### The second comparison: Unity, Unreal and the 2D engines

Nick, 2026-10-20 ([`decisions.md`](decisions.md)): a comparison with
Bevy 0.20, Godot 4.7, Unity 6, Unreal Engine 5.8, GameMaker, Defold and
Construct 3 looked at features, and at how the engine is built. Its
protections for the code are in the MVP (phase 2). What most 2D games
expect is below, after the MVP and not parked. The rest is parked
(*Parked*, below), and so is the real-time work, until Visions of Shuyi
launches.

Some of these close the engine part of a game ticket, so they come when
the game reaches it, whatever their place below: 2339 (game#0813,
screen transitions), 2347 (game#0250, key names that stand out), 2228
(game#1013, mod tables merged by id), 2342 (game#0826, the voices'
volume), and 2226 and 2349 (game#0825, the language option).

**Seeing what players see:**

| Ticket | What |
| ------ | ---- |
| 2332 | The raster: `frame-png`'s painter in `shuyi-ui`, the picture every backend must match |
| 2018 | The real window checked in CI under Xvfb, against the raster |
| 2019 | The web build checked in CI in headless Chromium, against the raster |
| 2514 | Frames over time from `frame-png`: a sheet, an APNG or a GIF, and the same from MCP |

**Drawing:**

| Ticket | What |
| ------ | ---- |
| 2333 | Scale modes: integer or fit, one placement for the window and `check --target`, and the player's pick |
| 2335 | Rotated sprites: an angle around a pivot and `flip_y`, with an integer `Angle` in `shuyi-core` |
| 2336 | Drawn shapes for every game: lines, circles and polygons |
| 2337 | Tints and blend modes on sprites and overlays |
| 2338 | Nine-slice and tiled pictures, and a picture frame for the engine's panels and text boxes |
| 2224 | Proportional fonts: an ADR for text off the cell grid, BMFont, and TTF and OTF baked by the font tools |
| 2334 | Text runs: proportional text placed in pixels, with an outline and a shadow, and pixel-placed UI |
| 2339 | Screen transition looks: fade, wipe and dissolve, on every painter |
| 2340 | Frame shake and flash, and the player's settings for reduced motion and flashing |

**Sound:**

| Ticket | What |
| ------ | ---- |
| 2342 | Sound groups as data: a master volume and a volume per group on a loudness curve |
| 2343 | Ducking: a group lowered while a voice plays |
| 2344 | Looped sounds with handles, paused under a pause menu and while the system suspends the game |
| 2345 | Cue rules as data: variants without repeats, random volume, copy caps, minimum gaps and priority |
| 2341 | Music crossfades and fade-ins, with today's fade-out-then-start as the default |
| 2225 | Sound effects from a few numbers (`.sfx.ron`), and menu sounds for new games |

**Text and languages:**

| Ticket | What |
| ------ | ---- |
| 2350 | The font atlas keeps the characters the text uses, and `shuyi check` fails on text it can't draw |
| 2351 | Built-in English for the engine's own text keys, which a game's text overrides |
| 2226 | Language tags in BCP 47, a fallback chain per key, and a source language the game declares |
| 2348 | Messages that choose their words: plural, ordinal and select in a subset of ICU MessageFormat |
| 2349 | The player's language: a setting per profile, an options row, the system's languages at first start |
| 2346 | Typing names in any language: letters the font draws, accented pages, the IME in the name box |
| 2353 | A generated pseudo-language and a Harness walk over every screen for cut text |
| 2347 | Styled text: key names that stand out, and colour roles inside a line |
| 2354 | A grid cursor: `ListCursor` in two dimensions |
| 2355 | `FocusMap`: the d-pad moves between the widgets of a laid-out screen (after 2329, before 2330) |

**The core:**

| Ticket | What |
| ------ | ---- |
| 2109 | Ease curves in integers, and tweens and sequences drawn from the time alone |
| 2114 | RNG helpers: ranges, chances, picks, shuffles, bags, named streams and forks |
| 2115 | Timers in steps or turns |
| 2117 | Eight-way moves with a corner rule, and point-to-point A* in the grid kit |
| 2112 | A level solver: breadth-first search over any `TurnRules` game |
| 2113 | Search bots: minimax and MCTS over any `TurnRules` game, sliced by work units (after 2125, 2112 and 3009) |

**Data, saves and versions:**

| Ticket | What |
| ------ | ---- |
| 3045 | Saves that survive a power cut: flushed writes and the last good copy kept |
| 2110 | Save migrations: an opt-in chain of steps from older versions, and golden fixtures of each |
| 2111 | Stamped records: the engine, the game and the content that made a record or a save |
| 2228 | `Bundle::versions`: every layer's copy of a file, for loaders that merge layers |
| 2229 | Import settings beside any file and per folder |
| 2230 | JSON and CSV tables as formats of a game's own data kinds |
| 2515 | `shuyi kinds`: every kind a game registers, with a JSON schema of each format |
| 2516 | References between assets checked, and unused shipped files reported |
| 2517 | Format upgrades: old names warned about, and `shuyi upgrade` rewriting a game's files |
| 2523 | The `shuyi-upgrade` skill: a game moved to a newer engine |
| 2906 | Research: asset memory, lifetimes, groups and loading in steps |
| 2907 | Research: one description of a game's types for scripts, scenes, schemas and agents (before 2101 and 2221) |

**Debugging, and agents:**

| Ticket | What |
| ------ | ---- |
| 2116 | Determinism fixtures, the same on x64, ARM64 and wasm32 in CI |
| 2020 | Fuzzing in CI over every registered format and the pack, and a threat model |
| 2518 | Session records at the platform boundary, played back exactly in the Harness |
| 2519 | Session records from real builds, played back by `shuyi run`, `scene` and MCP |
| 3041 | Log sinks in shipped builds: a log file, and the browser console on the web |
| 3046 | Crash reports: a panic hook writes a report with the game's record, and the game hears of it at the next start |
| 2520 | `shuyi crash` and `replay --crash`, with the debug file kept for each release |
| 2356 | Debug commands, a console to type them, and a command step in the contract and MCP |
| 2357 | Rewind: seek a session to any frame by replaying it headless |
| 2521 | The live link: `shuyi attach` and MCP reading a running windowed build |
| 2522 | The live link for the web build and for a Steam Deck on the LAN |
| 2510 | Compiler messages on the traits a game implements |
| 2512 | The lines a game's author owns, counted, with a CI cap on the minimal template |
| 2524 | An agent eval of the end goal: tasks as data, a grader through the contract, reference solutions in CI |
| 2525 | Scheduled agent runs of the eval, within a budget Nick sets |
| 2527 | `shuyi soak`: random legal commands, save and reload at every step, and a failure shrunk to a short record |
| 2528 | `shuyi web --serve`: served on localhost or the LAN, rebuilt and reloaded on change |

**The platform, and shipping:**

| Ticket | What |
| ------ | ---- |
| 3042 | Focus on the desktop and the web: held input let go on focus loss, and a game may pause or mute |
| 3043 | The web lifecycle: a hidden tab suspends and saves, and a closing page saves |
| 3044 | The window opens as it was left, and the cursor hides when idle and in fullscreen |
| 2405 | Mouse and touch for menus and grid maps, split out of parked 1121, never required |
| 3053 | Platform facts at run time: the target, quitting, full screen, keyboard, pointer and touch |
| 3048 | A game's identity in `shuyi.ron`: name, id, version, publisher and one icon |
| 3049 | The identity in every package: icons, version resources, `Info.plist`, `.desktop` files |
| 2223 | Credits are the game's: every third-party file covered, and the credits in every build's notices |
| 3050 | Release a game from a tag: every target's package built in CI, a GitHub Release, and itch.io |
| 3052 | The web shell: a download progress bar, and readable errors in place of a black page |

**The ecosystem:**

| Ticket | What |
| ------ | ---- |
| 2358 | Modules join a running game: an ADR and a `Module` trait (before 2003) |
| 2526 | Outside modules: `shuyi add` of a crate at a version or revision, read with no code run |
| 2809 | An Assets page on the website, with other people's work if Nick says yes |

The order, after the MVP, beside the Bevy tickets above (2009 and 2220
stay first):

1. What lets the rest be checked: 2332, 2514, 2116, 2020, 2524, 2111
   and 2518 (which waits for 2111).
2. What keeps players' data safe, before the game ships on Steam
   (game#0903): 3045, 3042, 3043, 3041, 3046 and 2110.
3. The rest, a group at a time, each in the order of its table. The
   shipping group goes with 3004, and 2112 comes before 3009.

### The end goal

Nick, 2026-10-19 ([`decisions.md`](decisions.md)): "it should take less
than an hour to make a wide port range game with basic functionality,
like snake or mario". On 2026-10-20 he put Visions of Shuyi first: the
real-time work both of those games need waits until the game launches.
So the end goal has two chains, and each waits for the launch at its
real-time step:

| Game | The chain |
| ---- | --------- |
| Snake | 2511, 2114 and 2115 (MVP and after MVP), then 1119 (real time, parked), then 2739 (the arcade template and a snake example). The web build works today; the Playdate build needs 3010 |
| A platformer | 1119, 1120, 1122, 1123 and 2231, 1124 and 2360 (all parked), then 1127 (the platformer and its template) |

Until then, the agent eval (2524, 2525) measures the end goal on
turn-based games, and 2511 and 2512 keep the lines a game's author owns
few. The small games of the gallery (2736–2738) and the roguelike
(2735) are the eval's reference answers once they are unparked.

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
| 2125 | A rollout bot: `Determinize`, and flat Monte Carlo on copies that never see the luck to come (split out of 2113) | game#0506 |
| 2126 | A playtest runner: K tries of each named bot, the game's measures, pass bands as data, `shuyi playtest` | game#0506 |
| 2127 | An autobalance search: the game's knobs change a stage, never the bots, until every bot is in its band (after 2126) | game#0509 |
| 2128 | A turn planner: a bot that plans its side's whole turn, several commands deep, on determinized copies (after 2125) | game#0506 |

The playtest bots (2125–2128, 2026-10-21) are on the game's Act 1
critical path: its persona bots (game#0506) and autobalance
(game#0509) wait for them, and every Act 1 battle is balanced with
them. 2128 came from game#0506's first real battle: one command at a
time, the personas couldn't coordinate their pieces and stalled. So they come with 2304 and 2305, ahead of the MVP's tickets.
2125 and 2126 wait for nothing and can run side by side; 2127 follows
2126. They run on a PC and don't wait for slicing (3009): its first
real user is the game's enemy AI on the Playdate (game#1051).

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
| 3007 | 1-bit sprites at bake time, hand-drawn overrides per target (after 2204 and 2229) |
| 3009 | Bots think in slices: the frame rate holds, the decision never changes (first user: the game's enemy AI on the Playdate, game#1051) |
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

On 2026-10-20 Nick put the game first, the way Jai is built for one game
and happens to handle many ([`decisions.md`](decisions.md)): "unless
these real-time work tickets overlap with visions of shuyi they should
be parked until visions of shuyi is complete". So the real-time work
(1119–1128, and the tickets built on it below) waits until Visions of
Shuyi launches, not only until the MVP. The game's 0060 still unparks
1119–1125, and a piece the game asks for through its `engine-change`
skill is split out and unparked alone.

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
when unparked, between the official runtime in an opt-in module
whose users bring their own Spine license, and a reader of our own (the
same day; Visions of Shuyi never uses Spine). The order, when unparked:

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

On 2026-10-20 a second comparison, with Unity, Unreal and the 2D engines
too, parked what the game doesn't need (phase 3, *The second
comparison*, has the rest). The order, when unparked:

- **Real time** (until the launch, like 1119–1128): 1119, with its
  input latch, then 1120. Animation clips for any game (2231) need
  nothing parked, then animated tiles and unit frames (2359). The
  tile-layer painter (2360) after 1120, 1124 and 2231, and before 1125
  and 1127. Drawing between steps (2361) after 1119 and 1120; camera
  feel (2362) after 1120; game speed and hit-stop (2123) after 1119;
  curves and a follower (2122) after 1119; collision queries (2124)
  after 1122; map objects as the game's types (2232) with 1124;
  positional sound (2363) and local multiplayer seats (2406) need
  nothing parked. The arcade template and snake (2739) after 1119.
- **Animation and sound:** cinematic tracks and markers (2364) after
  2109 and 2339; a beat and bar clock (2365) after 2341.
- **Accessibility and input:** captions for sounds (2366) and rumble
  (2408) need nothing parked.
- **Screens:** overlay screens that let the screen below run (2367),
  with its first user, 3036 or 1119.
- **Localization:** translator exchange files (2227) after 2302; the
  engine's screens machine-translated (2352) after 2351 and 2350; research on
  right-to-left and shaped scripts (2908) after 2301.
- **The debug view:** memory (2368) after 2315, 2316 and 3008.
- **Store services and the network:** leaderboards (3054) after 3036;
  presence (3055) after 3036 and 3037; HTTP requests (2601) with their
  first user; crash reports sent (3047) after 3046 and 2601; several
  peers in one headless run (2529) after 1140.
- **Export targets that cost money or need a yes:** phones' browsers
  (2407) after 2405; paid signing (3051) after 3004.

The same day Nick added three kinds of game. Like real-time strategy,
visual novels and detective games, they wait until after Visions of
Shuyi launches. The order inside each, when unparked:

- **Roguelikes:** field of view (2120) needs nothing; noise (2118),
  then the level generators (2119), then the example (2735).
- **A gallery of small games:** match-3 (2736) and the card battle
  (2737) in either order; the brick-breaker and the shooter (2738)
  after 1119, 1122 and 2739.
- **Hex grids:** 2121 after 2117, then 2740.

Video (2909) waits with the visual novels.

| Tickets | What | Unparked early if |
| ------- | ---- | ----------------- |
| 1119–1125 | Real time: fixed-step simulation, cameras, pointer and analog input, collision, animation, Tiled and LDtk, and a walkable town that starts a grid battle. Waits until Visions of Shuyi launches (2026-10-20) | the game's 0060 says the army walks around freely, or the game asks for a piece |
| 1126–1128 | Isometric, side view (platformer), optional physics. Wait until Visions of Shuyi launches (2026-10-20) | — |
| 2231, 2359–2363, 2122–2124, 2232, 2406, 2739 | Real time from the second comparison: clips for any game, animated tiles, the tile-layer painter, drawing between steps, camera feel, positional sound, curves, game speed and hit-stop, collision queries, map objects as the game's types, local multiplayer seats, and the arcade template with snake | the game asks for the piece (each ticket's Parked note says how) |
| 2364–2368, 2408, 2227, 2352, 2908 | Cinematic tracks, a beat clock, captions for sounds, overlay screens that let the screen below run, memory in the debug view, rumble, translator files, machine-translated engine screens, and research on right-to-left scripts | Nick asks |
| 3047, 3054–3055, 2601, 2529 | Crash reports sent, leaderboards, presence, HTTP requests, several peers in one headless run | Nick asks; 2529 with networking |
| 2407, 3051 | Phones' browsers as a way to play a web build; paid code signing | Nick decides (each ticket asks) |
| 2118–2121, 2735–2738, 2740 | Roguelikes, a gallery of small games, and hex grids | Nick asks, after the game launches (2026-10-20) |
| 2909 | Research: video playback | with the visual novels |
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
| 2103–2105, 2216–2217 | Animation beyond clips: particles drawn from a seed and the time, state machines over clips with direction sets and frame events, 2D skeletal animation in `Fx` (a `shuyi-bones` module), and DragonBones and Spine import (Spine through the official runtime, its users bringing their own Spine license, or a reader of our own: Nick picks when unparked) | Nick asks |
| 2106 | Paths for free movement: a navigation mesh built from tile collision, routes with clearance, and a path a mover follows | 1122 is unparked |
| 2322–2324, 2404, 3035 | Accessibility: an accessibility tree from the engine's views, a text size the player picks, a colour-vision and text-contrast check, hold or toggle and slower timings, and screen reader output through AccessKit | Nick asks |
| 2325 | A frame profiler in the debug view: phase times, spans, a graph, and a trace file | Nick asks |
| 3036–3037 | Store services: achievements and stats as data with a local backend, an unlock notice and an achievements screen, then Steam achievements, stats and Steam Cloud saves behind the `steam` feature | Nick asks (2026-10-20: "make sure steam achievements and cloud saves are covered by some ticket so it's not lost") |
| 2331 | A render graph on the wgpu renderer: passes as nodes, a game's own passes, recorded in parallel on native | 3034 is unparked |
| 3110 | 3D animation: several clips blended at once, and morph targets from glTF | 3D is unparked |
| 2807 | A custom domain for the website | Nick has a domain (2026-10-20: "can use custom domain later") |
| 2326–2328 | 2D effects on the wgpu renderer: screen passes (CRT, bloom, palette remap, vignette, flash), a game's own WGSL passes and sprite materials, and 2D lights with normal maps | 3034 is unparked |
