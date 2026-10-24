---
id: "3020"
title: "Parked: Old ARM targets in CI: pinned-nightly builds for thumbv4t and armv5te, no atomics, no allocation in a steady frame, and emulator cycle counts in shuyi bench"
type: infra
milestone: "Parked: export targets"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["3018", "3005", "3008", "2012", "3017"]
nick_input: none
completed:
---

# 3020 — Old ARM targets in CI

> **Parked** (Nick, 2026-10-17: "I want GBA, and DS ports"): until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

[ADR-0012](../../docs/adr/0012-gba-and-ds-targets-capabilities-indexed-colour-two-screens-and-homebrew.md)
§6 and §7. The GBA's ARM7TDMI (16.78 MHz) and the DS's ARM946E-S
(67 MHz) bring three rules the engine's crates must keep from the start,
because each gets dearer as code piles up:

- **Builds on nightly.** Their Rust targets, `thumbv4t-none-eabi` and
  `armv5te-none-eabi`, are tier 3: nightly and `build-std`. CI pins one
  nightly, so a nightly change can't break `main` by surprise. 3018
  picks it.
- **No atomics.** Neither ARMv4T nor ARMv5TE has atomic
  compare-and-swap, so `alloc::sync::Arc` and anything using atomics
  don't build there. `alloc::rc::Rc` is fine. The CI build enforces it.
- **No allocation in a steady frame.** The budget for `gba` and `nds`
  is zero allocations after the first frame of a stage. Arenas and
  buffers are sized when a stage starts. 3008's counting allocator
  measures it.

And the CI proxy for frame time on these two targets is **cycles counted
by an emulator** (§7), not Cachegrind: an x86 count says little about a
16 MHz ARM7 with slow external memory. 3018 picks one emulator per
system that counts the same on every run, headless, with no Nintendo
BIOS or firmware file. The provisional budgets (ADR-0012 §7):

| Target | `frame_cycles` | `peak_heap_bytes` | Steady-frame allocations |
| ------ | -------------- | ----------------- | ------------------------ |
| `gba` | 200 k (prov.; of about 280 k a frame) | 160 KB (prov.) | 0 |
| `nds` | 700 k on the ARM9 (prov.; of about 1.1 M) | 2.5 MB (prov.) | 0 |

The `gba` and `nds` profiles exist as `planned` (3017).

## Nick input

None.

## Scope

**In:**
- **The CI builds**, on every PR, on Linux, with the nightly that 3018
  pinned (written once, as the `channel` of
  `ci/old-arm/rust-toolchain.toml`, and read by the job and the
  `run-gates` skill):
  - `cargo +<nightly> build -Z build-std=core,alloc --target
    thumbv4t-none-eabi --no-default-features -p shuyi-core -p shuyi-ui
    -p shuyi-platform -p shuyi-assets` (the pack reader's features only,
    2204);
  - the same for `armv5te-none-eabi`;
  - the backends `shuyi-gba` and `shuyi-nds` join their target's build
    in their own tickets (3021, 3024). This ticket leaves a comment in
    the job where they go.

  Everything else stays on stable.
- **No atomics, said early.** A `clippy.toml` `disallowed-types` entry
  for `alloc::sync::Arc` and `core::sync::atomic::*` in those crates, so
  the stable build names the rule with a clear message. The nightly
  build is the real enforcement. A type that needs shared ownership uses
  `Rc`.
- **Allocations per frame.** The counting allocator
  (`shuyi-alloc-count`, 3008's and 3064's) also counts calls to `alloc`
  and `realloc` per frame. `shuyi bench` reports `allocs_per_frame`
  (the most, after the first frame of each stage). `App` (3003) marks
  the frame a stage starts, so the bench knows where "first frame" is.
  **Either order with 3064:** if 3064 is done, that call counter is
  already in the crate (`take_allocs`, read once a frame for the perf
  traces); use it. If not, add it here, and 3064 uses it (its own
  clause says so).
- **Two budget fields** in the profile format, each an `Option` that
  defaults to `None`, so other profiles are unchanged. The format has
  no version field to bump (3017):
  - `budgets.frame_cycles: Option<u64>`;
  - `budgets.steady_frame_allocs: Option<u32>`.

  `targets/gba.ron` and `targets/nds.ron` get the values in the table
  above, each with a comment naming the device ticket that calibrates it
  (3022, 3026).
