---
id: "2291"
title: "Text padded, right-aligned and clipped in cells, a column's width, and a word wrap with a hanging indent or a marker"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2291 — Text padded and clipped in cells, and a hanging wrap

## Context

A request from the game.

1. **The game ticket:** game#1410, the game's menus and notes on these
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1410-cell-width-text-from-the-engine.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** lay text out in columns measured in
   cells (where a Chinese character is two), and wrap a bulleted or
   indented paragraph so its later lines hang under its text.
3. **The friction numbers:** the game's third engine-friction sweep
   (game#1397, 2026-11-27); three of its readers found these on their
   own. About 130 production lines in `crates/ui/src`, of which the game
   would keep about 30 (its markers and widths):
   - `screens/mod.rs`: `clip` (the start of a text that fits, cut with
     no `…`; the engine's `frame::cut` adds one) and `pad_cells` (pad to
     a width in cells, since `format!`'s width counts characters): about
     30 calls in 16 files;
   - "the widest name with `text_width(..).max()`, then pad each",
     written 5–6 times (the map menu, the objective, the item, magic and
     skill lists);
   - `widgets/mod.rs`: `wrap_indented` and `after_line` (a hanging
     indent over `word_wrap`), 6 calls; `screens/battle/notes.rs`:
     `wrap_notes` (a marker, then later lines indented under its text).
     The engine's own dialogue preview does the same by hand
     (`crates/ui/src/dialogue/preview/glyph.rs`, `FIX_INDENT`).
   A screen that pads by `format!("{name:<W$}")` breaks its columns in
   Chinese. The game fixed that in its menus once (its ticket 0263) and
   still has one left, which shows that the trap is easy to fall into.
4. **The generic shape**, in `shuyi_assets::text` (beside `word_wrap`)
   or `shuyi_assets::lang` (beside `text_width`):
   - `pad_cells(text, w, widths)` and `pad_cells_left(text, w, widths)`
     (right-aligned);
   - `clip_cells(text, w, widths)`: the start that fits, a wide
     character that would end past `w` left out whole;
   - `column_width(texts, widths) -> usize`: the widest;
   - `wrap_hanging(text, width, first: &str, rest: &str, widths) ->
     Vec<String>`: the first line starts with `first` (a marker, or
     nothing), the later lines with `rest` (spaces), each wrapped in the
     room left after it. A wide-character break inserts no space, as
     `word_wrap` does.
5. **Examples from games that aren't tactics:** a farming game's shop
   list of item names and prices in two columns in Chinese and English;
   a quest log of bulleted goals whose long lines hang under the text.

## Nick input

None.

## Scope

**In:**
- The five functions, with docs and doc examples, and `frame::cut`'s
  doc pointing to `clip_cells` for a cut with no mark.
- The engine's dialogue preview on `wrap_hanging`.
- New free functions are additive.

**Out (do not do):**
- A table widget or column layout in the UI (3235's look file may use
  these later).
- The game's use (game#1410).

## Implementation steps

1. **Read** `crates/assets/src/lang.rs` (`text_width`),
   `crates/assets/src/text.rs` (`word_wrap`, `NO_BREAK_SPACE`),
   `crates/ui/src/frame/draw.rs` (`cut`) and the dialogue preview's
   glyph skin.
2. The padding, clipping and width functions, with tests in English and
   with wide characters.
3. `wrap_hanging`, with tests on a marker, on an indent only, and on a
   wide-character break.
4. The preview onto it; its tests pass unchanged.

## Acceptance criteria

- [ ] `pad_cells("剑", 4, widths)` is the character and two spaces;
      `pad_cells_left` puts them before it; a text wider than `w` comes
      back unchanged (unit tests).
- [ ] `clip_cells` never ends inside a wide character and never adds a
      mark.
- [ ] `wrap_hanging` with `first = "- "` and `rest = "  "` gives lines
      no wider than `width`, each later line starting with two spaces.
- [ ] The dialogue preview's tests pass unchanged.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [ ] Guides in `docs/guides/` updated, if this adds a direction, a module or a move (ADR-0009 §5).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each function, English and wide characters.
- Property: `text_width(pad_cells(t, w)) == max(w, text_width(t))`;
  `text_width(clip_cells(t, w)) <= w`.
- Snapshot / integration: none.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
