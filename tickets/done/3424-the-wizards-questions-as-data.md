---
id: "3424"
title: "The wizard's questions as data: wizard::questions and shuyi new --questions --json"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2502"]
nick_input: none
completed: 2026-11-24
---

# 3424 — The wizard's questions as data

## Context

Split from 2502 at its size valve (2,052 counted lines with this part
in). [ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§1, §2 and §6: the questions come from the engine, the catalog's
concerns and the template; an agent gets them as JSON; at the end the
wizard prints the one command that makes the same game and keeps the
answers in `shuyi.ron` under `created_with`.

**From 2502 (done):** every concern of `docs/modules.ron` but `time`
has `pick` and `question` (`shuyi_cli::catalog::Concern`), and
`bodies` has `serves` (the perspectives its answers are for;
`Concern::serves_any`). `shuyi new` takes a flag per asked concern
(`wizard::concern_flag`: `--perspective`, `--bodies`, `--online`,
`--world-drawing`, `--kind`, `--scripting`), `--targets` (checked
against the engine's profiles, `wizard::engine_targets`, the template's
always-on parts' targets by default, `wizard::default_targets`), `--id`
(`NewInput::id`, `NewOutput::id`, the `{{id}}` placeholder) and
`--no-input`. The composer refuses a planned or parked answer of a
concern with its status and tickets (`template::check_answers` over
`Concerns`), gives each concern its first ready answer when none is
given, and turns on a module a picked answer names. Nothing lists the
questions yet, and nothing records how a game was made.

## Nick input

None.

## Scope

**In:**
- **`wizard::questions(&Catalog, &Template, &[Target], &Named,
  &Answers) -> Vec<Question>`**, pure, in
  `crates/cli/src/contract/new/wizard.rs`:
  1. the folder, the name and the game's id (text questions, their
     defaults the folder, its name and `com.example.<crate>`);
  2. the perspectives (`Many`), always asked;
  3. one question per catalog concern, in the catalog's order, never
     `time`;
  4. the template's own questions;
  5. the targets (`Many`), from `engine_targets`, defaulting to
     `default_targets`.

  A `Question` has its `id`, who asks it (`engine`, `catalog`,
  `template`), what it takes (`text`, `one`, `many`), its `answers`
  (each with its `id`, a few words, the catalog's `Status` and its
  `tickets` if not ready), its `default`, its `flag`, and `skipped`
  with why when it isn't asked: none of its answers is for the
  perspectives picked (`serves`), or fewer than two of its answers are
  ready or planned (ADR-0013 §1's World drawing rule, for every
  concern), or another question answers it (`private-files`, which the
  GitHub questions answer). The types are `#[non_exhaustive]` (ADR-0039).
- **`shuyi new <dir> --questions [--json]`** answers `Answer<Questions>`
  for the answers given so far (so `--perspective` changes what is
  asked), and makes nothing. Its schema is a new file,
  `crates/cli/schemas/new.questions.json`, in `contract::schemas()`
  after `export.exposure.json`, its `op` the `new` const; raise
  `contract::VERSION`. Without `--json` it prints the same for people.
- **The command again.** `NewOutput::command`: `shuyi new <dir>
  --no-input --name <name>` with `--id` if given, each concern's
  answers by its flag (an empty list as `--set <id>=`), `--targets`,
  `--examples`/`--no-examples`, `--set` for the template's other
  questions, and the engine (`--engine-rev`, `--engine-path`) and
  `--private-url`; each word quoted for PowerShell and a POSIX shell
  alike. `new`'s text ends with it.
- **`created_with`** in `shuyi.ron`: the composer merges `created_with:
  (template, version, answers)` into the game's `shuyi.ron` (through
  `ron_merge`), and `project::Config` gains `created_with:
  Option<CreatedWith>` (it denies unknown fields).
- **The skill** `skills/shuyi-new-game/SKILL.md`: call `shuyi new <dir>
  --questions --json`, ask the perspectives first and list again with
  them, ask each question not `skipped`, show planned and parked answers
  as coming with their tickets, and pass the answers as flags with
  `--no-input`; `check-skills` passes.
- **Docs:** `docs/templates.md`'s *The wizard's questions* gains
  `--questions`, the printed command and `created_with`; the book's quick
  start names `--questions` and the command; `docs/contract.md`'s `new`
  row.
