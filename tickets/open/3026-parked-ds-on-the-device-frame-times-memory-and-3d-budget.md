---
id: "3026"
title: "Parked: DS on the device: run the bench scenarios on a DS and a DSi through a flash cart, measure frame times, memory and the 3D budget, and calibrate the nds budgets"
type: research
milestone: "Parked: export targets"
model: implementation
effort: medium
status: blocked
blocked_by: ["3024", "3025"]
nick_input: setup
completed:
---

# 3026 — DS on the device

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§7, and [ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6, as 3022 does for the GBA. The `nds` budgets are provisional:

- 60 fps held; 3D on both screens at 30;
- 700 k emulator cycles a frame on the ARM9 (of about 1.1 M);
- 2.5 MB of heap (of 4);
- a 64 MB ROM;
- 3 s to start;
- ≤ 2 048 triangles a frame, textures within 512 KB, and no allocation
  in a steady frame.

The `dsi` profile differs in memory (16 MB) and speed (the ARM9 at
133 MHz in DSi mode). CI counts cycles in an emulator (3020). This
ticket runs the same scenarios on real hardware, records the numbers in
`docs/targets/nds.md`, and sets the budgets from the ratio of device to
emulator cycles. It also finds how many triangles a real frame holds at
60 fps, and whether 3D on both screens holds 30.

## Nick input

**Setup.** This needs hardware, and **it costs money**. The session
asks Nick to confirm before he buys anything:

1. A Nintendo DS, DS Lite or DSi. A used one is fine. Lending one works
   as well as buying. A DSi is needed only for the `dsi` numbers.
2. A DS flash cart that runs ROMs from a microSD card. The session
   names two or three current ones with their prices when the ticket
   starts, and Nick picks.
3. **DSi mode.** A flash cart runs ROMs in DS mode, even on a DSi. Running
   in DSi mode (16 MB, 133 MHz) needs another way to start homebrew on
   the DSi, and some of those change the system's own software and can
   break it. The session lists the options with their risks, and Nick
   decides. If he picks none, the `dsi` numbers are recorded as not
   measured.
4. A microSD card the cart takes, and a card reader for the PC.
5. Copy the ROM the session gives you (`hello-grid-bench.nds`) to the
   card. Start it, pick "run all", and wait for "done".
6. Put the card back in the PC, and give the session the
   `bench-results.ron` file the ROM wrote next to itself.

**Sign-off:** play `hello-grid` and the 3D example on the device for two
minutes each, and say whether they feel smooth.

## Scope

**In:**
- **A bench build.** `shuyi export nds --bench` builds `hello-grid` and
  the 3D example (3025) with the bench scenarios (3008) built in, and a
  menu to run one or all. Each frame, the build counts ARM9 cycles with
  the hardware timers around `App::frame` and the vertical blank work,
  and keeps the peak heap. The results are written to a file on the
  card, through 3024's storage.
- **`shuyi bench --read-results <file>`** turns the file into the same
  report as the emulator runner (3020).
- **A triangle sweep.** A scenario draws 500, 1 000, 1 500, 2 048 and
  more triangles of one test mesh, to find the most a frame holds at
  60 fps on the device, and where the hardware's own limit drops
  polygons.
- **Both screens.** A scenario with a 3D view on each surface (3025's
  capture), checking each holds 30 fps.
- **Music.** **Either order with 3061:** if 3061 is done, its music
  scenario (a song and four sound effects) is one of the bench
  scenarios: run it with the others, so the song's cue is in the
  numbers; if not, the report says music isn't measured, and 3061
  writes the ticket that measures it on the device.
- **The report** in `docs/targets/nds.md`, per scenario and per console
  (DS or DS Lite, and DSi in DS mode and in DSi mode if measured):
  - frame cycles: p50, p99 and the worst, on the device and in the
    emulator, and their ratio;
  - frames that missed the vertical blank;
  - peak heap, and allocations after a stage's first frame;
  - triangles and texture bytes per frame;
  - start-up to the title screen;
  - the ROM's size;
  - the console model, the flash cart and its firmware version.
- **Calibration:**
  - `frame_cycles` from the worst ratio, with 20 % headroom;
  - `max_triangles` in `three_d`, if the sweep shows 2 048 doesn't hold
    60 fps with the engine's own work;
  - **Either order with 3009:** if 3009 is done,
    `think_units_per_frame` the same way; if not, 3009 sets it from
    these numbers;
  - the `dsi` profile's budgets from the DSi-mode run, if there is one;
  - `status` becomes `supported` once every scenario holds its frame
    rate on a DS or DS Lite.

  The PR carries the measurements that justify every number it changes.
- **If a scenario misses its frame rate**, write a ticket per cause
  found, each with its measurement. Don't fix them here.

**Out (do not do):**
- optimizing (follow-up tickets);
- buying anything for Nick, or changing a console's system software;
- the GBA (3022);
- the game's own content (a game ticket, if the game ships on the DS).

## Implementation steps

1. Add `--bench` to the DS exporter, with the menu, the timers and the
   results file.
2. Add the triangle sweep and the both-screens scenario. Write
   `shuyi bench --read-results`. Test them in the emulator first.
3. After Nick's setup, run every scenario five times per console, and
   keep the median run of each.
4. Write the report and calibrate the profiles.
5. Write the follow-up tickets.

## Acceptance criteria

- [ ] `docs/targets/nds.md` has every scenario's numbers, device and
      emulator, per console, with the hardware and cart recorded.
- [ ] The triangle sweep's result is recorded, and `max_triangles`
      matches it or the PR says why not.
- [ ] The `nds` (and, if measured, `dsi`) budgets are set from the
      measurements, and the PR shows the arithmetic.
- [ ] `status: supported` only if every scenario holds its frame rate.
      Otherwise, follow-up tickets exist for each miss.
- [ ] Nick's sign-off is recorded in the completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the results file round trips through `--read-results`.
- Integration: the bench build in the emulator writes a results file
  that `--read-results` reads. This ticket measures; the bench gate
  (3020) enforces the result.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
