+++
title = "The frame and skins"
description = "A screen says what it shows as plain data, and a skin paints it; tests read the data."
weight = 20

[extra]
part = "chapters"
chapter = 2
+++

{% newcomer(to="shuyi", topic="why the look is kept apart") %}
What's on the screen and how it looks are two different questions. "The second plot
has a sprout in it" is one. "A sprout is a green triangle" is another.

If one piece of code answers both, you can't change the look without risking the game,
and a test of the game breaks whenever a color changes.
{% end %}

The engine keeps what's on the screen apart from how it looks. **Graphics are a skin.** A screen says *what* it shows, as plain data called its
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

{% newcomer(to="rust", topic="an enum and `usize`") %}
`enum Plot` is one of a fixed list, `Soil` or `Sprout`. `usize` is a whole
number, never negative, of the kind used to count a list's items.
{% end %}

{% newcomer(to="rust", topic="an iterator chain") %}
`plots.map(|&grows| ...).collect()` is an
[iterator chain](@/learn/book/box-pusher/03-walk.md#the-rules): go through each plot, turn
each into a `Plot`, and gather them into a list. `|&grows|` is a small nameless
function whose one argument is `grows`, and the `&` takes the value out of the
reference it comes as. In Rust, `if` gives back a value, so `if grows { Plot::Sprout }
else { Plot::Soil }` is one `Plot`.
{% end %}

{% newcomer(to="rust", topic="`match` on an `Option`") %}
`match garden.outcome()` opens an `Option`: `None` while the game goes on, or
`Some(...)` with how it ended. It picks which line of text to show.
{% end %}

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

{% newcomer(to="programming", topic="a hex color") %}
A color like `#7ad07a` is three pairs of digits in base 16, for how much red, green and
blue it has, from `00` to `ff`.
{% end %}

## The skin

The garden's skin is one function of its view:

{{ snippet(file="src/frame.rs", anchor="skin") }}

{% newcomer(to="rust", topic="a tuple") %}
`fg` and `bg` are the cell's foreground and background colors. `let
(glyph, fg) = match plot { ... }` takes a pair apart: each arm gives back a glyph and
a color, side by side, as a **tuple**, and this names both.
{% end %}

{% newcomer(to="rust", topic="a range, `step_by` and `zip`") %}
`(left..).step_by(4)` counts up from `left` in fours, forever, and `.zip(&view.plots)`
pairs each number with a plot, stopping when the plots run out. So `x` and `plot` go
along together. `..` makes a **range**.
{% end %}

{% newcomer(to="rust", topic="`i32`, `usize` and `try_from`") %}
The cell positions are `i32`, which may go negative, as something can be drawn partly
off the screen. A list's length is a `usize`. Rust won't mix number types without
being told, so `i32::try_from(view.at)` converts, and `unwrap_or(0)` says what to use
if it can't.
{% end %}

There isn't a single fractional number here: cells are counted, and everything lands
on the same place on every machine.

A screen's `draw` builds the view and hands it to the skin. Since the view says nothing
of how it looks, changing the look changes one line of the skin:

{{ try_it(name="book-skin-glyph") }}

Nothing else in the game knows. And a color doesn't need code at all:

{{ try_it(name="book-sprout-blue") }}

A second skin of the same view (sprites in place of glyphs, a one-bit look for a
handheld) is a second function, picked by a setting, and nothing else in the game
changes. The grid kit's map (`shuyi::ui::grid`) works this way: a **scene** of the
visible tiles, painted by a glyph skin or a sprite skin, with the game's units by a
painter of its own. Hello, Grid shows both. Here is the same view of its stage, the same
turn, painted by the glyph skin and by the sprite skin, which its options switch between:

<div class="figure-pair">

{{ shot(example="hello-grid", name="stage", alt="Hello, Grid's stage in glyphs: walls drawn as shaded blocks, the floor as dots, the goal as square brackets in the top right, your piece as an at sign and the foe as an ampersand", caption="The glyph skin") }}

{{ shot(example="hello-grid", name="tiles", alt="The same stage in tiles: each wall and floor cell a square picture from the tileset, the goal, your piece and the foe in the same cells as in glyphs", caption="The sprite skin") }}

</div>

The sprite skin paints the ground from a tileset's terrain tiles: a picture for each
terrain, and layers over them (a shore round a pond, a bridge that turns to cross the
water) whose pictures it picks from the tiles around, so a map names its terrain and no more.
A game writes them in a `terrain-tiles` file, reads it with `TerrainTilesRon`, and makes
the skin's tiles with `TerrainTilesDef::resolve` once its pictures and terrains are
known. The formats page lists [what the file is checked
for](https://github.com/Zafnok/shuyi-engine/blob/main/docs/formats.md#the-built-in-formats).

A few things no terrain says, a lamp at a gate or a signpost at a crossing, a map
places by hand in its file's `look`:
`look: (props: { (19, 5): "lamp" })`. The tileset's look says what each name is drawn as
(`Look::props`), and the sprite skin stands it on its tile, so a picture taller than a
tile reaches up over the one above. They are only a look: the rules never see them, and
the glyph skin draws none.

## A picture the skin paints

Pictures from the game's image files are made before the game runs. Some
pictures can't be: a video's frames, an old console's screen in an
emulator, a minimap of what the player has seen, a moving plasma. Their
**pixels** (the dots a picture is made of) are made while the game runs.
A skin paints such a picture itself, as a `Painted` picture, and places
it in the frame like any other:

{{ snippet(file="src/frame.rs", anchor="plasma") }}

The frame keeps its pictures from one frame to the next, by name. Each
has a *revision*, a count that goes up whenever its pixels are written,
and the window sends a picture to the graphics card again only when its
revision moved. `update` puts the picture in only if it differs from the
one already there, so a plasma painted afresh at every frame costs
nothing in the frames where it didn't change.

{% newcomer(to="rust", topic="chunks, `zip` and `abs_diff`") %}
`as_chunks_mut::<4>()` cuts the pixels' bytes into groups of
four, red, green, blue and how solid the pixel is (255 is fully), and
`.0` takes the whole groups. `.chunks_mut(64)` cuts those into rows of
64 pixels, and `(0u32..).zip(rows)` numbers the rows, so `py` is a
pixel's row; the inner loop numbers each row's pixels, its column `px`.
`abs_diff` is how far apart two numbers are, never negative.
{% end %}

A picture comes in three kinds: `Painted::rgba` (four bytes a pixel, as
here), `Painted::indexed` (a byte a pixel and a palette of up to 256
colors, so changing a color changes every pixel of it at once) and
`Painted::one_bit` (black and white). It's drawn without smoothing, at any
size, and on a black-and-white display a colored picture is turned to a
pattern of black and white dots as it's drawn. The test reads the
picture's revision, not its pixels:

{{ snippet(file="src/frame.rs", anchor="test_plasma") }}

A **sprite atlas** is many small pictures packed onto a few big ones, its
*pages*, so the graphics card holds one picture for many sprites. A file
`atlas.ron` beside your sprites packs every picture of their folder as the
game loads (`(from: "sprites/", page_size: 1024, padding: 1)`). Its pages
go in the frame as painted pictures with `shuyi::ui::frame::atlas::show`,
and `atlas::sprite` places one of its sprites. A picture made while the
game runs (a player's avatar) goes in with `atlas::insert`: it finds free
space on a page, or starts a new one, and only that page is sent to the
graphics card again.

## Why tests read the view

A test of what happened reads the view, or the state behind it:

{{ snippet(file="src/frame.rs", anchor="test_view") }}

`h.screen::<GardenScreen>()` asks the stack for the screen of that type, and
gives back an `Option`, as it might not be there.

{% newcomer(to="rust", topic="`unwrap`") %}
`.unwrap()` opens it, and stops the test if it's `None`. That's fine in a test, and
in a game you'd handle the `None`.
{% end %}

Only a test of a look reads cells and colors. `h.snapshot()` is the frame as text, the
cells' glyphs and then their colors:

{{ snippet(file="src/frame.rs", anchor="test_look") }}

{% newcomer(to="rust", topic="`assert!` with a message") %}
`assert!` fails the test unless its first argument is true, and the rest is
what to print if so, with `{frame}` filled in by the value of `frame`.
{% end %}

Change bare soil's glyph from `·`, and the second test breaks, as it should, since the
look is what it's about. The first test doesn't notice. That's the whole reason for the
view.

For a picture, `cargo shuyi frame-png garden.png --keys "Enter"` paints the frame to a
PNG with no window. Every picture in this book is made that way. Look at it, and keep
the tests on the view. [Chapter 8](@/learn/book/testing.md#a-look) says how to keep a
look in a test.

## Things that move

A sprout that pops up, a menu that slides in, a title that bobs: the skin shows a change
over time, and the rules never wait for it. The engine draws such a motion from *the
time alone*. A **tween** is a value going from one end to another over a time, along an
**ease curve** that says how it speeds up and slows down, and its value at a moment is
worked out from how long it has run. Nothing steps it along from the frame before, so
it looks the same at any frame rate, and a test can ask for any moment:

{{ snippet(file="src/frame.rs", anchor="tween") }}

{% newcomer(to="rust", topic="`const` and `const fn`") %}
`const` makes a value the program has from the start, built by `const fn`s
such as `Tween::new`.
{% end %}

`Tween<i32>` is a tween of whole numbers; a tween of a pair, a color or a number of
your own type works the same way.

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

{% newcomer(to="rust", topic="`windows`, `filter` and `count`") %}
`frames.windows(2)` goes through each pair of neighbors in the list, here
one frame's time and the next. `.filter(...)` keeps the pairs where the sprout landed,
and `.count()` counts them.
{% end %}

`finished_between` is how a screen hears "it landed": given the time of the frame
before and this one's, it is true once, so the screen plays its sound or moves on
without keeping a flag.

## Seeing what the game sees

A debug build can draw what your logic works with over the running game: where the map's
cells are, which part of it is in view, where a box collides. This is the **debug view**.
Each thing it can show is a **channel**, turned on and off from the debug menu's **Debug
view**, which lists every channel the screens running offer; the choice is kept for the
next launch. A build for players draws none of it and asks no screen for it.

A screen offers a channel in `debug_draw`, a method of `Screen` that does nothing unless
you write it. It hands in **gizmos**: rectangles, lines, crosses, circles and labels, in
console pixels, each with a **role** that says what it shows (bounds, solid, trigger,
path, bone, note). The engine picks their colors from the role, so a gizmo never names a
color, and `debug_draw` only reads the screen (`&self`), so looking never changes the
game. The garden's own channel boxes each plot:

{{ snippet(file="src/frame.rs", anchor="gizmos") }}

The grid kit offers its own channel, `grid`: a map screen calls
`shuyi::ui::grid::debug::layout_gizmos` from its `debug_draw`, and the cell lines, the
map's edges, the cursor's cell and each unit's appear. Hello, Grid's stage and the stage
of a `shuyi new --template grid` game offer it already. A test reads the gizmos as it
reads a view, never their pixels:

{{ snippet(file="src/frame.rs", anchor="test_gizmos") }}

To see the same as a picture, `cargo shuyi frame-png out.png --debug plots` turns the
channels on from the first frame (`--debug all` turns on every one).

### Stopping on the frame that goes wrong

A debug build can also pause the game, step it one update at a time and slow it down.
Give two of your actions the engine's `UiAction::DebugPause` and `UiAction::DebugStep`
in your action set's `ui`, like your Debug action, and give them keys in `keymap.ron`
(they are keyboard only, and no player can rebind them). The pause key stops every screen
of the game: nothing updates, and the keys pressed are dropped, but the debug menu still
opens and works. Each press of the step key runs one update with the keys pressed with
it, as one frame at 60 a second, so a game stepped through ends where it would have
played straight. The debug menu's **Debug view** sets slow motion: one update in 2, 4 or
8 frames, every key pressed in between kept for the next.

The **readout** channel prints a few lines about the top screen in the top-left corner,
`paused` first while the game is paused. A screen writes them in `debug_readout`, another
method of `Screen` that gives none unless you write it: `turn 3, cursor 4,7` for a grid
game, `on ground, vx 1.5` for a platformer. Like `debug_draw`, it only reads the screen.

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
`&self` ([chapter 6](@/learn/book/games-and-screens.md#when-it-goes-wrong)).

## On a target's screen

A **target** is a kind of machine or place a game runs. Each target says how big its
display is and how small its text may be, and `cargo shuyi check --target` checks the
game's frame and scale against it ([chapter 9](@/learn/book/targets-and-export.md)). A
one-bit display, such as the Playdate's, has only black and white, and takes each
palette role's pattern from the game's one-bit map.

## Check yourself

{% check() %}
You change bare soil's glyph from `·` to `.` in the skin. Which test breaks?

- [x] Only the test of the look, which reads `h.snapshot()`. — A test of what happened reads the view, and the view names no glyph ([why tests read the view](#why-tests-read-the-view)).
- [ ] Both, since the view's plots are drawn with that glyph. — The view holds `Plot::Soil`, and only the skin knows it's drawn as `·` ([the view](#the-view)).
- [ ] Neither, since tests never read cells. — A test of a look does, and `the_garden_looks_like_a_row_of_plots` looks for the `·` ([why tests read the view](#why-tests-read-the-view)).
{% end %}

{% check() %}
The skin gets a color with `ctx.palette.lookup(name).unwrap_or(c(EngineColor::Text))`.
`palette.ron` spells the game's color `sprot`. What happens?

- [ ] `cargo shuyi check` reports the missing color. — It catches a missing or bad color of the engine's, not a name of the game's own that nothing defines ([when it goes wrong](#when-it-goes-wrong)).
- [x] The sprouts are drawn in the engine's text color, and no error is shown. — `lookup` gives `None` for a name that isn't there, and `unwrap_or` falls back to the text color ([when it goes wrong](#when-it-goes-wrong)).
- [ ] The game stops with an error when it draws the garden. — `lookup` gives an `Option`, so a missing name is handled, not a crash ([when it goes wrong](#when-it-goes-wrong)).
{% end %}

{% check() %}
`sprout_offset(ms)` is `SPROUT_RISE.value_at(ms)`, a tween of 300 milliseconds. The game
drops from 60 frames a second to 30. What happens to the sprout's rise?

- [ ] It takes twice as long, as it moves once a frame. — Nothing steps a tween along from the frame before ([things that move](#things-that-move)).
- [ ] It lands at a slightly different place, as the curve rounds differently. — The curves are worked out in whole numbers, so they are the same on every machine and at every rate ([things that move](#things-that-move)).
- [x] It still takes 300 milliseconds and is at the same place at each moment, shown in fewer frames. — Its value is worked out from the time alone, so it looks the same at any frame rate ([things that move](#things-that-move)).
{% end %}
