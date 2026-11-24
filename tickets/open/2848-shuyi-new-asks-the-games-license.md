---
id: "2848"
title: shuyi new asks for the game's license, and the scaffold is licensed so a game owes nothing for it
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: ["3529"]
nick_input: none
completed:
---

# 2848 — shuyi new asks for the game's license, and the scaffold is licensed so a game owes nothing for it

## Context

`shuyi new` writes a game's first files: its `Cargo.toml`, crates,
`AGENTS.md`, workflows, and the engine's skill pack under
`.claude/skills/`. Those files are the engine's code, and today they
carry the engine's license with no notice, and the game gets no license
of its own:

- `templates/shuyi-default/template.ron` line 9 says
  `license: "MIT OR Apache-2.0"`. `crates/cli/src/template.rs` line 75
  stores it as `Template::license` ("the license of the files it
  writes"), and nothing reads it: a grep for `license` in
  `crates/cli/src/contract/new.rs`, `contract/new/questions.rs` and
  `template/compose.rs` finds nothing.
- There is no license file under `templates/` or `skills/`. All 18
  shipped skills (`skills/*/SKILL.md`) say `license: MIT OR Apache-2.0`,
  as does `.claude-plugin/marketplace.json` line 13.
- The game's `Cargo.toml.tmpl` has no `license` under
  `[workspace.package]`, and `crates/game/Cargo.toml.tmpl` has none
  either. The game gets no `LICENSE` file, and no question about its
  license.
- By the decision of 2026-11-12 (`docs/decisions.md`, "`shuyi new` sets
  up a game's repositories"), `shuyi new --github` makes the game's
  repository **public by default** (2591, 2592, ADR-0112). Neither ticket
  nor the ADR mentions a license.

So every new public game repository starts as "all rights reserved"
code that holds MIT-licensed files with no copyright notice. That breaks
MIT's one condition, confuses anyone who forks it, and leaves a
commercial developer unsure what the starter code obliges. Other
engines' starters avoid this with a public-domain-style license (MIT-0,
0BSD or CC0-1.0) for the scaffold, so it becomes the developer's own.
`MIT-0`, `0BSD` and `CC0-1.0` are all in ADR-0002 §8's code list and in
`deny.toml`'s allow list.

The scaffold's license is a product decision: 3529 asks Nick (its
question on the starter code's license, proposed as `MIT-0 OR MIT OR
Apache-2.0` for the templates and the shipped skills). This ticket
carries out whatever he answers, and adds the half that needs no
decision: the game's own license.

Source: the engine blind-spot audit of 2026-11-23 (proposed ticket 96).

## Nick input

`None.` Nick's answer is 3529's, recorded in `docs/decisions.md` before
this starts. The scaffold's license (`MIT-0 OR MIT OR Apache-2.0`,
`CC0-1.0`, or unchanged) is Nick's answer to 3529's question 3, in
`docs/decisions.md`.

## Scope

**In:**
- **The game's license.** `NewInput` gains `license: Option<String>`
  (`#[serde(default)]`; `NewInput` is `#[non_exhaustive]`, so the field
  is additive, ADR-0039) and `copyright: Option<String>`. Flags:
  `--license <SPDX expression | none>` and `--copyright HOLDER`. In a
  terminal, `new` asks it as a plain prompt (`questions::license`),
  after the examples question and before the GitHub ones, with `none`
  as the default:
  "Which license for the game's own files? MIT, Apache-2.0,
  MIT OR Apache-2.0, MIT-0, another SPDX expression, or none (all
  rights reserved)". Off a terminal or with `--json`, it asks nothing,
  and no `--license` means `none`.