- **An emulator runner in `shuyi bench`**, `--runner emulator`:
  - a **bench ROM** per system, built from the bench scenarios (3008)
    over the headless backend (3003), with the minimal runtime 3018's
    spike used (`gba`'s runtime for the GBA; BlocksDS's for the DS). It
    draws nothing; it runs `App::frame` and prints, through the
    emulator's debug output, the cycle count of each frame (from the
    hardware timers, or the emulator's own counter, as 3018 found
    works);
  - `shuyi bench` launches the emulator 3018 chose as a separate
    program, reads the output, and reports `frame_cycles` per scenario
    (the most, p99 and the mean);
  - the emulator is downloaded in CI at a pinned version, its license
    recorded in `docs/targets/README.md`. It is never vendored, linked
    or shipped.
- **The gate.** The CI bench job fails when a scenario is over any
  profile's `frame_cycles`, `peak_heap_bytes` or `steady_frame_allocs`.
  For these three fields the gate covers `planned` profiles too, unlike
  3008's `supported`-only rule: ADR-0012 §7 makes the emulator the CI
  proxy for these targets before any device exists. Say so in
  `docs/targets/README.md`.
- **The `run-gates` skill** gets the two nightly builds, with the
  `rustup toolchain install` and `rust-src` lines, and notes that the
  emulator runner runs in CI only. Check that the library builds need no
  linker on Windows with the GNU toolchain, and say so.
- **Who moves the pinned nightly and when** goes in a section of the
  `bump-rust` skill (2012). Dependabot's `rust-toolchain` ecosystem
  moves a dated nightly (`nightly-YYYY-MM-DD`) only in a
  `rust-toolchain` or `rust-toolchain.toml` file. That is why the
  nightly is pinned in `ci/old-arm/rust-toolchain.toml`, in a folder of
  its own, and not in a plain text file. Add that folder
  (`/ci/old-arm`) to the `rust-toolchain` update in
  `.github/dependabot.yml`. `cargo xtask check-toolchain` compares only
  the root and `templates/` pins, so that folder needs no exception
  there.

**Out (do not do):**
- the backends (3021, 3024);
- measuring on devices (3022, 3026);
- removing allocations from existing frames: each scenario that
  allocates gets a ticket of its own, with its measurement (write them;
  don't fix them here);
- moving any other crate or job off stable;
- raising a budget to pass the gate (`AGENTS.md` rule 6).

## Implementation steps

1. Add `ci/old-arm/rust-toolchain.toml` with the pinned nightly, and
   the two CI builds. Fix what breaks in the four crates; list each
   change in the PR.
2. Add the `disallowed-types` entries, and replace any `Arc` in those
   crates with `Rc` or a design without shared ownership.
3. Add the allocation counting (or use 3064's, if it is done) and the
   stage-start marker. Add the two budget fields and their values in
   the two profiles.
4. Write the bench ROMs and the emulator runner. Run each scenario three
   times in CI and check the counts are the same.
5. Add the gate and the docs. For each scenario over budget, write a
   ticket.
6. Update the `run-gates` skill. Add the pinned nightly's section to
   the `bump-rust` skill and `/ci/old-arm` to the `rust-toolchain`
   update in `.github/dependabot.yml`.

## Acceptance criteria

- [ ] CI builds the four crates for `thumbv4t-none-eabi` and
      `armv5te-none-eabi` on the pinned nightly, on every PR.
- [ ] A fixture crate that uses `Arc` fails the `thumbv4t-none-eabi`
      build (a CI step that expects the failure), and fails the
      `disallowed-types` lint on stable.
- [ ] A fixture scenario that allocates once per frame after its first
      frame fails on `gba`, naming the scenario and the frame; one that
      allocates only in its first frame passes.
- [ ] Two runs of the emulator runner on the same commit give the same
      `frame_cycles`.
- [ ] A fixture that burns cycles in a loop fails on `gba`, naming the
      frame and both numbers.
- [ ] Tickets exist for each example scenario over a budget, or none is.
- [ ] `.github/dependabot.yml`'s `rust-toolchain` update lists
      `/ci/old-arm`, the `bump-rust` skill has a section on the pinned
      nightly, and `cargo xtask check-toolchain` still passes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the allocation counter, and the budget comparison with the new
  fields.
- Integration: the bench ROMs in the emulators in CI, twice, with the
  same counts; the fixture failures above.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
