# ADR-0012: The GBA and the DS: target capabilities, indexed colour, two screens, and homebrew backends

- **Status:** Accepted; its tickets are parked
- **Date:** 2026-10-17
- **Related tickets:** 3017–3027; they build on 3001 (profiles), 3002
  (controls fit), 3003 (the platform boundary), 3005 (`no_std`), 3006
  and 3007 (1-bit, the pattern for indexed colour), 3008 (budgets), 2003
  (module metadata), 2204 (packs), 3103 (the 3D view)
- **Extends:** ADR-0008 (profiles, backends, controls, budgets);
  ADR-0009 §4 (module metadata); ADR-0011 §8 (3D per target)

## Context

Nick, 2026-10-17 (`docs/decisions.md`): "I want GBA, and DS ports", and
"not all hardware supports all features". ADR-0008 made a target a
profile, a backend and a device ticket, and checks that a game's
controls fit. It doesn't yet say what else a target's hardware can do.

### The hardware

From GBATEK (Martin Korth's public hardware reference), the BlocksDS
documentation, and Wikipedia's articles on each system; 3018 re-checks
each number before it is relied on.

- **Game Boy Advance** (2001):
  - an ARM7TDMI at 16.78 MHz, with no floating-point unit and no divide
    instruction;
  - 32 KB of fast internal RAM, 256 KB of slower external RAM, and
    96 KB of video RAM;
  - a 240 × 160 screen with 15-bit colour, at about 59.73 Hz;
  - tile backgrounds (up to four layers, with hardware scrolling, some
    of them rotatable and scalable) and 128 hardware sprites; colours
    come from palettes of 16 × 16 colours, or one of 256;
  - the d-pad, A, B, L, R, Start and Select;
  - saves in battery RAM, flash or EEPROM on the cartridge, up to
    128 KB; ROMs up to 32 MB;
  - two 8-bit sample channels and four simple tone channels; music is
    mixed in software.
- **Nintendo DS** (2004):
  - an ARM946E-S at 67 MHz for the game, and an ARM7TDMI at 33 MHz for
    sound, wireless and input;
  - 4 MB of RAM (16 MB on the DSi, whose ARM9 also runs at 133 MHz),
    and 656 KB of video RAM in banks;
  - **two 256 × 192 screens**, the lower one a resistive touchscreen;
  - two 2D engines, one per screen, each like an enhanced GBA's;
  - **a 3D engine** on one screen (both at 30 fps through a capture
    trick): up to 6 144 vertices a frame, so 2 048 triangles or 1 536
    quads; fixed-point geometry; textures, fog, toon shading, edge
    marking and alpha blending; no programmable shaders;
  - the d-pad, A, B, X, Y, L, R, Start, Select, the touchscreen, a
    microphone, and a lid that closes.

### Rust and the toolchains (crates.io and the projects' pages, 2026-10-17)

- **GBA:** `gba` 0.15.0 is **Zlib OR Apache-2.0 OR MIT**, so we take it
  under MIT or Apache-2.0. `agb` 0.25.0 is **MPL-2.0**, which ADR-0002
  §8 denies, so it is not used. Rust's GBA targets
  (`thumbv4t-none-eabi`, `armv4t-none-eabi`) are tier 3: they need
  nightly and `build-std`.
- **DS:** BlocksDS is a DS SDK whose `libnds` is Zlib, its default ARM7
  core's wireless library DSWiFi is MIT, and its C library picolibc is
  under MIT- and BSD-like licenses. Rust has a tier-3 `armv5te-none-eabi`
  target for the ARM9. A Rust `no_std` library linked against BlocksDS
  has been shown to work (a public blog post); no maintained Rust crate
  for the DS was found.
- **Neither ARMv4T nor ARMv5TE has atomic compare-and-swap**, so
  `alloc::sync::Arc` and anything using atomics don't build there.
- **Emulators** for CI exist (mGBA for the GBA, melonDS for the DS).
  They are tools run as separate programs, never linked or shipped.

### Selling

Nintendo no longer licenses either system. Games for them ship as
homebrew: a ROM for flash carts and emulators, or a cartridge from a
small publisher. The GBA's BIOS and the DS's firmware check for
Nintendo's logo in the ROM's header before they boot it, and the
toolchains' header tools write it in. That is a legal question, not a
technical one; 3018 puts it to Nick, with a lawyer's look advised.

## Decision

### 1. A profile says what its hardware can do; a module says what it needs

`TargetProfile` (ADR-0008 §1) gains `capabilities`:

- **`colour`**: `Rgba8`, `OneBit` (ADR-0008 §4), or
  `Indexed { banks, colours_per_bank, bits_per_pixel }` (§4);
- **`screens`**: a list of surfaces, each with its size and whether it
  is touch (§5);
- **`three_d`**: `none`, `software { max_triangles }`,
  `hardware { max_triangles, max_texture_bytes, effects }` or `full`
  (ADR-0011 §8);
- **`sprites`**: `None` (no limit) or `Some { max, max_per_line }`;
- **`memory`**: RAM and video RAM in bytes;
- **`audio`**: the channels, and whether music is streamed or mixed.

Each module's metadata (ADR-0009 §4) gains **`needs`**: the capabilities
it needs on a target, such as `three_d: software` or `colour: Rgba8`.
A game's own scenes can add needs too (a stage that shows 200 sprites
at once).

