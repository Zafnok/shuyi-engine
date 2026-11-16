---
id: "2078"
title: "cargo xtask seed-target starts a new worktree's target from a warm one: third-party output hard-linked, the workspace's own crates built fresh"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: []
nick_input: none
completed: 2026-11-16
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
- `cargo xtask seed-target --into <worktree>`, run in the warm donor
  (usually the main worktree, the first line of `git worktree list
  --porcelain`), into the new worktree's `target/`, which must be absent
  or empty. Not a `--from` run inside the new worktree: there, `cargo
  xtask` first builds xtask into the very `target/` it must find empty,
  so it would always refuse. The donor's xtask is already built.
- Only `debug/` (`deps`, `build`, `.fingerprint`) of the host triple.
  No `incremental/`, no other triple, no `gates/`, `tutorial/` or
  `skill-examples*`.
- The `work-ticket` skill's start: in a new worktree, before the first
  build, run it once from the main worktree with `--into` this one (the
  command spelled out, in Bash and PowerShell).
- **For 3403:** 3403 gives a game the same command (`cargo shuyi
  seed-target --into`). So the seed goes in `shuyi-cli`
  (`crates/cli/src/worktrees/seed.rs`, public as
  `shuyi_cli::worktrees::seed`), and xtask's command is a thin wrapper
  over it. Write it once there now; moving it later would count every
  line twice in `pr-size`. If `check-layers` asks for it, the edge from
  xtask to `shuyi-cli` gets its line in `tools/layers.ron`.

**Out (do not do):**
- Seeding from a target with a cargo build running (refuse).
- Linking any file cargo rewrites in place.
- A Dev Drive or block cloning (needs administrator rights; the same
  path rules would apply).
- Running it from a git hook.

## Implementation steps

1. New `crates/cli/src/worktrees/seed.rs` (the **For 3403** item), and
   xtask's `seed-target` wired in its `main.rs` over it.
