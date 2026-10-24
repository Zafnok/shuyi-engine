---
id: "3021"
title: "Parked: shuyi-gba: the GBA backend (tile backgrounds, hardware scroll, sprites, a frame per vertical blank, saves) and shuyi export gba"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3018", "3003", "3004", "3017", "3019", "3020", "3045"]
nick_input: none
completed:
---

# 3021 — The GBA backend

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§2 makes the GBA a profile, `gba`, and a backend crate, `shuyi-gba`,
over the `gba` crate (0.15.0, **Zlib OR Apache-2.0 OR MIT** on
crates.io on 2026-10-17; taken under MIT or Apache-2.0; `agb` is
MPL-2.0 and denied). The hardware (ADR-0012, *The hardware*; the
numbers as 3018 re-checked them in `docs/targets/gba.md`):

- an ARM7TDMI at 16.78 MHz, with no FPU and no divide instruction;
- 32 KB of fast internal RAM, 256 KB of slower external RAM, 96 KB of
  video RAM;
- 240 × 160, 15-bit colour, about 59.73 Hz;
- up to four tile backgrounds with hardware scroll, 128 hardware
  sprites, palettes of 16 × 16 or 256;
- the d-pad, A, B, L, R, Start and Select;
- saves in battery RAM or flash on the cartridge.

This ticket builds on:

- `App::frame` and the `Platform` trait (3003);
- `shuyi export` (3004);
- the capabilities and the planned `gba` profile (3017);
- images, glyphs and palettes baked to tiles (3019);
- the pinned nightly, no atomics, and the emulator runner (3020);
- the power-cut suite every backend's storage runs (3045);
- the toolchain, emulator and header tool 3018 chose.

**Audio is split out** to 3027 (samples and music baked for the GBA and
a software mixer), to keep this one PR. Until 3027 lands, `audio()`
returns a silent `Audio`.

## Nick input

None. Selling is 3018's question.

## Scope

**In:**
- **The `gba` profile** (`targets/gba.ron`, written `planned` by 3017):
  check every value against `docs/targets/gba.md` and fix what differs.
  Inputs: the d-pad, A `East`, B `West` (as on the Playdate), `L1`,
  `R1`, `Start`, `Select`; no pointer, touch or keyboard. It stays
  `planned` until 3022.
