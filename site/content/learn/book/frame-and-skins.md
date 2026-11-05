+++
title = "The frame and skins"
description = "A screen says what it shows as plain data, and a skin paints it; tests read the data."
weight = 40

[extra]
chapter = 4
+++

What's on the screen and how it looks are two different questions. "The second plot
has a sprout in it" is one. "A sprout is a green triangle" is another.

If one piece of code answers both, you can't change the look without risking the game,
and a test of the game breaks whenever a color changes. So the engine keeps them apart.
**Graphics are a skin.** A screen says *what* it shows, as plain data called its
**view**, and a **skin** says *how* that looks. Glyph art, pixel art and bought sprite
packs then swap without touching the rules, and a test of what happened never breaks
because a color changed.

The whole chain is short: the rules' **state** (everything that's true now) gives the
**view**, and the **skin** paints the view into the **frame**.

{{ try_it(name="book-garden") }}

## The view

The garden's view is built from the rules' state and the game's text, and nothing else:

{{ snippet(file="src/frame.rs", anchor="view") }}

It names no glyph, no color and no position: those are the skin's.

> **Rust:** `enum Plot` is one of a fixed list, `Soil` or `Sprout`. `usize` is a whole
> number, never negative, of the kind used to count a list's items.
>
> `plots.map(|&grows| ...).collect()` is an
> [iterator chain](@/learn/tutorial/03-walk.md#the-rules): go through each plot, turn
> each into a `Plot`, and gather them into a list. `|&grows|` is a small nameless
> function whose one argument is `grows`, and the `&` takes the value out of the
> reference it comes as. In Rust, `if` gives back a value, so `if grows { Plot::Sprout }
> else { Plot::Soil }` is one `Plot`.
>
> `match garden.outcome()` opens an `Option`: `None` while the game goes on, or
> `Some(...)` with how it ended. It picks which line of text to show.

## The frame

Every screen draws into one **frame**, the picture the player sees. Its first layer is
a **console**, a grid of **cells** like an old terminal, each holding one **glyph** (a
character) with its colors. That grid is a `GlyphBuffer`, sized by the game: 48 × 24
cells of 8 × 16 pixels on the short path, `BASIC_CONSOLE`, and a game sets its own with
`BasicGame::console`. Over and under the cells go pictures from the game's images
(sprites, tiles, a backdrop), each with a layer that says what it covers.

A glyph skin paints cells: `fill_rect`, `draw_box`, `print` (which returns how many
cells it used), `set`, and the prelude's `print_centred`. Colors come from the palette
by name, never as numbers: `ctx.palette.engine(EngineColor::Text)` for the engine's own
roles, and `ctx.palette.lookup(name)` for one of the game's, from `files/palette.ron`:

{{ snippet(file="files/palette.ron", anchor="own", language="ron") }}

A color like `#7ad07a` is three pairs of digits in base 16, for how much red, green and
blue it has, from `00` to `ff`.

## The skin

The garden's skin is one function of its view:

{{ snippet(file="src/frame.rs", anchor="skin") }}

> **Rust:** `fg` and `bg` are the cell's foreground and background colors. `let
> (glyph, fg) = match plot { ... }` takes a pair apart: each arm gives back a glyph and
> a color, side by side, as a **tuple**, and this names both.
>
> `(left..).step_by(4)` counts up from `left` in fours, forever, and `.zip(&view.plots)`
> pairs each number with a plot, stopping when the plots run out. So `x` and `plot` go
> along together. `..` makes a **range**.
>
> The cell positions are `i32`, which may go negative, as something can be drawn partly
> off the screen. A list's length is a `usize`. Rust won't mix number types without
> being told, so `i32::try_from(view.at)` converts, and `unwrap_or(0)` says what to use
> if it can't.
>
> There isn't a single fractional number here: cells are counted, and everything lands
> on the same place on every machine.

A screen's `draw` builds the view and hands it to the skin. Since the view says nothing
of how it looks, changing the look changes one line of the skin:

{{ try_it(name="book-skin-glyph") }}

Nothing else in the game knows. And a color doesn't need code at all:

{{ try_it(name="book-sprout-blue") }}

A second skin of the same view (sprites in place of glyphs, a one-bit look for a
handheld) is a second function, picked by a setting, and nothing else in the game
changes. The grid kit's map (`shuyi::ui::grid`) works this way: a **scene** of the
visible tiles, painted by a glyph skin or a sprite skin, with the game's units by a
painter of its own. Hello, Grid shows both.

## A picture the skin paints

Pictures from the game's image files are made before the game runs. Some
pictures can't be: a video's frames, an old console's screen in an
emulator, a minimap of what the player has seen, a moving plasma. Their
**pixels** (the dots a picture is made of) are made while the game runs.
A skin paints such a picture itself, as a `Painted` picture, and places
it in the frame like any other:

{{ snippet(file="src/frame.rs", anchor="plasma") }}

The frame keeps its pictures from one frame to the next, by name. Each
has a **revision**, a count that goes up whenever its pixels are written,
and the window sends a picture to the graphics card again only when its
revision moved. `update` puts the picture in only if it differs from the
one already there, so a plasma painted afresh at every frame costs
nothing in the frames where it didn't change.

> **Rust:** `as_chunks_mut::<4>()` cuts the pixels' bytes into groups of
> four, red, green, blue and how solid the pixel is (255 is fully), and
> `.0` takes the whole groups. `.chunks_mut(64)` cuts those into rows of
> 64 pixels, and `(0u32..).zip(rows)` numbers the rows, so `py` is a
> pixel's row; the inner loop numbers each row's pixels, its column `px`.
> `abs_diff` is how far apart two numbers are, never negative.

A picture comes in three kinds: `Painted::rgba` (four bytes a pixel, as
here), `Painted::indexed` (a byte a pixel and a palette of up to 256
colors, so changing a color changes every pixel of it at once) and
`Painted::one_bit` (black and white). It's drawn without smoothing, at any
size, and on a black-and-white display a colored picture is turned to a
pattern of black and white dots as it's drawn. The test reads the
picture's revision, not its pixels:

{{ snippet(file="src/frame.rs", anchor="test_plasma") }}

## Why tests read the view

A test of what happened reads the view, or the state behind it:

{{ snippet(file="src/frame.rs", anchor="test_view") }}

> **Rust:** `h.screen::<GardenScreen>()` asks the stack for the screen of that type, and
> gives back an `Option`, as it might not be there. `.unwrap()` opens it, and stops the
> test if it's `None`. That's fine in a test, and in a game you'd handle the `None`.

Only a test of a look reads cells and colors. `h.snapshot()` is the frame as text, the
cells' glyphs and then their colors:

{{ snippet(file="src/frame.rs", anchor="test_look") }}

> **Rust:** `assert!` fails the test unless its first argument is true, and the rest is
> what to print if so, with `{frame}` filled in by the value of `frame`.

Change bare soil's glyph from `·`, and the second test breaks, as it should, since the
look is what it's about. The first test doesn't notice. That's the whole reason for the
view.

For a picture, `cargo shuyi frame-png garden.png --keys "Enter"` paints the frame to a
PNG with no window. Every picture in this book is made that way. Look at it, and keep
the tests on the view. [Chapter 8](@/learn/book/testing.md#a-look) says how to keep a
look in a test.

## Things that move

A sprout that pops up, a menu that slides in, a title that bobs: the skin shows a change
over time, and the rules never wait for it. The engine draws such a motion from **the
time alone**. A **tween** is a value going from one end to another over a time, along an
**ease curve** that says how it speeds up and slows down, and its value at a moment is
worked out from how long it has run. Nothing steps it along from the frame before, so
it looks the same at any frame rate, and a test can ask for any moment:

{{ snippet(file="src/frame.rs", anchor="tween") }}

> **Rust:** `const` makes a value the program has from the start, built by `const fn`s
> such as `Tween::new`. `Tween<i32>` is a tween of whole numbers; a tween of a pair, a
> color or a number of your own type works the same way.

The curves have the names other engines and websites give them, after Robert Penner's
equations: `Ease::Out(Family::Back)` is `easeOutBack` on easings.net, and Godot's
`TRANS_BACK` with `EASE_OUT`. `Back` goes a little past its end and comes back; `Linear`
moves as the time does, `InOut(Sine)` starts and stops gently, `Out(Bounce)` bounces.
The API docs of `shuyi::core::ease` list them all, beside other engines' names. In a
RON file a tween is data, like everything else: `(from: 8, to: 0, duration: 300, ease:
Out(Back))`.

The time is a whole number in the unit you choose. A screen counts milliseconds: it
keeps the moment its sprout was planted, and draws it at `sprout_offset(now -
planted)`. The rules count steps or turns. The curves are worked out in whole numbers,
never fractions, so they are the same on every machine, and the rules may use them too.

A test reads the motion at any moment, and asks when it ended:

{{ snippet(file="src/frame.rs", anchor="test_tween") }}

> **Rust:** `frames.windows(2)` goes through each pair of neighbors in the list, here
> one frame's time and the next. `.filter(...)` keeps the pairs where the sprout landed,
> and `.count()` counts them.

`finished_between` is how a screen hears "it landed": given the time of the frame
before and this one's, it is true once, so the screen plays its sound or moves on
without keeping a flag.

## When it goes wrong

**A glyph the font doesn't have.** The font is a picture of the characters it knows, in
`files/font/`. A character outside it can't be drawn as itself, and nothing stops the
game: the window only notes it once in its log. So look at a new glyph in a picture.

**A color name that isn't there.** `ctx.palette.lookup(name)` gives an `Option`, as a
name may be missing, and the skin above falls back to the engine's text color. A typo
in a name shows as a sprout in the wrong color, not as an error. A missing color of the
engine's, or a bad one, is caught by `cargo shuyi check`:

```text
check: 1 error(s)
  files/palette.ron: missing required colour "focus"
```

```text
check: 1 error(s)
  files/palette.ron:20: colour "sprout": "green" must start with '#' (format "#RRGGBB")
```

**Trying to change the game in `draw`.** The compiler stops you, as `draw` only has
`&self` ([chapter 2](@/learn/book/games-and-screens.md#when-it-goes-wrong)).

## On a target's screen

A **target** is a kind of machine or place a game runs. Each target says how big its
display is and how small its text may be, and `cargo shuyi check --target` checks the
game's frame and scale against it ([chapter 9](@/learn/book/targets-and-export.md)). A
one-bit display, such as the Playdate's, has only black and white, and takes each
palette role's pattern from the game's one-bit map.
