---
id: "2812"
title: "New games point their agent at the engine's docs: an {{engine.docs}} placeholder filled from the game's Engine"
type: feature
milestone: MVP
model: implementation
effort: medium
status: todo
blocked_by: ["2808"]
nick_input: none
completed:
---

# 2812 — New games point their agent at the engine's docs

## Context

2808 put the docs on the website in the shape agents read: `llms.txt`,
`llms-full.txt` and a Markdown twin of each Learn page, written by
`cargo xtask llms` (`crates/xtask/src/llms.rs`). Its last part, the
links a new game's `AGENTS.md` gives its agent, came out of its PR
because the PR passed the 600 changed lines of the `write-ticket`
skill. This is that part.

A new game's `AGENTS.md` comes from the engine's template, the base
part's `templates/shuyi-default/base/AGENTS.md` (2501, 2511). Its
second paragraph links the engine's `AGENTS.md` at `blob/main`, even
for a game that `shuyi new --engine-rev` pins to a commit
(`Engine::Rev` in `crates/cli/src/contract/new.rs`), and it names none
of the docs on the site. An agent working on a game should read the
docs of the engine the game builds with: the site's copy follows
`main`, the repository at the pinned commit has the book's Markdown,
and a checkout on the machine has both.

The composer fills placeholders in `shuyi_cli::template::fill`
(`crates/cli/src/template/compose.rs`), from `template::Fill`, which
`contract::new::fill_for` makes from the game's name and `Engine`.
`{{engine.<crate>}}` and `{{engine_ref}}` are filled that way already.
The placeholders are listed in two tables: the module doc of
`crates/cli/src/contract/new.rs` and `docs/templates.md`.

## Nick input

None.

## Scope

**In:**
- In `templates/shuyi-default/base/AGENTS.md`, the sentence of its
  second paragraph linking the engine's `AGENTS.md` at `blob/main`
  becomes a short list filled from one new placeholder,
  `{{engine.docs}}`.
- A new field of `template::Fill`, `engine_docs: String`: additive, as
  `Fill` is `#[non_exhaustive]` and built with `default()` (ADR-0039).
  `fill` replaces `{{engine.docs}}` with it. Today `{{engine.docs}}`
  falls through the `engine.<crate>` branch (no crate `docs`, and no
  quotes around it) and fails as an unknown placeholder; match it by
  name before that branch, or in the same `match` as `name`.
- `fill_for` sets it from the game's `Engine`, with a new
  `ENGINE_SITE` constant (`https://zafnok.github.io/shuyi-engine`) beside
  `ENGINE_GIT`:
  - `Main`: the engine's `AGENTS.md` at `blob/main`, and the site's
    `llms.txt` and `llms-full.txt` (`<ENGINE_SITE>/llms.txt`,
    `<ENGINE_SITE>/llms-full.txt`);
  - `Rev`: `AGENTS.md` at `blob/<rev>`, and the book's Markdown at
    `tree/<rev>/site/content/learn`, saying its code is in
    `site/snippets/` at the same commit;
  - `Path`: the checkout's `AGENTS.md` and `site/content/learn/`, as
    paths (the `root` `fill_for` gets).
  Each is a Markdown list, one link per line, ready to sit in the
  paragraph's place.
- The tables of placeholders gain the row: the module doc of
  `crates/cli/src/contract/new.rs` and `docs/templates.md`.

**Out (do not do):**
- the docs themselves (2808, done);
- a game on a released version: 2005 makes `Engine`'s default the
  released version and fills `{{engine.docs}}` for it with that
  version's `<ENGINE_SITE>/v<x.y>/llms-full.txt`;
- moving `ENGINE_SITE` to a custom domain (2807, parked).

## Implementation steps

1. Read `crates/cli/src/template/compose.rs` (`Fill`, `fill`) and
   `crates/cli/src/contract/new.rs` (`Engine`, `dependency`,
   `engine_ref`, `fill_for`), and the tests of each
   (`crates/cli/src/template/tests.rs`, `new.rs`'s).
2. Add `ENGINE_SITE`, the field, its filling in `fill`, and a function
   beside `dependency` that writes the list for an `Engine`.
3. Change the base part's `AGENTS.md`, and the two tables.
4. Run the gates.

## Acceptance criteria

- [ ] `shuyi new` with `--engine-rev`, with the default and with
      `--engine-path` writes an `AGENTS.md` whose engine links name
      that commit, `main` and the site, or the checkout.
- [ ] No file of a new game holds `{{` but a GitHub Actions
      expression (`${{ … }}`), for each `Engine` variant and each short
      name of the template.
- [ ] The book's chapter is updated (2802), or the completion notes say
      why not: *Tools for agents* (`site/content/learn/book/tools-for-agents.md`)
      says what a new game's `AGENTS.md` links, if it says what the file
      holds.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`compose.rs`'s tests): `fill` replaces `{{engine.docs}}`.
- Unit (`new.rs`): the list for each `Engine` variant; the template
  filled with each variant, with each short name, has no `{{` left but
  `${{`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
