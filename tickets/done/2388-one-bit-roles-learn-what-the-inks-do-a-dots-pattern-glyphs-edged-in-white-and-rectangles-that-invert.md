---
id: "2388"
title: "One-bit roles learn what the inks do: a Dots pattern, glyphs edged in white, and rectangles that invert"
type: feature
milestone: MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-06
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

The game's device build waits for this ticket:
[game#1170](https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1170-the-playdate-front-end-on-the-engines-backend.md)
moves it onto `shuyi-playdate`, whose display paints with a
`PatternMap` only (`OneBitLook::map`), so the inks' dots, edges and
inverted rectangles would be lost there. 1170 hands the backend the
map below for its device build. The game's ticket this PR writes moves
what still calls `render` after that: its tool pictures (`frame-png`'s
one-bit frames) and its tests.

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

- [x] `Dots` has the ink's pixels exactly (test over all 64).
- [x] A pattern map with `edge: true` draws a glyph with a white edge,
      and one with `invert: true` flips what a rectangle covers (tests
      through `to_one_bit`).
- [x] The proof test passes: the map above draws every random ink-only
      buffer exactly as `InkShades` does.
- [x] `check` refuses a text role with `invert`, naming the role.
- [x] Old pattern maps read the same (`edge` and `invert` default to
      `false`; an existing map's test still passes).
- [x] The game's ticket exists in the game repository, and 2389's
      `blocked_by` names it.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `Dots`' rows; `Inverted(Dots)`; reading `edge` and `invert`;
  the default; the text-role check; `PatternMap`'s `glyph`, `shape` and
  `ground` with each.
- Property: the proof test (inks against the map, random buffers).
- Snapshot: none (a one-bit frame is checked by its bits).

## Completion notes

`Base::Dots` joins the patterns (black where `y` is even and
`x % 4 == y % 4`, the ink's pixels: a test over all 64 against
`ink_is_white(Ink::Dots, …)`). `RolePatterns` gains `edge` and `invert`
(both `#[serde(default)]`), becomes `#[non_exhaustive]`, and gets
`with_edge`, `with_invert` and `of(fg)` beside `new`. `PatternMap` sets
`edged` on a role's glyph shade and `flips` on its shape shade; its
ground has neither. `check` makes a text role with `invert` an error
(`Rule::Text`, naming the role). The proof
(`the_inks_pattern_map_draws_as_the_inks_do`, in
`crates/ui/src/frame/onebit/tests.rs`) draws random ink-only buffers
with `render` and with `render_with` the map, byte for byte equal; the
speed tests' generators take a count of colours for it (`ink_buffer`).
ADR-0089 amends ADR-0026; the pattern map's fields are in
`shuyi_assets::onebit`'s module docs.

Deviations:

- **`dashes` is `Checker2`, not `Inverted(Checker2)`.** The ticket's
  table was wrong: the `Dashes` ink is black where `x / 2 % 2 == y / 2 %
  2`, which is `Checker2` exactly; its inverse is the same pattern moved
  two pixels. The proof's map, the game's ticket and 2389 say
  `Checker2`.
- **`edged` and `flips` are not set on a ground.** The renderer reads
  `edged` only off a glyph's shade and `flips` only off a rectangle's,
  and the proof passes without them.
- **No new `Rule` for an inverting text role:** it is `Rule::Text` with
  `Severity::Error`, so the check's API doesn't break a second time.
- **Roles of one colour must also agree on `edge` and `invert`**
  (`Rule::SharedColour`), as they must on their patterns: the frame
  can't tell them apart, so the renderer would draw both as the first.
- **`RolePatterns::of(fg)`** added beside the ticket's builders: a map
  with no `bg` couldn't be built outside the crate once the struct is
  `#[non_exhaustive]`, and the game builds one (below).
- **The game does build `RolePatterns`**, which the ticket said it
  didn't: `crates/xtask/src/game_tools/one_bit.rs` writes
  `RolePatterns { fg, bg: None }`. Its pin bump changes that to
  `RolePatterns::of(fg)` (game#1251, step 1).

The game's ticket is **game#1251** ("The Playdate build draws with a
pattern map instead of inks", Zafnok/visions-of-shuyi#567), in 2389's
`blocked_by`. Its PR moves the pin to this change.

Breaking: shuyi-assets, shuyi-ui: onebit::RolePatterns: `#[non_exhaustive]` with two new fields, `edge` and `invert`; build one with `RolePatterns::new(fg, bg)` or `RolePatterns::of(fg)`.

```migration
since: 0.2.0
what: A role's patterns are built by a constructor, since they gained edge and invert.
old: RolePatterns { fg, bg }
new: RolePatterns::new(fg, bg), RolePatterns::of(fg), .with_edge(bool), .with_invert(bool)
before: |
  let patterns = RolePatterns { fg, bg: None };
after: |
  let patterns = RolePatterns::of(fg);
find: ["RolePatterns { fg"]
```

Breaking: shuyi-assets, shuyi-ui: onebit::Base: a new variant, `Dots`, and `Base::ALL` holds 11; a `match` on `Base` adds an arm.

```migration
since: 0.2.0
what: The patterns gain Dots, one pixel in eight.
old: "Base::ALL: [Base; 10]"
new: "Base::ALL: [Base; 11], Base::Dots"
before: |
  let name = match base {
      Base::Checker2 => "checker",
      _ => "other",
  };
after: |
  let name = match base {
      Base::Checker2 => "checker",
      Base::Dots => "dots",
      _ => "other",
  };
```

Breaking: shuyi-assets: formats.ron "pattern map: pattern-map": a pattern map's state hash moves, as it holds `edge` and `invert`; nothing for a game to change.

Breaking: shuyi-assets: pack::VERSION: 10, as a pattern map's binary form gains `edge` and `invert` and `Dots`; a game bakes its pack again.

```migration
since: 0.2.0
what: Packs are format version 10, so a pack baked before is refused.
old: pack::VERSION 9
new: pack::VERSION 10
before: |
  cargo shuyi bake
after: |
  cargo shuyi bake
```

Book: none, the book's chapter 4 names the one-bit map in one sentence and teaches none of its fields; they are in the API docs of `shuyi_assets::onebit`.
