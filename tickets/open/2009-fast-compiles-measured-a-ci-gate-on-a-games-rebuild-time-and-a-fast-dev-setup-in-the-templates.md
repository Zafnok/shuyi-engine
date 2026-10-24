---
id: "2009"
title: "Fast compiles: a game's rebuild time measured, a CI gate that keeps it down, and a fast dev setup in the templates"
type: infra
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2513"]
nick_input: none
completed:
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

- [ ] `cargo xtask compile-times` prints the clean build, the rebuild,
      the `check` rebuild and the dependency count.
- [ ] The PR description has a table of each option's effect on the
      rebuild, on both runners.
- [ ] A game made with `shuyi new` gets the setup that helped, and
      CI's bare-container `templates` job still passes with it.
- [ ] CI fails when the rebuild passes the budget (shown by a PR-local
      test run with the budget set to 0 s) or when the dependency count
      grows.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: parsing `cargo` output and `cargo metadata`; the budget check
  (over, under, count grown) on fake numbers.
- CI: the gate itself.

## Completion notes

*(Filled in by the session that completes the ticket.)*
