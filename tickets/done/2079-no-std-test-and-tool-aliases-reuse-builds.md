---
id: "2079"
title: "The no-std host test and the xtask and shuyi aliases reuse the workspace's build instead of making copies of it"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2079 — The no-std host test and the xtask and shuyi aliases reuse the workspace's build instead of making copies of it

## Context

A study of worktree `target/` sizes on 2026-11-13 found two small
sources of copies of the same crates under other hashes:

- The `no-std` gate's last step (`crates/xtask/src/gates.rs:245`) runs
  `RUSTFLAGS="-D warnings -A unused-imports" cargo test ...`.
  `RUSTFLAGS` is part of every unit's hash, so every host dependency of
  those crates is built again: about 0.15 GB (estimate).
- The aliases in `.cargo/config.toml` are `xtask = "run -q -p xtask
  --"` and `shuyi = "run -q -p shuyi-cli --"`. With `-p`, only that
  package's features are unified, so shared dependencies get other
  features and other hashes than in `cargo build`. The study found
  `run -q --bin xtask` and `--bin shuyi` add no new units after
  `cargo build --workspace`.

Related: 2074, 2075, 2076 (the larger target-size tickets).

## Nick input

None.

## Scope

**In:**
- The no-std host test without `RUSTFLAGS`, still failing on a warning
  in a `--no-default-features` build. For example, a `cargo clippy`
  step with `-- -D warnings -A unused-imports` on the same packages
  and features, or a `[lints]` way. Measure that the new form adds no
  hash.
- The two aliases with `--bin` instead of `-p`, if measuring confirms
  they add no units, and xtask's and the CLI's tests still pass.

**Out (do not do):**
- The Playdate step's `RUSTFLAGS="--cfg shuyi_playdate_sdk"` (needed;
  another triple anyway).
- The game templates' aliases.

## Implementation steps

1. Count the units: from a fresh target, `cargo build --workspace`,
   then the current form, then the new form; count `Compiling` lines
   after each (in the notes).
2. Change the no-std step in `crates/xtask/src/gates.rs` and its CI
   twin if there is one (`.github/workflows/`).
3. Change the aliases in `.cargo/config.toml`, keeping their comments.
   Grep the docs and skills for `-p xtask` / `-p shuyi-cli` that say
   what the alias is.

## Acceptance criteria

- [x] A warning in `shuyi-core` built with `--no-default-features`
      still fails the `no-std` gate (tried and reverted, in the notes).
- [x] After `cargo build --workspace`, `cargo xtask --help` compiles
      no third-party crate (notes).
- [x] ~~After `cargo build --workspace`, the new no-std test step
      compiles no third-party crate~~ (not done: 13 third-party units
      remain, the crates' no-`std` feature set the test exists to try;
      down from 54, notes).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the gates list test that names the no-std steps, updated.
- Manual: the unit counts in the notes.

## Completion notes

What was done:

- The `no-std` gate's host test (`crates/xtask/src/gates.rs`) and its CI
  twin (`ci.yml`, "The tests without std") deny warnings with cargo's
  `build.warnings` setting (`CARGO_BUILD_WARNINGS=deny`) instead of a
  `RUSTFLAGS` of their own. The setting is cargo's, outside every unit's
  hash. In CI the workflow's own `RUSTFLAGS: -D warnings` now applies to
  the step too, as to the job's other steps.
- The `xtask` alias in `.cargo/config.toml` is `run -q --bin xtask --`,
  with a comment on why.
- The run-gates skill's no-std block names the new step and why.
- `tools/sound-parity/footprint.ron` (the sound give-up's edits) finds
  the skill's new line.
- A unit test, `the_no_std_host_test_denies_warnings_without_rustflags`,
  checks the gate's one `cargo test` step sets `CARGO_BUILD_WARNINGS=deny`
  and no `RUSTFLAGS`. No earlier test named the no-std steps; this is the
  one the ticket asks for.

Unit counts (`Compiling` lines), one fresh target, each step after the
ones above it:

| step | units |
|---|---|
| `cargo build --workspace --locked` | 148 |
| `cargo run --bin xtask -- --help` | 0 |
| `cargo run --bin shuyi -- --help` | 0 |
| `cargo test --workspace --locked --no-run` | 47 |
| new no-std test (`CARGO_BUILD_WARNINGS=deny`) | 18 |
| old no-std test (`RUSTFLAGS="-D warnings -A unused-imports"`) | 59 |
| `cargo run -p xtask -- --help` (old alias) | 13 |
| `cargo run -p shuyi-cli -- --help` (old alias) | 7 |

The warning: a dead `fn planted_2079() {}` added to
`crates/core/src/lib.rs` made the gate's step fail ("warnings are denied
by `build.warnings` configuration", exit 101); reverted.

Deviations:

- Not a `cargo clippy` step, as the ticket suggested: clippy's check
  units are another set again, while `build.warnings` keeps the one
  `cargo test` and adds nothing.
- `-A unused-imports` is gone: the build gives no warning at all today
  (the dev-dependencies turn `std` on, so the `no_std` code's `Float`
  imports are not compiled there). A future unused import in that build
  fails the gate, as in CI's other jobs.
- Criterion 2 holds for `cargo xtask --help` (0 units) but not wholly
  for the test step: after `cargo build --workspace` and the `test`
  gate's `cargo test --workspace`, it still builds 13 third-party units
  (`serde`, `serde_core` twice, `serde_json`, `num-traits`, `memchr`,
  `bitflags`, `postcard`, `ron`, `schemars`, `tempfile`, `insta`,
  `proptest`) and our five crates. Those are the crates with the
  no-`std` feature set that `--no-default-features` selects, which is
  what the test exists to try; no flag can share them with the `std`
  build. `cargo build --workspace` alone builds no dev-dependency, so
  the `test` gate's build is the fair baseline. The 41 units the old
  form added over that were all `RUSTFLAGS`'s.
- The `shuyi` alias stays `run -q -p shuyi-cli --`: with `--bin`, the
  `wasm` gate's `examples/hello-grid: cargo shuyi check` failed ("no bin
  target named `shuyi` in default-run packages"). From a member's
  folder, cargo looks for the binary in that folder's package only, and
  `cargo shuyi` is meant to run from a game's folder. Its 7 units stay.
  No ticket: removing them would mean giving `shuyi-cli` alone the
  workspace's features, a larger change than 7 small units. The `xtask`
  alias now needs the root, where every gate, hook, CI step and skill
  runs it; from a member's folder cargo names the problem.
- The docs and skills name `-p shuyi-cli` and `-p xtask` only as direct
  commands (`cargo run -q -p shuyi-cli -- lint`, `cargo test -p
  xtask`), none as what the alias is; left as they are.

Follow-up: 2081, the repository's MCP server command (`.mcp.json`,
`.codex/config.toml`, the dev container) still runs `-p shuyi-cli`.

Book: none, no public API, format or behaviour a game sees changed;
this is the engine's own gate and aliases.

No break.
