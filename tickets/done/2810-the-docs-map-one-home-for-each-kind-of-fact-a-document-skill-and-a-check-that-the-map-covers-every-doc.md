---
id: "2810"
title: "The docs map: one home for each kind of fact, by who reads it, a document skill that writes there, and a check that the map covers every Markdown file"
type: infra
milestone: MVP
model: sonnet-5
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-21
---

# 2810 — The docs map

## Context

Nick, 2026-10-21: work on documentation sooner, so that our own
AI-driven work gets easier. Sessions here should spend less context
searching, and the same responsibility shouldn't be written down in
two places. He asked for a skill along the lines of "always document
in these places for end users, and for engine developers".

What the repository has today:

- Knowledge lives in many places, each of them reasonable, with no
  index of which holds what:
  - the rules in `AGENTS.md`;
  - the reasons in 28 ADRs (`docs/adr/`, indexed in its README);
  - Nick's answers in `docs/decisions.md` (about 46 KB);
  - the plan in `docs/ROADMAP.md` (about 41 KB);
  - the contract in `docs/contract.md` and `crates/cli/schemas/`;
  - the formats in `docs/formats.md`;
  - backends in `docs/platform-backends.md`;
  - the carve in `docs/carve.md`;
  - problem codes in `crates/cli/errors/`;
  - the rules a game keeps in `templates/*/AGENTS.md`;
  - how to work in `.claude/skills/`;
  - what each crate holds in its crate docs (`missing_docs` is on).
- The `work-ticket` skill's §0 tells every session to read `AGENTS.md`,
  `tickets/README.md` and all of `docs/ROADMAP.md` before it starts,
  even for a ticket whose area needs none of the roadmap. Nothing tells
  it which of the other files its ticket's area needs, so it searches.
