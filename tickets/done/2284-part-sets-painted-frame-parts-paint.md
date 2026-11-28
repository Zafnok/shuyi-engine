---
id: "2284"
title: "A part set's look painted: shuyi-ui's parts feature, frame::parts::paint composites a rolled look into a painted picture"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: medium
status: done
blocked_by: ["2260"]
nick_input: none
completed: 2026-11-27
---

# 2284 — A part set's look painted

## Context

**From 2260 (done):** this is that ticket's split, at its `**Split:**`
line (past 1,400 lines, the skin's half). 2260 made part sets' file and
roll: `shuyi_assets::parts` behind `shuyi-assets`' off-by-default
`parts` feature (`PartsDef`, `PartsRon`, `PartSet::appearance`,
`PartSet::check_pixels`, `Appearance`), its fixture
(`crates/assets/tests/fixtures/parts/faces.ron` and `faces.png`, four
slots of 8 × 8 parts and two kinds) and its golden table.
[ADR-0165](../../docs/adr/0165-part-sets-grey-keyed-parts-rolled-per-slot-and-painted-by-the-skin.md)
decided where the painting goes: an off-by-default `parts` feature of
`shuyi-ui`, which turns `shuyi-assets`' on, so a headless build rolls
without painting.

The painting was written and tested in 2260's session before the split;
its code is on the local branch `t2284-part-sets-paint` of that
worktree. A session that can't reach it writes it again from this
ticket.

## Nick input

None.

## Scope

**In:**
- **`shuyi-ui`'s `parts` feature**, off by default, `parts =
  ["shuyi-assets/parts"]`, and `pub mod parts` in `frame` behind it. Its
  dev-dependency on itself turns it on, and its dev-dependency on
  `shuyi-assets` turns on `png` and `parts` (to read the fixture).
- **`frame::parts::paint(set: &PartSet, look: &Appearance, pixels: impl
  Fn(ImageId) -> Option<&Image>, out: &mut Painted) -> Result<(),
  PaintError>`**, as 2260's ticket said: `out` must be RGBA and the
  set's size. It clears `out`, then draws each slot's part in slot order:
  a pixel with alpha under 128 (`parts::DRAWN_ALPHA`) is skipped; an
  opaque grey that is a key the look colours is drawn in its colour; any
  other pixel keeps its own colour; every drawn pixel is opaque. A
  one-pass lookup table from grey to colour (`[Option<[u8; 3]>; 256]`).
  It borrows `out`'s pixels once, so the revision moves once, and not at
  all when it fails: every part and its pixels are checked first.
- **`PaintError`** (`thiserror`, `#[non_exhaustive]`): `out` not RGBA,
  the wrong size, a look not of the set (a part it lacks, more slots),
  a part whose pixels are missing or fewer than its rectangle needs.
- **The `no-std` line** of CI and `cargo xtask gates`: `cargo build -p
  shuyi-ui --no-default-features --features raster,parts --target
  thumbv7em-none-eabihf`.
- **The API check** (`crates/xtask/src/api.rs` and its tests' list):
  `parts` among `shuyi-ui`'s features.
- **Docs**: the book's chapter (`site/content/learn/book/frame-and-skins.md`,
  *A picture the skin paints*): a paragraph on part sets, the rules
  keeping a seed and a kind, the skin rolling and painting; the formats
  page's part sets section and `shuyi_assets::parts`' module docs name
  `paint` and the feature; the `shuyi-import-art` skill's paragraph says
  to turn on `shuyi-ui`'s feature and paint with `frame::parts::paint`.

**Out (do not do):**
- Anything of the format or the roll (2260 did them).
- A cache of painted looks, a GPU palette swap (2327) or tints (2337),
  as 2260 said.

**Estimated size:** about 500 lines (code 150, tests 250, docs 100).

## Implementation steps

1. The feature, the module and `paint`, with `PaintError`.
2. Its tests on the fixture, and the raster snapshot.
3. The `no-std` line, the API check's list, the docs.

## Acceptance criteria

- [x] `paint`, on the 8 × 8 fixture: a key-grey pixel is the rolled
  colour; a red pixel stays red; a pixel with alpha 100 leaves what is
  below it; the later slot's pixel is over the earlier's; `out`'s
  revision moved by one; a wrong size or kind of `out` is an error, and
  moves no revision.
- [x] A raster-painter snapshot (`frame::raster::Painter`) of three
  rolled faces of the fixture set side by side, with three skins: a look
  test, so it reads pixels.
- [x] Opt-in: a game that doesn't turn the feature on builds none of it
  (`cargo tree -e features`, recorded in the completion notes); every
  existing snapshot is unchanged.
- [x] `shuyi-ui` builds without `std` with `raster,parts`.
- [x] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [x] The skill pack (`skills/`) is updated for a changed schema, template, format or problem code, or the completion notes have a `Skills:` line saying why not (`cargo xtask docs-rules`, 3534).
- [x] All gates in the `run-gates` skill pass.

## Tests required

- Unit: `paint`'s pixel rules, its errors, the revision.
- Snapshot / integration: the raster snapshot of three faces.

## Completion notes

**Done.** `shuyi-ui`'s `parts` feature (off by default, `parts =
["shuyi-assets/parts"]`) and `frame::parts` behind it:
`paint(set, look, pixels, out)` and `PaintError` (`NotRgba`,
`WrongSize`, `NotOfTheSet`, `NoPixels`), as the scope says. The code is
the one 2260's session wrote on the local branch `t2284-part-sets-paint`,
taken over unchanged onto `main` after 2260 merged; the feature, gate,
API-list and doc edits were made again against `main`, where 2260 had
already moved part of them (the `shuyi-assets` `parts` no-std line, its
API-list entry).

- Tests (`crates/ui/src/frame/parts/tests.rs`, on the fixture): a key
  grey takes the rolled colour, red stays red, a key the look doesn't
  colour keeps its grey, the hair's alpha-100 pixels leave the head's
  outline below them, a later slot over an earlier one, clear where no
  part draws, the revision moving by one per paint; painting again
  clears; a wrong size, an indexed picture, missing or too few pixels, a
  part not in the set and too many slots are errors that move no
  revision.
- The raster snapshot: three faces rolled from the fixture (seeds 1 and
  3 of `plain`, 6 of `capped`, three skin colours) side by side through
  `frame::raster::Painter`, as text with a colour legend.
- Opt-in: `cargo tree -p hello-grid -e features` shows no `parts`
  feature of `shuyi-ui` or `shuyi-assets`; `cargo tree -p shuyi-ui
  --features parts -e features` shows `shuyi-assets feature "parts"`
  pulled by it. No existing snapshot changed.
- No-std: `cargo build -p shuyi-ui --no-default-features --features
  raster,parts --target thumbv7em-none-eabihf` builds, now in CI's
  `no-std` job and `cargo xtask gates`.
- The ticket said "three skins" for the snapshot; read as three skin
  colours of rolled faces (one painter), which is what a look test of a
  roll needs.
- An opaque key grey: `paint` colours a key grey at any alpha it draws
  (128 and up), drawn opaque, the same rule as "every drawn pixel is
  opaque".

Book: `frame-and-skins.md` gains the part sets paragraph (seed and kind
kept by the rules, the skin rolling and painting with
`frame::parts::paint`).

Skills: `shuyi-import-art` now says to turn on `shuyi-ui`'s `parts`
feature and paint with `frame::parts::paint`.

No break: an added feature, module, function and error type.

Game: nothing to adopt, the game doesn't use part sets yet and turns no
`parts` feature on.
