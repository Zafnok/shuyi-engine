# ADR-0008: Export targets: profiles as data, platform backends, controls that fit, 1-bit graphics, performance budgets

- **Status:** Accepted; extended by ADR-0057 (§4's "an image's sidecar"
  is its import settings file, `<image>.meta`); extended by ADR-0058
  (export packages: paths remapped and searched, the Linux floor)
- **Date:** 2026-10-15
- **Related tickets:** 3001–3016; game 1025 (the parked Playdate port),
  game 0903 (Steam readiness)
- **Extends:** ADR-0002 §1 (crates), §2 (determinism), §4 (skins), §5
  (actions); ADR-0007 §4 (derived formats)

## Context

Nick, 2026-10-15 (`docs/decisions.md`): Visions of Shuyi should run on
the Playdate and the Steam Deck, and eventually on the Nintendo eShop,
and maybe on mobile and other consoles. On the Playdate he set three
conditions:

- the graphics must translate to 1-bit easily;
- it must run at a good frame rate;
- it must play with the d-pad, A, B and the crank only. If any screen
  needs more inputs than that for something required, the game can't
  ship on the Playdate.

He also asked for expected performance benchmarks per target.

### What the game does today

These figures come from the game repository on 2026-10-15:

- **The frame.** 100 × 32 cells of 8 × 16 px, so 800 × 512 logical
  pixels (`crates/ui/src/console.rs`).
- **The actions.** It has 18. Seven are required: four cursor
  directions, Confirm, Cancel and End turn (`crates/content/src/keymap.rs`,
  `Action::is_required`).
- **Colour.** Sides, unit states, danger zones and terrain are told apart
  by colour (game ticket 1025, *No colour*).
- **Earlier Playdate thoughts.** Nick, 2026-10-07 (game 1025): "chording
  A+B if we need to", and "the rewind mechanic can use the winding
  mechanism on the side".

### The targets

Facts checked on 2026-10-15. Each one's source is given, and the tickets
re-check them before they rely on them.

- **Playdate** (Panic):
  - **Screen.** 400 × 240 px, a 1-bit memory LCD. Its refresh rate is
    variable, up to 50 Hz, and the SDK's default is 30.
  - **Chip and memory.** A Cortex-M7, run at 168 MHz by most accounts.
    Some sources say 180 MHz, and Rev B has slower memory. It has 16 MB
    of RAM.
  - **Inputs.** The d-pad, A, B, the crank, the accelerometer, and the
    system's Menu button. The system menu can hold about three of the
    game's own items, according to developers; Panic's documentation
    wasn't checked.
  - **No standard library.** C games don't get one, so Rust there is
    `no_std` with `alloc`. The Rust bindings `playdate` (boozook) are
    MIT, and `crankstart` is MIT.
  - **Selling.** Games can be sold on itch.io and sideloaded, or
    submitted to Panic's curated Catalog. The SDK's license is Panic's
    own.

  Sources: Wikipedia, *Playdate (console)*; the Playdate Community Wiki;
  Panic's help pages *Distributing Playdate games* and *Sideloading*; the
  `github.com/boozook/playdate` README.
- **Steam Deck** (Valve):
  - **A Linux PC.** It has a 1280 × 800 screen.
  - **Deck Verified asks for:**
    - text at least 9 px tall at 1280 × 800, with 12 px recommended;
    - a default controller layout that reaches all content;
    - the on-screen keyboard where the player types.

  Source: Steamworks documentation, *Steam Deck Compatibility Review
  Process*.
- **macroquad**, the engine's window layer, runs on Windows, Linux,
  macOS, the web, Android and iOS (its README).
- **Consoles** (Nintendo, Sony, Microsoft):
  - Their SDKs and tools are under non-disclosure agreements, and their
    code may not be published.
  - Registering on Nintendo's developer portal costs nothing, but the
    hardware does: a Switch development kit cost about 50,000 yen in
    2017, and access to Switch 2 kits has been limited (press reports,
    2025).
  - Microsoft's GDK needs MSVC.

The engine's layers already make a port possible:

- the core is pure;
- screens build plain-data views and a skin paints them;
- only `shuyi-app` touches devices (ADR-0002).

Four things are missing:

- a way to describe a target;
- a second backend;
- a check that a game fits a target;
- numbers to measure against.

## Decision

### 1. A target is data: `TargetProfile`

The engine describes every export target in a RON file under `targets/`
(`targets/playdate.ron`, `targets/steam-deck.ron`, …). The files are
read by `shuyi_assets::target::TargetProfile`, an asset kind with its own
`Format` (ADR-0007). A game may add a profile of its own, or override one
of these in its bundle. A profile holds:

