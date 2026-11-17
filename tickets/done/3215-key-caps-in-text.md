---
id: "3215"
title: "Key caps: a key's name in text drawn as a key, boxed, bold and with room around it, and the four direction keys as one picture"
type: feature
milestone: MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: done
blocked_by: ["2347"]
nick_input: none
completed: 2026-11-16
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

- [x] With the style off (every existing palette and theme), every
      engine snapshot is unchanged. (The render card's snapshot changed:
      it holds every kind of thing a frame draws, and now two caps.)
- [x] With the style on and a 2 px margin, a test reads the pixels of a
      one-letter cap (`W`, `D`, `F`, `E`) and finds 2 px between the
      ink and the box on all four sides.
- [x] A cap followed by `.` and by `,`: a test finds every pixel of the
      `.` and `,` glyphs drawn as without the cap.
- [x] `StyledText::width` of `"F select · Space end turn"` with caps
      on counts the caps' extra cells, and `print_styled` advances by
      the same.
- [x] Capitals on: `f` is drawn `F`; `Space` and `Shift+Space` as
      written. `plain()` is unchanged either way.
- [x] Bold without a bold face: double-struck, `frame-png` reviewed.
      ~~With a bold face in the atlas a cap draws its glyphs from it.~~
      (not done: the bold face moved to 3216, to keep the PR's size)
- [x] ~~The cursor on the arrow keys gives one cluster span, drawn as the
      theme's picture; on W A S D, four `Key` spans in up, left, down,
      right order with spaces between.~~ (not done: the direction
      cluster moved to 3217, at the ticket's size valve)
- [x] On a one-bit target with no one-bit setting, nothing changes.
- [x] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the style's loading and defaults; widths with caps; capitals;
  the cluster's spans for arrow keys and other keys.
- Property: with caps on, `plain()` never changes; widths are the
  plain width plus the caps' extra cells.
- Snapshot / pictures: a help line, a text box's corner and a line of
  prose with caps on (glyph cells and `frame-png`), and the cluster.

## Completion notes

Done (ADR-0124, which supersedes ADR-0122 §4):

- **The style:** `KeyCapStyle` (`shuyi_ui::frame::key_cap`: bold,
  capitals, `margin_px` 2, `one_bit`), turned on by
  `GameData::key_caps` (default `None`). Its colours are the palette's
  `key`, `key_bg` (the box) and a new optional `key_edge` (its 1 px edge,
  else the fill). `Ctx::key_caps` makes the `KeyCaps` the frame draws
  with; `Game` hands it to its buffer before each drawing. A one-bit
  screen (`PlatformFacts::screen`, new, from the profile's display
  colour) gets none unless `KeyCapStyle::one_bit`.
- **Room:** a cap takes its name's cells and a fixed number more
  (`KeyCaps::extra_cells`: one for Terminus and 2 px). `CellWidths`
  carries it (`with_key_cap_cells`), so `StyledText::width` and `cut`
  count it with a frame's `buf.widths()`; a cut keeps a cap whole.
- **Drawing:** `print_styled` prints the name (one letter as its
  capital), the extra cells, the box as `Under` rectangles inside the
  cap's cells (corners left out, rounded), and a `GlyphStyle` (moved
  `dx`, `dy` pixels, bold) on the name's cells, kept in a sparse map
  beside the cells (`glyph_style`, `set_glyph_style`), cleared by
  whatever replaces a cell. Both renderers (`shuyi-app`, the raster's
  `Painter`, so `frame-png` and the web) draw a styled glyph moved, and
  bold struck twice; snapshots list the styles; the render card has two
  caps.
- **Even margins:** `font-atlas` measures the font's ink into
  `atlas.ron` (`FontInk`: the box most capitals share the edges of, and
  the columns of each printable ASCII glyph that differs). Terminus:
  columns 1–6, rows 2–11, as the game measured; `W` 0–6. The template's
  atlas carries it (a test makes it again and compares); Hello, Grid's
  and the book's copies stay as they were, as Hello, Grid's recorded
  sessions are stamped with its files, and none of them draws caps. A
  pack holds the ink: the pack format is version 16.

Deviations:

- **The direction cluster is split off** to 3217, at the ticket's own
  size valve (past 900 lines).
- **The bold face is split off** to 3216 (`FontAtlasDef::bold`,
  `font-atlas --bold`, renderers drawing from it): the PR was at 2,162
  counted lines with it, and it lands apart cleanly, as bold struck
  twice works without it. It was built and taken out; commit `3f6b8875`
  on this branch has it, and 3216 says to start from there.
- **Ink per glyph, not one box per font.** The ticket's numbers are
  Terminus's capitals, but `W` inks column 0: one box for every letter
  gave `W` 1 px on its left. The atlas records the columns of each
  printable ASCII glyph that differs, and a cap fits its box to its
  name's first and last glyph.
- **The style is code, not the theme's file.** A palette file holds
  only colours; the theme's colours (`key`, `key_bg`, `key_edge`) are
  the palette's, the switches and the margin `GameData::key_caps`
  (ADR-0124 §3).
- **Vertical margin:** the box is never taller than the line; with a
  margin too big for it, the ink is centred (ADR-0124 §6). Terminus at
  2 px is the line's height, 2 px all round.
- **Pictures:** the review pictures are written by an ignored test,
  `cargo test -p shuyi-cli key_caps_pictures -- --ignored`
  (`target/key-caps/`), not committed: a help line, a text box's
  corner (`b log  f ▼`) and a line of prose, caps off and on.

Size: 1,937 counted lines, not split further: the cluster and the bold
face went to 3217 and 3216, and what is left lands together. A cap's
room (`CellWidths`, `width`, `cut`), its printing and its glyph style
in the renderers only work as a whole (a cap counted but not drawn
misaligns every hint; drawn but not counted, it covers its neighbour),
and its margins need the atlas's ink; 835 of the lines are the tests
that read its pixels.

Tickets written here:

- 3216: the bold face in the font atlas (above).
- 3217: the direction cluster (above).
- No ticket: a bold face's pictures for the game, and Terminus Bold, are
  the game's (game#0254).

Game: game#0254 (the game turns the style on, adds its bold face and
the cluster's picture); it now waits for 3216 and 3217 as well, and its
`blocked_by` should name them. Its pin move breaks one struct literal:
`gh search code --repo Zafnok/visions-of-shuyi 'FontAtlasDef {'` finds
`crates/xtask/src/icon.rs`'s test atlas, which adds `ink: None`. The
game's other uses of the changed types (`gh search code --repo
Zafnok/visions-of-shuyi CellWidths`, `PlatformFacts`, `print_styled`)
read them or build them by their functions: no change. With no
`key_caps` the game's look and snapshots are unchanged.

Breaking: shuyi-assets: FontAtlasDef: a new field `ink` (the font's ink, `Option<FontInk>`); a struct literal adds `ink: None`, and an atlas file without it still reads

```migration
since: 0.1.0
what: The font atlas's description has a new field, ink, which font-atlas writes; a game that builds a FontAtlasDef by hand adds it.
old: "FontAtlasDef { cell_w, cell_h, columns, glyphs, wide, wide_sets }"
new: "FontAtlasDef { cell_w, cell_h, columns, glyphs, wide, wide_sets, ink }"
before: |
  FontAtlasDef {
      cell_w: 2,
      cell_h: 4,
      columns: 2,
      glyphs: [('b', 0), ('a', 1)].into_iter().collect(),
      wide: BTreeMap::new(),
      wide_sets: BTreeMap::new(),
  }
after: |
  FontAtlasDef {
      cell_w: 2,
      cell_h: 4,
      columns: 2,
      glyphs: [('b', 0), ('a', 1)].into_iter().collect(),
      wide: BTreeMap::new(),
      wide_sets: BTreeMap::new(),
      ink: None,
  }
```

Breaking: shuyi-assets: pack::VERSION: 16 (a font atlas's ink); a pack baked before is refused, so a game bakes again with `shuyi bake`

```migration
since: 0.1.0
what: The pack format is version 16; a pack of version 15 is refused.
old: "pack::VERSION == 15"
new: "pack::VERSION == 16"
before: |
  cargo shuyi bake   # with the engine before 3215
after: |
  cargo shuyi bake   # again, with this engine
```

Breaking: shuyi-assets: the golden value `font atlas: font-atlas` (`crates/assets/tests/golden/formats.ron`): a decoded atlas has `ink`, so its state hash changed; a game's golden table of a font atlas is blessed again

```migration
since: 0.1.0
what: A decoded font atlas has an ink field, so its state hash in a golden table changes.
old: "golden font atlas hash before 3215"
new: "golden font atlas hash with ink: None"
before: |
  cargo xtask golden
after: |
  cargo xtask golden --bless
```

Behaviour: shuyi-cli: `font-atlas` writes an `ink` block at the end of `atlas.ron` (the capitals' box and the columns of the ASCII glyphs that differ); an atlas made again differs from one made before by that block

Behaviour: shuyi-ui: with a game's `key_caps` on, `StyledText::width` and `cut` measured with a frame's widths count a key cap's extra cells, and a key's name in the frame's cells is followed by a space (its room) and, if one letter, is its capital; with it off (every game today), nothing changes

Book: chapter 4, *Naming a key in text* (`site/content/learn/book/actions.md`), says how a game turns key caps on; the `shuyi-input` skill and `docs/formats.md` (the atlas's `ink`) too.

Nick input: none.
