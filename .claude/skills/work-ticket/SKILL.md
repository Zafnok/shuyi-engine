---
name: work-ticket
description: Pick up, implement, verify, archive and open a PR for exactly one ticket from tickets/open/. Use whenever asked to "work ticket NNNN", "do the next ticket", or start any implementation work in this repository.
---

# Work a ticket

One ticket = one branch = one PR. Do **only** what the ticket says. The
owner (Nick) does not review code; CI and this checklist are the review,
so discipline here is the whole quality system.

## 0. Orient

1. Read `AGENTS.md` and `docs/README.md` (the docs map).
2. Read the ticket fully, then the docs that the map's *What each task
   reads* names for its block, then every ADR, decision and
   game-repository page the ticket links.

## 1. Choose the ticket

- If the user named a ticket, use it, and read neither file below.
- A Dependabot PR or an `advisory` issue is worked with the
  `dependency-upkeep` skill instead (ADR-0037 §5).
- Otherwise read `tickets/README.md` and `docs/ROADMAP.md`, and list
  ready tickets: `status: todo`, every `blocked_by` done.
  - A plain id is done when it is in this repository's `tickets/done/`.
  - A `game#NNNN` id is done when it is in the game repository's
    `tickets/done/`. Check with `gh api
    repos/Zafnok/visions-of-shuyi/contents/tickets/done --jq '.[].name'`.

  Prefer the order in `docs/ROADMAP.md`. Tell the user which one you
  picked and why, in one sentence.
- **Never start a `Parked:` ticket** unless Nick has unparked it. If the
  user asks for one, say it's parked and why (its **Parked** note). Ask
  whether Nick wants it unparked.
- If the ticket needs a product decision that `docs/decisions.md`
  doesn't hold (license, naming, what goes public, scope), stop. Ask it
  with the `ask-nick` skill.

## 2. Start

```bash
git switch main && git pull --ff-only
git switch -c t<NNNN>-<slug>
```

Set `status: in-progress` in the ticket's frontmatter.

## 3. Implement

- Follow the **Implementation steps** in order. If a step turns out to be
  wrong, deviate minimally and explain it in the Completion notes.
- **Respect the layers (ADR-0002):**
  - `shuyi-core` is pure and deterministic, with no floats in a
    simulation;
  - `shuyi-assets` gets bytes and does no I/O;
  - only platform backends (`shuyi-app` today) touch devices;
    `shuyi-platform` is the boundary they implement.
- **Nothing of the game.** No game crate, no game data id, no game asset,
  no bought art (`AGENTS.md` rule 4).
- **Input:** follow the `input-actions` skill (never a hard-coded key).
- **Pictures and maps:** graphics are a skin (ADR-0002 §4). Tests of
  what happened read the scene or the state.
- **A new file format** is a `Format` plugin (ADR-0007), with a
  fuzz-style test and a determinism fixture.
- **Write the tests the ticket lists:**
  - unit and property tests for logic;
  - Harness and snapshot tests for screens;
  - `frame-png` pictures for looks.
- **Scope creep rule:** if you notice something else worth doing, don't
  do it. Write a ticket with the `write-ticket` skill and mention it in
  the PR.
- **Adding a dependency or asset:** check its license against ADR-0002
  §8 *before* using it. Use `default-features = false` and only the
  features you need (traps: ADR-0004 §7). `cargo deny check` must pass,
  and a non-crate asset gets a row in `THIRD_PARTY_ASSETS.md`.
- **A new architectural choice** needs an ADR in the same PR (the
  `write-adr` skill).
- **Public API:** CI's `api` job finds the breaks on every PR, before
  and after the first crates.io release (ADR-0038): a break needs a
  `Breaking:` line (§5). After that release, a break also needs a
  version bump and a line in `CHANGELOG.md`. The game's pin update is a
  game PR.

## 4. Verify

Run the `run-gates` skill. Check a change a player sees through the MCP
session tools (the root `.mcp.json`) on `examples/hello-grid`: `start`,
`step`, `scene`, `frame_png`; not by reading the Harness's code to drive
it. Walk the **Acceptance criteria** and tick each box honestly. If one can't be met, say so in the Completion notes and
the PR.

## 5. Archive the ticket (same PR)

1. **Write the docs.** Run the `document` skill over the change, or say
   in the completion notes why the change needs none.
2. Fill in `## Completion notes`: what was done, deviations, follow-up
   tickets, and anything Nick should know. A ticket that names a game
   ticket (`game#NNNN`) also lists the game PR that will move the pin to
   this change (`Zafnok/visions-of-shuyi#N`), or the game ticket that
   will, if that PR doesn't exist yet.

   Add one `Breaking:` line per break of the public API, a file format
   or a golden value, in the notes (ADR-0038 §6):

   ```text
   Breaking: <crate>[, <crate>…]: <item>: <what a game changes>
   ```

   CI's `api` job fails a break whose crate no line names.
3. Set `status: done` and `completed: YYYY-MM-DD`.
4. `git mv tickets/open/<file>.md tickets/done/<file>.md`
5. **Settle what named this ticket.** Run `grep -rnw "<NNNN>"
   tickets/open`, and for each hit:
   - a clause that waits on it (*either order with NNNN*, *if NNNN is
     done*, *NNNN, if done*): rewrite it as what now holds, and drop the
     other branch;
   - a promised follow-up: write it now (the `write-ticket` skill), and
     point the promise at its number;
   - a `blocked_by` entry: leave it, it is met.

   `cargo xtask ticket-lint` then prints no warning naming NNNN. In a
   parked ticket only the clause changes.

## 6. Commit and PR

- Commit messages: `[NNNN] imperative summary`.
- Title: `[NNNN] <ticket title>`. **No bare `@word`** in a title, body,
  commit message or comment. Use backticks.
- Body: summary bullets, the acceptance criteria (checked), follow-up
  tickets, the ticket's `Breaking:` lines (or "No break"), and a `Nick
  input:` line.
- If the game must change too (a pin bump, a call site), write or link
  the game ticket. Say in both PRs which merges first: the engine's.
- Don't run mutation testing locally; CI runs it. Push, open the PR,
  and read CI once.
- If a check fails, fix it on the branch. Never disable a gate, lower a
  threshold, or add a skip or an allow just to pass.

## Don'ts

- Don't work two tickets in one PR.
- Don't ask Nick technical questions. Decide, and write an ADR if it
  matters.
- Don't edit accepted ADRs; supersede them.
- Don't start parked tickets.
