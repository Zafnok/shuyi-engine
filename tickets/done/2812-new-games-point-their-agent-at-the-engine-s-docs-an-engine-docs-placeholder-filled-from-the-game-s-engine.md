---
id: "2812"
title: "New games point their agent at the engine's docs: an {{engine.docs}} placeholder filled from the game's Engine"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: ["2808"]
nick_input: none
completed: 2026-11-01
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

- [x] `shuyi new` with `--engine-rev`, with the default and with
      `--engine-path` writes an `AGENTS.md` whose engine links name
      that commit, `main` and the site, or the checkout.
- [x] No file of a new game holds `{{` but a GitHub Actions
      expression (`${{ … }}`), for each `Engine` variant and each short
      name of the template. (Of the files the composer fills: see the
      notes.)
- [x] The book's chapter is updated (2802), or the completion notes say
      why not: *Tools for agents* (`site/content/learn/book/tools-for-agents.md`)
      says what a new game's `AGENTS.md` links, if it says what the file
      holds.
- [x] All gates in the `run-gates` skill pass. (Those that run locally:
      see the notes.)

## Tests required

- Unit (`compose.rs`'s tests): `fill` replaces `{{engine.docs}}`.
- Unit (`new.rs`): the list for each `Engine` variant; the template
  filled with each variant, with each short name, has no `{{` left but
  `${{`.

## Completion notes

A new game's `AGENTS.md` now lists the rules and docs of the engine the
game builds with, from `{{engine.docs}}`: `Fill::engine_docs`, set by
`fill_for` through a new `engine_docs` function, with `ENGINE_SITE` beside
`ENGINE_GIT`. The two tables of placeholders and *Tools for agents* say so,
and `plans.golden` moved for `AGENTS.md` only.

Deviation: the test that no `{{` is left reads only the files the composer
fills. It skips `.claude/skills/`, copied as they are, whose Rust example
holds `format!`'s `{{`, and files that aren't UTF-8 (`atlas.png`). Neither
is a placeholder.

Gates: everything in `run-gates` that changes with this work ran locally
and passed, with the `templates` job's gates on `grid` and `blank` and every
skill's worked example. `shuyi-test-attach` is manual, as it needs a window.
Not run: the `no-std`, wasm and web builds and the golden tables on wasm32,
as no crate they build changed; and the `api` job, which CI runs.

No break: `Fill` is `#[non_exhaustive]`, and the field and `ENGINE_SITE` are
additions, which the book's chapter covers.

No follow-up tickets. The released version's list is 2005's and the domain
is 2807's, and both already say so.
