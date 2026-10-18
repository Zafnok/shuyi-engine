---
id: "2004"
title: "Guides: one per direction and one per move between directions, kept in step with the catalog"
type: infra
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["2003", "1132"]
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
stages. The catalog (2003) lists the directions and modules. The docs
site (1132) publishes the guides.

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
  - `top-down-grid.md`, written in full from `hello-grid` (1118);
  - `top-down-free.md`, `isometric.md` and `side-view.md`, as planned
    pages: one paragraph, the parked tickets, and what not to use
    meanwhile.
- **Move guides**: `top-down-free-to-top-down-grid.md` as a planned page
  pointing at 1125, which writes it in full.
- **A check**, part of `cargo xtask check-modules` (2003):
  - every direction in `docs/modules.ron` has a direction guide;
  - every guide links only to modules and tickets that exist;
  - a guide marked planned names at least one open ticket.
- **The docs site** (1132) publishes `docs/guides/` with the catalog.
- **The skill pack** (1134): the `shuyi-new-game` skill sends an agent
  to the direction guide before it chooses a template.

**Out (do not do):**
- writing guides for directions that aren't built, beyond the planned
  pages;
- tutorials longer than one page (the examples are the long form).

**Either order with 2502** (the wizard, ADR-0011): if 2502 is done, each
direction guide's "which template starts it" names the wizard answers
and the `shuyi new … --no-input` command. If not, write the template
name, and 2502 updates the guides.

## Implementation steps

1. Write `docs/guides/README.md` and the top-down grid guide.
2. Write the planned pages.
3. Add the checks to `check-modules`, with fixtures.
4. Publish them on the docs site and point the `shuyi-new-game` skill at
   them.
5. Add a line to `tickets/TEMPLATE.md`'s acceptance criteria: "Guides
   in `docs/guides/` updated, if this adds a direction, a module or a
   move (ADR-0009 §5)."

## Acceptance criteria

- [ ] Every direction in `docs/modules.ron` has a guide, and the check
      fails on a fixture without one.
- [ ] The top-down grid guide's code runs (it is a doc test, or taken
      from `hello-grid`'s tests).
- [ ] The guides are on the docs site.
- [ ] Nick's sign-off is recorded in the Completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the guide checks, on fixtures.
- Doc test: the top-down grid guide's code.

## Completion notes
