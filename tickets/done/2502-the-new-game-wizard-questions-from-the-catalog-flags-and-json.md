---
id: "2502"
title: "The new-game wizard: shuyi new asks one question per concern from the catalog, with multi-pick, a flag for every answer, and the questions as JSON for agents"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2501"]
nick_input: none
completed: 2026-11-24
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
perspectives to be picked, but until 2503 lands a game has one stage.

**From 2501 (done):**
- `templates/shuyi-default/template.ron` asks `perspective` as a
  question of its own, with the one ready answer, `top-down-grid`, so
  that `--set perspective=top-down-grid` and the short name `grid` work.
  This ticket asks perspective from the catalog instead, and takes the
  question out of `template.ron`; the short names and the parts'
  `Picked("perspective", …)` keep their ids.
- The composer refuses a second part with a stage
  (`ComposeError::Stage`, "a game has one stage for now"): with two
  perspectives that each add a stage picked before 2503, the wizard
  says so before it composes, and `limits` names 2503's moves as not
  there yet.
- `compose(&Template, &Answers, &Catalog, &Fill)`: the answers go in as
  `template::Answers`; `contract::new::answers_of` joins a short name,
  `--answers` and `--set`, and this ticket's flags join there.

## Nick input

None. Nick's sign-off moved to 3423, with the terminal's prompts he
signs off on.

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
  its schema and raise `contract::VERSION`. **From 2086 (done):**
  `Concern` is `#[non_exhaustive]`, so a new field, `#[serde(default)]`,
  is no break.
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
- **`wizard::questions`**, the questions with their answers' status:
  split off into 3424 at the second size valve (below).
- **Interactive prompts** in a terminal, and the keys they use: split
  off into 3423 at the size valve (below).
