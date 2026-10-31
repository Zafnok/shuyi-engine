---
id: "3018"
title: "Parked: Research: GBA and DS toolchains, Rust on old ARM, emulators for CI, header tools, and whether to sell homebrew"
type: research
milestone: "Parked: export targets"
model: reasoning
effort: high
status: blocked
blocked_by: ["3005"]
nick_input: decision
completed:
---

# 3018 — Research: GBA and DS toolchains, emulators and selling homebrew

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
set the design from facts gathered on 2026-10-17, and said this ticket
re-checks each one before anything relies on it:

- **The hardware** (ADR-0012, *The hardware*): GBATEK, the BlocksDS
  documentation, and Wikipedia. Several numbers are marked provisional:
  the GBA's sprites per line (32, depending on sprite sizes), the DS's
  texture memory (512 KB), and every budget in §7.
- **Rust** (*Rust and the toolchains*):
  - `gba` 0.15.0 was **Zlib OR Apache-2.0 OR MIT** on crates.io on
    2026-10-17, so we take it under MIT or Apache-2.0;
  - `agb` 0.25.0 is **MPL-2.0**, which ADR-0002 §8 denies, so it is not
    used;
  - the GBA targets `thumbv4t-none-eabi` and `armv4t-none-eabi`, and the
    DS's ARM9 target `armv5te-none-eabi`, are tier 3: nightly and
    `build-std`;
  - BlocksDS: `libnds` Zlib, DSWiFi MIT, picolibc MIT- and BSD-like;
  - no maintained Rust crate for the DS was found.
- **Emulators** (§7): mGBA (MPL-2.0) and melonDS (GPL-3.0) exist. They
  are CI tools run as separate programs, never linked or shipped, so
  their licenses don't reach the engine. One is needed per system that
  counts cycles the same on every run, headless, with no Nintendo BIOS or
  firmware file.
- **Nothing of Nintendo's in this repository** (§8). The GBA's BIOS and
  the DS's firmware check for Nintendo's logo in the ROM's header. The
  toolchains' header tools write it. Which tools those are, and that
  this repository needs none of Nintendo's files, is confirmed here.
- **Selling** (*Selling*, §8): Nintendo no longer licenses either system.
  A game ships as homebrew: a ROM for flash carts and emulators, or a
  cartridge from a small publisher. That is Nick's call for Visions of
  Shuyi.

`no_std` (3005) must have landed, so the spike builds the real crates.

## Nick input

**Decision** (the `ask-nick` skill), when unparked. The session gives
Nick the facts it found first, in plain words:

- Will Visions of Shuyi, or anything else of Nick's, be sold on the GBA
  or the DS?
- If yes, how: as a ROM (on itch.io, for flash carts and emulators), or
  as a cartridge through a small homebrew publisher?
- Before anyone sells one, a lawyer's look is advised at the logo in the
  ROM header that the hardware needs to boot. The session explains the
  question and points to public sources only. It is not legal advice,
  and the ticket says so.

Record the answer in `docs/decisions.md`. If the answer is "no", the
engine's ports still go ahead as engine features; only the selling notes
change.

## Scope

**In:**
- **The hardware numbers**, each re-checked in GBATEK and recorded with
  its source:
  - the GBA's CPU, memory sizes and speeds, display timing (cycles per
    frame), backgrounds per mode, sprites, palettes, saves, audio;
  - **the GBA's sprite limit per line**: how it depends on sprite sizes
    and on the "H-blank interval free" bit, as a table the bake check of
    3021 can use;
  - the same for the DS and the DSi, including the 3D engine's vertex
    and polygon limits, its vertex format and texture memory.

  Any number that differs from ADR-0012 is fixed in that ADR's
  *Context* in this PR (a fact, not a decision). If a fact changes a
  decision, write a new ADR with the `write-adr` skill.
- **The Rust libraries**, by their current versions and licenses (with
  `cargo deny` on a scratch crate):
  - `gba`: what it gives (the runtime and linker script, the registers,
    interrupts, DMA, saves) and what we would write;
  - why not `agb` (MPL-2.0), stated once in `docs/targets/gba.md`;
  - a `no_std` allocator for the heap that needs no atomic
    compare-and-swap (ARMv4T and ARMv5TE have none), with its license.
