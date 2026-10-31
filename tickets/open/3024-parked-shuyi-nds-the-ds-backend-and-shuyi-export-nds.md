---
id: "3024"
title: "Parked: shuyi-nds: the DS backend (Rust on the ARM9 over BlocksDS, two 2D engines, touch, the lid, saves, sound effects) and shuyi export nds"
type: feature
milestone: "Parked: export targets"
model: opus-5.5
effort: xhigh
status: blocked
blocked_by: ["3018", "3003", "3004", "3017", "3019", "3020", "3023", "3045"]
nick_input: none
completed:
---

# 3024 — The DS backend

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§3, its 2D part. The DS (ADR-0012, *The hardware*; as 3018 re-checked
it in `docs/targets/nds.md`):

- an ARM946E-S at 67 MHz for the game, and an ARM7TDMI at 33 MHz for
  sound, wireless and input;
- 4 MB of RAM (16 MB on the DSi, whose ARM9 also runs at 133 MHz), and
  656 KB of video RAM in banks;
- two 256 × 192 screens, the lower one a resistive touchscreen;
- two 2D engines, one per screen, each like an enhanced GBA's;
- the d-pad, A, B, X, Y, L, R, Start, Select, the touchscreen, a
  microphone, and a lid that closes.

ADR-0012 decides how the backend is built:

- the game and the engine run on the ARM9 in Rust, `no_std` with
  `alloc`, over BlocksDS's `libnds` (Zlib) through **bindings we write
  for only the part we use**, because no maintained Rust crate for the
  DS exists;
- the ARM7 runs BlocksDS's stock core, in C, **unchanged**;
- 2D frames map to each screen's 2D engine as on the GBA (§2);
- closing the lid is a `SystemEvent::Suspend`.

It builds on:

- `App::frame` and `Platform` (3003), `shuyi export` (3004);
- the planned `nds` and `dsi` profiles (3017);
- images baked to tiles and palettes (3019);
- the pinned nightly and the emulator runner (3020);
- two surfaces and pointers per surface (3023);
- saves that survive a power cut, and the suite that checks them (3045);
- BlocksDS, the emulator and `ndstool` as 3018 chose them.

**3D is not in this ticket** (3025).

## Nick input

None. Selling is 3018's question.

## Scope

**In:**
- **The profiles** `targets/nds.ron` and `targets/dsi.ron` (written
  `planned` by 3017): check every value against `docs/targets/nds.md`.
  Inputs: the d-pad, A `East`, B `South`, X `North`, Y `West`, `L1`,
  `R1`, `Start`, `Select`, and a pointer on the lower surface. They stay
  `planned` until 3026.
- **The crate `shuyi-nds`** (`crates/nds`), `#![no_std]` with `alloc`,
  for `armv5te-none-eabi`, implementing `Platform` (3003), with a
  README and its catalog row (2003 wrote it, parked):
  - **`check-modules`.** **From 3010 (done):** the check already takes
    a crate whose row is in `docs/modules.md`'s *Platform backends*
    table as a backend, needing its README and that row, not
    `[package.metadata.shuyi]` (`backends` in
    `crates/xtask/src/modules.rs`, with fixtures): give this crate no
    metadata;
  - **bindings**: `crates/nds/src/sys.rs`, hand-written `extern "C"`
    declarations for the `libnds` functions and registers the backend
    calls, and nothing else. Each one names the `libnds` header it comes
    from. If a binding generator is used once to draft them, its license
    is checked and its output is reviewed by hand;
  - **video RAM banks**: one fixed mapping, documented in the README,
    for the main and sub engines' backgrounds and sprites. It leaves
    the banks the 3D engine needs (3025) free;
  - **two surfaces** (3023): the game's first surface goes to the main
    engine and the second to the sub engine. Which one is on the top
    screen is a game setting. A game with one surface uses the top
    screen, and the lower one shows nothing;
  - **each 2D engine** draws as ADR-0012 §2 says for the GBA: glyph cells on one
    background, world layers on the others with hardware scroll (each
    background showing one tile-layer item from 2360, mapped as 3021
    maps it), sprite items as hardware sprites (affine when rotated),
    and the same overflow rules (check error at bake time, counted drop
    at run time). If 2360 isn't done, write a follow-up ticket for the
    world layers, as 2360's Out says, and draw the cells and the sprites
    here. Either order works with the GBA backend: if 3021 has landed, share
    its code where the hardware is the same, through a small crate both
    use, and reuse its `capabilities.tile_layers` (3 per engine here);
    if not, write the shared part and the field here, and 3021 reuses
    them. Record which in the PR. **Either order with 2335:** if 2335
    is done, rotated sprites are affine sprites. If not, sprites have
    no angle yet: leave the affine sprites out, and write a follow-up
    ticket blocked by 2335 for them (or add the DS to the one 3021
    wrote);
  - **touch**: `libnds`'s touch reading becomes pointer events (2405's
    `PointerMoved`, `PointerDown`, `PointerUp` and `PointerLeft`) on the
    lower surface, in that surface's coordinates (3023);
  - **the lid**: closing it sends `SystemEvent::Suspend` (so `App` saves
    and stops stepping, 3003), then the system sleeps; opening it sends
    `Resume`;
  - **the frame driver**: one `App::frame` per vertical blank, with
    `now` = vertical blanks × 1000 / 60 ms in integers, so a tick stage
    takes exactly one step per frame;
  - **storage**: save slots and settings as files on the flash cart's
    or the DSi's SD card, through BlocksDS's file system support, in the
    game's folder, written in the order `FileStorage` uses after 3045 (a
    temporary file, the last good copy kept as `.bak`, then the rename);
  - **audio**: sound effects only, baked to the formats the stock ARM7
    core's sound channels play, through `libnds`'s sound calls. Music is
    3061 (it may reuse 3027's format and mixer, or BlocksDS's own sound
    library if 3018 found its license permissive). **From 3029
    (done):** implement the shared backend trait
    (`shuyi_platform::audio::player::Backend`, ADR-0063) under its player,
    so that cue volumes, variants and fades are not written again. Fill in
    the provided methods the sound channels can play, leave the rest at
    their defaults, and list them in `targets/nds.ron`'s audio `features`
    (3017);
  - **memory**: the heap through the allocator 3018 chose, guarded by
    turning interrupts off (no atomics).
