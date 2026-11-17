---
id: "2061"
title: "`cargo xtask api` runs from a Windows worktree"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-13
---

# 2061 — `cargo xtask api` runs from a Windows worktree

## Context

`cargo xtask api` (ADR-0038) is the local copy of CI's `api` job. In the
retro of 2026-11-08, sessions working from a git worktree on Nick's
Windows machine reported that it "couldn't run locally (a Windows path
error in its base copy)": #271, #291, #293 and #295. Every one fell back
on CI. #292's API failure was then found only in CI, and that PR was
closed and redone. #296 (2041) also saw `cargo-semver-checks` crash on
Windows and wrote "if it comes back, it is worth a ticket". This is that
ticket.

## Nick input

None.

## Scope

**In:**
- Reproduce from a worktree under `.claude/worktrees/` on Windows with
  the GNU toolchain.
- Fix the base copy so it works there. The likely suspects are a
  `git worktree add` or `git archive` path handed to a tool as an MSYS
  path, a path over `MAX_PATH`, or a `\\?\` prefix.
- If `cargo-semver-checks` itself crashes, record the version and the
  backtrace in the notes, and work around it in xtask. No issue
  upstream (`AGENTS.md`, rule 12).
- `run-gates`' *The public API* section says it works from a worktree.

**Out (do not do):**
- Changing what the `api` job checks.

## Implementation steps

1. Run `cargo xtask api --base origin/main` from a worktree and save the
   full error. Find the base-copy code in `crates/xtask/src/api.rs`.
2. Fix the path handling. Two known traps on this machine are where to
   look first: Git Bash (MSYS) rewrites a `/`-prefixed argument into a
   Windows path, and Windows tools want a Windows path (`pwd -W`, not
   `$PWD`).
3. Add a test for the path conversion where the code allows one.

## Acceptance criteria

- [x] `cargo xtask api --base origin/main` completes from a worktree
      under `.claude/worktrees/` on Windows (output pasted in the notes).
- [x] The `cargo-semver-checks` crash, if there is one, is worked
      around in xtask, its version and backtrace in the notes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the path conversion, if one is added.

## Completion notes

**What failed.** Not the base copy itself: `git worktree add` into
`target/api-base` works from a worktree. The build that failed was
`cargo-semver-checks`'. It builds each crate through a placeholder crate
in `target/semver-checks/local-<crate>-<version>-<host>-<16 hex>/`, in
that placeholder's own `target/`, about 90 characters below the
repository's root. A build script there is
`…/target/debug/build/<dependency>-<16 hex>/build-script-build.exe`, and
Windows starts no program at a path of 260 characters or more, even with
`LongPathsEnabled` set (it is, on this machine). So it depends on how long
the worktree's name is: the worktrees here run 43 to 77 characters, and
the longest build-script path was 218 from this worktree's 69. That is why
some sessions failed and others (#296's fourth run, 2026-11-11 and -12)
passed.

Reproduced with a worktree whose root was 106 characters,
`cargo xtask api --base origin/main`, `cargo-semver-checks` 0.51.0:

```text
error: `cargo semver-checks` failed on shuyi-core without naming a break:
    Building shuyi-core v0.1.0 (current)
error: running cargo-doc on crate 'shuyi-core' failed with output:
error: failed to run custom build command for `proc-macro2 v1.0.107`
Caused by:
  could not execute process `D:\shuyi-engine\.claude\worktrees\zz-probe-2061-a-very-long-worktree-name-to-push-paths-over-max-path-aaaa\target\semver-checks\local-shuyi_core-0_1_0-x86_64_pc_windows_gnu-6888883aaf82cf7f\target\debug\build\proc-macro2-4d31d48d40c814d4\build-script-build` (never executed)
Caused by:
  The system cannot find the file specified. (os error 2)
```

The file was there; with `.exe` its path is exactly 260 characters.

**The fix** (`crates/xtask/src/api.rs`): the semver check runs with
`CARGO_TARGET_DIR` set to `target/api-semver`. `cargo-semver-checks` then
builds both sides there instead of in each placeholder's own folder, so a
build script's path is `<root>/target/api-semver/debug/build/…`: about 90
characters shorter. It has no `--target-dir` flag; it follows the variable
(checked by hand first). The listings' folders (`target`,
`target/api-base-target`) were already short enough. No path conversion was
needed: no MSYS path, `\\?\` prefix or `git archive` was involved.

**Evidence**, after the fix, from that same 106-character worktree and from
this one (69), both against `d34b6245` (the tip of `main`):

```text
## Public API

Against `d34b6245b1c1da2070c837657d05a33458f2fca3`.

| Crate | Items removed | Items added | Semver findings |
| ----- | ------------- | ----------- | --------------- |
| `shuyi-core` | 0 | 0 | 0 |
| `shuyi-assets` | 0 | 0 | 0 |
| `shuyi-ui` | 0 | 0 | 0 |
| `shuyi-platform` | 0 | 0 | 0 |
| `shuyi-app` | 0 | 0 | 0 |
| `shuyi-cli` | 0 | 0 | 0 |
| `shuyi` | 0 | 0 | 0 |
| `shuyi-sound` | 0 | 0 | 0 |
| `shuyi-sound-out` | 0 | 0 | 0 |
| `shuyi-playdate` | 0 | 0 | 0 |
| `shuyi-alloc-count` | 0 | 0 | 0 |
| `shuyi-script` | 0 | 0 | 0 |

No `--pr-branch`: the rule was not checked.

api: ok
```

**The `cargo-semver-checks` crash.** Version 0.51.0. 2041 saw a panic in
its repro hint after the baseline build of `shuyi-sound-out` failed. No
panic came back here: the failing build printed its repro hint normally,
so there is no backtrace to record. The failed build under it is the one
above, which the fix removes; nothing else needed working around. No issue
upstream (`AGENTS.md`, rule 12).

Tests: `the_semver_check_builds_in_a_short_target_folder_of_its_own`
(`crates/xtask/src/api/tests.rs`) checks that the semver check gets
`CARGO_TARGET_DIR=<root>/target/api-semver`, and that a build script's path
there stays under 260 from a root of 120 characters. The fake runner now
records the variables a command is given.

Deviations: the ticket's suspects (an MSYS path, `MAX_PATH` in the base
copy, a `\\?\` prefix) were not it; the long path was in
`cargo-semver-checks`' own build, so no path conversion was added. A
half-removed `target/api-base` (2041's notes) did not happen in four runs
here.

Docs: `run-gates`' *The public API* says it runs from a worktree, and why
the semver check has its own target folder; `api.rs`'s docs say the same.

Book: none, the public API of no crate changed; only `xtask` did, which no
game calls.

No break: only `xtask` changed, and it is never shipped.

Nick input: none.
