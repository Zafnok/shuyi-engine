# ADR-0081: The bench: Callgrind dumps at each frame's end, a profile of its own, and scenarios as test inputs

- **Status:** Accepted
- **Date:** 2026-11-05
- **Related tickets:** 3008 (this), 3064, 3006, 3004, 3011, 3013, 2566
- **Extends:** ADR-0008 §6

## Context

ADR-0008 §6 gates performance in CI on proxies that come out the same on
every run: instructions per frame counted under Cachegrind, the heap's
peak from a counting allocator, and the package's size. Device tickets
(3011, 3013) measure real frame times and calibrate the proxies. Ticket
3008 builds `shuyi bench`, which plays scenarios headless and compares
those numbers with each target profile's `budgets`.

The forces:

- **A number per frame.** A budget is per frame, and a failure names the
  frame. Cachegrind counts a whole run; it has client requests to start
  and stop, not to dump. A client request is inline assembly, which the
  CLI's `unsafe_code = "forbid"` rules out, or a crate around it
  (`crabgrind` builds C against Valgrind's headers; `gungraun`, formerly
  `iai-callgrind`, is a benchmark harness with a runner of its own).
- **What a frame is.** The bench plays a scenario through the contract's
  `Session` (`play`, then `idle`), as `run --perf` does (ticket 2566's
  clause). A step of its script runs several frames inside the Harness:
  nothing outside it sees each frame begin or end.
- **Which build.** Shipped builds are optimized, so an unoptimized
  build's count says little of a device. The heap counter
  (`shuyi_alloc_count::global`, ADR-0046 §9, ticket 3064) counts only
  with debug assertions on, and a binary has one global allocator.
- **One-bit targets.** On the Playdate each frame is converted to one
  bit (`to_one_bit`, ADR-0026) by the backend's `present`. The headless
  backend's `present` only keeps the frame.

## Decision

### 1. Callgrind, dumping before each frame's heap reading

`shuyi bench` runs the game's tool program under
`valgrind --tool=callgrind --dump-before=*contract::session::process_heap*`.
A measured frame reads the heap once, at its end
(`shuyi_platform::App` records it with `Platform::memory`), and
`shuyi_cli::contract::session::process_heap` is the reading
`Session::set_perf` gives the Harness. It is `#[inline(never)]`, so it
keeps its name. Callgrind dumps its count and starts again from zero at
each call, so each dump is one frame. `bench-play` calls it once itself
before the scenario, so the first dump is the start, kept apart and never
gated; the dump at the program's end is left out. The dumps' order comes
from their `part:` line, then the file's number.

`bench-play` measures with a fake clock (`PerfClock::fake`, through
`contract::session::measure_with_a_fake_clock`): a real clock's
readings could change a frame's count from run to run, and the bench
reads no time.

No client request, no `unsafe` and no new crate. Callgrind is part of
Valgrind, counts as Cachegrind does, and is as repeatable. CI checks this
by running the bench twice and comparing every frame's count.

A game whose tool program doesn't count the heap (it has its own
allocator, `shuyi_no_heap_count`) has no frame marker: `bench` refuses to
count its instructions, and `--no-instructions` measures the rest.

### 2. A cargo profile of its own: `shuyi-bench`

`bench` builds the tool program with
`cargo build --profile shuyi-bench`, the profile given on the command
line (`--config profile.shuyi-bench.inherits="release"`,
`debug-assertions=true`, `strip=false`). So:

- the code is the release profile's, with its LTO and codegen units;
- debug assertions are on, so `global` counts the heap and
  `shuyi_ui::DEBUG_TOOLS` records each frame;
- the symbols stay, so Callgrind finds the marker;
- its output goes in `target/shuyi-bench`, apart from the release build.

The bench reads `global`'s counter, as `run --perf` does
(`HeapSummary::peak_bytes`). It declares no allocator of its own.

### 3. The game converts its frames in a present hook

`HeadlessPlatform::set_present_hook` and `Harness::set_present_hook` run a
`PresentHook` (`Box<dyn FnMut(&Frame<'_>)>`) on each frame presented, in
the backend's `present`, where a device's backend does its work. A new
`GameTools::session_on(target, options)` starts the game on a target's
console. On a one-bit target it sets a hook that runs `to_one_bit` with
the game's pattern map and one-bit font, as `frame_one_bit` paints. The
bench plays each scenario once for the colour targets and once on each
one-bit target. A game that leaves `session_on` at its default is measured
on its colour session there, and the report says so.

### 4. Scenarios are test inputs, read with serde

A scenario is a `*.bench.ron` file anywhere in the game's folder, outside
`target`, `dist` and hidden folders: `format_version` (1), `about`,
`start` (the game's start options), `script` (`run`'s steps), `frames`,
and `measure` (`Every`, or `Frames(first, last)`). It is never shipped or
loaded by a game, so it gets no `Format` (ADR-0007 is about what a game
loads), like 2018's capture plans and 2518's session records. An error
names the file, the line and the column.

### 5. What is gated against what

The targets are those the game's `shuyi.ron` lists in `targets` whose
profile is `Supported` (or `--target` alone). ADR-0008 §6 says "every
`supported` profile"; that predates the game's list (ticket 3001), and a
game is gated only where it ships. For each run and each of its targets:

- the frame with the most instructions against `frame_instructions`;
- the heap's highest peak against `peak_heap_bytes`;
- each package `export` left under `dist/export` (by its report's
  `profile`) against `package_bytes`.

Each failure names the scenario, the frame, the target and both numbers.
A budget a target hasn't is no gate. `bench-report.json` holds every
frame's numbers, and `bench-report.md` is the table for a pull request.

## Consequences

- The numbers are an x86-64 Linux program's, and a debug assertion costs
  a little. They are proxies, as ADR-0008 says, and only device tickets
  change a budget.
- The bench runs only where Valgrind does: Linux, in CI. On Windows,
  `--no-instructions` measures the heap and the packages.
- Renaming `process_heap` breaks the marker. The bench then fails with
  "Callgrind dumped nothing at a frame's end", never passes in silence.
- The instructions between two frames (the Harness's bookkeeping, the
  script's next step) count in the next frame. They are a few thousand
  against budgets of millions.
- A game gets one more `GameTools` method, which it may leave out.

## Alternatives considered

- **Cachegrind with one process per frame** — a run of *n* frames costs
  *n* runs of the whole start. Far too slow under Valgrind.
- **Client requests (`gungraun`, `crabgrind`)** — `unsafe` or C in the
  CLI, and a crate whose job is a benchmark runner of its own. The marker
  by name needs neither.
- **Measuring the dev build** — its counts are many times a shipped
  build's, so the Playdate's 2 M budget would mean nothing.
- **A second counting allocator in release builds** — a game with its own
  allocator would no longer link, and the profile with debug assertions
  makes it needless.
- **Converting to one bit in the headless backend itself** — a Harness on
  a running game has no look to convert with (`Show::Game`'s `look` is
  `None`), and the device's work is the game's backend's to choose.
