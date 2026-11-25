---
id: "3423"
title: "The new-game wizard's prompts: shuyi new asks its questions in a terminal as lists to pick from, the GitHub and examples questions among them"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["3424"]
nick_input: sign-off
completed: 2026-11-24
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
`questions::listed`), and `wizard::text` prints them. From 3425 (done):
`new`'s text ends with the command that makes the game again
(`wizard::command`, `NewOutput::command`), so the prompts' answers
reach it as the flags' do, and `shuyi.ron` keeps them under
`created_with`. Nothing asks them
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

- [x] In a terminal, `shuyi new demo` asks the name, the id, the
      perspectives (top-down grid ticked), the questions the catalog
      asks for them, the template's, the targets and the GitHub ones,
      each a list where it has answers, planned and parked answers
      greyed with their tickets.
- [x] Answering the prompts (through the scripted `Prompter`), then
      running the command `new` prints with `--no-input` into another
      folder, gives the same bytes.
- [x] Off a terminal, or with `--no-input`, nothing is asked.
- [x] `--questions --json` lists the GitHub questions, with the field
      about `github: yes`, and matches its schema.
- [x] ~~Nick's sign-off is recorded in the Completion notes.~~ (not done: asked in the PR, recorded here before it merges)
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each prompt kind through the scripted `Prompter`; a planned
  answer can't be picked.
- Integration: the prompts' game against the printed command's; the
  questions JSON against its schema.

## Completion notes

What was done:

- `crates/cli/src/contract/new/prompts.rs`: the `Prompter` trait (`text`,
  `one`, `many`, `say`), the terminal's (`prompts::Terminal`, `dialoguer`
  on stderr, with a theme that puts the planned and parked answers,
  greyed with their status and tickets, and a help line naming the keys
  under the list's last choice), and a scripted one for tests
  (`prompts::scripted`). `prompts::ask` asks `wizard::questions` that
  nothing given answers, listing them again after each answer, so the
  name sets the id's default and the perspectives what the catalog
  asks; a skipped question prints one line, its answer and why.
  `prompts::pick` asks any list question: only `ready` answers are
  choices, so a planned one can't be picked. Esc stops with nothing
  made.
- `questions.rs`: the line prompts of 2592 and 2597 are gone. The
  examples question is the template's, asked in the wizard; the GitHub
  questions are `wizard::Question`s (`github_question`, `visibility_question`,
  `keep_question`, `kinds_question`, `folders_question`,
  `assets_visibility_question`), asked through the `Prompter` after the
  wizard's, and listed by `--questions` (`github_questions`) after the
  targets, those after `github` skipped unless `--github`. Every flag
  stays, and `--no-input` still means `--no-github`.
- `wizard::Question` gains `agent`: `github`'s says an agent asks its
  user and never answers yes itself. `new.questions.json` regenerated;
  `contract::VERSION` 59 to 60.
- `dialoguer` 0.12.0: MIT (its `Cargo.toml` and `LICENSE`, checked
  2026-11-24), with `default-features = false` (no `editor`/`tempfile`,
  no `password`/`zeroize`); it brings `shell-words` (MIT or Apache-2.0)
  and uses `console` 0.16, already in the lock. No MSVC. The CLI isn't
  in a shipped game, so rule 13's toggle doesn't apply; but a new game's
  tools program depends on `shuyi-cli`, so its tree grows by 6 crates
  (`dialoguer`, `console`, `shell-words`, `encode_unicode`,
  `unicode-width`, a `windows-sys`), and `tools/compile-budget.ron`'s
  `dependencies` goes from 143 to 149. The ten tutorial steps' locks
  gain the same crates.
- `shuyi.ron`'s `check_keys` lets `prompts.rs` name the terminal's keys
  (its help lines) in place of `questions.rs`, which no longer does.
- Docs: `docs/templates.md` (what a terminal asks, the GitHub questions
  in the list and their `agent` field), the `shuyi-new-game` skill, the
  book's quick start and box-pusher chapter 1, and `new`'s usage. 2848's
  clause about 3423 and 2739's mention are rewritten as what now holds,
  and 3426 (MCP's `new` lists the questions) gets a note on the GitHub
  questions it will list.

Deviations:

- 2848 isn't done, so the license question was left to it (its clause
  now says where it goes and how).
- The folder isn't asked: it is `new`'s first word, always given. A
  name that makes no crate (a folder named `9`) is now asked again in a
  terminal rather than refused.
- The examples question moved from first to among the template's, after
  the scripting question, and its text is the template's, shorter than
  2597's line prompt.
- The yes-or-no GitHub questions are lists of `yes` and `no`; the help
  line goes under each list but not under a text prompt, which is one
  line.
- The terminal run was checked by hand only up to the first list
  (`Which perspectives?` with the grid ticked and the parked answers
  greyed under it); answering it all is Nick's sign-off.

Size: 2,244 counted lines, the terminal prompts replace the line prompts
in one place: 420 of the lines are the old prompts and their tests
removed, and landing the lists without moving the examples and GitHub
questions would ask the examples twice and mix two prompt styles in one
run.

Behaviour: shuyi-cli: `shuyi new` in a terminal asks the wizard's
questions (the name, the id, the perspectives, the catalog's, the
template's, the targets) as lists before the GitHub ones, and draws its
prompts on stderr; it asks only when stdin, stdout and stderr are all a
terminal. The examples question comes after the perspectives, not first.

Behaviour: shuyi-cli: `shuyi new <dir> --questions` lists six GitHub
questions after the targets, and each question in its JSON has an
`agent` field (null but for `github`).

Book: updated, `quick-start.md` and `box-pusher/01-new.md` say what a
terminal asks.

Game: nothing to adopt, the game isn't made by `shuyi new` and never
runs it (searched the game for `shuyi new` and `questions --json`: only
its copies of the skill pack, which the pin's skill sync updates).

Nick input: sign-off pending: run `shuyi new` in a terminal, answer the
questions, and say whether the wording is clear and whether anything
expected is missing.

Nick's sign-off, in progress (2026-11-25): he found the id question
("What is its reverse-DNS id, which its saves are kept under?")
confusing and Java-looking, and asked whether it really names the save
folder. It does, by default (ADR-0059: the id is unique, so two games
of one name don't share saves). He agreed to the wording "A unique id
for the game, like io.github.yourname.<crate name>: app stores ask for
one, and the game's saves are kept under it.", which the question now
says.