- **BlocksDS**, component by component, with each license: `libnds`,
  the default ARM7 core, DSWiFi, picolibc, its sound library, `ndstool`,
  and its compiler toolchain. A build tool under a copyleft license is
  acceptable only as a separate program the developer installs, never
  vendored, linked or shipped (ADR-0002 §8); say which tools that is.
  Also how BlocksDS is installed on Windows without MSVC.
- **The Rust targets.** For each of `thumbv4t-none-eabi`,
  `armv4t-none-eabi` and `armv5te-none-eabi`: does it build `core` and
  `alloc` with `-Z build-std=core,alloc` on a current nightly? Pick the
  GBA target (Thumb for ROM code, with ARM code in fast RAM, or not) and
  one nightly to pin. Note the date.
- **A spike** (kept in the PR as `spikes/old-arm/`, not a crate of the
  workspace):
  - a GBA ROM, over `gba`, and a DS ROM, linked with BlocksDS, each
    holding `shuyi-core` built `--no-default-features`;
  - each runs one replay fixture of `hello-grid` (1118) and writes its
    final state hash to the emulator's debug output;
  - the hash equals the desktop run's.
- **The emulators.** For each system, pick the one CI uses:
  - it runs headless on Linux, as a separate program;
  - it counts cycles, and the count is the same on every run (run the
    spike three times);
  - it boots with no Nintendo BIOS or firmware file (a built-in
    replacement BIOS, or direct boot);
  - it gives the ROM a way to print (a debug register or log);
  - its license, recorded. It is never linked or shipped.

  Compare mGBA and melonDS with any other candidate found, and record
  how close each one's cycle counts are said to be to the hardware.
- **The header tools.** Which tool writes the GBA ROM header (and its
  checksum), and which writes the DS's (`ndstool`, or another); where
  each gets the logo bytes; and the confirmation that `shuyi export gba`
  and `shuyi export nds` can call the installed tool and need nothing
  from Nintendo in this repository.
- **The write-ups**, `docs/targets/gba.md` and `docs/targets/nds.md`
  (start them if 3008 hasn't; extend them if it has): every fact above
  with its source and date, the chosen tools and versions, how to
  install them on Windows with the GNU toolchain, and a plain section
  *Selling* that says these systems are not licensed by Nintendo any
  more, what that means, and Nick's answer.

**Out (do not do):**
- **anything of Nintendo's in this repository**: no logo bytes, no BIOS,
  no firmware, nothing from an official SDK, not even in a test fixture
  or the spike;
- writing a backend (3021, 3024);
- the CI jobs (3020);
- legal advice; the ticket gathers public facts and asks Nick;
- buying hardware or a cartridge for Nick.

## Implementation steps

1. Re-check the hardware numbers, and fix ADR-0012's *Context* where
   they differ.
2. Check the libraries' and BlocksDS's licenses with `cargo deny` and
   the projects' license files. Write them down with the date.
3. Build `core` and `alloc` for the three targets on a nightly. Pick the
   nightly and the GBA target.
4. Write the spike ROMs and run the replay in each candidate emulator.
   Compare hashes with desktop, and cycle counts across three runs.
5. Find and test the header tools.
6. Write `docs/targets/gba.md` and `docs/targets/nds.md`.
7. Ask Nick the selling question (the `ask-nick` skill) and record it.
8. If a fact changed a decision, write the ADR. Write a ticket for
   anything else found.

## Acceptance criteria

- [ ] `docs/targets/gba.md` and `docs/targets/nds.md` hold every fact
      above, each with a source and a date.
- [ ] Every library and tool the ports will use has its license recorded,
      and none denied by ADR-0002 §8 is linked or shipped.
- [ ] The spike's replay hash matches desktop on both systems, in the
      chosen emulators, and the PR shows the three runs' cycle counts.
- [ ] The chosen emulators boot the spike with no Nintendo file present.
- [ ] A search of the repository finds no Nintendo logo bytes, BIOS or
      firmware (the PR says how it searched).
- [ ] Nick's answer on selling is in `docs/decisions.md`.
- [ ] ADR-0012's facts match what was found, or a new ADR says what
      changed.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- None beyond the gates and the spike runs. This ticket researches; the
  CI builds (3020) and the backends (3021, 3024) test what it found.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
