---
id: "2376"
title: "Parked: A console that grows by whole cells when the window's shape changes: an aspect policy like Godot's expand, from the game's console up to the bounds it sets, checked at the smallest and largest"
type: feature
milestone: "Engine: accessibility"
model: reasoning
effort: medium
status: blocked
blocked_by: ["2323", "2333"]
nick_input: none
completed:
---

# 2376 — A console that grows by whole cells to the window's shape

> **Parked** (Nick, 2026-10-14: "document the parked (i.e. after
> MVP/parity) tickets for networking, styles for later"): a follow-up
> of parked work is parked. It builds on 2323, which the game doesn't
> ask for and which waits until after the engine reaches parity with
> the game (the MVP in `docs/ROADMAP.md`), and on 2333. Nobody starts
> it until Nick unparks it, after 2323. Whoever unparks it sets
> `status: todo` and drops `Parked:` from the title.

## Context

Found by the 2026-10-20 comparison with Bevy 0.20 and Godot 4.7.
Godot's aspect policy `expand` keeps the base size as the smallest
view, and a window of another shape shows more of the scene instead of
bars
(docs.godotengine.org/en/stable/tutorials/rendering/multiple_resolutions.html).
2323's **Out** and 2333's **Out** both leave it to this ticket.

Shuyi draws in a console: `cols × rows` cells of `cell_w × cell_h`
logical pixels (`ConsoleSpec`, `crates/ui/src/frame/console.rs`). The
game picks one with `GameData::console()`, and the window shows it at
the scale 2333's `place` gives (`Integer` or `Fit`), with bars around
it. An 800 × 512 console (100 × 32 cells of 8 × 16) on a 1920 × 1080
screen draws at 2× in `Integer`, with bars of 160 pixels left and
right. In `Fit` the bars are 116 pixels. A text console can't stretch,
but it can grow by whole cells: the same cells, at the same scale, more
of them. There it becomes 120 × 33 cells in `Integer` (bars of 12
pixels, top and bottom) or 113 × 32 in `Fit` (bars of 7). Examples: a
roguelike that shows more of its map on a wide screen, a text
adventure whose log gets wider, a farming game's map view on a tall
phone-shaped browser window.

What it builds on (both are needed first):

- **2333:** `ScaleMode`, `Factor`, `scale`, `place` and `MAX_DRAW_PX`
  in `shuyi_ui::frame::console`, in integers only, which the window
  and `check --target` share; and `Frame::scale`, the mode the frame is
  drawn in, which `present_game` sets.
- **2323 (parked):** a new `GlyphBuffer` with a new spec when the
  console's size changes (there, for a new text size), the base console
  its text sizes divide, `harness::check_text_sizes`, and the guide's
  rule to lay screens out from `buf.cols()` and `buf.rows()`.

**Why `check --target` needs no change.** The console grows only at the
scale its smallest size gets (the property test below proves it). So
the text is the size the check works out from `shuyi.ron`'s `frame`,
which becomes the smallest console's frame, and that frame still has to
fit each display.

**From 2511 (done):** a game on the short path picks its console with
`BasicGame::console` (`crates/platform/src/basic.rs`); it gets
`BasicGame::aspect` beside it.

## Nick input

None.

## Scope

**In:**
- **`Aspect`** in `shuyi_ui::frame::console` (integers only, still
  `no_std`):
  ```rust
  #[non_exhaustive]
  pub enum Aspect {
      Keep,
      #[non_exhaustive]
      Expand { max_cols: u16, max_rows: u16 },
  }
  ```
  - `Keep` is the default (`#[derive(Default)]`): the console stays the
    game's size, with bars around it, as today;
  - `Expand` grows the console by whole cells to fill the window's
    shape, from the game's console up to `max_cols × max_rows` cells. A
    max below the game's console counts as the game's console. Build it
    with `Aspect::expand(max_cols, max_rows)` (ADR-0039: it may grow);
  - `Copy`, `Eq`, `Debug`. No serde: it is a `GameData` choice, not a
    file's.
