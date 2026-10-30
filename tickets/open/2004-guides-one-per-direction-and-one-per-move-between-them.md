---
id: "2004"
title: "Guides: one per direction and one per move between directions, kept in step with the catalog"
type: infra
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2003", "2802", "2810"]
nick_input: sign-off
completed:
---

# 2004 — Guides: one per direction, one per move

## Context

Nick, 2026-10-16 (`docs/decisions.md`): "we should make it very obvious
OK this behavior is built for this type of game, this one for this type
of game, here is how to transition between the two... make it smartly
designed and well documented at all steps."

[ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)
§5 sets two kinds of guide in `docs/guides/`: a direction guide for each
kind of game, and a move guide for each move a game can make between
stages. The catalog (2003, done) lists the concerns, their answers and
the modules. The website's book (2802, done) publishes the guides.

At the MVP only one direction is built: top-down grid with turns, the
game's own. The others are parked. Their guides exist from the start as
short "planned" pages, so a developer sees what is coming and doesn't
mistake one direction's kit for another's.

## Nick input

**Sign-off:** read the top-down grid guide and one planned guide, and
say whether a new developer would know where to start.

## Scope

**In:**
- **`docs/guides/README.md`**: how to choose a direction, the concerns
  of ADR-0009 §2 in plain words, and a list of the guides.
- **Direction guides**, each in the fixed order of ADR-0009 §5 (the
  template, what it uses, what looks related but isn't for it, which
  example to read first):
  - `top-down-grid.md`, written in full from `hello-grid` (1118).
    **Either order with 2117:** if 2117 is done, it says the kit's
    topologies (four-way by default, eight-way with `Topology::Square8`)
    and `find_path` from one tile to another; if not, 2117 adds that
    paragraph. It ends with a short planned section, *In real time*: a
    grid that steps in real time (snake) waits for 2739, which builds
    on 2137. Until then a turn game times its rules in turns, with
    `shuyi_core::timer`'s `Countdown` and `Every`, never with `dt`.
    2739 replaces the section with the full one. If
    2739 is done first, the file already has its full section: keep
    it and write the rest around it;
  - `top-down-free.md`, `isometric.md` and `side-view.md`, as planned
    pages: one paragraph, the parked tickets, and what not to use
    meanwhile. **Either order with 1125, 1126 and 1127:** if one of
    them is done, its guide (`top-down-free.md` from 1125,
    `isometric.md` from 1126, `side-view.md` from 1127) is already
    written in full: keep it and list it in the README. If not, write
    the planned page, and that ticket writes it in full;
  - `hex-grid.md` as a planned page: hex grids, parked until after
    Visions of Shuyi launches (Nick, 2026-10-20), naming 2121 and 2740.
    2740 writes it in full.
- **Move guides**: `top-down-free-to-top-down-grid.md` as a planned page
  pointing at 1125, which writes it in full.
- **Either order with 1125:** if 1125 is done, `top-down-free.md` and
  `top-down-free-to-top-down-grid.md` are already written in full:
  keep them and list them, and write no planned page over them. If
  not, write them as planned pages, and 1125 replaces them.
- **A check**, part of `cargo xtask check-modules` (2003):
  - **From 2003 (done):** `docs/modules.ron` holds concerns and their
    answers, not directions (a direction is one answer per concern,
    ADR-0009 §2). So each guide's first lines name the answers it
    covers, as `concern/answer` ids from that file, and the check wants
    a guide for every answer of `perspective` and `kind`, and for every
    answer of `world-drawing` but `flat`. The file names stay as the
    tickets give them: `top-down-grid.md` covers `top-down-grid` and
    `tactics`; `top-down-free.md` covers `top-down`; `hd-2d.md` covers
    `diorama`; `dungeon-crawler.md` covers `first-person-grid` too; the
    3D pages cover `full-3d`;
  - the parked 3D directions (ADR-0011 §2) get planned pages naming
    their tickets: `first-person.md`, `third-person.md`,
    `top-down-3d.md` and `hd-2d.md`. So do the parked kinds, under
    their answer ids, each naming the tickets `docs/modules.ron` gives
    it and the ticket that writes it in full: `rts.md` (2719),
    `visual-novel.md` (2726), `mystery.md` (2732), `roguelike.md`
    (2735), `dungeon-crawler.md` (2754) and `action-rpg.md` (2764).
    No ticket writes `party-rpg.md` yet, so its planned page names
    2745–2752;
  - every answer a guide names is in `docs/modules.ron`;
  - every guide links only to modules and tickets that exist;
  - a guide marked planned names at least one open ticket.
- **The book. From 2802 (done):** it publishes `docs/guides/` with the
  catalog, as a part after its chapters
  (`site/content/learn/book/`, numbered by `extra.chapter`; a page with
  none lists after them, `site/templates/book.html`): give the guides
  their own heading on the book's contents and in Learn
  (`site/templates/learn.html`).
  **From 2808 (done):** the guides reach Learn as Markdown files
  copied into `site/content/learn/` before Zola runs, in `pages.yml`
  and in `ci.yml`'s `site` job alike (before its `cargo xtask llms`
  too), each with front matter (`title`, `description`, `weight`) and a
  name in slug form. `cargo xtask llms` twins them and puts them in
  `llms-full.txt`: check it passes within its budget, and add the
  expansion of any shortcode they use to `crates/xtask/src/llms.rs`.
- **From 1134 (done)** (the skill pack): the `shuyi-new-game` skill
  (`skills/shuyi-new-game/SKILL.md`) sends an agent to the direction
  guide before it chooses a template. It names the guide in words: the
  guides are the engine's docs, which a game reaches through its
  `AGENTS.md`, and `check-skills` fails a link that leaves the pack.
- **The docs map** (`docs/README.md`, 2810): a row for `docs/guides/`
  under *People making games*, updated by "a change that adds a
  module, a direction or a move", and a line in *What each task reads*
  for the tickets that add one.

**Out (do not do):**
- writing guides for directions that aren't built, beyond the planned
  pages;
- tutorials longer than one page (the examples are the long form).

**Either order with 2502** (the wizard, ADR-0013): if 2502 is done, each
direction guide's "which template starts it" names the wizard answers
and the `shuyi new … --no-input` command. If not, write the template
name, and 2502 updates the guides.

**Either order with 2206** (parked, MIDI playback): if 2206 is done,
each direction guide's "modules" list names MIDI (`shuyi-midi`) as an
option for music. If not, nothing to do (2206 adds it).

**Either order with 2503** (several perspectives in one new game): if
2503 is done, its generated README links ADR-0009 §5 for a door with no
move guide; point that link at `docs/guides/README.md`. If not, nothing
to do (2503 links the guides).

## Implementation steps

1. Write `docs/guides/README.md` and the top-down grid guide.
2. Write the planned pages.
3. Add the checks to `check-modules`, with fixtures.
4. Publish them on the docs site, and point the `shuyi-new-game` skill
   at them (the skill pack's clause above).
5. Add a line to `tickets/TEMPLATE.md`'s acceptance criteria: "Guides
   in `docs/guides/` updated, if this adds a direction, a module or a
   move (ADR-0009 §5)."

## Acceptance criteria

- [ ] Every answer the check covers in `docs/modules.ron` has a guide,
      and the check fails on a fixture without one.
- [ ] The top-down grid guide's code runs (it is a doc test, or taken
      from `hello-grid`'s tests).
- [ ] The guides are on the docs site.
- [ ] Nick's sign-off is recorded in the Completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the guide checks, on fixtures.
- Doc test: the top-down grid guide's code.

## Completion notes
