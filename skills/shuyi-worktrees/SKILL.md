---
name: shuyi-worktrees
description: Keep the disk and the first build of a game's many git worktrees in check, as coding agents make them (Codex's and Claude Code's worktree folders included). Seed a new worktree's build from the warm main checkout before its first build (`cargo shuyi seed-target --into`), and trim the incremental caches of idle worktrees after the gates pass or when the disk runs low (`cargo shuyi trim-targets`). Use when starting work in a new worktree, after the gates pass, or when the disk is low. Not for removing, pruning or archiving a worktree, which it never does.
license: MIT OR Apache-2.0
metadata:
  version: "1.0"
---

# A game's worktrees

A coding agent often works in a git worktree of its own, and an agent
running many tasks makes many: Claude Code under `.claude/worktrees/`,
Codex under `~/.codex/worktrees/`. Both are plain `git worktree`s, so
`git worktree list` lists them and the two commands here work on them
alike. Each worktree has its own `target/`, a few gigabytes, and builds
everything from nothing the first time. Two commands keep that in
check:

- `cargo shuyi seed-target --into <worktree>` starts a new worktree's
  `target/` from a warm one: the third-party crates' compiled output
  hard-linked (it takes no disk of its own), the game's own crates left
  to build fresh with the new worktree's paths. Its first build then
  compiles the game's crates, not every dependency.
- `cargo shuyi trim-targets` deletes the incremental caches of the idle
  worktrees' `target/` (no build in 3 days), about half of each. Cargo
  still finds those builds fresh: only the first edit there is slower.

Neither ever removes, prunes or archives a worktree, nor deletes
anything outside a `target/`. Both act on the worktrees of the game's own
git repository, so a game not in git yet is refused (`SH1010`).

## When to use it

- **Seed:** in a new worktree, before its first build.
- **Trim:** after the gates pass (the game's `AGENTS.md` ends its gate
  list with it), and whenever the disk runs low.

## When not to

- Removing, pruning or archiving a worktree: that is the person's call,
  or the harness's, never this skill's.
- A worktree that has built already: seeding it gains little. It is
  safe (the seed keeps what is there), only slower than nothing.
- A worktree on another volume than the main checkout (`SH3017`): skip
  the seed, and let it build from nothing.

## Steps

1. **Seed the new worktree, from the main checkout.** Run in the new
   worktree, the lines below find the main checkout (the first line of
   `git worktree list --porcelain`) and run the seed there, with `--into`
   the new worktree. It runs in the warm checkout because `cargo shuyi`
   first builds the game's tool program where it runs, and in the new
   worktree that is the slow build the seed saves.

   In Bash:

   ```bash
   main=$(git worktree list --porcelain | sed -n '1s/^worktree //p')
   (cd "$main" && cargo shuyi seed-target --into "$OLDPWD")
   ```

   In PowerShell:

   ```powershell
   $main = (git worktree list --porcelain | Select-Object -First 1) -replace '^worktree ', ''
   $here = (Get-Location).Path
   Push-Location $main; cargo shuyi seed-target --into $here; Pop-Location
   ```

   It says what it linked, copied, kept and left to rebuild. The new
   worktree's `target/` may already hold a build started as the session
   began (the engine's MCP server builds there): the seed keeps it and
   adds the rest. It refuses while a cargo build runs in either worktree
   (`SH3016`): wait for it, then run it again.

2. **Build in the new worktree.** `cargo test --workspace --no-run`
   compiles the game's own crates (and the engine's, which are the
   game's path dependencies when it builds from a checkout of the
   engine), and few or no third-party crates.

3. **Trim after the gates pass,** from any worktree of the game:

   ```bash
   cargo shuyi trim-targets --dry-run
   cargo shuyi trim-targets
   ```

   `--dry-run` says what each worktree would lose, with its `target/`'s
   size, and deletes nothing. Each worktree gets a line: trimmed, or why
   it was left alone (this one, no `target/`, a build running, built in
   the last days). With `--json`, the answer's `result` holds the lines
   (`worktrees`) and the counts (`idle`, `kept`, `freed` in bytes).

4. **When the disk is still low,** go further:

   - `--idle-days 1` counts a worktree idle after one day, not three;
   - `--stale-hashes` also deletes, in `deps/`, the older builds of a
     crate that no build read in those days;
   - `--merged` empties all of `target/` of a worktree whose branch is
     merged into `origin/main` (or whose pull request is merged, when
     `gh` is installed). The worktree itself stays.

## Where it looks

The trim reads each worktree's `target/` for cargo's profile folders:
`target/debug`, `target/release`, a triple's
`target/<triple>/<profile>`, and the web build's
`target/wasm32-unknown-unknown/release`. A folder inside `target/` that
is a target folder of its own (it holds cargo's `.rustc_info.json`) is
another build's, and left alone. The seed covers the host's
`target/debug/` only, the folder the tests and the tools build in.

## When it fails

Branch on the answer's `code`, never its message:

- `SH1010`: the game is in no git repository of its own: `git init`,
  or `cargo shuyi repos`, first.
- `SH0007`: `--into` names a folder that is no worktree of the game's
  repository: make it with `git worktree add`, and pass its folder.
- `SH0008`: `--into` names the worktree the command runs in: run it in
  the main checkout instead.
- `SH1011`: the checkout it runs in has never built: build there first,
  or run it in one that has.
- `SH3016`: a cargo build is running in one of the two worktrees: wait,
  then run it again.
- `SH3017`: the two worktrees are on different volumes: skip the seed.
- `SH0001`: a wrong flag: the answer says which, with the usage.

## Worked example

[`example.ron`](example.ron) runs on a new `blank` game, put in git:
`trim-targets --dry-run` finds one worktree, the one it runs in, and
trims nothing; `seed-target --into` a folder that is no worktree is
refused with `SH0007`, and `--into` the game's own folder with
`SH0008`. A real seed needs a second worktree with a build, which the
example doesn't make: the steps above are that run.