- **`expand(spec, aspect, mode, window: (u32, u32)) -> ConsoleSpec`**,
  beside `place`:
  - `Keep`: `spec`;
  - `Expand`: the scale is `scale(mode, spec.px(), window)`, the
    smallest console's. A scale of 0 (an empty window, or one too
    small for `Integer`) gives `spec`. Otherwise `cols = window_w × den
    / (num × cell_w)` and `rows = window_h × den / (num × cell_h)`,
    rounded down, in `u64`. Each is clamped between `spec`'s own and
    the max, and to what `MAX_DRAW_PX` logical pixels hold, but never
    below `spec`'s own;
  - the answer is `spec.sized(cols, rows)`, so the cells keep their
    size.
- **The game's choice:**
  - `GameData::aspect(&self) -> Aspect`, by default `Aspect::Keep`, so
    a game that doesn't say draws the same frames. The rustdoc of
    `GameData::console` says that with `Expand` it is the smallest
    console;
  - `BasicGame::aspect(self, Aspect) -> Self` beside
    `BasicGame::console`, and the field it fills in `BasicData`
    (`crates/ui/src/basic.rs`), so a game on the short path opts in
    too;
  - the toy (`crates/ui/src/toy.rs`) gets a public field `aspect:
    Aspect`, `Keep` by default, so a test opts in and no toy snapshot
    changes.
