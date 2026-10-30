---
id: "3073"
title: "frame-png on a one-bit target paints sprites from the target's one-bit pictures, not grey rectangles"
type: feature
milestone: After MVP
model: sonnet-5
effort: medium
status: todo
blocked_by: ["3007"]
nick_input: none
completed:
---

# 3073 — frame-png on a one-bit target paints sprites from its one-bit pictures

## Context

[ADR-0008](../../docs/adr/0008-export-targets-profiles-as-data-platform-backends-controls-that-fit-one-bit-and-budgets.md)
§4. `cargo shuyi frame-png --target playdate` paints a frame as a
one-bit screen shows it (`GameTools::frame_one_bit`, ticket 3006). Its
sprites are grey rectangles: Hello, Grid's `one_bit_picture`
(`examples/hello-grid/src/tools.rs`) hands `to_one_bit` no pictures
(`&()`), and its doc says the pictures "aren't made one-bit yet".

**From 3007 (done):** they are. A one-bit target's bake makes every
picture a one-bit picture file (`Bake::under_as("", OneBitFile)`), from
the target's layer (`Bundle::for_target`) or dithered by its `onebit`
settings (`shuyi_assets::bitmap::dither`). `impl Images for Bundle`
(`shuyi_ui::frame::onebit`) draws the one-bit file at each image's
path, so a bundle of a one-bit pack feeds `to_one_bit` as it is. A
frame-png of a stage in tiles still shows grey rectangles, so an artist
can't see the sprites as the device will.

## Nick input

None.

## Scope

**In:**
- Hello, Grid's `frame_one_bit` paints its sprites from the one-bit
  pictures its Playdate bake makes: the bundle of a pack baked with
  `GameTools::bake(Some(target))` (in memory, as `check --baked` bakes),
  or each picture of `hello_grid::bundle(Some(target))` made one-bit
  with the standard formats (`Formats::load::<OneBitPicture>`, which
  dithers a PNG and reads a one-bit file as it is) into an `Images` of
  the example's own. Pick the smaller; say which in the PR.
- The doc of `one_bit_picture` and the README's *Targets* say what it
  paints.
- A snapshot of the stage in tiles on the Playdate
  (`frame-png --target playdate --stage --tiles`), beside the title's.

**Out (do not do):**
- a change to `GameTools::frame_one_bit`'s signature, or to the
  contract (it needs none);
- the template's game: it has no sprites;
- dithering at run time on a device (3007's rule: pictures are made
  one-bit at bake time).

## Implementation steps

1. In `examples/hello-grid/src/tools.rs`, give `one_bit_picture` the
   pictures, as **In** says, and pass them to `to_one_bit`.
2. Add the stage snapshot to `tests/it/onebit.rs`.
3. Update the doc comment and the README.

## Acceptance criteria

- [ ] `frame-png --target playdate --stage --tiles` shows the tileset
      drawn in `files@playdate/`, not grey rectangles (the snapshot).
- [ ] The title's snapshot is unchanged.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Snapshot: the stage in tiles on the Playdate.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
