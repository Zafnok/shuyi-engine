---
id: "3403"
title: "A game trims its idle worktrees and seeds a new one from a warm one: `cargo shuyi trim-targets` and `seed-target --into`, and the shuyi-worktrees skill"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: high
status: done
blocked_by: ["2076", "2078"]
nick_input: none
completed: 2026-11-16
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

**From 2076 (done):** the trim is `shuyi_cli::worktrees::trim::run(repo,
&options, out)`, writing a line per worktree to `out` and returning a
`Report` (worktrees idle and kept, bytes freed, files skipped,
notes, `summary()`). `Options::new(roots)` takes the roots: `""` is
`target/` itself, so `[""]` covers a game's `target/debug`,
`target/<triple>/<profile>` and the web build's
`target/wasm32-unknown-unknown/release`. `Report` has no `Serialize`
yet: this ticket adds it for `--json`.

**From 2092 (done):** `Options::stale_hashes` deletes the stale hashes
in the roots' `deps/` (ADR-0123), and `Report::stale` counts them:
`shuyi trim-targets --stale-hashes` sets the one, and `--json` shows
the other.

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

- [x] `cargo shuyi trim-targets --help` and `seed-target --help` work in
      a game fresh from `shuyi new`, and neither is an MCP tool or a
      contract operation (`shuyi schema` unchanged).
- [x] In a game's worktree, `trim-targets` deletes only `incremental`
      folders under the idle worktrees' `target/`. `git worktree list`
      is the same before and after (a test with temp worktrees).
- [x] `seed-target --into` from a game's main checkout gives a new
      worktree whose first `cargo test --workspace --no-run` compiles
      only the game's own crates (the notes), with the donor's files
      unchanged.
- [x] ~~`shuyi-worktrees` passes `check-skills`, and its worked example
      passes `cargo xtask skill-examples`.~~ (not done: moved to 3405 by
      the split; it passed at the PR's first commit, `96ecf2ef`)
- [x] ~~A new game's `.claude/skills/` has `shuyi-worktrees`.~~ (not
      done: moved to 3405 by the split)
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the flags of each command parsed, and their refusals with
  their codes.
- Unit: the game's target list gives the `incremental` folders of a
  game's layout (temp folders) and none of the engine's.
- Worked example: `example.ron`, run in CI by `skill-examples`.
- Manual, in the notes: step 7.

## Completion notes

What was done:

- `shuyi trim-targets [--idle-days N] [--dry-run] [--stale-hashes]
  [--merged] [--json]` and `shuyi seed-target --into <worktree>
  [--json]`, each with `--help`: `crates/cli/src/worktrees/command.rs`,
  dispatched from `run` in `crates/cli/src/lib.rs` beside `repos`, so
  they work in `shuyi` and in the game's tool program alike. Both are
  in `COMMANDS` and the usage, and neither is in `Op::ALL` or the MCP
  server (`shuyi schema` is unchanged: no schema names them or the new
  codes).
