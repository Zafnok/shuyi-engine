---
id: "2009"
title: "Fast compiles: a game's rebuild time measured, a CI gate that keeps it down, and a fast dev setup in the templates"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: done
blocked_by: ["2513"]
nick_input: none
completed: 2026-10-27
---

# 2009 — Fast compiles

## Context

Nick, 2026-10-20 ([`docs/decisions.md`](../../docs/decisions.md)): every feature
on Bevy's homepage, "like hot reload, fast compile". Bevy's homepage
says a change rebuilds in 0.8 to 3.0 seconds with its "fast compiles"
setup: its `dynamic_linking` feature (the engine as one dylib), a fast
linker (lld or mold), and, on nightly, Cranelift and shared generics.

What a game's developer waits for is the **rebuild after a one-line
change in their own game**, not the engine's clean build. Nothing
measures it today. The engine's dependencies are few (macroquad and its
own small crates), which helps, but nothing keeps them few.

This ticket measures first, then adds what helps on the stable
toolchain the repository pins (`rust-toolchain.toml`), and gates it so
it stays fast.

## Nick input

None.

## Scope

**In:**
- **`cargo xtask compile-times`** (`crates/xtask/src/compile_times.rs`):
  - in a temporary copy of `examples/hello-grid`, it builds `dev`, then
    appends a comment line to the game's `src/main.rs` and builds
    again, three times, and reports the clean build and the median
    rebuild in seconds;
  - the same for `cargo check`, which is what an editor runs;
  - it counts the crates in a new game's dependency tree (`cargo
    metadata` on a project from `shuyi new blank <dir>`);
  - prints a table, or JSON with `--json`.
- **Faster, on stable:**
  1. **A fast linker** where the platform has one: `rust-lld` or
     `mold` on Linux, `rust-lld` on Windows GNU if it links the game
     (it may not with MSYS2's libraries: measure), the default on
     macOS. Written into the templates' `.cargo/config.toml`
     (`templates/`, the `shuyi new` output) behind a comment saying
     how to turn it off. If the Linux linker it writes there isn't
     part of rustup (`mold`, or `clang` driving `rust-lld`), add its
     package to the table in `crates/cli/src/contract/doctor/linux.rs`
     and to the `APT`, `DNF` and `PACMAN` lines of every
     `scripts/setup-linux.sh` (2513). Otherwise CI's bare-container
     `templates` job and the copy tests fail.
  2. **A `dynamic` feature** like Bevy's `dynamic_linking`: a
     `shuyi-dylib` crate (`crate-type = ["dylib"]`) that re-exports the
     engine's crates, used by a game's `dev` builds only. Windows DLLs
     have a 65 535-symbol export limit that Rust dylibs can hit, and
     Bevy documents the same: measure it on Windows GNU. If it fails
     there, the feature is off on Windows and the docs say why.
  3. Whatever the measurement shows is worth it among: `opt-level` per
     package in `dev` (today the root `Cargo.toml` builds dependencies
     at 2; check whether 1 rebuilds faster at an acceptable frame
     rate), `debug` settings, `split-debuginfo` on Linux and macOS,
     `incremental` for workspace crates. The templates get what helps.
- **The gate:** a CI job on `ubuntu-latest` and `windows-latest` runs
  `cargo xtask compile-times --json` and fails if:
  - the median rebuild is over the budget in `tools/compile-budget.ron`,
    which this ticket sets at the measured value plus 50 % headroom for
    runner noise;
  - a new game's dependency count grows past the budget's number
    without the budget changing in the same PR (so a new dependency is
    a visible choice).
  The job writes the table into the run's step summary.
- **Either order with 2907:** if 2907 is done, nothing to do (its
  research note timed each candidate's rebuild with
  `cargo build --timings`); if not, nothing to do (2907 times them
  with `cargo xtask compile-times`).
- **The book** (2802, if done; else a section in `README.md`, which
  2802 moves into the book, as its clause says): a page "Fast compiles"
  with the numbers and the setup, and the nightly-only options
  (Cranelift, `-Zshare-generics`) named as things a developer may try
  themselves.

**Out (do not do):**
- moving the repository to nightly;
- running the measurement on Nick's machine in CI terms (it runs where
  CI runs; locally it is a tool anyone may run);
- hot reload of code (2905).

## Implementation steps

1. Write the tool and run it on CI runners to get the baseline.
2. Try each option, one at a time, recording each one's numbers in the
   PR description.
3. Keep what helps, in the templates and the root `Cargo.toml`. A
   Linux linker outside rustup also goes in 2513's table and setup
   scripts.
4. Write the budget and the CI job.
5. Write the page, and run the gates.

## Acceptance criteria

- [x] `cargo xtask compile-times` prints the clean build, the rebuild,
      the `check` rebuild and the dependency count.
- [x] The PR description has a table of each option's effect on the
      rebuild, on both runners.
- [x] A game made with `shuyi new` gets the setup that helped, and
      CI's bare-container `templates` job still passes with it.
- [x] CI fails when the rebuild passes the budget (shown by a PR-local
      test run with the budget set to 0 s) or when the dependency count
      grows.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: parsing `cargo` output and `cargo metadata`; the budget check
  (over, under, count grown) on fake numbers.
- CI: the gate itself.

## Completion notes

Done, with what the measurements showed (ADR-0049 has the table):

- `cargo xtask compile-times` (`crates/xtask/src/compile_times.rs`)
  prints the clean build, the median rebuild and `check` rebuild after
  a one-line change in `main.rs`, and a new `blank` game's dependency
  count (144), as a table or JSON; `--budget`, `--summary` and `--dir`
  serve CI. The xtask's `make_game` (`skills.rs`) is now `pub` for it.
- CI's `compile-times` job runs it on `ubuntu-latest` and
  `windows-latest` against `tools/compile-budget.ron`, with the table in
  the step summary, and `ci-result` needs it.
- The template's `[profile.dev]` has `debug = false`: about a third off
  each rebuild on both runners. Nothing else helped, so nothing else
  was kept.
- The page is `README.md`'s *Fast compiles* (2802 moves it into the
  book; its clause says so now). `docs/ROADMAP.md`'s row and the
  website's card say done and Ready.

Deviations:

- The tool times a game made with `shuyi new grid` (with the engine's
  `Cargo.lock`), not a copy of `examples/hello-grid`. Hello, Grid lives
  in the engine's workspace, with the engine's profiles and
  `.cargo/config.toml`: a copy would need its manifest rewritten into a
  workspace that is no game's, and wouldn't measure the templates'
  setup, which is what the gate protects.
- **No `dynamic` feature and no `shuyi-dylib`.** It was built and
  measured: it links on Windows GNU and MSVC (no export-limit failure)
  but made no rebuild faster (Linux 1.15 s against 1.09–1.52, MSVC 2.84
  against 2.61–3.25, Windows GNU 1.70 against 1.32). Step 3 keeps only
  what helps.
- **No linker line in the templates**, so nothing joins `doctor`'s
  table or `setup-linux.sh`: Rust has linked with `rust-lld` on Linux
  by default since 1.90 (ld.bfd: 1.80 s), `mold` was no faster (1.33
  s); on Windows GNU `rust-lld` fails on `winapi`'s import libraries; on
  MSVC it was no faster (2.75 s).
- `opt-level = 1` for dependencies made no rebuild faster (only the
  runner's speed moved it), so they stay at 2 and the frame rate stays
  as it was. `split-debuginfo = "unpacked"` (1.58 s) and
  `incremental = false` (1.33 s; 3.09 s on MSVC) were slower.
- The root `Cargo.toml` keeps `debug = "line-tables-only"`: the
  engine's test failures are read from CI's logs, where a backtrace's
  lines are worth more than a second a rebuild (ADR-0049 §4).

No break: the xtask isn't shipped, and the template change is a profile.
No follow-up tickets. Nothing for Nick, beyond: a new game rebuilds in
about a second on Linux and two on Windows, and CI keeps it so.