2. Refuse when the donor's `.cargo-lock` can't be locked
   (`File::try_lock`), when this `target/` isn't empty, or when donor
   and target are on different volumes (hard links can't cross them).
3. Walk the donor's `target/debug/{deps,build,.fingerprint}`:
   - hard-link `.rlib .rmeta .exe .dll .a .o .lib`;
   - copy every other file, keeping its mtime (`File::set_modified`);
   - set each folder's mtime to the donor's after filling it.
4. In copied `build/*/output` and `build/*/root-output`, replace the
   donor's root path with this worktree's.
5. Delete `.fingerprint/<pkg>-*` for every path package, so each
   rebuilds with its own paths. A path package is one whose `source` is
   null in `cargo metadata`'s `packages`. Cargo hashes such a package's
   path relative to the workspace root, so it gets the same hash in
   every worktree. In the engine these are the workspace members. In a
   game (3403) they also include an engine checked out inside the game
   (the game's `cargo xtask engine`).
6. The `work-ticket` skill and the run-gates skill's notes (`pwsh -File
   scripts/sync-agent-skills.ps1`).

## Acceptance criteria

- [x] After seeding, `cargo test --workspace --no-run` compiles the
      workspace crates and no third-party crate (its "Compiling" lines
      in the notes), and the time against a cold build is in the notes.
- [x] The donor's files have the same hashes before and after seeding
      and the seeded worktree's build (all of `deps`, `build`,
      `.fingerprint`).
- [x] In the seeded worktree, `cargo xtask ticket-lint` reads its own
      tickets (a broken ticket there fails it), and
      `env!("CARGO_MANIFEST_DIR")` in a test is its own path.
- [x] It refuses ~~a non-empty target~~ a target with a build running,
      a locked donor, and another volume (deviation 1: a non-empty
      target is added to, not refused).
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit, with temp folders: which files are linked and which copied,
  mtimes kept on files and folders, the root rewrite in `output`, the
  workspace fingerprints deleted and the others kept.
- Unit: each refusal.
- Integration (manual, in the notes): the acceptance's build on Nick's
  machine.

## Completion notes

**What was done.**

- `shuyi_cli::worktrees::seed` (`crates/cli/src/worktrees/seed.rs`):
  `seed(donor, into, path_packages)` fills the new worktree's
  `target/debug/{deps,build,.fingerprint}` from the donor's. It
  hard-links `.rlib .rmeta .exe .dll .a .o .lib` (and `.so`, `.dylib`
  for a game off Windows, ticket 3403), copies every other file with
  its modification time, gives each folder its donor's time, rewrites
  the donor's root in `build/*/output` and `build/*/root-output`, and
  leaves out the fingerprint of every path package.
  `path_packages_of(root)` asks `cargo metadata` for those packages
  (`source` null).
- `cargo xtask seed-target --into <worktree>`, a thin wrapper in
  `crates/xtask/src/main.rs`. xtask now depends on `shuyi-cli`; it is no
  engine crate, so `tools/layers.ron` needs no line (`check-layers`
  passes).
- The `work-ticket` skill's §2 runs it at the start of a new worktree
  (Bash and PowerShell), and the `run-gates` skill says what it does and
  why a shared `target/` is wrong.
- 17 unit tests on temp folders: linked against copied, times on files
  and folders, the root rewrite (either separator, case on Windows, not
  inside a longer name), path packages left out and a third-party
  `game-extra` kept beside path package `game`, the donor unchanged, a
  partly built target, and each refusal (both locks, another volume
  through the error a cross-volume link gives, the donor itself, no
  donor build, no worktree).

**Measured on Nick's machine** (24 cores, Windows, GNU toolchain), at
f01f652f, the donor this worktree after `cargo test --workspace
--no-run`:

- Seed into an empty target: 3.8 s. "linked 622 files (2744 MiB),
  copied 1762 files (2 MiB), rewrote the root in 42 build outputs, left
  out 115 fingerprints of path packages".
- Its first `cargo test --workspace --no-run`: 2 min 24 s, 17
  "Compiling" lines, all workspace packages: shuyi-alloc-count,
  shuyi-cli, shuyi-core, shuyi-app, shuyi-sound, cloning-fixture,
  hello-grid, book-snippets, shuyi-playdate, shuyi-script,
  shuyi-sound-out, shuyi-assets, cold-rules-web-fixture, shuyi-ui,
  shuyi-platform, shuyi, xtask. No third-party crate.
- A cold worktree, the same command: 2 min 41 s, 174 "Compiling" lines.
  The gain is small here: with 24 cores the third-party crates build
  beside the workspace's, which set the build's length. It is larger on
  a machine with fewer cores, and the CPU time of 157 crates is saved
  either way.
- As an agent session runs it, after the MCP server's `cargo build
  --bin shuyi` (1 min): the seed kept 325 units and files; the test
  build took 1 min 46 s against 2 min 15 s unseeded, and recompiled 40
  third-party crates (dependents of those the MCP build made, whose
  newer outputs make the donor's dependents stale) and 17 workspace
  ones, against 115.
- The donor's 2,824 files in `deps`, `build` and `.fingerprint` had the
  same SHA-256 and modification times before the seed and after the
  seeded worktree's build. A check after the second seed differed
  only in the donor's own `shuyi-cli` and `xtask`, which the donor
  rebuilt itself because `seed.rs` had been edited.
- In the seeded worktree, a `status: nonsense` in a ticket failed
  `cargo xtask ticket-lint` there (2 errors naming that ticket), and the
  lint passed once it was restored. A test printing
  `env!("CARGO_MANIFEST_DIR")` printed
  `D:\shuyi-engine\.claude\worktrees\seed-2078-a\crates\core`. One
  linked test program (`shuyi_cli-0a9d…`, of the donor's `cargo test -p
  shuyi-cli`, a feature set the workspace build doesn't make) still held
  the donor's path; its fingerprint was left out, so `cargo test -p
  shuyi-cli --lib --no-run` there rebuilt it, and the rebuilt one names
  only the seeded worktree.
- `cargo run` holds none of the three lock files while the program
  runs: seeding from `cargo xtask` in the donor, and into a worktree
  whose MCP server runs, both worked.

**Deviations.**

1. **A target that isn't empty is added to, not refused.** The `shuyi`
   MCP server (`.mcp.json`, and Codex's `.codex/config.toml`) runs
   `cargo run --bin shuyi` as a session starts, so a new worktree's
   `target/debug` has a build in it before the session can seed (this
   worktree had 70 MB). With the ticket's refusal the skill's step
   would always refuse. So the seed keeps a unit whose fingerprint
   folder or build script folder the target has (cargo removes a unit's
   fingerprint before building it, so one interrupted there rebuilds),
   keeps any file already in `deps`, and adds the rest. It refuses only
   while a build runs in either worktree: it locks the donor's lock
   files that exist and the target's (made when missing), all three
   cargo 1.99 keeps (`.cargo-lock`, `.cargo-build-lock`,
   `.cargo-artifact-lock`), and holds them while it seeds.
2. **The workspace fingerprints are never copied**, rather than copied
   and deleted: the same result, with no moment where a stale one sits
   in the target. Each copied fingerprint folder is written under
   `<unit>.seeding` and renamed into place after its unit's files, so a
   seed stopped part-way leaves no unit Fresh without its files. Nothing
   is deleted on a failure, since the target may hold its own build.
3. **`.so` and `.dylib` are linked too**, for a game's worktree off
   Windows (3403); rustc writes them new as it does a `.dll`.
4. The volume check links one of the donor's files into the target and
   removes it: Windows' volume serial number is not in stable Rust.

**Follow-ups.** 3403 and 2076 now say what this left them ("From 2078
(done)"): 3403 refuses on a lock, not on a target that isn't empty, and
its worked example refuses a folder that isn't a worktree; 2076 checks
the same three lock files, and its ADR records the seed as built.

**For Nick.** A new worktree's first full test build is about 30 s
shorter on your machine (from 2 min 15 s to 1 min 46 s). Agents run it
at the start of each ticket.

Size: about 1,330 counted lines; most of it is the module (600) and its
tests (580), which can't land apart. Past 1,000 it would split at the
skills' text (step 6), which needs the command first.

Breaking: none. The PR adds `shuyi_cli::worktrees` and changes nothing
public.

Book: none, the seed is the engine repository's own tool for its
worktrees; a game gets it as `cargo shuyi seed-target` and the
`shuyi-worktrees` skill with ticket 3403, which writes its docs.

Nothing a game uses changes: `worktrees` is a new module, and no
existing item, output or rule moves (searched the game with `gh search
code --repo Zafnok/visions-of-shuyi 'seed-target'`: no hit).
