---
id: "3202"
title: "A sprite-test debug screen: one picture drawn scaled, flipped, cropped, under the glyphs, see-through and cut by a box"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3202 — A sprite-test debug screen

## Context

A request from the game.

1. **The game ticket:** game#1212, which deletes the game's own sprite
   test once this lands
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1212-use-the-engines-sprite-test-screen.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** open a debug screen in a running build
   and see by eye that its renderer draws a picture right at 1×, 3× and
   5×, mirrored, cropped, under the glyphs, at half opacity and cut by a
   box drawn after it.
3. **The friction numbers:** no friction review: found by the
   game/engine boundary audit of 2026-11-03. The game's
   `SpriteTestScreen` (`crates/ui/src/debug/sprite_test.rs`, game ticket
   0231, about 120 production lines) stayed game-side in the carve (game
   1112) "because it draws this game's test card", but the card is this
   engine's (`frame::raster::test_card`, which the game only writes to a
   PNG) and the screen names no game concept. It is the same kind of tool
   as `debug::SceneCameraScreen`, which moved here in game 1154.
4. **The generic shape:** `shuyi_ui::debug::SpriteTestScreen`, given an
   image id and its size (and what to say without one), shows that
   picture as sprite items in each way a `Sprite` can be drawn today. Its
   view (`SpriteTestView`) is plain data; `debug::glyph::paint_sprite_test`
   paints it (ADR-0002: the look is a skin). The render card (2392) holds
   a backend to the raster in CI; this screen is for a person looking at a
   real window, a new backend, the web build or a console.
5. **An example from a game that isn't tactics:** a platformer bringing
   up its web build checks that its hero's flipped sprite and the
   see-through water draw right before blaming its own art.

## Nick input

None.

## Scope

**In:**
- `crates/ui/src/debug/sprite_test.rs` (with `sprite_test/tests.rs`):
  `SpriteTestScreen`, `SpriteTestView`, `NAME = "sprite_test"`.
- `debug/glyph.rs`: `paint_sprite_test(palette, view, buf)` in
  `EngineColor` roles.
- Re-exports in `debug.rs` beside `CameraLayout, SceneCameraScreen`, and
  its module doc.
- Additive only, no API break. `SpriteTestView` is `#[non_exhaustive]`
  (ADR-0039: a new `Sprite` field may add a shot), built only by the
  screen. The game's PR that moves the pin follows this one (game#1212).

**Out (do not do):**
- Building the picture in memory: a screen can't upload a texture, so
  the game passes the id of a picture its bundle loads.
- Rotated, flipped-vertically or tinted sprites (2335, 2337): not built
  yet; the screen shows what `Sprite` has today.
- Changing the toy's `Gallery`, the render card or `frame-png`.
- A one-bit skin of this screen (debug tools draw in glyphs).

## Implementation steps

1. **Read** the game's screen (`crates/ui/src/debug/sprite_test.rs` at
   the game's `main`) and its tests; this engine's
   `debug/camera.rs` and `debug::glyph::paint_camera` for the shape.
2. **The screen:** `SpriteTestScreen::new(card: Option<(ImageId,
   ImageInfo)>, missing: String)`. `Screen<G>`: `actions` is
   `ui_actions(&[UiAction::Cancel])`; Cancel plays `MenuSound::Cancel`
   and pops; `view(&self, spec: ConsoleSpec) -> SpriteTestView`; `draw`
   calls `view` with `buf.spec()` and paints it.
3. **The view:** title (`Sprite test (debug)`), help (Cancel's key via
   `help_line`/`key_name`), `missing` when there's no card, and the
   shots in order, each a label, its cell and its `Sprite` with `dest`
   in console pixels: 1×, 3×, 5×, flipped, the top-right quarter, under
   the glyphs (with the lines printed over it), half opacity over a
   panel, and whole then cut by a double-lined text box drawn after it.
   Keep the game's layout (cells 2 to 76, rows 0 to 19) and words.
   English only, marked `// check-text: not player text`.
4. **The skin:** `paint_sprite_test` fills with `Background`, prints in
   `Highlight`/`TextDim`/`Text`, panels in `Panel` with `Border` and the
   text box in `Focus`, adding each sprite before the box that cuts it.
5. **Measure:** about 150 production lines (screen 45, view 35, skin 70),
   judged against the game's 120 and `camera.rs`.

## Acceptance criteria

- [ ] `shuyi_ui::debug::{SpriteTestScreen, SpriteTestView}` exist and
      `debug::glyph::paint_sprite_test` paints the view.
- [ ] On a console of 8 × 16-pixel cells the 16 × 16 card gives the
      nine sprites of the game's test
      `shows_the_card_in_every_way_a_sprite_can_be_drawn` (dests from
      `16,48 16x16` to `176,208 80x80`; the cut shot's two clips
      `176,208 80x32` and `176,240 40x48`).
- [ ] Without a card the screen prints `missing` and adds no item.
- [ ] Cancel pops with the cancel sound; nothing else does anything.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit (`sprite_test/tests.rs`, on the toy's `ctx()` and its card):
  the sprites and that no overlay stands in for a picture; glyphs over
  the under-the-glyphs shot; the missing card; Cancel; `actions`.
- Snapshot: one text snapshot of the frame's items.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