- **`id` and `status`.** `status` is `supported` once the target's
  example runs on the device, and `planned` before that.
- **`display`.** Width and height in device pixels, `color`
  (`Rgba8` or `OneBit`), `min_text_px` (an error below this),
  `good_text_px` (a warning below this), and `safe_area` insets.
- **`frame_rate`.** `target` and `floor`, in frames a second.
- **`input`.** What the player can press, used by §3:
  - `buttons`: positions, as the engine's `Button` names them;
  - `chords`: pairs of buttons that count as one more input;
  - `axes`;
  - `system_menu_slots`;
  - whether a pointer, touch and a keyboard exist.
- **`budgets`.** §6.
- **`backend`.** Which platform backend builds it (§2).

These are the profiles the engine ships, with the budgets that are
provisional until measured (§6):

| Target | Display | Colour | Text (min / good) | Frame rate (target / floor) | Inputs | Backend |
| ------ | ------- | ------ | ----------------- | --------------------------- | ------ | ------- |
| `desktop` | any, integer scale | RGBA | 10 / 12 px | 60 / 60 | keyboard, gamepad, pointer | `shuyi-app` |
| `web` | browser canvas | RGBA | 10 / 12 px | 60 / 30 | keyboard, gamepad, pointer | `shuyi-app` |
| `steam-deck` | 1280 × 800 | RGBA | 9 / 12 px | 60 / 60 | gamepad only (`South East West North`, d-pad, `L1 R1 L2 R2`, `Start Select`, two sticks); touch exists but nothing may need it | `shuyi-app` |
| `playdate` | 400 × 240 | 1-bit | 8 / 10 px | 30 / 30 | d-pad, A, B (`East`, `West`), chord A+B, crank (an axis, and `CrankForward` / `CrankBack` per detent); 3 system menu slots | `shuyi-playdate` |
| `switch` | 1280 × 720 handheld, 1920 × 1080 docked | RGBA | 12 / 16 px | 60 / 60 | to be filled by 3014 | private |
| `android`, `ios` | many; safe areas | RGBA | 12 / 16 px | 60 / 30 | touch (on-screen controls), gamepad optional | `shuyi-app` |

`switch`, `android` and `ios` are `planned`, and their tickets are
parked. PlayStation and Xbox have no profile until research ticket 3016
reports.

### 2. Platform backends: the engine drives a frame, the backend owns the loop

- **A new crate, `shuyi-platform`.** It sits between `shuyi-ui` and the
  backends. It defines the `Platform` trait and `App`, which runs one
  frame: it turns raw input into actions, steps the screens and the tick
  driver, builds the frame, and hands it to the backend.
  `App::frame(&mut self, platform: &mut impl Platform, now: Millis)` is
  called by the backend's own loop:
  - macroquad's async loop;
  - the Playdate SDK's update callback;
  - a console's main loop.

  The clock is read only by backends, and time is an argument
  (ADR-0002 §2).
- **The trait** (the shape; 3003 fixes the names):

  ```rust
  pub trait Platform {
      fn display(&self) -> DisplayInfo;              // pixels, colour, scale
      fn present(&mut self, frame: &Frame);          // RGBA or 1-bit (§4)
      fn poll_input(&mut self, out: &mut Vec<RawInputEvent>);
      fn system_events(&mut self, out: &mut Vec<SystemEvent>); // suspend, resume, focus, quit asked, low battery
      fn storage(&mut self) -> &mut dyn Storage;     // save slots and settings, by key
      fn audio(&mut self) -> &mut dyn Audio;
      fn text_input(&mut self) -> Option<&mut dyn TextInput>; // the system keyboard, if there is one
  }
  ```
- **The backends:**
  - **`shuyi-app`** stays the macroquad backend, with gilrs and audio,
    for desktop, the web and the Steam Deck. Android and iOS come later.
    A game that depends on `shuyi-app` today keeps doing so.
  - **`shuyi-playdate`** is the Playdate backend, `no_std`, over the
    MIT `playdate` crates.
  - **Console backends** are private crates, one per console, in private
    repositories under that console's agreement. They are never in this
    repository. The engine's part is to keep `Platform` small, stable and
    documented, so that a backend is a few hundred lines against an SDK.
- **The rule changes.** ADR-0002 §1's "only `shuyi-app` touches devices"
  becomes "only platform backends touch devices". `AGENTS.md` rule 5 says
  so.

### 3. Controls must fit, or the game doesn't ship on that target

