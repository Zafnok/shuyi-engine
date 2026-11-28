---
id: "2289"
title: "A walker over a .dlg scene's steps with their source lines, and a character policy for its text (plain ASCII with suggested spellings)"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2289 — A walker over a scene's steps, and a text character policy

## Context

A request from the game.

1. **The game ticket:** game#1408, the game's `.dlg` checks on these
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1408-dlg-walker-and-ascii-policy-from-the-engine.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** visit every step of a parsed scene with
   the line it came from (inside replies and `@if` branches too), and
   have the parser hold a scene's text to a character set a game
   chooses, such as plain ASCII.
3. **The friction numbers:** the game's third engine-friction sweep
   (game#1397, 2026-11-27). About 210 production lines in
   `crates/content/src/` that the game would mostly drop (it keeps about
   70: what each check does at a step):
   - **The walker, 5 copies:** the same recursion (zip `steps` with
     `lines.steps`, take `blocks.next()`, recurse into each `Choice`
     option and each `If` branch), about 20 lines each, in
     `dialogue/deaths.rs` (108–137), `openings.rs` (51–80),
     `presence.rs` (293–330), `setting.rs` (210–232) and `stand_in.rs`
     (157–178), plus a mutable copy over the text without lines
     (`stand_in.rs` 73–94). The engine writes it by hand too
     (`dialogue/check.rs` around 427, `dialogue/present.rs` around 158)
     and keeps a private `visit_mut` (`dialogue/line_id.rs` 113).
   - **The character policy, about 90 lines:** the game's ADR-0045
     keeps `.dlg` text plain ASCII. Since 2350 the parser accepts any
     character but a control character, so the game's `parse_dlg`
     removes the parser's errors by their wording
     (`errors.retain(|e| !e.message.starts_with("control character U+"))`)
     and runs its own pass, whose table of suggested spellings copies
     `shuyi_assets::font::ascii_spelling`. Because of that it also has
     its own `scripts_from_sources` (about 15 lines) and an `AsciiDlg`
     format (about 30 lines) in place of `Dlg<D>`.
4. **The generic shape:**
   - **A walker:** `ParsedScene::walk(|at: StepAt| …)`, where `StepAt`
     holds the step, its 1-based line, its depth, and whether it is
     inside a reply or an `@if` branch; and `Scene::texts_mut(|text|
     …)` visiting every line of text a player reads, mutably. The
     engine's own three copies move onto it.
   - **A character policy:** a field of `DialogueLimits` (or an option
     of the `Dlg` format), `TextChars::Any` (today's rule: every
     character but a control character) or `TextChars::Ascii` (printable
     ASCII only; each other character an error at its column, with
     `ascii_spelling`'s suggestion when it has one). Comments are not
     checked. The same policy reaches `Dlg<D>` and the scripts loaders,
     so a game needs no format of its own. **From 2096 (done):**
     `DialogueLimits` is `#[non_exhaustive]`, built with
     `DialogueLimits::new(box_width, box_lines)` and its `with_<field>`
     methods, so the policy's field is additive (`TextChars::Any` in
     `new`, and a `const fn with_text_chars`).
5. **Examples from games that aren't tactics:** a visual novel with its
   own `@bgm` and `@cg` directives that checks each against its files
   at its line; a retro adventure whose bitmap font has only ASCII, so
   its writers' curly quotes are caught when they save, with the
   straight spelling suggested.

## Nick input

None.

## Scope

**In:**
- `walk` and `texts_mut` in `crates/assets/src/dialogue/` with docs and
  a doc example; `check.rs`, `present.rs` and `line_id.rs` on them.
- `TextChars` and its use in the parser, with `Any` the default, so
  no game's results change unless it chooses `Ascii`.
- New public types are `#[non_exhaustive]` (ADR-0039); the walker's
  `StepAt` may grow.
- The game consequence: its ADR-0045 rule is what `Ascii` gives, so the
  game drops its pass (game#1408). Nothing breaks for a game that keeps
  `Any`.

**Out (do not do):**
- Problem codes on `.dlg` errors: 2237. Whichever lands second gives
  the policy's errors their code.
- Any character set but these two (a game's own font coverage is
  2350's `shuyi check`).
- The game's own directives and checks (game#1408).

## Implementation steps

1. **Read** `crates/assets/src/dialogue/parse.rs` (`ParsedScene`,
   `Lines`, `PartLines`, `char_problem`), `dialogue/check.rs`,
   `present.rs`, `line_id.rs` (`visit_mut`), `font.rs`
   (`ascii_spelling`) and 2350 in `tickets/done/`.
2. `walk` and `texts_mut`, with tests on a scene with a reply inside an
   `@if` inside a reply.
3. Move the engine's three hand-written recursions onto them; their
   tests pass unchanged.
4. `TextChars` in `DialogueLimits`, the parser's check by it, the
   suggested spelling in the message.
5. Docs, and the `shuyi-dialogue` skill's line on characters.

## Acceptance criteria

- [ ] `walk` visits every step once, in file order, with its line, its
      depth and its reply and `@if` flags (unit test on a nested scene).
- [ ] `texts_mut` changes every line of text and reply label, and
      nothing else (unit test).
- [ ] With `TextChars::Ascii`, a line with `’` and `…` gives two
      errors at their columns, suggesting `'` and `...`; a comment with
      them gives none; with `Any`, results are as today.
- [ ] The engine's dialogue tests pass unchanged after the three copies
      move onto `walk`.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `walk`, `texts_mut`, both policies.
- Property: for any scene, `walk` visits as many steps as the scene has
  (counted by a plain recursion in the test).
- Snapshot / integration: none.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
