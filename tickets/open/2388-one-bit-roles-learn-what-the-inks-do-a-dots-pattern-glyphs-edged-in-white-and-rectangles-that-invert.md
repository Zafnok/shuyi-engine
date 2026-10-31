---
id: "2388"
title: "One-bit roles learn what the inks do: a Dots pattern, glyphs edged in white, and rectangles that invert"
type: feature
milestone: After MVP
model: implementation
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2388 — One-bit roles learn what the inks do

## Context

Found by an analysis of the engine for areas that do the same job twice
(2026-10-27). A one-bit screen has two ways to say what a colour is
drawn in, both behind the renderer's `Shades` trait
(`crates/ui/src/frame/onebit.rs`):

- **The game's inks** (`shuyi_assets::ink::Ink`, `InkShades`, game
  ADR-0074): eight inks found by reserved RGB codes. They came with the
  carve, and Visions of Shuyi's Playdate build draws with them
  (`onebit::render`, `render_marked`, `render_changed`).
- **Roles' patterns** (ADR-0026, `shuyi_assets::onebit`, `PatternMap`):
  a pattern map beside the palette gives each palette role a foreground
  and a background pattern. This is the engine's own, generic way.

ADR-0026 kept the inks so the game's build wouldn't change, and said the
game "can move to roles one screen at a time, since both are `Shades`".
It can't yet: roles lack three things the inks do.

| Ink | As a role today |
| --- | --------------- |
| `White`, `Black` | `White`, `Black` |
| `Grey` (black where `x + y` is even) | `Dither50`, the same pixels |
| `Diagonals` (black where `x − y` is a multiple of 4) | `DiagonalDown`, the same pixels |
| `Dashes` (2 × 2 squares) | `Inverted(Checker2)`, the same pixels |
| `Dots` (one pixel in eight, rows staggered) | **none** |
| `BlackEdged` (a glyph gets a white edge one pixel wide) | **none** |
| `Invert` (a rectangle flips what is under it) | **none** |

The renderer can already draw all three: a `Shade` has `edged` and
`flips`, and only `InkShades` ever sets them. This ticket lets a pattern
map say them, and proves a map can draw everything the inks draw, pixel
for pixel. 2389 then removes the inks, once the game draws with roles.

Its PR also writes the game's ticket for that move, in the game
repository (`github.com/Zafnok/visions-of-shuyi`, its `write-ticket`
skill): "the Playdate build draws with a pattern map instead of inks",
with the pattern map below. It adds that ticket to 2389's `blocked_by`
as `game#NNNN`.

## Nick input

None.

## Scope

**In:**
- **`Base::Dots`** in `shuyi_assets::onebit`: black where `y` is even
  and `x % 4 == y % 4`, white elsewhere (the ink's pixels exactly; 8
  black pixels in 64). It joins `Base::ALL`, `black_pixels`, the
  spelling a map reads (`Dots`, `Inverted(Dots)`) and the check's
  look-alike rule, which works on rows and needs nothing new.
- **`RolePatterns`** gains `edge: bool` and `invert: bool`, both `false`
  when the map doesn't say (`#[serde(default)]`):
  - `edge`: a glyph in the role gets a white edge one pixel wide
    (`Shade::edged`);
  - `invert`: a rectangle in the role flips the pixels under it, after
    everything else (`Shade::flips`).
  A map spells them `"cursor": (fg: Black, invert: true)` and
  `"text.on_pattern": (fg: Black, edge: true)`.
  `RolePatterns` has public fields and is not `#[non_exhaustive]` yet:
  make it so now, with `RolePatterns::new` kept and `with_edge` and
  `with_invert` added (ADR-0039).
- **`PatternMap`** (`crates/ui/src/frame/onebit/roles.rs`) sets them:
  `glyph` gives the role's `fg` with `edged` set by `edge`, `shape` gives
  it with `flips` set by `invert`, `ground` gives neither.
