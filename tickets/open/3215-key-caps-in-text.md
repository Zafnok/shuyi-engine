---
id: "3215"
title: "Key caps: a key's name in text drawn as a key, boxed, bold and with room around it, and the four direction keys as one picture"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: ["2347"]
nick_input: none
completed:
---

# 3215 — Key caps: a key's name in text drawn as a key

## Context

**A request from the game:**

- **The game ticket:** game#0250
  ([`0250-key-names-stand-out-in-hints.md`](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0250-key-names-stand-out-in-hints.md)),
  whose decision is recorded in the game's `docs/design/look-and-feel.md`,
  *Key names in hints* (Nick, 2026-11-16, after five rounds of mockups
  on real frames). The game's follow-up that switches its hints over is
  game#0254; it waits for this ticket and 2347.
- **The job:** every hint that names a key shows the key the way a
  keyboard looks: the name in a small rounded box, in bold, with the
  same margin all round, nothing beside it hidden; and the four arrow
  keys, when they move the cursor, as one small picture of the arrow
  cluster.
- **Friction:** no friction review: a missing feature, found in a
  playtest (game#0804) and decided in game#0250.
- **The generic shape:** a `Key` span (2347) may be painted as a
  *key cap*, a style a game turns on in its theme: a box in pixels
  behind the name (fill, 1 px edge, rounded corners), the name's glyphs
  bold, single-letter names in capitals, a fixed margin between the
  glyphs' ink and the box, and the cap given the room its box needs so
  it never covers a neighbouring glyph. The cursor's four direction keys
  are named either as one *direction cluster* (a picture the game
  supplies, when they are the arrow keys) or as four key spans in up,
  left, down, right order (any other keys).
- **Outside tactics:** a platformer's sign post ("press [Space] to
  jump", the cap a box), a farming game's tool bar, a puzzle game's
  "[↑←↓→] move" corner: every PC game that shows its keys in text.

What 2347 gives and leaves out (read 2026-11-16 at engine `main`
`4eae83e6`): `TextRole::Key` spans, filled by `fill_styled`,
`help_line_styled` and `Ctx::text_styled`; `print_styled` paints a
role as a cell's `fg` and `bg` only. Its Out names "bold, italic, inline
icons or pictures of buttons", and its ADR says "a role never changes
how many cells a span takes". This ticket is that next step and amends
the ADR: a key cap may take more cells than its name.

What the engine has today (same commit):

- `crates/ui/src/frame/glyph_buffer.rs`: `Cell { glyph, fg, bg,
  see_through }` (line 47), cell-aligned glyphs only; `Overlay` (174,
  a filled pixel rect under or over the glyphs), `Sprite` (231, an
  image part at a pixel rect), `GizmoShape::Rect` (`gizmo.rs:121`). No
  bold, no per-cell pixel offset.
- The font atlas (`cargo shuyi font-atlas`) is built from one face; the
  game's is Terminus regular (`ter-u16n.bdf`). Terminus has a bold face
  of the same cell size (`ter-u16b.bdf`, same license, SIL OFL 1.1).
- `Keymap::cursor_keys_name` (`crates/ui/src/input/keymap.rs:234`) gives
  the language's arrows word when the four cursor actions are on the
  arrow keys, else the four chords run together (`wasd`), and
  `help_keys::cursor_keys_name` (`help_keys.rs:178`) returns it as one
  `String`.

What the game measured on its mockups (Terminus 8 × 16): a capital's ink
is rows 2–11 of its cell and columns 1–6 (1–7 bold). A box of the cell's
height with the glyphs moved down 1 px leaves 2 px above and below; a
box 3 px outside the ink leaves 2 px left and right, which reaches 2 px
into each neighbouring cell. The mockups showed a `.` or `,` right after
a cap covered by it, which Nick asked to fix.

## Nick input

None here. The look is the game's decision (game#0250); this ticket
builds what the game's theme asks for, and changes no other game's look.

## Scope

**In:**

- **ADR-0122 (2347's), amended** (or a new ADR that supersedes its §4):
  a key cap may take more cells than its name; a role still never
  changes the plain text (`plain()`), only how many cells it is drawn
  in. Say how widths are measured (`StyledText::width`, `cut`, word
  wrap where 2347 or its follow-up wraps).
- **A key-cap style in the theme** (`crates/ui/src/frame/color.rs` or
  the theme's file, the implementer's choice, said in the ADR): off by
  default, so no game's look changes until it turns it on. Its settings:
  the box's fill and edge (palette names, `key_bg` from 2347 and a new
  optional `key_edge`), the glyphs' colour (`key`), `bold: bool`,
  `capitals: bool` (a name of one letter drawn as its capital; longer
  names as the keymap writes them), `margin_px` (between the glyphs'
  ink and the box, all four sides; 2 by default). Public types follow
  ADR-0039.
- **Even margins:** with the style on, the box is the line's height,
  and the name sits so its capitals have `margin_px` above and below,
  and the leftmost and rightmost ink `margin_px` from the sides. The
  implementer picks how a glyph is placed off its cell grid (a pixel
  offset on the cells of a cap, or the name drawn as a picture item)
  and records it in the ADR.
- **Room around a cap:** a cap takes the cells its box needs, so the
  glyph before and after it (a space, `.`, `,`, `:`) is drawn whole
  and uncovered. `StyledText::width` counts those cells, so help bars
  that fit by measuring still fit.
- **Bold:** the font atlas gets an optional bold face (`font-atlas`
  takes a second BDF of the same cell size; the atlas's RON names which
  glyphs have one), and the renderers draw a bold cap's glyphs from it.
  A game with no bold face falls back to drawing each glyph twice, 1 px
  apart. Both are drawn the same by `shuyi-app`, `frame-png` and the
  web.
- **The direction cluster:** a new `TextRole` value (or a `Key` span
  flag, the implementer's choice) for "the four direction keys, as the
  arrow keys". `help_keys::cursor_keys_name`'s styled form gives it when
  the four cursor actions are on the arrow keys, else four `Key` spans
  in up, left, down, right order with one space between them. With the
  key-cap style on, a cluster is drawn as a picture the theme names (an
  image and its part, one line tall; the game's is 24 × 16, four 8 × 8
  caps in an upside-down T); with it off, as today's arrows word.
- **One-bit targets** (the Playdate, 3006): the style is off there
  unless the theme's one-bit palette turns it on; 2347's patterns hold
  until a game decides (game#0256).
- **The engine's own screens** that 2347 switched to `StyledText` get
  the caps when the style is on: nothing more to switch.
- **For the game:** API additions only (the style, the cluster role,
  the bold face); the game's PR that moves the pin turns the style on
  (game#0254). Say it in the PR.
- **Estimated size:** about 1,400 lines (code 700, tests 550, docs 150).
- **Split:** past 900 lines, split off the direction cluster (the role,
  the styled `cursor_keys_name`, the picture) into a new ticket blocked
  by this one.

**Out (do not do):**

- Controller buttons' looks (coloured face buttons, the platform's
  icons): game#0255 decides them first; a ticket of their own here.
- The Playdate's one-bit look of a cap (game#0256).
- Proportional text (2334), markup in text files, wrapping beyond what
  2347 or its follow-up wraps.
- Any game's palette or picture: the game supplies its own.

## Implementation steps

1. Read 2347's ADR and code, game#0250's decision in the game's
   `docs/design/look-and-feel.md` (*Key names in hints*), and the files
   above.
2. Amend the ADR (or write the new one).
3. The style in the theme, off by default; its loading and checks.
4. Cell widths: a cap's cells in `StyledText::width`, `cut` and
   `print_styled`.
5. Drawing: the box (pixel items under the glyphs), the name's placement
   with even margins, capitals, bold (the atlas's second face and the
   double-strike fallback) in `shuyi-app`, `frame-png` and the web.
6. The direction cluster: its role, the styled `cursor_keys_name`, the
   picture.
7. Snapshots and pictures; the gates.

## Acceptance criteria

- [ ] With the style off (every existing palette and theme), every
      engine snapshot is unchanged.
- [ ] With the style on and a 2 px margin, a test reads the pixels of a
      one-letter cap (`W`, `D`, `F`, `E`) and finds 2 px between the
      ink and the box on all four sides.
- [ ] A cap followed by `.` and by `,`: a test finds every pixel of the
      `.` and `,` glyphs drawn as without the cap.
- [ ] `StyledText::width` of `"F select · Space end turn"` with caps
      on counts the caps' extra cells, and `print_styled` advances by
      the same.
- [ ] Capitals on: `f` is drawn `F`; `Space` and `Shift+Space` as
      written. `plain()` is unchanged either way.
- [ ] Bold: with a bold face in the atlas a cap draws its glyphs from
      it; without one, double-struck. `frame-png` of both reviewed.
- [ ] The cursor on the arrow keys gives one cluster span, drawn as the
      theme's picture; on W A S D, four `Key` spans in up, left, down,
      right order with spaces between.
- [ ] On a one-bit target with no one-bit setting, nothing changes.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the style's loading and defaults; widths with caps; capitals;
  the cluster's spans for arrow keys and other keys.
- Property: with caps on, `plain()` never changes; widths are the
  plain width plus the caps' extra cells.
- Snapshot / pictures: a help line, a text box's corner and a line of
  prose with caps on (glyph cells and `frame-png`), and the cluster.

## Completion notes

*(Filled in by the session that completes the ticket.)*
