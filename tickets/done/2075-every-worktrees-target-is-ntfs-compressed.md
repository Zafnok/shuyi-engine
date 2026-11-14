---
id: "2075"
title: "Every worktree's target folder is compressed by NTFS: a post-checkout hook and a check in the gates"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-14
---

# 2075 — Every worktree's target folder is compressed by NTFS: a post-checkout hook and a check in the gates

## Context

Each worktree's `target/` is about 25 GB on Nick's Windows machine,
and he runs about 30 worktrees. A study on 2026-11-13 measured NTFS
compression (LZNT1) on a `target/`:

- Set on the folder, it is inherited by every file cargo, rustc and the
  linker write there, including files that replace old ones on
  rebuild. After a cold `cargo test --workspace --no-run`: 5.87 GB
  logical, 3.01 GB on disk (1.95:1), no file left uncompressed. After
  11 edit-rebuild cycles: 1.97:1, still none uncompressed.
- By part: `deps` 2.25:1, `incremental` 1.63:1.
- Build time: cold 208 s against 221 s uncompressed; an edit-rebuild
  median of 50 s against 47 s. Both under heavy load, within noise.
- No administrator rights needed.
- `compact /c /s /i target` does **not** mark the existing subfolders.
  `compact /c /s:target /i /q` does (verified).
- A `post-checkout` hook runs on `git worktree add`, with `$1` all
  zeros, so it can tell a first checkout (verified).
  `core.hooksPath = scripts/git-hooks` is set per clone (`AGENTS.md`,
  *Environment*), so Codex's worktrees of that clone get it too.

Related: 2074 and 2076. 2076 writes the ADR for the whole policy.

## Nick input

None.

## Scope

**In:**
- `scripts/git-hooks/post-checkout`: on a first checkout, on Windows
  only, create `target/` and mark it compressed. It never fails the
  checkout.
- `cargo xtask gates`, on Windows, locally: when `target/` lacks the
  compressed attribute, compress it first, saying so, and go on if
  that fails. This covers worktrees made before the hook, or without
  hooks.
- `AGENTS.md`'s *Environment* line about hooks and the run-gates skill
  say what the hook does.

**Out (do not do):**
- LZX or XPRESS (`compact /exe`): rebuilt files lose it.
- A Scheduled Task, or any change to Windows settings.
- Anything on Linux or macOS (no-op there).

## Implementation steps

1. Write `scripts/git-hooks/post-checkout` (POSIX `sh`, as `pre-push`
   is). If `$1` is all zeros (40 or 64 of them) and `uname -s` starts
   with `MINGW` or `MSYS`: `mkdir -p target`, then
   `MSYS2_ARG_CONV_EXCL='*' compact.exe /c /s:target /i /q >/dev/null
   2>&1`. Always `exit 0`. Git Bash rewrites a `/`-prefixed argument
   into a path, so `MSYS2_ARG_CONV_EXCL` is needed.
2. In `crates/xtask/src/gates.rs`, before the first gate, under
   `#[cfg(windows)]` and not when `CI` is set: read `target/`'s
   attributes (`std::os::windows::fs::MetadataExt::file_attributes`,
   `FILE_ATTRIBUTE_COMPRESSED` = `0x800`). If unset, run
   `compact.exe /c /s:target /i /q` and print one line saying so (it
   can take minutes on an existing 25 GB folder). A failure is a
   warning, not a failed gate.
3. Update `AGENTS.md` (*Environment*) and
   `.claude/skills/run-gates/SKILL.md`, then
   `pwsh -File scripts/sync-agent-skills.ps1`.

## Acceptance criteria

- [x] On Windows, `git worktree add <dir>` leaves `<dir>/target` with
      the `Compressed` attribute (`(Get-Item <dir>\target).Attributes`).
- [x] A build there leaves its files compressed:
      `compact /s:target` reports a ratio of at least 1.5 (output in
      the notes).
- [x] The hook exits 0 when `compact.exe` is missing, and does nothing
      on a branch switch (`$1` not all zeros) or off Windows.
- [x] `cargo xtask gates` compresses an uncompressed `target/` once,
      and doesn't run `compact` when it is already compressed.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the attribute check reads `0x800` (Windows only), and the
  gates' pre-step is skipped when `CI` is set.
- Manual, in the notes: the hook on `git worktree add`, the ratio after
  a build.

## Completion notes

What was done:

- `scripts/git-hooks/post-checkout` (POSIX `sh`): when `$1` is 40 or
  64 zeros and `uname -s` is `MINGW*` or `MSYS*`, it runs `mkdir -p
  target` and `compact.exe /c /s:target /i /q` (with
  `MSYS2_ARG_CONV_EXCL='*'`), and always exits 0.
- `crates/xtask/src/compress.rs`: `ensure` reads `target/`'s attributes
  (`MetadataExt::file_attributes`, `0x800`); when unset it prints one
  line, runs `compact.exe /c /s:<target> /i /q`, and judges the result
  by the folder's attribute afterwards (`/i` may exit non-zero past the
  running `xtask.exe`). A failure is a `warning:` line. `gates::run`
  calls it before the first gate.
- Unit tests: `0x800` is the attribute; the check is Windows-only and
  skipped whenever `CI` is set; on Windows, a temp folder reads
  compressed after `compact /c` and not after `compact /u`.
- `AGENTS.md` (*Environment*) and the run-gates skill say what the hook
  and the pre-step do; `sync-agent-skills.ps1` reports the loaders
  current.

Deviations:

- The pre-step runs on a full `cargo xtask gates` only: `--list` runs
  nothing, and `--fast` (the pre-push hook) shouldn't hold a push for
  minutes. The full run builds the workspace, so it is the one that
  needs it.

Manual evidence (Windows 11, at `42e5f9a0`, this branch's first commit;
main's tip then was `4ce11a3b`, the branch's base):

- `git -c core.hooksPath=<this branch>/scripts/git-hooks worktree add
  --detach %TEMP%\wt2075 HEAD`: `(Get-Item target).Attributes` was
  `Directory, Compressed`.
- The hook with a non-zero `$1` (a branch switch): exit 0, no `target/`
  made. With `PATH=/usr/bin` (no `compact.exe`): exit 0.
- A build there, `compact /s:target`:
  - `cargo build -p xtask -p shuyi-core`: 1049 files, all compressed,
    305,083,060 bytes in 212,301,162, 1.4:1 (a small build, mostly
    `incremental`, which the study measured at 1.63);
  - then `cargo test --workspace --no-run`: 12645 files compressed,
    0 not; 6,587,632,122 bytes stored in 3,439,654,677, **1.9:1**.
- `cargo xtask gates --base HEAD` in this worktree, with `target/`'s
  attribute cleared (`compact /u target`): run 1 printed
  `== target/ is not compressed: compressing it with NTFS first …` and
  left `Directory, Compressed`; run 2 printed nothing before the gates.
  With `CI=true`, or `--fast`, nothing ran and the attribute stayed
  clear.

Notes for Nick:

- This clone's `core.hooksPath` is the absolute
  `D:\shuyi-engine\scripts\git-hooks`, so new worktrees run the hook
  from the main checkout: it starts working once that checkout is on a
  `main` that has this change. Worktrees made before that are
  compressed by their next full `cargo xtask gates`.

Book: none, this is the repository's own tooling (a git hook and
`cargo xtask gates`); no public item, contract or format changed, and
the book is for people making games.

No break. Game: nothing to adopt, the change is this repository's
tooling only (`gh search code` in the game for `post-checkout` and
`compact.exe`: no hits).