- **Flags** for every question: `--id` (see below),
  `--perspective`, `--bodies`, `--music` (if 2205 is done), `--kind`
  (from 2003's *Kind of game* concern, ADR-0015 §1), `--online`,
  `--targets`, and `--set` (2501) for template questions. A planned
  answer given by flag fails with its status and ticket. `--no-input`,
  or stdin not being a terminal, asks nothing and uses the defaults.
- **`--questions --json`, the printed command and `created_with`:**
  3424's (below).
- **The game's id. From 3048 (done):** the template's base part has
  `app: (name: "{{name}}", id: "com.example.{{crate_name}}", code:
  "crates/game/src/app_info.rs")` in `shuyi.ron`, and
  `crates/game/src/app_info.rs` holds the same id in its string (the
  file `cargo shuyi app-info` writes, its `const` kept from rustfmt).
  The wizard's id goes into both: a new placeholder, `{{id}}`, in place
  of `com.example.{{crate_name}}` in the two files (`docs/templates.md`
  and `new.rs`'s table), whose value is `com.example.<crate>` without
  `--id`. A new game then passes `cargo shuyi app-info --check`.
- **Each picked answer that is a module** turns it on through the
  template's parts or, if no part names it, through `shuyi add`'s code
  directly (ADR-0013: every catalog answer works with no part).
- **The skill pack:** its `shuyi-new-game` skill calls `--questions
  --json`, so it is 3424's.
- **Docs:** `docs/templates.md` (2501) gets a section on the wizard and
  its flags. **From 2004 (done):** each ready direction guide's *What
  starts it* (`docs/guides/`, today `top-down-grid.md` alone) names the
  wizard's answers and the `shuyi new … --no-input` command, in place of
  `--template grid` alone.
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
- **The `time` question of 2739's part** in `--questions --json`:
  3424's.
- **Either order with 2771** (parked: the `stage-hex-grid` part): if
  2771 is done, `--template hex` and `--set perspective=hex-grid` make
  the game its part makes. Check that `--perspective hex-grid
  --no-input` makes the same plan (a test), and name that command
  beside `--template hex` in `docs/guides/hex-grid.md`'s *what starts
  it*. If not, nothing to do: 2771 names the command and adds the test
  when it lands (its clause says so).
- **The GitHub, license and examples prompts (2592, 2848, 2597):**
  moving them into the wizard's prompt style is 3423's, with the
  prompts.
- **Either order with 3338** (each target's support level): if 3338 is
  done, the targets question offers a `Preview` profile like a
  supported one, with "preview: not yet run on its device" beside it,
  and never offers a `Planned` one; if not, nothing to do (3338 adds
  that to this question when it lands).

**Estimated size:** about 2,000 lines (code 900, tests 800, docs 300).

**Split:** past 1,000 lines, split off the interactive prompts and the
move of 2592's and 2597's prompts into them: done, as 3423. At 2,052
counted lines with the rest, the questions as data, `--questions
--json`, the printed command, `created_with` and the skill were split
off too, as 3424.

**Out (do not do):**
- the terminal's prompts (3423);
- the questions as data, `--questions --json`, the printed command and
  `created_with` (3424);
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
   Add the id question and `--id`.
4. Print the equivalent command and write `created_with`.
5. Update the skill and the docs.

## Acceptance criteria

- [x] `shuyi new demo --no-input` makes the same game as `shuyi new
      demo --template grid`.
- [x] ~~`shuyi new demo --questions --json` lists every catalog concern
      with each answer's status, and matches its schema.~~ (not done:
      split off to 3424, whose acceptance has it)
- [x] A parked answer by flag (for example `--bodies procedural`)
      fails, naming its status and ticket 2701.
- [x] ~~Running the printed command with `--no-input` into another
      folder gives the same bytes.~~ (not done: split off to 3424, whose
      acceptance has it)
- [x] With every ready answer picked, the new game builds and passes its
      tests in CI.
- [x] `shuyi new demo --no-input --id org.example.demo` writes that id
      into `shuyi.ron`'s `app.id` and `app_info.rs`, and the game passes
      `app-info --check`. A bad id by flag fails with `check_id`'s
      error.
- [x] `check-modules` fails on a concern other than `time` without
      `pick` or `question`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `wizard::questions` over fixture catalogs (a concern skipped for
  grid-only, `time` never asked, planned answers marked, template
  questions placed last before targets); flag parsing to `Answers`.
- Integration: `--questions --json` against its schema; `--no-input`;
  the printed command reproduces the game; the every-ready-answer game
  builds (CI).

## Completion notes

**What was done.** The catalog's concerns (`docs/modules.ron`) gained
`pick` and `question`, every one but `time`, and `cargo xtask
check-modules` fails a concern without them (its rule 6, tested by
`a_concern_but_time_needs_its_pick_and_question_and_serves_names_perspectives`
and over the repository's own file). `shuyi_cli::catalog::Concern` and
the xtask read them (RON's `implicit_some` now on for `modules.ron`).
The perspective is the catalog's question: the template's `perspective`
question is gone, its parts' `Picked("perspective", …)` and the short
name `grid` keep their ids, a part's `Picked` may name any concern, and
a template question may not take a concern's id or `targets`. The
composer checks a concern's answers against the catalog (a planned or
parked one fails with its status and tickets), gives each concern its
first ready answer when none is given (so no answer at all is the grid
game, and `blank` is now `{ "perspective": [] }`), and turns on the
module a picked answer names when no part does.

`shuyi new` takes a flag per asked concern (`wizard::concern_flag`:
`--perspective`, `--bodies`, `--online`, `--world-drawing`, `--kind`,
`--scripting`), `--targets` (checked against the engine's profiles; a
planned one refused; the template's `desktop,web` by default; a target
picked beside them is a `limits` entry naming 2504), `--id` (checked by
`app_info::check_id`, filling a new `{{id}}` placeholder in the base's
`shuyi.ron` and `app_info.rs`; `NewOutput::id`) and `--no-input`. CI's
grid game picks every ready answer by flag. `docs/templates.md` has *The
wizard's questions*; the book's quick start, `docs/contract.md` and
`docs/modules.md` say what changed. Contract version 56 (`Concern`'s
fields in `modules`, `NewInput::id`, `NewOutput::id`).

**Deviations.**
- The ticket was 37 days old. Checked against the code, before the
  code: the catalog has seven concerns, not six (`scripting` came since,
  and got `pick: One`, "How are the rules written?"); the perspective
  concern already had `hex-grid` and `first-person-grid` beside those
  the ticket names, with their statuses; 3108 and 3109 were added to
  `first-person`, `third-person` and `top-down-3d` as asked.
- **Split twice at the size valve.** First, before the code: the
  terminal's prompts, the keys they name, moving 2592's GitHub and
  2597's examples prompts into them, 2848's license clause, and Nick's
  sign-off, as 3423. Then, with everything written and passing, at
  2,052 counted lines: `wizard::questions` and its types, `--questions
  [--json]` and its schema, the printed command, `created_with` and the
  skill, as 3424, which 3423 now waits for. The two halves land apart:
  this one makes every answer work by flag; 3424 lists them and records
  them. That code exists, tested, on the local branch `t3424-draft`
  (this ticket's session), for 3424 to start from.
- A third, optional field per concern, `serves` (the perspectives its
  answers are for), for 3424's skipping rule: no field held which
  perspectives an answer applies to for answers by `default` or by a
  module not built. It is on the concern, not the answer, because
  `catalog::Choice` isn't `#[non_exhaustive]` and a field there would be
  a break. Only `bodies` has one; `check-modules` checks it names
  perspectives.
- The answers' one-line description is the catalog answer's `name`: no
  new field.
- The direction menu: 3108 isn't done, so there was none to replace.
- Either-order clauses: 2004, 2205, 2206, 2707, 2719, 2726, 2732, 2735,
  2739, 2754, 2771, 2848, 3108, 3109 and 3338 aren't done, so nothing was
  done for them here; each one's clause now says what holds, and those
  about the questions' list name 3424 (2504, 2551 and 2775 wait for it
  too).

**Follow-up tickets.** 3423 (the terminal's prompts, and Nick's
sign-off) and 3424 (the questions as data, `--questions --json`, the
printed command and `created_with`).

Breaking: shuyi-cli: `template::Template::check_answers`: takes the catalog's concerns, `check_answers(&answers, &catalog.concerns)`.

```migration
since: 0.1.0
what: Template::check_answers checks the catalog's concerns too, and takes them.
old: Template::check_answers(&self, &Answers)
new: Template::check_answers(&self, &Answers, &Concerns)
before: |
  template.check_answers(&answers)?;
after: |
  template.check_answers(&answers, &catalog.concerns)?;
find: ["check_answers(&answers)"]
```

Breaking: shuyi-cli: `contract::new::NewOutput`: gains `id`, so a literal of it names it.

```migration
since: 0.1.0
what: NewOutput, new's answer, gains the game's reverse-DNS id.
old: NewOutput { dir, name, crate_name, template, answers, parts, files, overwritten, modules, targets, stages, skills }
new: NewOutput { dir, name, crate_name, id, template, answers, parts, files, overwritten, modules, targets, stages, skills }
before: |
  let out = NewOutput { dir, name, crate_name, template, answers, parts, files, overwritten, modules, targets, stages, skills };
after: |
  let out = NewOutput { dir, name, crate_name, id, template, answers, parts, files, overwritten, modules, targets, stages, skills };
```

Behaviour: shuyi-cli: `shuyi new <dir>` with no template and no answers makes the grid game (the perspective's first ready answer, `top-down-grid`), where it made the blank one; `--template blank` is unchanged.
Behaviour: shuyi-cli: `shuyi new` refuses a planned or parked answer of a concern (`--set perspective=side-view`: "`side-view` (side view), an answer of `perspective`, is parked: ticket 1127 builds it, so no game picks it yet"), where `side-view` was "no answer of `perspective`"; an unknown question's error lists the engine's questions too.
Behaviour: shuyi-cli: `shuyi new --engine-path <checkout>` reads the concerns from that checkout: one whose `docs/modules.ron` predates this change (no `perspective` concern with a `pick`) refuses `--template grid` and `blank` ("no question `perspective`"); move the checkout forward.
Behaviour: shuyi-cli: a new game's answers (`NewOutput::answers`) hold each concern's answer and `targets`, and its `app.id` and `app_info.rs` come from `{{id}}`, `com.example.<crate>` without `--id`, as before.

The game builds no `NewOutput` and calls no `check_answers`
(`gh search code --repo Zafnok/visions-of-shuyi 'check_answers'`: no
hit; `'NewOutput'`: only `docs/engine/pins.md`, saying the game builds
none).

Book: the quick start's *Make a game* names the flags and `--id`.

Nick input: none here; his sign-off on the prompts is 3423's.
