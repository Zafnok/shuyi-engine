---
id: "2390"
title: "One placeholder parser: fill and the placeholder lists built on split_tokens"
type: infra
milestone: After MVP
model: sonnet-5
effort: low
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2390 — One placeholder parser

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-27). Text names what is filled in when it is shown between
braces (`Press {Confirm}`, `{n:hero}`, `Hello, {name}.`), and three
places find those braces, each with its own loop:

- **`shuyi_core::text::split_tokens`** (`crates/core/src/text.rs`): the
  shared one. It cuts text into `Part::Text`, `Part::Token` and
  `Part::Unclosed`. The dialogue checks (`crates/assets/src/dialogue/check.rs`)
  and the names table (`Names::substitute`, `crates/assets/src/names.rs`)
  use it, and so does the game (its `lead` re-exports it).
- **`shuyi_ui::text::fill`** (`crates/ui/src/text.rs`): fills each
  `{name}` from the arguments, else a lookup, else writes it back, with a
  `split_once('{')` loop of its own. `fill_values`, `Ctx::text_with` and
  `Ctx::label` go through it.
- **`shuyi_assets::tip::placeholders`** (`crates/assets/src/tip.rs`):
  lists the names between braces, with another `split_once` loop. The
  tips check uses it, and so does the language packs' check that a
  translation has its source's placeholders (`crates/assets/src/lang.rs`,
  which borrows it from the tips module). The game re-exports it
  (`content::tip::placeholders`).

The three agree today on every input (an unclosed `{` and what follows
it is plain text; `{}` is a token named nothing; `{a{b}` is the token
`a{b`). Nothing keeps them agreeing, and 2348 adds a fourth parser
(ICU messages, `Message::walk`), whose fallback for text that doesn't
parse fills "simple placeholders only", as `fill` does.

## Nick input

None.

## Scope

**In:**
- **`shuyi_core::text::tokens(text) -> impl Iterator<Item = &str>`**:
  the names between braces, in order (the `Part::Token`s of
  `split_tokens`). A doc example.
- **`fill`** is built on `split_tokens`: a `Text` part is copied, a
  `Token` is filled (arguments, then the lookup, else written back as
  `{name}`), an `Unclosed` part is copied as it is. Its signature, its
  docs' promises and its results don't change.
- **`tip::placeholders`** is `tokens(text).collect()`. It stays public
  with the same signature, since the game re-exports it.
- **`lang`** calls `shuyi_core::text::tokens` instead of the tips
  module's function.
- **Either order with 2348:** if 2348 is done, the text that still finds
  braces by hand is `fill`'s fallback for a text that doesn't parse as a
  message, and whatever is left of `tip::placeholders`: move those onto
  `split_tokens` and `tokens`, and leave `Message`'s own parser alone
  (it reads plural and select, which `split_tokens` can't); if not,
  nothing to do (2348 builds its fallback on `split_tokens`).
- **For the game:** no API change. The game's `lead::split_tokens` and
  `content::tip::placeholders` re-exports keep working.

**Out (do not do):**
- The source scanners in `check-keys` and `check-text`
  (`crates/cli/src/check_keys.rs`, `check_text.rs`): they count braces
  in Rust source, which is another job.
- `Names::substitute`'s `{n:id}` rules, which already use
  `split_tokens`.
- Changing what any of these accept.

## Implementation steps

1. Add `tokens` to `crates/core/src/text.rs` with its tests, and
   re-export it from `crates/core/src/lib.rs` beside `split_tokens`.
2. Rewrite `fill`'s loop in `crates/ui/src/text.rs` over `split_tokens`.
3. Make `tip::placeholders` call `tokens`; point `lang.rs` at `tokens`.
4. Settle the 2348 clause above with what is done.
5. Run the gates.

## Acceptance criteria

- [ ] `grep -rn "split_once('{')" crates` finds only
      `crates/core/src/text.rs`.
- [ ] `fill`, `fill_values` and `tip::placeholders` give the same
      results as before on every existing test, and on the property test
      below.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`shuyi-core`): `tokens` of plain text, of `{a}{b}`, of `{}`, of
  an unclosed `{`, of `{a{b}`.
- Property (`shuyi-ui`): for any text, `fill` with no arguments and a
  lookup that knows nothing gives the text back unchanged; with a lookup
  that answers every name, the result has no `{name}` left for any name
  `tokens` gave.
- The existing tests of `fill`, `tip` and `lang` pass unchanged.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
