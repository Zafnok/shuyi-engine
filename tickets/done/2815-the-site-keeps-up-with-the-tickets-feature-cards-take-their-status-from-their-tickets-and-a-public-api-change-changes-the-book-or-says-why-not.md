---
id: "2815"
title: "The site keeps up with the tickets: the home page's feature cards take their status from the tickets they name, and a public API change changes the book or has a Book line"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2808"]
nick_input: none
completed: 2026-10-28
---

# 2815 — The site keeps up with the tickets

## Context

Nick, 2026-10-28 ([`docs/decisions.md`](../../docs/decisions.md),
*pictures in the book, a playground, and a site that keeps up with the
tickets*), asked whether the site's pages "auto update as tickets
complete". `pages.yml` builds the whole site again on every push to
`main`. Some of what it shows is made from the repository at each
build, and some is written by hand:

| Part | Made at each build from | Follows the tickets |
| ---- | ----------------------- | ------------------- |
| The Changelog and the release posts (2804) | `tickets/done/` and the merges | yes |
| The API docs at `/api/` (2802) | `cargo doc` | yes |
| The problem codes' pages (2802) | `crates/cli/errors/` | yes |
| The Examples (2801) | each example's web build; one with no card fails the build | yes |
| `llms.txt`, `llms-full.txt` and the twins (2808) | the Learn pages | as the book does |
| The book's code (2802) | `site/snippets/`, built and tested in CI | yes: a change that breaks it fails |
| The book's words | written by hand | only when a PR changes them |
| The home page's feature cards | `site/data/features.toml`, written by hand | no |

Two parts can fall behind:

- **The feature cards.** Each has a `status` ("Ready", "In progress",
  "Planned") and a `link_text` that names its tickets ("Tickets 2904,
  2107, 2108"). When the last of them is done, the card still says
  "Planned" until someone edits the file. Nothing checks it.
- **The book's words.** The ticket template asks for "The book's
  chapter is updated (2802), or the completion notes say why not", and
  the docs map (`docs/README.md`) says which change updates which
  chapter. Nothing checks that either: the docs map's own row says
  "its words, nothing yet". `llms.txt` and its twins are made from the
  book (2808), so they are only as current as it is.

CI's `api` job (`cargo xtask api`, `crates/xtask/src/api.rs`,
ADR-0038) already reads the public API's change on each pull request
and fails a break with no `Breaking:` line in the branch's done
ticket. A public change is when the book may need its words changed,
so the same job can ask for them.

## Nick input

None.

## Scope

**In:**
- **Feature cards from their tickets:**
  - a card in `site/data/features.toml` may hold `tickets = ["2904",
    "2107", "2108"]`. Then its status is made: **Ready** when every one
    is in `tickets/done/`, **In progress** when some are, **Planned**
    when none is; and its `link_text`, when it has none, is "Ticket
    2904" or "Tickets 2904, 2107 and 2108". A card without `tickets`
    keeps its own `status`, as today.
  - `cargo xtask site-data --site site` (`crates/xtask/src/site_data.rs`)
    writes `site/data/tickets.toml`, never committed (`.gitignore`):
    each card's made status and link text, by card title. The home
    page's template (`site/templates/index.html`) reads it with
    `load_data` and takes the made status over the file's. It fails,
    naming the card, on a card with both `tickets` and `status`, and
    on a ticket id in neither `tickets/open/` nor `tickets/done/`.
  - `pages.yml` runs it before `zola build`, and so does the CI job
    that builds the site. Locally, `site/README.md` says to run it
    before `zola serve`; without the file, the template shows each
    card's own `status`, or "Planned".
  - Every card in `features.toml` whose `link_text` names tickets moves
    to `tickets`, and loses its `status`. The file's header comment says
    how the status is made.
