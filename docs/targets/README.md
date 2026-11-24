# The targets' budgets

What each target may cost a game, how CI checks it, and how a game adds
its own benchmarks ([ADR-0008](../adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§6, [ADR-0081](../adr/0081-the-bench-callgrind-dumps-at-each-frames-end-a-profile-of-its-own-and-scenarios-as-test-inputs.md)).
Each target has a page here with its budget row and what was measured on
its device:

| Target | Page |
| ------ | ---- |
| `desktop` | [`desktop.md`](desktop.md) |
| `web` | [`web.md`](web.md) |
| `steam-deck` | [`steam-deck.md`](steam-deck.md) |
| `playdate` | [`playdate.md`](playdate.md) |
| `switch` | [`switch.md`](switch.md) |
| `android` | [`android.md`](android.md) |
| `ios` | [`ios.md`](ios.md) |

## The budgets

Each profile in [`targets/`](../../targets/) has `budgets`
(`shuyi_assets::target::Budgets`):

- `frame_instructions`: the most instructions one frame may take;
- `peak_heap_bytes`: the most heap in use at once;
- `package_bytes`: the biggest package; on the web, what the page loads
  before the game starts ([`web.md`](web.md));
- `startup_ms`: the longest start-up, measured on the device only;
- `load_slice_ms`: how long one frame runs a game's load in steps for, on
  a device whose system ends a game whose frame runs too long. It is no
  limit `bench` checks: the backend reads it
  ([ADR-0097](../adr/0097-a-load-in-steps-a-slice-a-frame-from-the-platform-and-the-one-shot-load-kept-beside-it.md)).

A budget a profile leaves out isn't gated. A budget marked provisional
is a starting guess until its device is measured. A budget changes only
in a pull request that carries a device measurement, written on the
target's page. Raising one to let a slow change pass is weakening a
gate.

## The graphics tier

Each profile also says how much a game moves and draws there, beyond its
stills: `graphics` (`shuyi_assets::target::GraphicsTier`, ticket 3347),
read into `PlatformFacts::graphics`. The tiers are ordered:

| Tier | What a game shows | Targets |
| ---- | ----------------- | ------- |
| `Full` (the default) | everything it moves: tiles that loop through frames, particles, effects over the screen | `desktop`, `web`, `steam-deck`, `switch`, `android`, `ios` |
| `Reduced` | stills, with at most a few cheap moving looks the game picks; no particles, no effects over the screen | `playdate` |
| `Minimal` | stills only | none yet |

A moving look says the lowest tier it plays at, and a skin draws its
still below that (`GraphicsTier::plays`). The engine enforces none of it:
it draws what a skin gives it, and the budgets above are what `bench`
gates. A debug build of `shuyi-app` shows another tier than its target's
when `SHUYI_GRAPHICS` names one (`SHUYI_GRAPHICS=minimal`), so a lesser
target's stills are seen on the desktop; a build that ships never reads
it.

## The proxies, and why not wall time

CI runs on shared machines whose speed changes from run to run, so a
frame's time there is never a gate. CI gates on numbers that come out
the same on every run of the same commit instead:

- **instructions per frame**, counted by Valgrind's Callgrind on Linux:
  the program's instructions, the same on every run, whatever else the
  machine does;
- **the heap's peak**, from the heap counter of the game's tool program
  (`shuyi_alloc_count`): the bytes the game asked for, never the
  allocator's own;
- **the package's size**, from the `export-report.json` that `shuyi
  export` writes beside the package. A web package counts the files of
  its folder but the `web.music` and `web.voice` folders, which the game
  fetches when it first plays a track or a line; the table shows the
  whole zip beside it.

They are proxies: an x86-64 instruction is not a Playdate's, and a
desktop's heap isn't a device's memory. A device ticket runs the same
scenarios on the device, measures the frame times, the dropped frames,
the memory, the start-up and the power, writes them on the target's
page, and sets the instruction budget from the ratio it found.

## `shuyi bench`

```
cargo shuyi bench                       # every scenario, against every target the game ships on
cargo shuyi bench --target playdate     # against one target, listed or not
cargo shuyi bench --scenario idle-map   # one scenario
cargo shuyi bench --no-instructions     # without Valgrind (on Windows): the heap and the packages
```

It builds the game's tool program in the `shuyi-bench` profile (the
release profile with debug assertions on and its symbols kept), plays
each scenario under Callgrind, and compares each number with the budgets
of each `Supported` target the game's `shuyi.ron` lists in `targets`. A
scenario plays once for the colour targets, and once more on each
one-bit target, on the console the game shows there and with each frame
converted to one bit as the device converts it.

It writes `bench-report.json` (every frame's numbers) and
`bench-report.md` (the table) to `dist/bench`, prints the table, and
fails when a number is over a budget, naming the scenario, the frame,
the target and both numbers. A package is read from each `export-report.json`
under `dist/export` whose profile is the target's: run `cargo shuyi
export <target>` first to gate it.

Valgrind runs on Linux: on Windows, run it in WSL, or leave the
instructions to CI with `--no-instructions`.

### Long runs

```
cargo shuyi bench --long                     # every long scenario
cargo shuyi bench --long --twice             # each twice, in two processes, hashes compared
cargo shuyi bench --long --scenario stage-loop
```

A slow leak shows only over hours: a cache that grows with each map, a
sound never stopped, a log that keeps every line. `--long` plays the
game's long scenarios ([ADR-0145](../adr/0145-long-runs-flat-memory-and-sounds-and-equal-hashes.md)),
in the same `shuyi-bench` profile and never under Valgrind, so it runs on
Windows too. Every `sample_every` frames it reads the live heap, each
memory tag's live bytes, the sounds alive and the frame's hash. After
`warm_up` it splits the samples in quarters, and fails when the most live
heap of the last quarter is over the first quarter's most by more than
`slack.heap_bytes`, or the most sounds alive by more than `slack.sounds`,
naming the scenario, both numbers and the three memory tags that grew
most. With `--twice` it fails at the first sample whose frame's hash
differs between the two runs. It writes `long-report.json` and
`long-report.md` to `dist/bench`. Wall time is never read.

A long scenario is a file named `<name>.long.ron` anywhere in the game's
folder, found as the scenarios are:

```ron
(
    format_version: 1,
    about: "the stage, moves, the options, looped",
    // The game's start options, as a scenario's.
    start: [],
    // The steps played each time: go round a loop that ends where it began.
    script: (steps: [keys("Enter"), wait(1.0), keys("Escape"), wait(0.3)]),
    // The times the script plays, and the frames with no input after each.
    repeat: 2000,
    idle: 0,
    // The first frames are not judged: caches filling, the first load.
    warm_up: 20000,
    // Frames between samples.
    sample_every: 5000,
    // How much the last quarter may be over the first: 64 KiB and no
    // sound, unless given.
    slack: (heap_bytes: 65536, sounds: 0),
)
```

Write one that goes round the game's whole loop, from the title through
play and back, so each part is entered many times. Hello, Grid's is
[`examples/hello-grid/long/stage-loop.long.ron`](../../examples/hello-grid/long/stage-loop.long.ron):
a million frames, about four and a half hours of play. The engine's own
weekly run (`.github/workflows/long-run.yml`) plays it; a game runs
`cargo shuyi bench --long --twice` in its own CI if it wants to.

## A game's scenarios

A scenario is a file named `<name>.bench.ron` anywhere in the game's
folder (not under `target`, `dist` or a hidden folder). It is a test
input of the tool: no build ships it.

```ron
(
    format_version: 1,
    about: "the foe's turns, after two moves",
    // The game's start options, as `cargo shuyi run` takes them.
    start: [(flag: "--stage"), (flag: "--seed", value: "3")],
    // The steps to play, as `run`'s script: keys, pad buttons, waits.
    script: (steps: [keys("z Up Right z"), wait(1.0)]),
    // Frames to run after the steps, with no input.
    frames: 30,
    // The frames measured: `Every` (the default), or a range. The first
    // frame the game runs is 1.
    measure: Frames(first: 10, last: 60),
)
```

Key and button names in a scenario are the Harness's test-script names,
input to a test like `frame-png`'s steps. A scenario plays at most 600
measured frames. Write one for each of the game's heaviest moments: its
biggest battle, its fullest menu, its longest dialogue with a portrait.
Hello, Grid's are in [`examples/hello-grid/bench/`](../../examples/hello-grid/bench/).

## In CI

The `bench` job runs `shuyi bench` on Hello, Grid on every pull request,
on Linux, with the web package the `export` job made. It runs twice and
checks that every frame's count is the same both times, posts the table
as the job's summary, and fails on any number over its budget. It also
checks the toy's fixtures in `crates/cli/toy/bench/`: a frame that burns
instructions fails on the Playdate naming its frame, and a scenario
holding 11 MB fails on the Playdate and passes on the desktop. And the
toy's long-run fixtures in `crates/cli/toy/long/`: `flat` passes twice,
`leaks` fails naming its memory tag, `hums` fails naming the sounds
alive, and `pid` fails `--twice` at its first sample. Hello, Grid's long
run is weekly, never on a pull request (`long-run.yml`): a failure opens
an issue labelled `long-run`.