- **The game follows the window** (`crates/ui/src/game.rs`):
  - `Game::fit_window(&mut self, window: (u32, u32), mode: ScaleMode)`:
    `expand` on the console the game started on (the base 2323 keeps)
    with `aspect()`. When the frame's cells change, it calls
    `set_frame_cells`;
  - `Game::set_frame_cells(&mut self, cols: u16, rows: u16)`: the
    logical frame becomes `cols × rows` of the game's cells, clamped to
    the aspect's bounds (nothing happens under `Keep`). When it
    changes, the buffer is made again with 2323's rebuild,
    `Ctx::console` becomes the new spec (if 2323's rebuild doesn't
    already set it), and the screens redraw (`Game::redraw`);
  - `Game::frame_cells(&self) -> (u16, u16)`;
  - one private function gives the buffer's spec from the frame's cells
    and the text size (2323's text size divides the grown frame as it
    divides the game's console). 2323's rebuild on a text size change
    calls it too, so a resize and a text size change give one answer in
    either order;
  - the window's size is a frame input, like the text size. No
    simulation reads it: turns and ticks never see the console, and a
    replay draws at the window it runs in.
- **`App` tells the game** (`crates/platform/src/app.rs`): in `load`,
  before the first `present_game`, and in `step`, before `game.frame`
  (beside `set_ticks`), it reads `platform.display()` and calls
  `fit_window` with its width and height and the frame's mode. One
  helper gives that mode to both `fit_window` and `present_game`'s
  `Frame::scale`, so the two can't differ.
- **Headless:** `HeadlessPlatform::set_display(&mut self, DisplayInfo)`,
  and `Harness::resize(&mut self, width: u32, height: u32)`, which sets
  it and runs one frame with no input and no time, so the next snapshot
  shows the new console. `Harness::on` fits the game to its platform's
  display before the first frame, as `load` does. The tools (`shuyi
  run`, the MCP session) run on `HeadlessPlatform::DISPLAY` (1280 ×
  720), so an `Expand` game's frames there show the console that window
  gets; the contract doesn't change.
- **`shuyi-app` needs nothing new:** `Renderer::draw` places
  `buf.spec()`, so it draws the grown console. Check with `grep` that
  every caller of the placement in `crates/app/` (the renderer, and any
  inverse of it for the pointer or the IME) takes the buffer's spec,
  never `GameData::console()`.
- **The check:** for a game whose aspect is `Expand`,
  `check_text_sizes` (2323) draws each named screen at every offered
  text size on four consoles, through `set_frame_cells`: the smallest
  (the game's), the widest (max columns, the game's rows), the tallest
  (the game's columns, max rows) and the largest (both max). The max is
  taken after the `MAX_DRAW_PX` cap. A finding names the console's size
  beside the screen, the text size and the text key. A `Keep` game is
  checked as before.
- **Docs:**
  - an ADR (the `write-adr` skill), amending 2333's: the console's size
    may change while a game runs, by whole cells only; the game's
    console is the smallest; the scale comes from the smallest console,
    so the text `check --target` measures stays true and the scale
    doesn't jump as the window grows by a pixel; screens lay out from
    the buffer's size; `Keep` stays the default;
  - in the guide where 2323 wrote "lay screens out from `buf.cols()`
    and `buf.rows()`" (the `document` skill says where), a paragraph on
    `Aspect::Expand`, its bounds and its check;
  - the templates' `shuyi.ron` comment: with `Aspect::Expand`, `frame`
    is the smallest console's;
  - `docs/platform-backends.md`: `App` reads `display()` every frame,
    so a backend reports a resized window at once.

**For the game:** additive. `GameData::aspect` has a default; `Aspect`
follows ADR-0039; the methods on `Game`, `BasicGame`,
`HeadlessPlatform` and `Harness` are new. A game that keeps `Keep`
draws the same frames, and the game's PR that moves the pin needs no
change. 2010's `api` job shows the new items.

**Out (do not do):**
- stretching, or a console that grows by part of a cell: cells stay
  whole, and the bars are what is left over (less than one cell);
- Godot's other aspect policies (`keep_width`, `keep_height`,
  `ignore`): the game's console fits whole in every window, which is
  what `expand` keeps;
- a player setting for the aspect: whether the screens can take more
  cells is the game's to know, so the game picks;
- a different aspect per target, or `shuyi.ron` naming it:
  `check --target` checks the smallest console, which every window
  shows at least;
- a window size option for the tools (`shuyi run`, the MCP `start`):
  they keep the headless 1280 × 720;
- hello-grid and the templates: they keep `Keep`, so their screens and
  snapshots don't change;
- changing the game's own screens to lay out from the buffer's size
  (the game's tickets, if it opts in);
- world views that show more of a world in a bigger frame: they draw
  in the rectangle a screen gives them, and the screen sizes it from
  the buffer.

## Implementation steps

1. Read 2323's and 2333's completion notes, then what they added: the
   rebuild of the buffer on a text size change and where it keeps the
   base console, `check_text_sizes`, `ScaleMode`, `Factor`, `scale`,
   `place`, `MAX_DRAW_PX`, `Frame::scale` and `present_game`. Then read
   `console.rs`, `game.rs` (`with_stack`, `frame`, `redraw`), `ctx.rs`
   (`GameData`, `Ctx::console`), `app.rs` (`load`, `step`,
   `present_game`), `headless.rs`, `harness.rs`, both `basic.rs`,
   `toy.rs` and `crates/app/src/render.rs`.
2. Write `Aspect` and `expand` in `console.rs`, with the table test and
   the property tests below.
3. Add `GameData::aspect`, `BasicGame::aspect` and the toy's field.
4. Add `Game::fit_window`, `set_frame_cells` and `frame_cells`, with
   the shared spec function, `Ctx::console` following.
5. Call `fit_window` from `App`'s `load` and `step`, with the shared
   mode helper.
6. Add `HeadlessPlatform::set_display`, `Harness::resize` and the fit
   in `Harness::on`, then the toy's harness tests.
7. Run `check_text_sizes` on the four consoles, with its tests.
8. Opt hello-grid into `Aspect::Expand` with a local change that isn't
   committed. Run it at 1280 × 800 and 1920 × 1080 in both modes, on
   the desktop and in the web build, drag the window to a new shape,
   and put screenshots in the PR.
9. Count the changed lines (about 600, fixtures and snapshots not
   counted). Write the ADR and the doc lines. Run the gates.

## Acceptance criteria

- [ ] `expand` gives these for an 800 × 512 console (100 × 32 cells of
      8 × 16) with `Aspect::expand(200, 64)`, and `place` puts each
      grown console where the last column says (a table test):

  | Window | Mode | Console | Placed |
  | ------ | ---- | ------- | ------ |
  | 1280 × 800 | `Integer` | 160 × 50 | 1×: 1280 × 800 at (0, 0) |
  | 1366 × 768 | `Integer` | 170 × 48 | 1×: 1360 × 768 at (3, 0) |
  | 1920 × 1080 | `Integer` | 120 × 33 | 2×: 1920 × 1056 at (0, 12) |
  | 3840 × 1080 | `Integer` | 200 × 33 (max columns) | 2×: 3200 × 1056 at (320, 12) |
  | 800 × 1280 | `Integer` | 100 × 64 (max rows) | 1×: 800 × 1024 at (0, 128) |
  | 640 × 400 | `Integer` | 100 × 32 (too small) | 1×: 800 × 512 at (-80, -56) |
  | 1280 × 800 | `Fit` | 102 × 32 | 1275 × 800 at (2, 0) |
  | 1920 × 1080 | `Fit` | 113 × 32 | 1906 × 1080 at (7, 0) |

  Also: `Aspect::Keep` gives 100 × 32 on every row. A 40 × 20 console
  of 8 × 16 with `Aspect::expand(u16::MAX, u16::MAX)` in `Fit` on
  8000 × 400 is capped at 512 × 20 (4096 pixels wide).
- [ ] The toy with `aspect: Aspect::expand(80, 40)`, in its default
      `Fit` (2333), on the headless 1280 × 720 display draws a 71 × 20
      console from its first frame. After `Harness::resize(320, 640)`
      it draws 40 × 40, and after `Harness::resize(320, 320)` 40 × 20
      again. `Ctx::console` and `buf.spec()` agree each time.
- [ ] `check_text_sizes` on that toy draws each screen on the four
      consoles at every offered text size. A test screen that writes
      on row 30 passes on the tallest and the largest consoles at the
      first text size, and fails on the smallest, naming the screen,
      the text size and 40 × 20.
- [ ] A game that keeps `Aspect::Keep` draws the same frames: every
      existing snapshot is unchanged.
- [ ] The tests in `crates/cli/src/contract/target/tests.rs` pass
      unchanged.
- [ ] The PR has screenshots of hello-grid (opted in locally) at
      1280 × 800 and 1920 × 1080 in both modes, on the desktop and in
      the web build, and after a drag to a new shape.
- [ ] `shuyi-ui` still builds for `thumbv7em-none-eabihf`.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the table above; `Keep`; the `MAX_DRAW_PX` cap; a scale of 0
  in both modes (a 0 × 0 window, and one too small for `Integer`); a
  max below the game's console; `Aspect::default()` is `Keep`;
  `set_frame_cells` clamping, and doing nothing under `Keep`;
  `BasicGame::aspect` reaching `GameData::aspect`.
- Property (proptest; consoles of 1 to 200 cells a side, cells of 1 to
  32 pixels, windows of 0 to 8000 pixels, both modes, any max):
  - the grown console lies between the game's and the bounds;
  - `place` gives the grown console the same scale as the game's
    console (the same fraction, compared by cross-multiplying);
  - unless a bound or the cap stopped it, one more column would not
    fit at that scale, in exact fractions (`(cols + 1) × cell_w × num
    > window_w × den`), and likewise one more row;
  - the grown frame stays within `MAX_DRAW_PX` on each side, unless
    the game's own console is bigger.
- Integration: the toy's `Harness` tests above; an `App` test in
  `crates/platform/src/app/tests.rs` on a headless platform whose
  display changes between frames, where the presented buffer follows;
  the check on the four consoles and its failure.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
