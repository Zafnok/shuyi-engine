---
id: "2323"
title: "Parked: A text size the player picks: the game offers console sizes, the options screen switches between them, and a check that every screen fits each"
type: feature
milestone: "Engine: accessibility"
model: opus-5.5
effort: high
status: blocked
blocked_by: ["2209"]
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
say. 3001's check makes sure a target's smallest text is big enough;
that is the developer's floor, not the player's choice.

Bigger text in the same window means bigger cells, so fewer of them.
Screens that lay out by the buffer's size keep working; screens that
assume 80 columns don't, and the check here finds them.

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
  list, saved with the engine's settings (old saves read as 0). A row
  `OptionRow::text_size()` for the options screen. **Either order with
  2314:** if 2314 is done, make it an enum option row; if not, a
  stepped row like `text_speed`, and 2314 converts it.
- **Switching:** when the setting changes, `Game::frame` makes a new
  `GlyphBuffer` with the new spec on the next frame. Screens draw from
  their state each frame, so they redraw at the new size.
- **The renderers:** `shuyi-app`'s `render.rs` and `frame-png` draw a
  glyph at `glyph_scale` with nearest-neighbour sampling, with
  identical pixels on a test frame.
- **The check:** a harness helper, `harness::check_text_sizes(&mut
  Harness, screens)`, draws each named screen at every offered size and
  reports text cut off or drawn past the buffer, with 2209's
  fits-its-box check, naming the screen, the size and the text key.
  **Either order with 3001:** if 3001 is done, its text check runs at
  every offered size too.
- **The toy game** offers two sizes, and its tests run the check.
- **The guide**, `docs/accessibility.md` (created here, or by 2322 if it
  is done first): lay screens out from `buf.cols()` and `buf.rows()`,
  never from a constant.

**Out (do not do):**
- sizes that are not whole numbers (pixel fonts blur);
- per-element sizes, such as only the dialogue bigger;
- changing the game's own screens (the game's tickets, after the check
  shows which ones);
- a second, larger font in the atlas (a follow-up if a game wants a
  sharper big font than a scaled one).

## Implementation steps

1. Read `ConsoleSpec`, `GlyphBuffer::new`, `GameData`, `Settings` and the
   options rows, and 2209's check.
2. Add `TextSize`, the default, the setting and its saved form.
3. Rebuild the buffer on a change; scale glyphs in both renderers.
4. Write the harness check and run it on the toy game.
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
- [ ] `shuyi-app` and `frame-png` give identical pixels for a scaled
      glyph on the test frame.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: cols and rows for each scale; the setting's saved form.
- Property: for any scale, every cell lies inside the logical frame.
- Snapshot / integration: the toy game's screens at both sizes; the
  check failing on a bad screen.

## Completion notes

*(Filled in by the session that completes the ticket.)*
