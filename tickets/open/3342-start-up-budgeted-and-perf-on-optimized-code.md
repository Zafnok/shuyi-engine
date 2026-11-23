---
id: "3342"
title: The bench budgets the start-up's instructions, and shuyi run --perf can measure optimized code
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3342 — The bench budgets the start-up's instructions, and shuyi run --perf can measure optimized code

## Context

Found by the engine blind-spot audit of 2026-11-23 (its items 57 and
58, area "Performance, memory and scalability"). Two gaps in how the
engine measures itself, both checked against the code on 2026-11-23
(`main` at `74736aa8`).

**The start-up is counted and never gated.**

- Every target profile declares `budgets.startup_ms`
  (`crates/assets/src/target.rs`, `Budgets`, line 223; `targets/*.ron`:
  2,000 on `desktop`, 3,000 on `steam-deck`, `android` and `ios`, 5,000
  on `web` and `playdate`). Only tests read it. `docs/targets/README.md`
  says it is "measured on the device only", and ADR-0008 §6 leaves it to
  the device tickets (3011, 3013).
- `shuyi bench` (ADR-0081, ticket 3008) already counts the start.
  `bench-play` (`crates/cli/src/bench.rs`, `play`) calls
  `process_heap()` once before the scenario, so Callgrind's first dump
  is the start. `Built::play` keeps it as `Measured::start_instructions`
  and `bench-report.json` writes it as `Run::start_instructions`
  (`crates/cli/src/bench/report.rs`), documented "never gated".
  The audit said the start dump is thrown away. That is wrong: it is
  written to the JSON. But nothing gates it. The Markdown table never
  shows it. CI's second run doesn't compare it: the `$counts` filter in
  `.github/workflows/ci.yml`'s `bench` job reads only `.frames[]`.
- So a change that doubles the work before the first frame passes every
  gate: decoding every image up front, building a font atlas twice, a
  slower registry. Time to the first frame is the first thing a web
  player sees. Consoles certify it.
- The heap needs nothing new. `HeapCounts::peak_bytes` is "the most
  bytes live at once since the program started" (`crates/ui/src/perf.rs`),
  so frame 1's peak already holds the start's, and `peak_heap_bytes`
  gates it.

**Frame times only come from unoptimized code.**

- `shuyi run --perf` (2553) and the live link's `attach --perf-follow`
  (2555) read frame records that exist only where `shuyi_ui::DEBUG_TOOLS`
  is on: `cfg!(any(debug_assertions, feature = "debug-tools"))`
  (`crates/ui/src/lib.rs`, line 111). ADR-0046 §2 says "Debug builds
  only", and its consequences say real numbers come "in an unoptimized
  build". Each answer carries that as a limit: `perf::TIMES`
  (`crates/cli/src/contract/perf.rs`, line 40).
- The engine's CLI runs a game's `run` in its tool program with
  `cargo run --quiet -p <package> …` (`GameProgram::cargo_args`,
  `crates/cli/src/project.rs`), so in the `dev` profile. `run --watch`
  (`crates/cli/src/watch.rs`) runs the window the same way. The
  template's `Cargo.toml.tmpl` gives `opt-level = 2` to dependencies
  only (`[profile.dev.package."*"]`). The game's own crate is built at
  opt-level 0.
- Iterator-heavy Rust is often 10 to 50 times slower at opt-level 0.
  An agent reading `perf` chases hot spots that vanish in release.
- A profile that fits already exists. `bench` builds with
  `--profile shuyi-bench`, given on the command line:
  `inherits="release"`, `debug-assertions=true`, `strip=false`
  (`Built::new`, `crates/cli/src/bench.rs`; ADR-0081 §2). Release code,
  with the heap counter and `DEBUG_TOOLS` on. Only the bench uses it.

No open ticket plans either. Searches of `tickets/open` find no
`startup_instructions`, no `--optimized` and no `shuyi-bench` outside
the done 3008. 3319 compares frames with `main`, and its Out leaves the
start out. 3052 states the web's `startup_ms`, and 3011 and 3013
measure start-up on their devices.

A ticket of the export-targets block: it guards the targets' budgets
(ADR-0008 §6).

## Nick input

None.

## Scope

**In:**
- **A `startup_instructions` budget** (`shuyi-assets`,
  `target::Budgets`): `Option<u64>`, "the most instructions from the
  program's start to the end of its start (the first dump), in the
  bench". `Budgets` is `#[non_exhaustive]` and `#[serde(default)]`, so
  the field is additive (ADR-0039), and a profile without it reads.
- **Each profile's value, provisional,** from the profile's own numbers:
  `frame_instructions × startup_ms ÷ the frame's time in ms`, rounded
  down to two significant figures, with a comment saying so. On `web`,
  first take off the download: ADR-0128 §6 puts about 12 MB before the
  first frame at 25 Mbit/s, about 3,840 ms of the 5,000. That gives:
  - `desktop` 2,300,000,000;
  - `steam-deck`, `android`, `ios` 3,500,000,000;
  - `web` 1,300,000,000;
  - `playdate` 300,000,000 (30 fps, so 33.3 ms a frame);
  - `switch` none (3014).

  It is a proxy as `frame_instructions` is. A device ticket recalibrates
  it, as ADR-0008 §6 says for `frame_instructions`.
