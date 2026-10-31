---
id: "3022"
title: "Parked: GBA on the device: run the bench scenarios on a GBA through a flash cart, record frame times and memory, and calibrate the gba budgets"
type: research
milestone: "Parked: export targets"
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: blocked
blocked_by: ["3021"]
nick_input: setup
completed:
---

# 3022 — GBA on the device

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§7, and [ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6: CI gates on a proxy, and a device ticket measures the truth and sets
the budgets from the measured ratio. The `gba` budgets are provisional:

- 60 fps held where the game is animating; turn screens may run 30;
- 200 k emulator cycles a frame (of about 280 k);
- 160 KB of heap (of 256);
- a 16 MB ROM (of 32);
- 2 s to start;
- no allocation in a steady frame, and each enemy decision ≤ 2 s,
  thinking in slices (3009).

CI counts cycles in an emulator (3020). This ticket runs the same
scenarios on a real GBA through a flash cart, records the numbers in
`docs/targets/gba.md`, and sets the profile's budgets from the ratio of
device cycles to emulator cycles. That ratio should be close to 1
(ADR-0012, *Consequences*); how close is what this finds.

## Nick input

**Setup.** This needs hardware, and **it costs money**. The session
asks Nick to confirm before he buys anything:

1. A Game Boy Advance or a Game Boy Advance SP. A used one is fine.
   Lending one works as well as buying.
2. A GBA flash cart that runs ROMs from a microSD card and writes saves
   back to the card. The session names two or three current ones with
   their prices when the ticket starts, and Nick picks.
3. A microSD card of any size the cart takes, and a card reader for the
   PC.
4. Copy the ROM the session gives you (`hello-grid-bench.gba`) to the
   card, as the cart's manual says. Boot the GBA, start the ROM, and
   pick "run all" in its menu. It takes a few minutes.
5. When the screen says "done", turn the GBA off, put the card back in
   the PC, and give the session the save file the cart wrote next to the
   ROM (same name as the ROM, with the cart's save extension).

**Sign-off:** play `hello-grid` on the device for two minutes, and say
whether it feels smooth.

## Scope

**In:**
- **A bench build.** `shuyi export gba --bench` builds `hello-grid` with
  the bench scenarios (3008) built in, and a menu to run one or all.
  Each frame, the build counts cycles with the hardware timers (two
  cascaded timers at the CPU clock) around `App::frame` and the vertical
  blank work, and keeps the peak heap.
- **The results go to the save.** The device has no cable we can read,
  so the bench writes its results to save memory, and the flash cart
  writes that to the card. `shuyi bench --read-save <save-file>` turns
  it into the same report as the emulator runner (3020).
- **The report** in `docs/targets/gba.md`, per scenario:
  - frame cycles: p50, p99 and the worst, on the device and in the
    emulator, and their ratio;
  - frames that missed the vertical blank;
  - peak heap, and allocations after a stage's first frame;
  - start-up to the title screen;
  - the ROM's size;
  - the console model, the flash cart and its firmware version.
- **Audio.** **Either order with 3027:** if 3027 is done, the scenarios
  run with music on, so the mixer's cost is in the numbers; if not, the
  report says audio isn't measured, and 3027 re-runs the music scenario
  on the device when it lands.
- **Calibration:**
  - `frame_cycles` becomes the emulator count that matches a full frame
    on the device, by the worst ratio, with 20 % headroom;
  - **Either order with 3009:** if 3009 is done,
    `think_units_per_frame` is set the same way; if not, 3009 sets it
    from these numbers;
  - `status` becomes `supported` once every scenario holds its frame
    rate.

  The PR carries the measurements that justify every number it changes.
- **If a scenario misses its frame rate**, write a ticket per cause
  found, each with its measurement. Don't fix them here.

**Out (do not do):**
- optimizing (follow-up tickets);
- buying anything for Nick;
- the DS (3026);
- the game's own content (a game ticket, if the game ships on the GBA).

## Implementation steps

1. Add `--bench` to the GBA exporter, with the menu and the timers.
2. Write the save-file results and `shuyi bench --read-save`. Test them
   in the emulator first.
3. After Nick's setup, run every scenario five times, and keep the
   median run of each.
4. Write the report and calibrate the profile.
5. Write the follow-up tickets.

## Acceptance criteria

- [ ] `docs/targets/gba.md` has every scenario's numbers, device and
      emulator, with the hardware and cart recorded.
- [ ] The `gba` budgets are set from the measurements, and the PR shows
      the arithmetic.
- [ ] `status: supported` only if every scenario holds its frame rate.
      Otherwise, follow-up tickets exist for each miss.
- [ ] Nick's sign-off is recorded in the completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the save-file results round trip through `--read-save`.
- Integration: the bench build in the emulator writes a save that
  `--read-save` reads. This ticket measures; the bench gate (3020)
  enforces the result.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