- **Screens declare their actions.** Each screen declares the actions it
  reacts to (`Screen::actions()`), and a Harness check fails if a screen
  reacts to an action it didn't declare.
- **The check.** For each target, the required actions a screen declares
  must bind to *distinct* inputs of that target's profile. The inputs
  are the profile's buttons, its chords and its discrete axis inputs
  (such as `CrankForward`). `shuyi check --target <id>` reads the game's
  keymap layout named after the target (for example `playdate`) and
  fails if:
  - a required action has no input in that layout;
  - two required actions of one screen share an input;
  - an input isn't in the profile.

  The error names the screen and the actions that don't fit.
  `shuyi export <id>` refuses to export a game that fails.
- **What doesn't count:**
  - long presses and double presses;
  - chords the profile doesn't list;
  - the system menu, which may hold optional actions only;
  - the Playdate's Menu and Lock buttons and the Steam Deck's Steam and
    Quick Access buttons, which belong to the system.

  A game that needs more than its target offers has to redesign the
  screen: a context menu on A, or "Cancel with nothing to cancel opens
  the menu". The engine never drops a required action to make a game
  fit.
- **The same check covers Deck Verified's "default controls reach all
  content"**, with the `steam-deck` profile, and later mobile's on-screen
  controls.

For Visions of Shuyi today, the seven required actions fit the
Playdate: four directions, A, B, and End turn on A+B or the crank. The
game decides which (game 1025).

### 4. Colour depth is part of the skin; 1-bit has rules

- **Skins name colours by role.** A skin's palette file maps roles to
  colours: `team.player`, `team.enemy`, `tile.danger`, `text`, …
- **A 1-bit target adds a pattern map.** It maps each role to one of a
  fixed set of 8 × 8 patterns:
  - solid black and solid white;
  - 25, 50 and 75 % ordered dither;
  - two diagonals;
  - horizontal lines;
  - each of these inverted.
- **Groups that must stay distinct.** The palette file lists groups of
  roles that tell things apart, such as `distinct: [team.player,
  team.enemy, team.ally]`. `shuyi check --target playdate` fails if two
  roles of one group get the same pattern. Colour alone can't carry
  meaning on 1-bit, and this check is how the engine says so.
- **The frame becomes 1-bit in two ways:**
  - **Cells and rectangles** use their role's pattern. A colour with no
    role uses 8 × 8 Bayer ordered dithering of its luminance (integer
    Rec. 601 weights, 0–255).
  - **Sprites are made 1-bit at bake time, not every frame.** `shuyi
    bake --target playdate` dithers each image with Atkinson error
    diffusion in integers. An image's sidecar may choose `dither:
    atkinson | bayer | threshold(N) | none` and an alpha cut. ADR-0007
    §4 holds: decoding the baked 1-bit image is tested equal to
    dithering the decoded source.
- **Hand-drawn art wins.** A bundle layer per target (`assets@playdate/`)
  overrides any asset for that target, so an artist can redraw what
  dithering spoils.
- **Pictures.** `frame-png --target playdate` renders exactly what the
  device would show. Tests of a look use it, and tests of behaviour still
  read scenes (ADR-0002 §7).

### 5. Small targets get `no_std` crates

- **The crates.** `shuyi-core`, `shuyi-ui`, `shuyi-platform` and
  `shuyi-assets`' pack reader build with `default-features = false` as
  `#![no_std]` with `alloc`, for `thumbv7em-none-eabihf`. CI builds them
  for that target on every PR, and that build is a gate.
- **The rules this brings:**
  - maps are `BTreeMap` (determinism wants them anyway) or `hashbrown`
    with a fixed hasher;
  - errors use `thiserror` 2 without its `std` feature;
  - no `std::` paths outside `cfg(feature = "std")`.
- **Text stays on big machines.** The text decoders (RON, PNG, `.dlg`,
  Tiled) may stay `std`-only. A `no_std` target loads only baked packs
  (2204).

### 6. Budgets: deterministic proxies in CI, real devices for the truth

- **Each profile's `budgets`:**
  - `frame_instructions`: the most instructions one frame of the engine's
    work may take in the heaviest benchmark scenario;
  - `peak_heap_bytes`;
  - `package_bytes`;
  - `startup_ms`.
- **Proxies, in CI.** `shuyi bench` (3008) plays benchmark scenarios
  headlessly. Scenarios are Harness scripts over the examples, and a game
  writes its own. It measures:
  - instructions per frame, under Cachegrind on Linux, which counts the
    same on every run;
  - peak heap, through a counting allocator;
  - package size.

  The CI gate compares each against every `supported` profile. Wall
  clock time is never a gate, because it changes from run to run.
