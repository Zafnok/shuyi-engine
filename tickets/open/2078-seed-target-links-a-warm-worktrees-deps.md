---
id: "2078"
title: "cargo xtask seed-target starts a new worktree's target from a warm one: third-party output hard-linked, the workspace's own crates built fresh"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2078 — cargo xtask seed-target starts a new worktree's target from a warm one: third-party output hard-linked, the workspace's own crates built fresh

## Context

A new worktree builds every third-party crate from nothing, and keeps
its own copy of them. Nick asked whether a worktree could share a warm
build and break the link only where something changed. A study on
2026-11-13 tested this:

- **A shared target (`CARGO_TARGET_DIR`, `build-dir`, a junction) is
  wrong.** Workspace crates get the same hash in every worktree, and
  cargo decides freshness by mtime. In worktree a, cargo said Fresh and
  a's test program listed worktree b's test. One build lock also
  serializes every worktree.
- **Hard links break cleanly for compiled output.** rustc and the
  linker always write a new `.rlib`, `.rmeta`, `.exe`, `.dll`, `.a`,
  `.o` (link count 2 to 1 on rebuild, the donor's file unchanged).
- **cargo rewrites some files in place:** `.fingerprint/*`, `*.d`,
  `build/*/root-output` and `build/*/output`. A plain `cp -al`
  corrupted 31 files in the donor and made the donor rebuild. Those
  must be copied, not linked.
- **A seeded workspace crate that stays Fresh is wrong.** It keeps the
  donor's paths compiled in (`env!("CARGO_MANIFEST_DIR")`,
  `CARGO_BIN_EXE_*`). Measured: `cargo xtask ticket-lint` in the seeded
  worktree linted the donor's tickets, a golden test passed on the
  donor's file, and `SHUYI_BLESS=1` wrote the donor's golden. So the
  workspace packages' fingerprints must be deleted after seeding, and
  they rebuild with their own paths.
- Copying file mtimes isn't enough: folder mtimes matter too, or
  `shuyi-cli` rebuilds anyway.
- With that done, units were Fresh in 2–5 s. Disk saved is small,
  about 1.5 GB per worktree (third-party output is only about 1.5 GB).
  **What it buys is a faster first build.** A prototype is in the
  study's notes (`seed.py`: link the immutable outputs, copy the rest
  with mtimes, rewrite the donor's root in build-script output).

Related: 2074, 2075 and 2076 (the target-size tickets). 2076's ADR
names this as the safe form of sharing.

## Nick input

None.

## Scope

**In:**
- `cargo xtask seed-target [--from <worktree>]`, default the main
  worktree (the first line of `git worktree list --porcelain`), into
  this worktree's `target/`, which must be absent or empty.
- Only `debug/` (`deps`, `build`, `.fingerprint`) of the host triple.
  No `incremental/`, no other triple, no `gates/`, `tutorial/` or
  `skill-examples*`.
- The `work-ticket` skill's start: in a new worktree, run it once
  before the first build.

**Out (do not do):**
- Seeding from a target with a cargo build running (refuse).
- Linking any file cargo rewrites in place.
- A Dev Drive or block cloning (needs administrator rights; the same
  path rules would apply).
- Running it from a git hook.

## Implementation steps

1. New `crates/xtask/src/seed.rs`, wired in `main.rs`.
2. Refuse when the donor's `.cargo-lock` can't be locked
   (`File::try_lock`), when this `target/` isn't empty, or when donor
   and target are on different volumes (hard links can't cross them).
3. Walk the donor's `target/debug/{deps,build,.fingerprint}`:
   - hard-link `.rlib .rmeta .exe .dll .a .o .lib`;
   - copy every other file, keeping its mtime (`File::set_modified`);
   - set each folder's mtime to the donor's after filling it.
4. In copied `build/*/output` and `build/*/root-output`, replace the
   donor's root path with this worktree's.
5. Delete `.fingerprint/<pkg>-*` for every workspace member (`cargo
   metadata --no-deps`, `workspace_members`), so they rebuild with their
   own paths.
6. The `work-ticket` skill and the run-gates skill's notes (`pwsh -File
   scripts/sync-agent-skills.ps1`).

## Acceptance criteria

- [ ] After seeding, `cargo test --workspace --no-run` compiles the
      workspace crates and no third-party crate (its "Compiling" lines
      in the notes), and the time against a cold build is in the notes.
- [ ] The donor's files have the same hashes before and after seeding
      and the seeded worktree's build (all of `deps`, `build`,
      `.fingerprint`).
- [ ] In the seeded worktree, `cargo xtask ticket-lint` reads its own
      tickets (a broken ticket there fails it), and
      `env!("CARGO_MANIFEST_DIR")` in a test is its own path.
- [ ] It refuses a non-empty target, a locked donor, and another
      volume.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit, with temp folders: which files are linked and which copied,
  mtimes kept on files and folders, the root rewrite in `output`, the
  workspace fingerprints deleted and the others kept.
- Unit: each refusal.
- Integration (manual, in the notes): the acceptance's build on Nick's
  machine.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
