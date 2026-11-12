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
3. **Check a ticket older than 7 days against the code.** Its age is the
   date of the commit that added it (`git log --diff-filter=A --format=%ad
   --date=short -- tickets/open/<file>.md`). Re-check every type,
   function, file and ticket its steps name:
   - against `main`: does it still exist, under that name, and is it
     still missing where the ticket says it is;
   - against the tickets done since it was written (`git log
     --since=<date> --name-only -- tickets/done`): did one of them build
     it, or rename it.

   Fix the ticket in this PR, before the code, and record each fix as a
   deviation in the Completion notes. Why: four of the six major
   deviations of a retro (2026-11-08) were in tickets 14–16 days old.
   3042 planned a method 2411 had built, and 2350 and 2252 named files
   and loaders that were gone.

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

In a worktree made for you (a `claude/<slug>-<hex>` branch), rename it
**before the first push**: a rename after the PR opens closes the PR on
GitHub.

```bash
git branch -m t<NNNN>-<slug>
```

CI reads the ticket from the PR's title (`[NNNN] …`) first, so a branch
left unrenamed still passes, but a `t<NNNN>-` branch must name the
title's ticket (`tickets/README.md`, *Pull requests*).

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
- **Never open a PR or an issue on another project** (`AGENTS.md`,
  rule 12), even where a ticket asks for one: fix or work around the
  bug here, and say in the Completion notes that it was left out and
  why.
- **Scope creep rule:** if you notice something else worth doing, don't
  do it. Write a ticket with the `write-ticket` skill and mention it in
  the PR.
- **A decision gets its ticket before its code.** A change found along
  the way with anything to decide is written as a ticket first, and the
  code follows it. Its acceptance values come from the requirement,
  never from pasting the code's output, and each bound or clamp it
  changes gets a case that reaches it. Why: 3210 was written after its
  code, inside #294, and its case "23 lines at row 2" copied an
  off-by-one; fixing it took three PRs for about 10 lines.
