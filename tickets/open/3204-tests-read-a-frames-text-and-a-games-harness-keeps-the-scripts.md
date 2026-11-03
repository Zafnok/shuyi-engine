---
id: "3204"
title: "Tests read a frame's text, find glyphs the font lacks, and a game's own Harness keeps the scripts"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3204 — Tests read a frame's text, and a game's own Harness keeps the scripts

## Context

A request from the game.

1. **The game ticket:** game#1214, which deletes the game's copies once
   this lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1214-tests-read-frames-through-the-engines-readers.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** a test asks "what does row 31 say?",
   "is `LEVEL CLEAR` on screen?" or "does this frame draw a glyph the
   font lacks?" in one call, and a game that wraps the `Harness` with
   lookups of its own chains the engine's scripts on its wrapper.
3. **The friction numbers:** no friction review; found by the
   game/engine boundary audit of 2026-11-03. The game holds about 54
   copies of a 5-8 line `row`/`shows`/`text(buf, x, y, w)` helper over
   `GlyphBuffer::get`, and 7 copies of an 8-line "every glyph drawn is
   in the font" test (split `snapshot()` at `--- colours ---`, then
   `FontAtlasDef::glyph_rect`): about 350-420 test lines. Its Harness
   newtype re-declares 10 scripts as one-line forwarders (about 40
   production lines), only because `Harness::keys` and the others
   return `&mut Harness<G>`, which drops the wrapper's own methods from
   a chain. This repository has 5 copies of `fn row` too
   (`ui/src/debug/tests.rs:19`, `debug/camera/tests.rs:38`,
   `dialogue/text_box/glyph/tests.rs:55`, `frame/draw/tests.rs:13`,
   `frame/glyph_buffer.rs:815`).
4. **The generic shape:** text readers on `GlyphBuffer` (`shuyi_ui`)
   and the same on the `Harness` for its current frame; a list of the
   glyphs a frame draws that a `FontAtlasDef` lacks; and a
   `forward_scripts!` macro that gives a game's wrapper type the
   Harness's scripts, returning the wrapper.
5. **An example from a game that isn't tactics:** a puzzle game's test
   checks that row 2 reads `LEVEL CLEAR` after the last move; a farming
   game wraps the Harness with `crops()` and `with_rain()` and writes
   `h.keys("f").with_rain()`.

## Nick input

None.

## Scope

**In:**
- `GlyphBuffer` (`crates/ui/src/frame/glyph_buffer.rs`): `row_text(y)`,
  `text_at(x, y, w)`, `rows()`, `shows(text)`, `glyphs_missing(font)`.
- `Harness` (`crates/platform/src/harness.rs`): the same five on
  `self.game().buffer()`.
- `forward_scripts!` (`#[macro_export]`, documented in the `harness`
  module).
- The 5 copies of `fn row` above use `row_text`.
- `site/content/learn/book/testing.md` (*The Harness*) and
  `skills/shuyi-test/SKILL.md`: the readers, and that a test of what
  happened reads a view, the readers being for looks and end-to-end
  checks; the macro for a wrapper.
- Additive only: no API break. Expected size: about 130 production
  lines (readers 40, Harness 30, macro 60), about 25 test lines removed.

**Out (do not do):**
- Text runs, sprites and overlays: the readers read cells only.
- A check of text against the atlas in `shuyi check` (that is 2350's).
- A generic wrapper (`struct W<G>(Harness<G>)`): the macro takes a
  concrete type.

## Implementation steps

1. `GlyphBuffer::row_text(&self, y: i32) -> String`: the glyph of each
   cell of row `y`, full width, untrimmed; empty for a row off the
   buffer. `text_at(&self, x, y, w: i32) -> String`: cells `x..x + w`,
   a cell off the buffer read as `' '`. `rows(&self) -> Vec<String>`.
   `shows(&self, text: &str) -> bool`: some row contains `text`.
2. `GlyphBuffer::glyphs_missing(&self, font: &FontAtlasDef) ->
   Vec<char>` (`shuyi_assets::font`): each distinct glyph a cell holds
   whose `font.glyph_rect` is `None`, in reading order.
3. `Harness::row`, `text_at`, `rows`, `shows`, `glyphs_missing`,
   delegating to the current frame's buffer.
4. `forward_scripts!($wrapper:ty, $field:tt, $game:ty)`: an inherent
   `impl $wrapper` with every `&mut self -> &mut Self` script of
   `Harness` from `music_load_delay` to `clear_audio` (keys, type_text,
   hold, pad, hold_pad, wait, use_pad, crank, set_crank, act, system,
   frame, change_file, music_load_delay, without_music, clear_audio),
   each calling `self.$field.<script>(..)` and returning `self`, with a
   doc line linking the Harness method. Types by `$crate::` paths;
   `act` takes `<$game as GameData>::Action`. A doc comment on
   `Harness` says a new script of that shape goes in the macro too.
5. Replace the 5 `fn row` copies; update the book and the skill.

## Acceptance criteria

- [ ] `GlyphBuffer::row_text`, `text_at`, `rows`, `shows` and
      `glyphs_missing` exist, with unit tests in `glyph_buffer.rs`.
- [ ] `Harness::row`, `text_at`, `rows`, `shows` and `glyphs_missing`
      exist.
- [ ] A test wrapper over `shuyi_ui::toy::game()`
      (`crates/platform/tests/it/harness.rs`) calls
      `forward_scripts!` and compiles a chain
      `w.keys("…").own_method()`.
- [ ] `grep -rn "fn row(buf: &GlyphBuffer\|fn row(b: &GlyphBuffer" crates`
      finds nothing.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: each reader on a small buffer (a row off the buffer, a span
  past its edge, a glyph missing twice listed once).
- Integration: the wrapper chain; `glyphs_missing` on a toy frame with
  a font lacking one glyph it draws.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