- **`shuyi bench` gates it.** `report::Budget` gains
  `startup_instructions`. `What` gains `StartupInstructions`
  (`startup-instructions`). An `Over` names the scenario, with the line
  "`<scenario>` on `<target>`: the start took N instructions, over the
  budget of M". The Markdown table gets a "Start" row per scenario.
  `Run::start_instructions`'s doc loses "never gated".
- **CI's two-run check compares the start too.** The `$counts` filter
  adds `.start_instructions`, and `jq -e` checks it isn't null. If the
  two runs differ (`std`'s `RandomState` seeds a `HashMap` differently
  each run, or something reads the environment or the time), find why,
  then make the start count the same on every run, or say why not. If
  it can't be made the same, the start is reported but not gated, the
  limit says why, and the completion notes name a follow-up ticket.
- **A toy fixture that fails at start.** `crates/cli/src/toy.rs` gains
  `--burn-start N` (N million loops before the first frame), and
  `crates/cli/toy/bench/burns-at-start.bench.ron` sets it over the
  Playdate's start budget and within a desktop's. The CI step "The
  toy's fixtures fail where they must" expects it to fail on `playdate`
  and pass on `desktop`.
- **`--optimized`** on `shuyi run` and `shuyi run --watch`. The
  launcher that builds the game's tool program or window program builds
  it in the `shuyi-bench` profile: the same `--profile` and `--config`
  arguments `bench` passes, moved into one shared function (for
  example `pub(crate) fn bench_profile_args() -> Vec<String>` beside
  `bench::PROFILE`). Both `bench` and the launcher call it. The paths:
  - `game::run_in_game_program` for `run`, through `GameProgram`;
  - `watch::plan` for `run --watch`, which today refuses every argument
    but `--watch`.

  The launcher removes `--optimized` before it forwards the arguments.
  Without it, nothing changes.
- **The answer says which build it measured.** The launcher sets an
  environment variable for the child, `SHUYI_PERF_PROFILE=shuyi-bench`.
  The child's `perf_of` (`crates/cli/src/contract/run.rs`) reads it.
  It then gives the limit "times are this machine's, from an optimized
  build with debug assertions on (the shuyi-bench profile): compare
  runs on the same machine" in place of `perf::TIMES`. Under `run
  --watch --optimized` the launcher prints "measuring an optimized
  build (shuyi-bench)" before the window opens. `attach` runs in
  another process and can't read the window's variable, so its answers
  keep `perf::TIMES`, whose "unless it says otherwise" covers it. The
  variable never reaches a rule or a screen (ADR-0046 §1). Only
  `shuyi-cli`'s measuring code reads it.
- **`--optimized` given to a program that is already running it**
  (the game's tool program called directly, with no launcher) is an
  error with a usage line: "--optimized is read by `shuyi run`, which
  builds the game; this program is already built". Exit code 2.
- **Docs:** `docs/targets/README.md` (the budgets list: what
  `startup_instructions` is, and that `startup_ms` stays device-only;
  the `shuyi bench` section), each `docs/targets/<id>.md` budget table
  that lists the budgets, the `run-gates` skill's *The performance
  budgets* section, and the `shuyi run` usage text and the
  CLI reference wherever `--perf` is documented (`grep -rn -- "--perf"
  docs .claude/skills`). Then `pwsh -File scripts/sync-agent-skills.ps1
  -Check`.
- **An ADR** (the `write-adr` skill) amending ADR-0081 §1 (the start is
  gated against `startup_instructions`) and ADR-0046 §2 and its
  consequences (`--optimized` measures release code, and the limit says
  which build). It amends ADR-0008 §6's list of budgets.
- **For the game:** the engine's profiles gain a budget the game's
  `bench` job then gates. Before the PR merges, read the game's latest
  `bench-report.json` from its CI. Fetch the artifact by id
  (`gh api repos/Zafnok/visions-of-shuyi/actions/artifacts`), not
  `gh run download`. Check its runs' `start_instructions` against the
  new budgets, and put the numbers in the PR. If one is over, stop and
  say so in the PR: the budget's derivation is wrong, or the game's
  start is. Never raise a budget to fit. The game pins `50472bbc` today.
  `gh search code --repo Zafnok/visions-of-shuyi 'shuyi bench'` finds
  its `bench/` scenarios, gated in CI, `docs/targets.md` and
  `scripts/bench-known-over.sh`. The game's PR that moves the pin
  follows this one. The completion notes get `Behaviour:` lines: the
  start is gated (`shuyi-cli`, `shuyi-assets`), and the perf limit's
  text changes under `--optimized` (`shuyi-cli`). No public Rust API
  breaks. `Budgets` gains a field on a `#[non_exhaustive]` type, and
  `bench`'s types are `pub(crate)`.
