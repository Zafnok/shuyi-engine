---
id: "2368"
title: "Parked: Memory in the debug view: the heap and its peak, allocations per frame, and loads by asset kind"
type: feature
milestone: "Engine: debug view"
model: sonnet-5
effort: high
status: blocked
blocked_by: ["2315", "2316", "3008"]
nick_input: none
completed:
---

# 2368 — Memory in the debug view

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"; on 2026-10-20 he kept parked the gaps the game doesn't need): the game doesn't ask for it, so this waits until after the MVP (`docs/ROADMAP.md`). Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8, and the 2D engines GameMaker and Defold. Each one shows
a developer where memory goes while the game runs:

- **Godot:** the debugger's Monitors show static memory and the counts
  of objects and resources
  (<https://docs.godotengine.org/en/stable/tutorials/scripting/debug/debugger_panel.html>).
- **Unity:** the Memory Profiler package takes snapshots of a running
  player.
- **Unreal:** Memory Insights traces allocations
  (<https://dev.epicgames.com/documentation/en-us/unreal-engine/memory-insights-in-unreal-engine>).
- **Bevy:** `SystemInformationDiagnosticsPlugin` reports the process's
  memory
  (<https://docs.rs/bevy/latest/bevy/diagnostic/struct.SystemInformationDiagnosticsPlugin.html>).
- **Defold:** the profiler lists the loaded resources by type and size
  (<https://defold.com/manuals/profiling/>).
- **GameMaker:** the debugger draws a memory graph.

**What Shuyi has today** (read on 2026-10-21):

- Nothing in `crates/`, `examples/` or `templates/` counts
  allocations. There is no `GlobalAlloc` anywhere.
- `crates/ui/src/debug.rs`: the debug menu, the glyph sampler and the
  scene camera. No memory readout.
- `crates/assets/src/format.rs`: the registry's loads go through
  `Formats::decode` (line 600): `Formats::load` (line 678),
  `load_under` (line 697) and `load_each` (line 730) through `read`,
  and `ImageTable::from_files_with` (`crates/assets/src/image.rs:256`).
  `ImageTable::from_files` reads PNG files without the registry, and a game
  may parse a file itself; neither is counted here. Nothing records what
  was loaded. A game keeps what it loads in its own data (`GameData`),
  so nothing knows which assets are still in memory.
- `crates/assets/src/bundle.rs:453`: `bundle::watch` sets a
  process-wide `fn` in a `OnceBox` that hears every file read, "for a
  front end that logs or times what it loads". The decode watcher below
  copies it.
- `Formats` is `Send + Sync`: its kinds are `Box<dyn Kind>`, and
  `Kind: Any + Send + Sync` (line 370). A tally kept inside it behind
  `&self` would need a `RefCell` and lose both, so the tally lives in
  the backend.
- `crates/platform/src/app.rs`: `step` hands the game what the backend
  knows each frame (`Game::set_music_playing` and `Game::set_ticks`,
  line 600) before the game's frame. `Harness` (`harness.rs`) runs a
  game in an `App` on `HeadlessPlatform` (`headless.rs`), which has
  builders such as `with_display` and `with_storage`. Hello, Grid's
  `frame-png` starts its Harness with `Harness::new`
  (`examples/hello-grid/src/tools.rs`).
- `crates/app/src/backend.rs`: `Macroquad` implements `Platform`. It
  can't be made in a test (`Macroquad::new` needs macroquad's window),
  so code a test must reach goes in a module of its own.
- `targets/playdate.ron`: `peak_heap_bytes: 10_485_760` (10 MB,
  provisional). A running game reads no profile: only the CLI embeds
  `targets/` (`crates/cli/src/contract/target.rs`).
- The workspace's lints forbid `unsafe_code` (the root `Cargo.toml`).
  `shuyi-app` alone denies it instead, with `#[allow(unsafe_code)]` only
  in `src/pads/web.rs`. `GlobalAlloc` is an unsafe trait, so a counting
  allocator can't live in `shuyi-core`, `shuyi-ui` or `shuyi-platform`.
  A `#[global_allocator]` static of a type that implements it needs no
  `unsafe` of its own.

**What is planned:**

- 2315 and 2316: the debug view's gizmos, channels, `frame-png --debug`,
  pause and step, and the readout.
- 2325 (parked, this ticket's sibling): a frame profiler with a bar
  graph of the last 120 frames. Its Out lists "memory profiling"; this
  is that ticket.
- 3008: a counting allocator for `shuyi bench`, gating peak heap in CI
  against each target's `peak_heap_bytes`. It keeps the allocator "able
  to report per frame". This ticket asked 3008 to write it as a
  reusable type in a crate that allows `unsafe` in one module, so 2368,
  3020 and 3010 share it.
- 3020 (parked): allocations per frame for the GBA and DS budgets,
  through 3008's allocator.
- 3010 and 3011: the Playdate backend, whose global allocator goes
  through the SDK's `realloc`, and its bench build that logs peak heap
  on the device.
- 2906: research on what stays resident; it may decide an asset store.

So CI and the device bench builds measure peak heap. Nothing shows the
heap, the allocations per frame or what the assets cost to a developer
playing the game.

**Why it matters:**

- **The game first.** Visions of Shuyi doesn't ask for this. Its
  Playdate front end reports frame times and stack use, and 3008 gates
  its peak heap in CI. So this is parked. A reason to unpark it: a
  game's build on the Playdate backend (3010) runs out of heap, and the
  bench's one peak number doesn't say where the heap went.
- **Other 2D games** on targets with little memory: the Playdate's
  10 MB heap budget, and the parked GBA (160 KB) and DS (2.5 MB) rows of
  ADR-0012.
- **The end goal** (Nick, 2026-10-19): an agent makes a game "with a
  web ver, a playdate ver". `frame-png --debug memory` lets the agent
  see what a person sees.

The rules it keeps (`AGENTS.md`):

- **The core stays pure** (rule 5). Only backends count. The numbers
  reach the debug view as plain data, as 2325's frame times do, and
  never reach an update. A replay has the same hash with the counter on
  and off.
- **Graphics are a skin** (rule 9). The channel is a list of 2315's
  gizmos; tests of what it shows read that list.
- **Debug tools only.** A game turns the counter on for its debug runs
  with a feature. A shipped build has no counting allocator and reads
  nothing.

## Nick input

None.

## Scope

**In:**
- **The counter in `shuyi-app`**, behind a new feature `debug-memory`,
  off by default. It turns on `shuyi-ui/debug-tools` and the crate that
  holds 3008's counting allocator:
  - a `#[global_allocator]` static of 3008's counting allocator around
    `std::alloc::System`, under `#[cfg(feature = "debug-memory")]`, in
    a new module `crates/app/src/memory.rs`. It needs no `unsafe`; the
    `unsafe impl` stays in 3008's crate. If 3008 left the allocator
    inside the bench binary, first move it to a crate of its own whose
    lints are the workspace's except `unsafe_code = "deny"`, with one
    `#[allow(unsafe_code)]` module and a test like `shuyi-app`'s
    `the_lints_are_the_workspace_lints_except_unsafe_code`;
  - **allocations per frame:** if 3008's allocator counts calls to
    `alloc` and `realloc` (3020 asks for it), use that; if not, add the
    call counter to it here, read and reset in one step, and 3020 uses
    it;
  - `memory::reading() -> Option<MemoryReading>`: the counter's numbers
    and the tally below with the feature, `None` without it. `Macroquad`
    implements `Platform::memory` by calling it, with `budget: None`,
    since it reads no profile at run time;
  - say in `shuyi-app`'s crate docs how a game turns it on: `cargo run
    --features shuyi-app/debug-memory` on native, and on the web a
    feature of the game's own, listed in `shuyi.ron`'s `web.features`,
    built with `cargo shuyi web --<feature>`. Say that a game with its
    own `#[global_allocator]` can't turn it on (two don't link).
- **The reading, as plain data,** in a new module
  `shuyi_ui::debug::memory` (`no_std` with `alloc`, like the rest of
  `shuyi-ui`):
  - `HeapCounts { live_bytes: u64, peak_bytes: u64, allocs: u32 }`,
    where `allocs` is the calls since the last reading;
  - `HeapBudget { target: &'static str, bytes: u64 }`;
  - `KindLoads { kind: &'static str, loads: u32, failed: u32,
    bytes_read: u64 }`, and `LoadTally`, its rows keyed by the kind's
    name in a `BTreeMap`, with `add(kind, bytes, ok)` and `rows()`
    sorted by bytes read (most first), then by name;
  - `MemoryReading { heap: HeapCounts, budget: Option<HeapBudget>,
    loads: Option<LoadTally> }`. `loads` comes only when the tally
    changed since the last reading;
  - `bytes_text(u64) -> String`: under 1,024 bytes as `1023 B`, then
    `KB`, `MB` and `GB` with one decimal, rounded down, worked out in
    integers (`1.0 KB` is 1,024 bytes and `1.0 MB` is 1,048,576, as the
    profiles' comments count them).

  If 2013 is done, these follow its rule for public types
  (`#[non_exhaustive]` and constructors).
- **From the backend to the view:**
  - `Platform::memory(&mut self) -> Option<MemoryReading>` in
    `crates/platform/src/platform.rs`, defaulted to `None`, so no
    backend breaks;
  - `App` asks for it once a frame in `step`
    (`crates/platform/src/app.rs`), only while the game's
    `Ctx::debug_tools` is on, and hands it
    to a new `Game::set_memory(Option<MemoryReading>)` beside
    `set_music_playing` and `set_ticks` in `crates/ui/src/game.rs`;
  - `Game::set_memory` keeps the last 120 heap counts in a ring and the
    last tally, in `Ctx::debug_view` (2315), never saved with it;
  - `HeadlessPlatform::with_fake_memory(self) -> Self`
    (`crates/platform/src/headless.rs`), a builder like `with_display`,
    gives a reading that moves by fixed amounts: on frame *n*, live
    bytes 1,048,576 + 16,384 × (*n* mod 30), the peak the most of those
    so far, allocations *n* mod 7, a budget of 2,097,152 bytes named
    `fake`, and in the first reading only a tally of `image` (3 loads,
    0 failed, 49,152 bytes) and `palette` (2 loads, 1 failed, 2,048
    bytes). `HeadlessPlatform` also counts its `memory` calls
    (`memory_calls()`), for the tests.
- **Loads by kind, through the registry:**
  - `shuyi_assets::format::watch_decodes(watcher: DecodeWatcher) ->
    Result<(), DecodeWatcher>`, set once, in a `OnceBox` like
    `bundle::watch`;
  - `DecodeWatcher = fn(&Decoded<'_>)`, with
    `Decoded<'a> { kind: &'static str, path: &'a str, bytes: usize, ok:
    bool }`, where `kind` is `K::NAME` and `path` is `file.path()`;
  - `Formats::decode` calls it once per call, after it decides, when one
    is set. Nothing else in `shuyi-assets` changes, and `Formats` stays
    `Send + Sync`;
  - `shuyi-app`, with `debug-memory`, sets a watcher when it starts
    (before the game's load) that adds each decode to a `static`
    `std::sync::Mutex<LoadTally>` and marks it changed;
    `memory::reading()` hands the tally when it changed.
- **The `memory` channel** (2315's channels). `Game::frame` offers it
  once a reading has come, and draws it after the screens' gizmos,
  since it belongs to no screen. In the bottom-left corner:
  - one column per frame for the last 120 frames (or as many as the
    buffer's width allows, the latest): a vertical `Line` (role
    `Solid`) for the live heap, scaled so the peak is 32 px tall; under
    it, a strip 12 px tall of `Line`s (role `Path`) for the allocations
    in each frame, scaled to the most of those frames (none when that
    is 0);
  - a `Line` across the graph at the peak (role `Note`), and a `Line`
    at the budget (role `Bounds`) when there is one and it is less than
    twice the peak;
  - a `Label` above them (role `Note`):
    `heap 3.2 MB  peak 4.1 MB  12 allocs/frame`, ending
    `  budget 10.0 MB (playdate)` when there is a budget;
  - above that, a `Label` row (role `Note`) for each of the five kinds
    with the most bytes read, as `image  3 loads  0 failed  48.0 KB
    read`, and a row `2 more kinds` when there are more;
  - every gizmo that would leave the buffer's `pixel_bounds` is left
    out;
  - **either order with 2325:** if 2325 is done, use its ring type and
    its bar drawing, and sit above its graph when both channels are on.
    If not, write the ring here (`debug::memory::Ring<T>`, its length
    set when it is made, 120 here), and 2325 uses it for its 240
    frames.
- **The readout** (2316): when the `readout` channel is on too, the
  heap label's text is also a readout line, after `paused` and 2325's
  frame line when those are there, and before the top screen's own
  lines.
- **`frame-png --debug memory`** works with the fake counter. Where
  2315's `--debug` reaches the game's Harness, `memory` among the
  channels starts it on `HeadlessPlatform::new().with_fake_memory()`
  through `Harness::on`: in Hello, Grid's `tools.rs` and the toy's
  (`crates/cli/src/toy.rs`). The snapshot is then the same on every run.
- **Either order with 3010:** if `shuyi-playdate` is done, its debug
  build wraps its SDK allocator in 3008's counting allocator,
  implements `Platform::memory`, and gives the `playdate` profile's
  `peak_heap_bytes` as its budget, taken the way 3010 takes the
  profile's refresh rate. If it isn't done, 3010 does it, by its own
  either-order clause.
- **The gates:** `cargo test -p shuyi-app --features debug-memory
  --locked` and `cargo clippy -p shuyi-app --features debug-memory
  --all-targets --locked -- -D warnings` join the `run-gates` skill
  and CI's test job, since `--workspace` builds without the feature.

**Out (do not do):**
- frame times and spans (2325);
- a screen that lists every kind and scrolls: a follow-up if five rows
  are not enough;
- what the game's load kept, as a number of its own: a follow-up;
- resident bytes per asset or per kind. A game keeps what it loads in
  its own data, so nothing knows what is still in memory. That waits for
  an asset store owned by the engine, if 2906's ADR decides one (a
  follow-up then);
- the bytes each kind's decode allocated (the counter's change around
  each decode): a follow-up if bytes read are not enough;
- a budget line in `shuyi-app`. It reads no profile at run time, and a
  64-bit desktop heap is only a proxy for a Playdate's; `shuyi bench`
  (3008) gates the proxies. A follow-up after 3053 if a developer asks;
- loads by kind on the Playdate backend (no `std`, so no `Mutex`). They
  are the same as on desktop, apart from per-target layers (3007);
- call stacks per allocation, snapshots, leak diffs, and Tracy's memory
  view;
- GPU and texture memory (after 3034; 2906 measures textures);
- memory in CI (3008, and 3020 for allocations per frame) and on GBA and
  DS devices (3022, 3026). No second CI ticket;
- the reading in the contract's answers or over the live link (2521):
  a follow-up;
- new actions or keys. The debug menu is the only way in.

## Implementation steps

1. Read 2315's and 2316's ADR and code, 2325 if it is done, 3008's
   counting allocator, `crates/assets/src/format.rs` and `bundle.rs`
   (`watch`), `crates/platform/src/platform.rs`, `app.rs`, `harness.rs`
   and `headless.rs`, `crates/app/src/lib.rs` and `backend.rs`, and
   `examples/hello-grid/src/tools.rs`.
2. If 3008's allocator is not yet a reusable type with a call counter,
   make it one (the first bullet of Scope).
3. Add `DecodeWatcher`, `Decoded`, `watch_decodes` and the call in
   `Formats::decode`, with their tests.
4. Add `shuyi_ui::debug::memory`: the types, `bytes_text`, the ring
   and the tally, with unit tests.
5. Add `Platform::memory`, `Game::set_memory`, the call in `step`
   (`app.rs`), and `with_fake_memory` and `memory_calls` on
   `HeadlessPlatform`.
6. Add the `memory` channel's gizmos and the readout line, with the
   property test.
7. Add `debug-memory` to `shuyi-app`: `src/memory.rs` with the global
   allocator, the watcher and `reading`, `Macroquad`'s
   `Platform::memory`, and the crate docs.
8. Start the Harness on the fake for `frame-png --debug memory` in
   Hello, Grid's and the toy's tools. Run it on Hello, Grid, review the
   PNG and commit the snapshot.
9. Add the feature's test and clippy lines to the `run-gates` skill and
   CI.
10. Run the gates.

Size: expect about 600 changed lines. If the diff runs well past that,
loads by kind (step 3, the tally and the kind rows) go to a follow-up
ticket written in this PR.

## Acceptance criteria

- [ ] With the fake counter, `cargo shuyi frame-png --debug memory` on
      Hello, Grid draws the columns, the peak and budget lines, the
      heap label and the two kind rows. The PNG is reviewed, and its
      snapshot is committed.
- [ ] A Harness replay of the toy's grid game has the same state hash
      with the fake counter and the `memory` channel on as with both
      off.
- [ ] With `debug-memory`, a test holds a `Vec` of 1 MB: the reading's
      `live_bytes` and `peak_bytes` are both at least 1 MB, `peak_bytes`
      (read after `live_bytes`) is at least `live_bytes`, and `allocs`
      is at least 1.
- [ ] Without `debug-memory`, `shuyi-app`'s `memory::reading()` is
      `None` (a test with the feature off), and its `#[global_allocator]`
      is under `#[cfg(feature = "debug-memory")]`.
- [ ] With `Ctx::debug_tools` off, `App` never calls
      `Platform::memory` (`HeadlessPlatform::memory_calls` stays 0).
- [ ] A test in `crates/assets` loads a bundle of two kinds, one file
      of them broken, and the watcher hears each decode with its kind,
      bytes and result. A test in `shuyi-app` gives its watcher those
      decodes and gets one `LoadTally` row per kind.
- [ ] `Formats` is still `Send + Sync` (a compile-time test).
- [ ] No `memory` gizmo lies outside the buffer's `pixel_bounds` (the
      property test below).
- [ ] The `run-gates` skill and CI run `shuyi-app`'s tests and clippy
      with `debug-memory`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the ring wrapping; the heap and allocation scales; `bytes_text`
  (`0 B`, `1023 B`, `1.0 KB`, `3.2 MB`, `1.0 GB`); the budget line left
  out at twice the peak or more; the tally's rows and their order; five
  kind rows and `N more kinds`; the heap line's place in the readout
  after `paused`; `watch_decodes` hearing a good and a failed decode;
  the fake's readings on frames 0, 1, 29 and 30.
- Property: no `memory` gizmo lies outside the buffer's `pixel_bounds`,
  for any readings and any console size.
- Snapshot / integration: the `memory` channel with the fake counter on
  Hello, Grid; the replay-hash test; `shuyi-app`'s reading with the
  feature on and off.

## Completion notes

*(Filled in by the session that completes the ticket.)*
