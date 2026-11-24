---
id: "2004"
title: "Guides: one per direction and one per move between directions, kept in step with the catalog"
type: infra
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2003", "2802", "2810"]
nick_input: sign-off
completed: 2026-11-24
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
  catalog. Give the guides a heading in Learn
  (`site/templates/learn.html`).
  **From 2823 (done):** [ADR-0090](../../docs/adr/0090-one-book-the-project-folded-into-the-chapters-a-newcomer-mark-around-a-passage-and-a-readers-filter-kept-in-the-browser.md) §1–§2 make the guides a part of the
  book, *Guides*, after the chapters and before the follow-up project
  (2830) and the appendices. It is a
  section, `site/content/learn/book/guides/`: an `_index.md` holding the
  catalog's lines, and the copied guides as its pages.
  **From 2824 (done):** the book's contents (`book.html`) and its pager
  (`book-macros.html`) come from the book's `[extra] parts`
  (`site/README.md`, *The book's order*): add `{ section = "guides" }`
  there, after the `chapters` entry. A `section` entry already lists the
  folder's landing page, then its pages by `weight`.
  The site's *How-to* section (2843, `/learn/how-to/`) is generated
  from the skill pack and is a separate thing from this *Guides* part.
  Keep the two names apart.

  **From 2808 (done):** the guides reach Learn as Markdown files
  copied into `site/content/learn/book/guides/` before Zola runs, in `pages.yml`
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

- [x] Every answer the check covers in `docs/modules.ron` has a guide,
      and the check fails on a fixture without one.
- [x] The top-down grid guide's code runs (it is a doc test, or taken
      from `hello-grid`'s tests).
- [x] The guides are on the docs site.
- [ ] Nick's sign-off is recorded in the Completion notes.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the guide checks, on fixtures.
- Doc test: the top-down grid guide's code.

## Completion notes

**Done.**

- `docs/guides/`: the README (how to choose a direction, the concerns in
  plain words, what each guide holds, the list), `top-down-grid.md` in
  full with its *In real time (planned)* section, and 17 planned pages:
  `top-down-free`, `isometric`, `side-view`, `hex-grid`,
  `dungeon-crawler`, `first-person`, `third-person`, `top-down-3d`,
  `hd-2d`, `rts`, `visual-novel`, `mystery`, `roguelike`, `party-rpg`,
  `action-rpg`, and the move guide `top-down-free-to-top-down-grid`.
  None of 1125, 1126, 1127, 2117, 2502, 2503, 2206 and 2739 was done, so
  each took the *if not* branch.
- Each guide starts with its facts, `- **Covers:**` (or `- **Move:**`),
  `- **Status:**` and `- **Tickets:**`, which the check reads
  (`crates/xtask/src/guides.rs`).
- The check, in `cargo xtask check-modules`: a guide for every answer of
  `perspective` and `kind`, and of `world-drawing` but `flat`; every
  answer a guide names in `docs/modules.ron`; every relative link there;
  every ticket named open or done, a planned guide naming an open one;
  each guide linked from the README; and a guide with Rust included as a
  doc test. 20 unit tests on fixtures, and one over the repository's own.
- The doc test: `crates/shuyi/src/lib.rs` includes `top-down-grid.md`
  under `#[cfg(doctest)]` (`TopDownGridGuide`), so `cargo test -p shuyi
  --doc` runs its Rust.
- The book: `cargo xtask site-data`, which `pages.yml`, CI's `site` job,
  the local `site` gate and `book-pad` already run before Zola, copies
  the guides into `site/content/learn/book/guides/` (never committed):
  the README as `_index.md` (new template `guides.html`), each guide a
  page weighted in the README's order, links between guides made `@/`
  links and the others links to GitHub. `{ section = "guides" }` follows
  the chapters in the book's `parts`, and Learn has a *Guides* heading.
  `llms` passes: `llms-full.txt` is 343,164 bytes of its 400,000.
- `skills/shuyi-new-game` sends an agent to the direction guide before it
  picks a template; a new game's `AGENTS.md`, at a commit or in a
  checkout, now lists the engine's `docs/guides/` (at `main` it already
  had `llms.txt`, which holds them).
- `docs/README.md`: a row for the guides under *People making games*,
  and the `27xx` block reads them. `tickets/TEMPLATE.md` has the guides'
  acceptance line. `changes.yml` counts `docs/guides/**` as code (the
  `modules` and `test` jobs read them).

**Deviations.**

- Ticket 39 days old: re-checked. `docs/modules.ron` gives `top-down`
  the tickets 1120 and 1122, `rts` 2707, `visual-novel` 2721, `mystery`
  2727, `roguelike` 2118 and 2119, `dungeon-crawler` 2741, `party-rpg`
  2745 and `action-rpg` 2756: each planned page names those and the
  ticket that writes it in full. No module README exists for the default
  crates, so the guides link their folders.
- Learn's heading went in `site/content/learn/_index.md`, Learn's text,
  not `learn.html`, which only adds the API list after it.
- `site/check-build.sh` bars fenced Rust in the book (its code comes from
  `site/snippets/`). The guides are excepted: they are read on GitHub
  too, where no shortcode runs, and their Rust is a doc test instead.
- The docs map's rule 4 keeps tickets out of what people making games
  read. The guides name tickets because this ticket asks for it, as the
  catalog (`docs/modules.md`) does; the README says what a ticket is.
- No ticket writes `party-rpg.md` in full: its page says so, and names
  2745–2752.

Behaviour: shuyi-cli: `shuyi new` with `--engine-rev` or
`--engine-path` writes a line more in the game's `AGENTS.md`, its
engine's guides (`docs/guides/`). Searched the game: `gh search code
--repo Zafnok/visions-of-shuyi 'its book, in Markdown'` and
`'engine_docs'`, no hits.

No break: `TopDownGridGuide` is built only under `cfg(doctest)`.

Game: nothing to adopt, the game names none of this.

Nick's sign-off: asked when the PR opened; not yet recorded.