`shuyi check --target <id>` fails when a game turns on a module, or a
stage uses a feature, that its target can't run. The error names the
module or stage, what it needs, and what the target has. `shuyi export`
refuses, as for controls (ADR-0008 §3). An `optional` part (an HD-2D
effect, ADR-0011 §7) is left out and listed instead. The engine never
drops a required feature silently.

### 2. The GBA: profile `gba`, backend `shuyi-gba`

- **The profile:** 240 × 160, `Indexed { banks: 16, colours_per_bank:
  16, bits_per_pixel: 4 }` (or 256 colours in one bank, chosen per
  layer), `three_d: none`, sprites `{ max: 128, max_per_line: 32 }`
  (prov.; 3018 checks the per-line limit, which depends on sprite
  sizes), text 8 / 8 px, 60 / 30 fps.
- **Inputs:** the d-pad, A and B as `East` and `West` (as on the
  Playdate), `L1`, `R1`, `Start` and `Select`. No pointer, no touch,
  no keyboard.
- **The backend** `shuyi-gba` (`crates/gba`), `no_std` with `alloc`,
  over `gba`:
  - **glyph cells** go to one tile background, from a baked font tile
    set;
  - **2D world layers** go to the other backgrounds. Their cameras and
    parallax become the hardware scroll registers, so they cost nothing
    each frame;
  - **sprite items** go to hardware sprites, and rotated ones (2703)
    use the affine sprites;
  - **a frame that doesn't fit** the hardware (more layers than
    backgrounds, more sprites than slots) is a check error at bake time
    where it can be seen, and a counted, logged drop at run time where
    it can't;
  - **the frame driver** steps once per vertical blank. A tick stage
    runs 60 steps for about 59.73 real frames, so it is 0.45 % slow and
    stays exact;
  - **storage** maps save slots to battery RAM or flash, chosen in the
    game's export settings;
  - **audio** mixes in software into the sample channels, from samples
    and music baked for the GBA (§4);
  - **heap**: in external RAM; the internal RAM holds the stack and the
    hot code.
- **What it runs:** 2D stages of every 2D perspective, turn and tick
  simulations, and the 2D kits. No 3D, no HD-2D, no rigid-body physics
  in floats.

### 3. The DS: profile `nds` (and `dsi`), backend `shuyi-nds`

- **The profile:** two surfaces of 256 × 192 (the lower is touch),
  `Indexed` per 2D layer and `Rgba8`-like 15-bit colour for 3D,
  `three_d: hardware { max_triangles: 2048, max_texture_bytes: 512 KiB
  (prov.), effects: [fog, edge_marking, toon, alpha] }`, sprites
  `{ max: 128 }` per screen, text 8 / 10 px, 60 / 30 fps. The `dsi`
  profile differs in memory and speed.
