---
id: "2017"
title: "The backlog and the docs kept true as tickets land: a ticket-lint warning on clauses about done tickets, a work-ticket step that settles them, and contract.md tested against the code"
type: infra
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-10-21
---

# 2017 — The backlog and the docs kept true as tickets land

## Context

Found by the 2026-10-20 comparison with Bevy 0.20, Godot 4.7, Unity 6
and Unreal 5.8. Bevy's and Godot's CI fail when docs fall behind the
code:

- Bevy's `check-missing-examples-in-docs` and
  `check-missing-features-in-docs` jobs fail on stale generated pages
  (https://github.com/bevyengine/bevy/blob/main/.github/workflows/ci.yml);
- Godot's CI runs `--doctool` and fails when the class reference
  differs from the code
  (https://github.com/godotengine/godot/blob/master/.github/workflows/linux_builds.yml).

Neither checks its issue tracker for stale text. That half is ours:
our backlog is tickets written for agents to finish "without asking
questions" (the `write-ticket` skill).

What Shuyi has today:

- `cargo xtask ticket-lint` (2002, `crates/xtask/src/tickets.rs`)
  checks the frontmatter, unique ids, that `blocked_by` ids exist
  (`game#` ones too), the `Parked:` rules and `--pr-branch`. Of a
  ticket's body it reads only the Parked note (`check_parked`,
  line 275). `Report` (line 94) already has warnings, which don't fail
  the lint.
- The `write-ticket` skill, *Check the dependency chain*: each open
  ticket a ticket names is a blocker, an either-order clause or an Out
  line, and "Look the other way too", when a ticket is **written**. The
  `work-ticket` skill's §5 (*Archive the ticket*) has no matching step
  when a ticket is **closed**.
- `crates/cli/tests/it/schemas.rs` (1129): the checked-in schemas are
  what the types generate, and `versions.json` lists
  `contract::VERSION`. Nothing reads `docs/contract.md`.
- 2802 compiles the book's snippets in a crate CI builds, and 2810's
  docs map sends a ticket's change to its chapter. 2004's guides are
  doc tests.
  Neither covers ticket text, `docs/contract.md` or the crates' own
  docs.

What has gone stale (checked on 2026-10-20 and 2026-10-21):

- On 2026-10-21 the four forms below, run by hand, found about 760
  clauses in 185 of the 309 open tickets. 22 of them, in 18 files,
  wait on a ticket that is already done (1129, 2204, 3001, 3002 and
  3003). Examples: 2321 line 47, 2322 line 76 and 2325 line 67 on
  3003, and the visual novel and mystery tickets (2722–2732) on 3002.
- 3102's Out list (lines 117 to 119) promised a parked follow-up ticket
  for an optimized mesh format once 2204 was done. 2204 is done, and
  no such ticket exists.
- `docs/contract.md` line 45 shows `"contract": 3`, and
  `crates/cli/src/contract.rs` line 52 has `VERSION = 4`.
- `README.md` line 130 lists the contract's operations without `bake`.
- `crates/ui/src/lib.rs` lines 18 to 20 list the widgets as "a menu and
  word wrap" and only two screens. `widgets.rs` and `screens.rs` have
  more (`ListCursor`, `Ask`, `name_entry`; `options`, `profiles`), and
  the modules `debug`, `dialogue`, `float` and `tips` aren't named.
  `shuyi-core`'s crate docs don't name `save`, and `shuyi-assets`' don't
  name `tileset` or `text` (word wrap).
- `crates/platform/src/lib.rs` line 1, `app.rs` line 2 and
  `platform.rs` line 1 cite ADR-0023 (the pack) for the platform
  boundary, which is ADR-0024.
- 2003 and 2005 list the default crates without `shuyi-platform`, which
  `shuyi-app` and `shuyi-cli` depend on. 3104 line 76 still points at
  "1124's reload hook"; hot reload is 2220's now.

Why it matters:

- **The game first.** The game's sessions work engine tickets through
  its `engine-change` skill. A stale conditional sends a worker down a
  branch that no longer exists, and a promised follow-up silently drops
  out of the plan.
- **Agents read the docs.** The MCP server (1133) and the skill pack
  (1134) teach from `docs/contract.md`. The end goal (2026-10-19,
  `docs/decisions.md`) is the most AI-friendly 2D engine; a wrong
  example answer teaches every agent the wrong contract.
- **It grows.** Each ticket that closes leaves clauses behind, so this
  is cheap now and dearer later.

## Nick input

None.

## Scope

**In:**

- **A ticket-lint warning** in `crates/xtask/src/tickets.rs`, never an
  error:
  - it reads the body of each ticket in `tickets/open/`, outside fenced
    code blocks. Done tickets are history and are never read;
  - it finds clauses in exactly these forms, in any case, across line
    breaks and Markdown emphasis (a `**Either order` at a line's end and
    `with NNNN:**` on the next):
    1. `either order with <ids>` (and `either order works with <ids>`);
    2. `if <ids> is done` (and `are done`);
    3. `when <ids> is done`;
    4. `<id>, if done` (as in "the book (NNNN, if done)");

    where `<ids>` is one four-digit id, or several joined by `,`, `and`
    or `or`;
  - for each id in a clause whose ticket is in `tickets/done/`, one
    warning, such as `tickets/open/NNNN-….md:47: "if MMMM is done":
    MMMM is done, so settle the clause (work-ticket §5)`. One warning
    per file, line and id. The line is the clause's first line in the
    file, frontmatter counted;
  - no new dependency: a small matcher over words, not the `regex`
    crate.
- **The warnings on CI.** When `GITHUB_ACTIONS` is `true`,
  `report` in `crates/xtask/src/main.rs` (line 71) also prints each
  warning on stdout as a workflow command, `::warning::<text>`, so it
  shows on the PR. The text has `%`, CR and LF written as `%25`, `%0D`
  and `%0A`, as GitHub's workflow commands require. The exit code
  doesn't change, and stderr prints as today. This covers every
  `report` caller, so the lint's existing warning about skipped
  `game#` ids shows on the PR too.
- **The `work-ticket` skill**, §5: a new step after the `git mv` that
  settles what named the ticket (its text is in *Implementation
  steps*).
- **The `write-ticket` skill**, *Check the dependency chain*:
  - the either-order bullet gives the words the lint reads:
    "**Either order with NNNN:** if NNNN is done, …; if not, …";
  - a new bullet: a follow-up you can name now, write now, instead of
    promising one for later. It is parked if its work is parked, and
    its `blocked_by` holds what it waits for;
  - its last line drops "(after 2002 lands)", since 2002 is done.
- **The `run-gates` skill**, *The repository's own checks*: one line
  saying the lint warns on clauses about done tickets, and that the PR
  closing a ticket settles the clauses that name it.
- **`docs/contract.md` tested against the code**, in
  `crates/cli/tests/it/docs.rs` (with `mod docs;` in
  `crates/cli/tests/it/main.rs`):
  - the JSON block under *The answer* parses as `Answer<SceneOutput>`
    (`shuyi_cli::contract::Answer`, line 116, and
    `shuyi_cli::contract::run::SceneOutput`; both have
    `deny_unknown_fields`). It serializes back to the same JSON value,
    so no field is left out. Its `op` is `scene`, and its `contract` is
    `contract::VERSION`. Each failure says what to change in the doc;
  - every name in `Op::ALL` has a row in the table under *The
    operations*, and every row is an `Op` or `schema`. A row's name is
    the first word inside its first backticks (the `check --target`
    row is `check`);
  - `README.md`'s `shuyi-cli` row names every op in `Op::ALL` in
    backticks (`` `bake` ``, so `check-keys` doesn't count as `check`).
- **The one-time sweep**, in this PR:
  1. Run the new lint and settle every clause it warns about. Keep what
     holds now and drop the other branch, as a plain statement ("MMMM
     moved the run loop into `App::frame`, so the pacer goes in the
     backend"). In a parked ticket only the clause changes, never what
     the ticket does. On 2026-10-21 the four forms, run by hand, found
     the 22 counted above; some may be settled before this ticket
     starts, and tickets written since may add more.
  2. Write the follow-up 3102 promised, with the `write-ticket` skill:
     - "Parked: 3D meshes in the pack", the next free `31xx` number
       (3111 on 2026-10-20), milestone `Parked: 3D`,
       `blocked_by: ["3102"]`, and 3102's Parked note;
     - its scope: 3102's `Mesh`, `Material`, `Skeleton` and `Clip3`
       are `Bakeable` (`crates/assets/src/pack/bake.rs`: serde's
       `Serialize` and `Deserialize`, `PartialEq` and `Debug`), so
       2204's `Bake` writes them as postcard sections; `Bake::check`
       proves each equal to the glTF it came from (ADR-0007 §4,
       ADR-0011 §4, ADR-0023); a snapshot pins each kind's binary form,
       as the pack's tests pin the engine's kinds; the load time from
       glTF and from the pack is measured;
     - then 3102's Out line names it.
  3. `docs/contract.md`: the example's `contract` set to `VERSION`.
  4. `README.md` line 130: `bake` in the list of operations.
  5. Crate docs:
     - `crates/ui/src/lib.rs` names every `pub mod`. For `widgets` and
       `screens`, one line each that links the module, whose own docs
       list its widgets and screens, so a new widget changes one place;
     - `shuyi-core`'s and `shuyi-assets`' crate docs name every
       `pub mod` too: today that adds `save` to the first, and
       `tileset` and `text` to the second;
     - the three `shuyi-platform` files cite ADR-0024.
  6. 3104 line 76: hot reload is 2220's (the `reloaded` hook it adds to
     `GameHooks`).
  7. The batch that wrote this ticket asked for these to be fixed at
     once: `shuyi-platform` in 2003's re-export list, in its
     `check-modules` list of default crates and in 2005's publish
     order, and 2321's *Related* clause and Out line on 3003. Check
     them, and fix what is still wrong.
  8. `.github/workflows/changes.yml` line 5 says "See ADR-0014". This
     repository's ADR-0014 is MIDI music; the comment means game
     ADR-0014. Write "game ADR-0014".

**Out (do not do):**

- making any of these warnings an error: the forms are prose, and a
  false match must never fail the gate;
- finding promised follow-ups by free text: the `work-ticket` step and
  the `write-ticket` rule cover them;
- a check that crate docs name every module: a one-time fix here, and a
  ticket if they drift again;
- the book's and the guides' code (2802, 2004);
- a release-notes or migration-guide bot (2804, 2010, 2523);
- changing what any parked ticket does;
- the game's tickets that name this repository's (`engine#NNNN`): the
  game's `engine-change` skill keeps those;
- accepted ADRs. ADR-0009's "five crates" is context that ADR-0024's
  `shuyi-platform` came after; an ADR is superseded, never edited.

**Either order:**

- **Either order with 2810**, which adds a first step to `work-ticket`
  §5 (*Write the docs*, the `document` skill): both steps go in, and
  whichever lands second keeps the other's. 2802 adds no step of its
  own: its chapters are a row of 2810's map.
- **Either order with 2509 and 3040**, which change the answer (2509's
  `code` and `help` on `Answer`, 3040's `log` on `SceneOutput`) and
  raise `VERSION`. Each updates `docs/contract.md`'s example in its own
  PR. If either is done, the example already shows its fields, and this
  ticket's test holds them. If not, this ticket sets `"contract"` to
  today's `VERSION`, and they keep the test passing: the example stays
  a `scene` answer with every field the code writes.

## Implementation steps

1. Read `crates/xtask/src/tickets.rs`, `main.rs` and `tests/cli.rs`;
   the `work-ticket`, `write-ticket` and `run-gates` skills; ADR-0022
   and `docs/contract.md`.
2. In `tickets.rs`:
   - `struct Clause { line: usize, text: String, ids: Vec<String> }`,
     and `Ticket` (line 70) gains `clauses: Vec<Clause>`, filled by
     `parse_ticket` (line 166) for `Folder::Open` only;
   - `fn find_clauses(body: &str, first_line: usize) -> Vec<Clause>`:
     - the body's words (split on whitespace) outside fenced code
       blocks, each with its line. A line whose trimmed start is
       `` ``` `` opens or closes a block;
     - each word: whether a comma is among its trailing punctuation
       remembered, then its edges stripped of `*`, `_`, backticks,
       brackets, `,`, `:`, `;` and `.`, then lower-cased. So
       `(2802,` is `2802` with a comma, and `2802**,` too;
     - the four forms matched over the words, so a clause may run
       across lines;
     - `first_line` is the body's first line in the file.
       `extract_frontmatter` (line 353) returns the body as a slice of
       the content, so it is one more than the newlines in the content
       before the body (after `parse_ticket` turns CRLF into LF);
   - `fn check_clauses(tickets: &[Ticket]) -> Vec<String>`: the done
     ids are the ids of the `Folder::Done` tickets; one warning per
     file, line and id;
   - `run` (line 102) adds them to `report.warnings`.
3. In `main.rs`: `fn annotation(warning: &str) -> String` (the
   escaping above), used by `report` when `GITHUB_ACTIONS` is `true`.
   `USAGE`'s `ticket-lint` lines add that it warns on clauses about
   done tickets.
4. Write the tests below, enough to kill the mutants: CI's mutation
   testing runs over the PR's diff, and `xtask` isn't excluded
   (`.cargo/mutants.toml`).
5. Change the three skills. The `work-ticket` step goes in §5 after
   step 3:

   > 4. **Settle what named this ticket.** Run `grep -rnw "<NNNN>"
   >    tickets/open`, and for each hit:
   >    - a clause that waits on it (*either order with NNNN*, *if
   >      NNNN is done*, *NNNN, if done*): rewrite it as what now
   >      holds, and drop the other branch;
   >    - a promised follow-up: write it now (the `write-ticket` skill),
   >      and point the promise at its number;
   >    - a `blocked_by` entry: leave it, it is met.
   >
   >    `cargo xtask ticket-lint` then prints no warning naming NNNN. In
   >    a parked ticket only the clause changes.

6. Write `crates/cli/tests/it/docs.rs`. It reads the files from
   `env!("CARGO_MANIFEST_DIR")` and `../..`, as `schemas.rs` reads
   `schemas/`:
   - the JSON block is the first fenced `json` block after the line
     `## The answer`;
   - the table rows are the lines starting with `` | ` `` (a pipe, a
     space and a backtick) between `## The operations` and the next
     `## `;
   - the README row is the line starting with `` | `shuyi-cli` ``;
   - the module doc of `crates/cli/tests/it/main.rs` adds
     `docs/contract.md` and the README to what the tests check.
7. Run the lint and do the sweep, one commit per item, so the PR reads
   easily.
8. Run the gates.

Size: if the diff runs well past 600 changed lines, move the sweep's
item 1 (settling the clauses the lint warns about) to a follow-up
ticket, with the acceptance line on no warnings, and keep the lint, the
docs test, the skills and items 2 to 8 here.

## Acceptance criteria

- [x] `cargo xtask ticket-lint` warns, and still exits 0, on an open
      ticket with a clause in each of the four forms about a done
      ticket, naming the file, the line and the id.
- [x] It doesn't warn on a clause in a fenced code block, a clause in a
      done ticket, a clause about an open ticket, or a done id named
      outside the four forms.
- [x] With `GITHUB_ACTIONS=true`, each warning is also printed on
      stdout as a `::warning::` line with `%`, CR and LF escaped;
      without it, stdout has none.
- [x] After the sweep, `cargo xtask ticket-lint` on the repository
      prints no warning about a clause.
- [x] `crates/cli/tests/it/docs.rs` passes. The PR description shows it
      failing in local runs with the example's `contract` set to 3, with
      a field added to the example, with a field left out, and with the
      `bake` row removed.
- [x] The parked `31xx` ticket exists, 3102's Out line names it, and
      the ticket lint passes on both.
- [x] Each item of the sweep is done, or the PR says it was already
      done.
- [x] The `work-ticket`, `write-ticket` and `run-gates` skills say what
      the Scope says.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`crates/xtask/src/tickets.rs`):
  - each of the four forms, in upper and lower case;
  - ids joined by `,`, `and` and `or`, with warnings for the done ones
    among them only;
  - a clause split over two lines, and one inside `**…**`;
  - the line number, with the frontmatter counted;
  - a fenced code block skipped;
  - one warning per file, line and id, when one line has two forms that
    name the same id;
  - no warning for an open id, a plain mention of a done id, or a done
    ticket's own body;
  - `run` over a temporary tree (as
    `run_reports_real_errors_from_a_directory_tree` does): a stale
    clause gives a warning and no error.
- Unit (`crates/xtask/src/main.rs`): `annotation`, with plain text and
  with `%`, CR and LF in it.
- Integration (`crates/xtask/tests/cli.rs`): `ticket-lint` on the
  repository with `GITHUB_ACTIONS=true` and `--game-repo` at
  `no_game()` prints `::warning::` lines on stdout (the skipped `game#`
  ids give one), and with the variable removed (`env_remove`, since CI
  sets it) prints none. The `xtask` helper reads only stderr, so this
  test captures stdout itself.
- Integration (`crates/cli/tests/it/docs.rs`): the example answer, the
  operations table and the README row.
- The existing `real_repo_tickets_pass` (errors only) and
  `crates/xtask/tests/cli.rs` still pass.

## Completion notes

Done on 2026-10-21, on a branch stacked on 2128's PR
(`t2128-turn-planner`) with `main` merged in.

- **The lint.** `find_clauses` in `crates/xtask/src/tickets.rs` reads
  the four forms over the words of an open ticket's body, outside fenced
  blocks; `check_clauses` gives one warning per file, line and done id.
  `report` in `main.rs` prints each warning on stdout as `::warning::`
  when `GITHUB_ACTIONS` is `true` (`annotation`).
- **The docs test.** `crates/cli/tests/it/docs.rs`: the example answer
  parses as `Answer<SceneOutput>`, serializes back to the same JSON, is
  a `scene` answer and shows `VERSION`; every `Op` has its row and every
  row is an `Op` or `schema`; README's `shuyi-cli` row names every op.
- **The skills.** `work-ticket` §5 step 5 (after 2810's *Write the docs*
  and the `git mv`), `write-ticket`'s dependency chain, and `run-gates`.
- **The sweep:**
  1. the lint found 36 clauses in 30 tickets (2509, 2125, 3001, 3002,
     3003, 3006, 1129 and 2204 had closed); each now states what holds.
     Only the clause changed in parked tickets;
  2. 3111, "Parked: 3D meshes in the pack", written, with a row in
     `docs/ROADMAP.md`'s parked table; 3102's Out line names it;
  3. already done: `docs/contract.md` showed `"contract": 7`, today's
     `VERSION`, since 2509;
  4. the README row lacked `bake`, and `playtest` and `explain` too: all
     three added (the new test asked for them);
  5. the crate docs of `shuyi-ui`, `shuyi-core` and `shuyi-assets` name
     every `pub mod`; the three `shuyi-platform` files cite ADR-0024;
  6. 3104 names 2220's hook. 2220 puts it on `Module` (`Module::reloaded`),
     not on `GameHooks` as this ticket's Scope said, so 3104 says that;
  7. already done: 2003 and 2005 list `shuyi-platform`, and 2321's
     Related clause on 3003 was settled;
  8. `changes.yml` says "game ADR-0014".
- **Deviations:** none beyond item 6's hook name.
- **Docs:** the skills are this change's docs (the `document` skill's map
  sends rules of work to the skills); no public item of a crate changed.
- **Follow-up tickets:** 3111 (parked).