- **A bug the game's pin move finds** is one PR, its ticket and the fix
  together, when the fix is small (the `write-ticket` skill's
  *A request from the game*). Why: a ticket-only PR followed by a fix
  PR doubles the review and the CI for the same few lines (#305 and
  #313).
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
  game PR. A public type you add, or add to, follows ADR-0039 (2013).

## 4. Verify

Run the `run-gates` skill: `cargo xtask gates` runs every gate the
diff needs and prints a summary block; keep it for the PR's body. Check a change a player sees through the MCP
session tools (the root `.mcp.json`) on `examples/hello-grid`: `start`,
`step`, `scene`, `frame_png`; not by reading the Harness's code to drive
it. Walk the **Acceptance criteria** and tick each box honestly. If one can't be met, say so in the Completion notes and
the PR.

## 5. Archive the ticket (same PR)

1. **Write the docs.** Run the `document` skill over the change, or say
   in the completion notes why the change needs none.
2. Fill in `## Completion notes`: what was done, deviations, follow-up
   tickets, and anything Nick should know. The ticket lint reads the
   notes of the ticket the PR moves to `done/` (ticket 2059):

   - **A follow-up named, not promised.** A line, or a bullet with its
     indented lines, that says "worth a ticket", "for now", "follow-up",
     "later", "not yet", "out of scope" or "TODO" names the ticket that
     does it (write it with the `write-ticket` skill), or starts
     `No ticket: <why>`. Otherwise the lint warns, naming the line.
     Words in `` `code` `` and fenced blocks don't count.
   - **The game's adopter.** A ticket whose body names a game ticket
     (`game#NNNN`) has a `Game:` line naming the game PR that will move
     the pin to this change (`Zafnok/visions-of-shuyi#N`), or the game
     ticket that will (`game#NNNN`) if that PR doesn't exist yet, or
     `Game: nothing to adopt, <why>`. Without one the lint fails.

   It also fails an open ticket, not parked, that is `status: blocked`
   with every `blocked_by` id done: set it to `todo`.

   **Manual evidence** (a browser walk-through, a device run, a
   picture) is recorded after merging the tip of `main` into the
   branch, and the notes name that commit. Why: #308 (2817) tested a
   revision a day old and drafted a corrective ticket for a bug 2576
   (#304) had already fixed.

   Add one `Breaking:` line per break of the public API, a file format
   or a golden value, in the notes (ADR-0038 §6):

   ```text
   Breaking: <crate>[, <crate>…]: <item>: <what a game changes>
   ```

   CI's `api` job fails a break whose crate no line names.

   Beside them, when the public API changed (a break or an addition) and
   the PR changes no file under `site/content/learn/` or `site/snippets/`,
   add a `Book:` line saying why the book needs no change (ticket 2815):

   ```text
   Book: none, <why>
   ```

   CI's `api` job fails such a change with neither. The better answer is
   usually the book's chapter the docs map names for the item.

   Under each such line, add a `migration` block: how a game moves over
   the break, written in YAML in a fenced block (the fields are in
   `tickets/README.md`, *Breaking changes*). The changelog's migration
   guide is made from it (`cargo xtask changelog`, ticket 2804).

   Add one `Behaviour:` line per change a game sees **without** a break
   (ADR-0108): a validation loosened or tightened, a default changed, an
   error's code or text, a label or output a game might test, a timing or
   an order.

   ```text
   Behaviour: <crate>[, <crate>…]: <what a game sees change>
   ```

   No gate finds these: you are the check. The changelog lists them in
   their own section, and the game's pin review reads them as it reads
   `Breaking:` lines.

   **"Nothing in the game must change" is earned, not assumed.** Before
   writing it (or "no break", or no `Behaviour:` line for a change to a
   rule), search the game's code and ADRs for the rule the change
   touches, and name the search in the notes:

   ```bash
   gh search code --repo Zafnok/visions-of-shuyi '<the rule, type or text>'
   ```

   A hit becomes a `Behaviour:` line (or a `Breaking:` line) saying what
   the game sees. Why: 2350 said "nothing in the game must change" while
   game ADR-0045 keeps `.dlg` text ASCII, and the game's pin PR wrote
   about 45–80 lines to keep its rule; #276 said "no break" and changed
   labels and panic text the game tested.
3. Set `status: done` and `completed: YYYY-MM-DD`.
4. `git mv tickets/open/<file>.md tickets/done/<file>.md`
5. **Settle what named this ticket.** Run `grep -rnw "<NNNN>"
   tickets/open docs --exclude-dir=adr`, and for each hit:
   - a clause that waits on it (the forms are in the doc comment of
     `find_clauses`, `crates/xtask/src/tickets.rs`): rewrite it as what
     now holds, and drop the other branch;
   - a promised follow-up: write it now (the `write-ticket` skill), and
     point the promise at its number;
   - a `blocked_by` entry: leave it, it is met.

   `cargo xtask ticket-lint` then prints no warning naming NNNN. In a
   parked ticket only the clause changes.
6. **Hand over what this ticket leaves.** Look at each piece this
   ticket leaves to another open ticket:
   - the *if not* branch of a clause, which you took;
   - a part that waits for it;
   - an **Out** line naming it;
   - a deviation in your notes.

   Open that ticket and check that it names this one and says it does
   the piece. If it doesn't, write it there now, as what holds:
   "**From NNNN (done):** …". After the move to `done/`, no later grep
   finds this ticket, so this is the last moment the handoff can be
   written (the `write-ticket` skill, *Check the dependency chain*).

## 6. Commit and PR

- Commit messages: `[NNNN] imperative summary`.
- Title: `[NNNN] <ticket title>`: CI reads the ticket from it. A PR
  that only writes or edits tickets is `Ticket NNNN: …` instead (the
  `write-ticket` skill). **No bare `@word`** in a title, body, commit
  message or comment. Use backticks.
- Body: summary bullets, the acceptance criteria (checked), follow-up
  tickets, the ticket's `Breaking:` lines (or "No break"), its
  `Behaviour:` lines, a `Nick input:` line, and the summary block of
  `cargo xtask gates` run on the commit you push (§4). The game's
  watcher reads both kinds of line from the body and from the ticket's
  notes. Not "all gates pass": the block says which ran and which CI
  alone runs.
- If the game must change too (a pin bump, a call site), write or link
  the game ticket. Say in both PRs which merges first: the engine's.
- **After the last merge of `main`** into the branch, re-read the PR
  body and the completion notes for every ADR number, ticket id and
  contract version (`contract::VERSION`) they name. A parallel PR may
  have taken one first: renumber with `cargo xtask next-id` (and the
  contract's to `main`'s plus one), fix the links to it, and edit the
  PR body to match. The squash message is the PR body (ticket 2052).
- **Before the last push, re-read the PR body against the final diff:**
  - the gates it says ran are the gates that ran, named one by one,
    never "all gates" over a shorter list;
  - every number (ADR, ticket, contract version) is the one the diff
    holds;
  - each "no break" or "nothing changes" claim still holds.

  Why: #302 ticked "All gates" and listed three, #297 said contract
  version 38 where the merge took it to 39, and #283 named the wrong
  ADR.
- **A picture in the PR body links at a commit**
  (`…/blob/<sha>/…?raw=true`), never at the branch. Why: the branch is
  deleted on merge, and the game's #516, #531, #534 and #555 have dead
  pictures.
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
