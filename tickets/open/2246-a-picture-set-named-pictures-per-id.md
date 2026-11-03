---
id: "2246"
title: "A picture set: named pictures per id, with the required names and a size limit as format parameters, and the check that a name exists"
type: feature
milestone: After MVP
model: sonnet-5 / gpt-6.1-terra
effort: medium
status: todo
blocked_by: []
nick_input: none
completed:
---

# 2246 — A picture set: named pictures per id

## Context

A request from the game.

1. **The game ticket:** game#1222, portraits as the engine's picture set
   (`github.com/Zafnok/visions-of-shuyi/blob/main/tickets/open/1222-portraits-as-the-engines-picture-set.md`).
   It is not a blocker of this one.
2. **The job in one sentence:** give each id (a person, an item, a
   card) a file of named pictures (`"happy": "ana/smile.png"`), check
   that every required name is there and every picture fits a size, and
   ask whether an id has a picture of a name.
3. **The friction numbers:** no friction review. The game/engine
   boundary audit of 2026-11-03 found the game's
   `crates/content/src/portrait.rs` (112 production lines) generic, and
   marked E in its `docs/engine/boundary.md` (line 170: "the five
   required expressions and the 256 px frame become parameters") but
   never moved. Its rule "a name must be one of the id's pictures, or of
   a standard list when the id has none" is written twice more in the
   game, once per `SceneRules::expression_problem` and once in a
   cinematic check.
4. **The generic shape:** a kind `PictureSetDef` (an id and a name →
   image path map), its format `PictureSetRon` taking `PictureRules`
   (required names, a largest size) as `PaletteRon::requiring` takes
   its names, a `resolve` against the `ImageTable` into a `PictureSet`,
   and `name_problem`, the answer a `SceneRules::expression_problem`
   gives from picture sets.
5. **An example from a game that isn't tactics:** a farming game's
   villagers, each with `neutral`, `happy` and `upset` faces for its
   shop talk; a card game's cards, each with `front` and `back`.

## Nick input

None.

## Scope

**In:** a module `crates/assets/src/picture_set.rs`, all additive (no
`Breaking:` line), public types `#[non_exhaustive]` with `new`
(ADR-0039):
- `PictureRules<'a> { required: &'a [&'a str], max_px: (u32, u32) }`.
- `PictureSetDef { id: String, pictures: BTreeMap<String, String> }`,
  `deny_unknown_fields`, the file `( id: "ana", pictures: { "neutral":
  "ana/neutral.png", … } )`. `AssetKind`, `NAME` `"picture set"`.
  `PictureSetDef::from_source(file, stem, source, rules)` reports every
  problem: the RON, an `id` other than the stem, each required name
  missing. It keeps each picture's line (`error::line_of`) for
  `resolve`'s errors.
- `PictureSetRon::new(rules: PictureRules<'static>)`, id `picture-set`,
  extension `ron`, `TEXT_MAX_BYTES`, decoding with `from_source` and
  `AssetPath::stem`; not in `register_standard` (it takes the game's
  names).
- `PictureSetDef::resolve(&self, file, dir, images: &ImageTable,
  bundle: Option<&Bundle>, rules) -> Result<PictureSet, Vec<ContentError>>`:
  each path, relative to `dir` (the file's folder), in the table and
  within `max_px`; images shown by `Bundle::display_path` when a bundle
  is given.
- `PictureSet { id, pictures: Vec<NamedPicture> }`, the required names
  first in `rules.required`' order, then the others by name, and
  `get(name)`; `NamedPicture { name, image: ImageId, size: ImageInfo }`.
- `name_problem<'a>(sets: &[&'a PictureSet], name: &str, without_a_set:
  &'a [&'a str]) -> Option<NameProblem<'a>>`:
  `NameProblem::NotInSet(&PictureSet)` for the first set lacking `name`,
  `NameProblem::NotAllowed(&[&str])` when `sets` is empty and `name` isn't
  in `without_a_set`. `Display` in generic words; a game may word its own.
- `docs/formats.md`: the row and a *Checks after loading* bullet.
- **Either order with 2721:** if 2721 is done, its `CharacterDef`'s
  `Expression` layer and the `SceneRules` built from the characters
  answer through `name_problem`, and the module docs say a VN character
  is a richer picture set; if not, nothing to do (2721's clause says so).

**Out (do not do):**
- Drawing a picture set, poses, layers or offsets (2721's `CharacterDef`,
  parked).
- The game's use of it (game#1222).

## Implementation steps

1. **Read** `palette.rs` (`PaletteRon::requiring`), `image.rs`,
   `error.rs` (`line_of`), `dialogue/check.rs` (`SceneRules`),
   `format/builtin_tests.rs`, and the game's
   `crates/content/src/portrait.rs` and its tests at the pin.
2. The types, `from_source`, `resolve`, `name_problem`.
3. `PictureSetRon`, a fixture in `format/builtin_tests.rs`, and a row
   `picture set: picture-set` in `tests/golden/formats.ron`
   (`cargo xtask golden --bless`).
4. A `SceneRules` example in the module docs answering
   `expression_problem` with `name_problem`. Expected size: about 130
   production lines (the game's 112 plus `name_problem`), 200 of tests.

## Acceptance criteria

- [ ] A set lacking a required name, with an id other than its stem, a
      missing image and one too big gives all four errors, the image
      ones with their lines.
- [ ] A good set's pictures come required first, then by name.
- [ ] `name_problem` gives each variant and `None` (tests).
- [ ] Random bytes into `PictureSetRon` never panic; its golden row
      passes.
- [ ] The book's chapter is updated (2802), or the completion notes have
      a `Book:` line saying why not.
- [ ] All gates in the `run-gates` skill pass, `api` reporting additions
      only.

## Tests required

- Unit: `from_source`, `resolve`, `name_problem`.
- Property: random bytes never panic the format.
- Fixture and determinism: the golden row.

## Completion notes

*(Filled in by the session that completes the ticket: what was done,
deviations, follow-up tickets created, notes for Nick.)*
