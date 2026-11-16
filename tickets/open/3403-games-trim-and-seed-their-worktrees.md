---
id: "3403"
title: "A game trims its idle worktrees and seeds a new one from a warm one: `cargo shuyi trim-targets` and `seed-target --into`, and the shuyi-worktrees skill"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: todo
blocked_by: ["2076", "2078"]
nick_input: none
completed:
---

# 3403 — A game trims its idle worktrees and seeds a new one from a warm one: `cargo shuyi trim-targets` and `seed-target --into`, and the shuyi-worktrees skill

## Context

Nick, 2026-11-14: once the worktree track is done, the template should
ship it. The engine is meant to be driven by coding agents, so the
developers of other games will use Codex or Claude Code too, each in
many worktrees. The disk work done here for the engine's own worktrees
should reach them as well. 3402 covers the compression hook and the
checks. This ticket covers the other two pieces:

- **Trimming (2076).** `cargo xtask trim-targets` deletes the
  incremental caches of idle worktrees, never a worktree. On Nick's
  machine that is 57% of each `target/`.
- **Seeding (2078).** `seed-target` starts a new worktree's `target/`
  from a warm one: third-party output hard-linked, the workspace's own
  crates built fresh. It buys a faster first build.

Both are engine-only today: `cargo xtask` exists only in this
repository. A game's tools are `cargo shuyi`, which builds and runs
the game's own tool program (the template's alias, `run -q -p <game>
--example tools --features tools`). 2076 and 2078 write their walks in
`shuyi-cli` (their **For 3403** notes), so this ticket adds the
commands, not the logic.

Seeding runs **from the warm checkout, into the new one**
(`--into <worktree>`). Run inside a new worktree, `cargo shuyi` would
first build the tool program there, the slow build seeding saves.

**From 2078 (done):** `shuyi_cli::worktrees::seed::seed(donor, into,
path_packages)` is the whole seed, and `path_packages_of(root)` asks
`cargo metadata` for the packages to leave out; xtask's `seed-target`
(`crates/xtask/src/main.rs`) is the wrapper to copy. A `target/` that
isn't empty is **not** refused: a session's MCP server builds there as
the session starts, so the seed keeps what the target has and adds the
rest (`Report::kept`). Its refusals are `Error::Locked` (a build running
in either worktree, the lock file named), `Error::OtherVolume`,
`Error::NoDonorBuild`, `Error::SameWorktree` and `Error::NoWorktree`;
each gets its problem code here. So step 6's worked example refuses a
folder that isn't a worktree (`NoWorktree`) instead of one that isn't
empty.

Like `repos` (`crates/cli/src/repos.rs`), both act on folders outside
the game: other worktrees' build output. So they are commands of the
CLI only (`COMMANDS` in `crates/cli/src/lib.rs`), never operations of
the contract or tools of the MCP server.

## Nick input

None.

## Scope

**In:**
- `shuyi trim-targets [--idle-days N] [--dry-run] [--stale-hashes]
  [--merged]` and `shuyi seed-target --into <worktree>`, run on the
  current project's repository, calling `shuyi_cli::worktrees` (2076,
  2078). Both need no game, so they work from the game's tool program
  and from the `shuyi` binary alike.
  - Each answers with `--json`, for the skill's worked example.
  - Each is in `COMMANDS` and `usage`.
  - Neither is in `Op::ALL` or the MCP server.
- A game's target folders: the walks' list of where `incremental/`
  lives covers a game's own build output (`target/debug`, a triple's
  `target/<triple>/debug`, the web build's `wasm32-unknown-unknown`),
  with no engine-only folder (`gates/`, `tutorial/`,
  `skill-examples*`). 2076's walk takes that list as a parameter (its
  note).
- The skill `skills/shuyi-worktrees` (`SKILL.md`, `example.ron`):
  - when to seed: in a new worktree, before its first build;
  - the command from the new worktree, in Bash and PowerShell. It runs
    in the main checkout, the first line of `git worktree list
    --porcelain`, with `--into` set to the new worktree;
  - when to trim: after the gates pass, and when the disk is low;
  - what it never does: remove or archive a worktree;
  - that the same works under Codex's and Claude Code's worktree
    folders, since both are `git worktree`s.
