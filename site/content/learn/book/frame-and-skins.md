+++
title = "The frame and skins"
description = "A screen says what it shows as plain data, and a skin paints it; tests read the data."
weight = 40

[extra]
chapter = 4
+++

Graphics are a skin. A screen says *what* it shows, as plain data
called its **view**, and a **skin** says *how* that looks. Glyph art,
pixel art and bought sprite packs then swap without touching the rules,
and a test of what happened never breaks because a color changed.

## The view

The garden's view is built from the rules' state and the game's text,
and nothing else:

{{ snippet(file="src/frame.rs", anchor="view") }}

It names no glyph, no color and no position: those are the skin's.

## The frame

Every screen draws into one frame. Its first layer is a **console** of
glyph cells, `GlyphBuffer`, sized by the game (48 × 24 cells of 8 × 16
pixels on the short path, `BASIC_CONSOLE`; a game sets its own with
`BasicGame::console`). Over and under the cells go pictures from the
game's images (sprites, tiles, a backdrop), each with a layer that says
what it covers.

A glyph skin paints cells: `fill_rect`, `draw_box`, `print` (which
returns how many cells it used), `set`, and the prelude's
`print_centred`. Colors come from the palette by name, never as
numbers: `ctx.palette.engine(EngineColor::Text)` for the engine's own
roles, and `ctx.palette.lookup(name)` for one of the game's, from
`files/palette.ron`:

{{ snippet(file="files/palette.ron", anchor="own", language="ron") }}

## The skin

The garden's skin is one function of its view:

{{ snippet(file="src/frame.rs", anchor="skin") }}

A screen's `draw` builds the view and hands it to the skin. A second
skin of the same view (sprites in place of glyphs, a one-bit look for a
handheld) is a second function, picked by a setting, and nothing else
in the game changes. The grid kit's map (`shuyi::ui::grid`) works this
way: a scene of the visible tiles, painted by a glyph skin or a sprite
skin, with the game's units by a painter of its own; Hello, Grid shows
both.

## Why tests read the view

A test of what happened reads the view, or the state behind it:

{{ snippet(file="src/frame.rs", anchor="test_view") }}

Only a test of a look reads cells and colors. `h.snapshot()` is the
frame as text, the cells' glyphs and then their colors:

{{ snippet(file="src/frame.rs", anchor="test_look") }}

For a picture, `cargo shuyi frame-png garden.png --keys "Enter"`
paints the frame to a PNG with no window. Look at it, and keep the
tests on the view.
[Chapter 8](@/learn/book/testing.md#a-look) says how to keep a look
in a test.

## On a target's screen

Each target says how big its display is and how small its text may be,
and `cargo shuyi check --target` checks the game's frame and scale
against it ([chapter 9](@/learn/book/targets-and-export.md)). A one-bit
display, such as the Playdate's, takes each palette role's pattern from
the game's one-bit map.
