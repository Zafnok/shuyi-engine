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
  today (2003 wrote six): time, perspective (`Many`), bodies and
  motion (`Many`), online play (`One`), world drawing (`Many`: a 3D
  stage and a 2D one draw differently) and kind of game (`Many`, "What
  kind of game is it?", ADR-0015 §1). **Either order with 2205**
  (parked: the music concern): if 2205 is done, give music `pick: Many`
  and the question "What plays the music?"; if not, nothing to do (2205
  gives them when it lands). Add both fields to
  `shuyi_cli::catalog::Concern`, which denies unknown fields, and to
  the xtask's `Concern` (`crates/xtask/src/modules.rs`), and make
  2003's `check-modules` require both fields on every concern but
  `time` (below). `Concern` is in `shuyi modules`'s answer: regenerate
  its schema and raise `contract::VERSION`. It is a public struct with
  public fields, so write the `Breaking:` line CI's `api` job asks for.
- **Time is not asked** (ADR-0013 §1): it follows from the
  perspectives, so the `time` concern gets neither field and the wizard
  has no `--time` flag. A template asks its own `time` question when a
  game needs both, as 2739's `stage-arcade-grid` part does. Its answers
  are the template's (`turns`, `ticks`), not the catalog's.
- **Perspectives are catalog answers.** Make sure every answer of the
  perspective concern in `docs/modules.ron` (2003 put them there) has a
  status and its tickets: `top-down-grid` ready; `top-down`,
  `isometric` and `side-view` from 1122, 1126 and 1127; `first-person`,
  `third-person` and `top-down-3d` from 3101 (ADR-0011 §2), where
  2003 put them; add 3108 to the first two and 3109 to the third,
  since they set them ready. The *World drawing* concern (HD-2D)
  becomes a question like any other, asked only when one of its
  answers other than the default is ready or planned.
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
     (bodies and motion isn't asked for grid-only games, ADR-0013 §1),
     and never `time`;
  4. the template's own questions;
  5. the targets (`Many`), from the target profiles (3001).

  Each answer carries its status (`Ready`, `Planned { ticket }`,
  `Parked { ticket }`) and a one-line description. 2003's module
  metadata (`ModuleInfo`) has none, and an answer by `default` or by a
  module not built yet has no crate to read one from: take it from
  `docs/modules.ron` (the answer's `name`, or a new field beside
  `pick` and `question`).
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
  `--perspective`, `--bodies`, `--music` (if 2205 is done), `--kind`
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
- **The skill pack. Either order with 1134:** if 1134 is done, update
  the `shuyi-new-game` skill to call `--questions --json`, ask its
  user, and pass the answers as flags. If not, nothing to do (1134
  writes it, its own clause says so).
- **Docs:** `docs/templates.md` (2501) gets a section on the wizard and
  its flags. If 2004 is done, each direction guide names the answers
  that start it.
- **Either order with 2206** (parked: MIDI playback and its
  `music-midi` part): if 2206 is done, a test makes `shuyi new demo
  --no-input --music midi` and checks that the game builds and plays the
  template's sequence; if not, nothing to do (2206 adds that test).
- **The RTS answer, either order with 2707:** if 2707 is done, list
  `--kind rts` among the flags in `docs/templates.md`. If not, nothing
  to do: 2707 checks the list when it lands (its clause says so).
- **The RTS part, either order with 2719:** if 2719 is done, a test
  checks that `--kind rts` makes the game its `kind-rts` part makes. If
  not, nothing to do: 2719 checks `--kind rts` when it lands (its
  clause says so).
- **Either order with 2726, 2732, 2735 and 2754** (parked: the
  `kind-visual-novel`, `kind-mystery`, `kind-roguelike` and
  `kind-dungeon-crawler` template parts): if one is done, its part is
  made with `--set kind=<answer>`. Check that `--kind <answer>` makes
  the same game, and change that command to `--kind` where its guide
  and CI name it. If not, nothing to do: each uses `--kind` when it
  lands (its scope says so).
- **Either order with 2739** (parked: the `stage-arcade-grid` part): if
  2739 is done, the built-in template already asks `time` among its own
  questions (step 4 above), so the prompts and `--questions --json`
  show it; check that they do. If not, nothing to do: 2739 adds the
  question to the template, and its scope says the prompts show it.
- **Either order with 2771** (parked: the `stage-hex-grid` part): if
  2771 is done, `--template hex` and `--set perspective=hex-grid` make
  the game its part makes. Check that `--perspective hex-grid
  --no-input` makes the same plan (a test), and name that command
  beside `--template hex` in `docs/guides/hex-grid.md`'s *what starts
  it*. If not, nothing to do: 2771 names the command and adds the test
  when it lands (its clause says so).

**Out (do not do):**
- moves between stages (2503); until then, several perspectives give
  separate stages and a `limits` entry;
- checking targets against modules, per-target CI (2504);
- a planned target's ticket: a profile's `Status::Planned` holds none
  (3001), so a planned target shows its status alone; 2504 adds the
  tickets to the profile format;
- templates from outside the engine (2505);
- a graphical wizard (the CLI and JSON are enough for now).

## Implementation steps

1. Add `pick` and `question` to the catalog, and the perspective
   answers, and update `check-modules` and its fixtures.
2. Write `wizard::questions`, with the skipping rule, and its tests.
3. Write the prompts, the flags, `--no-input` and `--questions --json`.
   Check whether 3048 is done; if so, add the id question and `--id`.
4. Print the equivalent command and write `created_with`.
5. Update the skill if 1134 is done, and the docs.

## Acceptance criteria

- [ ] `shuyi new demo --no-input` makes the same game as `shuyi new
      demo --template grid`.
- [ ] `shuyi new demo --questions --json` lists every catalog concern
      with each answer's status, and matches its schema.
- [ ] A parked answer by flag (for example `--bodies procedural`)
      fails, naming its status and ticket 2701.
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
- [ ] `check-modules` fails on a concern other than `time` without
      `pick` or `question`.
- [ ] Nick's sign-off is recorded in the Completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `wizard::questions` over fixture catalogs (a concern skipped for
  grid-only, `time` never asked, planned answers marked, template
  questions placed last before targets); flag parsing to `Answers`.
- Integration: `--questions --json` against its schema; `--no-input`;
  the printed command reproduces the game; the every-ready-answer game
  builds (CI).

## Completion notes
