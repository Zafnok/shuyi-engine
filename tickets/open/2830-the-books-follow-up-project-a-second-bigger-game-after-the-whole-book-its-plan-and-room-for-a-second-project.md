---
id: "2830"
title: "The book's follow-up project: a second, bigger game after the whole book, its plan of steps, and room in the site for a second project"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2824", "2825"]
nick_input: decision
completed:
---

# 2830 — The book's follow-up project

## Context

Nick, 2026-11-08 ([`docs/decisions.md`](../../docs/decisions.md), *the
book's project folds into its chapters, and a fourth kind of reader*):
"many tutorial books for lets say Rust Book will have an end of chapter
project or something or a keystone project or followup project after
finishing the whole book". He chose both. The box-pusher is folded into
the chapters, a step after the chapter whose idea it uses (2824). This
ticket is the other half: a second, bigger game, built after the whole
book with everything it taught, as the Rust book ends on its web server.

[ADR-0090](../../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md)
§1 says what a second project adds:

- its section, `site/content/learn/book/<name>/`, with
  `template = "project.html"`, `page_template = "chapter.html"`,
  `[extra] project = true` and `code`;
- a `{ section = "<name>" }` entry in the book's `[extra] parts`, after
  the guides and before the appendices;
- its steps' code in `site/projects/<name>/<step>/`. The box-pusher's
  code moves from `site/tutorial/` to `site/projects/box-pusher/` in
  the same PR, so the projects sit side by side;
- the `snippet` shortcode's `step` and a try's `step` in `tries.toml`
  naming `<project>/<step>`;
- the `tutorial` job's matrix, `web-render`'s loop, `pages.yml`'s copy
  to `play/`, and `check-build.sh`'s `play/` check, each looping over
  the projects.

The box-pusher is a turn game on a grid
([ADR-0003](../../docs/adr/0003-game-styles-stages-two-kinds-of-simulation-cameras-and-projections.md)).
A follow-up project shows what the book's first project couldn't, such
as a game that steps in real time or one that draws pictures rather than
glyphs.

A whole project is more than one PR (`docs/decisions.md`, 2026-10-29: a
ticket is about 1,500 changed lines or fewer). This ticket chooses the
game, plans its steps, makes the room, and writes its first step. It
then writes a ticket for each batch of the remaining steps.

## Nick input

**Decision:** which game the follow-up project builds. Ask with the
`ask-nick` skill. Give two to four options, each a small game that a
reader finishes in about ten steps, drawn from the engine's demos (the
decisions of 2026-10-30, *real demos*) or from what the engine already
ships, with what each teaches that the box-pusher doesn't. Recommend one.
Don't propose anything from Visions of Shuyi (`AGENTS.md` rule 4).

## Scope

**In:**
- **The game, chosen by Nick,** and a plan of its steps, in this
  ticket's completion notes. Each step names what it adds, the chapters
  it leans on (by title and link), and its data files for 2820's
  playground.
- **Room for a second project**, as ADR-0090 §1 lists: the code move
  (`git mv`, so history follows), `snippet` and `tries.toml` naming
  `<project>/<step>`, `crates/xtask/src/tries.rs` and `llms.rs` reading
  the new paths, the CI jobs and `pages.yml` looping over projects, and
  `check-build.sh`. Every box-pusher page, picture and play link works
  as before.
- **The project's section:** its landing page and the book's `parts`
  entry, so the contents and the pager show it.
- **Its first step:** a page and its whole project, built and tested by
  CI and playable at its `play/`, with its newcomer marks (2825) and its
  pictures as tries.
- **Tickets for the remaining steps,** in batches of one PR each (the
  `write-ticket` skill), and their rows in `docs/ROADMAP.md`.
- `site/README.md` (*What is where*, *The rules*) for two projects, and
  the `document` skill.

**Out (do not do):**
- the remaining steps (the tickets this one writes);
- quick checks and exercises on the new steps beyond what 2827 and 2828
  set as every step's due (each step ticket adds them);
- changing the box-pusher's steps or their place in the chapters;
- anything of Visions of Shuyi.

## Implementation steps

1. Read ADR-0090, 2824's and 2825's completion notes, `site/README.md`,
   `crates/xtask/src/tries.rs`, the `snippet` shortcode, and the CI jobs
   that read `site/tutorial/`.
2. Ask Nick which game, with the `ask-nick` skill, and record his answer.
3. Plan the steps.
4. Move the box-pusher's code and change every reader of its path, with
   their tests; build the site and check every box-pusher page, picture
   and `play/` link as before.
5. Add the project's section and write its first step.
6. Write the tickets for the other steps.
7. Run `cargo xtask llms`, the `document` skill, then the gates.

## Acceptance criteria

- [ ] Nick's choice of game is in `docs/decisions.md`, and the plan of
      steps is in the completion notes.
- [ ] The box-pusher's code is in `site/projects/box-pusher/`, and its
      pages, pictures and `play/` links work as before.
- [ ] The follow-up project's section is in the contents and the pager,
      after the guides and before the appendices.
- [ ] Its first step builds and tests in CI and plays at its `play/`.
- [ ] `llms-full.txt` leaves the new project out, as it does the first.
- [ ] Each remaining batch of steps has its ticket.
- [ ] The book's chapter is updated (2802): this ticket is that change.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- `tries.rs` and `llms.rs`: their tests for `<project>/<step>` paths.
- Site: `check-build.sh` checks each project's steps link their `play/`.
- CI: the first step's tests in the `tutorial` job.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
