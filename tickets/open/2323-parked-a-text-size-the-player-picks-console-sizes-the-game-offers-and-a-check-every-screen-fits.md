---
id: "2323"
title: "Parked: A text size the player picks: the game offers console sizes, the options screen switches between them, and a check that every screen fits each"
type: feature
milestone: "Engine: accessibility"
model: opus-5.5 / gpt-6.1-sol
effort: high
status: blocked
blocked_by: ["2209", "2018", "2392"]
nick_input: none
completed:
---

# 2323 — A text size the player picks

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after MVP/parity) tickets for networking, styles for later"): the game doesn't ask for it, so this waits until after the engine reaches parity with the game (the MVP in `docs/ROADMAP.md`) and after 2209. Nobody starts it until Nick unparks it. Whoever unparks it sets `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.6 (Bevy's
`UiScale`, Godot's content scale and theme font sizes). The Game
Accessibility Guidelines and Microsoft's Xbox Accessibility Guidelines
(XAG 101) both ask that players can make text bigger.

Shuyi draws text in a console: `cols × rows` cells of `cell_w × cell_h`
logical pixels (`ConsoleSpec`, `crates/ui/src/frame/console.rs`). The
game picks one with `GameData::console()` (game ADR-0070 §6), and the
console is scaled to fit the window (game ADR-0012). The player has no
say in the console. The window may show the logical frame at a whole or
a fractional scale (2333's `ScaleMode`). `glyph_scale` here sizes the
cells inside the logical frame and stays whole either way; under `Fit`,
glyphs are still drawn with nearest sampling into the whole-scale
picture that 2333 then smooths. 3001's check makes sure a target's
smallest text is big enough; that is the developer's floor, not the
player's choice.

Bigger text in the same window means bigger cells, so fewer of them.
Screens that lay out by the buffer's size keep working; screens that
assume 80 columns don't, and the check here finds them.

Sound captions (2366, parked) are drawn in cells by
`sound_captions::paint`, which reads the buffer's width and height, so
they follow the text size with no work here.

## Nick input

None.

## Scope

**In:**
- **`GameData::text_sizes(&self) -> Vec<TextSize>`**, defaulted to one
  size, `self.console()`, so nothing changes for a game that doesn't
  offer more (the game's PR that moves the pin needs no change).
  `TextSize { label: text key, glyph_scale: u8 }`: the font is drawn
  at a whole-number scale (1, 2 or 3), cells grow by that scale, and
  `cols` and `rows` shrink to what fits the same logical frame. Sprites
  and rectangles, placed in pixels, don't move.
- **The setting:** `Settings::text_size: u8`, an index into the game's
  list, saved with the engine's settings (old saves read as 0).
  **Either order with 2110:** if it is done, each versioned settings
  form that gains the field (2110 versions 2511's `()` form and the grid
  template's) raises its `VERSION`, reads the old version by
  `Self::deserialize` in `Migrate::upgrade`, and adds a `v<N>_*.ron`
  fixture beside the old ones, which must still read (its saves test
  fails otherwise); if not, 2110's version 1 has it. A row
  `OptionRow::text_size()` for the options screen: a choice row over
  the game's list, written by hand like `text_speed`. 2314's enum option
  rows don't fit it: its values are an index into a list the game gives
  at run time, not an enum's `ALL`. **Either order
  with 2351:** if it is done, the row's own text keys get their English
  in the engine's `crates/ui/lang/en/ui.ron`; if not, in the toy's
  `en.ron`, and 2351 moves them. The labels of the sizes the game
  offers (`TextSize::label`) stay the game's.
- **Switching:** when the setting changes, `Game::frame` makes a new
  `GlyphBuffer` with the new spec on the next frame. Screens draw from
  their state each frame, so they redraw at the new size.
- **The renderers:** `shuyi-app`'s `render.rs` and the raster
  (`shuyi_ui::frame::raster`, 2332) draw a glyph at `glyph_scale` with
  nearest-neighbour sampling, with the same pixels: a capture at the
  second size, on 2392's render card or in a hello-grid plan, compared
  by 2018's `render` CI job.
- **Text runs (2334), if done:** a run keeps its position, and its
  glyphs, advances, line height, and outline and shadow offsets grow by
  `glyph_scale`, so text placed in pixels grows with the cells. The
  check covers runs too (`TextRun::cut`). If 2334 isn't done, nothing
  here: 2334 does it when it lands (its clause says so).
- **The check:** a harness helper, `harness::check_text_sizes(&mut
  Harness, screens)`, draws each named screen at every offered size and
  reports text cut off or drawn past the buffer, with 2209's
  fits-its-box check (`shuyi_assets::dialogue::lines_at_longest`),
  naming the screen, the size and the text key.
  **Either order with 2353:** if 2353 is done, build `check_text_sizes`
  on its walk (`shuyi_platform::harness::walk`: its `Stop` list and its
  cut finding) and run it at every offered size. If not, write the
  check so 2353 can reuse it, and 2353 moves it onto its walk.
  3001's text check runs at every offered size too.
- **The toy game** offers two sizes, and its tests run the check.
- **The guide**, `docs/accessibility.md` (created here, or by 2322 if it
  is done first): lay screens out from `buf.cols()` and `buf.rows()`,
  never from a constant.

**Out (do not do):**
- sizes that are not whole numbers (pixel fonts blur);
- per-element sizes, such as only the dialogue bigger;
- changing the game's own screens (the game's tickets, after the check
  shows which ones);
- a second cell font in the atlas (a follow-up if a game asks). A
  sharper big font for text off the grid is a proportional font drawn
  as a text run (2224, 2334);
- a console that grows by whole cells when the window's shape changes
  (an aspect policy like Godot's `expand`, found by the 2026-10-20
  comparison): 2376 (parked), after this ticket and 2333, reusing
  this ticket's new buffer for a new console size and 2333's `place`.

## Implementation steps

1. Read `ConsoleSpec`, `GlyphBuffer::new`, `GameData`, `Settings` and the
   options rows, 2209's check, 2392's render card and 2018's plans. Check
   which of 2110, 2334, 2351, 2353 and 3001 are done.
2. Add `TextSize`, the default, the setting and its saved form.
3. Rebuild the buffer on a change; scale glyphs in `shuyi-app` and the
   raster, and text runs if 2334 is done. Add the capture at the second
   size.
4. Write the harness check (on 2353's walk if it is done) and run it on
   the toy game.
5. Write the guide section.
6. Run the gates.

## Acceptance criteria

- [ ] A game that doesn't override `text_sizes` draws the same frames as
      before (the existing snapshots don't change).
- [ ] The toy game at its second size draws every screen with no text
      cut, by the check; snapshots at both sizes are committed and
      reviewed with `frame-png`.
- [ ] A screen that draws at column 79 at every size fails the check,
      naming the screen and the size.
- [ ] Old saved settings load with size 0.
- [ ] `shuyi-app` and the raster give the same pixels for a scaled
      glyph (2018's `render` job passes with a capture at the second
      size).
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: cols and rows for each scale; the setting's saved form; a text
  run's advances and line height at each scale, if 2334 is done.
- Property: for any scale, every cell lies inside the logical frame.
- Snapshot / integration: the toy game's screens at both sizes; the
  check failing on a bad screen; the capture at the second size in
  2018's `render` job.

## Completion notes

*(Filled in by the session that completes the ticket.)*
