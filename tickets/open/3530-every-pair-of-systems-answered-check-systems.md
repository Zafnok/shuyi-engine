---
id: "3530"
title: Every pair of engine systems says how they work together, and a new system lands only with an answer for each one before it
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3530 — Every pair of engine systems says how they work together, and a new system lands only with an answer for each one before it

## Context

Nick, 2026-11-23: the drift he fears most is a system written before
another one exists and never wired to it later. For example, a renderer
written before sound would never learn the sound's side of a shared job,
because the ticket that adds sound doesn't say "go back to the
renderer". Each system works on its own, and their seam is missing
without a word.

It has happened already:

- #438 (2082) folded `Paint::Image` into a `_` arm of
  `crates/app/src/render.rs`, so a new paint would draw as an image.
  2097 now checks that narrow form: a variant swallowed by a match.
- Shake and flash (2340) and the reduced-motion preference (2377) came
  as separate tickets. Each had to name the other by hand.
- Sound in the background (3043), sound in the headless harness (2393)
  and captions for sound (2366, parked) are each a seam between sound
  and an older system. Each was found by someone remembering it.

What exists today doesn't cover this:

- `docs/modules.ron` (ADR-0009) records which module answers which
  concern, not how two systems must work together.
- `tools/layers.ron` (ADR-0032) records which crate may depend on which,
  not which systems owe each other a seam.
- The ticket lint and `work-ticket` §5–6 track tickets that name tickets.
  A seam no ticket names is invisible to them.
- 2097 catches one form of the gap (a `_` arm). A system that never
  calls the other, or never reacts to it, has no match to check.

So the seams become data, checked like the layers and the modules. The
rule that makes forgetting impossible: when system B is added, it gets
an answer for **every** system already listed, and the check fails
until it has them. An answer is one of three things:

- the seam is wired, with a test that proves it;
- it is planned, by a named open ticket;
- there is none, and the answer says why.

## Nick input

None.

## Scope

**In:**

- **`docs/systems.ron`**, the systems and their seams as data. Each
  system has:
  - an `id`, a `name`, and what it is made of: crates, modules or
    `modules.ron` answers. Each piece belongs to exactly one system.
  - `with`: one entry for **each system listed before it**. The order
    of the list is the order they were added, so a new system goes at
    the end and answers for all the others. Each entry is one of:
    - `wired(test: "<crate>::<path>::<test fn>", what: "…")`;
    - `planned(ticket: "NNNN", what: "…")`;
    - `none(why: "…")`;
    - `unreviewed`, allowed only under the ratchet below.
- **`cargo xtask check-systems`** fails when:
  1. a system lacks an entry for some earlier system, or has one for a
     later one;
  2. a `wired` test isn't found: no `fn <name>` under `#[test]` in that
     crate's `src/` or `tests/` (a plain text search, as `check-skills`
     does);
  3. a `planned` ticket is not in `tickets/open/`. A ticket in
     `tickets/done/` fails with "wire it or say why not": the PR that
     closes a ticket flips the seams it planned;
  4. a `none` has an empty `why`;
  5. there are more `unreviewed` entries than the count in
     `tools/systems-unreviewed.txt`. That count only goes down, so a new
     system can never add one;
  6. an engine crate in `tools/layers.ron`, or a `ready` answer in
     `docs/modules.ron`, belongs to no system.
- **Where it runs:** beside `check-modules`, in `cargo xtask gates`
  (`crates/xtask/src/gates.rs`) and in CI's `modules` job
  (`.github/workflows/ci.yml`, the job that runs `check-modules`), with
  the same `changes.yml` filter plus `tickets/**`. That way check 3
  runs on a ticket's move to `done/`.