- **The DSi.** The `dsi` profile exports the same ROM, built to run in
  DSi mode where the system allows it. When it runs on a DSi, the
  backend uses its memory and speed (`libnds`'s DSi-mode check). Any
  DSi-only feature (the cameras, the extra RAM for bigger packs) is a
  follow-up ticket.
- **A fake hardware layer** behind a small internal trait (as 3010 has
  for the Playdate SDK), for host tests of everything above.
- **`shuyi export nds`** (and `dsi`), an `Exporter` (3004):
  1. it runs `check --target nds`, and refuses on any error;
  2. it builds the game as a static library for `armv5te-none-eabi`
     with the pinned nightly and `-Z build-std=core,alloc`;
  3. it links it with `libnds` and the stock ARM7 core, using the
     BlocksDS the developer installed (found through its environment
     variable);
  4. it bakes the pack for `nds` and adds it to the ROM's file system;
  5. it calls the installed `ndstool` (or the tool 3018 named) to write
     the `.nds` and its header, saying which tool it ran;
  6. it reports the ROM's size against `package_bytes` (64 MB, prov.).

  A missing BlocksDS, nightly or `rust-src` gives the install steps from
  `docs/targets/nds.md`.
- **CI**: `shuyi-nds` joins 3020's `armv5te-none-eabi` build, and
  `hello-grid` is exported for `nds` on every PR.
- **One example on the DS.** `hello-grid` (1118), with the second
  surface 3023 gave it, gets an `nds` keymap layout and a role file
  (3019), and exports. In the emulator 3018 chose:
  - a replay fixture with Harness-scripted input, touches on the lower
    surface included, ends with the same state hash as on desktop;
  - one `frame-png --target nds` is compared with the emulator's
    screenshot of the same frame, by eye in the PR.
- **Docs.** `docs/targets/nds.md` gets how to install BlocksDS, export,
  and run in an emulator and on a flash cart, and what doesn't work yet.

**Out (do not do):**
- **anything of Nintendo's in this repository**: no logo bytes, BIOS,
  firmware or official SDK, in code, fixtures or docs. The header tool
  and BlocksDS are the developer's installed ones;
- **changing the ARM7 core**, or writing our own;
- 3D and HD-2D (3025);
- music (3061);
- wireless play, the microphone, and DSi-only features;
- the Rumble Pak in the GBA slot: a ticket when a DS game asks, which
  implements 2408's `Platform::motors` (parked, rumble) in `shuyi-nds`;
- saves on retail-style cartridge memory;
- measuring on a device (3026).

## Implementation steps

1. Re-read `docs/targets/nds.md` and BlocksDS's docs at the chosen
   version. Fix the profiles.
2. Write the bindings for the first calls needed (video, input, the
   vertical blank), the internal hardware trait, and the fake.
3. Write `present` for both engines, sharing with `shuyi-gba` where it
   helps. Test against the fake.
4. Add touch, the lid, storage, sound effects and memory.
5. Write the exporter. Add the CI build and export.
6. Export `hello-grid`, run the replay in the emulator, and fix what's
   found.
7. Write the docs and the catalog row (and the world-layer follow-up
   ticket, if 2360 wasn't done).

## Acceptance criteria

- [ ] `shuyi export nds` on `hello-grid` produces a `.nds`, and its size
      is in the report.
- [ ] The ROM's replay hash in the emulator, touches included, equals
      the desktop run's (CI).
- [ ] Host tests with the fake hardware:
  - [ ] a touch at the lower screen's pixel (10, 20) reaches the second
        surface as a pointer at (10, 20);
  - [ ] closing the lid sends `Suspend`, and the game's save hook runs
        once;
  - [ ] the storage passes `shuyi_ui::storage::power_cut_suite` (3045)
        over the fake hardware's files;
  - [ ] 130 sprite items on one engine show 128, and 2 are counted.
- [ ] `shuyi-nds` builds in 3020's `armv5te-none-eabi` job.
- [ ] `cargo deny check` passes, and every BlocksDS part linked into the
      ROM has its license in the export's notices.
- [ ] The ARM7 core is BlocksDS's stock one, unchanged (the exporter
      links it from the installed BlocksDS).
- [ ] No Nintendo logo bytes, BIOS or firmware are in the repository.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the surface-to-engine mapping, touch, the lid, storage and sound
  calls, all against the fake hardware.
- Integration: the export and the replay in the emulator, in CI.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
