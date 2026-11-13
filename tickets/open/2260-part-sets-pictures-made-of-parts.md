---
id: "2260"
title: "Part sets, opt-in: pictures made of grey-keyed parts (faces, figures), a look rolled from a seed with kinds that override only what they need, and a skin that composites and colours it"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2260 — Part sets: pictures made of grey-keyed parts, opt-in

## Context

Games with many generated people, creatures or things want a picture
for each without drawing each one: a farming town's villagers, a
management game's staff, a roguelike's monsters in colour variants, a
card game's random portraits, a shop's potions (bottle, liquid, label).
The usual way is a picture built from parts:

- the parts (heads, eyes, hair, beards, coats) are drawn in grey, and
  each shade of grey is a **key**: this grey is skin, that one skin in
  shadow, that one hair;
- when a person is made, the game **rolls** a part for each slot and a
  colour for each key, and swaps every key's grey for its colour;
- each **kind** (a people, a species, a class of item) overrides only
  what it needs: one kind adds pointed ears, another horns;
- a part can **hide** slots: a helmet hides the hair;
- a modder adds a whole new kind with one data file and a part or two.

What the engine has today:

- **Painted pictures** (ADR-0078, `crates/ui/src/frame/painted.rs`): a
  skin paints a `Painted` picture at run time, keeps it in the frame's
  `PaintedTable`, and places it with a `PaintedSprite`. `shuyi-app`
  uploads it only when its revision moves; the raster painter
  (`frame-png`) and the one-bit renderer draw it too (an RGBA picture is
  dithered at paint time). So a composited face needs no new drawing
  path, on any target.
- **Pixels.** `shuyi_assets::image::Image` with the `Png` format (`png`
  feature) gives a part's pixels; `ImageTable` gives its id and size.
- **Seeded rolls**: `shuyi_core::rng::SimRng`, the same on every
  platform.
- **No recolouring by key.** `Paint::Shaded` multiplies by a light, and
  a per-sprite palette swap is planned only on the GPU, in the parked
  shader ticket (2327).
- **Related, not the same:** the picture set (2246) gives each named
  person a file of hand-drawn pictures (`"happy": "ana/smile.png"`).
  Part sets make pictures for people nobody drew.

**Opt-in, and kept apart.** Hand-drawn pictures stay the default. This
is something a game chooses to use, never the main way:

- a game that doesn't opt in sees nothing change and builds none of it;
- the code sits in one place that could be removed whole;
- a game can use it for its crowds and draw its named characters by
  hand (2246) in the same screen.

Where the code lives is the session's first decision, made against those
goals: its own module crate
([ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)),
or features that are off by default in the crates the halves belong
beside (Scope, *Where it lives*). A whole crate may be more than this
needs, but it isn't ruled out.

**Graphics are a skin** (`AGENTS.md` rule 9). The rules keep a person's
seed and kind (and what they wear) in their state. The look is a pure
function of the part set, the kind, the seed and the worn parts, so the
skin can work it out each time, and the rules never store pixels or
colours. Adding a slot or a colour group to a part set must not change
the other slots' rolls: a mod that adds earrings must not change every
face in a saved game. So each slot and group rolls from its own
`SimRng`, seeded by the FNV hash (`shuyi_assets::fnv`) of the seed and
the slot's or group's name.

## Nick input

None.

## Scope

