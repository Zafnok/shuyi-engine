---
id: "3430"
title: "shuyi new asks the save folder's name after the id, prefilled with the id, and --storage answers it"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: sign-off
completed:
---

# 3430 — shuyi new asks the save folder's name after the id

## Context

Nick, signing off 3423 (the wizard's prompts in a terminal) on
2026-11-25 (`docs/decisions.md`, *shuyi new's prompts show only what can
be answered*): the id question shouldn't say the saves are kept under
the id. Since a game can name its save folder itself, that is a question
of its own:

> "since we said they can modify the save folder locaiton manually, it
> should be a followup question. Rather than say here the saves will be
> under com.example.doom it should be just the unique id in this
> question. Next question should be what folder name are game's saves
> under and pre-filled with the id used from prev answer. Then they can
> overwrite that."

What holds today:

- `shuyi.ron`'s `app` has `storage`, the name the game's saves are kept
  under, the id when unset
  ([ADR-0059](../../docs/adr/0059-a-games-identity-once-in-shuyi-ron-written-as-a-rust-constant-checked-like-generated-files-and-the-storage-name-following-the-id.md)
  §1, §4). `crate::app_info::check_storage` checks it (one folder name:
  ASCII letters, digits, `.`, `-`, `_` and spaces, no `.` or space
  first or last), and `app_info::rust` writes `app_info.rs` with
  `.with_storage(…)` when it is set.
- The template's `base/shuyi.ron` has no `storage` line, and its
  `base/crates/game/src/app_info.rs` builds `AppInfo::new("{{name}}",
  "{{id}}", …)` with no storage.
- **From 3423 (done):** `wizard::questions` lists the folder, the name
  and the id (`text_question`), and `prompts::ask`
  (`crates/cli/src/contract/new/prompts.rs`) asks the id through
  `Prompter::text`, checked by `app_info::check_id`, then lists the
  questions again so the next one sees the answer. The id question now
  reads "A unique id for the game, like io.github.yourname.<crate name>:
  app stores ask for one." `wizard::command` prints the command that
  makes the game again; `--questions --json` lists every question
  (`new.questions.json`).

## Nick input

**Sign-off:** run `shuyi new` in a terminal: after the id, a question
asks the save folder's name with the id already typed in. Say whether
its wording is clear.

## Scope

**In:**
- **The question** `storage`, the engine's, right after `id` in
  `wizard::questions`: a text question, default the id as answered so
  far (so the prompt shows the id prefilled and Enter keeps it), flag
  `--storage`. Its text says what it is in plain words, for example
  "Which folder are the game's saves kept in, on the player's
  computer? The id is a good name: it is unique." In the terminal it is
  checked by `app_info::check_storage` as `prompts::ask` checks the id.
- **The flag and the input:** `--storage <name>` (`parse_args`, `usage`),
  and `NewInput::storage: Option<String>` (`#[serde(default)]`;
  `NewInput` is `#[non_exhaustive]`, so the field is additive,
  ADR-0039). Checked in `plan_with` by `check_storage`, refused as
  `--storage: …`. A storage equal to the id is the same as none.
- **The game:** with a storage other than the id, the game's
  `shuyi.ron` `app` gains `storage: "<name>"` and its `app_info.rs`
  is what `app_info::rust` writes for that `app` (so `cargo shuyi
  app-info --check` passes on the new game). Without one, both stay as
  today, byte for byte.
- **The command again:** `wizard::command` adds `--storage <name>` after
  `--id` when one is given, so the printed command makes the same game.
- **The listing:** `--questions --json` lists `storage` after `id`;
  regenerate `new.input.json` and `new.questions.json` and raise
  `contract::VERSION` by one (with `schemas/versions.json`, and
  `docs/contract.md`'s example answer).
- **Docs:** `docs/templates.md`'s wizard section (the engine's
  questions), the `shuyi-new-game` skill if it lists them, and the
  book's quick start where it names what a terminal asks.

**Out (do not do):**
- moving the save folder of a game that exists, or a migration of its
  saves (a game sets `storage` itself, ADR-0059 §4);
- the license question (2848);
- MCP's `new` listing the questions (3426): it gets `storage` through
  `questions::listed` with the rest.

**Estimated size:** about 800 lines (code 300, tests 400, docs 100).

## Implementation steps

1. `NewInput::storage`, `--storage` in `parse_args` and `usage`, and
   `--storage` among `questions.rs`' `TAKES_A_VALUE`.
2. `wizard::STORAGE` and its question after `ID` in
   `wizard::questions`; `Named` gains the storage so far (its default
   the id); `prompts::ask` asks it with `check_storage`, and sets
   `input.storage` only when the answer differs from the id.
3. In `plan_with`, check it; when set, merge `storage` into the planned
   `shuyi.ron`'s `app` (as `compose.rs`' `keep_created_with` merges with
   `ron_merge`) and replace the planned `app_info.rs` with
   `app_info::rust` of the merged `app`.
4. `wizard::command`: `--storage` after `--id`.
5. Schemas, `contract::VERSION`, `versions.json`, `docs/contract.md`.
6. Docs as in the Scope.

## Acceptance criteria

- [ ] In a terminal, after the id, `shuyi new demo` asks the save
      folder's name with the id prefilled; Enter keeps it and the game
      is as without the question, byte for byte.
- [ ] `--storage "Demo Saves"` makes a game whose `shuyi.ron` has
      `storage: "Demo Saves"` and whose `cargo shuyi app-info --check`
      passes; the printed command makes the same bytes in another
      folder.
- [ ] `--storage ".bad"` is refused with `check_storage`'s reason, and
      in a terminal such an answer is asked again.
- [ ] `--questions --json` lists `storage` right after `id`, and
      matches its schema.
- [ ] Nick's sign-off is recorded in the Completion notes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the question's place and default (the id, and the id typed in
  the terminal before it), through the scripted `Prompter`; `--storage`
  parsed and refused; the command with and without it.
- Integration: a game made with `--storage` against the game its
  printed command makes; `app-info --check` on it; the questions JSON
  against its schema.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
