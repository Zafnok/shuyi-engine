---
id: "2077"
title: "Dev and test programs are built without debug info, but shuyi-cli's"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2077 — Dev and test programs are built without debug info, but shuyi-cli's

## Context

Every test and example program links its whole dependency graph, so a
worktree's `target/debug/deps` holds about 3 GB of `.exe` (96 of them,
31.8 MB on average, up to 104 MB), measured on 2026-11-13. On Windows
GNU the debug info (`[profile.dev] debug = "line-tables-only"`) is
inside each `.exe`.

`strip = "debuginfo"` in `[profile.dev]` cut the test programs from
2.78 to 1.84 GB over a full gates run, and every gate passed (measured).
On disk with 2075's compression that is about 0.4 GB per worktree, so
this is the smallest of the target-size tickets (2074, 2075, 2076).

What stays: a panic's `file:line` (it is compiled in; the `crash_hook`
test passed). What goes: `file:line` in backtrace frames of the
stripped programs.

2520's acceptance needs a `shuyi-cli` test program that carries DWARF on
Windows GNU, so `shuyi-cli` keeps its debug info.

## Nick input

None.

## Scope

**In:**
- `strip = "debuginfo"` in the root `[profile.dev]`.
- A `[profile.dev.package.shuyi-cli]` override keeping its debug info.
- **Either order with 2520:** if 2520 is done, check that its DWARF
  test still passes with this change; if not, nothing to do (2520's
  test will run under the override).

**Out (do not do):**
- The templates' `Cargo.toml` (ADR-0049's compile budget).
- `strip = "symbols"` (backtraces would lose function names).
- `split-debuginfo` (ignored on windows-gnu, measured).

## Implementation steps

1. Root `Cargo.toml`: add `strip = "debuginfo"` under `[profile.dev]`
   with a comment on what it keeps and drops.
2. Add `[profile.dev.package.shuyi-cli]` with `strip = "none"`. If cargo
   refuses `strip` in a package override, find another way that keeps
   `shuyi-cli`'s test program's DWARF on Windows GNU and Linux (say
   which in the notes).
3. Check a panic in a test still names its `file:line`, and that a
   backtrace in a `shuyi-cli` test still names `file:line`.
4. Measure `target/debug/deps/*.exe` before and after one
   `cargo test --workspace --no-run`.

## Acceptance criteria

- [x] `shuyi-cli`'s test program still carries DWARF (`objdump -h`
      shows `.debug_line`), and another crate's test program doesn't.
- [x] A failing test still prints its panic's `file:line`.
- [x] The notes give the `.exe` total before and after.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Manual, in the notes: the `objdump -h` check on two test programs,
  and a panic's output.

## Completion notes

What was done:

- Root `Cargo.toml`: `strip = "debuginfo"` under `[profile.dev]` (the
  `test` profile inherits it), with a comment on what it keeps and
  drops, and `[profile.dev.package.shuyi-cli]` with `strip = "none"`.
  Cargo takes `strip` in a package override, so no other way was needed;
  it is a link setting, so it applies to the programs `shuyi-cli` links
  (its unit tests, its `it` tests, the `shuyi` binary), on Windows GNU
  and Linux alike.
- 2520 was still open: its "either order" clause now says the override
  is there and its DWARF test relies on it (**From 2077 (done)**).

Sizes, `target/debug/deps/*.exe` after one `cargo test --workspace
--no-run` in this worktree (logical size, before NTFS compression):

| | Programs | Total |
| --- | --- | --- |
| Before | 48 | 2.06 GB |
| After | 47 relinked | 1.23 GB |

The 48th before (`shuyi-*.exe`, 65.6 MB) was the `shuyi` MCP server's
own build, which the test build doesn't relink; without it, 1.99 to
1.23 GB, 38 % less. `shuyi_cli`'s test program stays 112 MB.

Manual checks (Windows GNU, on `34960150`, `main`'s tip):

- `objdump -h`: `shuyi_cli-*.exe` (unit tests) has `.debug_line`,
  `.debug_info` and four other `.debug_*` sections;
  `shuyi_core-*.exe` has no `.debug_*` section. `crash_hook-*.exe`
  had them before and has none after.
- A throwaway failing test (`panic!` in an `#[inline(never)]` function,
  deleted after) in `shuyi-core` and in `shuyi-cli`, run with
  `RUST_BACKTRACE=1`. Both print the panic's place:
  `panicked at crates\core\tests\zz_2077_probe.rs:8:5`. The backtrace
  of `shuyi-core`'s names its frames (`zz_2077_probe::inner`) with no
  `at file:line`; `shuyi-cli`'s names them with `at
  .\tests\zz_2077_probe.rs:9:5`.

Deviations: none.

Book: none, no public API changed; the book's fast-compiles chapter is
about a game's own `Cargo.toml` (the templates, left out by this
ticket), not the engine's workspace.

Game: nothing to adopt, the root profile only builds this repository;
a game's build reads its own `Cargo.toml`.

Nick input: none.