**In:**
- **Where it lives: decide first, and write down why.** Weigh the
  options against two goals that pull against each other: **keep the
  default engine clean** (a game that doesn't opt in builds none of it
  and sees nothing change; no new concept in the default crates' docs)
  and **keep it modular** (one place, or two gated halves, removable
  whole; opt-in, never the main way to draw people; not more paperwork
  than the feature). At least:
  - **A module crate**, `shuyi-parts`, as ADR-0009 §4 describes:
    metadata, a README with its measured size, a `tools/layers.ron`
    line, a feature of the `shuyi` crate off by default, a catalog row,
    a skill; the painting half behind the module's own feature, so a
    headless build rolls without `shuyi-ui`. Cleanest separation; the
    most paperwork.
  - **Features off by default** where the halves belong: the format and
    the roll as a `parts` feature of `shuyi-assets` (ADR-0009 §4 already
    makes formats features of `shuyi-assets`), the painting as a `parts`
    feature of `shuyi-ui`, and one feature of the `shuyi` crate that
    turns both on. Less paperwork; the code sits in default crates,
    gated.
  - Anything else the session finds that serves both goals better.

  Whichever is chosen: the roll stays pure and in integers, and the
  painting stays in the skin (`AGENTS.md` rule 9); new engine edges or
  outside crates get their `tools/layers.ron` lines; the format's and
  roll's tests run in CI's `determinism` job on wasm32; and the choice
  and the reasons for it go in the completion notes and the ADR below.

  If the choice is a module and it answers a concern in
  `docs/modules.ron`: **Either order with 2259:** if 2259 is done and
  added a `pictures` concern ("Pictures made from data",
  `alternatives: false`), add this answer and its catalog row there; if
  not, add that concern with its `drawn` answer (`by: "default"`,
  `ready`) and this one, or no concern at all if this isn't a module.
- **The format**, `no_std` with `alloc` if its home allows it, as
  ADR-0007 asks:
  - `PartsDef`, the file as written; `PartsRon`, its format (id
    `parts`, extension `ron`, `shuyi_assets::format::TEXT_MAX_BYTES`),
    with `deny_unknown_fields`, checked alone; then `PartSet`, resolved
    against an `ImageTable` (each part's image exists, and its rect is
    inside the image). Kind name `part set`. The game registers the
    format with its others; nothing is registered for a game that
    doesn't.
  - The file:
    ```ron
    (
        size: (32, 32),
        // Draw order, first undermost.
        slots: ["back_hair", "head", "ears", "eyes", "hair", "beard", "helmet"],
        // A grey (red = green = blue) and what it stands for.
        keys: { 200: "skin", 150: "skin_shadow", 120: "hair", 90: "eyes" },
        // Keys coloured together: one roll picks a row.
        colours: {
            "skin": (keys: ["skin", "skin_shadow"], rows: [
                ["#F1C27D", "#C68642"], ["#8D5524", "#5C3A1A"],
            ]),
            "hair": (keys: ["hair"], rows: [["#2B1B10"], ["#D8B060"]]),
            "eyes": (keys: ["eyes"], rows: [["#3A6EA5"], ["#4A3020"]]),
        },
        parts: {
            "head_round": (slot: "head", image: "faces/heads.png", at: (0, 0)),
            "hair_short": (slot: "hair", image: "faces/hair.png", at: (0, 0)),
            "helm_iron": (slot: "helmet", image: "faces/gear.png", at: (0, 0),
                hides: ["hair", "back_hair", "ears"]),
            // ...
        },
        kinds: {
            "plain": (slots: {
                "head": ["head_round", "head_long"],
                "hair": ["hair_short", "hair_long", None],
                "beard": [None, "beard_full"],
            }),
            "elf": (base: Some("plain"), slots: { "ears": ["ears_long"] }),
        },
    )
    ```
    Each part is the set's `size`, taken at `at` in its image, and drawn
    over the whole picture. A kind with a `base` takes the base's slots
    and colours and replaces only those it names. A slot that no kind in
    the chain names is empty. `None` in a list is "nothing in this
    slot".
  - **Checked alone**, every problem reported with its line: no slot, a
    slot twice; a part's slot, a `hides` slot or a kind's slot not in
    `slots`; a kind's list naming a part of another slot, or empty; a
    `base` that doesn't exist or makes a cycle; a key grey twice; a
    colour group naming a key that isn't one, a key in two groups, no
    rows, a row of the wrong length, a bad `#RRGGBB`; caps of 64 slots,
    1024 parts, 256 kinds, 64 rows a group, and a size of at most 256 ×
    256.
  - `PartSet::check_pixels(&self, pixels: impl Fn(ImageId) -> Option<&Image>)
    -> Vec<ContentError>`: a part whose image has no pixels, and an
    opaque grey pixel within 4 of a key but not on it (a slip of the
    brush that would show grey in the game), each naming the file and
    the pixel.
  - ADR-0039: `PartSet`, `PartsDef` and `Appearance` are
    `#[non_exhaustive]`, built by the format or a constructor.
- **The roll**, pure and in integers:
  - `Appearance { kind: String, parts: Vec<Option<String>> /* by slot */,
    colours: BTreeMap<u8, [u8; 3]> /* by key grey */ }`, plain data with
    `serde`, and an FNV hash for tests.
  - `PartSet::appearance(&self, kind: &str, seed: u64, worn: &[&str]) ->
    Result<Appearance, PartsError>`:
    1. each slot rolls one of its kind's list with `roll_below`, from
       `SimRng::new(fnv(seed, "slot", name))`;
    2. each colour group rolls a row the same way (`"colour"`, its
       name);
    3. each worn part takes its slot (replacing the roll);
    4. every slot that a shown part `hides` is emptied (a hidden part's
       own `hides` don't count).
    An unknown kind or worn part, or two worn parts in one slot, is a
    `PartsError` (`thiserror`).
- **The skin's half**, gated apart from the format and roll (a module's
  own feature, or `shuyi-ui`'s), so a headless build can roll without
  painting: `paint(set: &PartSet, look: &Appearance, pixels: impl
  Fn(ImageId) -> Option<&Image>, out: &mut Painted) -> Result<(),
  PaintError>`. `out` must be RGBA and the set's size. It clears `out`,
  then draws each slot's part in slot order: a pixel with alpha under
  128 is skipped; an opaque grey that is a key is drawn in the key's
  colour; any other pixel keeps its own colour (outlines, eye whites);
  every drawn pixel is opaque. It borrows `out`'s pixels once, so the
  revision moves once (ADR-0078 §2). A part whose pixels are missing is
  an error naming it.
- **Docs** (the `document` skill): where the chosen home's docs go (a
  module's README and skill, or the format's row in `docs/formats.md`
  marked as behind its feature), and a short guide: draw in keyed greys,
  roll, paint, keep the seed in the rules' state.
- **An ADR** (the `write-adr` skill): where it lives and why, grey keys
  over indexed colour or a shader, rolls per slot from hashed seeds,
  looks worked out by the skin and never stored.

Nothing breaks, and nothing changes for a game that doesn't opt in: no
`Breaking:` line.

**Out (do not do):**
- **Making it the main way to draw people.** Nothing that exists today
  changes how it works; a game opts in.
- **Palette swap on the GPU or per sprite** (2327, parked), and tints
  (2337): this composites on the CPU into a painted picture, which every
  painter already draws. Neither ticket changes.
- **Animation.** A part set makes still pictures (portraits, cards,
  icons). Parts per animation frame, or bodies with moving limbs
  (ADR-0010), are another feature; a game that asks gets its own
  ticket.
- **Weights in a list** ("a beard one time in four"): a list may repeat a
  part to weight it. Weights as numbers can come later, additively.
- **The picture set (2246).** Named, hand-drawn pictures per id stay
  there; nothing here changes it.
- **A cache of painted looks.** A game keeps one `PaintedId` per place a
  face shows and repaints it when the person changes; a 32 × 32 face of
  ten parts is cheap.
- **An example game.** The tests and the look snapshot prove it; an example comes when a kind of game asks (ADR-0009 §7).
- **Art of the game's**, or any bought art (`AGENTS.md` rule 4): the
  fixtures are the engine's own, drawn for the tests.

## Implementation steps

1. Choose the home (*Where it lives*): read ADR-0009, `docs/modules.md`
   and how `shuyi-assets`' format features are set up, then set up the
   home with what it needs and nothing more.
2. The serde types of `PartsDef`, the checks, `PartsRon`, and the
   resolution against `ImageTable`. Follow `palette.rs` for colours and
   errors, and `docs/formats.md`'s *Checks after loading* for "checked
   alone, then resolved".
3. The roll: a helper `slot_rng(seed, what, name) -> SimRng` hashing with
   `fnv::Hasher` (`word(seed)`, then `bytes` of `what` and `name`), and
   `appearance`.
4. `check_pixels`.
5. `paint`, with a one-pass lookup table from grey to colour
   (`[Option<[u8; 3]>; 256]`).
6. Fixtures: a 4-slot set of 8 × 8 parts in the chosen crate's
   `tests/fixtures` (pictures drawn for the test, the engine's own), and
   a 2-kind set.
7. What the home needs (features, `tools/layers.ron`, the CI step; for a
   module, its catalog row and skill too), docs and the ADR. Run `cargo
   xtask check-layers` (and `check-modules` for a module).

## Acceptance criteria

- [ ] The example file above (completed with the parts it names) loads;
  each check in *Checked alone* has a file that fails it, with its
  line.
- [ ] `appearance("plain", s, &[])` gives the same `Appearance` for the
  same `s` on every run, and at least two different ones over seeds 0 to
  99.
- [ ] Adding a slot (and its parts) to a set, or a colour group, leaves
  every other slot's part and every other key's colour unchanged for
  seeds 0 to 999.
- [ ] `"elf"` with base `"plain"` rolls `"plain"`'s heads and hair and
  always has `ears_long` in `"ears"`; a base cycle is an error naming
  the kinds in it.
- [ ] Wearing `helm_iron` puts it in `"helmet"` and empties `"hair"`,
  `"back_hair"` and `"ears"`; wearing an unknown part, or two parts of
  one slot, is an error.
- [ ] `paint`, on an 8 × 8 fixture: a key-grey pixel is the rolled
  colour; a red pixel stays red; a pixel with alpha 100 leaves what is
  below it; the later slot's pixel is over the earlier's; `out`'s
  revision moved by one; a wrong size or kind of `out` is an error.
- [ ] `check_pixels` reports a grey of 198 in a set whose key is 200,
  naming the file and pixel, and nothing for 190.
- [ ] `PartsRon` has ADR-0007's fuzz-style test (as `PaletteRon`'s in
  `crates/assets/src/format/builtin_tests.rs`), and a golden table,
  `tests/golden/parts.ron` in the chosen crate, has a row `part set: parts`
  (the decoded fixture's hash) and a row `appearance` (seed 7's look of
  the fixture set), which the `determinism` job runs on wasm32.
- [ ] The format and the roll build and work without the painting half
  (and so without `shuyi-ui`).
- [ ] A raster-painter snapshot (`frame-png`'s painter) of three rolled
  faces of the fixture set side by side: a look test, so it reads
  pixels.
- [ ] Opt-in: a game that doesn't opt in builds none of it (`cargo
  tree` or a build without the feature, recorded in the completion
  notes), and every existing snapshot and golden row is unchanged.
  `cargo xtask check-layers` passes (and `check-modules`, if it is a
  module).
- [ ] The completion notes say which home was chosen, which others were
  weighed, and why.
- [ ] The book's chapter is updated (2802), or the completion notes have a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass.

## Tests required

- Unit: every check of the format; `appearance`'s four steps and its
  errors; `paint`'s pixel rules; `check_pixels`.
- Property: the "adding a slot changes nothing else" rule over random
  sets and seeds; `appearance` never panics on any seed.
- Snapshot / integration: the golden table, the fuzz-style test, and the
  raster snapshot.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