- **The check** (`shuyi_assets::onebit::check`): a role in `text` with
  `invert` is an error ("a text role can't invert: its glyphs would
  flip"). Nothing else changes.
- **The proof** (a test in `crates/ui/src/frame/onebit/tests.rs`): a
  palette of the eight inks' codes, named by ink, and this pattern map
  over it:
  ```ron
  (
      roles: {
          "white": (fg: White),
          "black": (fg: Black),
          "black_edged": (fg: Black, edge: true),
          "grey": (fg: Dither50),
          "dots": (fg: Dots),
          "diagonals": (fg: DiagonalDown),
          "dashes": (fg: Inverted(Checker2)),
          "invert": (fg: Black, invert: true),
      },
  )
  ```
  `render_with` that map equals `render` (the inks) for random buffers
  drawn in those colours only, using the generators the speed tests use
  (`crates/ui/src/frame/onebit/speed_tests.rs`). If they differ because
  the renderer reads `edged` or `flips` off a ground, set them in
  `ground` too, and say so in the completion notes.
- **Docs** through the `document` skill: the pattern map's fields in
  `docs/formats.md` (or wherever the docs map says the pattern map is
  described), and ADR-0026 amended (the `write-adr` skill) with the
  three additions and the table above.
- **The game's ticket**, as Context says, and `game#NNNN` added to
  2389's `blocked_by` in this PR.
- **For the game:** an API break, which CI's `api` job reports
  (`RolePatterns` gains fields and `#[non_exhaustive]`; `Base` gains a
  variant), with `Breaking:` lines in the completion notes (ADR-0038
  §6). The game builds no `RolePatterns` and matches no `Base` today.

**Out (do not do):**
- Removing the inks, `InkShades` or `ink_of`: 2389 does that once the
  game has moved.
- Moving the game's Playdate build onto roles: the game's ticket does.
- Colours of no role: roles dither them by luminance where the inks
  draw them solid white or black by brightness. The proof holds only
  for the ink colours; 2389 says what the game checks about that.
- 2371's one-bit look for the engine's own screens: it can use `edge`
  and `invert` once they exist, and needs nothing from this ticket to
  start.

## Implementation steps

1. Add `Base::Dots` in `crates/assets/src/onebit.rs`, with its spelling,
   `ALL`, `black_pixels` and a test of its rows against the ink's
   (`ink_is_white(Ink::Dots, x, y)` for every pixel of 8 × 8).
2. Add `edge` and `invert` to `RolePatterns`, `#[non_exhaustive]`,
   `with_edge`, `with_invert`; the text-role check.
3. Set `edged` and `flips` in `PatternMap`'s `Shades`.
4. Write the proof test, and tests of a map with `edge` and `invert`
   drawn through `to_one_bit`.
5. Docs and the ADR-0026 amendment.
6. Write the game's ticket in the game repository and add its id to
   2389's `blocked_by`.
7. Run the gates.

## Acceptance criteria

- [ ] `Dots` has the ink's pixels exactly (test over all 64).
- [ ] A pattern map with `edge: true` draws a glyph with a white edge,
      and one with `invert: true` flips what a rectangle covers (tests
      through `to_one_bit`).
- [ ] The proof test passes: the map above draws every random ink-only
      buffer exactly as `InkShades` does.
- [ ] `check` refuses a text role with `invert`, naming the role.
- [ ] Old pattern maps read the same (`edge` and `invert` default to
      `false`; an existing map's test still passes).
- [ ] The game's ticket exists in the game repository, and 2389's
      `blocked_by` names it.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Dots`' rows; `Inverted(Dots)`; reading `edge` and `invert`;
  the default; the text-role check; `PatternMap`'s `glyph`, `shape` and
  `ground` with each.
- Property: the proof test (inks against the map, random buffers).
- Snapshot: none (a one-bit frame is checked by its bits).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