- No step tells a session where to write what its ticket changed. Each
  ticket says so on its own (2017 for `contract.md`, 2802 for the
  book's chapters), and each adds its own step to `work-ticket`.
- Some facts are written twice on purpose, because different readers
  read them in different places:
  - the layer, skin and key rules in `AGENTS.md` (rules 5, 8 and 9) and
    in the templates' `AGENTS.md` (*The layers*);
  - the contract's operations in `docs/contract.md` and in `README.md`'s
    `shuyi-cli` row (2017 tests that pair);
  - later, `input-actions` and the shipped `shuyi-input`, and
    `run-gates` and `shuyi-gates` (1134).

  Nothing lists these pairs, so one copy changes and the other drifts.

Coming homes, which this map gets a row for when they land:

- the book (2802);
- the engine's skill pack, `skills/` (1134);
- the guides, `docs/guides/` (2004);
- `llms.txt` and the site (2801, 2808).

This ticket comes first in the MVP's queue, so that every later ticket
writes into the map rather than inventing a place.
[ADR-0005](../../docs/adr/0005-ai-native-tooling-one-contract-three-surfaces-and-visual-scripting-as-a-view-over-text.md)
§4 already asks that the engine be written so agents can work on it.
This ticket is the docs half of that.

## Nick input

None.

## Scope

**In:**

- **`docs/README.md`, the map.** One table per reader. Each row is one
  kind of fact, with:
  - its one home (a link to the file or folder);
  - when to update it, as "a change that …".

  Start with these rows. Check each against the repository on the day,
  and add a row for any Markdown file the check (below) finds uncovered:

  - **People making games, and their agents:**
    - what a crate, a module or a public item does: rustdoc in the code.
      The crate doc (`//!` in `lib.rs`) names every `pub mod`. Update it
      when a change adds, renames or changes a public item;
    - the tool contract: [`contract.md`](../../docs/contract.md) for the
      prose and the example answer, and `crates/cli/schemas/` for the
      shapes, generated from the types (ADR-0022). Update it when a
      change adds or changes an operation or a field;
    - a problem code and its fix: `crates/cli/errors/SHnnnn.md`
      (`shuyi explain`, ADR-0028). Update it when a change adds or
      changes an error the contract gives;
    - the engine's file formats and the registry:
      [`formats.md`](../../docs/formats.md);
    - what a platform backend must do:
      [`platform-backends.md`](../../docs/platform-backends.md);
    - the rules a game on the engine keeps: `templates/*/AGENTS.md`;
    - what the engine is, and where to start: `README.md`;
    - how to contribute: `CONTRIBUTING.md`;
    - an example game: `examples/*/README.md`;
    - every third-party file: `THIRD_PARTY_ASSETS.md`.
  - **Engine developers (Claude, and anyone working on the engine):**
    - the rules: `AGENTS.md`;
    - why a technical choice was made: `docs/adr/` (the `write-adr`
      skill);
    - how to do a recurring task here: `.claude/skills/`;
    - what is to be done, and its order: `tickets/` (the `write-ticket`
      skill) and the order in [`ROADMAP.md`](../../docs/ROADMAP.md);
    - what came from the game: [`carve.md`](../../docs/carve.md);
    - the Sonar policy: `tools/sonar-triage/README.md`;
    - anything else under `crates/` or `tools/` that is Markdown
      (`crates/cli/web/README.md` today).
  - **Nick:**
    - his answers: [`decisions.md`](../../docs/decisions.md) (the
      `ask-nick` skill);
    - the plan in product terms: [`ROADMAP.md`](../../docs/ROADMAP.md).
      It is in two tables, and that's fine. Nick reads it for the
      plan, and sessions read it only to choose a ticket.

- **What each task reads.** A short table in `docs/README.md` from a
  ticket's area to the docs a session reads first. For example, a
  format ticket reads `formats.md` and ADR-0007 and ADR-0019, and a
  contract ticket reads `contract.md` and ADR-0022 and ADR-0028. Keep
  one row per ticket block of `tickets/README.md` (`20xx` to `31xx`).

- **The rules for writing docs**, in `docs/README.md` under the tables:
  1. **One home.** A fact is written in its home. Anywhere else links
     to it, or says it in one line with the link, and never with
     detail that could drift.
  2. **Generated beats written.** A shape the code can produce (a
     schema, a list of operations) is generated or tested against the
     code. A written copy is never the source.
  3. **Pairs.** A fact that two readers read in two places is a pair,
     listed in a *Pairs* table with both homes. A change to one changes
     the other in the same PR. Start with the pairs in *Context*. 1134
     adds its two skill pairs when it lands. A pair the code can check
     says which test checks it (2017's `docs.rs`, once it exists).
  4. **The reader decides the place.** Nothing technical goes in
     Nick's files. Nothing of this repository's process (tickets, Nick,
     the game's pin) goes in what people making games read: rustdoc,
     the templates, `contract.md`, the error pages and, later, the book
     and the skill pack.
  5. **A new home gets a row.** A ticket that makes a new home for docs
     (a new file under `docs/`, a new folder of pages) adds its row in
     the same PR.

- **The `document` skill**, `.claude/skills/document/SKILL.md`:
  - Its description says when to use it: any change that adds or
    changes a public item, a contract operation or field, a problem
    code, a format, a rule, a skill or a template, and any time a
    session is about to write a new Markdown file. When not: tickets
    (`write-ticket`), ADRs (`write-adr`), Nick's decisions (`ask-nick`).
  - Its steps: list what the change touches; find each row in
    `docs/README.md`; update each home, and the other half of any pair;
    put nothing in a second place; add a row for a new home; run
    `cargo xtask check-docs`.
  - It holds no copy of the map. It links `docs/README.md` (rule 1
    applies to the skill too).

- **The `work-ticket` skill:**
  - §0: read `AGENTS.md` and `docs/README.md`, then the ticket, then
    the docs that *What each task reads* names for its block.
    `tickets/README.md` and `docs/ROADMAP.md` move to §1, which is
    where they are used (choosing a ticket). A session handed a ticket
    by number reads neither.
  - §5: a new first step, before the completion notes: "**Write the
    docs.** Run the `document` skill over the change, or say in the
    completion notes why the change needs none."

- **`AGENTS.md`:** one line under the opening paragraph: where things
  are written is in `docs/README.md` (the `document` skill).

- **`cargo xtask check-docs`** (`crates/xtask/src/docs.rs`):
  - every Markdown file that `links.rs`'s walk finds (the same
    `SKIPPED_DIRS` and `SKIPPED_PATHS`) is covered by a relative link
    in `docs/README.md`: a link to the file itself, or to a folder
    that holds it. A pattern written as text, such as
    `templates/*/AGENTS.md`, is not a link. Write the row as a link to
    the folder;
  - `docs/README.md` itself, `CLAUDE.md` files (which only point to
    `AGENTS.md`) and `tickets/` are not checked; `tickets/` has its
    own row;
  - an error lists each uncovered file, with "add a row to
    docs/README.md (the document skill)";
  - links that don't resolve are left to `check-links`;
  - wired into `main.rs` (its module comment, `USAGE` and the match),
    into CI's `tickets` job after `check-links`, and into the
    `run-gates` skill's *The repository's own checks*.

- **The tickets that make new homes** were changed when this ticket
  was written, so nothing changes in them here:
  - 2802's book chapters are a row of the map, with no `work-ticket`
    step of its own, since §5's *Write the docs* covers them;
  - 1134, 2004 and 2808 each add their row, and 1134 adds its pairs.

  Check that each still says so. If one was rewritten since, put its
  line back.

**Out (do not do):**

- writing any of the coming docs (the book, the guides, the skill pack,
  `llms.txt`): 2802, 2004, 1134 and 2808;
- moving or splitting `docs/decisions.md` or `docs/ROADMAP.md`. Their
  size is a cost the reading table avoids; splitting them is Nick's
  call, since he reads them;
- editing accepted ADRs: the map links them, and an ADR is superseded,
  never edited;
- a check that a crate doc names every module (2017's Out line keeps
  that);
- a check that pairs agree. Each pair the code can test gets its test
  from the ticket that makes it (2017 for `contract.md` and the
  README);
- the game's docs: the game's repository keeps its own map, if it wants
  one;
- OpenAPI. The engine has no HTTP API. MCP tool definitions and other
  agents' tool calls take the JSON Schemas the contract already has.

## Implementation steps

1. Read `AGENTS.md`, `README.md`, every file under `docs/` but the ADRs
   and `decisions.md` (skim those two for what they hold), the eight
   skills, `crates/xtask/src/links.rs` and `main.rs`, and CI's `tickets`
   job in `.github/workflows/ci.yml`.
2. Write `crates/xtask/src/docs.rs` and its unit tests. Reuse
   `links.rs`'s walk and link parsing: make `markdown_files` and
   `relative_links` `pub(crate)` rather than copying them.
3. Run it. The list it prints is the rows the map needs.
4. Write `docs/README.md`: the three tables, *What each task reads*,
   the rules and *Pairs*.
5. Write the `document` skill. Change `work-ticket` (§0, §1 and §5),
   `run-gates` and `AGENTS.md`.
6. Add `check-docs` to CI's `tickets` job.
7. Run the gates.

## Acceptance criteria

- [x] `docs/README.md` has the three tables, *What each task reads* with
      a row per ticket block, the five rules and *Pairs*.
- [x] `cargo xtask check-docs` passes on the repository, and fails on a
      new uncovered `docs/x.md` (the PR shows it failing in a local
      run).
- [x] CI's `tickets` job runs `check-docs`.
- [x] `.claude/skills/document/SKILL.md` exists, with a description that
      says when and when not, and no copy of the map.
- [x] `work-ticket` §0 reads the map and the task's docs, §1 reads
      `tickets/README.md` and `ROADMAP.md`, and §5 starts with *Write
      the docs*.
- [x] `AGENTS.md` and `run-gates` say what the Scope says.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/docs.rs`), over a temporary tree:
  - a file linked directly;
  - a file covered by a link to its folder;
  - an uncovered file, named in the error;
  - a link to a sibling folder whose name starts the same
    (`docs/ad` doesn't cover `docs/adr/x.md`);
  - `CLAUDE.md`, `tickets/` and a skipped directory not checked;
  - a missing `docs/README.md`, which is an error.
- Integration (`crates/xtask/tests/cli.rs`): `check-docs` on the
  repository passes.

## Completion notes

- `docs/README.md` is the map: a table each for people making games,
  engine developers and Nick; *What each task reads*, one row per block
  `20xx`–`31xx`; the five rules; and *Pairs* (the layer rules, the
  contract's operations, and one more found on the way: the `run-gates`
  skill and `ci.yml`, which the skill says it mirrors).
- `cargo xtask check-docs` (`crates/xtask/src/docs.rs`) reuses
  `links.rs`'s walk, link parser, `display_path` and `percent_decode`
  (now `pub(crate)`). A link covers a file when its path, worked out
  from the text, is a whole-component prefix of the file's path. Run
  locally, it passes on the repository, and fails on a new `docs/x.md`
  with `docs/x.md: not in the docs map; add a row to docs/README.md (the
  document skill)`.
- Rows the first run of the check asked for, beyond the ticket's list:
  `templates/*/README.md` (covered by the `templates/` row, which now
  says it holds a new game's README too) and `docs/adr/README.md`
  (the ADR row links it).
- Rustdoc's row has no link: a link to `crates/` would cover every
  Markdown file under it, so `crates/cli/web/README.md` and the error
  pages keep rows of their own.
- The `document` skill links the map and holds no copy. `work-ticket`
  §0 reads `AGENTS.md`, the map, the ticket and its block's docs; §1
  reads `tickets/README.md` and `ROADMAP.md` only when choosing a
  ticket; §5 starts with *Write the docs*. CI's `tickets` job runs
  `check-docs` after `check-links`, and `run-gates` lists it.
- 2802, 1134, 2004 and 2808 still say they add their rows (and 1134 its
  pairs); nothing changed in them.
- Docs: this ticket is the docs change; the map's own row covers it.