- **The crate `shuyi-gba`** (`crates/gba`), `#![no_std]` with `alloc`,
  implementing `Platform` (3003), with a README and its catalog row
  (2003 wrote it, parked):
  - **`check-modules`.** **Either order with 3010:** if 3010 is done,
    the check already takes a crate whose row is in `docs/modules.md`'s
    *Platform backends* table as a backend (its README and row, no
    `[package.metadata.shuyi]`): give this crate no metadata. If not,
    teach the check that here (unless 3024 did), with a fixture, as
    3010 says;
  - **glyph cells** go to one tile background (BG0), from the font tile
    set baked by 3019. 240 × 160 is 30 × 20 cells of 8 × 8. Each cell's
    foreground and background roles pick a palette bank. More role pairs
    than free banks is a bake-time check error;
  - **2D world layers** (1120) go to BG1–BG3 in mode 0. Each layer's
    camera and parallax become its scroll registers, written in the
    vertical blank. A map larger than a screen block is streamed one row
    or column at a time as the camera moves. Each background shows one
    tile-layer item (2360): its tiles become screen-block entries with
    the hardware's horizontal and vertical flip bits; Tiled's diagonal
    flip, which the hardware lacks, becomes a transposed copy of the
    tile made at bake time and counted in its group's tiles (3019); an
    animated tile rewrites its entries when its frame changes. Sprite
    items in world layers stay hardware sprites. **Either order with
    1120:** if 1120 is done, map its world layers as above; if not,
    2360 isn't done either (it is blocked by 1120), so leave world
    layers and tile layers out, draw the cells and the sprite items, and
    write one follow-up ticket blocked by 1120 and 2360 that maps both,
    with the viewport check below and the scroll-register test. If 1120
    is done and 2360 isn't, leave tile layers out and write a follow-up
    ticket blocked by 2360;
  - **sprite items** go to hardware sprites (OAM), from the baked
    images, through a shadow OAM copied by DMA in the vertical blank.
    **Either order with 2335:** if 2335 is done, rotated sprites use
    the affine sprites and their 32 matrices. If not, sprites have no
    angle yet: leave the affine sprites out, and write a follow-up
    ticket blocked by 2335 for them and their limit of 32 (or add the
    GBA to the one 3024 wrote);
  - **a frame that doesn't fit** the hardware: more tile-layer items
    than backgrounds, a world layer whose viewport (1120) isn't the
    whole screen, more sprites than 128 or than the per-line limit
    (3018's table), more affine items than 32. Where it can be seen at
    bake time (a stage's declared needs, 3017, and the tile-layer items
    its map gives, 2360), it is a check error. For the layers, this
    ticket adds `capabilities.tile_layers: Option<u8>` (3 on `gba`,
    because BG0 holds the cells; `None` elsewhere) and a `tile_layers`
    need that a stage gets from its map (one per shown tile layer, as
    2360's `MapTiles::all` builds them) or declares. If the DS backend
    (3024) landed first, it added the field and a drawing part shared
    by both backends; reuse them. At run time, the extra items are
    dropped, counted in a `PresentStats`, and logged once per frame
    through the emulator's debug output;
  - **the frame driver**: the main loop halts until the vertical blank,
    then calls `App::frame` with `now` = vertical blanks × 1000 / 60 ms,
    in integers. So a tick stage takes exactly one step per frame, runs
    0.45 % slow, and stays exact (ADR-0012 §2);
  - **input**: `KEYINPUT` read once per frame into `RawInputEvent`s;
  - **storage**: save slots and settings in battery RAM or flash,
    chosen in the game's export settings (`targets.gba.save = "sram" |
    "flash64" | "flash128"`). Each write goes to the other of two slots
    with a sequence number and a checksum, and loading takes the newest
    valid one, so a power cut mid-write loses only that write. The fake
    save memory (below) is a `PowerCutRig`, and the storage passes
    3045's `power_cut_suite` over it. Use
    `gba`'s save support if it has it (3018 says), or write the flash
    command sequences here;
  - **memory**: the heap in external RAM, through the `no_std`
    allocator 3018 chose, guarded by turning interrupts off (no
    atomics). The stack and the hot drawing code (ARM code in a
    `.iwram` section) live in internal RAM;
  - **assets**: the baked pack (3019) is in ROM, which the CPU reads in
    place. The pack reader (2204) reads it from a `&'static [u8]`, and
    tile and palette data go to video RAM by DMA straight from ROM, so
    `bake --target gba` stores them uncompressed;
  - **suspend**: none (the GBA has no sleep the game must handle);
    nothing sends `Suspend`.
- **A fake hardware layer.** The crate talks to the hardware through a
  small internal trait (registers, OAM, video RAM, save memory). A fake
  behind it runs on the host, and the host tests use it, as 3010 does
  for the Playdate SDK.
- **`shuyi export gba`** (an `Exporter`, 3004):
  1. it runs `check --target gba`, and refuses on any error;
  2. it builds the game for the target 3018 chose, with the pinned
     nightly and `-Z build-std=core,alloc`;
  3. it bakes the pack for `gba` and puts it in the ROM;
  4. it turns the ELF into a ROM image, and calls the installed
     toolchain's header tool, saying which tool it ran;
  5. it reports the ROM's size against `package_bytes` (16 MB, prov.).

  A missing nightly, `rust-src` or header tool gives the install steps
  from `docs/targets/gba.md`.
- **CI**: `shuyi-gba` joins 3020's `thumbv4t-none-eabi` build; `hello-grid`
  is exported for `gba` on every PR.
- **One example on the GBA.** `hello-grid` (1118) gets a `gba` keymap
  layout, a role file (3019) and a frame that fits 30 × 20 cells, and it
  exports. In the emulator 3018 chose:
  - the ROM plays a replay fixture with Harness-scripted input, and its
    final state hash equals the desktop run's;
  - one `frame-png --target gba` of a screen is compared with the
    emulator's screenshot of the same frame, by eye in the PR.
- **Docs.** `docs/targets/gba.md` gets how to export, how to run in an
  emulator and on a flash cart, and what is known not to work yet.

**Out (do not do):**
- **anything of Nintendo's in this repository**: no logo bytes, BIOS,
  firmware or official SDK, in code, fixtures or docs. The header tool
  is the developer's installed one;
- audio (3027);
- affine and bitmap background modes (write a ticket when a game needs
  rotated backgrounds);
- a world layer's viewport through the hardware windows, and split
  screen by changing scroll registers mid-frame (write a ticket when a
  game needs either on the GBA);
- 3D, HD-2D and rigid-body physics (the profile says `three_d: None`);
- link-cable play;
- measuring on a device (3022).

## Implementation steps

1. Re-read `docs/targets/gba.md` and the `gba` crate's docs at its
   current version. Fix the profile.
2. Write the internal hardware trait, the real one over `gba`, and the
   fake.
3. Write `present`: cells, then world layers and scroll, then sprites,
   with the overflow counting. Test each against the fake.
4. Write the frame driver, input, storage and memory.
5. Write the exporter, and add the CI build and export.
6. Export `hello-grid`, run the replay in the emulator, and fix what's
   found.
7. Write the docs. Update the catalog row.

## Acceptance criteria

- [ ] `shuyi export gba` on `hello-grid` produces a ROM, and its size is
      in the report.
- [ ] The ROM's replay hash in the emulator equals the desktop run's (CI).
- [ ] Host tests with the fake hardware:
  - [ ] a camera moving 3 px writes 3 to the scroll register, and touches
        no tile (if 1120 isn't done, the world-layer follow-up has it);
  - [ ] 130 sprite items show 128, and `PresentStats` counts 2 dropped;
  - [ ] the storage passes `shuyi_ui::storage::power_cut_suite` (3045)
        over the fake save memory, with a cut at any byte or sector
        operation, and after a cut mid-write the older slot loads;
  - [ ] 60 vertical blanks step a tick stage exactly 60 times.
- [ ] A fixture stage with four tile layers fails `shuyi check --target
      gba`, naming the stage and both numbers.
- [ ] `shuyi-gba` builds in 3020's `thumbv4t-none-eabi` job.
- [ ] `cargo deny check` passes with `shuyi-gba` and `gba`.
- [ ] No Nintendo logo bytes, BIOS or firmware are in the repository.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the cell, layer and sprite mapping; overflow counting; the frame
  driver's timing; storage; all against the fake hardware.
- Integration: the export and the replay in the emulator, in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
