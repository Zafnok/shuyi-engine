---
id: "2240"
title: "Name tokens filled in dialogue: a Named hooks for a game whose only tokens are names"
type: bug
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: low
status: done
blocked_by: []
nick_input: none
completed: 2026-10-28
---

# 2240 — Name tokens filled in dialogue: a `Named` hooks

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-28). A dialogue line names a character or a place from the
names table with a token, `{n:<id>}` or `{N:<id>}` (capital first
letter). The check and the player each decide what a token is, by hand,
and they don't agree about names:

- **The check accepts name tokens.** `check_scene` asks the game's
  `Tokens` (`crates/assets/src/dialogue/check.rs:25`). `NoTokens`
  (line 59) is documented as "the tokens of a game that fills in nothing
  but names". `token_problems` (line 263) passes a name token whose id
  is in the names table, and any valid id when `SceneRules::names` is
  `None` (`name_token_problem`, line 248).
- **The player fills nothing.** What a text box shows goes through
  `DialogueHooks::fill` (`crates/ui/src/dialogue/player.rs:28`; the
  player calls it for the text, the caption and each reply, lines 247
  and 261-262). The engine's only hooks, `Plain` (line 37), returns the
  text as written (`Cow::Borrowed(text)`, line 44).
- **Nothing in the engine fills names.** `Names::substitute`
  (`crates/assets/src/names.rs:120`) does it, but nothing outside its own
  tests calls it: `grep -rn "substitute(" crates/ui` finds nothing.

So a game that plays its scenes with `Plain` and checks them with
`NoTokens` passes `cargo shuyi check` with `Hello, {n:smith}.`, and the
player reads `{n:smith}` on screen. A game with its own hooks has to
know to call `Names::substitute` in its `fill`; nothing says so. Today
no `{n:` token is in `crates/ui`, `examples`, `templates` or `skills`,
so the toy (`crates/ui/src/toy.rs:463` and `812`) and the dialogue
skill's example show nothing wrong. The first game that writes one does.

The fix is on the player's side. The check can't know which hooks a game
plays with, and `SceneRules::names` being `None` also means "the table
failed to load", where reporting every name token would only repeat
that error. So this ticket adds the hooks a game with name tokens and no
tokens of its own uses, `Named`, and says in the docs which hooks goes
with which `Tokens`.

The tip rules have the same split (`TipRules::placeholder` in
`crates/assets/src/tip.rs:93` and `shuyi_ui::text::key_text` in
`crates/ui/src/text.rs:18` are separate callbacks a game writes), but no
bug comes of it: the game gives both. It stays as it is.

## Nick input

None.

## Scope

**In:**
- **`shuyi_ui::dialogue::Named`**, beside `Plain` in
  `crates/ui/src/dialogue/player.rs`: the hooks of a game whose only
  tokens are the names table's. No lines of its own
  (`Directive = NoDirective`, `Stage = ()`, `direct` as `Plain`'s), and
  `fill` is `Names::substitute`.
  - A struct with one private field, `names: Arc<Names>`, and a
    constructor `Named::new(names: Arc<Names>) -> Self`; it derives
    `Debug, Clone, PartialEq, Eq` (which `DialogueHooks` needs). It
    won't grow, so it isn't `#[non_exhaustive]` (ADR-0039).
  - Re-exported from `crates/ui/src/dialogue.rs` beside `Plain`.
- **The docs say which goes with which:**
  - `Plain`: "no lines, no tokens, not even names: a `{n:<id>}` shows as
    written. A game with name tokens plays with `Named`."
  - `NoTokens`: "Play its scenes with `Named` (or `Plain` if its text
    has no name tokens)."
  - `DialogueHooks::fill`: a game with tokens of its own and name tokens
    fills both, the names with `Names::substitute`.
  - `skills/shuyi-dialogue/SKILL.md`, step 4: one line for `Named`
    beside `Plain`, and that a game's own `fill` calls
    `Names::substitute`.
- **For the game:** no API change, only an addition (`Named`). The
  game's own hooks keep working; no `Breaking:` line (ADR-0038 §6).

**Out (do not do):**
- Changing what `check_scene` or `token_problems` accept, or what
  `NoTokens` returns.
- Changing `Plain`: it is a unit struct the game and the examples build
  as `Plain`, and giving it a table would break them.
- `Names::substitute` itself, and `shuyi_ui::text::fill` (the
  `{name}` placeholders of screen text, not of dialogue lines): 2390,
  2347 and 2348 change `fill`, and none of them touches the dialogue
  player's `fill`.
- The tip rules' `placeholder` and `key_text` callbacks (see Context).
- The toy and the dialogue skill's example keep `Plain`: their text has
  no name tokens.

## Implementation steps

1. In `crates/ui/src/dialogue/player.rs`, add `Named` after `Plain`,
   with `use shuyi_assets::names::Names;`. Its `fill` is
   `self.names.substitute(text)`. Doc comment with an example
   (`Named::new(Arc::new(names))`).
2. Re-export it in `crates/ui/src/dialogue.rs` (line 10's `pub use`).
3. Write the doc lines of the Scope on `Plain`, `NoTokens`
   (`crates/assets/src/dialogue/check.rs:59`) and `DialogueHooks::fill`.
4. Add the `Named` line to `skills/shuyi-dialogue/SKILL.md` step 4.
5. Write the tests below in `crates/ui/src/dialogue/player/tests.rs`.
6. Run the gates.

## Acceptance criteria

- [x] `grep -rn "substitute(" crates/ui/src` finds `Named`'s `fill` in
      `crates/ui/src/dialogue/player.rs`.
- [x] A scene with `{n:<id>}` and `{N:<id>}` in its text, its caption
      and a reply passes `check_scene` with `NoTokens` and a names table,
      and played with `Named` over that table shows no `{n:` or `{N:` in
      any `View` (the test below).
- [x] The docs of `Plain` and `NoTokens` name `Named`.
- [x] CI's `api` job lists `Named` as an addition and nothing as removed
      or changed.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-ui`, `player/tests.rs`):
  - `Named` over a table `{ "smith": "ada brook" }` shows
    `Hello, {n:smith}.` as `Hello, ada brook.` and `{N:smith} waves.`
    as `Ada brook waves.`; a caption and a reply with a name token are
    filled the same way.
  - An unknown id and a game token (`{n:nobody}`, `{hero}`) are shown as
    written, as `Names::substitute` leaves them.
  - `Plain` shows `{n:smith}` as written (what its doc now says).
- Integration (`shuyi-ui`, same file): parse a scene with name tokens
  in a speech line, the caption and a choice's replies, run
  `check_scene` with a test `SceneRules` (its `tokens` is `NoTokens`, its
  `names` the table) and assert no errors; then play it with `Named`,
  choosing the first reply at the choice, and assert that no `View`'s
  `text`, `caption` or `choices` holds `{n:` or `{N:` until it is
  finished.
- The existing player, dialogue and names tests pass unchanged.

## Completion notes

Added `shuyi_ui::dialogue::Named` (hooks whose `fill` is
`Names::substitute`), re-exported beside `Plain`. Docs name which hooks
goes with which tokens on `Plain`, `NoTokens`, `DialogueHooks::fill` and
the `shuyi-dialogue` skill (step 4). Tests: unit and a check-then-play
integration in `player/tests.rs`. No deviations, no follow-ups, nothing
for Nick.

No break: an addition only.
