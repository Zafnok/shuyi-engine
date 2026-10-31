---
id: "2026"
title: Main passes CI on Rust 1.99 again (clippy and the api job), and a Rust bump is checked on today's main before it merges
type: bug
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-23
---

# 2026 — Main passes CI on Rust 1.99 again (clippy and the api job), and a Rust bump is checked on today's main before it merges

## Context

Main (`5e7eb6b`) fails CI's `clippy` job
(`cargo clippy --workspace --all-targets --locked -- -D warnings`), CI run
40013380384. Two PRs merged 23 seconds apart:

- #84 (ticket 2010, the public API gate) added tests in `crates/xtask`
  that check emptiness with `assert!(x.is_empty())`;
- #83 (Dependabot's `rust-toolchain` bump from 1.98.1 to 1.99.1, worked with
  the `bump-rust` skill) brought clippy 1.99, whose new `assert_is_empty`
  lint fires on those asserts.

#83's CI ran on a base from before #84 (`af366f5`), so its `clippy` passed,
and nothing made it run again on the main it merged into: main's ruleset
requires no status checks. The `bump-rust` skill runs clippy on the
branch (its step 2) but never on today's main right before the merge.

## Nick input

None.

## Scope

**In:**
- the ten `assert_is_empty` sites in `crates/xtask/src/api/tests.rs` and
  `crates/xtask/src/tickets.rs`, fixed as clippy suggests: an
  `assert_eq!`/`assert_ne!` against an empty value;
- a last step in the `bump-rust` skill's checklist: before merging, bring
  the branch up to date with main and let CI run again if main moved;
- the `api` job's nightly (`NIGHTLY` in `crates/xtask/src/api.rs` and its
  line in `ci.yml`), still `nightly-2026-05-23`, a 1.98 nightly. With
  every `rust-version` at 1.99, cargo refuses it ("rustc 1.98.0-nightly
  is not supported"), and the `api` job fails on main and on every PR
  (PR #88's run 40014469887). The `bump-rust` skill's step 3 says to move
  it on a new minor; #83 skipped that step.

**Out (do not do):**
- `#[allow]` or `#[expect]` on the asserts, or turning the lint off
  (`AGENTS.md` rule 6, the `bump-rust` skill's Don'ts);
- changing main's ruleset to require up-to-date branches: a repository
  setting, ticket 2016's (governance and the repository settings).

## Implementation steps

1. In `crates/xtask/src/api/tests.rs` and `crates/xtask/src/tickets.rs`,
   replace each `assert!(x.is_empty())` with `assert_eq!(x, <empty>)`
   (`Vec::<T>::new()` or `""`), and `assert!(!x.is_empty())` with
   `assert_ne!(x, "")`.
2. In `.claude/skills/bump-rust/SKILL.md`, add a checklist step:
   merge on today's main.
3. Move `NIGHTLY` and `ci.yml`'s `rustup toolchain install` line to the
   first 1.99 nightly that `cargo-public-api` 0.52.0 reads, and say in
   the skill's step 3 how to find it. Run `cargo xtask api`.
4. Run the `run-gates` skill.

## Acceptance criteria

- [x] `cargo clippy --workspace --all-targets --locked -- -D warnings`
  passes on Rust 1.99.1.
- [x] No `#[allow]` or `#[expect]` added.
- [x] The `bump-rust` skill's checklist says to check the bump on today's
  main before it merges.
- [x] The `api` job's nightly is a 1.99 nightly, and `cargo xtask api`
  passes on it.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: none new; the changed asserts are tests, and they still pass.

## Completion notes

- The ten asserts now compare with an empty value; the tests check the
  same thing, and an `assert_eq!` failure now prints what was there.
- The `bump-rust` skill gets step 7, "Merge on today's main".
- `NIGHTLY` moves from `nightly-2026-05-23` to `nightly-2026-07-06`
  (rustc 1.99.0-nightly 3659db0d3), the first 1.99 nightly by the dist
  manifests. Every 1.99 nightly writes rustdoc JSON format 60, not the 57
  that `cargo-public-api` 0.52.0 was built for, and there is no newer
  release. 0.52.0 reads format 60 for the engine's crates: `cargo xtask
  api` passed locally on it. If a later format stops parsing, `PUBLIC_API`
  moves as step 3 says. ADR-0038 §3 names the old nightly as the pin
  of its day and says the pins move with the toolchain, so it stays as it
  is. The test of `is_the_nightly` now builds its near-miss from `NIGHTLY`
  instead of a dated literal.
- The skill's step 3 now says how to find a nightly's version.
- No break. No docs change beyond the skill (the `document` skill's map
  puts how-we-work rules in the skills).
- Main's ruleset holds only `deletion` and `non_fast_forward`, so any PR
  can merge on a stale base, not only a Rust bump. Requiring status checks
  is a repository setting; ticket 2016 covers the settings.