- `shuyi new` copies the skill like the rest of the pack (it is not a
  module's skill). The skill-pack list in `skills/README.md` names it.
- The template's `AGENTS.md`: the two commands in the tools table, and
  one line at the end of the gate list ("then `cargo shuyi
  trim-targets`").
- Docs, by the `document` skill: `docs/contract.md`'s CLI-only
  commands, beside `repos`, and the book's page on a game's tools if it
  lists them.
- `pub const COMMANDS: [&str; N]` grows by two, which changes its type:
  a `Breaking:` line in the completion notes (ADR-0038), unless the
  change makes it a slice.

**Out (do not do):**
- The walks themselves, and the ADR on build output (2076, 2078).
- A Claude Code or Codex hook that seeds on its own. `AGENTS.md` says a
  shared instruction never relies on one harness's hooks, so the skill
  says what to run.
- A git hook that seeds or trims (2078's Out: never from a git hook).
- Removing, archiving or pruning a worktree.

**Estimated size:** about 900 lines (code 200, tests 300, the skill and
docs 400).

**Split:** past 1,000 lines, split off the skill and its worked example
(steps 5 and 6) into a new ticket.

## Implementation steps

1. `crates/cli/src/lib.rs`: add `trim-targets` and `seed-target` to
   `COMMANDS`, `usage` and the dispatch beside `repos`. Each parses its
   flags and calls `worktrees::trim` or `worktrees::seed` (2076, 2078)
   for the repository of the project's root (`git rev-parse
   --show-toplevel`).
2. The game's target list (**In**), passed to the trim walk. xtask's
   wrapper passes the engine's own list. Check that 2078's seed already
   covers only the host triple's `debug/`, which is the same in a game.
3. Text and `--json` answers: what was deleted and the space freed;
   for seed, what was linked, copied, kept and left to rebuild. Each
   refusal (a locked donor or target, another volume) is an
   error with a problem code from `codes` (new codes in `shuyi
   explain`'s table, the problem-codes rule of 2509).
4. Tests (below).
5. `skills/shuyi-worktrees/SKILL.md` (frontmatter as the pack's others;
   `cargo shuyi check-skills skills` passes).
6. `skills/shuyi-worktrees/example.ron`, on a game fresh from `shuyi
   new`:
   - `trim-targets --dry-run --json`: one worktree, the current one, so
     nothing to trim;
   - `seed-target --into` a folder that isn't a worktree, refused with
     its code.
   A real seed needs a second worktree, which the runner doesn't make:
   the skill says so, and step 7 checks it by hand.
7. By hand on Nick's machine, in the notes: in a game made with `shuyi
   new` and put in git, `git worktree add`. Then `cargo shuyi
   seed-target --into` it from the main checkout, then `cargo test
   --workspace --no-run` in the new worktree: its `Compiling` lines, and
   the time against a cold build. Then `cargo shuyi trim-targets
   --dry-run` from either.
8. Docs and the template's `AGENTS.md`. Then `pwsh -File
   scripts/sync-agent-skills.ps1` if the pack's loaders need it.

## Acceptance criteria

- [ ] `cargo shuyi trim-targets --help` and `seed-target --help` work in
      a game fresh from `shuyi new`, and neither is an MCP tool or a
      contract operation (`shuyi schema` unchanged).
- [ ] In a game's worktree, `trim-targets` deletes only `incremental`
      folders under the idle worktrees' `target/`. `git worktree list`
      is the same before and after (a test with temp worktrees).
- [ ] `seed-target --into` from a game's main checkout gives a new
      worktree whose first `cargo test --workspace --no-run` compiles
      only the game's own crates (the notes), with the donor's files
      unchanged.
- [ ] `shuyi-worktrees` passes `check-skills`, and its worked example
      passes `cargo xtask skill-examples`.
- [ ] A new game's `.claude/skills/` has `shuyi-worktrees`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the flags of each command parsed, and their refusals with
  their codes.
- Unit: the game's target list gives the `incremental` folders of a
  game's layout (temp folders) and none of the engine's.
- Worked example: `example.ron`, run in CI by `skill-examples`.
- Manual, in the notes: step 7.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