- **What each answer writes:**
  - an expression: `license = "<expression>"` under the game's
    `[workspace.package]`, and `license.workspace = true` in each crate's
    `Cargo.toml` the plan writes (after `publish.workspace = true`);
  - for each id of it whose text the CLI holds (`MIT`, `Apache-2.0`,
    `MIT-0`): its text at the game's root, `LICENSE-MIT`,
    `LICENSE-APACHE`, `LICENSE-MIT-0` (one id alone: `LICENSE`), MIT's
    and MIT-0's copyright line naming `--copyright`, else
    `the <name> authors`, with no year (no clock in the plan);
  - another id: no text, and a `limits` line saying to add its text;
  - `none`: no `license` field and no file. If the game's repository
    is to be public (the GitHub questions), `new` says once that a
    public repository with no license is all rights reserved: others
    may read it and not reuse it. It is a warning, never a refusal;
  - the game's `README.md` gets a `## License` section saying the
    game's license (or "none chosen yet") and the scaffold's (below).
- **An expression is checked** before anything is written: SPDX ids
  (letters, digits, `.`, `-`, `+`, or `LicenseRef-…`) joined by `OR`,
  `AND`, `WITH` and parentheses. A malformed one is refused with
  `new`'s refusal code and the expression.
- **The scaffold's license, as 3529 decided:**
  - **if a public-domain-style license** (the proposal:
    `MIT-0 OR MIT OR Apache-2.0`, or `CC0-1.0`, `0BSD`): set it in
    `template.ron`'s `license`, `docs/templates.md`'s example, every
    shipped skill's `license:` line and `marketplace.json`. Add
    `templates/LICENSE.md` and `skills/LICENSE.md`, each naming the
    expression and holding the MIT-0 (or CC0, 0BSD) text. The game's
    README section says the files `shuyi new` wrote are the game's own,
    owing nothing;
  - **if Nick keeps `MIT OR Apache-2.0`:** the template and skills keep
    it. `new` writes the engine's two texts as `STARTER-LICENSE-MIT` and
    `STARTER-LICENSE-APACHE` at the game's root, and the README section
    says the starter files are under them and the game keeps those two
    files.
  - Either way the composer reads `Template::license`: a new
    placeholder, `{{template.license}}`, fills the README section, and
    `NewOutput` gains `template_license: String` (additive:
    `NewOutput` is in the contract's answer, so regenerate its schema
    and raise `contract::VERSION`).
- **From 2502 (done)** (the new-game wizard): 2502 made the questions'
  flags, but wrote no terminal prompts and didn't move the examples and
  GitHub questions; 3424 lists the questions (`--questions --json`),
  and 3423 writes the prompts and moves them. Until
  3423 lands, ask the license as a plain prompt in
  `crates/cli/src/contract/new/questions.rs` beside the GitHub
  questions; keep `--license` and `--copyright`, and `--no-input`
  means `none`.