- **A Book line for a public change** (`cargo xtask api`):
  - when the public API changed (a verdict other than "none") and the
    pull request changes no file under `site/content/learn/` or
    `site/snippets/`, the branch's done ticket's `## Completion notes`
    must hold a `Book:` line saying why the book needs no change
    (`Book: none, a helper the tests call made public`). Without one,
    the job fails, naming the crates whose API changed and both ways to
    pass: change the chapter the docs map names, or write the line.
  - a branch with no ticket of this repository (Dependabot, a `sonar`
    batch, a game's small change) is skipped, as the `Breaking:` check
    skips it today, unless it breaks the API, which already fails.
  - the job's summary says whether the book changed, or quotes the
    `Book:` line.
- **The docs:**
  - `tickets/TEMPLATE.md`: the criterion becomes "The book's chapter is
    updated (2802), or the completion notes have a `Book:` line saying
    why not".
  - the `work-ticket` skill's completion notes step names the `Book:`
    line beside the `Breaking:` line;
  - `docs/README.md`'s row for the book: what checks its words now;
  - `site/README.md`: the made statuses, and the table above (which
    parts follow the tickets), in short.

**Out (do not do):**
- checking which chapter a change belongs in. The check only asks that
  the book changed or the notes say why not;
- making the feature cards' text, or the roadmap's table they come
  from, from the tickets;
- statuses anywhere else on the site;
- any change to `cargo xtask llms` (2808): it makes `llms.txt` from the
  book at each deploy, so a current book gives a current `llms.txt`.

## Implementation steps

1. Read `site/data/features.toml`, `site/templates/index.html`,
   `crates/xtask/src/api.rs` (how it finds the branch's ticket and its
   `Breaking:` lines, and what it does with no ticket),
   `crates/xtask/src/tickets.rs` (how tickets are read),
   `crates/xtask/src/changelog.rs` (another writer of the site's
   generated files), `pages.yml` and the CI job that builds the site.
2. Write `site_data.rs` and its command, with tests on a fixture
   repository.
3. Move the cards to `tickets`, change the template, run the command
   in both workflows, and add `site/data/tickets.toml` to `.gitignore`.
4. Add the `Book:` check to `cargo xtask api`, with tests.
5. Change the template, the skill and the two READMEs.
6. Run the gates. After the merge, look at the live home page.

## Acceptance criteria

- [x] On a fixture, a card whose tickets are all done is Ready, some
      done is In progress, none done is Planned, and its link text is
      made when it has none (unit tests).
- [x] A card with both `tickets` and `status`, or naming an unknown
      ticket, fails the command, naming the card.
- [x] Every card in `features.toml` that named tickets in its link text
      has `tickets` and no `status`, and the built home page shows each
      card's status as its tickets say.
- [x] `site/data/tickets.toml` isn't committed, and the CI site job
      writes it before building.
- [x] `cargo xtask api` fails a ticket's branch that changes the public
      API, touches no file of the book and has no `Book:` line; passes
      it with the line; passes it with a change under
      `site/content/learn/` or `site/snippets/`; and skips a branch with
      no ticket (unit tests over a fixture, as the `Breaking:` tests do).
- [x] This PR's own completion notes have a `Book:` line if the check
      asks for one.
- [x] The book's chapter is updated (2802), or the completion notes say
      why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`site_data.rs`): the three statuses, the made link text for one
  and several tickets, both errors.
- Unit (`api.rs`): the four cases of the `Book:` check, beside the
  `Breaking:` tests.
- Site: in the site's CI job, the home page's HTML holds a card's made
  status (grep for a card whose tickets are all done).

## Completion notes

What was done:

- `cargo xtask site-data --site <dir>` (`crates/xtask/src/site_data.rs`)
  reads the cards of `<dir>/data/features.toml` and the ids in
  `tickets/open/` and `tickets/done/`, and writes `<dir>/data/tickets.toml`
  (ignored by git): each card with `tickets`, by its title, with its made
  status and link text ("Ticket 2220", "Tickets 2904, 2107 and 2108"). It
  fails, naming the card, on `tickets` beside `status`, on an empty
  `tickets`, and on an id in neither folder. `--site` is a path from the
  current folder, as `changelog --site` takes, so `cargo xtask site-data
  --site .` works from `site/`.
- `site/templates/index.html` reads it with `load_data(...,
  required=false)` and takes it over the card's own; without it, a card
  shows its `status`, or "Planned", and its tickets joined by commas.
- `pages.yml` and CI's `site` job run it before `zola build`.
  `site/check-build.sh` then checks the built home page shows every card's
  made status (when the file is there, as in CI): this is the "Site" test.
- `cargo xtask api` with `--pr-branch` runs a second rule, `book_rule`: a
  listing diff or a semver finding, no file under `site/content/learn/` or
  `site/snippets/` in `git diff --name-only <base>`, a branch naming a
  ticket, and no `Book:` line in that done ticket's notes fails, naming the
  changed crates and both ways to pass. A branch with no ticket is skipped.
  The summary ends with `**Book:**` and the reason, or quotes the line.
  `book_lines` sits beside `breaking_lines` in `tickets.rs`, both reading
  the completion notes through one helper.
- Docs: `tickets/TEMPLATE.md`'s criterion, the `work-ticket` skill's step
  5.2, the `run-gates` skill's *The public API*, the docs map's rows for
  the book, the Bevy features and the site's pages, `site/README.md`
  (*Preview it*, the two data files, *What follows the tickets*), and the
  comments in `ci.yml` and `xtask`'s usage.

Deviations:

- **The cards partly there name a done ticket.** No ticket any card named
  was done, so moving them to `tickets` alone would have made "2D
  renderer" and "UI" say "Planned" though part of each works today. Each
  now names a done ticket that shipped part of it too: UI 2309 and 2310
  (widgets), 2D renderer 2332 (the raster painter), and the file's header
  says to do so. The 2D renderer keeps a link text of its own, for its
  range ("Tickets 2332, 2222, 2335–2338").
- **Sound keeps its hand status.** Today's sound came with the carve from
  the game, so no ticket here shipped it, and its tickets (3029,
  2341–2345) alone would make it "Planned". It keeps `status = "In
  progress"`, its link text no longer names tickets ("Sound on the
  roadmap"), and a comment above it says to move it to `tickets` once one
  of those is done. 3029 and 2341 each have a **From 2815 (done)** note
  saying so.
- **The site test greps the made statuses of every card**, in
  `check-build.sh`, not one card whose tickets are all done: no card's
  tickets are all done yet. "In progress" for UI and 2D renderer can only
  come from `tickets.toml`, since those cards have no `status`.

No break: `xtask` isn't one of the crates a game uses.

Book: none, this changes the repository's tooling and the site's home
page, not anything the book teaches.

Follow-up tickets: none.

Nick input: none.
