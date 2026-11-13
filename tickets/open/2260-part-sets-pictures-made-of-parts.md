---
id: "2260"
title: "The shuyi-parts module: pictures made of grey-keyed parts (faces, figures), a look rolled from a seed with kinds that override only what they need, and a skin that composites and colours it"
type: feature
milestone: After MVP
model: opus-5.5 / gpt-6.1-sol
effort: high
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2260 — The shuyi-parts module: pictures made of grey-keyed parts

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

**It is a module, opt-in** ([ADR-0009](../../docs/adr/0009-modules-a-small-default-engine-and-opt-in-crates.md)):
the crate `shuyi-parts`, turned on with `shuyi add parts`.

- Hand-drawn pictures stay the default, and nothing in the default
  engine changes: no default crate depends on the module (ADR-0009 §4),
  and a game that doesn't turn it on doesn't build it.
- A game can use it for its crowds and draw its named characters by
  hand (2246) in the same screen.

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
- **The crate** `shuyi-parts` in `crates/parts`, as ADR-0009 §4 says:
  - `[package.metadata.shuyi]`: `answers = [{ concern = "pictures",
    answer = "part-sets" }]`, `serves = ["turns", "fixed-ticks"]`;
  - its line in `tools/layers.ron`: engine `shuyi-core`, `shuyi-assets`,
    and `shuyi-ui` (only through its `paint` feature, below); outside
    `serde` and `thiserror`; `devices: false`, `floats: false`;
  - `README.md` in ADR-0009 §4's order, its size measured as
    `shuyi-script`'s README measures its own;
  - the `shuyi` crate's feature `parts = ["dep:shuyi-parts"]`, off by
    default (a library, so no `Module`, like `script`);
  - its row in `docs/modules.md` and its answer in `docs/modules.ron`.
    **Either order with 2259:** if 2259 is done, the `pictures` concern
    ("Pictures made from data", `alternatives: false`) and its catalog
    section exist: add `part-sets` (`by: "shuyi-parts"`, `status:
    ready`, `tickets: ["2260"]`) and the row; if not, add the concern
    with its `drawn` answer (`by: "default"`, `ready`) and this one, and
    the catalog's section with this row;
  - a skill `shuyi-parts` in `skills/` (`check-skills` passes);
  - `-p shuyi-parts` in CI's `determinism` job's crate list
    (`.github/workflows/ci.yml`).
- **The format**, `#![no_std]` with `alloc`, as ADR-0007 asks:
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
- **The skin's half**, behind the module's own `paint` feature (on by
  default; ADR-0009 §4: a part that pulls in `shuyi-ui` sits behind a
  feature, so a headless server can roll without it):
  `shuyi_parts::paint(set: &PartSet, look: &Appearance, pixels: impl
  Fn(ImageId) -> Option<&Image>, out: &mut Painted) -> Result<(),
  PaintError>`. `out` must be RGBA and the set's size. It clears `out`,
  then draws each slot's part in slot order: a pixel with alpha under
  128 is skipped; an opaque grey that is a key is drawn in the key's
  colour; any other pixel keeps its own colour (outlines, eye whites);
  every drawn pixel is opaque. It borrows `out`'s pixels once, so the
  revision moves once (ADR-0078 §2). A part whose pixels are missing is
  an error naming it.
- **Docs** (the `document` skill): the README, the skill, the format's
  page (a module's format is documented in its README and listed where
  the docs map puts formats), and a short guide: draw in keyed greys,
  roll, paint, keep the seed in the rules' state.
- **An ADR** (the `write-adr` skill): grey keys over indexed colour or a
  shader, rolls per slot from hashed seeds, looks worked out by the skin
  and never stored.

Nothing breaks and the default engine doesn't change: no `Breaking:`
line.

**Out (do not do):**
- **Any change to a default crate** (`shuyi-core`, `shuyi-assets`,
  `shuyi-ui`, …) beyond the `shuyi` crate's feature. The format and its
  kind live in the module, not in `shuyi-assets`.
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
- **An example game.** The module's tests and its look snapshot prove
  it; an example comes when a kind of game asks (ADR-0009 §7).
- **Art of the game's**, or any bought art (`AGENTS.md` rule 4): the
  fixtures are the engine's own, drawn for the tests.

## Implementation steps

1. Create `crates/parts` with its `Cargo.toml` (metadata, `paint`
   feature), `README.md`, and its `tools/layers.ron` line.
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
6. Fixtures: a 4-slot set of 8 × 8 parts in `crates/parts/tests/fixtures`
   (pictures drawn for the test, the engine's own), and a 2-kind set.
7. The `shuyi` feature, the catalog, `modules.ron`, the skill, the CI
   step, docs and the ADR. Run `cargo xtask check-modules` and
   `check-layers`.

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
  `crates/parts/tests/golden/parts.ron`, has a row `part set: parts`
  (the decoded fixture's hash) and a row `appearance` (seed 7's look of
  the fixture set), which the `determinism` job runs on wasm32.
- [ ] With `default-features = false` the crate builds without
  `shuyi-ui` and still rolls.
- [ ] A raster-painter snapshot (`frame-png`'s painter) of three rolled
  faces of the fixture set side by side: a look test, so it reads
  pixels.
- [ ] `cargo xtask check-modules` and `check-layers` pass; `shuyi
  modules` lists the module; a game without the `parts` feature builds
  no `shuyi-parts`.
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