- **Inputs:** the d-pad, A `East`, B `South`, X `North`, Y `West`, `L1`,
  `R1`, `Start`, `Select`, and a pointer on the lower surface. Closing
  the lid is a `SystemEvent::Suspend`.
- **The backend** `shuyi-nds` (`crates/nds`): the game and the engine
  run on the ARM9 in Rust, `no_std` with `alloc`, over BlocksDS's
  `libnds` through bindings we write for the part we use. The ARM7 runs
  BlocksDS's stock core, in C, unchanged.
  - 2D frames map to each screen's 2D engine as on the GBA (§2).
  - **The 3D view** (ADR-0011 §5) maps to the 3D engine: transforms
    are already 20.12, meshes are baked to the hardware's display lists
    and textures at bake time (2204), and billboards are textured
    quads. Over the triangle budget is a check error for baked scenes,
    and a counted drop at run time.
  - **HD-2D** runs reduced: terrain, billboards and fog, with edge
    marking for outlines. Depth of field, bloom and light shafts are
    effects the DS doesn't have, so a diorama skin must mark them
    `optional` to export there (§1).
- **What it runs:** everything the GBA runs, plus touch, two screens,
  3D within the triangle budget, and reduced HD-2D.

### 4. Indexed colour is baked, with a check, like 1-bit

The 1-bit work (ADR-0008 §4, 3006, 3007) has the pattern, and indexed
colour follows it:

- `shuyi bake --target gba` (and `nds`) quantizes each image to the
  target's palettes, in integers, deterministically: tiles of 8 × 8 at 4
  or 8 bits a pixel, and up to 16 banks of 16 colours shared across the
  layer or the sprites. Quantizing is median cut, then nearest colour in
  integer Rec. 601 distance, with ordered dither optional per image.
  ADR-0007 §4 holds: decoding the baked image is tested equal to
  quantizing the decoded source.
- Palette roles (ADR-0008 §4) map to fixed indices, so a role that must
  stay distinct does, and the same `distinct` groups are checked.
- `assets@gba/` and `assets@nds/` override any asset with hand-made art,
  as `assets@playdate/` does.
- Audio is baked too: samples to 8-bit at the target's rate, music to a
  tracker-style format the backend's mixer plays (3027 picks it, under a
  permissive license).

### 5. Two screens are two surfaces

- The frame may hold **more than one surface**. A game declares its
  surfaces and what each shows; by default there is one.
- On a target with that many screens, each surface goes to its screen.
  On a target with fewer, the profile's `layout` places them in one
  window: stacked, side by side, or one at a time with an action to
  switch, chosen by the player.
- A pointer on a surface is reported in that surface's coordinates
  (1121).
- A game that needs two surfaces visible at once declares it, and a
  one-at-a-time layout then fails the check.

### 6. Old ARM targets: nightly in CI, no atomics, no allocation in a frame

- CI builds `shuyi-core`, `shuyi-ui`, `shuyi-platform`, the pack reader
  and the backends for `thumbv4t-none-eabi` and `armv5te-none-eabi` with
  nightly and `build-std`, pinned to one nightly. Everything else stays
  on stable.
- **No atomics** in those crates: `alloc::rc::Rc` is allowed, `Arc` is
  not. The CI build is what enforces it.
- **No allocation in a steady frame.** On these targets a frame that
  allocates is a budget failure: the counting allocator of 3008 reports
  allocations per frame in the benchmark scenarios, and the budget for
  `gba` and `nds` is zero after the first frame of a stage. Arenas and
  buffers are sized when a stage starts.

### 7. Budgets: cycles in an emulator, then the device

- **The proxy in CI** for these two targets is **cycles counted by an
  emulator**, run headless as a separate program on the benchmark
  scenarios. The ARM7TDMI and ARM946E-S are simple enough that an
  accurate emulator's count is close to the device and is the same on
  every run. The emulator is a CI tool, never linked or shipped, so its
  own license (mGBA is MPL-2.0, melonDS is GPL-3.0) doesn't reach the
  engine. 3018 picks one per system, runnable without any Nintendo
  BIOS or firmware file.
