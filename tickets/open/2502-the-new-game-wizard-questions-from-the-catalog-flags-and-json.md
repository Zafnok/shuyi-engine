---
id: "2502"
title: "The new-game wizard: shuyi new asks one question per concern from the catalog, with multi-pick, a flag for every answer, and the questions as JSON for agents"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["2501"]
nick_input: sign-off
completed:
---

# 2502 — The new-game wizard

## Context

Nick, 2026-10-17 (`docs/decisions.md`): `shuyi new` should ask "do you
want top down? side scroller?", let the developer "tick off multiple",
and ask about physics, "what kind of physics", MIDI sound and export
targets.
[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§1, §2 and §6 set the design:

- the questions come from the engine, from the catalog's concerns
  (`docs/modules.ron`, 2003), and from the template;
- every answer also has a flag, and an agent gets the questions as JSON;
- planned and parked answers are shown with their ticket, never offered.

The template format and the composer are 2501. This ticket asks the
questions and hands the answers to the composer. It allows several
perspectives to be picked, but until 2503 lands it composes one stage
per perspective with no move between them, and says so in `limits`.

## Nick input

**Sign-off:** run `shuyi new` in a terminal and answer the questions.
Say whether the wording is clear and whether anything you expected to be
asked is missing.

## Scope

**In:**
- **The catalog gains two fields per concern** in `docs/modules.ron`
  (ADR-0013 §1): `pick: One | Many` and `question: "…"`. Concerns
  today: perspective (`Many`), bodies and motion (`Many`), online play
  (`One`). Music comes with 2205. Update 2003's `check-modules` to
  require both fields.
- **Perspectives are catalog answers.** Make sure every answer of the
  perspective concern in `docs/modules.ron` (2003 put them there) has a
  status and its tickets: `top-down-grid` ready; `top-down`,
  `isometric` and `side-view` from 1122, 1126 and 1127; `first-person`,
  `third-person` and `top-down-3d` from 3108 and 3109 (ADR-0011 §2). The
  *World drawing* concern (HD-2D) becomes a question like any other,
  asked only when one of its answers other than the default is ready or
  planned.
- **The direction menu.** If the parked 3108 has already added a
  direction menu to `shuyi new`, replace it with the wizard: its
  directions are now perspective answers (ADR-0013 §1). Keep its tests'
  promise that no terminal means no prompt.
- **`wizard::questions(&Catalog, &Template) -> Vec<Question>`**, pure:
  1. the name and folder, then, if 3048 is done, the game's id
     (reverse-DNS, default `com.example.<crate>`, checked by 3048's
     rule);
  2. the perspectives (`Many`);
  3. one question per catalog concern, in the catalog's order, skipping
     a concern none of whose answers applies to the perspectives picked
     (bodies and motion isn't asked for grid-only games, ADR-0013 §1);
  4. the template's own questions;
  5. the targets (`Many`), from the target profiles (3001).

  Each answer carries its status (`Ready`, `Planned { ticket }`,
  `Parked { ticket }`) and a one-line description from the module's
  metadata (2003).
- **Interactive prompts** in a terminal: single and multi-select lists,
  with planned answers shown greyed with their ticket and not
  selectable. Use `inquire` or `dialoguer`. Check the license from its
  `Cargo.toml` and repository on the day (both are believed MIT), that
  `cargo deny` passes, and that its default features pull in nothing
  denied. Never require MSVC.
- **Keys:** the prompt library's own keys (arrows, space, enter) are a
  terminal's, not the game's, so rule 8 doesn't apply to them. The help
  line under each prompt still names them.
- **Flags** for every question: `--id` (if 3048 is done, see below),
  `--perspective`, `--bodies`, `--music` (once 2205 adds it), `--kind`
  (from 2003's *Kind of game* concern, ADR-0015 §1), `--online`,
  `--targets`, and `--set` (2501) for template questions. A planned
  answer given by flag fails with its status and ticket. `--no-input`,
  or stdin not being a terminal, asks nothing and uses the defaults.
- **`shuyi new <dir> --questions --json`** returns the questions, their
  answers and their status, and makes nothing. Its schema is checked
  like the rest of the contract (1129).
- **At the end** the wizard prints the one `shuyi new … --no-input`
  command that makes the same game, and the composer writes the answers
  into `shuyi.ron` under `created_with`.
- **The game's id, either order with 3048:** if 3048 is done, the
  composer writes the id into `shuyi.ron`'s `app.id`, and `app-info`
  writes the template's `app_info.rs`. If 3048 isn't done, nothing:
  3048 adds the question and the flag (its scope says so).
- **Each picked answer that is a module** turns it on through the
  template's parts or, if no part names it, through `shuyi add`'s code
  directly (ADR-0013: every catalog answer works with no part).
- **The skill pack:** if 1134 is done, update the `shuyi-new-game`
  skill to call `--questions --json`, ask its user, and pass the
  answers as flags. If not, add that to 1134's scope in this PR.
- **Docs:** `docs/templates.md` (2501) gets a section on the wizard and
  its flags. If 2004 is done, each direction guide names the answers
  that start it.

**Out (do not do):**
- moves between stages (2503); until then, several perspectives give
  separate stages and a `limits` entry;
- checking targets against modules, per-target CI (2504);
- templates from outside the engine (2505);
- a graphical wizard (the CLI and JSON are enough for now).

## Implementation steps

1. Add `pick` and `question` to the catalog, and the perspective
   answers, and update `check-modules` and its fixtures.
2. Write `wizard::questions`, with the skipping rule, and its tests.
3. Write the prompts, the flags, `--no-input` and `--questions --json`.
   Check whether 3048 is done; if so, add the id question and `--id`.
4. Print the equivalent command and write `created_with`.
5. Update the skill (or 1134's scope) and the docs.

## Acceptance criteria

- [ ] `shuyi new demo --no-input` makes the same game as `shuyi new
      demo --template grid`.
- [ ] `shuyi new demo --questions --json` lists every catalog concern
      with each answer's status, and matches its schema.
- [ ] A parked answer by flag (for example `--bodies procanim`) fails,
      naming its status and ticket 2701.
- [ ] Answering the prompts, then running the printed command with
      `--no-input` into another folder, gives the same bytes (an
      integration test drives the prompts through the library's test
      backend, or through the answers the prompts produce).
- [ ] With every ready answer picked, the new game builds and passes its
      tests in CI.
- [ ] If 3048 is done, `shuyi new demo --no-input --id org.example.demo`
      writes that id into `shuyi.ron`'s `app.id` and `app_info.rs`, and
      the game passes `app-info --check`. A bad id by flag fails with
      3048's error.
- [ ] `check-modules` fails on a concern without `pick` or `question`.
- [ ] Nick's sign-off is recorded in the Completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `wizard::questions` over fixture catalogs (a concern skipped for
  grid-only, planned answers marked, template questions placed last
  before targets); flag parsing to `Answers`.
- Integration: `--questions --json` against its schema; `--no-input`;
  the printed command reproduces the game; the every-ready-answer game
  builds (CI).

## Completion notes
