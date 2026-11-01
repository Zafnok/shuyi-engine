---
id: "3113"
title: "A game's icon with pictures drawn per size, used in place of the downscale"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: done
blocked_by: []
nick_input: none
completed: 2026-11-01
---

# 3113 — A game's icon with pictures drawn per size

## Context

1. **The game ticket:** game#1199
   (<https://github.com/Zafnok/visions-of-shuyi/blob/main/tickets/done/1199-the-games-identity-in-shuyi-ron.md>),
   and its follow-up game#1201, which moves the game onto this.
2. **The job:** a pixel-art game's window shows its icon crisp at 16 and
   32 pixels, from pictures drawn for those sizes.
3. **The friction numbers:** no friction review: a missing feature.
4. **The generic shape:** ADR-0061 makes every window icon by box-filtering
   one square picture of 512 pixels or more, and says "Pictures drawn per
   size wait for a target that needs them". A picture with lines one
   source pixel wide at the small size turns to mud. Measured on the game's
   icon, a font glyph on a square: the 64 px picture came out identical,
   while 64 of the 256 pixels differ at 16 px and 133 of the 1024 at 32 px,
   and the glyph can't be read at 16. No one source gives the drawn set
   either, since the box filter's 16 px picture is the mean of its 32 px
   one, and pictures drawn per size aren't means of each other.
5. **A game that isn't tactics:** a farming game whose icon is its 16 px
   turnip sprite, drawn pixel by pixel, wants that sprite in the title bar,
   not a blur of a bigger painting.

## Nick input

None.

## Scope

**In:**
- `AppPart` gains `icon_sizes`, a map from a side in pixels to a PNG file
  (`icon_sizes: { 16: "art/icon-16.png", 32: "art/icon-32.png" }`), empty
  by default. `AppPart` is already `#[non_exhaustive]`, so the field is
  additive (ADR-0039), and the API doesn't break.
- `app-info` writes a window icon whose side is in `icon_sizes` as that
  file's pixels, re-encoded with `codec::encode_png`, and downscales the
  others from `icon`, as today. `stale`, `--check` and `check --target`
  cover the result unchanged.
- `app_info::problems`, each `app.icon_sizes`, an error: a file that doesn't
  read or decode, or isn't square at the side its key says.
- `icon` stays required when `window_icons` is set: the sizes it doesn't
  draw come from it.
- An ADR extending ADR-0061, and `shuyi.ron`'s template comment and the
  `shuyi-ship` skill's icon step naming the field.
- What changes for the game: nothing until it sets the field; game#1201's
  PR moves the pin and follows this one.

**Out (do not do):**
- The packages' sizes (3049): they keep downscaling from `icon`, and this
  ticket leaves them no work.
- Scaling a drawn picture to another size, or a sharper filter: the box
  filter's bytes must stay the same everywhere (ADR-0061).
- `WindowIcon` and `AppInfo`: their fields don't change.

## Implementation steps

1. Add `icon_sizes` (`BTreeMap<u32, String>`, `#[serde(default)]`) to
   `AppPart` in `crates/cli/src/project.rs`.
2. In `crates/cli/src/app_info.rs`, check the files in `checked`, and in
   `files` take a drawn picture in place of `downscale` for its side.
3. Write the ADR, then update the template's comment and the skill.

## Acceptance criteria

- [x] With `icon_sizes` set for 16 and 32, `app-info` writes those pictures'
      pixels and downscales 64 from `icon`.
- [x] A drawn file of the wrong side, or that isn't a PNG, is an
      `app.icon_sizes` error in `check --target`, and `app-info` writes
      nothing.
- [x] A drawn file changed on disk makes `app-info --check` fail until
      `app-info` is run.
- [x] A project without the field writes the same bytes as before.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: the problems above; a drawn picture beats the downscale at its side
  and leaves the others alone; `project.rs` reads the field, and its default.
- Snapshot / integration: `shuyi.ron`'s template still passes `check
  --target`.

## Completion notes

`app.icon_sizes` draws window icons per size: `app-info` writes each drawn
file's pixels through `codec::encode_png` and downscales the other sides
from `icon`. Written up in ADR-0069, the template's comment, the
`shuyi-ship` skill and the book's *Targets and export* chapter.

Deviations: two rules beyond the ticket's list, both `app.icon_sizes` errors,
since otherwise the pictures would be ignored without a word: a key that
isn't 16, 32 or 64, and a non-empty map without `icon` or `window_icons`.

The game: game#1201's PR moves the pin and sets the field; nothing changes
for it until then. No break, and no follow-up tickets.
