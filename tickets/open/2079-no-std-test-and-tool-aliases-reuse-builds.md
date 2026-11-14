---
id: "2079"
title: "The no-std host test and the xtask and shuyi aliases reuse the workspace's build instead of making copies of it"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
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

- [ ] A warning in `shuyi-core` built with `--no-default-features`
      still fails the `no-std` gate (tried and reverted, in the notes).
- [ ] After `cargo build --workspace`, the new no-std test step and
      `cargo xtask --help` compile no third-party crate (notes).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the gates list test that names the no-std steps, updated.
- Manual: the unit counts in the notes.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
