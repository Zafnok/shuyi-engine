---
id: "2579"
title: "A lint that a screen's draw only paints its view, and its logic names no cell or color"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2579 — `lint` checks the screen split

## Context

A request from the game.

1. **The game ticket:** game#1225, which deletes the game's hand-kept
   test once this lands
   ([`1225-the-screen-skin-fence-by-the-engines-lint.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1225-the-screen-skin-fence-by-the-engines-lint.md)).
   It is not a blocker of this one.
2. **The job in one sentence:** fail the gates when a screen's `draw`
   does more than hand its view to a skin, or its logic names how things
   look.
3. **The friction numbers:** no friction review: found by the
   game/engine boundary audit of 2026-11-03. The game keeps the rule as
   a test over an `include_str!` list of 19 files (its
   `crates/ui/src/screens/title/tests.rs`, about 75 lines), and the list
   has already missed four screens added later. This engine keeps the
   same test over its own three screens (`crates/ui/src/screens/tests.rs`).
4. **The generic shape:** the screen contract (logic → view → skin,
   `AGENTS.md` rule 9, the `shuyi-screen` skill) checked by `shuyi lint`
   over a folder, finding screens by `impl … Screen<…> for`, not by a
   list. Two rules, one code (`SH2105`, `screen-skin`):
   - in an `impl Screen` block, `fn draw(&self, <ctx>, <buf>)`'s body is
     one call, `<any painter>(<ctx>, &self.view(<ctx>), <buf>);`, with
     the draw's own parameter names;
   - a logic file (one with an `impl Screen`, or listed in `logic`)
     names none of `names` as a whole identifier, and a listed logic file
     with no screen names no `GlyphBuffer`.
5. **An example from a game that isn't tactics:** a farming game's shop
   screen that started printing its prices in `draw` fails the lint
   before its one-bit skin is found to lack them.

## Nick input

None.

## Scope

**In:**
- `shuyi.ron`'s new part `screen_skin: (dir, skip, logic, names, hint)`
  (`project::ScreenSkin`): the screens' folder; files or folders (`/`)
  left out; logic files with no screen of their own; the names a logic
  file never says (empty means this engine's frame types: `Rgb`,
  `EngineColor`, `Rect`, `BoxStyle`, `Cell`; a game adds its own, such as
  a palette enum's name); what a finding says to do. Tests (`tests.rs`,
  `*_tests.rs`, `tests/`) are skipped. A project without the part gets a
  limit, as `check_text` does.
- `crates/cli/src/screen_skin.rs`: the scan, on `check_keys::lex` (no
  parser), returning findings with file and line.
- `lint`: `LintCheck::ScreenSkin` and `codes::SCREEN_SKIN` (`SH2105`),
  with `explain`'s text. `LintCheck` isn't `#[non_exhaustive]`, so the
  new variant is a `Breaking:` line if CI's `api` job reports one.
- This engine's own screens: a test in `crates/cli` runs the scan on
  `crates/ui/src/screens` with an in-memory `ScreenSkin`, and the old
  test in `crates/ui/src/screens/tests.rs` goes.
- `docs/contract.md` (the `lint` row and its `shuyi.ron` parts) and the
  `shuyi-screen` skill's step 6 name the lint.
- The game's PR that moves the pin follows this one (game#1225).

**Out (do not do):**
- Turning the part on in the template's `base/shuyi.ron` and Hello,
  Grid's: run the scan on them, and write a ticket for what they would
  need, in the completion notes.
- The other checks' findings as data (2543's job); this check returns
  its own findings directly.
- A parser (`syn`): the line scan is enough, as `check-keys` is.

## Implementation steps

1. Read `contract/lint.rs`, `codes.rs`, `check_keys.rs` (`lex`, `Line`),
   `check_text.rs` (its `skip` and test rules) and `project.rs`
   (`CheckText`); and the game's test (`the_logic_modules_name_no_cells_or_colours`,
   `names_word`) for the rules to keep.
2. `project::ScreenSkin` with `serde(deny_unknown_fields, default)`.
3. `screen_skin.rs`, tests first: find each `impl` line naming `Screen<`
   and ` for `; track braces to its `fn draw(`; read its two parameter
   names; accept the body only as the one call above; then the
   whole-identifier check over each logic file.
4. Wire it into `lint`, the code and `explain`; the engine's own test.
5. The docs above.
6. **Measure:** about 150 production lines and 150 test lines, judged by
   `check_text.rs`'s rules of the same kind.

## Acceptance criteria

- [ ] A screen whose draw is `glyph::paint(ctx, &self.view(ctx), buf);`,
      `SKINS.paint(…)`, `paint(…)` or `(self.paint)(…)` passes; one that
      also prints a line, or paints `&self.other(ctx)`, is an `SH2105`
      finding at its `fn draw(` line.
- [ ] A logic file naming `Rect` is a finding; `RectSomething` and a
      comment naming `Rect` are not.
- [ ] Two screens in one file are each checked.
- [ ] This engine's screens pass, and the old ui test is gone.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`screen_skin` tests, on source strings): each rule both ways,
  `skip`, test files, the `names` default.
- Integration: `lint` on a fixture game with one bad screen gives one
  `SH2105` finding; without `screen_skin`, a limit.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