- **The device tickets** (3022, 3026) measure on hardware through a
  flash cart, and set the profile's budgets from the measured ratio,
  as ADR-0008 §6 says.

| Target | Frame time on device | `frame_cycles` (emulator) | `peak_heap_bytes` | `package_bytes` | `startup_ms` | Also |
| ------ | -------------------- | ------------------------- | ----------------- | --------------- | ------------ | ---- |
| `gba` | 60 fps held where the game is animating; turn screens may run 30 | 200 k (prov.; of about 280 k a frame, leaving the rest to the mixer and interrupts) | 160 KB (prov., of 256) | 16 MB (prov., of 32) | 2 000 | no allocation in a steady frame; each enemy decision ≤ 2 s (prov.), thinking in slices (3009) |
| `nds` | 60 fps held; 3D on both screens at 30 | 700 k on the ARM9 (prov.; of about 1.1 M) | 2.5 MB (prov., of 4) | 64 MB (prov.) | 3 000 | ≤ 2 048 triangles a frame; textures within 512 KB (prov.); no allocation in a steady frame |

### 8. Homebrew, and nothing of Nintendo's in this repository

- **No Nintendo SDK** is used: the GBA backend uses `gba`, the DS
  backend BlocksDS, both public and permissive. So, unlike the Switch
  (ADR-0008 §2), these backends are public and live in this repository.
- **This repository never holds** Nintendo's logo, a BIOS, firmware, or
  anything from an official SDK. `shuyi export gba` and `shuyi export
  nds` call the header tools of the toolchain the developer installed,
  and say which. 3018 confirms which tools those are and that they need
  nothing from Nintendo's files.
- **Selling** a GBA or DS game is the developer's call, and for Visions
  of Shuyi it is Nick's (3018 asks him, with a lawyer's look advised).
  The engine's docs say plainly that these systems are not licensed by
  Nintendo any more and what that means.

## Consequences

- "Not every target runs every feature" becomes a check, not a surprise
  on the device. The same check grows with each new module and target.
- The GBA gets 2D games of every 2D perspective, at a cost: indexed art
  baked per target, tight memory, and no allocation in a frame.
- The DS gets 2D, touch, two screens, and real 3D in a budget, because
  the 3D space was chosen to be its native format.
- The no-allocation rule helps every small target, the Playdate
  included.
- Two nightly-only CI builds join the gates. They are pinned, so a
  nightly change can't break `main` by surprise.
- Writing our own DS bindings costs some work, because no maintained
  Rust crate exists. Only the parts the backend uses are bound.
- The emulator proxy is closer to the device than Cachegrind's x86
  counts are, so these budgets should need less calibration.

## Alternatives considered

- **`agb` for the GBA** — the more complete Rust GBA library, with
  sound, saves and image tools, but MPL-2.0, which ADR-0002 §8 denies.
- **devkitPro's libnds instead of BlocksDS** — long the standard, but
  BlocksDS's licensing is documented per component and is all
  permissive, and its build doesn't need devkitPro's installer. Either
  produces the same ROMs.
- **One profile per feature set, without capabilities** — a target
  would be checked only for controls and text, and a 3D game would
  export to the GBA and fail on the device.
- **Make the engine degrade features by itself** (drop 3D on the GBA,
  turn HD-2D into flat 2D) — hidden changes a developer can't predict.
  It breaks the rule Nick set for the Playdate: what doesn't fit doesn't
  ship, unless the game marks it optional.
- **Floats on these targets** — no FPU, and determinism needs integers
  anyway (ADR-0003 §3).
- **Cachegrind instruction counts, as for other targets** — an x86
  count says little about a 16 MHz ARM7 with slow external memory. An
  emulator's cycle count is exact and much closer.
- **A Rust re-implementation of the DS's ARM7 core** — a lot of work
  for code no game touches. The stock core works and is permissive.