- They act on the repository of the project's root (`git rev-parse
  --show-toplevel`), and refuse (`SH1010`) a game in none, or in one
  that ignores its `shuyi.ron` (a game made in another repository's
  `target/`, as the worked examples' are).
- `seed-target` first checks that `--into` is one of the folders `git
  worktree list` lists (`SH0007` otherwise), so it never makes a
  `target/` in a folder that isn't a worktree. Then
  `seed::path_packages_of` and `seed::seed`, each refusal with its code:
  `SameWorktree` `SH0008`, `NoDonorBuild` `SH1011`, `Locked` `SH3016`,
  `OtherVolume` `SH3017`, `Io` `SH3004`. Six new codes, each with its
  row and page in `crates/cli/errors/`.
- `--json` answers `{ok, result, code, error, help}`. `trim::Report` and
  `seed::Report` derive `Serialize`, flattened into the result beside
  the worktree's lines (`worktrees`), the `summary`, and for the seed
  `donor` and `into`.
- The game's target list is `trim::GAME_ROOTS`, `[""]`: `target/debug`,
  `target/release`, `target/<triple>/<profile>` and the web build's.
  Step 2's check: the seed covers only the host's `target/debug/`
  (`seed::PROFILE`), the same in a game.
- Docs: `docs/contract.md` (the CLI-only commands, beside `repos` and
  `reveal`), the template's `AGENTS.md` (its worktrees section, the
  two rows of the tools table, and "then `cargo shuyi trim-targets`"
  after the gate list), and the book's quick start (a paragraph after
  the hooks).

Deviations:

- **Split at the ticket's own valve.** At 1,323 counted lines the PR
  was past the **Split:** line (1,000), so the skill, its worked example
  (steps 5 and 6) and what only they need went to 3405: the runner's
  `Git` step, `skills/README.md`'s entry, and the mentions of the skill
  in the template and the book. The whole draft is in this PR's first
  commit, `96ecf2ef`, where `cargo xtask skill-examples
  shuyi-worktrees` passed (4 steps: `git init`, `trim-targets --dry-run`
  with `idle` 0 and `kept` 1, `seed-target --into crates` refused with
  `SH0007`, `--into .` with `SH0008`); 3405 starts from it. The two
  acceptance criteria of the skill are left unticked here, and 3405
  carries them. Size: 1,088 counted lines after the split, under 1,500;
  no further split is clean, since the commands, their codes and their
  tests land together.
- **The worked example needs the game in git.** The runner makes each
  game in the engine's `target/skill-examples/`, which is in no git
  repository of its own, so the example the ticket planned couldn't
  find "one worktree, the current one". The draft added a `Git` step to
  the runner (`Git(args: ["init", "-q"])`); it moves with the example
  to 3405.
- **A folder inside `target/` that is a target folder of its own is not
  a triple.** With the game's one root, `""`, the walk's
  `<root>/<triple>/<profile>` form also matched the engine's
  `tutorial/debug` and `skill-examples-build/debug`, which the ticket's
  test says a game's list must not reach. `under_roots` now refuses that
  form when the middle folder holds cargo's `.rustc_info.json`, which
  cargo writes at the top of every target folder and never in a
  triple's. The engine's trim is unchanged: xtask names those folders
  as roots of their own (a test checks both lists).
- `COMMANDS` became a slice, `&[&str]`, so the next command changes no
  type (the ticket's option).
- **Step 6's refusal.** The worked example refuses `--into` a folder
  that exists and isn't a worktree (`crates`) with `SH0007`: the
  command checks the worktree list itself, where `seed::seed` alone
  would have seeded any folder.

Manual check (step 7), on Nick's machine (Windows 11, GNU toolchain),
at `96ecf2ef`, with that commit's `target/debug/shuyi.exe` (a game
fresh from `shuyi new` depends on the engine's `main`, whose `cargo
shuyi` doesn't have the commands until this merges; the binary runs
them the same way):

- `shuyi new moon --template grid --no-github`, then `git init`, `git
  add -A` and a commit in it.
- The main checkout's cold `cargo test --workspace --no-run`: 55 s, 112
  `Compiling` lines.
- `git worktree add ../moon-wt -b wt`, then from `moon`, `shuyi
  seed-target --into ../moon-wt`: "linked 278 files (539 MiB), copied
  811 files (1 MiB), rewrote the root in 24 build outputs, kept 0 units
  and files already there, left out 12 fingerprints of path packages
  (they rebuild)". Run again with `--json`, it linked nothing and kept
  551: a second seed is harmless.
- In `moon-wt`, the first `cargo test --workspace --no-run`: 7 s, and
  its only `Compiling` lines are the game's own crates, `moon` and
  `moon-rules` (against 55 s and 112 cold).
- From `moon-wt`, `shuyi trim-targets --dry-run`: `moon` "target 752
  MB, built 0 hours ago (in the last 3 days), left alone", `moon-wt`
  "this worktree, left alone", "would free 0 MB in 0 idle worktree(s),
  left 2 alone". `git worktree list` still listed both.
- The donor's files unchanged: the new worktree's build writes new
  files over the links (2078's `the_donors_files_are_unchanged` test
  checks it), and the newest rlibs in `moon/target/debug/deps` were
  still the cold build's.
- `shuyi trim-targets --help` and `shuyi seed-target --help` print their
  usage and flags.

Follow-up tickets: 3405 (the skill and its worked example, split off).

Nick input: none.

Breaking: shuyi-cli: `COMMANDS`: its type is `&[&str]`, not `[&str; 17]`; a game that iterates it by value calls `.iter().copied()`

```migration
since: 0.1.0
what: shuyi_cli::COMMANDS is a slice, so a new command changes no type.
old: "pub const COMMANDS: [&str; 17]"
new: "pub const COMMANDS: &[&str]"
before: |
  let names: Vec<&str> = names.chain(shuyi_cli::COMMANDS).collect();
after: |
  let names: Vec<&str> = names.chain(shuyi_cli::COMMANDS.iter().copied()).collect();
find: ["chain(shuyi_cli::COMMANDS)"]
```

Searched the game for it: `gh search code --repo Zafnok/visions-of-shuyi
'shuyi_cli::COMMANDS'` and `'trim-targets'` find nothing.

Behaviour: shuyi-cli: `shuyi` has two new commands, `trim-targets` and `seed-target`, in its usage and `COMMANDS` (a skill may name them), and `shuyi explain` lists six new problem codes: `SH0007` (`not-a-worktree`), `SH0008` (`same-worktree`), `SH1010` (`no-repository`), `SH1011` (`no-donor-build`), `SH3016` (`build-running`) and `SH3017` (`other-volume`)

Behaviour: shuyi-cli: `worktrees::trim` no longer reads `<root>/<folder>/<profile>` as a triple's profile when `<folder>` holds cargo's `.rustc_info.json`; the engine's `cargo xtask trim-targets` names such folders as roots, so it trims the same

Behaviour: templates: a new game's `AGENTS.md` lists `seed-target` and `trim-targets` among its tools, and ends its gates with `cargo shuyi trim-targets`

Book: the quick start's paragraph on git's hooks and worktrees is followed by one on the two commands.
