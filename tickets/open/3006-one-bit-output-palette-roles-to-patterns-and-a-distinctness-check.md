---
id: "3006"
title: "1-bit output: palette roles map to 8×8 patterns, roles that tell things apart must stay distinct, and frame-png --target playdate shows the device's picture"
type: feature
milestone: After MVP
model: opus-5.5
effort: high
status: todo
blocked_by: ["3001"]
nick_input: none
completed:
---

# 3006 — 1-bit output

## Context

Nick, 2026-10-15 (`docs/decisions.md`): the game "must be able to
translate its graphics to 1-bit easily".
[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§4 decides how:

- skins already name colours by role (ADR-0002 §4);
- a 1-bit target maps each role to an 8 × 8 pattern;
- a check makes sure the roles that tell things apart don't collapse
  into one pattern.

Today the game tells sides, unit states, danger zones and terrain apart
by colour (game ticket 1025, *No colour*). On the Playdate's black and
white screen, that would silently lose the meaning.

Sprites are made 1-bit at bake time (3007). This ticket handles the
frame: cells, rectangles and anything the skin gives a colour.

## Nick input

None.

## Scope

**In:**
- **Palette roles as data.** A skin's palette file maps role names to
  colours. If the skin format has no named roles yet, this ticket adds
  them, and the old palettes become roles; the old form is still read
  for one more minor release (2005's rule). **Either order with
  2517:** if 2517 is done, list the old form as changes of
  `PaletteRon` (`Format::changes`: a `Change::rename` where only a
  name changes, a `Change::manual` otherwise), so `shuyi check` warns
  and `shuyi upgrade` rewrites or explains it; if not, 2517 lists it
  when it lands.
- **The pattern set** in `shuyi_ui::onebit::Pattern`, 8 × 8 bits each:
  - `Black`, `White`;
  - `Dither25`, `Dither50`, `Dither75` (Bayer 8 × 8 ordered thresholds);
  - `DiagonalUp`, `DiagonalDown`, `HLines`, `VLines`, `Checker2`;
  - `Inverted(p)` of any of them.
- **The 1-bit pattern map**, `palette@onebit.ron` beside the palette. It
  maps each role to a foreground and a background pattern, plus a list
  of `distinct` groups, for example `[team.player, team.enemy,
  team.ally]`.
- **The conversion** in `shuyi_ui::onebit`:
  - `to_one_bit(frame: &Frame, map: &PatternMap) -> OneBitFrame`, a
    bitmap whose rows are padded to 32 bits, as the Playdate's LCD
    expects;
  - cells, rectangles and shapes (2336; if 2336 isn't done, it does
    this for shapes) use their role's patterns, anchored to the screen
    (so patterns don't swim when things move);
  - a colour with no role is dithered with Bayer 8 × 8 on its luminance
    `(77·r + 150·g + 29·b) >> 8`;
  - glyphs are drawn solid in their role's foreground (text is never
    dithered);
  - it uses integers only and is deterministic.
- **Changed rows.** `OneBitFrame::changed_rows(&previous) -> RowMask`,
  because the Playdate sends only changed rows to the display (budget
  in ADR-0008 §6).
- **The check**, in `shuyi check --target <id>` for profiles with
  `color: OneBit`:
  - every role the skin uses has a pattern;
  - roles in one `distinct` group have different foreground patterns;
  - text roles are `Black` on `White`, or the inverse (warn otherwise).
  - **Either order with 2347:** if 2347 is done, its optional palette
    names `key` and `key_bg` are text roles here: they get patterns in
    the map, `key` on `key_bg` keeps the text rule (black on white, or
    the inverse), and `key` and `text` form a default `distinct` group
    when the palette has `key`; if not, 2347 does it.
- **`frame-png --target playdate`** renders the `OneBitFrame` at
  400 × 240 (or the game's frame for that target), scaled by an integer
  for viewing.
- **The examples** get a pattern map, and a `frame-png` snapshot of one
  screen per example on `playdate`.

**Out (do not do):**
- dithering sprites (3007);
- drawing on the device (3010);
- checking the `distinct` groups as colour-blind players see them, and text
  contrast (2324, which reuses this ticket's groups);
- choosing the game's patterns, which is the game's work (game 1025);
- the new-game wizard's Playdate part. Either order: if 2504 is done,
  add a starting pattern map for the template's palette roles to the
  built-in template's `target-playdate` part in this PR; if not, 2504
  adds it.

## Implementation steps

1. Add the patterns and their tests (each pattern's bit count matches its
   density).
2. Add the pattern map format (a `Format`, ADR-0007) and the
   `distinct` check.
3. Write `to_one_bit` and `changed_rows`.
4. Wire up `frame-png --target`, add the check to `check --target`, and
   add the example maps and snapshots.

## Acceptance criteria

- [ ] A fixture skin with `team.player` and `team.enemy` both mapped to
      `Dither50` in a `distinct` group fails the check, naming both roles.
- [ ] The same frame converts to the same bits on x64, ARM64 and
      wasm32: a row of a golden table (2116's `shuyi_assets::golden`)
      in `shuyi-ui`, whose tests CI's `determinism` job runs on wasm32.
      Either order with 2116: if it isn't done, a plain checked-in
      hash, which 2116's job then runs.
- [ ] `changed_rows` after moving the cursor one cell marks only that
      cell's rows (test).
- [ ] `frame-png --target playdate` of an example screen matches its
      checked-in PNG.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the patterns, the luminance maths, and the role lookup with a
  fallback.
- Property: `to_one_bit` never panics, and its output's size is always
  the display's.
- Snapshot: one `frame-png` per example on `playdate`.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