- **Either order with 3319:** if 3319 is done, add each run's
  `start_instructions` to its `--baseline` comparison, with the same
  margin, as a `rises` entry named `<scenario> (start)`. If not, nothing
  to do (3319 compares it).

**Out (do not do):**
- Wall time at start as a gate: never (ADR-0081).
- Measuring `startup_ms` on a device, and recalibrating
  `startup_instructions` from it. 3011 (Playdate) and 3013 (Steam Deck)
  measure start-up on their devices, and 3340 on desktops and the web.
  Each sets `startup_instructions` from its measured ratio, as
  ADR-0008 §6 sets `frame_instructions`.
- Making `--optimized` the default for `--perf`. A debug build stays the
  quick rebuild. A follow-up if agents measure the wrong build anyway.
- The MCP server's own `run` tool building optimized code. The host
  program is built once, when the session starts. The CLI flag is the
  way to an optimized run.
- A release build with traces (no debug assertions): the heap counter
  and `DEBUG_TOOLS` need them (ADR-0046 §2).
- Labelling the live link's records with their build: `attach` keeps
  `perf::TIMES`. A follow-up if an agent misreads a window's numbers.
- The draw path's costs (3341) and the web's real frame times (3340).

**Estimated size:** about 1,100 lines (code 450, tests 400, docs and
the ADR 250).

**Split:** past 1,000 lines, split off `--optimized` (steps 7–9) into a
new ticket; the start-up budget lands alone.

## Implementation steps

1. Read `crates/cli/src/bench.rs`, `bench/report.rs`, `bench/callgrind.rs`,
   `bench/tests.rs`, `crates/cli/src/toy.rs`, the `bench` job in
   `.github/workflows/ci.yml`, ADR-0081, ADR-0046 and ADR-0008 §6.
2. `Budgets::startup_instructions` in `crates/assets/src/target.rs`.
   Add its value and a comment to each `targets/*.ron`. Extend the
   profile tests in `target.rs` and `crates/cli/src/contract/target/tests.rs`.
3. `report::Budget::startup_instructions`, filled where `bench` builds
   its `Budget`s. `What::StartupInstructions`, its `Over::text`, and the
   check in `over` beside the frame check. The Markdown "Start" row.
4. `--burn-start` in the toy, and `burns-at-start.bench.ron`.
5. CI: `$counts` and the null check take the start. Add the toy's new
   fixture. Run the job, and make the start agree across two runs
   (the Scope says what to do if it can't).
6. The game's start counts against the budgets (the Scope).
7. The shared `bench_profile_args`. `bench` calls it, and its tests
   still pass unchanged.
8. `--optimized` in `run_in_game_program`'s path and in `watch::plan`.
   Strip the flag, add the profile arguments, set `SHUYI_PERF_PROFILE`.
   The refusal when the tool program gets the flag directly.
9. The limit's text from `SHUYI_PERF_PROFILE` in `run.rs`'s `perf_of`,
   picked by one function in `contract/perf.rs`. The line `run --watch
   --optimized` prints.
10. Docs, the skills, the ADR; sync the skill loaders.
11. The gates.

## Acceptance criteria

- [ ] A profile with `startup_instructions` reads, and one without it
      still reads.
- [ ] `bench --target playdate` fails on `burns-at-start` with "the
      start took", naming the scenario, and `bench --target desktop`
      passes on it. CI's toy step checks both.
- [ ] A start exactly at the budget passes, and one instruction over it
      fails (a unit test on hand-made counts).
- [ ] `bench-report.json` lists a `startup-instructions` entry in `over`
      for an over-budget start, and `bench-report.md` has a "Start" row
      for each scenario.
- [ ] CI's two runs compare `start_instructions` and agree on Hello,
      Grid. If they can't agree, the ungated limit and the follow-up
      ticket the Scope asks for are there.
- [ ] Hello, Grid's start passes every supported profile. The PR gives
      its count and the game's against each budget.
- [ ] `shuyi run --perf --optimized` on Hello, Grid builds in
      `target/shuyi-bench`. Its answer has the optimized limit and not
      `perf::TIMES`. Without `--optimized` the answer is as before.
- [ ] `shuyi run --watch --optimized` builds the window program in the
      `shuyi-bench` profile (the planned cargo arguments, in a unit test).
- [ ] The game's tool program given `--optimized` directly exits 2 with
      the usage line.
- [ ] `bench`'s cargo arguments are unchanged (its existing test).
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not (a CI gate and a CLI flag; the
      targets doc carries it).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/assets`): `startup_instructions` read from a profile,
  absent, and an unknown neighbour still refused.
- Unit (`crates/cli/src/bench/tests.rs`): the start's gate at, under and
  over the budget; no budget, no gate; an uncounted start
  (`--no-instructions`) is not gated and says so; the "Start" row.
- Unit (`crates/cli`): `bench_profile_args`; `watch::plan` with and
  without `--optimized`; the launcher's arguments for `run --perf
  --optimized` (the flag stripped, the profile added); the limit's text
  with and without `SHUYI_PERF_PROFILE`.
- Integration: CI's toy fixture step, and the two-run comparison.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
