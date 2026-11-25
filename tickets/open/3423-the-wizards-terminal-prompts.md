---
id: "3423"
title: "The new-game wizard's prompts: shuyi new asks its questions in a terminal as lists to pick from, the GitHub and examples questions among them"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["3424"]
nick_input: sign-off
completed:
---

# 3423 — The new-game wizard's prompts

## Context

Split from 2502 at its size valve. Nick, 2026-10-17
(`docs/decisions.md`): `shuyi new` should ask "do you want top down?
side scroller?" and let the developer "tick off multiple".
[ADR-0013](../../docs/adr/0013-starting-a-game-a-wizard-over-the-catalog-and-templates-made-of-parts.md)
§1, §2 and §6.

**From 2502 (done):** the flags (`--perspective`, `--bodies`,
`--online`, `--world-drawing`, `--kind`, `--scripting`, `--targets`,
`--id`, `--set`) and `--no-input` work.

**From 3424 (done):** `wizard::questions(&Catalog, &Template,
&[Target], &Named, &Answers) -> Vec<Question>` (in
`crates/cli/src/contract/new/wizard.rs`) gives every question in order
(the folder, the name and the game's id; the perspectives; one per
catalog concern but `time`; the template's own; the targets). A
`Question` has its `id`, its `asker` (`Engine`, `Catalog`, `Template`),
its `text`, what it `takes` (`Takes::Text`, `One`, `Many`), its
`answers` (`Offered`: `id`, `text`, the catalog's `Status`, `tickets`
when not ready), its `default` (the answers given so far, else its
default), its `flag` (`None` for the folder) and `skipped` with the
reason when it isn't asked. `wizard::listed(cwd, &NewInput)` computes
them for `shuyi new <dir> --questions` (`questions::Flags::questions`,
`questions::listed`), and `wizard::text` prints them. The command that
makes the game again and `created_with` are 3425's. Nothing asks them
in a terminal yet. `shuyi new` still asks the
plain line prompts 2592 and 2597 wrote
(`crates/cli/src/contract/new/questions.rs`): the examples, then the
six GitHub questions.

## Nick input

**Sign-off:** run `shuyi new` in a terminal and answer the questions.
Say whether the wording is clear and whether anything you expected to be
asked is missing.

## Scope

**In:**
- **Prompts** in a terminal, over `wizard::questions`: a text line for
  the name, the folder and the id (the id checked by
  `app_info::check_id` as it is typed), a single-select list for a
  `One` question and a multi-select list for a `Many` one. The
  perspectives come first, then `wizard::questions` is called again with
  them, so a concern skipped for them (bodies and motion for a
  grid-only game) isn't asked. A skipped question prints one line: its
  answer, and why it wasn't asked.
- **Planned and parked answers** are shown under the list, greyed, each
  with its status and tickets, and can't be selected (ADR-0013 §6).
- **The library:** `dialoguer` (MIT, checked on 2026-11-24 with `cargo
  info dialoguer`: 0.12.0, MIT; it uses `console` 0.16, already in the
  lock). Check the license again from its `Cargo.toml` and repository on
  the day, that `cargo deny` passes, and turn off its default features
  (`editor` pulls `tempfile`, `password` pulls `zeroize`): only
  `Select`, `MultiSelect` and `Input` are needed. Never require MSVC.
- **Keys:** the library's own keys (arrows, space, enter) are a
  terminal's, not the game's, so rule 8 doesn't apply to them. The help
  line under each prompt still names them.
- **Behind a trait:** the prompts ask through a `Prompter` (one method
  per kind of prompt), so a test answers them from a script; the
  terminal's `Prompter` is `dialoguer`'s.
- **The GitHub questions and the examples one (from 2592 and 2597):**
  move `questions::examples` into the wizard (it is the template's
  `examples` question, asked with the template's), and the six GitHub
  questions after the catalog's and the template's, in the wizard's
  prompt style. `--questions --json` lists the GitHub questions too,
  with a field saying an agent must not answer `github: yes` itself.
  Every flag stays (`--examples`, `--no-examples`, `--github`,
  `--no-github`, `--owner`, `--visibility`, `--assets-visibility`,
  `--private`, `--private-folder`), and `--no-input` still means
  `--no-github`.
- **Either order with 2848** (the game's license): if 2848 is done,
  `questions::license` (`--license`, `--copyright`) moves in with the
  GitHub questions, in the wizard's prompt style, and `--questions
  --json` lists it with a field saying an agent asks its user and never
  picks a license itself; `--no-input` means `none`, and both flags
  stay. If not, nothing to do (2848 adds it to the wizard when it
  lands).
- **The skill** `skills/shuyi-new-game/SKILL.md` names the GitHub
  questions' field; `docs/templates.md`'s wizard section says what is
  asked in a terminal.

**Out (do not do):**
- new questions, flags or answers (2502 made them);
- a graphical wizard.

**Estimated size:** about 900 lines (code 400, tests 350, docs 150).

## Implementation steps

1. Add `dialoguer`, checked as above.
2. Write the `Prompter` trait, the terminal's, and a scripted one for
   tests.
3. Ask `wizard::questions` through it in `questions::main`, before the
   game is composed, and put the answers in the input's `set`.
4. Move the examples and GitHub questions into it, and list the GitHub
   ones in `--questions --json`.
5. Update the skill and the docs.

## Acceptance criteria

- [ ] In a terminal, `shuyi new demo` asks the name, the id, the
      perspectives (top-down grid ticked), the questions the catalog
      asks for them, the template's, the targets and the GitHub ones,
      each a list where it has answers, planned and parked answers
      greyed with their tickets.
- [ ] Answering the prompts (through the scripted `Prompter`), then
      running the command `new` prints with `--no-input` into another
      folder, gives the same bytes.
- [ ] Off a terminal, or with `--no-input`, nothing is asked.
- [ ] `--questions --json` lists the GitHub questions, with the field
      about `github: yes`, and matches its schema.
- [ ] Nick's sign-off is recorded in the Completion notes.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each prompt kind through the scripted `Prompter`; a planned
  answer can't be picked.
- Integration: the prompts' game against the printed command's; the
  questions JSON against its schema.

## Completion notes