- **Either order with 3423** (the wizard's prompts): if 3423 is done,
  ask the license in the wizard after the GitHub questions, in its
  prompt style, and list it in `--questions --json` with a field saying
  an agent asks its user and never picks a license itself. If not,
  nothing to do (3423 moves it, its clause says so).
- What changes for the game (Visions of Shuyi): nothing. It isn't made
  by `shuyi new` again, and no public item it uses breaks. `NewInput`
  and `NewOutput` gain fields; the completion notes have a `Behaviour:`
  line for `shuyi new`'s new question and its README section.
- 2512's line counter already leaves out files whose name holds
  `LICENSE`, so the new files don't count against the template's cap.

**Out (do not do):**
- choosing the scaffold's license: Nick does, in 3529;
- the game's third-party credits and license texts in `NOTICES.txt`
  (2223); the game's own license is not a third-party notice;
- a check in `shuyi check` that a public game has a license: the
  warning at `new` is enough for now;
- templates from outside the engine and their license warning (2505);
- the engine repository's own licensing, contributor policy and
  community files (2847, 2849);
- changing the root `README.md`'s `## License` section beyond one line
  naming the templates' and skills' license, if 3529 changed it.

**Estimated size:** about 600 lines (code 250, tests 250, docs and data
100).

## Implementation steps

1. Read 3529's answer in `docs/decisions.md`. Write the scaffold's
   license into `templates/shuyi-default/template.ron`,
   `docs/templates.md` (the example and the `license` row: "the license
   of the files it writes, which `new` names in the game's README"),
   `skills/*/SKILL.md`'s `license:` lines and
   `.claude-plugin/marketplace.json`, and add `templates/LICENSE.md` and
   `skills/LICENSE.md` (or, if he kept `MIT OR Apache-2.0`, the two
   `STARTER-LICENSE-*` files the plan writes). Raise each skill's
   `version` if `cargo xtask check-skills` asks for it.
2. `crates/cli/src/template.rs` and `template/compose.rs`: add
   `template_license` to `Fill` and the `{{template.license}}`
   placeholder; add it to `new.rs`'s placeholder table and
   `docs/templates.md`'s.
3. `crates/cli/src/contract/new.rs`: `NewInput::license` and
   `copyright`, their flags in `parse_args` and `usage`, the expression
   check, and in `plan` the `license` lines in the planned `Cargo.toml`
   files, the license texts (reuse `notices.rs`'s `ENGINE_MIT`,
   `ENGINE_APACHE` and `mit_text`; add an embedded MIT-0 text from
   <https://spdx.org/licenses/MIT-0.html>) and the README section
   (`templates/shuyi-default/base/README.md` gets the section with
   placeholders `{{license.game}}` and `{{template.license}}`).
4. `crates/cli/src/contract/new/questions.rs` (or the wizard, per the
   3423 clause): `questions::license` with its parse, and the warning
   for a public repository with no license.
5. `NewOutput::template_license`; regenerate the contract's schema;
   raise `contract::VERSION`.
6. Update `template/plans.golden` and the `new` snapshots; the
   `shuyi-new-game` skill's text names `--license`.

## Acceptance criteria

- [ ] `shuyi new demo --no-input --license "MIT OR Apache-2.0"
      --copyright "Ada"` writes `LICENSE-MIT` (its copyright line
      `Copyright (c) Ada`) and `LICENSE-APACHE`, `license = "MIT OR
      Apache-2.0"` under `[workspace.package]`, and
      `license.workspace = true` in each crate's manifest; `cargo
      metadata` on the game reports that license for each crate.
- [ ] `--license MIT-0` writes one `LICENSE` holding the MIT-0 text.
- [ ] `--license "LicenseRef-Proprietary"` writes the field, no text,
      and a `limits` line saying to add it.
- [ ] `--license "MIT OR"` is refused before any file is written.
- [ ] With no `--license` (and with `--json`) the game has no `license`
      field and no license file, and its README says no license is
      chosen yet.
- [ ] Answering public to the visibility question with license `none`
      prints the all-rights-reserved warning once, and goes on.
- [ ] The game's README names the scaffold's license as
      `template.ron` says it; changing `template.ron`'s `license`
      changes that line (a test over a fixture template).
- [ ] `template.ron`, the skills' `license:` lines and
      `marketplace.json` say the license 3529 chose, and the files it
      needs exist.
- [ ] The 3423 clause is settled as it says.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `parse_args` reads `--license` and `--copyright`; the expression
  check accepts `MIT`, `(MIT OR Apache-2.0) AND Unicode-3.0`,
  `LicenseRef-Proprietary`, `none`, and refuses `MIT OR`, `OR MIT`,
  `MIT AND (` and an empty string; `questions::license` over a scripted
  `Terminal` (Enter gives `none`, a bad line asks again).
- Unit: the plan for each case above (files, the `Cargo.toml` lines,
  the README section, the MIT copyright line), with no I/O.
- Snapshot / integration: `plans.golden` with a license and without;
  the existing test that a new game builds passes with
  `--license "MIT OR Apache-2.0"`; the contract schema test.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