- **Devices, by hand.** A ticket per target (3011, 3013) runs the same
  scenarios on the device and records the results in
  `docs/targets/<id>.md`:
  - frame time p50 and p99;
  - dropped frames;
  - memory;
  - start-up time;
  - power, where the device shows it.

  It then sets the profile's instruction budget from the measured ratio.
  A budget changes only in a PR that carries a device measurement.
  Raising one to make a slow change pass is weakening a gate (`AGENTS.md`
  rule 6).
- **Bots think in slices.** A bot may spread its thinking over many
  frames (3009), counted in work units, never in time. It reaches the
  same decision on a 168 MHz Playdate as on a desktop; it only reaches it
  later. That keeps replays and lockstep exact while the frame rate holds.

The budgets to start from. Everything marked *prov.* is a starting
guess until its device ticket measures it:

| Target | Frame time on device | `frame_instructions` | `peak_heap_bytes` | `package_bytes` | `startup_ms` | Also |
| ------ | -------------------- | -------------------- | ----------------- | --------------- | ------------ | ---- |
| `desktop` | p99 ≤ 16.7 ms | 20 M (prov.) | 512 MB | — | 2 000 | — |
| `web` | p99 ≤ 16.7 ms in Chrome on a mid-range laptop | 20 M (prov.) | 256 MB | 25 MB download (prov.) | 5 000 at 25 Mbit/s | — |
| `steam-deck` | p99 ≤ 16.7 ms, no frame over 33 ms | 20 M (prov.) | 1 GB | — | 3 000 | ≤ 8 W device draw on an idle battle map (about 5 hours on the 40 Wh LCD model; prov.) |
| `playdate` | 30 fps held: p99 ≤ 33.3 ms, no frame over 50 ms, on Rev A and Rev B | 2 M (prov.: half of 168 MHz ÷ 30, leaving the rest to the SDK, the display and audio) | 10 MB (prov., of 16) | 40 MB (prov.) | 5 000 | while a bot thinks, frames still hold 30 fps; each enemy decision ≤ 1 s; only changed rows are sent to the display |
| `switch` | p99 ≤ 16.7 ms docked and handheld | — (3014) | — (3014) | — | — | — |
| `android`, `ios` | p99 ≤ 16.7 ms on a four-year-old mid-range phone | 20 M (prov.) | 300 MB | 150 MB | 3 000 cold | — |

## Consequences

- **The Playdate rule can't be broken by accident.** Every new screen
  is checked against every supported target's controls, so a game finds
  out when it adds an eighth required action to one screen, not on the
  device.
- **1-bit becomes a skin decision with a check.** It is no longer a
  port-time surprise. The cost is a pattern map per game, and
  hand-drawn overrides for art that dithers badly.
- **Ports become small.** A new target is a profile, a backend and a
  device ticket. A closed console's backend lives privately and reuses
  everything else.
- **`no_std` costs the core a little.** No `HashMap` from `std`, `alloc`
  imports, and a CI target to keep green. It is cheapest from the start,
  which is why the gate comes early (3005).
- **The CI numbers are only proxies.** An x86 instruction count is not
  a Playdate frame time. The device tickets calibrate them, and until
  then the budgets are labelled provisional.
- **`shuyi-app` keeps its name and job**, so the game's dependency
  doesn't change, but the run loop moves out of it into
  `shuyi-platform` (3003).

## Alternatives considered

- **Port to the Playdate with Lua or C, sharing only data** — the rules
  would be written twice and drift, and replays wouldn't match across
  platforms.
- **Dither every frame at run time, sprites included** — it costs the
  Playdate's CPU every frame and gives artists no control. Dithering
  only what has no role and baking sprites is cheaper and can be
  overridden.
- **Let the engine fold actions onto fewer buttons by itself** (long
  presses, automatic chords) — hidden controls players can't find, and
  it breaks Nick's rule that a game which doesn't fit doesn't ship. The
  check fails instead, and the game redesigns the screen.
- **Wall clock benchmarks as the CI gate** — they flake on shared
  runners. Instruction counts are exact, and the devices give the real
  times.
- **Steam Input action sets for the Deck** — Steam-only, and our actions
  already rebind. The game's 0903 may still choose it for the Steam
  build.
- **Console code in this repository behind features** — the SDK
  agreements forbid publishing it.
- **One backend crate with a feature per platform** — `no_std` Playdate
  code and `std` macroquad code in one crate means a tangle of `cfg`s.
  Separate crates keep each backend readable.
