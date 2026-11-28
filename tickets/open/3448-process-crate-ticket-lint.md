---
id: "3448"
title: "A light shuyi-process crate: the ticket lint and next-id, set up per repository, run by the engine's xtask and by a game's"
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: decision
completed:
---

# 3448 — A light `shuyi-process` crate: the ticket lint and next-id, set up per repository

## Context

Found by the redundancy sweep of 2026-11-28. The game keeps its own
forks of the engine's repository-process tools and has six open tickets
that port more of them, one at a time. The game's pin is `230080db`
(`gh api repos/Zafnok/visions-of-shuyi/contents/shuyi-engine.rev`).

**What the game has today**, in its light process crate (game#0188,
done):
- `crates/xtask/src/tickets.rs`: 1,193 lines, 516 before its tests.
- `pr_size.rs`: 968 lines, 446 before its tests.
- `mentions.rs`: 94 lines.

They share about 40 functions with the engine's copies, by name and by
body: `check_enum`, `scalar_string`, `string_list`, `parse_ticket`,
`check_unique_ids` and `check_blocked_by_exists` in the lint, and
`numstat`, `untracked_lines`, `hook_json` and `runs_a_pr_step` in
pr-size. The engine's `crates/xtask/src/tickets.rs` says it was ported
from the game's lint, and `pr_size.rs` from the game's ADR-0101 tool.
They have drifted apart since.

**What the game plans to port.** Each ticket says it ports a done
engine twin rather than designing one:
- game#0131: ticket numbers and blockers across parallel PRs.
- game#0143: the ticket a PR is for, read from its title (engine 2051).
- game#0144: next-id across open PRs (engine 2052).
- game#0147: done boxes, stale blockers and follow-ups left in prose
  (engine 2023 and 2059).
- game#0181: stub and parked tickets and estimates (engine
  `tickets/size.rs`).

That is about 2,000 planned lines, on top of the 2,161 the game keeps
now. Game#0145 (the gates runner) is 3449's.

**What stopped this before.** game#0145, 0147 and 0181 put "an engine
command for every game" **Out**, as an engine step that needs Nick's OK.
That is this ticket's Nick input. The precedent is 3403, which shipped
`trim-targets` and `seed-target` to games through `shuyi-cli`.

**A light crate, not `shuyi-cli`.** The game's process crate was split
out of its xtask (game#0188) so that the commit hooks build in seconds.
`shuyi-cli` is too heavy for it. So the shared code goes in a new crate
whose only dependencies are `serde`, `serde_norway` and `serde_json`.

## Nick input

**Decision:** may the engine ship its repository-process tools to every
game? These are the ticket lint and next-id here, and pr-size and the
gates runner in 3449. A game made with `shuyi new` would get them as
`cargo shuyi tickets …`, and the game would drop its copies. Ask with
the `ask-nick` skill before step 2. If he says no, the crate is still
the engine's own, and the game keeps porting.

## Scope

**In:**
- **A new crate, `crates/process` (`shuyi-process`)**, with its line in
  `tools/layers.ron`. It is pure apart from reading files and running
  `git` and `gh`, both behind a trait so the tests reach no network.
- **The engine's ticket lint core moves into it**:
  - the front matter and its checks;
  - ids, blocks and the `blocked_by` existence;
  - the either-order clauses and their names back;
  - done boxes, stale blockers and estimates (`tickets/size.rs`);
  - the PR title rules (`tickets/pr.rs`).
- **`next-id` moves into it** (`crates/xtask/src/ids.rs`).
- **A repository's rules become config**, read from one RON file
  (`tools/process.ron` here):
  - the id blocks, the types and statuses;
  - the other repository's prefix (`game#` here, `engine#` in a game);
  - whether a parked ticket is marked by a `Parked:` title or by
    `status: parked`;
  - whether a blocked ticket may have an empty `blocked_by`;
  - the size thresholds.
- **A repository's own rules stay as hooks** it passes in: for the game,
  the Nick-quote warning and the playtest headings. The engine's
  `cargo xtask ticket-lint` and `next-id` keep every message and exit
  code they have today.
- **`cargo shuyi tickets lint` and `cargo shuyi tickets next-id`** for a
  game, through `shuyi-cli` as 3403 did, if Nick says yes.
- **The API is additive:** a new crate. The xtask is not public.
- **Estimated size:** about 1,400 lines (code 500, tests 500, docs and
  config 400). Most of the code is moved, not written.
- **Split:** past 1,500 lines, split the `cargo shuyi tickets` commands
  and their docs off into a new ticket. Keep the crate and the engine's
  use of it here.

**Out (do not do):**
- pr-size and the gates runner: 3449.
- The game's adoption: game#1423. It deletes the game's `tickets.rs`
  and its ports.
- Changing any engine lint rule: the engine's tickets must pass exactly
  as before.

## Implementation steps

1. Create the crate. Move the lint's parsing and checks from
   `crates/xtask/src/tickets.rs` (and `tickets/*.rs`) and `ids.rs`,
   with their tests. The xtask keeps thin callers.
2. Ask Nick (the `ask-nick` skill). Record his answer in
   `docs/decisions.md`.
3. Turn every engine-specific constant into a `Config` field read from
   `tools/process.ron`. Defaults are the engine's values.
4. Read the game's `tickets.rs` (public repository) for the rules it
   has and the engine lacks. Make each one a config field or a hook,
   and port its test cases.
5. If Nick said yes, add `tickets lint` and `tickets next-id` to the
   `shuyi-cli` tool program and the template's alias, and document them
   in the skill pack (`skills/`).
6. Write an ADR (the `write-adr` skill): one process crate for every
   repository.

## Acceptance criteria

- [ ] `cargo xtask ticket-lint` passes and fails on exactly the same
      tickets as before the move.
- [ ] A test repository with the game's config (`engine#` prefix,
      `status: parked`) lints with the game's rules.
- [ ] `shuyi-process` builds with only `serde`, `serde_norway` and
      `serde_json` (`cargo tree -p shuyi-process`).
- [ ] Nick's answer is in `docs/decisions.md`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the moved tests, plus config parsing and the game-shaped
  config's cases.
- Integration: the lint over a fixture `tickets/` folder for each
  config.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
