---
id: "3221"
title: A steady frame of Hello, Grid with key caps on allocates no more than a count its test pins
type: infra
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 3221 — A steady frame of Hello, Grid with key caps on allocates no more than a count its test pins

## Context

The retro of PRs #379–#484 (T23) found that the sound mixer has a test
that its render allocates nothing (`crates/sound/tests/mix_alloc.rs`,
over `shuyi-alloc-count`'s global counter), and the frame has none.
Nothing measures what a frame of a game allocates once it is steady,
so a change that adds an allocation to every frame goes unseen until a
game's bench catches it, as 3218's text fill did (3219). Key caps
(3215, ADR-0124) are the newest per-frame drawing, and the game turns
them on in every hint next (game#0254,
`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/0254-key-caps-in-every-hint.md`).
A search of the game for `key_caps` and `KeyCapStyle` (game pinned at
engine `e738ffc2`) finds only that ticket, its `docs/engine/pins.md`
and its `shuyi-input` skill: the game draws no caps yet, so nothing in
it changes here.

The retro's hot spots, checked against the code of `main` at `5a1b6e29`:

- **`GlyphBuffer::clear_styles`** (`crates/ui/src/frame/glyph_buffer.rs`,
  about line 642) runs `BTreeMap::retain` over the whole `styles` map
  once for each row of the rect. `print_from` calls it once for each
  character it prints (about line 1103), and `fill_rect` (about line
  1137) and `blit` (about line 1243) once each. Once a key cap is on the
  buffer, every character printed after it visits every styled cell: a
  frame costs characters × styled cells. No allocation, but time.
  **Confirmed.**
- **The styles map itself** allocates each frame. Each frame starts
  with `fill_rect` over the whole buffer (`Game::redraw`,
  `crates/ui/src/game.rs` about line 743), which empties the map and so
  frees its nodes; the caps drawn after it insert their cells again
  (`set_glyph_style`), which allocates the nodes again. **Found while
  checking.**
- **`print_cap`** (`crates/ui/src/frame/glyph_buffer/caps.rs`, about
  line 42) makes a `String` of spaces (`" ".repeat(layout.extra)`) for
  each cap, each frame; `print_arrows` (about line 71) does the same for
  the arrows' picture. **Confirmed.**
- **`add_cap_box`** (same file, about line 97) builds a
  `Vec::with_capacity(5)` of the box's rectangles for each cap, each
  frame. **Found while checking.**
- **`KeyCaps::shown`** (`crates/ui/src/frame/key_cap.rs`, about line
  208) returns `Cow::Owned(u.to_string())` for a one-letter key name
  with `capitals` on (the default): one `String` for each such cap, each
  frame. **Found while checking.**
- **`Ctx::key_caps()`** (`crates/ui/src/ctx.rs`, about line 503) builds
  a `KeyCaps` every frame, with two palette lookups by name, and
  **`Game::frame_widths`** (`crates/ui/src/game.rs`, about line 724)
  clones `CellWidths` every frame. **Checked: neither allocates.**
  `CellWidths` and `KeyCaps` hold only `Rc`s, numbers and colours, so a
  clone counts references, and `Palette::lookup` takes a `&str`. Each
  costs a few map lookups and comparisons a frame. They stay as they
  are (*Out*).
- **On Windows, `after_frame`** (`crates/app/src/backend.rs`, about
  line 232) calls `window_size()`, which calls `win32::client_size()`,
  which calls `drops::miniquad_window()`. That runs `EnumThreadWindows`
  and, for each window of the thread, `GetClassNameW` and
  `String::from_utf16_lossy` (`crates/app/src/drops/windows.rs`, lines
  71–97), every frame, for a window that never changes. **Confirmed**,
  but it is the backend's, Windows only, and a test of `Game::frame`
  can't see it (*Out*).
- **The renderer's `buf.glyph_style(x, y)`** for each glyph drawn
  (`crates/app/src/render.rs`, about line 288) looks the cell up in the
  styles map: a lookup among the few styled cells, no allocation. It
  stays as it is (*Out*).

Parked 3020 sets a budget of zero allocations in a steady frame on the
GBA and the DS, measured by `shuyi bench`, and says that each frame
which allocates gets a ticket of its own. This ticket is the first such,
on the desktop, and its test is how the count is kept from rising
before 3020 exists.

## Nick input

None.

## Scope

**In:**
- **A test,** `examples/hello-grid/tests/frame_alloc.rs`, its own test
  binary with one `#[test]` (the counter is the whole process's, as
  `mix_alloc.rs` says), skipped in a release build as `mix_alloc.rs` is.
  It drives `shuyi_ui::Game<Hello>` directly with `Game::frame`, not the
  Harness: the Harness's `note_frame` builds a list of the screens'
  names every frame, which would count. With key caps on, it measures
  the title (its menu and its hints) and the stage (the map, its labels
  and its help bar), each 60 frames to warm up and then 120 frames
  counted, each frame read from `shuyi_alloc_count::global::reading()`
  before and after `Game::frame`. It asserts:
  - the most allocations in one counted frame of each screen equal the
    count pinned for it, a `const` in the test: lowering a pin is how a
    later fix records itself, and the test fails when the count drops,
    so the fix must lower it. Each pin has a comment saying which
    ticket set it last;
  - with caps on, a stage frame allocates no more than with caps off:
    a cap costs no allocation;
  - live bytes after the 120 counted frames equal those before them:
    a steady frame keeps nothing.
- **Key caps in Hello, Grid, off by default:** a field
  `pub key_caps: Option<KeyCapStyle>` on `Hello`, `None` from
  `Hello::load`, and `GameData::key_caps` returning it. The game looks as
  it does today, and no snapshot changes. The test sets it on the
  context's game before it builds the `Game`, and sets the frame's widths
  from Hello, Grid's font atlas as the platform does
  (`Game::set_widths(CellWidths::of(font))`, as `use_look_widths` in
  `crates/platform/src/app.rs` does), so the caps are placed by the
  font's ink.
- **The styles store without a scan and without allocating a frame:**
  replace `GlyphBuffer`'s private
  `styles: BTreeMap<usize, GlyphStyle>` with a list sorted by cell index
  that keeps its capacity (`Vec<(usize, GlyphStyle)>`). `clear_styles`
  finds each row's span with `partition_point` and removes it with
  `drain`; `fill_rect` over the whole buffer calls `clear`, which keeps
  the capacity. `glyph_style` is a binary search, and `set_glyph_style`
  an insert at its place (caps print left to right, so it is usually at
  the end). If the implementer keeps the `BTreeMap` instead, then
  `clear_styles` removes only the keys of `range(first..=last)` for each
  row and skips a row with none, and the test's pin shows the nodes it
  still allocates each frame. The field is private: no API change.
- **Caps that allocate nothing:**
  - `print_cap` and `print_arrows` print their spaces without a
    `String`: a private `print_blank(cx, y, cells, paint)` that runs
    `print_from`'s per-cell work for `' '` `cells` times, or `print_from`
    over a slice of a `const` string of spaces, taken in chunks.
  - `add_cap_box` adds its rectangles without a `Vec`: each one handed
    to `add_overlay` as it is made.
  - `print_cap` prints a one-letter name's capital without a `String`:
    `KeyCaps::shown` stays public and as it is. A private sibling returns
    the capital as a `char`, and the cap encodes it into a `[u8; 4]`
    (`char::encode_utf8`) for `print_from`, `text_width` and `layout`.
- **The text fill's lists and joins, which 3219 (done in #485) left:**
  - **No list per label:** `Ctx::label` and `Ctx::label_styled`
    (`crates/ui/src/ctx.rs`) collect a `Vec` of values on every call.
    They hand their values to the fill through 3219's private lookup
    (`text::fill_styled_by`) instead. A `Label::Key`'s `[(String, Arg)]`
    gives that lookup as `&Args` does.
  - **A plain fill writes a `String`:** `fill`, `fill_values`,
    `Ctx::text_with`, `text_with_keys`, `fill_text` and `label` build a
    whole `StyledText` and then join it with `.plain()`
    (`text.rs`, `fill`). Instead they write into the `String` they
    return, through a private sink that a `StyledText` also takes.

  Public signatures don't change, and every text fills byte for byte as
  before: 3219's equivalence tests in `crates/ui/src/text/plain_tests.rs`
  still pass. The select's comparison and an argument written once are
  2268's, with its borrowed walk. The counts are pinned on top of 3219
  and of 2266 (done: no frame parses a text).
- **Either order with 2268:** if 2268 is done, pin the counts the test
  measures after it; if not, pin today's counts, and 2268's PR lowers
  the pins to what the test then measures (the test fails until it
  does).

No public API changes, and nothing a game sees changes: every frame
draws the same cells, styles and items as before, so no `Breaking:` or
`Behaviour:` line. The game's next pin move needs nothing for it.

**Out (do not do):**
- `Message::parse` on each fill of a message: 2266 (done) took it out
  of the frame. A select's comparison and an argument's copy: 2268 does
  them (the clause above). This ticket pins the count either way.
- Rebuilding `Ctx::key_caps()` and `Game::frame_widths` only when what
  they read changes: they allocate nothing, and a cache would have to
  follow the palette's reloads, the theme, the language and the screen.
- The Windows window lookup in `shuyi-app` (`miniquad_window`, called
  through `window_size()`): the backend's, Windows only, and out of
  this test's reach. Since 3222 it lists the thread's windows only
  until it finds miniquad's, and keeps it.
- The renderer's lookup of each glyph's style in `shuyi-app`: a binary
  search over the few styled cells, no allocation.
- `shuyi bench`'s `allocs_per_frame` and the budgets of the GBA and the
  DS (parked 3020).
- Getting to zero allocations a frame. The pin is what the frame
  allocates once the hot spots above are fixed; what remains is named
  in the completion notes, each with its source, for later tickets.
- Raising a pin to pass the test (`AGENTS.md` rule 6). A change that
  needs more allocations a frame says why in its PR and in the pin's
  comment.

**Estimated size:** about 1,150 lines (code 400, tests 650, docs 100).

**Split:** past 1,000 lines, split off the text fill's lists and joins
(the labels' lookup and the plain fill's sink) into a new ticket, and
keep the test, the styles store and the caps here.

## Implementation steps

1. Add `key_caps: Option<KeyCapStyle>` to `Hello`
   (`examples/hello-grid/src/lib.rs`), `None` from `Hello::load`,
   `load_on` and `load_from`,
   and `fn key_caps(&self) -> Option<KeyCapStyle> { self.key_caps }` in
   its `GameData` impl.
2. Write `examples/hello-grid/tests/frame_alloc.rs` from
   `crates/sound/tests/mix_alloc.rs`'s shape: a `counts()` helper, the
   `cfg_attr(not(debug_assertions), ignore = …)`, one test. Build the
   title with `hello_grid::game(ctx)` and the stage as `Start`'s
   `harness_of` does (`StageScreen::new(&mut ctx).briefed()` in
   `Game::new`), each from `hello_grid::ctx(seed)` with
   `ctx.game.key_caps = Some(KeyCapStyle::new())`, and once more with
   caps off for the stage. Set the widths from `ctx.game.font`. Warm up,
   count, and print the counts measured (`eprintln!`) before asserting,
   so a run shows the numbers. Run it before any fix and write the
   counts down for the PR.
3. Replace the styles store and `clear_styles` (*Scope*). Keep
   `glyph_style`, `set_glyph_style` and `glyph_styles` as they are seen
   from outside, `glyph_styles` row by row as today.
4. Make `print_cap`, `print_arrows` and `add_cap_box` allocate nothing
   (*Scope*).
5. Move `Ctx::label`, `label_styled` and the plain fills onto 3219's
   lookup and a private sink (*Scope*). 3219's equivalence tests must
   still pass.
6. Run the test, pin the counts it measures, and check the 2268 clause
   against `tickets/done/`.
7. List in the completion notes what still allocates in a steady frame
   and where (read with a breakpoint or a backtrace in the counter, or
   by bisecting the drawing), and the counts before and after.

## Acceptance criteria

- [ ] `examples/hello-grid/tests/frame_alloc.rs` runs in `cargo test -p
      hello-grid` in a debug build, its own test binary with one test,
      and fails when a change adds an allocation to a steady frame of
      the title or the stage (checked once by hand with a `Vec::new`
      pushed in `Game::redraw`, then removed).
- [ ] With caps on, a counted stage frame allocates no more than one with
      caps off.
- [ ] Live bytes after the 120 counted frames equal those before them,
      for each screen.
- [ ] `clear_styles` no longer visits styled cells outside the rect it
      clears (by the code: a range or a binary search, no `retain` over
      the whole store).
- [ ] `print_cap`, `print_arrows`, `add_cap_box` and the one-letter
      capital make no `String` or `Vec`.
- [ ] `Ctx::label` and `label_styled` build no `Vec`, and a plain fill
      builds no `StyledText` to join. Checked by the code, and by 3219's
      equivalence tests still passing.
- [ ] Every existing frame, cap and snapshot test passes unchanged.
- [ ] Hello, Grid looks as before: no snapshot of it changes.
- [ ] The PR states each screen's count per frame before and after the
      fixes.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Integration: `examples/hello-grid/tests/frame_alloc.rs` as in *Scope*.
- Property: in `glyph_buffer.rs`'s proptest module, a random sequence of
  prints (plain and styled, narrow and wide, with caps on), `fill_rect`s,
  `set_glyph_style`s and `blit`s over a buffer leaves `glyph_styles()`
  equal to a model that keeps a `BTreeMap` and clears a rect with the
  old full-scan `retain`.
- Unit: a cap with a one-letter name, with `capitals` on and off, and
  the arrows' picture print the same cells, styles and items as before
  (the existing `caps_tests.rs` cases pass unchanged; add a one-letter
  case with `capitals` off if none exists).

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
