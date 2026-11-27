# ADR-0162: Part sets: pictures of grey-keyed parts, a look rolled per slot from a hashed seed and painted by the skin, behind off-by-default features of `shuyi-assets` and `shuyi-ui`

- **Status:** Accepted
- **Date:** 2026-11-27
- **Related tickets:** 2260 (this: part sets), 2246 (picture sets, the
  hand-drawn way), 2327 (a palette swap on the GPU, parked), 2337
  (tints)
- **Amends:** ADR-0160 §5 (when a module, and when a feature of a
  default crate)

## Context

Games with many generated people, creatures or things want a picture
for each without drawing each: a town's villagers, a roguelike's
monsters in colour variants, a card game's random portraits. The usual
way is a picture built from parts drawn in grey, each grey that matters
a key (skin, skin in shadow, hair), with a part rolled for each slot and
a colour for each key, and kinds that override only what they need.

Ticket 2260 asked for this as something a game opts into, never the
main way to draw people, and for its home to be chosen against two
goals that pull against each other:

- **A clean default engine.** A game that doesn't opt in builds none of
  it, sees nothing change, and meets no new concept.
- **Modular.** One place, or two gated halves, removable whole, with no
  more paperwork than the feature needs.

It has two halves. The file and the roll are pure data and integers;
the painting turns a look into a `Painted` picture (ADR-0078), which is
`shuyi-ui`'s. A headless build (a game's rules, a server) may roll
without painting. Unlike the corner sheets of ADR-0160, a shipped game
runs both halves: its skin paints faces as it runs. ADR-0160 §5 says
such a part is a module.

## Decision

1. **Two off-by-default features named `parts`, one in each crate the
   halves belong beside**:
   - `shuyi-assets`' holds the format and the roll, in
     `shuyi_assets::parts`: `PartsDef` (the file as written), the format
     `PartsRon` (`parts`, `.ron`, checked alone), `PartSet` (resolved
     against an `ImageTable`), `PartSet::appearance`,
     `PartSet::check_pixels`, and `Appearance`, plain data;
   - `shuyi-ui`'s holds the painting, in `shuyi_ui::frame::parts`
     (`paint`, beside `frame::painted`), and turns the first on. A game
     turns on `shuyi-ui`'s, or only `shuyi-assets`' for a build that
     rolls without painting. No dependency and no `std` in either; both
     build for `thumbv7em-none-eabihf` (the `no-std` gate).
   - No feature of the `shuyi` front door: its features are modules
     (`cargo xtask check-modules`), and a game turns these on in its own
     `Cargo.toml`, as it does `sfx` (ADR-0144).
2. **ADR-0160 §5 is amended:** a format with its own pure function (a
   sound effect's render, a part set's roll) is a feature of
   `shuyi-assets` (ADR-0009 §4's form for formats), even when a shipped
   game runs it, as long as it answers no concern, pulls in no
   dependency, and its drawing half, if it has one, is a feature of the
   crate that draws. A part that answers a concern, or pulls in a
   dependency or a heavy layer, is still a module.
3. **Grey keys, composited on the CPU into a painted picture.** A grey
   (red = green = blue) that is a key is painted in the colour rolled
   for it; every other pixel keeps its own colour; a pixel with alpha
   under 128 is left out and every pixel painted is opaque. One pass, a
   table of 256 colours by grey. Every painter already draws a
   `Painted` picture, on every target, so this needs no new drawing
   path.
4. **Each slot and each colour group rolls from its own `SimRng`**,
   seeded by the FNV hash (`shuyi_assets::fnv`) of the seed, then
   `"slot"` or `"colour"`, then its name (`parts::slot_rng`). Adding a
   slot or a group changes no other one's roll, so a mod that adds
   earrings leaves every face of a saved game as it was.
5. **Hides go from the top down, and only downward.** A part may hide
   only slots under its own (the file is refused otherwise), and from
   the topmost slot down a part still shown empties what it hides. So a
   hidden part hides nothing, and the order is never ambiguous.
6. **Looks are worked out by the skin, never stored.** The rules keep a
   person's seed and kind and what they wear; `Appearance` is a pure
   function of the set, the kind, the seed and the worn parts, so the
   skin works it out when it paints (`AGENTS.md` rule 9). A game keeps
   one `PaintedId` per place a face shows and paints it again when the
   person changes; there is no cache.
7. **No concern, no catalog row.** A game doesn't choose part sets in
   place of drawn pictures; it adds them for its crowds, and may show
   both on one screen. So `docs/modules.ron` gains no `pictures`
   concern and `shuyi new` asks nothing new.

## Consequences

- A game that doesn't opt in builds none of it: neither module is
  compiled without its feature, and `cargo tree -e features` of a game
  shows no `parts` (ticket 2260's notes).
- Removing it is one module and one feature in each of two crates, with
  their tests and fixtures, two `no-std` lines, a golden table, a
  formats section, a book paragraph and a skill paragraph.
- The rustdoc of the two default crates shows the modules, marked as
  behind a feature: the one place the concept shows to a game that
  doesn't use it.
- The golden table (`crates/assets/tests/golden/parts.ron`) pins the
  fixture's decoded hash and seed 7's look; it runs with `shuyi-assets`'
  tests in CI's `determinism` job on wasm32. A moved row is a break.
- Weights are a part named twice in a list; numbers as weights can come
  later, additively, as a new field.

## Alternatives considered

- **A module crate, `shuyi-parts`** (ADR-0009 §4): the cleanest
  separation, and ADR-0160 §5 as written. But it needs metadata, a
  README with its measured size, a `tools/layers.ron` line, a front-door
  feature, a catalog row, a concern that `shuyi new` would ask about
  something no game must choose, and a skill; and the painting half
  would still need a feature of its own inside it, so a headless build
  rolls without `shuyi-ui`. That is more paperwork than code, for a
  format of a few hundred lines with no dependency.
- **One feature of the `shuyi` front door turning both on**: one switch
  for a game. But the front door's features are modules, checked
  against the catalog; and `shuyi-ui`'s feature is already one switch,
  since it turns `shuyi-assets`' on.
- **Indexed colour**: parts as `Painted::indexed` pictures, a palette
  per look. But an indexed picture has no alpha, so parts can't be laid
  over one another, and an artist draws in ordinary RGBA editors.
- **A palette swap in a shader** (2327, parked): cheaper per frame, but
  only on the GPU, not on the raster painter or the one-bit renderer,
  and a face of ten 32 × 32 parts is cheap to composite on the CPU when
  it changes.
- **One `SimRng` for the whole look**: simpler, but adding a slot moves
  every later draw, which changes every face of a saved game.
