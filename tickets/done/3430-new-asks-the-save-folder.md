---
id: "3430"
title: "shuyi new asks the save folder's name after the id, prefilled with the id, and --storage answers it"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: sign-off
completed: 2026-11-25
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

- [x] In a terminal, after the id, `shuyi new demo` asks the save
      folder's name with the id prefilled; Enter keeps it and the game
      is as without the question, byte for byte.
- [x] `--storage "Demo Saves"` makes a game whose `shuyi.ron` has
      `storage: "Demo Saves"` and whose `cargo shuyi app-info --check`
      passes; the printed command makes the same bytes in another
      folder.
- [x] `--storage ".bad"` is refused with `check_storage`'s reason, and
      in a terminal such an answer is asked again.
- [x] `--questions --json` lists `storage` right after `id`, and
      matches its schema.
- [x] Nick's sign-off is recorded in the Completion notes.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the question's place and default (the id, and the id typed in
  the terminal before it), through the scripted `Prompter`; `--storage`
  parsed and refused; the command with and without it.
- Integration: a game made with `--storage` against the game its
  printed command makes; `app-info --check` on it; the questions JSON
  against its schema.

## Completion notes

What was done:

- `NewInput::storage` (`#[serde(default)]`, additive: `NewInput` is
  `#[non_exhaustive]`), `--storage <name>` in `parse_args` and `usage`,
  and among `questions.rs`' `TAKES_A_VALUE`.
- `wizard::STORAGE`, a text question of the engine's right after `id`:
  "Which folder are the game's saves kept in, on the player's computer?
  The id is the default, since it is nearly sure to be unique.", flag
  `--storage`, its default
  the id so far. `Named` gains `storage` and `Named::new`, which defaults
  it to the id.
- `prompts::ask` asks it through `Prompter::text`, checked by
  `app_info::check_storage`, the id just answered typed in; it sets
  `input.storage` only when the answer isn't the id.
- `plan_with` checks `--storage` (refused as `--storage: <check_storage's
  reason>`, `SH0001`); a storage equal to the id is none. Otherwise
  `with_storage` merges `app: (storage: "<name>")`, with a comment, into
  the planned `shuyi.ron` (`ron_merge`, now `pub(crate)`), parses the
  merged `app` and writes `app_info.rs` as `app_info::rust` writes it, so
  `app-info --check` passes on the new game.
- `wizard::command` prints `--storage <name>` after `--id`, when given
  and not the id.
- `new.input.json` and `new.questions.json` regenerated;
  `contract::VERSION` 60 to 61, `versions.json`, `docs/contract.md`.
- Docs: `docs/templates.md` (the engine's questions, the command, the
  terminal's text prompts), the `shuyi-new-game` skill (the saves'
  folder follows the id), and the book's quick start (what a terminal
  asks, `--storage`, where the settings are kept).
- Tests: the question's place, default and flag (`wizard::tests`); the
  default is the id typed before it, Enter sets no `--storage`, `.bad`
  is asked again, a given `--storage` isn't asked (`prompts::tests`,
  through the scripted `Prompter`); the command with and without it;
  a game made with `--storage "Demo Saves"` against the game its printed
  command makes, `app-info --check` on it, a storage equal to the id
  making the same bytes as none, `.bad` refused (`new::tests`); the
  questions JSON lists `storage` after `id`, against its schema
  (`tests/it/binary.rs`).

Deviations:

- The question's words: the Scope's example said "The id is a good
  name: it is unique." Nick, at the sign-off, didn't want it to call
  the id a good name (it isn't friendly to a player), only to say it is
  the default because it is nearly sure to be unique; the question reads
  so.
- `wizard::questions` passed clippy's 100-line limit with the new
  question: the four text questions moved into `named_questions`.

The ticket was written today, so its names were current.

Handoff: 3426 (MCP's `new` lists the questions) gains a **From 3430
(done)** line: `NewInput::storage` reaches MCP as `id` does, and
`contract::VERSION` is 61 after this.

Game: nothing to adopt, the game calls no `NewInput` and makes no game
with `shuyi new` (searched `gh search code --repo Zafnok/visions-of-shuyi
'NewInput'`: only `docs/engine/pins.md` saying so).

Behaviour: shuyi-cli: `shuyi new` in a terminal asks one more question,
the saves' folder, after the id; `--questions --json` lists `storage`
after `id`; `NewInput` takes `storage`; contract version 61.

Book: the quick start names the new question and `--storage`
(`site/content/learn/book/quick-start.md`).

Guides: none, the change adds no direction, module or move.

Nick's sign-off (2026-11-25): shown the question after the id, with the
id typed in. He asked that it not suggest the id is a good name, since an
id isn't friendly to a player: only that the id is prefilled as the
default, because it is nearly sure to be unique. The wording was changed
to that ("The id is the default, since it is nearly sure to be unique.");
`docs/decisions.md` records it under the 2026-11-25 entry.
