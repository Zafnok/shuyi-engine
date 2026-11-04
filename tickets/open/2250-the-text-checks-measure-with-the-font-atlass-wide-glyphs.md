---
id: "2250"
title: "The text checks measure with the font atlas's wide glyphs, and a reloaded atlas changes the frame's widths"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2250 — The text checks measure with the font atlas's wide glyphs

## Context

- **From 2301 (done):** a font atlas may hold wide glyphs
  (`shuyi_assets::font::FontAtlasDef::wide`), two cells each, and
  `shuyi_assets::lang::text_width(text, &CellWidths)` counts a wide
  character as two. The frame (`GlyphBuffer::widths`) and the context
  (`Ctx::widths`) get the atlas's widths once the game has loaded
  (`Game::set_widths`, called by `shuyi_platform::App` from the look's
  atlas), so screens and widgets lay text out by its cells.
- **What 2301 left narrow:** the checks that keep text to a length run
  as the text loads, before anything has the atlas, and measure with
  `CellWidths::NARROW`:
  - `shuyi_assets::dialogue::check::longest_width` and `token_cells`
    (`crates/assets/src/dialogue/check.rs`), and through them
    `lines_at_longest`, `Checker::step` and a pack's `add_lines`
    (`crates/assets/src/lang/lines.rs`);
  - `Names::longest` (`crates/assets/src/names.rs`);
  - a tip's title and lines (`crates/assets/src/tip.rs`, `parse`).

  So a Japanese pack's line of 20 wide characters passes a 30-cell
  limit it overflows on the screen.
- **And one thing at run time:** a hot reload of the atlas
  (`shuyi-app`'s `Renderer::remake`, `Remake::Font`) gives the renderer
  the new atlas, but the frame keeps the widths it was given at load.
- No friction review: found while doing 2301.
- **Outside tactics:** any game with Japanese or Chinese text and a text
  box, a tip or a name limit.

## Nick input

None.

## Scope

**In:**
- The length checks above take a `&CellWidths` and measure with it.
- Where the game hands the rules over, a way to give the widths: a
  field `widths: CellWidths` on `lang::PackRules` and on `tip::TipRules`
  (both plain structs with public fields: adding one breaks a game that
  builds them, so a `Breaking:` line), and the dialogue `Checker` takes
  them where it takes its `DialogueLimits`.
- `shuyi check` and the game's tools measure with the atlas's widths:
  `GameTools::font_atlas` is there already (2350); the text checks run
  again over the loaded text with `CellWidths::of(&atlas)`, or the
  atlas loads first and its widths go into the rules. Pick one, say why
  in the completion notes.
- The template's and Hello, Grid's loads pass the atlas's widths
  (`CellWidths::of`) into their rules.
- A hot reload of the font atlas calls `Game::set_widths` with the new
  atlas's widths (`crates/platform/src/app.rs`, where the App tells the
  game of changed files, or `shuyi-app`'s `backend.rs`).

**Out (do not do):**
- Line breaking for text without spaces (2302).
- Widths by language (2303).
- Any change to how a cell draws.

## Implementation steps

1. Give `longest_width`, `token_cells`, `lines_at_longest`,
   `Names::longest` and the tip check a `widths: &CellWidths`, and pass
   `CellWidths::NARROW` where nothing has an atlas yet (the engine's own
   tests).
2. Add `widths: CellWidths` to `PackRules` and `TipRules`, and thread it
   to the checks (`add_lines`, `Checker`, `tip::parse`).
3. In the template (`templates/shuyi-default`) and `examples/hello-grid`,
   load the font atlas before the text and pass its widths.
4. On a reload of the font atlas, `Game::set_widths(CellWidths::of(…))`.
5. Unit tests: a wide character counts as two in each check.
6. Say in `docs/formats.md`, at the language pack, that its limits count
   a wide character as two cells.

**Either order with 2239:** if 2239 is done, `box_fit`
(`crates/assets/src/dialogue/check.rs`) measures with `longest_width`
and `lines_at_longest`: give it the `CellWidths` too, and its two
callers pass the table; if not, nothing to do (2239 passes it, as its
clause says).

## Acceptance criteria

- [ ] A pack's speech line of wide characters over `max_text` cells
      fails its check, though its characters are fewer (unit test).
- [ ] A tip and a name of wide characters are measured as two cells a
      character (unit tests).
- [ ] `shuyi check` measures with the atlas's widths (test).
- [ ] A reloaded atlas with a new wide glyph prints it in two cells
      (test).
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each check with a wide character.
- Snapshot / integration: `shuyi check` on a pack of wide text; a hot
  reload of the atlas in the Harness.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