- **Either order with 2739** (the `stage-arcade-grid` part, which asks
  the template's own `time` question): if 2739 is done, check that
  `--questions --json` lists `time` among the template's questions; if
  not, nothing to do (2739 checks it, its clause says so).

- **Either order with 3338** (each target's support level): if 3338 is
  done, the targets question offers a `Preview` profile like a supported
  one, with "preview: not yet run on its device" beside it, and never
  offers a `Planned` one; if not, nothing to do (3338 does it, its
  clause says so).

**Out (do not do):**
- the terminal's prompts (3423);
- the manifest's `targets` following the targets picked (2504).

**Estimated size:** about 1,150 lines (code 500, tests 450, docs 200).

**Split:** past 1,000 lines, split off `created_with` and the printed
command into a new ticket.

## Implementation steps

1. Write `wizard::questions` and its types, with the skipping rules, and
   their tests over a fixture catalog.
2. Add `--questions` (`questions::Flags`, `cli.rs`), `wizard::listed`
   and its text, and the schema.
3. Write `wizard::command` and `NewOutput::command`, and print it.
4. Write `created_with` in the composer and `project::CreatedWith`;
   bless `plans.golden`.
5. Update the skill and the docs.

## Acceptance criteria

- [x] `shuyi new demo --questions --json` lists every catalog concern
      but `time`, each answer with its status and tickets, makes
      nothing, and names only fields its schema has.
- [x] Bodies and motion is `skipped` for a grid-only game and asked with
      `--perspective top-down`… once a free perspective is ready (a
      fixture catalog shows it).
- [x] ~~Running the printed command with `--no-input` into another folder
      gives the same bytes.~~ (not done: split off to 3425, whose
      acceptance has it)
- [x] ~~A new game's `shuyi.ron` holds `created_with` with every answer,
      and loads.~~ (not done: split off to 3425, whose acceptance has it)
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `wizard::questions` over fixture catalogs (a concern skipped
  for grid-only, `time` never asked, planned answers marked, template
  questions last before targets); `wizard::command`'s flags and
  quoting; `created_with` merged.
- Integration: `--questions --json` against its schema; the printed
  command reproduces the game.

## Completion notes

What was done:

- `wizard::questions(&Catalog, &Template, &[Target], &Named, &Answers)
  -> Vec<Question>` in `crates/cli/src/contract/new/wizard.rs`, pure:
  the folder, the name and the id (text, defaulting to the folder, its
  name and `com.example.<crate>`); the perspectives, always asked; one
  question per concern with a `pick`, in the catalog's order (never
  `time`); the template's own; the targets. A `Question` has `id`,
  `asker` (`engine`, `catalog`, `template`), `text`, `takes` (`text`,
  `one`, `many`), `answers` (`Offered`: `id`, `text`, the catalog's
  `Status`, `tickets` when not ready), `default`, `flag` and `skipped`.
  `Questions` holds the template's id and the list. Every type is
  `#[non_exhaustive]` (ADR-0039).
- The skipping rules: a concern none of whose answers `serves` the
  perspectives picked; a concern with fewer than two answers ready or
  planned (every concern but the perspectives); a template question
  another answers (`private-files`, which the GitHub questions answer).
- `shuyi new <dir> --questions [--json]` (`questions::Flags::questions`,
  `questions::listed`, `wizard::listed`) answers `Answer<Questions>` for
  the answers given so far (`--template`, `--answers`, `--set`, the
  concern flags, `--examples`), and makes nothing; `wizard::text` prints
  it for people. Its schema is `crates/cli/schemas/new.questions.json`,
  after `export.exposure.json` in `contract::schemas()`, its `op` the
  `new` const; `contract::VERSION` is 57.
- The skill `shuyi-new-game` says how an agent lists the questions, asks
  the perspectives first and lists again, shows planned and parked
  answers as coming, and passes the answers as flags with `--no-input`.
- Docs: `docs/templates.md`'s *The wizard's questions* (the skipping
  rules and `--questions`), `docs/contract.md`'s `new` row and *The
  schemas*, the book's quick start.
- Tests: `wizard::tests` over a fixture catalog where a free
  perspective is ready (the order, `time` never asked, bodies skipped
  for grid-only and asked of a free game, planned answers marked with
  their tickets, the fewer-than-two rule, `private-files`, the answers
  given as defaults, the text); `new::tests`' listing makes nothing and
  follows the answers; the binary's `--questions --json` checked field
  by field against `new.questions.json` (a small walker over the
  schema's properties, `$ref`s and `anyOf`), and its text.

Deviations:

- **Split at the size valve.** With the printed command and
  `created_with` in, the PR measured 1,381 counted lines; the ticket's
  **Split:** says past 1,000 to split them off. They are ticket 3425
  (`tickets/open/3425-the-command-again-and-created-with.md`), whose code
  and tests were written and passed here and are kept on the local
  branch `t3425-the-command-again-and-created-with`. Without them this
  PR is 968 counted lines.
- The signature's `&Named` was undefined: it is `wizard::Named { dir,
  name, id }`, each as given or at its default.
- `Question` gained `text` (the words that ask it), which the ticket's
  list of fields left out and an agent needs to ask it; who asks it is
  the field `asker`.
- The perspectives are asked by `engine` (ADR-0013 §1 counts them among
  the engine's own), with the catalog's answers.
- A planned target is listed with status `planned` and "the engine
  doesn't run on it yet", shown and never picked, as ADR-0013 §6 shows a
  planned answer.
- Either order with 2739 and 3338: neither is done, so nothing to do
  here; their clauses now say what holds (2739's test of the template's
  `time` question, 3338's `Preview` target in the targets question).

Follow-up tickets: 3425 (the command again and `created_with`, split
from this one). 3423 (the terminal's prompts) now names what this left
it.

Breaking: none. The public API only gains `contract::new::wizard`'s
`Asker`, `Takes`, `Offered`, `Question`, `Questions`, `Named`, the
question ids' consts, `questions`, `listed` and `text`; `Flags` is in a
crate-private module.

Behaviour: shuyi-cli: every contract answer carries `contract: 57`, and the schemas gain `new.questions.json` (`shuyi schema --out` writes 50 files).
Behaviour: shuyi-cli: `shuyi new` takes `--questions`, which lists the questions and makes nothing; its usage names it.

Searched the game (`gh search code --repo Zafnok/visions-of-shuyi
'NewOutput'`, `'contract::new'`, `'shuyi new'`): it names `NewOutput`
only in `docs/engine/pins.md` and `shuyi new` in the skills it copies
from the engine's pack at the pin (game ADR-0097), so nothing in its
code changes; the skill comes with the pin.

Nick input: none.