- **The seed:** the systems that exist today, in the order they came.
  - The list starts from: turns, ticks, the grid and tactics kit, bots
    and search, the frame and its skins, the painter in `shuyi-app`,
    the raster, screens and widgets, text and fonts, localisation,
    input actions and the keymap, pointer and touch, controllers, the
    mixer and cues, sound output, assets and the pack, hot reload,
    saves and profiles, replays and state hashes, dialogue, cinematics,
    scripts, the debug view and console, the log, perf spans and heap
    tags, crash reports, the headless harness and scene dump, the tool
    contract with MCP and the live link, the player's settings (reduced
    motion, flashing), and the targets (desktop, web, Playdate) with
    export.
  - Merge or split those wherever the code says otherwise.
  - Fill every entry you can settle while seeding. A seam already
    proven gets `wired` with its test, a seam an open ticket plans gets
    `planned`, and a pair with nothing between them gets `none`.
  - Everything else is `unreviewed`, and its count goes in
    `tools/systems-unreviewed.txt`. 3531 works that count down to 0.
- **An ADR** (the `write-adr` skill): the seams as data, why each new
  system answers for every older one, and how this sits beside
  ADR-0009, ADR-0032 and 2097.
- **The rules, in the skills:**
  - `write-ticket`: a ticket that adds a system (a crate, a module or a
    `ready` answer) adds its row in the same PR. Its Scope lists the
    answer for each earlier system, and a `planned` answer is a ticket
    written in the same PR.
  - `work-ticket`: before the PR, run `cargo xtask check-systems`, and
    flip every `planned` entry that names this ticket to `wired` or
    `none`.
  - Then run `pwsh -File scripts/sync-agent-skills.ps1`.
- **The docs map:** a row in `docs/README.md` for `docs/systems.ron`,
  beside the modules row.

**Out (do not do):**

- Reviewing every seed entry that stays `unreviewed`, and writing the
  tickets for the gaps found: 3531.
- Checking that a `wired` test really exercises both systems. The check
  finds the test by name only, and review judges what it does.
- Seams inside one system, between its own modules.
- A game's own systems. This is about the engine; a game can copy the
  file's shape later.

**Estimated size:** about 1,400 lines (code 450, tests 350, the seeded
data 450, the ADR and skills 150).

**Split:** past 1,000 lines, leave the seed entries that need a search
`unreviewed`, and raise the ratchet count to match: 3531 settles them.

## Implementation steps

1. Read ADR-0009, ADR-0032, `crates/xtask/src/modules.rs` and
   `layers.rs`, and copy their RON loading and error style.
2. Write the types and the loader in `crates/xtask/src/systems.rs`, with
   checks 1–6 as separate functions, each with a test fixture that
   fails it.
3. Seed `docs/systems.ron` (see Scope). For each `wired` entry, find
   the test that proves the seam (grep `crates/*/tests` and `#[test]`).
   For each `planned` entry, find the ticket in `tickets/open`.
4. Write `tools/systems-unreviewed.txt` with the count left.
5. Wire the check into `gates.rs`, the `modules` job and the
   `changes.yml` filter.
6. Write the ADR, the docs map row and the two skill rules, then sync
   the loaders.
7. Run `cargo xtask check-systems`, `check-docs`, `ticket-lint` and the
   gates.

## Acceptance criteria

- [ ] `docs/systems.ron` lists every system, and each system answers
      every earlier one; `cargo xtask check-systems` passes on it.
- [ ] A test fixture fails each of checks 1–6, with a message naming
      the system, the entry and the fix.
- [ ] Adding a system at the end without entries fails, naming each
      missing earlier system.
- [ ] A `planned` entry whose ticket is moved to `tickets/done/` fails.
- [ ] Raising `tools/systems-unreviewed.txt`, or adding an `unreviewed`
      entry past it, fails.
- [ ] A crate added to `tools/layers.ron` and belonging to no system
      fails.
- [ ] The check runs in `cargo xtask gates` and CI's `modules` job,
      including on a PR that changes only `tickets/`.
- [ ] The ADR, the docs map row and the `write-ticket` and
      `work-ticket` rules are in, and `scripts/sync-agent-skills.ps1
      -Check` passes.
- [ ] The book's chapter is not updated: this is for engine developers
      only, so the completion notes have a `Book:` line saying so.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each check on a small fixture, passing and failing.
- Integration: `check-systems` on the real `docs/systems.ron`.

## Completion notes

*(Filled in by the session that completes the ticket.)*
